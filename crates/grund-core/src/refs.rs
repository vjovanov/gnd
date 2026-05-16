/// `grund refs <ID>[.<section>] [--summary] [--format text|json]` — the reverse of `grund show`:
/// list every place that cites the ID (§FS-refs.1, §FS-refs.2), scheme-aware where
/// a grep cannot be. Shares the scanner with `check` so the two never disagree on
/// what counts as a citation (§FS-refs.5). Empty results, including undeclared IDs
/// with no citations, exit `0` (§FS-refs.4).
fn command_refs(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("error: refs requires an ID");
        return ExitCode::from(2);
    }
    let mut id_arg = None;
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut section_override: Option<String> = None;
    let mut format_override: Option<String> = None;
    let mut summary = false;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--summary" => summary = true,
            "--section" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --section requires a value");
                    return ExitCode::from(2);
                }
                section_override = Some(args[idx].clone());
            }
            other if other.starts_with("--format=") => {
                format_override = Some(other.trim_start_matches("--format=").to_string());
            }
            "--format" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --format requires a value");
                    return ExitCode::from(2);
                }
                format_override = Some(args[idx].clone());
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other if id_arg.is_none() => id_arg = Some(other.to_string()),
            other => {
                if path_provided {
                    eprintln!("error: refs takes an ID and at most one path argument");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }
    let Some(id_arg) = id_arg else {
        eprintln!("error: refs requires an ID");
        return ExitCode::from(2);
    };
    let config = match resolve_workspace_config(&path, path_provided) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let (id, inline_section) = match parse_id_arg(&id_arg, &config.grammar) {
        Ok(parsed) => parsed,
        Err(err) => {
            // §FS-refs.1: an ID arg that does not match `[id] format` is a CLI-level
            // error (exit 2 — `refs` has no exit-`1` query-failure class, §FS-refs.4),
            // but the hint is the same one `grund show` gives for the same stumble
            // (§FS-show.3) — the common surprise in a repo whose format differs from
            // the `{kind}-{slug}` `grund` itself uses.
            eprintln!("error: {err:#}");
            eprintln!(
                "hint: this repo's [id] format is `{}` (run `grund config show`); `grund list` shows the IDs that exist",
                config.id_format
            );
            return ExitCode::from(2);
        }
    };
    if section_override.is_some() && inline_section.is_some() {
        eprintln!("error: --section cannot be combined with an inline section");
        return ExitCode::from(2);
    }
    let section = section_override.or(inline_section);
    let format = format_override.unwrap_or_else(|| config.output_format.clone());
    if !matches!(format.as_str(), "text" | "json") {
        eprintln!("error: unsupported refs format `{format}`");
        return ExitCode::from(2);
    }
    let (findings, scan_errors) = match scan_tree(&config, Some(&path), path_provided) {
        Ok(out) => out,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let mut citations = findings
        .citations
        .iter()
        // §FS-workspace.8, §AR-workspace.8: `grund refs` is project-local in
        // v1 — `§alias/<ID>` is a citation of *another* project's `<ID>`, not
        // of the local one. Skip qualified citations entirely.
        .filter(|citation| citation.namespace.is_none())
        .filter(|citation| citation.id == id)
        .filter(|citation| {
            section
                .as_deref()
                .map(|expected| citation.section.as_deref() == Some(expected))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    citations.sort_by(|a, b| {
        (sort_path_key(&a.file), a.line, a.column).cmp(&(sort_path_key(&b.file), b.line, b.column))
    });
    // §FS-refs.2: zero citations is a normal answer, not an error — but if the ID
    // is *also* undeclared, the caller most likely fat-fingered it, so leave a
    // breadcrumb on stderr without changing the exit code.
    if citations.is_empty() && !findings.declarations.contains_key(&id) {
        eprintln!(
            "note: {} is neither declared nor cited — run `grund list` to see every declared ID",
            render_id(&config, &id)
        );
    }
    // §FS-refs.3: the citation list is the *result* of the query, so it goes to
    // stdout (text and JSON alike), like `grund list` / `grund cover` / `grund show` —
    // even though a line shares the `path:line: <text>` shape `check` uses for
    // diagnostics on stderr. Only the `note:` breadcrumb above stays on stderr.
    if summary {
        let mut by_file: BTreeMap<PathBuf, (usize, BTreeSet<usize>)> = BTreeMap::new();
        for citation in citations {
            let entry = by_file
                .entry(citation.file.clone())
                .or_insert_with(|| (0, BTreeSet::new()));
            entry.0 += 1;
            entry.1.insert(citation.line);
        }
        let mut entries = by_file.iter().collect::<Vec<_>>();
        entries.sort_by_key(|(file, _)| display_path(&config, file));
        if format == "json" {
            for (file, (count, lines)) in entries {
                let lines_json = lines
                    .iter()
                    .map(|line| line.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                println!(
                    "{{\"path\":\"{}\",\"count\":{},\"lines\":[{}]}}",
                    json_escape(&display_path(&config, file)),
                    count,
                    lines_json
                );
            }
        } else {
            for (file, (count, lines)) in entries {
                let label = if lines.len() == 1 { "line" } else { "lines" };
                let lines_text = lines
                    .iter()
                    .map(|line| line.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                println!(
                    "{}: {} ({} {})",
                    display_path(&config, file),
                    count,
                    label,
                    lines_text
                );
            }
        }
    } else if format == "json" {
        for citation in citations {
            println!("{}", render_citation_json(&config, citation));
        }
    } else {
        for citation in citations {
            println!(
                "{}:{}: {}",
                display_path(&config, &citation.file),
                citation.line,
                citation.text
            );
        }
    }
    if scan_errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        // Partial-scan semantics (§FS-refs.4 / §FS-check.2): the listed citations
        // are real but the view of the tree was incomplete.
        for (file, message) in scan_errors {
            eprintln!("error: {}: {}", display_path(&config, &file), message);
        }
        ExitCode::from(2)
    }
}

fn render_citation_json(config: &Config, citation: &Citation) -> String {
    format!(
        "{{\"path\":\"{}\",\"line\":{},\"column\":{},\"id\":\"{}\",\"section\":{},\"marker\":{},\"text\":\"{}\"}}",
        json_escape(&display_path(config, &citation.file)),
        citation.line,
        citation.column,
        json_escape(&render_id(config, &citation.id)),
        citation
            .section
            .as_deref()
            .map(|section| format!("\"{}\"", json_escape(section)))
            .unwrap_or_else(|| "null".to_string()),
        citation.has_marker,
        json_escape(&citation.text)
    )
}

/// `grund list [path] [--kind K[,K]...] [--unused] [--summary] [--format text|json]` — print every
/// declared ID with its home `path:line` and one-line title (§FS-list.1,
/// §FS-list.3), optionally filtered to one kind or to declarations nothing cites
/// (the same set as the §FS-check.4.1 warning). The discovery side of the loop:
/// how an agent finds the right `<ID>` before citing it (§FS-list.5).
fn command_list(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut kind_filter: BTreeSet<String> = BTreeSet::new();
    let mut unused_only = false;
    let mut summary = false;
    let mut format_override: Option<String> = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--unused" => unused_only = true,
            "--summary" => summary = true,
            "--kind" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --kind requires a value");
                    return ExitCode::from(2);
                }
                add_kind_filters(&mut kind_filter, &args[idx]);
            }
            other if other.starts_with("--kind=") => {
                add_kind_filters(&mut kind_filter, other.trim_start_matches("--kind="));
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
            other => {
                if path_provided {
                    eprintln!("error: list takes at most one path argument");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }
    let config = match load_config(&path) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let format = format_override.unwrap_or_else(|| config.output_format.clone());
    if !matches!(format.as_str(), "text" | "json") {
        eprintln!("error: unsupported list format `{format}`");
        return ExitCode::from(2);
    }
    for kind in &kind_filter {
        if !config
            .kinds
            .iter()
            .any(|candidate| &candidate.prefix == kind)
        {
            eprintln!("error: unknown kind `{kind}`");
            eprintln!("known kinds: {}", kind_prefixes(&config.kinds).join(", "));
            return ExitCode::from(2);
        }
    }
    let (findings, scan_errors) = match scan_tree(&config, Some(&path), path_provided) {
        Ok(out) => out,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };

    let mut ref_counts: BTreeMap<&Id, usize> = BTreeMap::new();
    for citation in &findings.citations {
        *ref_counts.entry(&citation.id).or_insert(0) += 1;
    }

    struct Entry<'a> {
        id: &'a Id,
        home: &'a Declaration,
        duplicate: bool,
        refs: usize,
    }
    // `findings.declarations` is a BTreeMap keyed by `Id`, so the catalog comes
    // out in the same stable order `grund check` reports diagnostics in.
    let mut entries: Vec<Entry> = Vec::new();
    for (id, decls) in &findings.declarations {
        if !kind_filter.is_empty() && !kind_filter.contains(&id.kind) {
            continue;
        }
        let refs = ref_counts.get(id).copied().unwrap_or(0);
        if unused_only && refs > 0 {
            continue;
        }
        // `--unused` lists declarations nothing cites — but an E2E case is a proof
        // artifact, exercised by being run, not a citation target (the same reason
        // §FS-check.4.1 does not warn for uncited E2E cases). Bare `grund list --unused`
        // therefore skips E2E so the actionable signal — uncited specs and decisions —
        // is not buried under the whole case corpus; selecting E2E with `--kind`
        // opts back in for an inventory, even in a multi-kind filter (§FS-list.1,
        // §FS-list.3.1).
        if unused_only && id.kind == "E2E" && !kind_filter.contains("E2E") {
            continue;
        }
        // A stub paired with the inline declaration it points at is *one* home,
        // not two — collapse it the way `show` does (§FS-show.2.2.1). What's left
        // is one home in a healthy repo, more only when §FS-check.3.3 (duplicate
        // declaration) applies.
        let mut homes: Vec<&Declaration> = decls
            .iter()
            .filter(|decl| !is_stub_for_inline_decl(&config.root, decl, decls))
            .collect();
        homes.sort_by(|a, b| {
            (sort_path_key(&a.file), a.line).cmp(&(sort_path_key(&b.file), b.line))
        });
        let duplicate = homes.len() > 1;
        for home in homes {
            entries.push(Entry {
                id,
                home,
                duplicate,
                refs,
            });
        }
    }

    if summary {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for entry in &entries {
            *counts.entry(&entry.id.kind).or_insert(0) += 1;
        }
        if format == "json" {
            for kind in &config.kinds {
                let count = counts.get(kind.prefix.as_str()).copied().unwrap_or(0);
                if count == 0 {
                    continue;
                }
                println!(
                    "{{\"kind\":\"{}\",\"title\":\"{}\",\"home\":\"{}\",\"count\":{}}}",
                    json_escape(&kind.prefix),
                    json_escape(kind.title.as_deref().unwrap_or("Declaration")),
                    json_escape(kind.folder.as_deref().unwrap_or("")),
                    count
                );
            }
        } else {
            for kind in &config.kinds {
                let count = counts.get(kind.prefix.as_str()).copied().unwrap_or(0);
                if count == 0 {
                    continue;
                }
                println!(
                    "{:<4}  {:>3}  {}",
                    kind.prefix,
                    count,
                    kind.folder.as_deref().unwrap_or("")
                );
            }
        }
    } else if format == "json" {
        for entry in &entries {
            println!(
                "{{\"id\":\"{}\",\"kind\":\"{}\",\"path\":\"{}\",\"line\":{},\"title\":{},\"stub\":{},\"defines\":{},\"refs\":{},\"duplicate\":{}}}",
                json_escape(&render_id(&config, entry.id)),
                json_escape(&entry.id.kind),
                json_escape(&display_path(&config, &entry.home.file)),
                entry.home.line,
                entry
                    .home
                    .title
                    .as_deref()
                    .map(|title| format!("\"{}\"", json_escape(title)))
                    .unwrap_or_else(|| "null".to_string()),
                entry.home.is_stub,
                entry
                    .home
                    .defined_in
                    .as_ref()
                    .map(|target| format!("\"{}\"", json_escape(&format_path(target))))
                    .unwrap_or_else(|| "null".to_string()),
                entry.refs,
                entry.duplicate,
            );
        }
    } else {
        let id_width = entries
            .iter()
            .map(|entry| render_id(&config, entry.id).chars().count())
            .max()
            .unwrap_or(0)
            .min(40);
        for entry in &entries {
            let id_text = render_id(&config, entry.id);
            let location = format!(
                "{}:{}",
                display_path(&config, &entry.home.file),
                entry.home.line
            );
            let mut note = if entry.home.is_stub {
                entry
                    .home
                    .defined_in
                    .as_ref()
                    .map(|target| format!("→ {}", format_path(target)))
                    .unwrap_or_default()
            } else {
                entry.home.title.clone().unwrap_or_default()
            };
            if entry.duplicate {
                if note.is_empty() {
                    note = "(duplicate declaration — grund check)".to_string();
                } else {
                    note.push_str("  (duplicate declaration — grund check)");
                }
            }
            if note.is_empty() {
                println!("{id_text:<id_width$}  {location}");
            } else {
                println!("{id_text:<id_width$}  {location}  {note}");
            }
        }
    }

    if scan_errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        // Partial-scan semantics (§FS-check.2): the listed declarations are real
        // but the view of the tree was incomplete, so the catalog may be short.
        for (file, message) in scan_errors {
            eprintln!("error: {}: {}", display_path(&config, &file), message);
        }
        ExitCode::from(2)
    }
}

fn add_kind_filters(kind_filter: &mut BTreeSet<String>, raw: &str) {
    for kind in raw.split(',') {
        kind_filter.insert(kind.to_string());
    }
}

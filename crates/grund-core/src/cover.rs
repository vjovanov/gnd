fn command_cover(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut format_override: Option<String> = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
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
                    eprintln!("error: cover takes at most one path argument");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }
    let config = match resolve_workspace_config(&path, path_provided) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let format = format_override.unwrap_or_else(|| config.output_format.clone());
    if !matches!(format.as_str(), "text" | "json") {
        eprintln!("error: unsupported cover format `{format}`");
        return ExitCode::from(2);
    }
    let (findings, scan_errors) = match scan_tree(&config, Some(&path), path_provided) {
        Ok(out) => out,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };

    let mut by_file: BTreeMap<PathBuf, Vec<&Citation>> = BTreeMap::new();
    for file in &findings.scanned_files {
        by_file.entry(file.clone()).or_default();
    }
    for citation in &findings.citations {
        // §FS-workspace.8, §AR-workspace.8: `grund cover` is project-local in
        // v1; a `§alias/<ID>` in this file refers to another project's
        // declaration and would distort the per-file local citation map.
        if citation.namespace.is_some() {
            continue;
        }
        by_file
            .entry(citation.file.clone())
            .or_default()
            .push(citation);
    }
    for citations in by_file.values_mut() {
        citations.sort_by_key(|c| (c.line, c.column));
    }

    let mut cover_entries = by_file.iter().collect::<Vec<_>>();
    cover_entries.sort_by_key(|(file, _)| display_path(&config, file));

    if format == "json" {
        for (file, citations) in &cover_entries {
            let citation_json = citations
                .iter()
                .map(|citation| render_citation_json(&config, citation))
                .collect::<Vec<_>>()
                .join(",");
            println!(
                "{{\"path\":\"{}\",\"citations\":[{}]}}",
                json_escape(&display_path(&config, file)),
                citation_json
            );
        }
    } else {
        for (file, citations) in &cover_entries {
            println!("{}:", display_path(&config, file));
            if citations.is_empty() {
                println!("  (no citations)");
            } else {
                for citation in *citations {
                    println!("  {}:{} {}", citation.line, citation.column, citation.text);
                }
            }
        }
    }

    if scan_errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        // Partial-scan semantics (§FS-cover.4 / §FS-check.2): the emitted records
        // are real but incomplete, so callers must treat the result as untrusted.
        for (file, message) in scan_errors {
            eprintln!("error: {}: {}", display_path(&config, &file), message);
        }
        ExitCode::from(2)
    }
}

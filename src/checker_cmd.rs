/// `grund check [path] [--format text|json]` — the default subcommand (§FS-cli.1):
/// scan the tree, run the checker (§FS-check), print the report, and exit `0` clean
/// / `1` on a finding / `2` on a CLI or I/O error (§FS-check.2.1, §FS-cli.5).
fn command_check(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut format_override = None;
    let mut require_grounding = false;
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
            "--require-grounding" => require_grounding = true,
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other => {
                // §FS-cli.3: a path-taking subcommand accepts at most one path;
                // a second positional is a CLI error, never a silent drop.
                if path_provided {
                    eprintln!("error: check takes at most one path argument");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }
    if let Some(format) = &format_override
        && !matches!(format.as_str(), "text" | "json")
    {
        eprintln!("error: unsupported check format `{format}`");
        return ExitCode::from(2);
    }
    let mut config = match load_config(&path) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    // `--require-grounding` only ever turns the check on for this run; it never
    // turns off a `[reference] require_grounding = true` set in the config.
    if require_grounding {
        config.require_grounding = true;
    }
    let (findings, scan_errors) = match scan_tree(&config, Some(&path), path_provided) {
        Ok(out) => out,
        Err(e) => {
            eprintln!("error: {:#}", e);
            return ExitCode::from(2);
        }
    };
    let mut report = check(&findings, &config);
    // A file that could not be read mid-walk is reported as a CLI-shaped
    // `error: <path>: <reason>` finding (§FS-check.2): the walk continued, the
    // findings below are real, but the view of the tree was incomplete → exit 2.
    let had_scan_errors = !scan_errors.is_empty();
    for (file, message) in scan_errors {
        report.errors.push(Diagnostic {
            code: "io",
            path: Some(file),
            line: None,
            message,
            sites: Vec::new(),
        });
    }
    sort_diagnostics(&mut report.errors);
    // §FS-check.2.2: a walk that read no files and turned up nothing to report is
    // almost always a misconfigured scope, not a clean repo — say so on stderr
    // instead of printing nothing and exiting 0. This is a warning: it never
    // changes the exit code. (The agent-entrypoint check, §FS-check.3.5, runs even
    // when no source file is scanned, so a missing/stale `AGENTS.md` block still
    // reports normally and suppresses this notice.)
    if findings.scanned_files.is_empty() && report.errors.is_empty() && report.warnings.is_empty() {
        report
            .warnings
            .push(empty_scan_warning(&config, &path, path_provided));
    }
    let format = format_override.unwrap_or_else(|| config.output_format.clone());
    if !matches!(format.as_str(), "text" | "json") {
        eprintln!("error: unsupported check format `{format}`");
        return ExitCode::from(2);
    }
    if format == "json" {
        print_json_report(&config, &report);
    } else {
        print_report(&config, &report);
    }
    if had_scan_errors {
        ExitCode::from(2)
    } else if report.errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

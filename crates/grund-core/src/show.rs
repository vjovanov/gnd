/// `grund show <ID>[.<section>] [--brief|--toc|--full] [--section S] [--format text|md|json]`
/// — print a slice of one declaration's body (§FS-show.1): by default the *lead*
/// (prose down to the first child heading, §FS-show.2.1); `--brief` for heading + first
/// paragraph (§FS-show.2.1.1); `--toc` for the lead plus the section map (§FS-show.2.1.2);
/// `--full` for everything (§FS-show.2.1.3); a section with `.<section>` or `--section`
/// (§FS-show.2.2). Ambiguous IDs and missing IDs/sections exit `1` with a hint
/// (§FS-show.2.2.1, §FS-show.3).
fn command_show(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("error: show requires an ID");
        return ExitCode::from(2);
    }
    let mut id_arg = None;
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut mode = ShowMode::Default;
    let mut mode_flag: Option<&'static str> = None;
    let mut section_override = None;
    let mut format = "text".to_string();
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--brief" => {
                if let Some(previous) = mode_flag {
                    eprintln!("error: {previous} and --brief cannot be used together");
                    return ExitCode::from(2);
                }
                mode_flag = Some("--brief");
                mode = ShowMode::Brief;
            }
            "--toc" => {
                if let Some(previous) = mode_flag {
                    eprintln!("error: {previous} and --toc cannot be used together");
                    return ExitCode::from(2);
                }
                mode_flag = Some("--toc");
                mode = ShowMode::Toc;
            }
            "--full" => {
                if let Some(previous) = mode_flag {
                    eprintln!("error: {previous} and --full cannot be used together");
                    return ExitCode::from(2);
                }
                mode_flag = Some("--full");
                mode = ShowMode::Full;
            }
            other if other.starts_with("--format=") => {
                format = other.trim_start_matches("--format=").to_string();
            }
            "--section" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --section requires a value");
                    return ExitCode::from(2);
                }
                section_override = Some(args[idx].clone());
            }
            "--path" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --path requires a value");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(&args[idx]);
                path_provided = true;
            }
            "--format" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --format requires a value");
                    return ExitCode::from(2);
                }
                format = args[idx].clone();
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other if id_arg.is_none() => id_arg = Some(other.to_string()),
            other => {
                if path_provided {
                    eprintln!("error: show takes an ID and at most one path argument");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }
    let Some(id_arg) = id_arg else {
        eprintln!("error: show requires an ID");
        return ExitCode::from(2);
    };
    let config = match load_config(&path) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let (id, inline_section) = match parse_id_arg(&id_arg, &config.grammar) {
        Ok(parsed) => parsed,
        Err(err) => {
            let message = format!("{err:#}");
            if format == "json" {
                print_bare_query_json(&config, show_query_error_code(&message), &message);
            } else {
                eprintln!("{message}");
                eprintln!(
                    "hint: this repo's [id] format is `{}` (run `grund config show`); `grund list` shows the IDs that exist",
                    config.id_format
                );
            }
            return ExitCode::FAILURE;
        }
    };
    if section_override.is_some() && inline_section.is_some() {
        eprintln!("error: --section cannot be combined with an inline section");
        return ExitCode::from(2);
    }
    let section = section_override.or(inline_section);
    if !matches!(format.as_str(), "text" | "md" | "json") {
        eprintln!("error: unsupported show format `{format}`");
        return ExitCode::from(2);
    }
    let findings = match scan_tree_strict(&config, Some(&path), path_provided) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {:#}", e);
            return ExitCode::from(2);
        }
    };
    match show_declaration(
        &config,
        &findings,
        &id,
        section.as_deref(),
        mode,
        format == "md",
    ) {
        Ok(mut output) => {
            // §FS-show.3.2: `text` and `json` flatten `--cross-refs` link wrappers
            // back to bare `§…` citations; `md` keeps the renderable form verbatim.
            if format != "md" {
                output.body = flatten_cross_ref_links(&output.body, &config);
            }
            if format == "json" {
                if let Some(json) = output.json {
                    println!("{json}");
                } else {
                    let mut extra = String::new();
                    if matches!(mode, ShowMode::Toc) {
                        extra.push_str(",\"sections\":[");
                        extra.push_str(
                            &output
                                .sections
                                .iter()
                                .map(|section| {
                                    format!(
                                        "{{\"path\":\"{}\",\"title\":\"{}\",\"depth\":{}}}",
                                        json_escape(&section.path),
                                        json_escape(&section.title),
                                        section.depth
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(","),
                        );
                        extra.push(']');
                    }
                    println!(
                        "{{\"id\":\"{}\",\"section\":{},\"body\":\"{}\",\"path\":\"{}\",\"line\":{}{}}}",
                        json_escape(&render_id(&config, &id)),
                        match section.as_deref() {
                            Some(section) => format!("\"{}\"", json_escape(section)),
                            None => "null".to_string(),
                        },
                        json_escape(&output.body),
                        json_escape(&display_path(&config, &output.path)),
                        output.line,
                        extra
                    );
                }
            } else {
                print!("{}", output.body);
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            let message = format!("{err:#}");
            if format == "json" {
                print_bare_query_json(&config, show_query_error_code(&message), &message);
            } else {
                eprintln!("{message}");
                if message.starts_with("ID not found:") {
                    eprintln!(
                        "hint: run `grund list` to see every declared ID, or `grund id <KIND> \"<title>\"` to propose a new one"
                    );
                } else if message.starts_with("section not found:") {
                    eprintln!(
                        "hint: run `grund show {} --toc` to print the lead with the section map",
                        render_id(&config, &id)
                    );
                }
            }
            ExitCode::FAILURE
        }
    }
}


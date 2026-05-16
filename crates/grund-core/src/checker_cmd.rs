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
    let mut config = match resolve_workspace_config(&path, path_provided) {
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
    if config.workspace_declared && is_workspace_root_scope(&config, &path, path_provided) {
        return command_check_workspace(config, format_override, require_grounding);
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

struct ProjectScan {
    alias: String,
    config: Config,
    findings: Findings,
    scan_errors: Vec<ScanError>,
}

fn command_check_workspace(
    mut root_config: Config,
    format_override: Option<String>,
    force_require_grounding: bool,
) -> ExitCode {
    let member_roots = match expand_workspace_members(&root_config) {
        Ok(roots) => roots,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    // §AR-workspace.6: a root-scope scan must not descend into member roots
    // — otherwise the root namespace would absorb member declarations.
    root_config.workspace_boundary_roots = member_roots.clone();
    let mut projects = Vec::new();
    if root_config.workspace_include_root {
        let alias = match derive_alias(&root_config, None, RootMode::Root) {
            Ok(alias) => alias,
            Err(err) => {
                eprintln!("error: {err}");
                return ExitCode::from(2);
            }
        };
        projects.push((alias, root_config.clone()));
    }
    for member_root in member_roots {
        let mut member_config = match load_config_at(&member_root, &root_config.cli_base) {
            Ok(config) => config,
            Err(err) => {
                eprintln!("error: {err:#}");
                return ExitCode::from(2);
            }
        };
        // §AR-workspace.6.1: nested workspaces are rejected at load — not
        // silently flattened. A member declaring its own `[workspace]` is a
        // hard configuration error so the resolver invariants remain pinned.
        if member_config.workspace_declared {
            eprintln!(
                "error: workspace member `{}` declares its own `[workspace]` block (nested workspaces are not supported)",
                display_path(&root_config, &member_config.root)
            );
            return ExitCode::from(2);
        }
        if force_require_grounding {
            member_config.require_grounding = true;
        }
        let alias = match derive_alias(&member_config, Some(&member_root), RootMode::Member) {
            Ok(alias) => alias,
            Err(err) => {
                eprintln!("error: {err}");
                return ExitCode::from(2);
            }
        };
        projects.push((alias, member_config));
    }

    let mut seen_aliases = BTreeSet::new();
    for (alias, _) in &projects {
        if !seen_aliases.insert(alias.clone()) {
            eprintln!("error: duplicate workspace project alias `{alias}`");
            return ExitCode::from(2);
        }
    }

    let mut scanned = Vec::new();
    for (alias, config) in projects {
        let (findings, scan_errors) = match scan_tree(&config, Some(&config.root), true) {
            Ok(out) => out,
            Err(e) => {
                eprintln!("error: {:#}", e);
                return ExitCode::from(2);
            }
        };
        scanned.push(ProjectScan {
            alias,
            config,
            findings,
            scan_errors,
        });
    }

    let workspace = scanned
        .iter()
        .map(|project| (project.alias.clone(), &project.findings))
        .collect::<BTreeMap<_, _>>();
    let mut report = Report::default();
    let mut had_scan_errors = false;
    for project in &scanned {
        let mut project_report = check_with_workspace(
            &project.findings,
            &project.config,
            Some(&project.alias),
            &workspace,
        );
        let project_has_findings =
            !project_report.errors.is_empty() || !project_report.warnings.is_empty();
        report.errors.append(&mut project_report.errors);
        report.warnings.append(&mut project_report.warnings);
        had_scan_errors |= !project.scan_errors.is_empty();
        for (file, message) in &project.scan_errors {
            report.errors.push(Diagnostic {
                code: "io",
                path: Some(file.clone()),
                line: None,
                message: message.clone(),
                sites: Vec::new(),
            });
        }
        // §FS-check.2.2: same empty-scan warning as the single-project path —
        // a member that scanned zero files and reported nothing is almost
        // always a misconfigured scope, not a clean repo.
        if project.findings.scanned_files.is_empty()
            && project.scan_errors.is_empty()
            && !project_has_findings
        {
            report
                .warnings
                .push(empty_scan_warning(&project.config, &project.config.root, true));
        }
    }
    sort_diagnostics(&mut report.errors);
    sort_diagnostics(&mut report.warnings);
    let format = format_override.unwrap_or_else(|| root_config.output_format.clone());
    if format == "json" {
        print_json_report(&root_config, &report);
    } else {
        print_report(&root_config, &report);
    }
    if had_scan_errors {
        ExitCode::from(2)
    } else if report.errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn is_workspace_root_scope(config: &Config, path: &Path, path_provided: bool) -> bool {
    !path_provided
        || fs::canonicalize(path)
            .map(|path| path == config.root)
            .unwrap_or(false)
}

/// §AR-workspace.5.1, §AR-workspace.6, §AR-workspace.8: every CLI entry point
/// that walks the tree funnels through this helper so workspace handling is
/// identical across `check`, `fmt`, `refs`, `list`, `cover`, `show`, `id`, and
/// completions. The three steps are upward discovery, the configless-member
/// rewrite, and boundary-root population — the last is what stops a root-scope
/// scan from absorbing member declarations into the parent namespace.
fn resolve_workspace_config(path: &Path, path_provided: bool) -> Result<Config> {
    let mut config = load_config(path)?;
    config = config_for_configless_workspace_member(config, path)?;
    apply_workspace_boundary(&mut config, path, path_provided)?;
    Ok(config)
}

/// §AR-workspace.6: a workspace-declared scan must never descend into member
/// roots. The boundary is the same list that `command_check_workspace`
/// computes; setting it on the Config makes the scanner skip those subtrees.
fn apply_workspace_boundary(
    config: &mut Config,
    _path: &Path,
    _path_provided: bool,
) -> Result<()> {
    if !config.workspace_declared {
        return Ok(());
    }
    config.workspace_boundary_roots = expand_workspace_members(config)?;
    Ok(())
}

/// §FS-workspace.2 / §FS-workspace.5: upward discovery from a configless member
/// finds the parent workspace config, but a member-scoped check still runs as an
/// independent project rooted at that member and therefore uses member defaults.
fn config_for_configless_workspace_member(mut config: Config, path: &Path) -> Result<Config> {
    if !config.workspace_declared {
        return Ok(config);
    }
    let scope = config_scope_start(path);
    if scope == config.root {
        return Ok(config);
    }
    if let Some(member_root) = configured_member_root_for_scope(&config, &scope) {
        config = load_config_at(&member_root, &config.cli_base)?;
    }
    Ok(config)
}

fn config_scope_start(path: &Path) -> PathBuf {
    let start = if path.is_file() {
        path.parent().unwrap_or(Path::new("."))
    } else {
        path
    };
    fs::canonicalize(start).unwrap_or_else(|_| start.to_path_buf())
}

fn configured_member_root_for_scope(config: &Config, scope: &Path) -> Option<PathBuf> {
    config
        .workspace_members
        .iter()
        .filter_map(|member| configured_member_root_candidate(config, member, scope))
        .max_by_key(|root| root.components().count())
}

fn configured_member_root_candidate(config: &Config, member: &str, scope: &Path) -> Option<PathBuf> {
    if let Some(parent) = member.strip_suffix("/*") {
        let parent = canonical_workspace_path(&config.root.join(parent));
        let relative = scope.strip_prefix(&parent).ok()?;
        let Component::Normal(child) = relative.components().next()? else {
            return None;
        };
        let root = parent.join(child);
        return Some(canonical_workspace_path(&root));
    }
    let root = canonical_workspace_path(&config.root.join(member));
    if scope == root || scope.starts_with(&root) {
        Some(root)
    } else {
        None
    }
}

fn canonical_workspace_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn expand_workspace_members(config: &Config) -> Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    for member in &config.workspace_members {
        if let Some(parent) = member.strip_suffix("/*") {
            let parent = config.root.join(parent);
            if !parent.is_dir() {
                return Err(anyhow!(
                    "workspace member glob parent does not exist: {}",
                    display_path(config, &parent)
                ));
            }
            for entry in fs::read_dir(&parent)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    roots.push(fs::canonicalize(&path).unwrap_or(path));
                }
            }
        } else {
            let path = config.root.join(member);
            if !path.is_dir() {
                return Err(anyhow!(
                    "workspace member does not exist: {}",
                    display_path(config, &path)
                ));
            }
            roots.push(fs::canonicalize(&path).unwrap_or(path));
        }
    }
    roots.sort_by_key(|path| sort_path_key(path));
    roots.dedup();
    Ok(roots)
}

enum RootMode {
    Root,
    Member,
}

/// §AR-workspace.5.3: the single canonical place that derives a project's
/// workspace alias. `project_name` wins; otherwise the member directory's
/// basename, or the literal `root` for an unnamed workspace root. Whichever
/// source fires, the result is validated against the alias slug grammar so a
/// bad name fails fast at workspace expansion, not later inside a citation.
fn derive_alias(
    config: &Config,
    member_root: Option<&Path>,
    mode: RootMode,
) -> std::result::Result<String, String> {
    let alias = match (&config.project_name, &mode) {
        (Some(name), _) => name.clone(),
        (None, RootMode::Root) => "root".to_string(),
        (None, RootMode::Member) => {
            // Members always have a canonical absolute path with a final
            // component; the basename fallback is the alias source defined in
            // §AR-workspace.5.3.
            let path = member_root.expect("member alias derivation needs a member root");
            path.file_name()
                .and_then(|name| name.to_str())
                .expect("workspace member root has a final UTF-8 path component")
                .to_string()
        }
    };
    if is_valid_project_alias(&alias) {
        Ok(alias)
    } else {
        Err(format!(
            "invalid workspace project alias `{alias}` (expected [a-z][a-z0-9-]*)"
        ))
    }
}

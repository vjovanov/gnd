/// Options for programmatic declaration reads through [`show`]
/// (§FS-distribution.3.0, §FS-distribution.3.1).
#[derive(Clone)]
pub struct ShowOpts {
    pub path: PathBuf,
    pub section: Option<String>,
    pub mode: ShowMode,
    pub format: ShowFormat,
}

impl Default for ShowOpts {
    fn default() -> Self {
        Self {
            path: PathBuf::from("."),
            section: None,
            mode: ShowMode::Lead,
            format: ShowFormat::Text,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ShowMode {
    Brief,
    Lead,
    Toc,
    Full,
}

impl ShowMode {
    fn render_mode(self) -> ShowRenderMode {
        match self {
            ShowMode::Brief => ShowRenderMode::Brief,
            ShowMode::Lead => ShowRenderMode::Default,
            ShowMode::Toc => ShowRenderMode::Toc,
            ShowMode::Full => ShowRenderMode::Full,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ShowFormat {
    Text,
    Markdown,
    Json,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FindingSite {
    pub path: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    pub severity: &'static str,
    pub code: &'static str,
    pub path: Option<String>,
    pub line: Option<usize>,
    pub message: String,
    pub sites: Vec<FindingSite>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Report {
    pub errors: Vec<Finding>,
    pub warnings: Vec<Finding>,
}

/// Scan one project tree and return the raw scanner findings. This is the
/// embedding surface later frontends share instead of re-reading files.
pub fn scan(path: &Path) -> Result<Findings> {
    let config = resolve_workspace_config(path)?;
    scan_tree_strict(&config, Some(path), true)
}

/// Programmatic `check`: load config, scan, and return structured findings
/// without CLI argument parsing, stdout/stderr rendering, or exit-code mapping
/// (§FS-distribution.3.1, §RM-core-cli-split.3).
pub fn check(path: &Path) -> Result<Report> {
    let mut config = resolve_workspace_config(path)?;
    if config.workspace_declared && is_workspace_root_scope(&config, path, true) {
        let report = check_workspace(&mut config)?;
        return Ok(public_report(&config, report));
    }
    let (findings, scan_errors) = scan_tree(&config, Some(path), true)?;
    let mut report = check_findings(&findings, &config);
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
    if findings.scanned_files.is_empty() && report.errors.is_empty() && report.warnings.is_empty() {
        report.warnings.push(empty_scan_warning(&config, path, true));
    }
    Ok(public_report(&config, report))
}

fn check_workspace(root_config: &mut Config) -> Result<CheckReport> {
    let projects = load_workspace_projects(root_config)?;
    let workspace = projects
        .iter()
        .map(|project| {
            (
                project.alias.clone(),
                WorkspaceCheckTarget {
                    findings: &project.findings,
                    config: &project.config,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut report = CheckReport::default();
    for project in &projects {
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
        for (file, message) in &project.scan_errors {
            report.errors.push(Diagnostic {
                code: "io",
                path: Some(file.clone()),
                line: None,
                message: message.clone(),
                sites: Vec::new(),
            });
        }
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
    Ok(report)
}

fn public_report(config: &Config, report: CheckReport) -> Report {
    Report {
        errors: report
            .errors
            .into_iter()
            .map(|diagnostic| public_finding(config, "error", diagnostic))
            .collect(),
        warnings: report
            .warnings
            .into_iter()
            .map(|diagnostic| public_finding(config, "warning", diagnostic))
            .collect(),
    }
}

fn public_finding(config: &Config, severity: &'static str, diagnostic: Diagnostic) -> Finding {
    Finding {
        severity,
        code: diagnostic.code,
        path: diagnostic.path.map(|path| public_path(config, &path)),
        line: diagnostic.line,
        message: diagnostic.message,
        sites: diagnostic
            .sites
            .into_iter()
            .map(|site| FindingSite {
                path: public_path(config, &site.path),
                line: site.line,
            })
            .collect(),
    }
}

fn public_path(config: &Config, path: &Path) -> String {
    format_path(path.strip_prefix(&config.root).unwrap_or(path))
}

/// Programmatic declaration read. This mirrors `grund show` resolution but
/// returns the structured body instead of printing it.
pub fn show(id_arg: &str, opts: ShowOpts) -> Result<ShowOutput> {
    let context = load_workspace_context(&opts.path, true)?;
    let (alias, raw_id) = split_qualified_id_arg(id_arg)?;
    let project = match alias.as_deref() {
        Some(name) => context
            .project_by_alias(name)
            .ok_or_else(|| anyhow!("unknown project alias `{name}`"))?,
        None => context
            .current_project()
            .ok_or_else(|| anyhow!("unqualified ID requires a project alias when include_root = false"))?,
    };
    if let Some((file, message)) = project.scan_errors.first() {
        return Err(anyhow!(
            "{}: {}",
            display_path(&project.config, file),
            message
        ));
    }
    let config = &project.config;
    let (id, inline_section) = parse_id_arg(raw_id, &config.grammar)?;
    if opts.section.is_some() && inline_section.is_some() {
        return Err(anyhow!("--section cannot be combined with an inline section"));
    }
    let section = opts.section.or(inline_section);
    let mut output = show_declaration(
        config,
        &project.findings,
        &id,
        section.as_deref(),
        opts.mode.render_mode(),
        opts.format == ShowFormat::Markdown,
    )?;
    if opts.format != ShowFormat::Markdown {
        output.body = flatten_cross_ref_links(&output.body, config);
    }
    if opts.format == ShowFormat::Json {
        let json = render_show_output_json(
            config,
            &id,
            section.as_deref(),
            opts.mode.render_mode(),
            &output,
        );
        output.json = Some(json);
    }
    Ok(output)
}

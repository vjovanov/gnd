/// Whether a file is one the scanner reads: a non-hidden name with an extension in
/// `[scan] extensions` (§FS-config.3.5, §AR-scanner.1).
fn is_scannable(path: &Path, config: &Config) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if name.starts_with('.') {
        return false;
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    config.extensions.iter().any(|allowed| allowed == ext)
}

/// The per-file scan (§AR-scanner.2): line by line, find declaration headings
/// (§AR-scanner.2.1 — in Markdown or in a code/`"""` doc-comment, §AR-scanner.4),
/// nested section headings (§AR-scanner.2.2), and `<ID>[.<section>]` citations
/// (§AR-scanner.2.3, §FS-check.1.1) — skipping fenced code blocks and, outside
/// Markdown, bare ID-shaped tokens inside string literals (§FS-fmt.2.3.1) and any
/// bare token at all under `[reference] strict` (§FS-config.3.1).
///
/// One citation regex is used for every scan; whether a match is qualified
/// (marker + `<alias>/<ID>`) or unqualified (marker + `<ID>`) is determined by
/// whether the `<namespace>` capture fired (§AR-workspace.3.1). The alias
/// prefix is only honoured when the marker precedes it — an unmarked
/// `<alias>/<ID>` in prose is text, never a qualified citation
/// (§FS-workspace.1, §AR-workspace.3.1).
fn scan_file(path: &Path, config: &Config, findings: &mut Findings) -> Result<()> {
    let text = fs::read_to_string(path)?;
    let is_md = path.extension().and_then(|e| e.to_str()) == Some("md");
    let is_py = path.extension().and_then(|e| e.to_str()) == Some("py");
    let in_docs = path.components().any(|c| c.as_os_str() == "docs");
    let mut in_fence = false;
    let mut in_py_docstring = false;
    let mut current: Option<Declaration> = None;

    for (idx, line) in text.lines().enumerate() {
        let lineno = idx + 1;
        let trimmed = line.trim_start();
        if is_md && (trimmed.starts_with("```") || trimmed.starts_with("~~~")) {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if config.docstring_python
            && is_py
            && (trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''"))
        {
            in_py_docstring = !in_py_docstring;
            continue;
        }
        let scan_line = if in_py_docstring {
            line.trim_start()
        } else {
            line
        };

        if let Some(caps) = config.grammar.decl_re.captures(scan_line)
            && let Some(id) = parse_id(&caps)
        {
            if let Some(prev) = current.take() {
                findings
                    .declarations
                    .entry(prev.id.clone())
                    .or_default()
                    .push(prev);
            }
            let tail = &scan_line[caps.get(0).unwrap().end()..];
            let mut is_stub = false;
            let mut defined_in = None;
            if is_md
                && in_docs
                && let Some(link_caps) = STUB_LINK_HEADING.captures(tail)
            {
                is_stub = true;
                defined_in = Some(PathBuf::from(link_caps.name("path").unwrap().as_str()));
            }
            let title = if is_stub {
                None
            } else {
                let trimmed = tail.trim_start();
                let trimmed = trimmed.strip_prefix(':').unwrap_or(trimmed).trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            };
            current = Some(Declaration {
                id,
                file: path.to_path_buf(),
                line: lineno,
                heading_level: heading_level_for_line(scan_line, is_md || in_py_docstring, &caps),
                sections: BTreeMap::new(),
                is_stub,
                defined_in,
                e2e_case: None,
                title,
            });
            continue;
        }

        if let Some(caps) = config.grammar.section_re.captures(scan_line)
            && let Some(decl) = current.as_mut()
            && let Some(sec) = caps.name("sec")
            && heading_level_for_line(scan_line, is_md || in_py_docstring, &caps)
                > decl.heading_level
        {
            decl.sections.insert(
                sec.as_str().to_string(),
                section_anchor_text(scan_line, sec.as_str()),
            );
        }

        for caps in config.grammar.citation_re.captures_iter(scan_line) {
            let Some(full) = caps.get(0) else { continue };
            let namespace = caps.name("namespace").map(|m| m.as_str().to_string());
            let has_marker = scan_line[..full.start()].ends_with(&config.marker);
            // §FS-workspace.1, §AR-workspace.3.1: an unmarked `alias/ID` is text,
            // not a citation. The slash is part of the visual token; we do not
            // fall back to recognising the trailing ID as a bare citation.
            if namespace.is_some() && !has_marker {
                continue;
            }
            if config.strict && !has_marker {
                continue;
            }
            if !is_md && !has_marker && is_inside_string_literal(scan_line, full.start()) {
                continue;
            }
            let Some(id) = parse_id(&caps) else { continue };
            if let Some(decl) = current.as_ref()
                && decl.line == lineno
                && decl.id == id
            {
                continue;
            }
            let start = if has_marker {
                full.start().saturating_sub(config.marker.len())
            } else {
                full.start()
            };
            let text = scan_line[start..full.end()].to_string();
            findings.citations.push(Citation {
                namespace,
                id,
                section: caps.name("sec").map(|m| m.as_str().to_string()),
                file: path.to_path_buf(),
                line: lineno,
                column: start + 1,
                has_marker,
                text,
            });
        }
    }

    if let Some(decl) = current.take() {
        findings
            .declarations
            .entry(decl.id.clone())
            .or_default()
            .push(decl);
    }
    Ok(())
}

/// Discover `e2e/cases/<name>/` directories and register each as an `E2E-<name>`
/// declaration whose body is the case manifest (§AR-scanner.6, §FS-show.2.4) — so
/// `grund check` sees `§E2E-…` citations resolve and `grund refs` finds e2e tests.
fn scan_e2e_cases(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
    findings: &mut Findings,
) -> Result<()> {
    let Some(kind) = config.kinds.iter().find(|kind| kind.prefix == "E2E") else {
        return Ok(());
    };
    let Some(folder) = kind.folder.as_deref() else {
        return Ok(());
    };
    let cases_root = config.root.join(folder);
    if !cases_root.exists() || !cases_root.is_dir() {
        return Ok(());
    }
    let cases_root = fs::canonicalize(&cases_root).unwrap_or(cases_root);
    let mut scan_root = cases_root.clone();

    if explicit_scope {
        let scope = scope.unwrap_or(Path::new("."));
        if scope.is_file() {
            return Ok(());
        }
        let scope = fs::canonicalize(scope).unwrap_or_else(|_| scope.to_path_buf());
        if scope.starts_with(&cases_root) {
            scan_root = scope;
        } else if !cases_root.starts_with(&scope) {
            return Ok(());
        }
    } else if let Some(include) = &config.include {
        let covered = include.iter().any(|path| {
            let root = config.root.join(path);
            cases_root.starts_with(&root) || root.starts_with(&cases_root)
        });
        if !covered {
            return Ok(());
        }
    }

    let mut case_dirs = Vec::new();
    if scan_root.join("expected.exit").is_file() {
        case_dirs.push(scan_root);
    } else {
        for entry in fs::read_dir(&scan_root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.join("expected.exit").is_file() {
                case_dirs.push(path);
            }
        }
    }
    case_dirs.sort_by_key(|path| sort_path_key(path));

    for dir in case_dirs {
        let Some(name) = dir.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(id) = e2e_id_from_case_dir_name(config, name) else {
            continue;
        };
        let case = read_e2e_case(&dir)?;
        findings
            .declarations
            .entry(id.clone())
            .or_default()
            .push(Declaration {
                id,
                file: dir.clone(),
                line: 1,
                heading_level: 1,
                sections: BTreeMap::new(),
                is_stub: false,
                defined_in: None,
                e2e_case: Some(case),
                title: Some(format!("e2e case `{name}`")),
            });
    }
    Ok(())
}

/// Map an `e2e/cases/<name>/` directory name to its `E2E-<name>` `Id` under the
/// repo's `[id] format` (§AR-scanner.6, §FS-config.3.4).
fn e2e_id_from_case_dir_name(config: &Config, name: &str) -> Option<Id> {
    let after_kind_literal = literal_after_kind_placeholder(&config.id_format)?;
    let raw = format!("E2E{after_kind_literal}{name}");
    let (id, section) = parse_id_arg(&raw, &config.grammar).ok()?;
    if section.is_none() && id.kind == "E2E" {
        Some(id)
    } else {
        None
    }
}

/// The literal text between `{kind}` and the next placeholder in `[id] format`
/// (e.g. `-` in `{kind}-{slug}`) — the glue an `E2E-<dirname>` ID is reassembled
/// with (§AR-scanner.6).
fn literal_after_kind_placeholder(format: &str) -> Option<&str> {
    let marker = "{kind}";
    let start = format.find(marker)? + marker.len();
    let rest = &format[start..];
    let end = rest.find('{').unwrap_or(rest.len());
    Some(&rest[..end])
}

/// Inverse of `e2e_id_from_case_dir_name`: strip the `E2E` prefix off a rendered ID
/// to get the `e2e/cases/<name>/` directory `grund id` tells the author to create
/// (§FS-id.2, §AR-scanner.6).
fn e2e_case_dir_name(config: &Config, rendered: &str) -> String {
    let prefix = format!(
        "E2E{}",
        literal_after_kind_placeholder(&config.id_format).unwrap_or("-")
    );
    rendered
        .strip_prefix(&prefix)
        .unwrap_or(rendered)
        .to_string()
}

/// Read one e2e case directory into an `E2eCase` — `command.args` (defaulting to
/// `check`), `expected.exit`, and the recursive fixture file list — the data
/// `grund show E2E-<name>` renders (§FS-show.2.4).
fn read_e2e_case(dir: &Path) -> Result<E2eCase> {
    let command_args = dir.join("command.args");
    let args = if command_args.is_file() {
        fs::read_to_string(&command_args)?
            .split_whitespace()
            .map(str::to_string)
            .collect()
    } else {
        vec!["check".to_string()]
    };
    let expected_exit = fs::read_to_string(dir.join("expected.exit"))?
        .trim()
        .parse::<i32>()
        .with_context(|| format!("parse {}/expected.exit", format_path(dir)))?;
    let mut fixtures = Vec::new();
    collect_relative_fixture_files(dir, dir, &mut fixtures)?;
    fixtures.sort_by_key(|path| sort_path_key(path));
    Ok(E2eCase {
        dir: dir.to_path_buf(),
        args,
        expected_exit,
        fixtures,
    })
}

fn collect_relative_fixture_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_relative_fixture_files(root, &path, files)?;
        } else {
            files.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}

/// Depth of a heading line — count of leading `#` — used to decide whether a
/// section heading nests under the current declaration (§AR-scanner.2.2).
fn heading_level_for_line(line: &str, markdown_heading: bool, caps: &regex::Captures) -> usize {
    if markdown_heading {
        return line
            .trim_start()
            .chars()
            .take_while(|ch| *ch == '#')
            .count()
            .max(1);
    }
    // Code-form declarations (§DF-code-declarations-drop-hash) match the alternation
    // branch that has no `#+`, so neither named group is set; default to depth 1.
    // Markdown-form declarations inside a doc-comment land in either `hashes`
    // (after a comment prefix) or `mdhashes` (no prefix at this byte position).
    caps.name("hashes")
        .or_else(|| caps.name("mdhashes"))
        .map(|m| m.as_str().len())
        .unwrap_or(1)
}

/// The tree walk (§AR-scanner.1): from each scan root, descend skipping hidden and
/// `[scan] exclude` directories, honouring `.gitignore` and friends unless
/// `respect_gitignore = false` (§AR-scanner.1.1, §FS-config.3.5), keeping only
/// scannable files, in a sorted order so findings are deterministic (§FS-errors.4).
fn walk_scannable_files(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
) -> Result<Vec<PathBuf>> {
    let roots = scan_roots(config, scope, explicit_scope)?;
    let mut files = Vec::new();
    for scan_root in roots {
        if !scan_root.exists() {
            continue;
        }
        let canonical_scan_root =
            fs::canonicalize(&scan_root).unwrap_or_else(|_| scan_root.to_path_buf());
        // §AR-workspace.6: a root scan starts outside member namespaces; an
        // included path at or below a member boundary belongs to the member scan.
        if config
            .workspace_boundary_roots
            .iter()
            .any(|root| canonical_scan_root.starts_with(root))
        {
            continue;
        }
        if scan_root.is_file() {
            if is_scannable(&scan_root, config) {
                files.push(scan_root);
            }
            continue;
        }
        let mut builder = WalkBuilder::new(&scan_root);
        builder.hidden(false);
        if !config.respect_gitignore {
            builder
                .ignore(false)
                .git_ignore(false)
                .git_global(false)
                .git_exclude(false)
                .parents(false);
        }
        let excluded = config.exclude.clone();
        // §AR-workspace.6: precompute the boundary path components once,
        // expressed relative to the canonical scan root. The walker filter is
        // then a single component-suffix compare — no per-entry `canonicalize`
        // syscall, no allocation in the hot path. The suffix derived from
        // `canonical_scan_root` is identical to the one derived from
        // `scan_root` because `strip_prefix` only removes the root; the
        // descendant portion is invariant under symlink resolution. So the
        // inner walker compare against `scan_root_for_filter` matches the
        // suffix table cleanly even when `scan_root` itself is a symlink.
        let boundary_suffixes: Vec<PathBuf> = config
            .workspace_boundary_roots
            .iter()
            .filter_map(|root| root.strip_prefix(&canonical_scan_root).ok())
            .map(Path::to_path_buf)
            .collect();
        let scan_root_for_filter = scan_root.clone();
        builder.filter_entry(move |e| {
            if e.depth() == 0 {
                return true;
            }
            if e.file_type().is_some_and(|file_type| file_type.is_dir())
                && let Ok(relative) = e.path().strip_prefix(&scan_root_for_filter)
                && boundary_suffixes
                    .iter()
                    .any(|suffix| relative == suffix.as_path())
            {
                return false;
            }
            if e.file_type().is_some_and(|file_type| file_type.is_dir()) {
                let Some(name) = e.path().file_name().and_then(|name| name.to_str()) else {
                    return true;
                };
                return !name.starts_with('.') && !excluded.iter().any(|item| item == name);
            }
            true
        });
        let walker = builder.build();
        for entry in walker {
            let entry = entry?;
            if !entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
                || !is_scannable(entry.path(), config)
            {
                continue;
            }
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort_by_key(|path| sort_path_key(path));
    Ok(files)
}

/// The directories (or single file) the walk starts from: a `[path]` argument when
/// given (narrowing the default scope), otherwise `[scan] include` resolved against
/// the repo root, otherwise the whole root (§FS-config.3.5, §AR-scanner.1).
fn scan_roots(config: &Config, scope: Option<&Path>, explicit_scope: bool) -> Result<Vec<PathBuf>> {
    if explicit_scope {
        let scope = scope.unwrap_or(Path::new("."));
        if !scope.exists() {
            return Err(anyhow!("path does not exist: {}", scope.display()));
        }
        let scope = fs::canonicalize(scope).unwrap_or_else(|_| scope.to_path_buf());
        if scope.is_file() {
            return Ok(vec![scope]);
        }
        if scope == config.root
            && let Some(include) = &config.include
        {
            return Ok(include.iter().map(|path| config.root.join(path)).collect());
        }
        return Ok(vec![scope]);
    }
    if let Some(include) = &config.include {
        Ok(include.iter().map(|path| config.root.join(path)).collect())
    } else {
        Ok(vec![config.root.clone()])
    }
}

/// A file that could not be read or decoded during the walk. The walk continues
/// past it (§FS-check.2); callers that are point queries treat any entry here as
/// fatal, `check` and `refs` report it and exit 2 with a still-printed report.
type ScanError = (PathBuf, String);

/// One full tree walk: scan every file (§AR-scanner.2) plus the e2e case
/// directories (§AR-scanner.6), collecting unreadable files rather than aborting
/// so `check` can report them and keep going (§FS-check.2).
fn scan_tree(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
) -> Result<(Findings, Vec<ScanError>)> {
    let mut findings = Findings::default();
    let mut errors = Vec::new();
    for file in walk_scannable_files(config, scope, explicit_scope)? {
        match scan_file(&file, config, &mut findings) {
            Ok(()) => findings.scanned_files.push(file),
            Err(err) => errors.push((file, format!("{err:#}"))),
        }
    }
    if let Err(err) = scan_e2e_cases(config, scope, explicit_scope, &mut findings) {
        errors.push((config.root.join("e2e/cases"), format!("{err:#}")));
    }
    Ok((findings, errors))
}

/// Scan helper for point-query subcommands (`show`, `id`): any unreadable file
/// is fatal — a partial view of the tree could miss the declaration entirely or
/// allocate a colliding number (§FS-show.3, §FS-id.4).
fn scan_tree_strict(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
) -> Result<Findings> {
    let (findings, errors) = scan_tree(config, scope, explicit_scope)?;
    if let Some((path, message)) = errors.into_iter().next() {
        return Err(anyhow!("{}: {}", display_path(config, &path), message));
    }
    Ok(findings)
}

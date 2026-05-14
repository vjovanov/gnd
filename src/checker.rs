/// AR-checker: how grund validates the scanner's findings
///
/// The checker takes the `Findings` produced by §AR-scanner and produces a
/// `Report`. It implements the rules in §FS-check.
///
/// ## 1. Inputs and outputs
///
/// - Input: `Findings` from the scanner, plus the repo root and config (needed
///   to resolve stub-link paths, to read the `AGENTS.md` init block, and to know
///   whether `[reference] require_grounding` is on).
/// - Output: a `Report` containing two ordered lists: `errors` and `warnings`.
///   Order is deterministic — sorted into the fixed report order of §FS-errors.4
///   and §FS-non-goals.9 — for §GOAL-friendliness-first.
///
/// ## 2. Rules
///
/// Each rule is a single pass over part of the findings. Rules are independent —
/// adding a rule does not force re-scanning.
///
/// ### 2.1 Duplicate declarations (§FS-check.3.3)
///
/// For each ID with more than one declaration, emit one error anchored at the
/// lexicographically-first site (sort by `path`, then `line`); list every other
/// site parenthetically in the message. This keeps the report's `path:line:`
/// prefix invariant (§3, §FS-check.2.1) while still naming all sites. A stub and
/// the inline declaration it points at count as one home, not two.
///
/// ### 2.2 Dangling citations (§FS-check.3.1)
///
/// For each citation whose ID has no declaration, emit one error at the citation
/// site.
///
/// ### 2.3 Missing sections (§FS-check.3.2)
///
/// For each citation with a section path, look up the section in the matching
/// declaration's recorded sections. Missing → one error at the citation site.
///
/// ### 2.4 Broken inline-spec stubs (§FS-check.3.4)
///
/// For each declaration whose H1 has the stub shape `# <ID>: [<text>](<path>)`
/// (description after the colon is a single bare markdown link), extract the link
/// target, resolve it against the repo root, verify the path exists, then re-scan
/// that file for an inline declaration of the same ID. Either failure → one error
/// at the stub site. This is the only rule that re-reads a file; everything else
/// comes from `findings`.
///
/// ### 2.5 Unused declarations (§FS-check.4.1)
///
/// For each declared ID never cited, emit one warning. Warnings do not cause a
/// non-zero exit. `E2E` declarations are exempt — a case is exercised by being
/// run, not by being cited (§FS-check.4.1).
///
/// ### 2.6 Invalid agent-entrypoint init block (§FS-check.3.5)
///
/// When `<root>/AGENTS.md` exists, verify its versioned `grund init` block (and the
/// matching block in any non-symlink companion entrypoint that is present): a
/// missing block, a malformed begin/end pair, an older version, or a newer
/// unsupported version is one error at the entrypoint's line.
///
/// ### 2.7 Ungrounded source files — opt-in (§FS-check.3.6, §DF-require-grounding)
///
/// When `[reference] require_grounding = true` (or `grund check --require-grounding`),
/// every scanned non-`.md` file must carry at least one recognised citation that
/// resolves, or itself declare an ID inline; a source file that does neither is
/// one error anchored at line 1. Off by default.
///
/// ## 3. Error format
///
/// Every error and warning follows `<path>:<line>: <message>` so that editors and
/// agents can jump to the source. There is no severity prefix, and there is no
/// aggregate summary footer — the exit code is the machine-readable verdict. This
/// is mandated by §GOAL-friendliness-first and §FS-check.2.1.
///
/// Findings without a single source location (CLI launch errors, malformed
/// configuration that prevents a scan from starting, a per-file read failure
/// mid-walk) are emitted on stderr as `error: <message>` per §FS-check.2.1.1,
/// distinguishable from per-finding lines by the leading `error:`.
///
/// ## 4. Why a separate stage from the scanner
///
/// The scanner produces a complete view of the world; the checker enforces rules
/// on that view. Keeping them separate means:
///
/// - New rules can be added without touching the scanner.
/// - The optional LSP server (§AR-lsp) can run a subset of checks (e.g., only
///   dangling references on the active file's citations) against a cached scan.
/// - Tests can feed synthetic `Findings` directly to the checker without disk I/O.
fn check(findings: &Findings, config: &Config) -> Report {
    let mut report = Report::default();
    // §FS-check.3.5: an `AGENTS.md` whose managed block is out of date (or newer
    // than this binary) is a check error.
    check_agents_block_version(&config.root, &mut report);

    // §FS-check.3.3: an ID with more than one non-stub home is a duplicate.
    for (id, decls) in &findings.declarations {
        let duplicate_homes: Vec<&Declaration> = decls
            .iter()
            .filter(|decl| !is_stub_for_inline_decl(&config.root, decl, decls))
            .collect();
        if duplicate_homes.len() > 1 {
            let mut sites: Vec<Site> = duplicate_homes
                .iter()
                .map(|d| Site {
                    path: d.file.clone(),
                    line: d.line,
                })
                .collect();
            sites.sort_by(|a, b| {
                (sort_path_key(&a.path), a.line).cmp(&(sort_path_key(&b.path), b.line))
            });
            let primary = sites[0].clone();
            let others = sites[1..]
                .iter()
                .map(|site| format!("{}:{}", display_path(config, &site.path), site.line))
                .collect::<Vec<_>>();
            let suffix = if others.is_empty() {
                String::new()
            } else {
                format!(" (also declared at {})", others.join(", "))
            };
            report.errors.push(Diagnostic {
                code: "duplicate",
                path: Some(primary.path),
                line: Some(primary.line),
                message: format!("duplicate declaration of {}{suffix}", render_id(config, id)),
                sites,
            });
        }
    }

    for cite in &findings.citations {
        // §FS-check.3.1: a citation whose ID is declared nowhere is dangling.
        let Some(decls) = findings.declarations.get(&cite.id) else {
            report.errors.push(Diagnostic {
                code: "dangling",
                path: Some(cite.file.clone()),
                line: Some(cite.line),
                message: format!("unknown reference {}", render_id(config, &cite.id)),
                sites: Vec::new(),
            });
            continue;
        };
        // §FS-check.3.2: the ID resolves but no declaration has a heading at the
        // cited section path.
        if let Some(sec) = &cite.section {
            let any_match = decls.iter().any(|d| d.sections.contains_key(sec));
            if !any_match {
                report.errors.push(Diagnostic {
                    code: "missing-section",
                    path: Some(cite.file.clone()),
                    line: Some(cite.line),
                    message: format!(
                        "missing section {}{}{}",
                        render_id(config, &cite.id),
                        config.section_separator,
                        sec
                    ),
                    sites: Vec::new(),
                });
            }
        }
    }

    // §FS-check.3.4: a `# <ID>: [text](path)` stub is broken if `path` does not
    // exist, or exists but does not itself declare `<ID>` inline (§AR-checker.2.4).
    for (id, decls) in &findings.declarations {
        for decl in decls {
            if !decl.is_stub {
                continue;
            }
            let Some(target) = &decl.defined_in else {
                continue;
            };
            let resolved = if target.is_absolute() {
                target.clone()
            } else {
                config.root.join(target)
            };
            if !resolved.exists() {
                report.errors.push(Diagnostic {
                    code: "broken-stub",
                    path: Some(decl.file.clone()),
                    line: Some(decl.line),
                    message: format!("stub link target missing: {}", format_path(target)),
                    sites: Vec::new(),
                });
                continue;
            }
            let inline_ok = if resolved.is_file() && is_scannable(&resolved, config) {
                file_declares_inline_home(&resolved, id, &config.grammar).unwrap_or(false)
            } else {
                false
            };
            if !inline_ok {
                report.errors.push(Diagnostic {
                    code: "broken-stub",
                    path: Some(decl.file.clone()),
                    line: Some(decl.line),
                    message: format!(
                        "stub link target lacks {}: {}",
                        render_id(config, id),
                        format_path(target)
                    ),
                    sites: Vec::new(),
                });
            }
        }
    }

    // §FS-check.4.1: a declaration nothing cites is a warning, not an error —
    // except E2E cases, which are proof artifacts, not citation targets.
    let cited: BTreeSet<&Id> = findings.citations.iter().map(|c| &c.id).collect();
    for (id, decls) in &findings.declarations {
        if id.kind == "E2E" {
            continue;
        }
        if !cited.contains(id)
            && let Some(decl) = decls
                .iter()
                .find(|decl| !is_stub_for_inline_decl(&config.root, decl, decls))
                .or_else(|| decls.first())
        {
            report.warnings.push(Diagnostic {
                code: "unused",
                path: Some(decl.file.clone()),
                line: Some(decl.line),
                message: format!("declared but never cited: {}", render_id(config, id)),
                sites: Vec::new(),
            });
        }
    }

    // §FS-check.3.6 / §DF-require-grounding: under `[reference] require_grounding`,
    // every scanned source (non-Markdown) file must carry at least one citation to
    // a declared ID — or itself declare one inline (a spec home is grounded in the
    // spec it *is*). Pure function of (tree, config): no git, no AST.
    if config.require_grounding {
        // Collect the files that ground themselves in two linear passes — one over
        // citations, one over declarations — so the per-file test below is a set
        // lookup, not a re-scan of every citation and declaration for each file
        // (§GOAL-fast-feedback: speed is the ordering principle).
        let mut grounded_files: BTreeSet<&Path> = findings
            .citations
            .iter()
            .filter(|cite| findings.declarations.contains_key(&cite.id))
            .map(|cite| cite.file.as_path())
            .collect();
        grounded_files.extend(
            findings
                .declarations
                .values()
                .flatten()
                .filter(|decl| !decl.is_stub && decl.e2e_case.is_none())
                .map(|decl| decl.file.as_path()),
        );
        for file in &findings.scanned_files {
            if file.extension().and_then(|ext| ext.to_str()) == Some("md") {
                continue;
            }
            if !grounded_files.contains(file.as_path()) {
                report.errors.push(Diagnostic {
                    code: "ungrounded",
                    path: Some(file.clone()),
                    line: Some(1),
                    message: format!(
                        "ungrounded source file: no {} citation to a declared ID",
                        config.marker
                    ),
                    sites: Vec::new(),
                });
            }
        }
    }

    sort_diagnostics(&mut report.errors);
    sort_diagnostics(&mut report.warnings);
    report
}

/// Put diagnostics in the one fixed order `grund` ever prints them in — by path, then
/// line, then message text — so two runs over the same tree agree byte-for-byte
/// (§FS-errors.4) and ordering is not a knob (§FS-non-goals.9).
fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by(diagnostic_cmp);
}

fn diagnostic_cmp(a: &Diagnostic, b: &Diagnostic) -> std::cmp::Ordering {
    (
        a.path.as_ref().map(|p| sort_path_key(p)),
        a.line.unwrap_or(0),
        a.message.as_str(),
    )
        .cmp(&(
            b.path.as_ref().map(|p| sort_path_key(p)),
            b.line.unwrap_or(0),
            b.message.as_str(),
        ))
}

/// Validate the managed agent-entrypoint blocks (§FS-check.3.5): the begin/end
/// marker pair must be present and intact, and the `vN` version must match this
/// binary — an older `vN` is "run `grund init`" (§FS-init.2.3), a newer one is
/// fatal. `AGENTS.md` is canonical; known companion entrypoints are checked only
/// when present and not symlinked to `AGENTS.md`.
fn check_agents_block_version(root: &Path, report: &mut Report) {
    let canonical = root.join("AGENTS.md");
    if !canonical.exists() {
        return;
    }
    let mut paths = vec![canonical];
    match companion_agent_entrypoints(root) {
        Ok(companions) => paths.extend(companions),
        Err((path, message)) => {
            report.errors.push(Diagnostic {
                code: "io",
                path: Some(path),
                line: Some(1),
                message,
                sites: Vec::new(),
            });
        }
    }
    for path in paths {
        check_agent_block_path(&path, report);
    }
}

fn check_agent_block_path(path: &Path, report: &mut Report) {
    if !path.exists() {
        return;
    }
    let Ok(text) = fs::read_to_string(path) else {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("agent entrypoint");
        report.errors.push(Diagnostic {
            code: "io",
            path: Some(path.to_path_buf()),
            line: Some(1),
            message: format!("cannot read {file_name}"),
            sites: Vec::new(),
        });
        return;
    };
    if let Some(block) = find_agents_block(&text) {
        let line = line_for_byte_index(&text, block.start);
        if block.version < AGENTS_BLOCK_VERSION {
            report.errors.push(Diagnostic {
                code: "agents-init",
                path: Some(path.to_path_buf()),
                line: Some(line),
                message: format!(
                    "outdated grund init block v{} (run `grund init` to update to v{})",
                    block.version, AGENTS_BLOCK_VERSION
                ),
                sites: Vec::new(),
            });
        } else if block.version > AGENTS_BLOCK_VERSION {
            report.errors.push(Diagnostic {
                code: "agents-init",
                path: Some(path.to_path_buf()),
                line: Some(line),
                message: format!(
                    "unsupported grund init block v{} (this grund supports v{})",
                    block.version, AGENTS_BLOCK_VERSION
                ),
                sites: Vec::new(),
            });
        }
        return;
    }
    if AGENTS_BLOCK_LEGACY_BEGIN.is_match(&text) {
        let line = AGENTS_BLOCK_LEGACY_BEGIN
            .find(&text)
            .map(|m| line_for_byte_index(&text, m.start()))
            .unwrap_or(1);
        report.errors.push(Diagnostic {
            code: "agents-init",
            path: Some(path.to_path_buf()),
            line: Some(line),
            message: "malformed grund init block".to_string(),
            sites: Vec::new(),
        });
    } else {
        report.errors.push(Diagnostic {
            code: "agents-init",
            path: Some(path.to_path_buf()),
            line: Some(1),
            message: format!("missing grund init block v{}", AGENTS_BLOCK_VERSION),
            sites: Vec::new(),
        });
    }
}

fn line_for_byte_index(text: &str, byte_index: usize) -> usize {
    text[..byte_index]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

/// Whether this stub heading is the one-line pointer to an inline declaration in
/// code (`# <ID>: [text](src/foo.rs)` whose target also declares `<ID>`) — such a
/// stub does not count as a second home, so it is not a duplicate (§AR-scanner.4,
/// §FS-show.2.3).
fn is_stub_for_inline_decl(root: &Path, decl: &Declaration, decls: &[Declaration]) -> bool {
    if !decl.is_stub {
        return false;
    }
    let Some(target) = &decl.defined_in else {
        return false;
    };
    let resolved = if target.is_absolute() {
        target.clone()
    } else {
        root.join(target)
    };
    decls
        .iter()
        .any(|other| other.file == resolved && other.file != decl.file)
}

/// Whether `path` contains a real (non-stub) `# <ID>: …` declaration of `id` —
/// the check that a stub's link target actually carries the inline home it claims
/// (§FS-check.3.4, §AR-checker.2.4, §AR-scanner.4).
fn file_declares_inline_home(path: &Path, id: &Id, grammar: &Grammar) -> Result<bool> {
    let text = fs::read_to_string(path)?;
    for line in text.lines() {
        if let Some(caps) = grammar.decl_re.captures(line)
            && let Some(found) = parse_id(&caps)
            && &found == id
        {
            let tail = &line[caps.get(0).unwrap().end()..];
            if STUB_LINK_HEADING.is_match(tail) {
                continue;
            }
            return Ok(true);
        }
    }
    Ok(false)
}


/// `grund init [path] [--name N] [--docs] [--force|--append]` — scaffold a repo for
/// `grund` (§FS-init.1): write `AGENTS.md` and `.agents/grund.toml` (and, with
/// `--docs`, the `docs/`+`e2e/` tree, §FS-init.2.1), append/update the managed
/// `AGENTS.md` block when the file already exists (§FS-init.2.3), refuse to clobber
/// edited scaffold files without `--force` — and never overwrite an existing
/// `.agents/grund.toml` even with `--force`, since that file is the user's config
/// (§FS-init.3) — print a `next:` block, and exit `2` on a missing target / CLI
/// error / unsupported block version (§FS-init.4). Non-interactive — every choice
/// is a flag (§FS-non-goals.10).
fn command_init(args: &[String]) -> ExitCode {
    let mut path: Option<PathBuf> = None;
    let mut name: Option<String> = None;
    let mut docs = false;
    let mut force = false;
    let mut append = false;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--docs" => docs = true,
            "--force" => force = true,
            "--append" => append = true,
            "--name" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --name requires a value");
                    return ExitCode::from(2);
                }
                name = Some(args[idx].clone());
            }
            other if other.starts_with("--name=") => {
                name = Some(other.trim_start_matches("--name=").to_string());
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other => {
                if path.is_some() {
                    eprintln!("error: init takes at most one path argument");
                    return ExitCode::from(2);
                }
                path = Some(PathBuf::from(other));
            }
        }
        idx += 1;
    }

    if force && append {
        eprintln!("error: --force and --append cannot be used together");
        return ExitCode::from(2);
    }

    let target = path.unwrap_or_else(|| PathBuf::from("."));
    if !target.exists() {
        eprintln!(
            "error: target directory does not exist: {}",
            target.display()
        );
        return ExitCode::from(2);
    }
    if !target.is_dir() {
        eprintln!("error: target is not a directory: {}", target.display());
        return ExitCode::from(2);
    }

    let resolved_name = match name {
        Some(value) => value,
        None => match derive_default_name(&target) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("error: {err}");
                return ExitCode::from(2);
            }
        },
    };

    // §FS-init.2.3: render `AGENTS.md` against the config `init` leaves in place,
    // so the ID-shape / kind / marker prose in it matches `.agents/grund.toml`.
    let init_config = init_effective_config(&target);

    let agents_contents = render_agents_md(&resolved_name, &init_config);
    let agents_block = render_agents_append_block(&resolved_name, &init_config);

    if !write_or_update_canonical_agent_entrypoint(
        &target,
        CANONICAL_AGENT_ENTRYPOINT,
        &agents_contents,
        &agents_block,
        force,
    ) {
        return ExitCode::from(2);
    }

    let companion_entrypoints = match init_companion_agent_entrypoints(&target) {
        Ok(paths) => paths,
        Err((path, message)) => {
            eprintln!("error: inspect {}: {message}", path.display());
            return ExitCode::from(2);
        }
    };
    for entrypoint in companion_entrypoints {
        let path_ref = match &entrypoint {
            InitCompanionAgentEntrypoint::Existing(path)
            | InitCompanionAgentEntrypoint::MissingAlias(path) => path.as_path(),
        };
        let rel = path_ref.strip_prefix(&target).unwrap_or(path_ref).to_path_buf();
        let rel = format_path(&rel);
        match entrypoint {
            InitCompanionAgentEntrypoint::Existing(path) => {
                match update_agents_block(&path, &agents_block, &rel) {
                    Ok(AgentsUpdateResult::Appended) => eprintln!("appended {rel}"),
                    Ok(AgentsUpdateResult::Updated) => eprintln!("updated {rel}"),
                    Ok(AgentsUpdateResult::Unchanged) => eprintln!("exists {rel}"),
                    Err(err) => {
                        eprintln!("error: update {}: {err}", path.display());
                        return ExitCode::from(2);
                    }
                }
            }
            InitCompanionAgentEntrypoint::MissingAlias(path) => {
                if let Some(parent) = path.parent()
                    && let Err(err) = fs::create_dir_all(parent)
                {
                    eprintln!("error: create {}: {err}", parent.display());
                    return ExitCode::from(2);
                }
                if let Err(err) = fs::write(&path, &agents_block) {
                    eprintln!("error: write {}: {err}", path.display());
                    return ExitCode::from(2);
                }
                eprintln!("wrote {rel}");
            }
        }
    }

    // `.agents/grund.toml` is the project's configuration — the surface a repo
    // customizes (kinds, marker, scan scope, …, §GOAL-configurable). `init` writes
    // the canonical template only when it is **absent**; an existing config is never
    // overwritten, not even with `--force`. `--force` targets the things `init`
    // owns end to end — the managed `AGENTS.md` block and the `--docs` scaffold
    // stubs — not the user's settings (§FS-init.3).
    let config_rel = ".agents/grund.toml";
    let config_dest = target.join(config_rel);
    if config_dest.exists() {
        eprintln!("exists {config_rel}");
    } else {
        if let Some(parent) = config_dest.parent()
            && let Err(err) = fs::create_dir_all(parent)
        {
            eprintln!("error: create {}: {err}", parent.display());
            return ExitCode::from(2);
        }
        if let Err(err) = fs::write(&config_dest, render_grund_toml(&resolved_name)) {
            eprintln!("error: write {}: {err}", config_dest.display());
            return ExitCode::from(2);
        }
        eprintln!("wrote {config_rel}");
    }

    let files: Vec<(&'static str, String)> = if docs { docs_scaffold() } else { Vec::new() };
    for (rel, contents) in &files {
        let dest = target.join(rel);
        if !force && dest.exists() {
            eprintln!("exists {rel}");
            continue;
        }
        if let Some(parent) = dest.parent()
            && let Err(err) = fs::create_dir_all(parent)
        {
            eprintln!("error: create {}: {err}", parent.display());
            return ExitCode::from(2);
        }
        if let Err(err) = fs::write(&dest, contents) {
            eprintln!("error: write {}: {err}", dest.display());
            return ExitCode::from(2);
        }
        eprintln!("wrote {rel}");
    }

    eprintln!();
    eprintln!("next:");
    if docs {
        eprintln!("  1. run `grund check` — a freshly scaffolded tree is clean");
        eprintln!(
            "  2. allocate an ID:  ID=$(grund id FS \"…\")  then write  docs/functional-spec/$ID.md"
        );
        eprintln!("     (H1: `# <ID>: <one-line statement of the behavior>`)");
        eprintln!(
            "  3. cite it as §<ID> from the docs and e2e tests that depend on it, then `grund check` again"
        );
    } else {
        eprintln!(
            "  1. re-run with --docs to scaffold docs/ and e2e/ (or create those folders yourself) — until then `grund check` has nothing to scan"
        );
        eprintln!("  2. run `grund check` — a scaffolded tree is clean");
        eprintln!(
            "  3. allocate an ID:  ID=$(grund id FS \"…\")  then write  docs/functional-spec/$ID.md"
        );
    }
    eprintln!("see AGENTS.md for the full workflow.");

    ExitCode::SUCCESS
}

/// What `init` did to an existing `AGENTS.md`'s managed block — `appended ` (no
/// block before), `updated ` (a supported block whose bytes changed: an older
/// block upgraded, or a same-version block re-rendered against a changed
/// template or config), or `unchanged` (a supported block already byte-identical
/// to the current render — `init` rewrites nothing, §FS-init.2.2/§FS-init.2.3,
/// and reports it with the `exists ` prefix like any other untouched file).
#[derive(Debug, Eq, PartialEq)]
enum AgentsUpdateResult {
    Appended,
    Updated,
    Unchanged,
}

fn write_or_update_canonical_agent_entrypoint(
    target: &Path,
    rel: &str,
    contents: &str,
    block: &str,
    force: bool,
) -> bool {
    let dest = target.join(rel);
    if !force && dest.exists() {
        match update_agents_block(&dest, block, rel) {
            Ok(AgentsUpdateResult::Appended) => eprintln!("appended {rel}"),
            Ok(AgentsUpdateResult::Updated) => eprintln!("updated {rel}"),
            Ok(AgentsUpdateResult::Unchanged) => eprintln!("exists {rel}"),
            Err(err) => {
                eprintln!("error: update {}: {err}", dest.display());
                return false;
            }
        }
        return true;
    }
    if let Some(parent) = dest.parent()
        && let Err(err) = fs::create_dir_all(parent)
    {
        eprintln!("error: create {}: {err}", parent.display());
        return false;
    }
    if let Err(err) = fs::write(&dest, contents) {
        eprintln!("error: write {}: {err}", dest.display());
        return false;
    }
    eprintln!("wrote {rel}");
    true
}

/// Append or update the managed block in an existing agent entrypoint on disk
/// (§FS-init.2.3). A supported block is re-rendered from the current
/// template/config even when the schema version already matches — but when that
/// re-render is byte-identical to what is on disk the file is left untouched
/// (`Unchanged`, reported as `exists `), so re-running `grund init` on an
/// up-to-date repo writes nothing (§FS-init.2.2).
fn update_agents_block(dest: &Path, block: &str, label: &str) -> Result<AgentsUpdateResult> {
    let existing = fs::read_to_string(dest)?;
    let (updated, result) = update_agents_text(&existing, block, label)?;
    if result != AgentsUpdateResult::Unchanged {
        fs::write(dest, updated)?;
    }
    Ok(result)
}

/// The pure string transform behind `update_agents_block`: splice the current
/// managed block into `existing`, preserving everything outside it byte-for-byte
/// — including the block's position and any CRLF endings (§FS-init.2.3.1,
/// §FS-init.2.3.2). Returns `Unchanged` when the splice would reproduce
/// `existing` exactly. A newer-than-supported block is an error.
fn update_agents_text(
    existing: &str,
    block: &str,
    label: &str,
) -> Result<(String, AgentsUpdateResult)> {
    if let Some(existing_block) = find_agents_block(existing) {
        if existing_block.version > AGENTS_BLOCK_VERSION {
            return Err(anyhow!(
                "{label} contains newer grund init block v{}; this binary supports v{}",
                existing_block.version,
                AGENTS_BLOCK_VERSION
            ));
        }
        let mut updated = String::with_capacity(existing.len() + block.len());
        updated.push_str(&existing[..existing_block.start]);
        updated.push_str(block);
        updated.push_str(&existing[existing_block.end..]);
        let result = if updated == existing {
            AgentsUpdateResult::Unchanged
        } else {
            AgentsUpdateResult::Updated
        };
        return Ok((updated, result));
    }

    let separator = if existing.is_empty() || existing.ends_with("\n\n") {
        ""
    } else if existing.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    let mut updated = String::with_capacity(existing.len() + separator.len() + block.len());
    updated.push_str(existing);
    updated.push_str(separator);
    updated.push_str(block);
    Ok((updated, AgentsUpdateResult::Appended))
}

/// The byte span and `vN` version of the managed block inside an `AGENTS.md`
/// (§FS-init.2.3) — what both `grund init`'s update and `grund check`'s validation
/// (§FS-check.3.5) key off.
struct AgentsBlock {
    start: usize,
    end: usize,
    version: u32,
}

/// Locate the managed block in `AGENTS.md`. The current marker is an H2 line
/// (`## Grounding with grund (vN)`); the block runs until the next H1 or H2 (or
/// EOF) (§FS-init.2.3).
fn find_agents_block(text: &str) -> Option<AgentsBlock> {
    if let Some(caps) = AGENTS_BLOCK_H2.captures(text) {
        let begin_match = caps.get(0)?;
        let version = caps.name("version")?.as_str().parse::<u32>().ok()?;
        let after = begin_match.end();
        let section_end = AGENTS_SECTION_BOUNDARY
            .find_at(text, after)
            .map(|m| m.start())
            .unwrap_or(text.len());
        // Trailing blank lines before the next section are inter-section spacing,
        // not part of the managed body. Trim them back so a re-render of the same
        // content is a no-op (`exists `, §FS-init.2.3.1).
        let mut end = section_end;
        while end > after && text[..end].ends_with("\n\n") {
            end -= 1;
        }
        return Some(AgentsBlock {
            start: begin_match.start(),
            end,
            version,
        });
    }
    None
}

/// The default project name when `--name` is omitted: the basename of `<path>`
/// resolved to an absolute path (§FS-init.1).
fn derive_default_name(target: &Path) -> Result<String> {
    let absolute =
        fs::canonicalize(target).with_context(|| format!("resolve {}", target.display()))?;
    absolute
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .ok_or_else(|| anyhow!("cannot derive project name from {}", absolute.display()))
}

/// The `--docs` scaffold: the canonical `docs/` tree (stub `grund.md`,
/// `goals.md`, `roadmap.md`, `changelog.md`, the two spec READMEs, the
/// decision `.gitkeep`s) plus an empty `e2e/` with a README — the file list of
/// §FS-init.2.1, each a minimal starter that leaves `grund check` clean.
fn docs_scaffold() -> Vec<(&'static str, String)> {
    vec![
        ("docs/grund.md", canonical_template_text(GRUND_DOC_TEMPLATE)),
        (
            "docs/goals.md",
            canonical_template_text(GOALS_TEMPLATE),
        ),
        (
            "docs/roadmap.md",
            "# Roadmap\n\n<!-- placeholder - replace with real content -->\n".to_string(),
        ),
        (
            "docs/changelog.md",
            "# Changelog\n\n<!-- placeholder - replace with real content -->\n".to_string(),
        ),
        (
            "docs/functional-spec/README.md",
            canonical_template_text(FS_README_TEMPLATE),
        ),
        (
            "docs/architecture/README.md",
            canonical_template_text(AS_README_TEMPLATE),
        ),
        (
            "docs/decisions/architectural/.gitkeep",
            canonical_template_text(GITKEEP_TEMPLATE),
        ),
        (
            "docs/decisions/functional/.gitkeep",
            canonical_template_text(GITKEEP_TEMPLATE),
        ),
        (
            "e2e/README.md",
            canonical_template_text(E2E_README_TEMPLATE),
        ),
        (
            "e2e/cases/.gitkeep",
            canonical_template_text(GITKEEP_TEMPLATE),
        ),
    ]
}

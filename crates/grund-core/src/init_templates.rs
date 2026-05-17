// The scaffold templates `grund init` writes are embedded in the binary; the
// reference copies live under `templates/` in the source tree (§FS-init.2.1).
const AGENTS_TEMPLATE: &str = include_str!("../assets/templates/AGENTS.md");
const GRUND_TOML_TEMPLATE: &str = include_str!("../assets/templates/grund.toml");
const GRUND_DOC_TEMPLATE: &str = include_str!("../assets/templates/grund.md");
const GOALS_TEMPLATE: &str = include_str!("../assets/templates/goals.md");
const E2E_README_TEMPLATE: &str = include_str!("../assets/templates/e2e-README.md");
const FS_README_TEMPLATE: &str = include_str!("../assets/templates/functional-spec-README.md");
const AS_README_TEMPLATE: &str = include_str!("../assets/templates/architecture-README.md");
const GITKEEP_TEMPLATE: &str = include_str!("../assets/templates/gitkeep.md");
const AGENT_SETUP_INSTRUCTIONS: &str = include_str!("../assets/skills/grund-init/SKILL.md");
const AGENTS_BLOCK_VERSION: u32 = 1;
const CANONICAL_AGENT_ENTRYPOINT: &str = "AGENTS.md";
const COMPANION_AGENT_ENTRYPOINTS: &[CompanionAgentEntrypoint] = &[
    CompanionAgentEntrypoint {
        rel: "AGENTS.override.md",
        workspace: None,
        agent: None,
    },
    CompanionAgentEntrypoint {
        rel: "CLAUDE.md",
        workspace: Some(".claude"),
        agent: Some(AgentEntrypoint::Claude),
    },
    CompanionAgentEntrypoint {
        rel: ".claude/CLAUDE.md",
        workspace: Some(".claude"),
        agent: Some(AgentEntrypoint::Claude),
    },
    CompanionAgentEntrypoint {
        rel: "GEMINI.md",
        workspace: Some(".gemini"),
        agent: Some(AgentEntrypoint::Gemini),
    },
    CompanionAgentEntrypoint {
        rel: ".github/copilot-instructions.md",
        workspace: None,
        agent: Some(AgentEntrypoint::Copilot),
    },
];

#[derive(Clone, Copy, Eq, PartialEq)]
enum AgentEntrypoint {
    Claude,
    Gemini,
    Copilot,
}

#[derive(Default)]
struct InitAgentEntrypointSelection {
    canonical: bool,
    claude: bool,
    gemini: bool,
    copilot: bool,
}

impl InitAgentEntrypointSelection {
    fn any(&self) -> bool {
        self.canonical || self.claude || self.gemini || self.copilot
    }

    fn includes(&self, agent: AgentEntrypoint) -> bool {
        match agent {
            AgentEntrypoint::Claude => self.claude,
            AgentEntrypoint::Gemini => self.gemini,
            AgentEntrypoint::Copilot => self.copilot,
        }
    }
}

struct CompanionAgentEntrypoint {
    rel: &'static str,
    workspace: Option<&'static str>,
    agent: Option<AgentEntrypoint>,
}

enum InitCompanionAgentEntrypoint {
    Existing(PathBuf),
    MissingAlias(PathBuf),
}

fn canonical_template_text(template: &str) -> String {
    template.replace("\r\n", "\n").replace('\r', "\n")
}

/// The substitutions that turn `templates/AGENTS.md` into a concrete `AGENTS.md`
/// for a repo (§FS-init.2.3): the project name, plus the ID/marker shape taken
/// from the config `grund init` leaves in place — so a `{kind}-{slug}` repo gets a
/// `<KIND>-<slug>` description, a strict repo gets the strict-mode note, custom
/// kinds show up in the kind set, and so on. Everything *not* substituted here is
/// fixed for the block version. `{ID_SHAPE_SEC}` is listed before `{ID_SHAPE}`
/// only for readability; neither placeholder is a substring of the other.
fn agents_template_substitutions(name: &str, config: &Config) -> Vec<(&'static str, String)> {
    let sep = config.section_separator.as_str();
    let marker = config.marker.as_str();
    let id_shape = config
        .id_format
        .replace("{kind}", "<KIND>")
        .replace("{number}", "<NNN>")
        .replace("{slug}", "<slug>");
    let id_example = config
        .id_format
        .replace("{kind}", "FS")
        .replace("{number}", "042")
        .replace("{slug}", "user-login");
    let cite_example = format!("{marker}{id_example}{sep}3{sep}1");
    let kinds_set = format!("{{{}}}", kind_prefixes(&config.kinds).join(", "));
    let bare_note = if config.strict {
        format!(
            "Bare ID-shaped tokens are ignored — `[reference] strict = true` is set in `.agents/grund.toml`, so only `{marker}`-prefixed citations are checked."
        )
    } else {
        format!(
            "Bare ID-shaped tokens are also recognized as citations for backward compatibility; set `[reference] strict = true` in `.agents/grund.toml` to require the `{marker}` marker (run `grund fmt --marker` first to upgrade existing bare citations)."
        )
    };
    vec![
        ("{NAME}", name.to_string()),
        ("{ID_SHAPE_SEC}", format!("{id_shape}[{sep}<section>]")),
        ("{ID_SHAPE}", id_shape),
        ("{ID_EXAMPLE}", id_example),
        ("{CITE_EXAMPLE}", cite_example),
        ("{KINDS_SET}", kinds_set),
        ("{BARE_TOKEN_NOTE}", bare_note),
        ("{MARKER}", marker.to_string()),
        ("{TRIGGER}", config.trigger.clone()),
        ("{DECLARATION_MAP}", declaration_map(config)),
    ]
}

fn markdown_link_label(raw: &str) -> String {
    raw.replace('\\', r"\\")
        .replace('[', r"\[")
        .replace(']', r"\]")
}

fn markdown_link_destination(raw: &str) -> String {
    if raw
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '(' | ')' | '<' | '>'))
    {
        format!("<{}>", raw.replace('\\', r"\\").replace('>', r"\>"))
    } else {
        raw.to_string()
    }
}

fn declaration_map(config: &Config) -> String {
    let mut lines = Vec::new();
    for kind in &config.kinds {
        let prefix = markdown_link_label(&kind.prefix);
        let title = kind.title.as_deref().unwrap_or("Declaration");
        if let Some(home) = kind.file.as_deref().or(kind.folder.as_deref()) {
            lines.push(format!(
                "- [{prefix}]({}): {title}",
                markdown_link_destination(home)
            ));
        } else {
            lines.push(format!(
                "- `{}`: {title} (inline / configured by convention)",
                kind.prefix.replace('`', "\\`")
            ));
        }
    }
    lines.join("\n")
}

/// The managed block — just the H2 section that `init` appends to, or replaces
/// inside, an existing `AGENTS.md` (§FS-init.2.3). The template *is* the block;
/// the H2 line carrying the version is its own begin marker (§FS-init.2.3.1).
fn render_agents_append_block(name: &str, config: &Config) -> String {
    let mut rendered = canonical_template_text(AGENTS_TEMPLATE);
    for (placeholder, value) in agents_template_substitutions(name, config) {
        rendered = rendered.replace(placeholder, &value);
    }
    rendered
}

/// The full generated `AGENTS.md` for a fresh repo — the H1 scaffolding line
/// followed by the managed block (§FS-init.2.3). The H1 is *unmanaged* — `init`
/// owns the block, not the title. Deterministic: same `grund` version, same
/// `--name`, same effective config ⇒ byte-identical output (§FS-non-goals.13).
fn render_agents_md(name: &str, config: &Config) -> String {
    format!(
        "# {name} — agent instructions\n\n{}",
        render_agents_append_block(name, config)
    )
}

/// Existing companion agent entrypoints that should carry the same managed grund
/// block as `AGENTS.md` (§FS-init.2.1). A symlink to `AGENTS.md` is already
/// covered by the canonical file and is intentionally skipped.
fn companion_agent_entrypoints(root: &Path) -> Result<Vec<PathBuf>, (PathBuf, String)> {
    let mut paths = Vec::new();
    let canonical = root.join(CANONICAL_AGENT_ENTRYPOINT);
    for entrypoint in COMPANION_AGENT_ENTRYPOINTS {
        let path = root.join(entrypoint.rel);
        if !is_file_or_symlink(&path) {
            continue;
        }
        match is_symlink_to(&path, &canonical) {
            Ok(true) => continue,
            Ok(false) => paths.push(path),
            Err(err) => return Err((path, format!("{err:#}"))),
        }
    }
    Ok(paths)
}

/// Companion entrypoints `grund init` should update or create (§FS-init.2.1).
/// Existing companions are updated in place.
fn existing_init_companion_agent_entrypoints(
    root: &Path,
) -> Result<Vec<InitCompanionAgentEntrypoint>, (PathBuf, String)> {
    let mut paths = Vec::new();
    let canonical = root.join(CANONICAL_AGENT_ENTRYPOINT);
    for entrypoint in COMPANION_AGENT_ENTRYPOINTS {
        let path = root.join(entrypoint.rel);
        if is_file_or_symlink(&path) {
            match is_symlink_to(&path, &canonical) {
                Ok(true) => continue,
                Ok(false) => paths.push(InitCompanionAgentEntrypoint::Existing(path)),
                Err(err) => return Err((path, format!("{err:#}"))),
            }
        }
    }
    Ok(paths)
}

/// Missing neutral aliases are created only when their owning agent-specific
/// workspace directory already exists; generic project metadata directories
/// remain existing-file-only.
fn workspace_init_companion_agent_entrypoints(root: &Path) -> Vec<InitCompanionAgentEntrypoint> {
    let mut paths = Vec::new();
    for entrypoint in COMPANION_AGENT_ENTRYPOINTS {
        let path = root.join(entrypoint.rel);
        if entrypoint
            .workspace
            .is_some_and(|workspace| root.join(workspace).is_dir())
            && !path.exists()
        {
            paths.push(InitCompanionAgentEntrypoint::MissingAlias(path));
        }
    }
    paths
}

/// Explicit agent flags create their requested companion entrypoints even when
/// the normal automatic detection would not choose them.
fn requested_init_companion_agent_entrypoints(
    root: &Path,
    selection: &InitAgentEntrypointSelection,
) -> Result<Vec<InitCompanionAgentEntrypoint>, (PathBuf, String)> {
    let mut paths = Vec::new();
    let canonical = root.join(CANONICAL_AGENT_ENTRYPOINT);
    for entrypoint in COMPANION_AGENT_ENTRYPOINTS {
        if !entrypoint.agent.is_some_and(|agent| selection.includes(agent)) {
            continue;
        }
        let path = root.join(entrypoint.rel);
        if is_file_or_symlink(&path) {
            match is_symlink_to(&path, &canonical) {
                Ok(true) => continue,
                Ok(false) => paths.push(InitCompanionAgentEntrypoint::Existing(path)),
                Err(err) => return Err((path, format!("{err:#}"))),
            }
        } else {
            paths.push(InitCompanionAgentEntrypoint::MissingAlias(path));
        }
    }
    Ok(paths)
}

fn is_file_or_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type())
        .is_ok_and(|t| t.is_file() || t.is_symlink())
}

fn is_symlink_to(path: &Path, target: &Path) -> Result<bool> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_symlink() {
        return Ok(false);
    }
    let link = fs::read_link(path)?;
    let resolved = if link.is_absolute() {
        link
    } else {
        path.parent().unwrap_or_else(|| Path::new(".")).join(link)
    };
    Ok(normalize_path_lexically(&resolved) == normalize_path_lexically(target))
}

/// The config that `grund init` will leave governing `target`, which the generated
/// `AGENTS.md` must describe (§FS-init.2.3): an existing `target/.agents/grund.toml`
/// if there is one, otherwise the defaults (exactly what `init` is about to write
/// into `target/.agents/grund.toml`). We do **not** walk up to an ancestor's config
/// here — `init` always writes a config *in* `target`.
fn init_effective_config(target: &Path) -> Config {
    let local_config = target.join(".agents").join("grund.toml");
    if local_config.is_file() {
        load_config(target).unwrap_or_else(|_| Config::default_for(target.to_path_buf()))
    } else {
        Config::default_for(target.to_path_buf())
    }
}

/// The generated `.agents/grund.toml` — every default written out explicitly as a
/// teaching surface, with only `project_name` substituted (§FS-init.2.4).
fn render_grund_toml(name: &str) -> String {
    canonical_template_text(GRUND_TOML_TEMPLATE).replace("{NAME}", &escape_toml_basic(name))
}

fn escape_toml_basic(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('"', "\\\"")
}

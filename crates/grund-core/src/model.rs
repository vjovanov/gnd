/// A parsed ID: its kind plus whichever of `{number}` / `{slug}` the configured
/// `[id] format` carries (§FS-config.3.2).
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct Id {
    kind: String,
    num: Option<u32>,
    slug: Option<String>,
}

// `Id` is rendered for output via `render_id` / `format_id`, which honour the
// repo's `[id] format` and `--width` (§FS-config.3.2). There is deliberately no
// `Display` impl — a bare `{}` would have to guess the format and would be wrong
// on any repo that configured a non-default one.

/// One declaration site discovered by the scanner: a `# <ID>: …` heading in a
/// Markdown file or a code doc-comment (§AR-scanner.2.1, §AR-scanner.4), with its
/// section body map (§AR-scanner.2.2) and, for stub headings, the inline-home path
/// it points at (§FS-show.2.3, §FS-check.3.4).
#[derive(Debug)]
struct Declaration {
    id: Id,
    file: PathBuf,
    line: usize,
    heading_level: usize,
    sections: BTreeMap<String, String>,
    is_stub: bool,
    defined_in: Option<PathBuf>,
    e2e_case: Option<E2eCase>,
    /// Heading text after `# <ID>:` — the one-line title an author wrote
    /// (§AR-scanner.2.1). `None` when the heading carries no `: <text>` tail, or
    /// when the heading is a stub link (`# <ID>: [<text>](<path>)`), whose tail
    /// is a path, not a title.
    title: Option<String>,
}

/// An `e2e/cases/<name>/` directory treated as an `E2E-<name>` declaration
/// (§AR-scanner.6) — its `command.args`, `expected.exit`, and fixture file list
/// are what `grund show E2E-<name>` renders (§FS-show.2.4).
#[derive(Debug)]
struct E2eCase {
    dir: PathBuf,
    args: Vec<String>,
    expected_exit: i32,
    fixtures: Vec<PathBuf>,
}

/// One citation site: an `<ID>[.<section>]` token, optionally `§`-prefixed
/// (§AR-scanner.2.3, §FS-check.1.1). `has_marker` drives strict-mode filtering
/// (§FS-config.3.1) and is what `grund fmt` upgrades a bare token from (§FS-fmt.2.2).
#[derive(Debug)]
struct Citation {
    namespace: Option<String>,
    id: Id,
    section: Option<String>,
    file: PathBuf,
    line: usize,
    column: usize,
    has_marker: bool,
    text: String,
}

/// Everything the scanner found in one tree walk — declarations grouped by ID
/// (so duplicates surface, §FS-check.3.3) and citations in encounter order. This
/// is the scanner's whole output; the checker (§AR-checker) consumes it without
/// re-reading files.
#[derive(Default)]
struct Findings {
    declarations: BTreeMap<Id, Vec<Declaration>>,
    citations: Vec<Citation>,
    /// Every file the walk read successfully (§AR-scanner.1) — the universe the
    /// `[reference] require_grounding` check iterates over (§FS-check.3.6,
    /// §DF-require-grounding). Files that failed to read are not here; they are in
    /// the walk's `ScanError` list instead.
    scanned_files: Vec<PathBuf>,
}

/// `grund show` slice mode (§FS-show.1): each rung adds to the previous one —
/// `--brief` is heading + first paragraph; `Default` adds the rest of the lead
/// (cut at the first child section); `Toc` adds the nested section map; `Full`
/// adds every subsection body. `Outline` is an internal-only mode used by `Toc`
/// to collect the section map; the CLI does not expose it.
#[derive(Clone, Copy, Eq, PartialEq)]
enum ShowMode {
    Brief,
    Default,
    Toc,
    Full,
    Outline,
}

struct ShowSection {
    path: String,
    title: String,
    depth: usize,
}

/// One `[[kinds]]` entry: prefix plus the folder its declarations live in and the
/// human title `grund id` prints (§FS-config.3.4). When `file` is set, every
/// declaration of this kind must live in that exact file — a *single-file kind*,
/// used by `GND`/`GOAL`/`RM` whose IDs all live in one document
/// (`docs/grund.md`, `docs/goals.md`, `docs/roadmap.md`).
#[derive(Clone)]
struct KindConfig {
    prefix: String,
    folder: Option<String>,
    file: Option<String>,
    title: Option<String>,
}

/// The effective configuration: every `.agents/grund.toml` key (§FS-config.3) merged
/// over the built-in defaults (§FS-config.2), plus the compiled `Grammar` and the
/// `root` / `cli_base` paths the walk and the report use.
#[derive(Clone)]
struct Config {
    root: PathBuf,
    /// The resolved path argument (or cwd) — the base for reports when
    /// `[output] relative_paths = false`, i.e. the base `grund` would use if no
    /// `.agents/grund.toml` were discovered (§FS-config.3.6).
    cli_base: PathBuf,
    project_name: Option<String>,
    marker: String,
    trigger: String,
    strict: bool,
    /// `[reference] require_grounding` (§FS-config.3.1, §FS-check.3.6,
    /// §DF-require-grounding) — when true, `check` also reports every scanned
    /// source file that carries no resolving citation (and declares no ID inline).
    /// `--require-grounding` on `grund check` forces it on for one run.
    require_grounding: bool,
    include: Option<Vec<String>>,
    exclude: Vec<String>,
    extensions: Vec<String>,
    comment_prefixes: Vec<String>,
    docstring_python: bool,
    respect_gitignore: bool,
    output_format: String,
    relative_paths: bool,
    id_format: String,
    section_separator: String,
    number_pattern: String,
    slug_pattern: String,
    kinds: Vec<KindConfig>,
    fmt_cross_refs_enabled: bool,
    cross_ref_anchor_format: String,
    workspace_declared: bool,
    workspace_members: Vec<String>,
    workspace_include_root: bool,
    workspace_boundary_roots: Vec<PathBuf>,
    grammar: Grammar,
}

const DEFAULT_KINDS: &[&str] = &["GND", "GOAL", "FS", "AR", "DF", "DA", "E2E", "RM"];
const DEFAULT_ID_FORMAT: &str = "{kind}-{number}-{slug}";
const DEFAULT_SECTION_SEPARATOR: &str = ".";
const DEFAULT_NUMBER_PATTERN: &str = r"\d+";
const DEFAULT_SLUG_PATTERN: &str = r"[a-z0-9][a-z0-9-]*";

impl Config {
    /// The built-in defaults — the canonical grammar a conformant tree gets with
    /// no `.agents/grund.toml` at all (§FS-config.2, §GOAL-zero-config). `grund init`
    /// writes these same values out verbatim as a teaching surface (§FS-init.2.4).
    fn default_for(root: PathBuf) -> Self {
        let kinds: Vec<KindConfig> = DEFAULT_KINDS
            .iter()
            .map(|prefix| KindConfig {
                prefix: prefix.to_string(),
                folder: default_kind_folder(prefix).map(str::to_string),
                file: default_kind_file(prefix).map(str::to_string),
                title: default_kind_title(prefix).map(str::to_string),
            })
            .collect();
        let kind_prefixes = kind_prefixes(&kinds);
        let grammar = Grammar::build(
            DEFAULT_ID_FORMAT,
            &kind_prefixes,
            DEFAULT_NUMBER_PATTERN,
            DEFAULT_SLUG_PATTERN,
            DEFAULT_SECTION_SEPARATOR,
            &DEFAULT_COMMENT_PREFIXES
                .iter()
                .map(|prefix| prefix.to_string())
                .collect::<Vec<_>>(),
        )
        .expect("default grammar must compile");
        Self {
            cli_base: root.clone(),
            root,
            project_name: None,
            marker: "§".to_string(),
            trigger: "$$".to_string(),
            strict: false,
            require_grounding: false,
            include: Some(
                DEFAULT_INCLUDE
                    .iter()
                    .map(|path| path.to_string())
                    .collect(),
            ),
            exclude: vec![
                "target".into(),
                "node_modules".into(),
                ".git".into(),
                "dist".into(),
                "build".into(),
                ".venv".into(),
            ],
            extensions: vec![
                "md".into(),
                "rs".into(),
                "go".into(),
                "java".into(),
                "kt".into(),
                "ts".into(),
                "tsx".into(),
                "js".into(),
                "py".into(),
                "c".into(),
                "cpp".into(),
                "swift".into(),
                "scala".into(),
                "rb".into(),
                "php".into(),
                "cs".into(),
            ],
            comment_prefixes: DEFAULT_COMMENT_PREFIXES
                .iter()
                .map(|prefix| prefix.to_string())
                .collect(),
            docstring_python: true,
            respect_gitignore: true,
            output_format: "text".into(),
            relative_paths: true,
            id_format: DEFAULT_ID_FORMAT.into(),
            section_separator: DEFAULT_SECTION_SEPARATOR.into(),
            number_pattern: DEFAULT_NUMBER_PATTERN.into(),
            slug_pattern: DEFAULT_SLUG_PATTERN.into(),
            kinds,
            fmt_cross_refs_enabled: false,
            cross_ref_anchor_format: "github".into(),
            workspace_declared: false,
            workspace_members: Vec::new(),
            workspace_include_root: true,
            workspace_boundary_roots: Vec::new(),
            grammar,
        }
    }

    /// Recompile the `Grammar` after `[id]` / `[[kinds]]` / `[scan].comment_prefixes`
    /// keys are read from a config file (§FS-config.3) — keeps the regexes and the
    /// scalar config in lockstep.
    fn rebuild_grammar(&mut self) -> Result<()> {
        let prefixes = kind_prefixes(&self.kinds);
        self.grammar = Grammar::build(
            &self.id_format,
            &prefixes,
            &self.number_pattern,
            &self.slug_pattern,
            &self.section_separator,
            &self.comment_prefixes,
        )?;
        Ok(())
    }
}

fn kind_prefixes(kinds: &[KindConfig]) -> Vec<String> {
    kinds.iter().map(|kind| kind.prefix.clone()).collect()
}

/// Default home folder for each built-in kind — the directory `grund id` proposes
/// a path under and `grund check` expects the declaration to live in (§FS-config.3.4).
fn default_kind_folder(prefix: &str) -> Option<&'static str> {
    match prefix {
        "FS" => Some("docs/functional-spec"),
        "AR" => Some("docs/architecture"),
        "DA" => Some("docs/decisions/architectural"),
        "DF" => Some("docs/decisions/functional"),
        "E2E" => Some("e2e/cases"),
        // GND, GOAL, RM are single-file kinds — see `default_kind_file`. A
        // kind can always be broken up later by swapping `file = "…"` for
        // `folder = "…"` and moving the document into the folder.
        _ => None,
    }
}

/// Default single-file home for the three kinds whose declarations all live in
/// one document — `GND` in `docs/grund.md`, `GOAL` in `docs/goals.md`, `RM`
/// in `docs/roadmap.md` (§FS-config.3.4). Other built-in kinds have no `file`
/// (each declaration is its own file).
fn default_kind_file(prefix: &str) -> Option<&'static str> {
    match prefix {
        "GND" => Some("docs/grund.md"),
        "GOAL" => Some("docs/goals.md"),
        "RM" => Some("docs/roadmap.md"),
        _ => None,
    }
}

/// Default human title for each built-in kind, printed by `grund id` (§FS-config.3.4,
/// §FS-id.2).
fn default_kind_title(prefix: &str) -> Option<&'static str> {
    match prefix {
        "GND" => Some("Reason for existence"),
        "GOAL" => Some("Outcomes the project is trying to achieve"),
        "FS" => Some("User-visible behavior and requirements"),
        "AR" => Some("System design and technical structure"),
        "DA" => Some("Architecture decisions and tradeoffs"),
        "DF" => Some("Product behavior decisions and tradeoffs"),
        "E2E" => Some("Executable user scenarios"),
        "RM" => Some("Planned milestones and sequencing"),
        _ => None,
    }
}

/// A secondary location attached to a diagnostic — e.g. the other declaration in a
/// duplicate pair, or the citation that pointed at a missing section (§FS-errors.2.1).
#[derive(Clone)]
struct Site {
    path: PathBuf,
    line: usize,
}

/// One finding in the located-finding shape of §FS-errors.2.1: a fixed `code`, the
/// `path:line` it occurred at, the message text, and any cross-reference `sites`.
struct Diagnostic {
    code: &'static str,
    path: Option<PathBuf>,
    line: Option<usize>,
    message: String,
    sites: Vec<Site>,
}

/// The outcome of `check`: errors and warnings, kept apart so the exit code keys
/// off errors only (§FS-check.2, §FS-check.4) and the printed order is fixed
/// (§FS-errors.4, §FS-non-goals.9).
#[derive(Default)]
struct Report {
    errors: Vec<Diagnostic>,
    warnings: Vec<Diagnostic>,
}

/// What `grund show` resolved an ID to: the body text to print, the `path:line` it
/// came from, the section map (`--toc` only), and the pre-rendered JSON when
/// `--format json` was asked for (§FS-show.3, §FS-errors.5).
struct ShowOutput {
    body: String,
    path: PathBuf,
    line: usize,
    json: Option<String>,
    sections: Vec<ShowSection>,
}

fn resolve_stub_target(root: &Path, stub_file: &Path, target: &Path) -> PathBuf {
    if target.is_absolute() {
        return target.to_path_buf();
    }
    let stub_file = if stub_file.is_absolute() {
        stub_file.to_path_buf()
    } else {
        root.join(stub_file)
    };
    let markdown_relative =
        normalize_path_lexically(&stub_file.parent().unwrap_or(root).join(target));
    if markdown_relative.exists() {
        markdown_relative
    } else {
        normalize_path_lexically(&root.join(target))
    }
}

fn normalize_path_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

/// Pull an `Id` out of a `Grammar` regex match — the `kind` / `num` / `slug`
/// capture groups the `[id] format` defined (§FS-config.3.2, §AR-scanner.2.1).
fn parse_id(caps: &regex::Captures) -> Option<Id> {
    let kind = caps.name("kind")?.as_str().to_string();
    let num = match caps.name("num") {
        Some(m) => Some(m.as_str().parse().ok()?),
        None => None,
    };
    let slug = caps.name("slug").map(|m| m.as_str().to_string());
    Some(Id { kind, num, slug })
}

/// Parse a CLI `<ID>[.<section>]` argument (the form `grund show` / `grund refs` take,
/// §FS-show.1, §FS-refs.1) into an `Id` and an optional section path (§FS-config.3.3).
fn parse_id_arg(raw: &str, grammar: &Grammar) -> Result<(Id, Option<String>)> {
    let caps = grammar
        .id_input_re
        .captures(raw)
        .ok_or_else(|| anyhow!("invalid ID `{raw}`"))?;
    let id = parse_id(&caps).ok_or_else(|| anyhow!("invalid ID `{raw}`"))?;
    Ok((id, caps.name("sec").map(|m| m.as_str().to_string())))
}

fn render_qualified_id(config: &Config, namespace: Option<&str>, id: &Id) -> String {
    match namespace {
        Some(namespace) => format!("{}/{}", namespace, render_id(config, id)),
        None => render_id(config, id),
    }
}

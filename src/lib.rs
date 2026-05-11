use anyhow::{Context, Result, anyhow};
use ignore::WalkBuilder;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;
use unicode_normalization::UnicodeNormalization;

const SEC_GROUP: &str = r"(?P<sec>\d+(?:\.\d+)*)";
const DEFAULT_INCLUDE: &[&str] = &["docs", "e2e", "src"];
const DEFAULT_COMMENT_PREFIXES: &[&str] = &["//", "#", ";", "--", "*", "/*"];
const SUBCOMMANDS: &[&str] = &[
    "check",
    "show",
    "list",
    "refs",
    "cover",
    "fmt",
    "name",
    "init",
    "config",
    "agent-setup-instructions",
    "completions",
];

static STUB_LINK_HEADING: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*:\s*\[[^\]]*\]\(\s*(?P<path>[^)\s]+)\s*\)\s*$").unwrap());
static AGENTS_BLOCK_BEGIN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<!--\s*gnd:init:agents:v(?P<version>\d+)\s+begin\s*-->").unwrap());
static AGENTS_BLOCK_END: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<!--\s*gnd:init:agents:v\d+\s+end\s*-->").unwrap());

/// ID grammar compiled from [id].format + [[kinds]] — the single place that knows the
/// shape of a declaration heading or a citation. Built once per config load.
/// Realizes §FS-config.3.1, §FS-config.3.2, §FS-config.3.3 and the regex-not-a-parser
/// stance of §AS-scanner.5.
#[derive(Clone)]
struct Grammar {
    decl_re: Regex,
    section_re: Regex,
    citation_re: Regex,
    id_input_re: Regex,
}

impl Grammar {
    /// Compile the four regexes from the effective config. The validation rejections
    /// here (`{kind}` required, at least one of `{number}`/`{slug}`, separator must be
    /// lexically distinct) are §FS-config.3.2; the optional `§`-marker prefix on a
    /// citation is §FS-config.3.1 / §DF-reference-marker; the comment-prefix wrapper
    /// on declaration/section regexes is §AS-scanner.4 (declarations live in code
    /// doc-comments too).
    fn build(
        format: &str,
        kinds: &[String],
        number_pattern: &str,
        slug_pattern: &str,
        section_separator: &str,
        comment_prefixes: &[String],
    ) -> Result<Self> {
        let kind_alt = if kinds.is_empty() {
            return Err(anyhow!("[id] grammar needs at least one [[kinds]] entry"));
        } else {
            kinds
                .iter()
                .map(|k| regex::escape(k))
                .collect::<Vec<_>>()
                .join("|")
        };
        let kind_group = format!("(?P<kind>{})", kind_alt);
        let num_group = format!("(?P<num>{})", number_pattern);
        let slug_group = format!("(?P<slug>{})", slug_pattern);

        let mut id_pat = String::new();
        let mut literals: Vec<String> = Vec::new();
        let mut has_kind = false;
        let mut has_number = false;
        let mut has_slug = false;
        let mut cursor = 0;
        let bytes = format.as_bytes();
        while cursor < bytes.len() {
            if let Some(end) = format[cursor..].find('}') {
                let abs_end = cursor + end;
                if let Some(start_rel) = format[cursor..].find('{') {
                    let abs_start = cursor + start_rel;
                    if abs_start < abs_end {
                        // Append literal between cursor and abs_start (escaped).
                        if abs_start > cursor {
                            literals.push(format[cursor..abs_start].to_string());
                        }
                        id_pat.push_str(&regex::escape(&format[cursor..abs_start]));
                        let placeholder = &format[abs_start + 1..abs_end];
                        match placeholder {
                            "kind" => {
                                if has_kind {
                                    return Err(anyhow!("[id].format: {{kind}} appears twice"));
                                }
                                has_kind = true;
                                id_pat.push_str(&kind_group);
                            }
                            "number" => {
                                if has_number {
                                    return Err(anyhow!("[id].format: {{number}} appears twice"));
                                }
                                has_number = true;
                                id_pat.push_str(&num_group);
                            }
                            "slug" => {
                                if has_slug {
                                    return Err(anyhow!("[id].format: {{slug}} appears twice"));
                                }
                                has_slug = true;
                                id_pat.push_str(&slug_group);
                            }
                            other => {
                                return Err(anyhow!(
                                    "[id].format: unknown placeholder `{{{other}}}`"
                                ));
                            }
                        }
                        cursor = abs_end + 1;
                        continue;
                    }
                }
                return Err(anyhow!("[id].format: stray `}}` in template"));
            }
            // No more placeholders — append the rest as literal.
            if cursor < format.len() {
                literals.push(format[cursor..].to_string());
            }
            id_pat.push_str(&regex::escape(&format[cursor..]));
            break;
        }

        if !has_kind {
            return Err(anyhow!("[id].format must contain {{kind}}"));
        }
        if !has_number && !has_slug {
            return Err(anyhow!(
                "[id].format must contain at least one of {{number}} or {{slug}}"
            ));
        }

        // §FS-config.3.2: the section separator must be lexically distinguishable
        // from the ID grammar — otherwise a citation like `FS-foo<sep>bar` could
        // not be split into ID and section unambiguously.
        if section_separator.is_empty() {
            return Err(anyhow!("[id].section_separator must not be empty"));
        }
        if literals.iter().any(|lit| lit.contains(section_separator)) {
            return Err(anyhow!(
                "[id].section_separator `{section_separator}` collides with a literal in [id].format"
            ));
        }
        if Regex::new(slug_pattern)
            .map(|re| re.is_match(section_separator))
            .unwrap_or(false)
        {
            return Err(anyhow!(
                "[id].section_separator `{section_separator}` is matched by [id].slug_pattern"
            ));
        }
        if has_number
            && Regex::new(number_pattern)
                .map(|re| re.is_match(section_separator))
                .unwrap_or(false)
        {
            return Err(anyhow!(
                "[id].section_separator `{section_separator}` is matched by [id].number_pattern"
            ));
        }

        let sep_quoted = regex::escape(section_separator);
        let sec_suffix = format!(r"(?:{}{})?", sep_quoted, SEC_GROUP);

        let comment_prefix = comment_prefix_regex(comment_prefixes);
        let decl_re = Regex::new(&format!(
            r"^\s*(?:{})?\s*(?P<hashes>#+)\s+{}\b",
            comment_prefix, id_pat
        ))?;
        let section_re = Regex::new(&format!(
            r"^\s*(?:{})?\s*(?P<hashes>#+)\s+{}\.?\s+\S",
            comment_prefix, SEC_GROUP
        ))?;
        let citation_re = Regex::new(&format!(r"\b{}{}", id_pat, sec_suffix))?;
        let id_input_re = Regex::new(&format!(r"^{}{}$", id_pat, sec_suffix))?;

        Ok(Self {
            decl_re,
            section_re,
            citation_re,
            id_input_re,
        })
    }
}

/// Build the alternation a declaration/section heading may be prefixed by — one
/// entry per `[scan] comment_prefixes` value (§FS-config.3.5), with `//` widened to
/// also catch Rust/JS doc-comment forms `///` and `//!` so inline declarations in
/// code are seen (§AS-scanner.4). Longest-first so `//` does not shadow `///`.
fn comment_prefix_regex(comment_prefixes: &[String]) -> String {
    let mut prefixes = comment_prefixes
        .iter()
        .filter(|prefix| !prefix.is_empty())
        .map(|prefix| {
            if prefix == "//" {
                r"//[/!]?".to_string()
            } else {
                regex::escape(prefix)
            }
        })
        .collect::<Vec<_>>();
    prefixes.sort_by_key(|prefix| std::cmp::Reverse(prefix.len()));
    if prefixes.is_empty() {
        "(?!)".to_string()
    } else {
        format!("(?:{})", prefixes.join("|"))
    }
}

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
/// Markdown file or a code doc-comment (§AS-scanner.2.1, §AS-scanner.4), with its
/// section body map (§AS-scanner.2.2) and, for stub headings, the inline-home path
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
    /// (§AS-scanner.2.1). `None` when the heading carries no `: <text>` tail, or
    /// when the heading is a stub link (`# <ID>: [<text>](<path>)`), whose tail
    /// is a path, not a title.
    title: Option<String>,
}

/// An `e2e/cases/<name>/` directory treated as an `E2E-<name>` declaration
/// (§AS-scanner.6) — its `command.args`, `expected.exit`, and fixture file list
/// are what `gnd show E2E-<name>` renders (§FS-show.2.4).
#[derive(Debug)]
struct E2eCase {
    dir: PathBuf,
    args: Vec<String>,
    expected_exit: i32,
    fixtures: Vec<PathBuf>,
}

/// One citation site: an `<ID>[.<section>]` token, optionally `§`-prefixed
/// (§AS-scanner.2.3, §FS-check.1.1). `has_marker` drives strict-mode filtering
/// (§FS-config.3.1) and is what `gnd fmt` upgrades a bare token from (§FS-fmt.2.2).
#[derive(Debug)]
struct Citation {
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
/// is the scanner's whole output; the checker (§AS-checker) consumes it without
/// re-reading files.
#[derive(Default)]
struct Findings {
    declarations: BTreeMap<Id, Vec<Declaration>>,
    citations: Vec<Citation>,
    /// Every file the walk read successfully (§AS-scanner.1) — the universe the
    /// `[reference] require_grounding` check iterates over (§FS-check.3.6,
    /// §DF-require-grounding). Files that failed to read are not here; they are in
    /// the walk's `ScanError` list instead.
    scanned_files: Vec<PathBuf>,
}

/// One `[[kinds]]` entry: prefix plus the folder its declarations live in and the
/// human title `gnd name` prints (§FS-config.3.4).
#[derive(Clone)]
struct KindConfig {
    prefix: String,
    folder: Option<String>,
    title: Option<String>,
}

/// The effective configuration: every `.agents/gnd.toml` key (§FS-config.3) merged
/// over the built-in defaults (§FS-config.2), plus the compiled `Grammar` and the
/// `root` / `cli_base` paths the walk and the report use.
#[derive(Clone)]
struct Config {
    root: PathBuf,
    /// The resolved path argument (or cwd) — the base for reports when
    /// `[output] relative_paths = false`, i.e. the base `gnd` would use if no
    /// `.agents/gnd.toml` were discovered (§FS-config.3.6).
    cli_base: PathBuf,
    marker: String,
    trigger: String,
    strict: bool,
    /// `[reference] require_grounding` (§FS-config.3.1, §FS-check.3.6,
    /// §DF-require-grounding) — when true, `check` also reports every scanned
    /// source file that carries no resolving citation (and declares no ID inline).
    /// `--require-grounding` on `gnd check` forces it on for one run.
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
    fmt_md_links_enabled: bool,
    md_link_anchor_format: String,
    grammar: Grammar,
}

const DEFAULT_KINDS: &[&str] = &["G", "FS", "AS", "DF", "DA", "E2E", "RM"];
const DEFAULT_ID_FORMAT: &str = "{kind}-{number}-{slug}";
const DEFAULT_SECTION_SEPARATOR: &str = ".";
const DEFAULT_NUMBER_PATTERN: &str = r"\d+";
const DEFAULT_SLUG_PATTERN: &str = r"[a-z0-9][a-z0-9-]*";

impl Config {
    /// The built-in defaults — the canonical grammar a conformant tree gets with
    /// no `.agents/gnd.toml` at all (§FS-config.2, §G-zero-config). `gnd init`
    /// writes these same values out verbatim as a teaching surface (§FS-init.2.4).
    fn default_for(root: PathBuf) -> Self {
        let kinds: Vec<KindConfig> = DEFAULT_KINDS
            .iter()
            .map(|prefix| KindConfig {
                prefix: prefix.to_string(),
                folder: default_kind_folder(prefix).map(str::to_string),
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
            fmt_md_links_enabled: false,
            md_link_anchor_format: "github".into(),
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

/// Default home folder for each built-in kind — the directory `gnd name` proposes
/// a path under and `gnd check` expects the declaration to live in (§FS-config.3.4).
fn default_kind_folder(prefix: &str) -> Option<&'static str> {
    match prefix {
        "G" => Some("docs/goals"),
        "FS" => Some("docs/functional-spec"),
        "AS" => Some("docs/architectural-spec"),
        "DA" => Some("docs/decisions/architectural"),
        "DF" => Some("docs/decisions/functional"),
        "E2E" => Some("e2e/cases"),
        "RM" => Some("docs"),
        _ => None,
    }
}

/// Default human title for each built-in kind, printed by `gnd name` (§FS-config.3.4,
/// §FS-name.2).
fn default_kind_title(prefix: &str) -> Option<&'static str> {
    match prefix {
        "G" => Some("Goal"),
        "FS" => Some("Functional spec"),
        "AS" => Some("Architectural spec"),
        "DA" => Some("Architectural decision"),
        "DF" => Some("Functional decision"),
        "E2E" => Some("End-to-end test"),
        "RM" => Some("Roadmap milestone"),
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

/// What `gnd show` resolved an ID to: the body text to print, the `path:line` it
/// came from, and the pre-rendered JSON when `--format json` was asked for
/// (§FS-show.3, §FS-errors.5).
struct ShowOutput {
    body: String,
    path: PathBuf,
    line: usize,
    json: Option<String>,
}

/// Pull an `Id` out of a `Grammar` regex match — the `kind` / `num` / `slug`
/// capture groups the `[id] format` defined (§FS-config.3.2, §AS-scanner.2.1).
fn parse_id(caps: &regex::Captures) -> Option<Id> {
    let kind = caps.name("kind")?.as_str().to_string();
    let num = match caps.name("num") {
        Some(m) => Some(m.as_str().parse().ok()?),
        None => None,
    };
    let slug = caps.name("slug").map(|m| m.as_str().to_string());
    Some(Id { kind, num, slug })
}

/// Parse a CLI `<ID>[.<section>]` argument (the form `gnd show` / `gnd refs` take,
/// §FS-show.1, §FS-refs.1) into an `Id` and an optional section path (§FS-config.3.3).
fn parse_id_arg(raw: &str, grammar: &Grammar) -> Result<(Id, Option<String>)> {
    let caps = grammar
        .id_input_re
        .captures(raw)
        .ok_or_else(|| anyhow!("invalid ID `{raw}`"))?;
    let id = parse_id(&caps).ok_or_else(|| anyhow!("invalid ID `{raw}`"))?;
    Ok((id, caps.name("sec").map(|m| m.as_str().to_string())))
}

/// Discover and load the effective config: walk upward from `start` for the
/// nearest `.agents/gnd.toml` (§FS-config.1), parse it over the defaults
/// (§FS-config.2), or fall back to the pure defaults if none is found
/// (§G-zero-config).
fn load_config(start: &Path) -> Result<Config> {
    let start_dir = if start.is_file() {
        start.parent().unwrap_or(Path::new(".")).to_path_buf()
    } else {
        start.to_path_buf()
    };
    // Resolve to an absolute path before walking up, mirroring how `cargo` finds
    // `Cargo.toml` (§FS-config.1): a relative `.` or `subdir/` must still discover
    // a `.agents/gnd.toml` in an ancestor directory.
    let walk_start = fs::canonicalize(&start_dir).unwrap_or(start_dir);
    let mut cursor = Some(walk_start.as_path());
    while let Some(dir) = cursor {
        let candidate = dir.join(".agents").join("gnd.toml");
        if candidate.exists() {
            let mut config = Config::default_for(dir.to_path_buf());
            config.cli_base = walk_start.clone();
            // Report config errors against a stable relative path, never the
            // absolute discovered path (§FS-errors.4: deterministic, no absolute
            // paths outside the configured root).
            let report_path = candidate
                .strip_prefix(dir)
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| candidate.clone());
            parse_config_file(&candidate, &report_path, &mut config)?;
            return Ok(config);
        }
        cursor = dir.parent();
    }
    // Zero-config (§G-zero-config): the "project root" is the current working
    // directory, never the path that happened to be passed on the command line —
    // so `[scan] include` resolves against the repo and `gnd check src/` scopes
    // *into* it instead of looking for `src/docs`, `src/e2e`, `src/src`. Reports
    // stay relative to `cli_base` (the resolved path arg) when
    // `[output] relative_paths = false` (§FS-config.3.6).
    let root = std::env::current_dir()
        .ok()
        .and_then(|cwd| fs::canonicalize(&cwd).ok())
        .unwrap_or_else(|| walk_start.clone());
    let mut config = Config::default_for(root);
    config.cli_base = walk_start;
    Ok(config)
}

/// Parse one `.agents/gnd.toml` over `config` — the schema of §FS-config.3 and its
/// subsections (`[reference]` 3.1, `[id]` 3.2/3.3, `[[kinds]]` 3.4, `[scan]` 3.5,
/// `[output]` 3.6, `[fmt.md_links]` 3.7). Any unknown section/key or malformed
/// value is a hard error reported as `path:line:` (§FS-config.4.3, §FS-errors.2.1).
fn parse_config_file(read_path: &Path, report_path: &Path, config: &mut Config) -> Result<()> {
    let text =
        fs::read_to_string(read_path).with_context(|| format!("read {}", report_path.display()))?;
    // Everything below reports problems against the stable relative path.
    let path = report_path;
    let mut section = String::new();
    let mut grammar_dirty = false;
    let mut parsed_kinds: Vec<KindConfig> = Vec::new();
    let mut current_kind: Option<KindConfig> = None;
    let mut kinds_block_seen = false;
    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let is_array_table = line.starts_with("[[") && line.ends_with("]]");
            section = line.trim_matches(['[', ']']).to_string();
            match section.as_str() {
                "reference" | "scan" | "output" | "id" | "fmt.md_links" => {}
                "kinds" => {
                    if !is_array_table {
                        bail_config(
                            path,
                            line_no,
                            "expected `[[kinds]]` (array of tables)".to_string(),
                        )?;
                    }
                    // Flush any open kind entry, then start a new one.
                    if let Some(prefix) = current_kind.take() {
                        parsed_kinds.push(prefix);
                    }
                    current_kind = Some(KindConfig {
                        prefix: String::new(),
                        folder: None,
                        title: None,
                    });
                    kinds_block_seen = true;
                }
                other => bail_config(path, line_no, format!("unknown config section `{other}`"))?,
            }
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            bail_config(path, line_no, "expected `key = value`".to_string())?;
            unreachable!();
        };
        let key = key.trim();
        let value = value.trim();
        match (section.as_str(), key) {
            ("", "gnd_config_version") => {
                if value != "1" {
                    bail_config(path, line_no, "unsupported config version".to_string())?;
                }
            }
            ("", "project_name") => {
                parse_string(path, line_no, value)?;
            }
            ("reference", "marker") => config.marker = parse_string(path, line_no, value)?,
            ("reference", "trigger") => config.trigger = parse_string(path, line_no, value)?,
            ("reference", "strict") => config.strict = parse_bool(path, line_no, value)?,
            ("reference", "require_grounding") => {
                config.require_grounding = parse_bool(path, line_no, value)?
            }
            ("id", "format") => {
                config.id_format = parse_string(path, line_no, value)?;
                grammar_dirty = true;
            }
            ("id", "section_separator") => {
                config.section_separator = parse_string(path, line_no, value)?;
                grammar_dirty = true;
            }
            ("id", "number_pattern") => {
                config.number_pattern = parse_string(path, line_no, value)?;
                grammar_dirty = true;
            }
            ("id", "slug_pattern") => {
                config.slug_pattern = parse_string(path, line_no, value)?;
                grammar_dirty = true;
            }
            ("kinds", "prefix") => {
                let prefix = parse_string(path, line_no, value)?;
                if let Some(slot) = current_kind.as_mut() {
                    slot.prefix = prefix;
                } else {
                    bail_config(
                        path,
                        line_no,
                        "`prefix` outside of [[kinds]] block".to_string(),
                    )?;
                }
            }
            ("kinds", "folder") => {
                let folder = parse_string(path, line_no, value)?;
                if let Some(slot) = current_kind.as_mut() {
                    slot.folder = Some(folder);
                } else {
                    bail_config(
                        path,
                        line_no,
                        "`folder` outside of [[kinds]] block".to_string(),
                    )?;
                }
            }
            ("kinds", "title") => {
                let title = parse_string(path, line_no, value)?;
                if let Some(slot) = current_kind.as_mut() {
                    slot.title = Some(title);
                } else {
                    bail_config(
                        path,
                        line_no,
                        "`title` outside of [[kinds]] block".to_string(),
                    )?;
                }
            }
            ("scan", "include") => config.include = Some(parse_string_list(path, line_no, value)?),
            ("scan", "exclude") => config.exclude = parse_string_list(path, line_no, value)?,
            ("scan", "extensions") => config.extensions = parse_string_list(path, line_no, value)?,
            ("scan", "comment_prefixes") => {
                config.comment_prefixes = parse_string_list(path, line_no, value)?;
                grammar_dirty = true;
            }
            ("scan", "docstring_python") => {
                config.docstring_python = parse_bool(path, line_no, value)?;
            }
            ("scan", "respect_gitignore") => {
                config.respect_gitignore = parse_bool(path, line_no, value)?;
            }
            ("output", "format") => {
                let format = parse_string(path, line_no, value)?;
                if !matches!(format.as_str(), "text" | "json") {
                    bail_config(path, line_no, "unsupported output format".to_string())?;
                }
                config.output_format = format;
            }
            ("output", "color") => {
                parse_string(path, line_no, value)?;
            }
            ("output", "relative_paths") => {
                config.relative_paths = parse_bool(path, line_no, value)?;
            }
            ("fmt.md_links", "enabled") => {
                config.fmt_md_links_enabled = parse_bool(path, line_no, value)?;
            }
            ("fmt.md_links", "anchor_format") => {
                let format = parse_string(path, line_no, value)?;
                if !matches!(
                    format.as_str(),
                    "github" | "gitlab" | "mkdocs" | "pandoc" | "none"
                ) {
                    bail_config(path, line_no, "unknown md link anchor format".to_string())?;
                }
                config.md_link_anchor_format = format;
            }
            _ => bail_config(path, line_no, format!("unknown config key `{key}`"))?,
        }
    }
    if let Some(prefix) = current_kind.take() {
        parsed_kinds.push(prefix);
    }
    if config.strict && config.marker.is_empty() {
        return Err(anyhow!(
            "{}: reference.strict requires a non-empty marker",
            path.display()
        ));
    }
    if kinds_block_seen {
        // [[kinds]] replaces defaults entirely, per §FS-config.3.4.
        if parsed_kinds.iter().any(|p| p.prefix.is_empty()) {
            return Err(anyhow!(
                "{}: every [[kinds]] entry must declare a `prefix`",
                path.display()
            ));
        }
        if parsed_kinds.is_empty() {
            return Err(anyhow!(
                "{}: at least one [[kinds]] entry must declare a `prefix`",
                path.display()
            ));
        }
        // Reject kinds whose prefix is itself a prefix of another kind's prefix
        // (§FS-config.3.4 — would make tokenization ambiguous).
        for (i, a) in parsed_kinds.iter().enumerate() {
            for (j, b) in parsed_kinds.iter().enumerate() {
                if i != j
                    && a.prefix.len() <= b.prefix.len()
                    && b.prefix.starts_with(a.prefix.as_str())
                {
                    return Err(anyhow!(
                        "{}: kinds `{}` and `{}` collide (one is a prefix of the other)",
                        path.display(),
                        a.prefix,
                        b.prefix
                    ));
                }
            }
        }
        config.kinds = parsed_kinds;
    }
    if grammar_dirty || kinds_block_seen {
        config
            .rebuild_grammar()
            .with_context(|| format!("{}: invalid [id] grammar", path.display()))?;
    }
    Ok(())
}

/// Drop a trailing `#`-comment from a `.agents/gnd.toml` line (§FS-config.3).
fn strip_comment(line: &str) -> &str {
    // A `#` inside a quoted string is not a comment marker. Walk the line and stop at the
    // first unquoted `#`; otherwise return the line unchanged.
    let bytes = line.as_bytes();
    let mut in_string = false;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' if !is_escaped(bytes, i) => in_string = !in_string,
            b'#' if !in_string => return &line[..i],
            _ => {}
        }
        i += 1;
    }
    line
}

fn is_escaped(bytes: &[u8], pos: usize) -> bool {
    let mut count = 0;
    let mut j = pos;
    while j > 0 && bytes[j - 1] == b'\\' {
        count += 1;
        j -= 1;
    }
    count % 2 == 1
}

/// Fail config parsing with a `path:line: message` error — the located-finding
/// shape applied to a malformed `.agents/gnd.toml` (§FS-config.4.3, §FS-errors.2.1).
fn bail_config<T>(path: &Path, line: usize, message: String) -> Result<T> {
    Err(anyhow!("{}:{}: {}", path.display(), line, message))
}

fn parse_string(path: &Path, line: usize, value: &str) -> Result<String> {
    if !(value.starts_with('"') && value.ends_with('"') && value.len() >= 2) {
        return bail_config(path, line, "expected string".to_string());
    }
    let inner = &value[1..value.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => {
                return bail_config(
                    path,
                    line,
                    format!("invalid escape sequence `\\{other}` in string"),
                );
            }
            None => {
                return bail_config(path, line, "trailing backslash in string".to_string());
            }
        }
    }
    Ok(out)
}

fn parse_bool(path: &Path, line: usize, value: &str) -> Result<bool> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => bail_config(path, line, "expected boolean".to_string()),
    }
}

fn parse_string_list(path: &Path, line: usize, value: &str) -> Result<Vec<String>> {
    if !value.starts_with('[') || !value.ends_with(']') {
        return bail_config(path, line, "expected string list".to_string());
    }
    let inner = value[1..value.len() - 1].trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split(',')
        .map(|part| parse_string(path, line, part.trim()))
        .collect()
}

/// Whether a file is one the scanner reads: a non-hidden name with an extension in
/// `[scan] extensions` (§FS-config.3.5, §AS-scanner.1).
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

/// The per-file scan (§AS-scanner.2): line by line, find declaration headings
/// (§AS-scanner.2.1 — in Markdown or in a code/`"""` doc-comment, §AS-scanner.4),
/// nested section headings (§AS-scanner.2.2), and `<ID>[.<section>]` citations
/// (§AS-scanner.2.3, §FS-check.1.1) — skipping fenced code blocks and, outside
/// Markdown, bare ID-shaped tokens inside string literals (§FS-fmt.2.3.1) and any
/// bare token at all under `[reference] strict` (§FS-config.3.1).
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
            let has_marker = scan_line[..full.start()].ends_with(&config.marker);
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
/// declaration whose body is the case manifest (§AS-scanner.6, §FS-show.2.4) — so
/// `gnd check` sees `§E2E-…` citations resolve and `gnd refs` finds e2e tests.
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
    case_dirs.sort();

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
/// repo's `[id] format` (§AS-scanner.6, §FS-config.3.4).
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
/// with (§AS-scanner.6).
fn literal_after_kind_placeholder(format: &str) -> Option<&str> {
    let marker = "{kind}";
    let start = format.find(marker)? + marker.len();
    let rest = &format[start..];
    let end = rest.find('{').unwrap_or(rest.len());
    Some(&rest[..end])
}

/// Inverse of `e2e_id_from_case_dir_name`: strip the `E2E` prefix off a rendered ID
/// to get the `e2e/cases/<name>/` directory `gnd name` tells the author to create
/// (§FS-name.2, §AS-scanner.6).
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
/// `gnd show E2E-<name>` renders (§FS-show.2.4).
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
        .with_context(|| format!("parse {}/expected.exit", dir.display()))?;
    let mut fixtures = Vec::new();
    collect_relative_fixture_files(dir, dir, &mut fixtures)?;
    fixtures.sort();
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
/// section heading nests under the current declaration (§AS-scanner.2.2).
fn heading_level_for_line(line: &str, markdown_heading: bool, caps: &regex::Captures) -> usize {
    if markdown_heading {
        return line
            .trim_start()
            .chars()
            .take_while(|ch| *ch == '#')
            .count()
            .max(1);
    }
    caps.name("hashes").map(|m| m.as_str().len()).unwrap_or(1)
}

/// The tree walk (§AS-scanner.1): from each scan root, descend skipping hidden and
/// `[scan] exclude` directories, honouring `.gitignore` and friends unless
/// `respect_gitignore = false` (§AS-scanner.1.1, §FS-config.3.5), keeping only
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
        builder.filter_entry(move |e| {
            if e.depth() == 0 {
                return true;
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
    files.sort();
    Ok(files)
}

/// The directories (or single file) the walk starts from: a `[path]` argument when
/// given (narrowing the default scope), otherwise `[scan] include` resolved against
/// the repo root, otherwise the whole root (§FS-config.3.5, §AS-scanner.1).
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

/// One full tree walk: scan every file (§AS-scanner.2) plus the e2e case
/// directories (§AS-scanner.6), collecting unreadable files rather than aborting
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

/// Scan helper for point-query subcommands (`show`, `name`): any unreadable file
/// is fatal — a partial view of the tree could miss the declaration entirely or
/// allocate a colliding number (§FS-show.3, §FS-name.4).
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

/// The checker (§AS-checker): turn the scanner's `Findings` into a `Report` of
/// errors and warnings — duplicate declarations (§FS-check.3.3), dangling
/// citations (§FS-check.3.1), missing sections (§FS-check.3.2), broken inline-spec
/// stubs (§FS-check.3.4), an invalid `agents.md` init block (§FS-check.3.5),
/// ungrounded source files when `[reference] require_grounding` is set
/// (§FS-check.3.6, §DF-require-grounding), and the unused-declaration warning
/// (§FS-check.4.1) — then sort everything into the fixed report order
/// (§FS-errors.4, §FS-non-goals.9). It re-reads files only for stub verification
/// (§AS-checker.4); everything else comes from `findings`.
fn check(findings: &Findings, config: &Config) -> Report {
    let mut report = Report::default();
    // §FS-check.3.5: an `agents.md` whose managed block is out of date (or newer
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
            sites.sort_by(|a, b| (a.path.as_os_str(), a.line).cmp(&(b.path.as_os_str(), b.line)));
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
    // exist, or exists but does not itself declare `<ID>` inline (§AS-checker.2.4).
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
                    message: format!("stub link target missing: {}", target.display()),
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
                        target.display()
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
        for file in &findings.scanned_files {
            if file.extension().and_then(|ext| ext.to_str()) == Some("md") {
                continue;
            }
            let grounded = findings
                .citations
                .iter()
                .any(|cite| &cite.file == file && findings.declarations.contains_key(&cite.id))
                || findings
                    .declarations
                    .values()
                    .flatten()
                    .any(|decl| &decl.file == file && !decl.is_stub && decl.e2e_case.is_none());
            if !grounded {
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

/// Put diagnostics in the one fixed order `gnd` ever prints them in — by path, then
/// line, then message text — so two runs over the same tree agree byte-for-byte
/// (§FS-errors.4) and ordering is not a knob (§FS-non-goals.9).
fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by(diagnostic_cmp);
}

fn diagnostic_cmp(a: &Diagnostic, b: &Diagnostic) -> std::cmp::Ordering {
    (
        a.path.as_ref().map(|p| p.as_os_str()),
        a.line.unwrap_or(0),
        a.message.as_str(),
    )
        .cmp(&(
            b.path.as_ref().map(|p| p.as_os_str()),
            b.line.unwrap_or(0),
            b.message.as_str(),
        ))
}

/// Validate the managed agent-entrypoint blocks (§FS-check.3.5): the begin/end
/// marker pair must be present and intact, and the `vN` version must match this
/// binary — an older `vN` is "run `gnd init`" (§FS-init.2.3), a newer one is
/// fatal. `agents.md` is canonical; known companion entrypoints are checked only
/// when present and not symlinked to `agents.md`.
fn check_agents_block_version(root: &Path, report: &mut Report) {
    let canonical = root.join("agents.md");
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
                    "outdated gnd init block v{} (run `gnd init` to update to v{})",
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
                    "unsupported gnd init block v{} (this gnd supports v{})",
                    block.version, AGENTS_BLOCK_VERSION
                ),
                sites: Vec::new(),
            });
        }
        return;
    }
    if AGENTS_BLOCK_BEGIN.is_match(&text) {
        let line = AGENTS_BLOCK_BEGIN
            .find(&text)
            .map(|m| line_for_byte_index(&text, m.start()))
            .unwrap_or(1);
        report.errors.push(Diagnostic {
            code: "agents-init",
            path: Some(path.to_path_buf()),
            line: Some(line),
            message: "malformed gnd init block".to_string(),
            sites: Vec::new(),
        });
    } else {
        report.errors.push(Diagnostic {
            code: "agents-init",
            path: Some(path.to_path_buf()),
            line: Some(1),
            message: format!("missing gnd init block v{}", AGENTS_BLOCK_VERSION),
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
/// stub does not count as a second home, so it is not a duplicate (§AS-scanner.4,
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
/// (§FS-check.3.4, §AS-checker.2.4, §AS-scanner.4).
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

/// Print the report to stderr in the located-finding shape (§FS-errors.1,
/// §FS-errors.2.1) — `path:line: message`, one per line, in the fixed order
/// (§FS-errors.4). A clean run prints nothing (§FS-check.2.1).
fn print_report(config: &Config, report: &Report) {
    if report.errors.is_empty() && report.warnings.is_empty() {
        return;
    }
    let mut diagnostics = report
        .warnings
        .iter()
        .map(|diagnostic| ("warning", diagnostic))
        .chain(report.errors.iter().map(|diagnostic| ("error", diagnostic)))
        .collect::<Vec<_>>();
    diagnostics.sort_by(|(_, a), (_, b)| diagnostic_cmp(a, b));
    for (severity, diagnostic) in diagnostics {
        eprintln!("{}", render_diagnostic_text(config, severity, diagnostic));
    }
}

fn render_diagnostic_text(config: &Config, severity: &str, diagnostic: &Diagnostic) -> String {
    match (&diagnostic.path, diagnostic.line) {
        (Some(path), Some(line)) => {
            format!(
                "{}:{}: {}",
                display_path(config, path),
                line,
                diagnostic.message
            )
        }
        // A file-level finding with no line to point at (e.g. an unreadable file
        // discovered mid-walk) uses the CLI-level shape — §FS-check.2, §FS-errors.2.2.
        (Some(path), None) => format!(
            "{severity}: {}: {}",
            display_path(config, path),
            diagnostic.message
        ),
        _ => format!("{severity}: {}", diagnostic.message),
    }
}

fn sorted_json_diagnostics(report: &Report) -> Vec<(&'static str, &Diagnostic)> {
    let mut diagnostics = report
        .warnings
        .iter()
        .map(|diagnostic| ("warning", diagnostic))
        .chain(report.errors.iter().map(|diagnostic| ("error", diagnostic)))
        .collect::<Vec<_>>();
    diagnostics.sort_by(|(_, a), (_, b)| diagnostic_cmp(a, b));
    diagnostics
}

/// Print the report as newline-delimited JSON objects on stderr — the `--format
/// json` / `[output] format = "json"` shape (§FS-errors.5): one object per finding
/// with `severity`, `path`, `line`, `code`, `message`, `sites`.
fn print_json_report(config: &Config, report: &Report) {
    for (severity, diagnostic) in sorted_json_diagnostics(report) {
        eprintln!("{}", render_diagnostic_json(config, severity, diagnostic));
    }
}

fn render_diagnostic_json(config: &Config, severity: &str, diagnostic: &Diagnostic) -> String {
    let path = diagnostic
        .path
        .as_ref()
        .map(|path| format!("\"{}\"", json_escape(&display_path(config, path))))
        .unwrap_or_else(|| "null".to_string());
    let line = diagnostic
        .line
        .map(|line| line.to_string())
        .unwrap_or_else(|| "null".to_string());
    let sites = if diagnostic.sites.is_empty() {
        "null".to_string()
    } else {
        let values = diagnostic
            .sites
            .iter()
            .map(|site| {
                format!(
                    "{{\"path\":\"{}\",\"line\":{}}}",
                    json_escape(&display_path(config, &site.path)),
                    site.line
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", values)
    };
    format!(
        "{{\"severity\":\"{}\",\"path\":{},\"line\":{},\"code\":\"{}\",\"message\":\"{}\",\"sites\":{}}}",
        severity,
        path,
        line,
        diagnostic.code,
        json_escape(&diagnostic.message),
        sites
    )
}

fn print_bare_query_json(config: &Config, code: &'static str, message: &str) {
    let diagnostic = Diagnostic {
        code,
        path: None,
        line: None,
        message: message.to_string(),
        sites: Vec::new(),
    };
    eprintln!("{}", render_diagnostic_json(config, "error", &diagnostic));
}

fn show_query_error_code(message: &str) -> &'static str {
    if message.starts_with("ID not found:") {
        "not-found"
    } else if message.starts_with("section not found:") {
        "missing-section"
    } else if message.starts_with("invalid ID") {
        "invalid-id"
    } else if message.starts_with("ambiguous ID:") {
        "ambiguous"
    } else if message.starts_with("broken stub:") {
        "broken-stub"
    } else {
        "query-failed"
    }
}

fn json_escape(raw: &str) -> String {
    let mut escaped = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other if other.is_control() => escaped.push_str(&format!("\\u{:04x}", other as u32)),
            other => escaped.push(other),
        }
    }
    escaped
}

/// Render a path the way reports show it: relative to the repo root by default,
/// or relative to the CLI base directory when `[output] relative_paths = false`
/// (§FS-config.3.6, §FS-errors.4 — never an absolute path outside the root).
fn display_path(config: &Config, path: &Path) -> String {
    let base = if config.relative_paths {
        &config.root
    } else {
        &config.cli_base
    };
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

/// The CLI-level warning `check` reports when the tree walk matched no files
/// (§FS-check.2.2): a scan that read nothing is almost always a misconfigured
/// scope, so we say so instead of printing nothing and exiting `0`. This is a
/// warning — it never changes the exit code.
fn empty_scan_warning(config: &Config, path: &Path, path_provided: bool) -> Diagnostic {
    // `gnd`, `gnd check .`, and `gnd check <repo-root>` all walk `[scan] include`
    // relative to the config root — so the "looked under include" message is the
    // accurate one whenever the requested path *is* that root, not just when the
    // path was omitted.
    let scoped_to_root = !path_provided
        || path == Path::new(".")
        || fs::canonicalize(path)
            .map(|p| p == config.root)
            .unwrap_or(false);
    let message = match (&config.include, scoped_to_root) {
        (Some(dirs), true) => format!(
            "nothing to scan — gnd looked under [scan] include = [{}] and found no files. Run \
             `gnd init --docs` to scaffold the canonical docs/ and e2e/ trees, point `[scan] \
             include` in `.agents/gnd.toml` at your sources, or pass a path explicitly \
             (`gnd check <dir>`).",
            dirs.iter()
                .map(|dir| format!("\"{dir}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => format!(
            "nothing to scan — no files under `{}` matched gnd's extensions ({}).",
            path.display(),
            config.extensions.join(", ")
        ),
    };
    Diagnostic {
        code: "empty-scan",
        path: None,
        line: None,
        message,
        sites: Vec::new(),
    }
}

/// `gnd check [path] [--format text|json]` — the default subcommand (§FS-cli.1):
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
            "--format=json" => format_override = Some("json".to_string()),
            "--format=text" => format_override = Some("text".to_string()),
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
    // when no source file is scanned, so a missing/stale `agents.md` block still
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

/// `gnd show <ID>[.<section>] [--head|--full] [--section S] [--format text|md|json]`
/// — print the body of one declaration (§FS-show.1): the whole thing by default
/// (§FS-show.2.1), the lead paragraph with `--head` (§FS-show.2.1.1), one
/// subsection with `.<section>` or `--section` (§FS-show.2.2). Ambiguous IDs and
/// missing IDs/sections exit `1` with a hint (§FS-show.2.2.1, §FS-show.3).
fn command_show(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("error: show requires an ID");
        return ExitCode::from(2);
    }
    let mut id_arg = None;
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut head = false;
    let mut saw_head = false;
    let mut saw_full = false;
    let mut section_override = None;
    let mut format = "text".to_string();
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--head" => {
                saw_head = true;
                head = true;
            }
            "--full" => {
                saw_full = true;
                head = false;
            }
            "--format=json" => format = "json".to_string(),
            "--format=text" => format = "text".to_string(),
            "--format=md" => format = "md".to_string(),
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
    if saw_head && saw_full {
        eprintln!("error: --head and --full cannot be used together");
        return ExitCode::from(2);
    }
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
                    "hint: this repo's [id] format is `{}` (run `gnd config show`); `gnd list` shows the IDs that exist",
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
        head,
        format == "md",
    ) {
        Ok(output) => {
            if format == "json" {
                if let Some(json) = output.json {
                    println!("{json}");
                } else {
                    println!(
                        "{{\"id\":\"{}\",\"section\":{},\"body\":\"{}\",\"path\":\"{}\",\"line\":{}}}",
                        json_escape(&render_id(&config, &id)),
                        match section.as_deref() {
                            Some(section) => format!("\"{}\"", json_escape(section)),
                            None => "null".to_string(),
                        },
                        json_escape(&output.body),
                        json_escape(&display_path(&config, &output.path)),
                        output.line
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
                        "hint: run `gnd list` to see every declared ID, or `gnd name <KIND> \"<title>\"` to propose a new one"
                    );
                } else if message.starts_with("section not found:") {
                    eprintln!(
                        "hint: run `gnd show {}` to print the whole declaration with its section numbers",
                        render_id(&config, &id)
                    );
                }
            }
            ExitCode::FAILURE
        }
    }
}

/// Resolve an `Id` (and optional section) to its body: reject ambiguous IDs with
/// more than one non-stub home (§FS-show.2.2.1), follow an inline-spec stub to its source file
/// — erroring if the stub is broken (§FS-show.2.3.4) — dispatch e2e cases to
/// `show_e2e_case` (§FS-show.2.4), and otherwise extract the heading body
/// (§FS-show.2.1, §FS-show.2.3).
fn show_declaration(
    config: &Config,
    findings: &Findings,
    id: &Id,
    section: Option<&str>,
    head: bool,
    include_heading: bool,
) -> Result<ShowOutput> {
    let root = &config.root;
    let decls = findings
        .declarations
        .get(id)
        .ok_or_else(|| anyhow!("ID not found: {}", render_id(config, id)))?;
    let homes: Vec<&Declaration> = decls
        .iter()
        .filter(|decl| !is_stub_for_inline_decl(root, decl, decls))
        .collect();
    if homes.len() > 1 {
        let mut sites: Vec<String> = homes
            .iter()
            .map(|d| format!("{}:{}", display_path(config, &d.file), d.line))
            .collect();
        sites.sort();
        return Err(anyhow!(
            "ambiguous ID: {} (declared at {})",
            render_id(config, id),
            sites.join(", ")
        ));
    }
    let decl = decls.iter().find(|decl| decl.is_stub).unwrap_or(&decls[0]);
    if let Some(case) = &decl.e2e_case {
        return show_e2e_case(config, id, case, section, head);
    }
    let file = if let Some(target) = &decl.defined_in {
        if target.is_absolute() {
            target.clone()
        } else {
            root.join(target)
        }
    } else {
        decl.file.clone()
    };
    if decl.is_stub {
        if !file.exists() {
            return Err(anyhow!(
                "broken stub: {} (stub at {}:{} points at {}, which does not exist)",
                render_id(config, id),
                display_path(config, &decl.file),
                decl.line,
                decl.defined_in.as_ref().unwrap().display()
            ));
        }
        if !file_declares_inline_home(&file, id, &config.grammar).unwrap_or(false) {
            return Err(anyhow!(
                "broken stub: {} (stub at {}:{} points at {}, which contains no inline declaration of {})",
                render_id(config, id),
                display_path(config, &decl.file),
                decl.line,
                decl.defined_in.as_ref().unwrap().display(),
                render_id(config, id)
            ));
        }
    }
    extract_declaration_body(&file, id, section, head, include_heading, config)
}

/// Render an e2e case as a `gnd show` body: the invocation, expected exit, and
/// fixture list (or just the invocation with `--head`), plus the JSON shape — the
/// case manifest of §FS-show.2.4. E2E declarations have no sections, so any
/// `.<section>` is "section not found".
fn show_e2e_case(
    config: &Config,
    id: &Id,
    case: &E2eCase,
    section: Option<&str>,
    head: bool,
) -> Result<ShowOutput> {
    if let Some(section) = section {
        return Err(anyhow!(
            "section not found: {}{}{}",
            render_id(config, id),
            config.section_separator,
            section
        ));
    }
    let invocation = format!("gnd {}", case.args.join(" "));
    let body = if head {
        format!("{invocation}\n")
    } else {
        let mut lines = vec![
            invocation,
            format!("expected exit: {}", case.expected_exit),
            "fixtures:".to_string(),
        ];
        lines.extend(
            case.fixtures
                .iter()
                .map(|path| format!("- {}", path.display())),
        );
        format!("{}\n", lines.join("\n"))
    };
    let args_json = case
        .args
        .iter()
        .map(|arg| format!("\"{}\"", json_escape(arg)))
        .collect::<Vec<_>>()
        .join(",");
    let fixtures_json = case
        .fixtures
        .iter()
        .map(|path| format!("\"{}\"", json_escape(&path.display().to_string())))
        .collect::<Vec<_>>()
        .join(",");
    let json = format!(
        "{{\"id\":\"{}\",\"kind\":\"E2E\",\"path\":\"{}\",\"args\":[{}],\"expected_exit\":{},\"fixtures\":[{}]}}",
        json_escape(&render_id(config, id)),
        json_escape(&display_path(config, &case.dir)),
        args_json,
        case.expected_exit,
        fixtures_json
    );
    Ok(ShowOutput {
        body,
        path: case.dir.clone(),
        line: 1,
        json: Some(json),
    })
}

/// Pull the body text of a declaration out of its file: the lines under the
/// `# <ID>: …` heading down to the next same-or-shallower heading (§FS-show.2.1),
/// optionally just one numbered subsection (§FS-show.2.2) or just the lead
/// paragraph (§FS-show.2.1.1). For an inline declaration in a code/`"""` doc-comment
/// this walks the comment block (§FS-show.2.3.1) and strips comment markers
/// (§FS-show.2.3.2) before returning the text.
fn extract_declaration_body(
    path: &Path,
    id: &Id,
    section: Option<&str>,
    head: bool,
    include_heading: bool,
    config: &Config,
) -> Result<ShowOutput> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let is_md = path.extension().and_then(|e| e.to_str()) == Some("md");
    let is_py = path.extension().and_then(|e| e.to_str()) == Some("py");
    let mut in_decl = false;
    let mut line_style_comment = false;
    let mut in_py_docstring = false;
    let mut found_section = section.is_none();
    let mut target_depth = usize::MAX;
    let mut lines = Vec::new();
    let mut output_line = 1;

    for (idx, line) in text.lines().enumerate() {
        let lineno = idx + 1;
        let trimmed = line.trim_start();
        if config.docstring_python
            && is_py
            && (trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''"))
        {
            if in_decl && in_py_docstring {
                break;
            }
            in_py_docstring = !in_py_docstring;
            continue;
        }
        let scan_line = if in_py_docstring { trimmed } else { line };
        if let Some(caps) = config.grammar.decl_re.captures(scan_line) {
            let found = parse_id(&caps);
            if in_decl && found.as_ref() != Some(id) {
                break;
            }
            if found.as_ref() == Some(id) {
                in_decl = true;
                line_style_comment = is_line_style_comment_line(scan_line);
                output_line = lineno;
                // `md` format keeps the heading verbatim — including for `--head`,
                // which then prints heading + lead prose (§FS-show.3.1).
                if include_heading && section.is_none() {
                    lines.push(clean_body_line(scan_line, is_md || in_py_docstring));
                }
                continue;
            }
        }
        if !in_decl {
            continue;
        }
        if !is_md {
            let blank = line.trim().is_empty();
            if in_py_docstring {
                // Python docstring content is plain Markdown; the surrounding
                // triple-quote lines are skipped above (§FS-show.2.3.2).
            } else if blank {
                // A blank line ends a line-style comment block (`//`, `#`, …);
                // inside a `/* … */` block or a docstring it is part of the body
                // (§FS-show.2.3.1).
                if line_style_comment {
                    break;
                }
            } else if !is_comment_body_line(scan_line) {
                break;
            }
        }
        if let Some(caps) = config.grammar.section_re.captures(scan_line) {
            let sec = caps.name("sec").map(|m| m.as_str()).unwrap_or("");
            match section {
                // Whole-declaration head: stop at the first numbered subsection.
                None => {
                    if head {
                        break;
                    }
                }
                Some(target) => {
                    let depth = sec.split('.').count();
                    if sec == target {
                        found_section = true;
                        target_depth = depth;
                        output_line = lineno;
                        lines.push(clean_body_line(scan_line, is_md || in_py_docstring));
                        continue;
                    }
                    // Inside the target section: a sibling-or-shallower heading
                    // ends it (§FS-show.2.2); in `--head` mode any further numbered
                    // heading — including a child — ends the section's lead prose
                    // (§FS-show.2.1.1). Before the target section is found, keep
                    // scanning past unrelated headings.
                    if found_section && (head || depth <= target_depth) {
                        break;
                    }
                }
            }
        }
        if found_section {
            lines.push(clean_body_line(scan_line, is_md || in_py_docstring));
        }
    }

    if !in_decl {
        return Err(anyhow!("ID not found: {}", render_id(config, id)));
    }
    if !found_section {
        return Err(anyhow!(
            "section not found: {}{}{}",
            render_id(config, id),
            config.section_separator,
            section.unwrap_or("")
        ));
    }
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    let body = if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    };
    Ok(ShowOutput {
        body,
        path: path.to_path_buf(),
        line: output_line,
        json: None,
    })
}

/// Strip the comment marker (`///`, `//!`, `//`, `#`, `*`, `/*`, `*/`) off a body
/// line when the declaration lives in a code/`"""` doc-comment — Markdown bodies
/// pass through unchanged (§FS-show.2.3.2).
fn clean_body_line(line: &str, is_md: bool) -> String {
    if is_md {
        return line.to_string();
    }
    let mut trimmed = line.trim_start();
    for prefix in ["///", "//!", "//", "#", "*", "/*", "*/"] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            trimmed = rest.trim_start();
            break;
        }
    }
    trimmed.trim_end_matches("*/").trim_end().to_string()
}

/// Whether a line still looks like part of the comment block — used to decide
/// where an inline declaration's body ends (§FS-show.2.3.1).
fn is_comment_body_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    ["///", "//!", "//", "#", "*", "/*", "*/"]
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
}

/// Whether a declaration heading line sits inside a *line-style* comment
/// (`//`-family, `#`, `;`, `--`) as opposed to a `/* … */` block (which opens
/// `*` continuation lines). Line-style blocks end at a blank line; block-style
/// ones end at `*/` (§FS-show.2.3.1).
fn is_line_style_comment_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//")
        || trimmed.starts_with('#')
        || trimmed.starts_with(';')
        || trimmed.starts_with("--")
}

/// `gnd fmt [path] [--check|--write] [--marker] [--md-links]` — normalize citation
/// syntax in bulk (§FS-fmt.1): rewrite the `$$` trigger to the `§` marker
/// (§FS-fmt.2.1), and with `--marker` upgrade bare ID-shaped tokens to `§`-prefixed
/// (§FS-fmt.2.2); with `--md-links` (or `[fmt.md_links] enabled`) also wrap
/// citations as Markdown links (§FS-fmt.6, §DF-md-link-emission). `--check` reports
/// without writing and exits `1` if anything would change (§FS-fmt.3).
fn command_fmt(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut write = false;
    let mut check_flag = false;
    let mut marker = false;
    let mut md_links = false;
    for arg in args {
        match arg.as_str() {
            "--check" => check_flag = true,
            "--write" => write = true,
            "--marker" => marker = true,
            "--md-links" => md_links = true,
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other => {
                if path_provided {
                    eprintln!("error: fmt takes at most one path argument");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
    }
    if write && check_flag {
        eprintln!("error: --check and --write cannot be used together");
        return ExitCode::from(2);
    }
    let config = match load_config(&path) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let md_links = md_links || (write && config.fmt_md_links_enabled);
    let changes = match fmt_tree(&config, Some(&path), path_provided, marker, md_links, write) {
        Ok(changes) => changes,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    if write {
        let mut files: Vec<PathBuf> = changes.iter().map(|(path, _, _)| path.clone()).collect();
        files.sort();
        files.dedup();
        eprintln!(
            "rewrote {} reference{}{}",
            changes.len(),
            if changes.len() == 1 { "" } else { "s" },
            if files.is_empty() { "" } else { ":" }
        );
        for path in &files {
            let count = changes.iter().filter(|(p, _, _)| p == path).count();
            eprintln!("  {} ({})", display_path(&config, path), count);
        }
    } else {
        for (path, line, label) in &changes {
            eprintln!("{}:{}: {}", display_path(&config, path), line, label);
        }
    }
    if write || changes.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Walk the tree and rewrite each scannable file line by line — never touching a
/// declaration heading or anything inside a fenced code block (§FS-fmt.2.3) — and
/// either write the changes back (`--write`) or just collect `(path, line, label)`
/// for `--check`/dry-run (§FS-fmt.3). `--md-links` needs the full `Findings` first
/// so a link is only emitted when its target resolves (§FS-fmt.6.3).
fn fmt_tree(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
    add_marker: bool,
    md_links: bool,
    write: bool,
) -> Result<Vec<(PathBuf, usize, &'static str)>> {
    let mut changes = Vec::new();
    let findings = if md_links {
        Some(scan_tree_strict(config, scope, explicit_scope)?)
    } else {
        None
    };
    for path in walk_scannable_files(config, scope, explicit_scope)? {
        let original =
            fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let is_md = path.extension().and_then(|e| e.to_str()) == Some("md");
        let mut in_fence = false;
        let mut changed_lines = Vec::new();
        let mut changed = false;
        for (idx, line) in original.lines().enumerate() {
            let trimmed = line.trim_start();
            if is_md && (trimmed.starts_with("```") || trimmed.starts_with("~~~")) {
                in_fence = !in_fence;
                changed_lines.push(line.to_string());
                continue;
            }
            if in_fence || config.grammar.decl_re.is_match(line) {
                changed_lines.push(line.to_string());
                continue;
            }
            let (new_line, label) = fmt_line(
                line,
                &path,
                config,
                add_marker,
                md_links,
                is_md,
                findings.as_ref(),
            );
            if new_line != line {
                changes.push((path.clone(), idx + 1, label));
                changed = true;
            }
            changed_lines.push(new_line);
        }
        if write && changed {
            let mut output = changed_lines.join("\n");
            if original.ends_with('\n') {
                output.push('\n');
            }
            fs::write(&path, output).with_context(|| format!("write {}", path.display()))?;
        }
    }
    Ok(changes)
}

/// Apply the `fmt` rewrites to one line, in order: trigger→marker (§FS-fmt.2.1),
/// then optionally bare→marker (§FS-fmt.2.2), then optionally Markdown-link wrapping
/// (§FS-fmt.6) — returning the new line plus a label naming the most significant
/// rewrite that fired.
fn fmt_line(
    line: &str,
    path: &Path,
    config: &Config,
    add_marker: bool,
    md_links: bool,
    is_md: bool,
    findings: Option<&Findings>,
) -> (String, &'static str) {
    let triggered = replace_trigger(line, config, is_md);
    let trigger_changed = triggered != line;
    let marked = if add_marker {
        add_markers(&triggered, config, is_md)
    } else {
        triggered.clone()
    };
    let marker_changed = marked != triggered;
    let final_line = if md_links && is_md {
        match findings {
            Some(findings) => wrap_markdown_links(&marked, path, config, findings),
            None => marked.clone(),
        }
    } else {
        marked.clone()
    };
    let link_changed = final_line != marked;
    let label = if trigger_changed {
        "trigger \u{2192} marker"
    } else if marker_changed {
        "bare \u{2192} marker"
    } else if link_changed {
        "markdown link"
    } else {
        ""
    };
    (final_line, label)
}

/// Rewrite each `$$<ID>` trigger to `§<ID>` — but only where `$$` is immediately
/// followed by a real ID-shaped token, and never inside a string literal in source
/// code or Markdown link destinations (§FS-fmt.2.1, §FS-fmt.2.3.1,
/// §DF-reference-marker).
fn replace_trigger(line: &str, config: &Config, is_md: bool) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    while let Some(relative) = line[cursor..].find(&config.trigger) {
        let start = cursor + relative;
        let after = start + config.trigger.len();
        if let Some(found) = config.grammar.citation_re.find_at(line, after)
            && found.start() == after
            && (is_md || !is_inside_string_literal(line, start))
            && (!is_md || !is_inside_inline_code(line, start))
            && (!is_md || !is_inside_markdown_link_destination(line, start))
        {
            output.push_str(&line[cursor..start]);
            output.push_str(&config.marker);
            cursor = after;
            continue;
        }
        output.push_str(&line[cursor..after]);
        cursor = after;
    }
    output.push_str(&line[cursor..]);
    output
}

/// Prefix `§` onto bare ID-shaped tokens that lack it — the `--marker` upgrade
/// (§FS-fmt.2.2) — skipping tokens already marked, Markdown inline-code examples,
/// Markdown link destinations, and source-code string literals (§FS-fmt.2.3).
fn add_markers(line: &str, config: &Config, is_md: bool) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    for found in config.grammar.citation_re.find_iter(line) {
        if line[..found.start()].ends_with(&config.marker) {
            continue;
        }
        if is_md && is_inside_inline_code(line, found.start()) {
            continue;
        }
        if is_md && is_inside_markdown_link_destination(line, found.start()) {
            continue;
        }
        if !is_md && is_inside_string_literal(line, found.start()) {
            continue;
        }
        output.push_str(&line[cursor..found.start()]);
        output.push_str(&config.marker);
        output.push_str(found.as_str());
        cursor = found.end();
    }
    output.push_str(&line[cursor..]);
    output
}

/// Whether byte offset `pos` falls inside a `'…'`, `"…"`, or `` `…` `` literal on
/// this line — the source-code exclusion that keeps an ID printed in a string from
/// being treated as a citation by the scanner or rewritten by `fmt` (§FS-fmt.2.3.1).
fn is_inside_string_literal(line: &str, pos: usize) -> bool {
    let bytes = line.as_bytes();
    let mut single = false;
    let mut double = false;
    let mut backtick = false;
    let mut i = 0;
    while i < pos && i < bytes.len() {
        match bytes[i] {
            b'\'' if !double && !backtick && !is_escaped(bytes, i) => single = !single,
            b'"' if !single && !backtick && !is_escaped(bytes, i) => double = !double,
            b'`' if !single && !double && !is_escaped(bytes, i) => backtick = !backtick,
            _ => {}
        }
        i += 1;
    }
    single || double || backtick
}

/// Whether byte offset `pos` falls inside a `` `…` `` inline-code span in Markdown
/// — citations there are illustrative, not real, so `fmt` leaves them alone
/// (§FS-fmt.2.3, §FS-fmt.6.4).
fn is_inside_inline_code(line: &str, pos: usize) -> bool {
    let bytes = line.as_bytes();
    let mut in_code = false;
    let mut i = 0;
    while i < pos && i < bytes.len() {
        if bytes[i] == b'`' && !is_escaped(bytes, i) {
            in_code = !in_code;
        }
        i += 1;
    }
    in_code
}

/// Whether byte offset `pos` falls inside the destination part of an inline
/// Markdown link (`[text](destination)`). URLs are presentation syntax, not
/// citations, so `fmt --marker` must not rewrite ID-shaped file names there
/// (§FS-fmt.2.3).
fn is_inside_markdown_link_destination(line: &str, pos: usize) -> bool {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b']' && bytes[i + 1] == b'(' && !is_escaped(bytes, i) {
            let start = i + 2;
            let mut depth = 1usize;
            let mut j = start;
            while j < bytes.len() {
                match bytes[j] {
                    b'(' if !is_escaped(bytes, j) => depth += 1,
                    b')' if !is_escaped(bytes, j) => {
                        depth -= 1;
                        if depth == 0 {
                            if pos >= start && pos < j {
                                return true;
                            }
                            i = j;
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            if j >= bytes.len() {
                return pos >= start;
            }
        }
        i += 1;
    }
    false
}

/// Wrap each `§<ID>[.<section>]` citation on this Markdown line as `[§<ID>…](url)`
/// — the `--md-links` rewrite (§FS-fmt.6.2): re-derive an existing wrapper's URL,
/// skip citations in inline code (§FS-fmt.6.4), and emit nothing when the target
/// does not resolve (§FS-fmt.6.3).
fn wrap_markdown_links(line: &str, path: &Path, config: &Config, findings: &Findings) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    for caps in config.grammar.citation_re.captures_iter(line) {
        let Some(full) = caps.get(0) else { continue };
        let marker_start = full.start().saturating_sub(config.marker.len());
        if !line[..full.start()].ends_with(&config.marker) {
            continue;
        }
        if is_inside_inline_code(line, marker_start) {
            continue;
        }
        if marker_start < cursor {
            continue;
        }
        let Some(id) = parse_id(&caps) else { continue };
        let section = caps.name("sec").map(|m| m.as_str().to_string());
        let Some(target) = markdown_link_target(path, &id, section.as_deref(), config, findings)
        else {
            continue;
        };
        let marked_end = full.end();
        let already_wrapped = marker_start > 0 && line.as_bytes()[marker_start - 1] == b'[';
        if already_wrapped && line[marked_end..].starts_with("](") {
            let url_start = marked_end + 2;
            if let Some(close_rel) = line[url_start..].find(')') {
                let close = url_start + close_rel;
                output.push_str(&line[cursor..url_start]);
                output.push_str(&target);
                cursor = close;
                continue;
            }
        }
        output.push_str(&line[cursor..marker_start]);
        let citation = &line[marker_start..marked_end];
        output.push('[');
        output.push_str(citation);
        output.push_str("](");
        output.push_str(&target);
        output.push(')');
        cursor = marked_end;
    }
    output.push_str(&line[cursor..]);
    output
}

/// Compute the link URL for a citation: a repo-relative path to the declaration's
/// home file — following an inline-spec stub to its real source file — plus a
/// heading anchor when a section is cited and the home is Markdown (§FS-fmt.6.2,
/// §DF-md-link-anchor-strategy). `None` if the ID does not resolve (§FS-fmt.6.3).
fn markdown_link_target(
    from_file: &Path,
    id: &Id,
    section: Option<&str>,
    config: &Config,
    findings: &Findings,
) -> Option<String> {
    let decls = findings.declarations.get(id)?;
    let stub = decls.iter().find(|decl| decl.is_stub);
    let home_decl = decls
        .iter()
        .find(|decl| !is_stub_for_inline_decl(&config.root, decl, decls))
        .or_else(|| decls.first())?;
    let home = if let Some(stub) = stub {
        let target = stub.defined_in.as_ref()?;
        if target.is_absolute() {
            target.clone()
        } else {
            config.root.join(target)
        }
    } else {
        home_decl.file.clone()
    };
    let rel = relative_url(from_file, &home, config);
    let is_md = home.extension().and_then(|e| e.to_str()) == Some("md");
    if !is_md || section.is_none() || config.md_link_anchor_format == "none" {
        return Some(rel);
    }
    let heading = home_decl.sections.get(section?).cloned().or_else(|| {
        section_heading_text(&home, id, section?, config)
            .ok()
            .flatten()
    })?;
    let anchor = anchor_slug(&heading, &config.md_link_anchor_format);
    Some(format!("{}#{}", rel, anchor))
}

/// `../`-style relative path from one repo file to another — the link form
/// `gnd fmt --md-links` writes (§FS-fmt.6.2).
fn relative_url(from_file: &Path, to_file: &Path, config: &Config) -> String {
    let from_rel = from_file.strip_prefix(&config.root).unwrap_or(from_file);
    let to_rel = to_file.strip_prefix(&config.root).unwrap_or(to_file);
    let from_dir = from_rel.parent().unwrap_or(Path::new(""));
    let from_components = path_components(from_dir);
    let to_components = path_components(to_rel);
    let mut common = 0;
    while common < from_components.len()
        && common < to_components.len()
        && from_components[common] == to_components[common]
    {
        common += 1;
    }
    let mut parts = Vec::new();
    for _ in common..from_components.len() {
        parts.push("..".to_string());
    }
    parts.extend(to_components[common..].iter().cloned());
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

fn path_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

/// The heading text a section anchor is built from — `<number> <title>` taken
/// straight off the heading line, since anchors are derived from heading text, not
/// stored (§DF-md-link-anchor-strategy).
fn section_anchor_text(line: &str, section: &str) -> String {
    let trimmed = line.trim_start();
    let heading = trimmed
        .trim_start_matches('#')
        .trim_start()
        .trim_start_matches(section)
        .trim_start_matches('.')
        .trim_start()
        .to_string();
    format!("{} {}", section.replace('.', ""), heading)
        .trim()
        .to_string()
}

/// Re-read a home file to find the heading text of a cited section — the fallback
/// when the section isn't already in the declaration's section map, so a link
/// anchor is always re-derived from the current heading (§FS-fmt.6.3,
/// §DF-md-link-anchor-strategy).
fn section_heading_text(
    path: &Path,
    id: &Id,
    section: &str,
    config: &Config,
) -> Result<Option<String>> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut in_decl = false;
    for line in text.lines() {
        if let Some(caps) = config.grammar.decl_re.captures(line) {
            let found = parse_id(&caps);
            if in_decl && found.as_ref() != Some(id) {
                break;
            }
            if found.as_ref() == Some(id) {
                in_decl = true;
                continue;
            }
        }
        if !in_decl {
            continue;
        }
        if let Some(caps) = config.grammar.section_re.captures(line)
            && caps.name("sec").is_some_and(|sec| sec.as_str() == section)
        {
            return Ok(Some(section_anchor_text(line, section)));
        }
    }
    Ok(None)
}

/// Slugify a heading into a fragment anchor, dispatching on the configured
/// `[fmt.md_links] anchor_format` profile (github / gitlab / mkdocs / pandoc) —
/// §FS-fmt.6.7, §DF-md-link-anchor-strategy.
fn anchor_slug(text: &str, profile: &str) -> String {
    match profile {
        "pandoc" => anchor_slug_pandoc(text),
        "mkdocs" => anchor_slug_mkdocs(text),
        "gitlab" => anchor_slug_gitlab(text),
        _ => anchor_slug_github(text),
    }
}

fn anchor_slug_github(text: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in text.nfkd() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            last_dash = false;
        } else if (lower.is_ascii_whitespace() || lower == '-') && !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn anchor_slug_gitlab(text: &str) -> String {
    // Close to GitHub for the ASCII headings gnd emits in its own specs.
    anchor_slug_github(text)
}

fn anchor_slug_mkdocs(text: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in text.nfkd() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() || lower == '_' {
            out.push(lower);
            last_dash = false;
        } else if lower.is_ascii_whitespace() && !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn anchor_slug_pandoc(text: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in text.nfkd() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() || lower == '_' || lower == '-' || lower == '.' {
            out.push(lower);
            last_dash = lower == '-';
        } else if lower.is_ascii_whitespace() && !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// `gnd config validate|show [path]` — `validate` loads the discovered config and
/// exits `1` if it is malformed (§FS-config.4.1, §FS-config.4.3); `show` prints the
/// *effective* config (file merged over defaults) as TOML (§FS-config.4.2), which
/// is also what `agents.md` and `gnd name` read for the repo's grammar.
fn command_config(args: &[String]) -> ExitCode {
    let Some(action) = args.first().map(|arg| arg.as_str()) else {
        eprintln!("error: expected `config validate` or `config show`");
        return ExitCode::from(2);
    };
    if !matches!(action, "validate" | "show") {
        if action.starts_with('-') {
            eprintln!("error: unknown flag `{action}`");
        } else {
            eprintln!("error: unknown config command `{action}`");
            eprintln!("expected: config validate, config show");
        }
        return ExitCode::from(2);
    }

    let mut path: Option<PathBuf> = None;
    for arg in &args[1..] {
        if arg.starts_with('-') {
            eprintln!("error: unknown flag `{arg}`");
            return ExitCode::from(2);
        }
        if path.is_some() {
            eprintln!("error: config {action} takes at most one path argument");
            return ExitCode::from(2);
        }
        path = Some(PathBuf::from(arg));
    }
    let path = path.unwrap_or_else(|| ".".into());

    match action {
        "validate" => match load_config(&path) {
            Ok(_) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("error: {err:#}");
                ExitCode::FAILURE
            }
        },
        "show" => match load_config(&path) {
            Ok(config) => {
                println!("gnd_config_version = 1");
                println!();
                println!("[reference]");
                println!("marker = \"{}\"", config.marker);
                println!("trigger = \"{}\"", config.trigger);
                println!("strict = {}", config.strict);
                println!("require_grounding = {}", config.require_grounding);
                println!();
                println!("[id]");
                println!("format = \"{}\"", config.id_format);
                println!("section_separator = \"{}\"", config.section_separator);
                // `number_pattern` / `slug_pattern` each govern one `[id] format`
                // placeholder — under a format that omits the placeholder the pattern
                // is dead config, so don't print it.
                if config.id_format.contains("{number}") {
                    println!(
                        "number_pattern = \"{}\"",
                        escape_toml_basic(&config.number_pattern)
                    );
                }
                if config.id_format.contains("{slug}") {
                    println!(
                        "slug_pattern = \"{}\"",
                        escape_toml_basic(&config.slug_pattern)
                    );
                }
                println!();
                for kind in &config.kinds {
                    println!("[[kinds]]");
                    println!("prefix = \"{}\"", escape_toml_basic(&kind.prefix));
                    if let Some(folder) = &kind.folder {
                        println!("folder = \"{}\"", escape_toml_basic(folder));
                    }
                    if let Some(title) = &kind.title {
                        println!("title = \"{}\"", escape_toml_basic(title));
                    }
                    println!();
                }
                println!("[scan]");
                println!(
                    "include = {}",
                    format_toml_string_list(config.include.as_deref().unwrap_or(&[]))
                );
                println!("exclude = {}", format_toml_string_list(&config.exclude));
                println!(
                    "extensions = {}",
                    format_toml_string_list(&config.extensions)
                );
                println!(
                    "comment_prefixes = {}",
                    format_toml_string_list(&config.comment_prefixes)
                );
                println!("docstring_python = {}", config.docstring_python);
                println!("respect_gitignore = {}", config.respect_gitignore);
                println!();
                println!("[output]");
                println!("format = \"{}\"", config.output_format);
                println!("color = \"auto\"");
                println!("relative_paths = {}", config.relative_paths);
                println!();
                println!("[fmt.md_links]");
                println!("enabled = {}", config.fmt_md_links_enabled);
                println!("anchor_format = \"{}\"", config.md_link_anchor_format);
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("error: {err:#}");
                ExitCode::from(2)
            }
        },
        _ => unreachable!(),
    }
}

fn format_toml_string_list(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| format!("\"{}\"", escape_toml_basic(value)))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// `gnd name <KIND> "<title>" [--width N] [--explain] [--format text|json]` —
/// propose an ID for a new declaration (§FS-name.1): derive a slug from the title
/// (§FS-name.3), the next free number for number-bearing formats (§FS-name.4),
/// check it doesn't collide with an existing declaration (§FS-name.5), and print
/// the rendered ID plus where to put it; `--explain` shows the derivation
/// (§FS-name.2.3).
fn command_name(args: &[String]) -> ExitCode {
    let mut positional = Vec::new();
    let mut width = 3usize;
    let mut format = "text".to_string();
    let mut explain = false;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--explain" => explain = true,
            "--width" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --width requires a value");
                    return ExitCode::from(2);
                }
                width = match args[idx].parse::<usize>() {
                    Ok(value) => value,
                    Err(_) => {
                        eprintln!("error: --width requires a positive integer");
                        return ExitCode::from(2);
                    }
                };
            }
            "--format=json" => format = "json".to_string(),
            "--format=text" => format = "text".to_string(),
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
            other => positional.push(other.to_string()),
        }
        idx += 1;
    }
    if positional.len() < 2 {
        eprintln!("error: name requires <KIND> and <title>");
        return ExitCode::from(2);
    }
    if positional.len() > 3 {
        eprintln!("error: name takes <KIND>, <title>, and at most one path argument");
        return ExitCode::from(2);
    }
    if !matches!(format.as_str(), "text" | "json") {
        eprintln!("error: unsupported name format `{format}`");
        return ExitCode::from(2);
    }
    let kind = &positional[0];
    let title = &positional[1];
    let path_provided = positional.get(2).is_some();
    let path = positional
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let config = match load_config(&path) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let kind_config = match config
        .kinds
        .iter()
        .find(|candidate| &candidate.prefix == kind)
    {
        Some(kind_config) => kind_config,
        None => {
            eprintln!("error: unknown kind `{kind}`");
            eprintln!("known kinds: {}", kind_prefixes(&config.kinds).join(", "));
            return ExitCode::from(2);
        }
    };
    let slug = slugify_title(title, &config.slug_pattern);
    if slug.is_empty() {
        eprintln!("title produces empty slug after normalization: \"{title}\"");
        return ExitCode::FAILURE;
    }
    let findings = match scan_tree_strict(&config, Some(&path), path_provided) {
        Ok(findings) => findings,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let uses_number = config.id_format.contains("{number}");
    let number = if uses_number {
        let max = findings
            .declarations
            .keys()
            .filter(|id| &id.kind == kind)
            .filter_map(|id| id.num)
            .max()
            .unwrap_or(0);
        Some(max + 1)
    } else {
        None
    };
    let id = Id {
        kind: kind.clone(),
        num: number,
        slug: if config.id_format.contains("{slug}") {
            Some(slug.clone())
        } else {
            None
        },
    };
    if let Some(decls) = findings.declarations.get(&id)
        && let Some(decl) = decls.first()
    {
        eprintln!(
            "proposed ID `{}` already declared at {}:{}",
            format_id(&id, &config, width),
            display_path(&config, &decl.file),
            decl.line
        );
        return ExitCode::FAILURE;
    }
    let rendered = format_id(&id, &config, width);
    if format == "json" {
        let folder = kind_config.folder.as_deref().unwrap_or("");
        println!(
            "{{\"id\":\"{}\",\"kind\":\"{}\",\"number\":{},\"slug\":\"{}\",\"folder\":\"{}\"}}",
            json_escape(&rendered),
            json_escape(kind),
            number
                .map(|number| number.to_string())
                .unwrap_or_else(|| "null".to_string()),
            json_escape(&slug),
            json_escape(folder)
        );
    } else {
        println!("{rendered}");
        if explain {
            match kind_config.folder.as_deref() {
                Some(folder) if kind == "E2E" => {
                    let case_dir = e2e_case_dir_name(&config, &rendered);
                    eprintln!(
                        "next: create the case directory at {folder}/{case_dir}/ with expected.exit and fixtures, then cite it as §{rendered}"
                    );
                }
                Some(folder) => eprintln!(
                    "next: write the declaration at {folder}/{rendered}.md  (H1: `# {rendered}: <one-line statement>`), then cite it as §{rendered}"
                ),
                None => eprintln!(
                    "next: write the declaration with H1 `# {rendered}: <one-line statement>`, then cite it as §{rendered}"
                ),
            }
        }
    }
    ExitCode::SUCCESS
}

/// `gnd refs <ID>[.<section>] [--format text|json]` — the reverse of `gnd show`:
/// list every place that cites the ID (§FS-refs.1, §FS-refs.2), scheme-aware where
/// a grep cannot be. Shares the scanner with `check` so the two never disagree on
/// what counts as a citation (§FS-refs.5). Empty results, including undeclared IDs
/// with no citations, exit `0` (§FS-refs.4).
fn command_refs(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("error: refs requires an ID");
        return ExitCode::from(2);
    }
    let mut id_arg = None;
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut section_override: Option<String> = None;
    let mut format_override: Option<String> = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--section" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --section requires a value");
                    return ExitCode::from(2);
                }
                section_override = Some(args[idx].clone());
            }
            "--format=json" => format_override = Some("json".to_string()),
            "--format=text" => format_override = Some("text".to_string()),
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
            other if id_arg.is_none() => id_arg = Some(other.to_string()),
            other => {
                if path_provided {
                    eprintln!("error: refs takes an ID and at most one path argument");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }
    let Some(id_arg) = id_arg else {
        eprintln!("error: refs requires an ID");
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
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    if section_override.is_some() && inline_section.is_some() {
        eprintln!("error: --section cannot be combined with an inline section");
        return ExitCode::from(2);
    }
    let section = section_override.or(inline_section);
    let format = format_override.unwrap_or_else(|| config.output_format.clone());
    if !matches!(format.as_str(), "text" | "json") {
        eprintln!("error: unsupported refs format `{format}`");
        return ExitCode::from(2);
    }
    let (findings, scan_errors) = match scan_tree(&config, Some(&path), path_provided) {
        Ok(out) => out,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let mut citations = findings
        .citations
        .iter()
        .filter(|citation| citation.id == id)
        .filter(|citation| {
            section
                .as_deref()
                .map(|expected| citation.section.as_deref() == Some(expected))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();
    citations.sort_by(|a, b| {
        (a.file.as_os_str(), a.line, a.column).cmp(&(b.file.as_os_str(), b.line, b.column))
    });
    // §FS-refs.2: zero citations is a normal answer, not an error — but if the ID
    // is *also* undeclared, the caller most likely fat-fingered it, so leave a
    // breadcrumb on stderr without changing the exit code.
    if citations.is_empty() && !findings.declarations.contains_key(&id) {
        eprintln!(
            "note: {} is neither declared nor cited — run `gnd list` to see every declared ID",
            render_id(&config, &id)
        );
    }
    if format == "json" {
        for citation in citations {
            println!("{}", render_citation_json(&config, citation));
        }
    } else {
        for citation in citations {
            eprintln!(
                "{}:{}: {}",
                display_path(&config, &citation.file),
                citation.line,
                citation.text
            );
        }
    }
    if scan_errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        // Partial-scan semantics (§FS-refs.4 / §FS-check.2): the listed citations
        // are real but the view of the tree was incomplete.
        for (file, message) in scan_errors {
            eprintln!("error: {}: {}", display_path(&config, &file), message);
        }
        ExitCode::from(2)
    }
}

fn render_citation_json(config: &Config, citation: &Citation) -> String {
    format!(
        "{{\"path\":\"{}\",\"line\":{},\"column\":{},\"id\":\"{}\",\"section\":{},\"marker\":{},\"text\":\"{}\"}}",
        json_escape(&display_path(config, &citation.file)),
        citation.line,
        citation.column,
        json_escape(&render_id(config, &citation.id)),
        citation
            .section
            .as_deref()
            .map(|section| format!("\"{}\"", json_escape(section)))
            .unwrap_or_else(|| "null".to_string()),
        citation.has_marker,
        json_escape(&citation.text)
    )
}

/// `gnd cover [path] [--format text|json]` — expose the citation graph grouped by
/// scanned file (§FS-cover): no new scan logic, no git diff, just the same
/// `Findings` data `check` and `refs` already consume.
fn command_cover(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut format_override: Option<String> = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--format=json" => format_override = Some("json".to_string()),
            "--format=text" => format_override = Some("text".to_string()),
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
    let config = match load_config(&path) {
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
        by_file
            .entry(citation.file.clone())
            .or_default()
            .push(citation);
    }
    for citations in by_file.values_mut() {
        citations.sort_by_key(|c| (c.line, c.column));
    }

    if format == "json" {
        for (file, citations) in &by_file {
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
        for (file, citations) in &by_file {
            println!("{}:", display_path(&config, file));
            if citations.is_empty() {
                println!("  (no citations)");
            } else {
                for citation in citations {
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

/// `gnd list [path] [--kind K] [--unused] [--format text|json]` — print every
/// declared ID with its home `path:line` and one-line title (§FS-list.1,
/// §FS-list.3), optionally filtered to one kind or to declarations nothing cites
/// (the same set as the §FS-check.4.1 warning). The discovery side of the loop:
/// how an agent finds the right `<ID>` before citing it (§FS-list.5).
fn command_list(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut kind_filter: Option<String> = None;
    let mut unused_only = false;
    let mut format_override: Option<String> = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--unused" => unused_only = true,
            "--kind" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --kind requires a value");
                    return ExitCode::from(2);
                }
                kind_filter = Some(args[idx].clone());
            }
            other if other.starts_with("--kind=") => {
                kind_filter = Some(other.trim_start_matches("--kind=").to_string());
            }
            "--format=json" => format_override = Some("json".to_string()),
            "--format=text" => format_override = Some("text".to_string()),
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
                    eprintln!("error: list takes at most one path argument");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }
    let config = match load_config(&path) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };
    let format = format_override.unwrap_or_else(|| config.output_format.clone());
    if !matches!(format.as_str(), "text" | "json") {
        eprintln!("error: unsupported list format `{format}`");
        return ExitCode::from(2);
    }
    if let Some(kind) = &kind_filter
        && !config
            .kinds
            .iter()
            .any(|candidate| &candidate.prefix == kind)
    {
        eprintln!("error: unknown kind `{kind}`");
        eprintln!("known kinds: {}", kind_prefixes(&config.kinds).join(", "));
        return ExitCode::from(2);
    }
    let (findings, scan_errors) = match scan_tree(&config, Some(&path), path_provided) {
        Ok(out) => out,
        Err(err) => {
            eprintln!("error: {err:#}");
            return ExitCode::from(2);
        }
    };

    let mut ref_counts: BTreeMap<&Id, usize> = BTreeMap::new();
    for citation in &findings.citations {
        *ref_counts.entry(&citation.id).or_insert(0) += 1;
    }

    struct Entry<'a> {
        id: &'a Id,
        home: &'a Declaration,
        duplicate: bool,
        refs: usize,
    }
    // `findings.declarations` is a BTreeMap keyed by `Id`, so the catalog comes
    // out in the same stable order `gnd check` reports diagnostics in.
    let mut entries: Vec<Entry> = Vec::new();
    for (id, decls) in &findings.declarations {
        if let Some(kind) = &kind_filter
            && &id.kind != kind
        {
            continue;
        }
        let refs = ref_counts.get(id).copied().unwrap_or(0);
        if unused_only && refs > 0 {
            continue;
        }
        // A stub paired with the inline declaration it points at is *one* home,
        // not two — collapse it the way `show` does (§FS-show.2.2.1). What's left
        // is one home in a healthy repo, more only when §FS-check.3.3 (duplicate
        // declaration) applies.
        let mut homes: Vec<&Declaration> = decls
            .iter()
            .filter(|decl| !is_stub_for_inline_decl(&config.root, decl, decls))
            .collect();
        homes.sort_by(|a, b| (a.file.as_os_str(), a.line).cmp(&(b.file.as_os_str(), b.line)));
        let duplicate = homes.len() > 1;
        for home in homes {
            entries.push(Entry {
                id,
                home,
                duplicate,
                refs,
            });
        }
    }

    if format == "json" {
        for entry in &entries {
            println!(
                "{{\"id\":\"{}\",\"kind\":\"{}\",\"path\":\"{}\",\"line\":{},\"title\":{},\"stub\":{},\"defines\":{},\"refs\":{},\"duplicate\":{}}}",
                json_escape(&render_id(&config, entry.id)),
                json_escape(&entry.id.kind),
                json_escape(&display_path(&config, &entry.home.file)),
                entry.home.line,
                entry
                    .home
                    .title
                    .as_deref()
                    .map(|title| format!("\"{}\"", json_escape(title)))
                    .unwrap_or_else(|| "null".to_string()),
                entry.home.is_stub,
                entry
                    .home
                    .defined_in
                    .as_ref()
                    .map(|target| format!("\"{}\"", json_escape(&target.display().to_string())))
                    .unwrap_or_else(|| "null".to_string()),
                entry.refs,
                entry.duplicate,
            );
        }
    } else {
        let id_width = entries
            .iter()
            .map(|entry| render_id(&config, entry.id).chars().count())
            .max()
            .unwrap_or(0)
            .min(40);
        for entry in &entries {
            let id_text = render_id(&config, entry.id);
            let location = format!(
                "{}:{}",
                display_path(&config, &entry.home.file),
                entry.home.line
            );
            let mut note = if entry.home.is_stub {
                entry
                    .home
                    .defined_in
                    .as_ref()
                    .map(|target| format!("→ {}", target.display()))
                    .unwrap_or_default()
            } else {
                entry.home.title.clone().unwrap_or_default()
            };
            if entry.duplicate {
                if note.is_empty() {
                    note = "(duplicate declaration — gnd check)".to_string();
                } else {
                    note.push_str("  (duplicate declaration — gnd check)");
                }
            }
            if note.is_empty() {
                println!("{id_text:<id_width$}  {location}");
            } else {
                println!("{id_text:<id_width$}  {location}  {note}");
            }
        }
    }

    if scan_errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        // Partial-scan semantics (§FS-check.2): the listed declarations are real
        // but the view of the tree was incomplete, so the catalog may be short.
        for (file, message) in scan_errors {
            eprintln!("error: {}: {}", display_path(&config, &file), message);
        }
        ExitCode::from(2)
    }
}

/// `gnd complete <subcommand>` — the namespace for internal completion helpers
/// the generated shell scripts call (§FS-completions.2).
fn command_complete(args: &[String]) -> ExitCode {
    match args.first().map(|arg| arg.as_str()) {
        Some("ids") => command_complete_ids(&args[1..]),
        _ => {
            eprintln!("error: expected `complete ids`");
            ExitCode::from(2)
        }
    }
}

/// `gnd complete ids [--prefix P] [--sections] [path]` — the dynamic helper a
/// shell completion calls on every tab press (§FS-completions.2): emit declared
/// IDs (or `ID.section` candidates) matching the prefix, one per line. Scan/config
/// failures exit `0` silently so a broken repo never smears diagnostics across the
/// prompt; output is deterministic (§FS-completions.3).
fn command_complete_ids(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut prefix = String::new();
    let mut force_sections = false;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--prefix" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --prefix requires a value");
                    return ExitCode::from(2);
                }
                prefix = args[idx].clone();
            }
            other if other.starts_with("--prefix=") => {
                prefix = other.trim_start_matches("--prefix=").to_string();
            }
            "--sections" => force_sections = true,
            "--path" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --path requires a value");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(&args[idx]);
                path_provided = true;
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other => {
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }

    // Completion is called on every tab press. Config or scan failures must not
    // smear diagnostics across the prompt; explicit flag misuse above is still a
    // normal CLI error because it is a bug in the installed completion script.
    let config = match load_config(&path) {
        Ok(config) => config,
        Err(_) => return ExitCode::SUCCESS,
    };
    let (findings, _) = match scan_tree(&config, Some(&path), path_provided) {
        Ok(out) => out,
        Err(_) => return ExitCode::SUCCESS,
    };

    let complete_sections = force_sections || prefix.contains(&config.section_separator);
    let mut candidates = BTreeSet::new();
    for (id, decls) in &findings.declarations {
        let rendered = render_id(&config, id);
        if complete_sections {
            for decl in decls {
                for section in decl.sections.keys() {
                    candidates.insert(format!(
                        "{}{}{}",
                        rendered, config.section_separator, section
                    ));
                }
            }
        } else {
            candidates.insert(rendered);
        }
    }

    for candidate in candidates {
        if candidate.starts_with(&prefix) {
            println!("{candidate}");
        }
    }
    ExitCode::SUCCESS
}

/// `gnd completions <bash|zsh|fish>` — print the completion script for one shell
/// to stdout, ready to `source` (§FS-completions.1, §FS-completions.4). The scripts
/// call back into `gnd complete ids` for the dynamic ID list.
fn command_completions(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("error: completions requires <bash|zsh|fish>");
        return ExitCode::from(2);
    }
    if args.len() > 1 {
        eprintln!("error: completions takes exactly one shell argument");
        return ExitCode::from(2);
    }
    match args[0].as_str() {
        "bash" => {
            print_bash_completion();
            ExitCode::SUCCESS
        }
        "zsh" => {
            print_zsh_completion();
            ExitCode::SUCCESS
        }
        "fish" => {
            print_fish_completion();
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("error: unsupported shell `{other}`");
            eprintln!("known shells: bash, zsh, fish");
            ExitCode::from(2)
        }
    }
}

/// The bash completion script: subcommand + flag completion, with `gnd show` /
/// `gnd refs` ID arguments wired to `gnd complete ids` (§FS-completions.1,
/// §FS-completions.2).
fn print_bash_completion() {
    print!(
        r#"# bash completion for gnd
_gnd_complete_ids() {{
    local cur="${{COMP_WORDS[COMP_CWORD]}}"
    mapfile -t COMPREPLY < <(gnd complete ids --prefix "$cur" 2>/dev/null)
}}

_gnd() {{
    local cur="${{COMP_WORDS[COMP_CWORD]}}"
    local sub="${{COMP_WORDS[1]}}"
    COMPREPLY=()

    if [[ $COMP_CWORD -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "check show list refs cover fmt name init config agent-setup-instructions completions" -- "$cur") )
        return 0
    fi

    case "$sub" in
        show|refs)
            _gnd_complete_ids
            return 0
            ;;
    esac
}}

complete -F _gnd gnd
"#
    );
}

/// The zsh completion script — the zsh counterpart of `print_bash_completion`
/// (§FS-completions.1, §FS-completions.2).
fn print_zsh_completion() {
    println!(
        r#"#compdef gnd

_gnd_ids() {{
  local -a ids
  ids=("${{(@f)$(gnd complete ids --prefix "$words[CURRENT]" 2>/dev/null)}}")
  _describe 'gnd ids' ids
}}

_gnd() {{
  local -a commands
  commands=(
    'check:validate every reference in a repo'
    'show:print one declaration body by ID'
    'list:list declared IDs'
    'refs:list citations of an ID'
    'cover:group citations by file'
    'fmt:normalize citation syntax'
    'name:emit the next conflict-free ID'
    'init:scaffold agents.md and config'
    'config:inspect the effective config'
    'agent-setup-instructions:print the guided setup instructions for AI agents'
    'completions:print shell completion script'
  )

  if (( CURRENT == 2 )); then
    _describe 'gnd command' commands
    return
  fi

  case "$words[2]" in
    show|refs) _gnd_ids ;;
    *) _files ;;
  esac
}}

_gnd "$@"
"#
    );
}

/// The fish completion script — `complete -c gnd …` lines, ID arguments wired to
/// `gnd complete ids` (§FS-completions.1, §FS-completions.2).
fn print_fish_completion() {
    println!(
        r#"# fish completion for gnd
function __gnd_complete_ids
    set -l token (commandline -ct)
    gnd complete ids --prefix "$token" 2>/dev/null
end

complete -c gnd -f -n "__fish_use_subcommand" -a "check show list refs cover fmt name init config agent-setup-instructions completions"
complete -c gnd -f -n "__fish_seen_subcommand_from show refs" -a "(__gnd_complete_ids)"
"#
    );
}

/// The repeating character class of a slug pattern — the last `[...]` bracket
/// expression in `slug_pattern` (e.g. `[a-z0-9-]` from `[a-z0-9][a-z0-9-]*`) —
/// used when slugifying a `gnd name` title so the result fits the configured
/// `[id] slug_pattern` (§FS-name.3, §FS-config.3.2). Falls back to the canonical
/// default if the pattern has no bracket expression.
fn slug_char_class(slug_pattern: &str) -> String {
    if let Some(end) = slug_pattern.rfind(']')
        && let Some(start) = slug_pattern[..end].rfind('[')
    {
        return slug_pattern[start..=end].to_string();
    }
    "[a-z0-9-]".to_string()
}

/// Derive a slug from a `gnd name` title (§FS-name.3).
fn slugify_title(title: &str, slug_pattern: &str) -> String {
    // §FS-name.3: NFKD-normalize, drop combining marks, lower-case to ASCII, then
    // replace every run of characters outside the configured slug character class
    // with a single `-`; trim, collapse, truncate to 60 at a `-` boundary.
    let class = slug_char_class(slug_pattern);
    let valid = Regex::new(&format!("^(?:{class})$"))
        .unwrap_or_else(|_| Regex::new("^(?:[a-z0-9-])$").unwrap());
    let mut buf = [0u8; 4];
    let mut out = String::new();
    let mut last_dash = false;
    for ch in title.nfkd() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii() && valid.is_match(lower.encode_utf8(&mut buf)) {
            out.push(lower);
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    if out.len() > 60 {
        let mut truncated = out[..60].to_string();
        if let Some(cut) = truncated.rfind('-') {
            truncated.truncate(cut);
        }
        out = truncated;
    }
    out
}

/// Render an `Id` back to text under the repo's `[id] format`, zero-padding the
/// number to `width` (§FS-config.3.2, §FS-name.2 — the form `gnd name` prints and
/// every report uses).
fn format_id(id: &Id, config: &Config, width: usize) -> String {
    let mut rendered = config.id_format.clone();
    rendered = rendered.replace("{kind}", &id.kind);
    if let Some(number) = id.num {
        rendered = rendered.replace("{number}", &format!("{number:0width$}"));
    }
    if let Some(slug) = &id.slug {
        rendered = rendered.replace("{slug}", slug);
    }
    rendered
}

/// Render an `Id` at the default 3-digit number width — the form used everywhere
/// `gnd` prints an ID in a report, listing, or message (§FS-config.3.2).
fn render_id(config: &Config, id: &Id) -> String {
    format_id(id, config, 3)
}

// The scaffold templates `gnd init` writes are embedded in the binary; the
// reference copies live under `templates/` in the source tree (§FS-init.2.1).
const AGENTS_TEMPLATE: &str = include_str!("../templates/agents.md");
const GND_TOML_TEMPLATE: &str = include_str!("../templates/gnd.toml");
const RAISON_DETRE_TEMPLATE: &str = include_str!("../templates/raison-detre.md");
const GOALS_TEMPLATE: &str = include_str!("../templates/goals.md");
const E2E_README_TEMPLATE: &str = include_str!("../templates/e2e-README.md");
const FS_README_TEMPLATE: &str = include_str!("../templates/functional-spec-README.md");
const AS_README_TEMPLATE: &str = include_str!("../templates/architectural-spec-README.md");
const GITKEEP_TEMPLATE: &str = include_str!("../templates/gitkeep.md");
const AGENT_SETUP_INSTRUCTIONS: &str = include_str!("../skills/gnd-init/SKILL.md");
const AGENTS_BLOCK_VERSION: u32 = 1;
const AGENTS_APPEND_BEGIN: &str = "<!-- gnd:init:agents:v1 begin -->";
const AGENTS_APPEND_END: &str = "<!-- gnd:init:agents:v1 end -->";
const CANONICAL_AGENT_ENTRYPOINT: &str = "agents.md";
const COMPANION_AGENT_ENTRYPOINTS: &[&str] = &[
    "AGENTS.md",
    "AGENTS.override.md",
    "CLAUDE.md",
    ".claude/CLAUDE.md",
    "GEMINI.md",
    ".github/copilot-instructions.md",
];

/// The substitutions that turn `templates/agents.md` into a concrete `agents.md`
/// for a repo (§FS-init.2.3): the project name, plus the ID/marker shape taken
/// from the config `gnd init` leaves in place — so a `{kind}-{slug}` repo gets a
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
            "Bare ID-shaped tokens are ignored — `[reference] strict = true` is set in `.agents/gnd.toml`, so only `{marker}`-prefixed citations are checked."
        )
    } else {
        format!(
            "Bare ID-shaped tokens are also recognized as citations for backward compatibility; set `[reference] strict = true` in `.agents/gnd.toml` to require the `{marker}` marker (run `gnd fmt --marker` first to upgrade existing bare citations)."
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
        ("{SCAN_SCOPE}", scan_scope_summary(config)),
        ("{DECLARATION_TABLE}", declaration_table(config)),
    ]
}

fn markdown_cell(raw: &str) -> String {
    raw.replace('|', r"\|")
}

fn code_span(raw: &str) -> String {
    format!("`{}`", raw.replace('`', "\\`"))
}

fn scan_scope_summary(config: &Config) -> String {
    let include = config.include.as_deref().unwrap_or(&[]);
    let roots = if include.is_empty() {
        "the repository root".to_string()
    } else {
        include
            .iter()
            .map(|path| code_span(path))
            .collect::<Vec<_>>()
            .join(", ")
    };
    if config.exclude.is_empty() {
        roots
    } else {
        format!(
            "{roots}; excluded directories: {}",
            config
                .exclude
                .iter()
                .map(|path| code_span(path))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn declaration_table(config: &Config) -> String {
    let mut lines = vec![
        "| Kind | Home | Purpose |".to_string(),
        "|---|---|---|".to_string(),
    ];
    for kind in &config.kinds {
        let home = kind
            .folder
            .as_deref()
            .map(code_span)
            .unwrap_or_else(|| "inline / configured by convention".to_string());
        let title = kind.title.as_deref().unwrap_or("Declaration");
        lines.push(format!(
            "| `{}` | {} | {} |",
            markdown_cell(&kind.prefix),
            home,
            markdown_cell(title)
        ));
    }
    lines.join("\n")
}

/// The full generated `agents.md` for a fresh repo — the template with all
/// substitutions applied (§FS-init.2.3). Deterministic: same `gnd` version, same
/// `--name`, same effective config ⇒ byte-identical output (§FS-non-goals.13).
fn render_agents_md(name: &str, config: &Config) -> String {
    let mut rendered = AGENTS_TEMPLATE.to_string();
    for (placeholder, value) in agents_template_substitutions(name, config) {
        rendered = rendered.replace(placeholder, &value);
    }
    rendered
}

/// Just the `<!-- gnd:init:agents:vN begin -->`…`end` managed block — what `init`
/// appends to, or replaces inside, an existing `agents.md` (§FS-init.2.3).
fn render_agents_append_block(name: &str, config: &Config) -> String {
    let rendered = render_agents_md(name, config);
    let start = rendered
        .find(AGENTS_APPEND_BEGIN)
        .expect("agents template must contain append block start marker");
    let end = rendered
        .find(AGENTS_APPEND_END)
        .map(|index| index + AGENTS_APPEND_END.len())
        .expect("agents template must contain append block end marker");
    format!("{}\n", rendered[start..end].trim_end())
}

/// Existing companion agent entrypoints that should carry the same managed gnd
/// block as `agents.md` (§FS-init.2.1). A symlink to `agents.md` is already
/// covered by the canonical file and is intentionally skipped.
fn companion_agent_entrypoints(root: &Path) -> Result<Vec<PathBuf>, (PathBuf, String)> {
    let mut paths = Vec::new();
    let canonical = root.join(CANONICAL_AGENT_ENTRYPOINT);
    for rel in COMPANION_AGENT_ENTRYPOINTS {
        let path = root.join(rel);
        if !fs::symlink_metadata(&path)
            .map(|m| m.file_type())
            .is_ok_and(|t| t.is_file() || t.is_symlink())
        {
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

/// The config that `gnd init` will leave governing `target`, which the generated
/// `agents.md` must describe (§FS-init.2.3): an existing `target/.agents/gnd.toml`
/// if there is one, otherwise the defaults (exactly what `init` is about to write
/// into `target/.agents/gnd.toml`). We do **not** walk up to an ancestor's config
/// here — `init` always writes a config *in* `target`.
fn init_effective_config(target: &Path) -> Config {
    let local_config = target.join(".agents").join("gnd.toml");
    if local_config.is_file() {
        load_config(target).unwrap_or_else(|_| Config::default_for(target.to_path_buf()))
    } else {
        Config::default_for(target.to_path_buf())
    }
}

/// The generated `.agents/gnd.toml` — every default written out explicitly as a
/// teaching surface, with only `project_name` substituted (§FS-init.2.4).
fn render_gnd_toml(name: &str) -> String {
    GND_TOML_TEMPLATE.replace("{NAME}", &escape_toml_basic(name))
}

fn escape_toml_basic(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('"', "\\\"")
}

/// `gnd init [path] [--name N] [--docs] [--force|--append]` — scaffold a repo for
/// `gnd` (§FS-init.1): write `agents.md` and `.agents/gnd.toml` (and, with
/// `--docs`, the `docs/`+`e2e/` tree, §FS-init.2.1), append/update the managed
/// `agents.md` block when the file already exists (§FS-init.2.3), refuse to clobber
/// other existing files without `--force` (§FS-init.3), print a `next:` block, and
/// exit `2` on a missing target / CLI error / unsupported block version
/// (§FS-init.4). Non-interactive — every choice is a flag (§FS-non-goals.10).
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

    // §FS-init.2.3: render `agents.md` against the config `init` leaves in place,
    // so the ID-shape / kind / marker prose in it matches `.agents/gnd.toml`.
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

    let companion_entrypoints = match companion_agent_entrypoints(&target) {
        Ok(paths) => paths,
        Err((path, message)) => {
            eprintln!("error: inspect {}: {message}", path.display());
            return ExitCode::from(2);
        }
    };
    for path in companion_entrypoints {
        let rel = path
            .strip_prefix(&target)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        match update_agents_block(&path, &agents_block, &rel) {
            Ok(AgentsUpdateResult::Appended) => eprintln!("appended {rel}"),
            Ok(AgentsUpdateResult::Updated) => eprintln!("updated {rel}"),
            Ok(AgentsUpdateResult::AlreadyCurrent) => eprintln!("exists {rel}"),
            Err(err) => {
                eprintln!("error: update {}: {err}", path.display());
                return ExitCode::from(2);
            }
        }
    }

    let mut files: Vec<(&'static str, String)> =
        vec![(".agents/gnd.toml", render_gnd_toml(&resolved_name))];
    if docs {
        files.extend(docs_scaffold());
    }

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
        eprintln!("  1. run `gnd check` — a freshly scaffolded tree is clean");
        eprintln!(
            "  2. allocate an ID:  ID=$(gnd name FS \"…\")  then write  docs/functional-spec/$ID.md"
        );
        eprintln!("     (H1: `# <ID>: <one-line statement of the behavior>`)");
        eprintln!(
            "  3. cite it as §<ID> from the docs and e2e tests that depend on it, then `gnd check` again"
        );
    } else {
        eprintln!(
            "  1. re-run with --docs to scaffold docs/ and e2e/ (or create those folders yourself) — until then `gnd check` has nothing to scan"
        );
        eprintln!("  2. run `gnd check` — a scaffolded tree is clean");
        eprintln!(
            "  3. allocate an ID:  ID=$(gnd name FS \"…\")  then write  docs/functional-spec/$ID.md"
        );
    }
    eprintln!("see agents.md for the full workflow.");

    ExitCode::SUCCESS
}

/// What `init` did to an existing `agents.md`'s managed block — `appended ` (no
/// block before), `updated ` (older block replaced in place), or `exists ` (block
/// already current) — the three stderr prefixes of §FS-init.2.2 / §FS-init.2.3.
#[derive(Debug, Eq, PartialEq)]
enum AgentsUpdateResult {
    Appended,
    Updated,
    AlreadyCurrent,
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
            Ok(AgentsUpdateResult::AlreadyCurrent) => eprintln!("exists {rel}"),
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
/// (§FS-init.2.3) — leaves the file untouched when the block is already current.
fn update_agents_block(dest: &Path, block: &str, label: &str) -> Result<AgentsUpdateResult> {
    let existing = fs::read_to_string(dest)?;
    let (updated, result) = update_agents_text(&existing, block, label)?;
    if result == AgentsUpdateResult::AlreadyCurrent {
        return Ok(result);
    }
    fs::write(dest, updated)?;
    Ok(result)
}

/// The pure string transform behind `update_agents_block`: splice the current
/// managed block into `existing`, preserving everything outside the begin/end
/// markers byte-for-byte — including the block's position and any CRLF endings
/// (§FS-init.2.3.1, §FS-init.2.3.2). A newer-than-supported block is an error.
fn update_agents_text(
    existing: &str,
    block: &str,
    label: &str,
) -> Result<(String, AgentsUpdateResult)> {
    if let Some(existing_block) = find_agents_block(existing) {
        if existing_block.version == AGENTS_BLOCK_VERSION {
            return Ok((existing.to_string(), AgentsUpdateResult::AlreadyCurrent));
        }
        if existing_block.version > AGENTS_BLOCK_VERSION {
            return Err(anyhow!(
                "{label} contains newer gnd init block v{}; this binary supports v{}",
                existing_block.version,
                AGENTS_BLOCK_VERSION
            ));
        }
        let mut updated = String::with_capacity(existing.len() + block.len());
        updated.push_str(&existing[..existing_block.start]);
        updated.push_str(block.trim_end());
        updated.push_str(&existing[existing_block.end..]);
        return Ok((updated, AgentsUpdateResult::Updated));
    }

    if AGENTS_BLOCK_BEGIN.is_match(existing) {
        return Err(anyhow!(
            "{label} contains a gnd init block start without a matching end"
        ));
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

/// The byte span and `vN` version of the managed block inside an `agents.md`
/// (§FS-init.2.3) — what both `gnd init`'s update and `gnd check`'s validation
/// (§FS-check.3.5) key off.
struct AgentsBlock {
    start: usize,
    end: usize,
    version: u32,
}

/// Locate the `<!-- gnd:init:agents:vN begin -->`…`end` block in `agents.md`,
/// tolerating any whitespace (including `\r`) between marker tokens so a CRLF file
/// is still recognized (§FS-init.2.3.2, §FS-check.3.5).
fn find_agents_block(text: &str) -> Option<AgentsBlock> {
    let begin = AGENTS_BLOCK_BEGIN.captures(text)?;
    let begin_match = begin.get(0)?;
    let version = begin.name("version")?.as_str().parse::<u32>().ok()?;
    let end_match = AGENTS_BLOCK_END.find(&text[begin_match.end()..])?;
    Some(AgentsBlock {
        start: begin_match.start(),
        end: begin_match.end() + end_match.end(),
        version,
    })
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

/// The `--docs` scaffold: the canonical `docs/` tree (stub `raison-detre.md`,
/// `goals/goals.md`, `roadmap.md`, `changelog.md`, the two spec READMEs, the
/// decision `.gitkeep`s) plus an empty `e2e/` with a README — the file list of
/// §FS-init.2.1, each a minimal starter that leaves `gnd check` clean.
fn docs_scaffold() -> Vec<(&'static str, String)> {
    vec![
        ("docs/raison-detre.md", RAISON_DETRE_TEMPLATE.to_string()),
        ("docs/goals/goals.md", GOALS_TEMPLATE.to_string()),
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
            FS_README_TEMPLATE.to_string(),
        ),
        (
            "docs/architectural-spec/README.md",
            AS_README_TEMPLATE.to_string(),
        ),
        (
            "docs/decisions/architectural/.gitkeep",
            GITKEEP_TEMPLATE.to_string(),
        ),
        (
            "docs/decisions/functional/.gitkeep",
            GITKEEP_TEMPLATE.to_string(),
        ),
        ("e2e/README.md", E2E_README_TEMPLATE.to_string()),
        ("e2e/cases/.gitkeep", GITKEEP_TEMPLATE.to_string()),
    ]
}

/// `gnd --help` / `gnd help` — the top-level usage text: the subcommand list and
/// global flags (§FS-cli.2). `gnd help <cmd>` defers to `print_subcommand_help`.
fn print_help() {
    println!("gnd — ground your agents in the spec.");
    println!("Checks ID-based citations (§<ID>.<section>) across Markdown docs and source-code");
    println!("doc-comments, so every reader — human or AI — points at the same facts.");
    println!();
    println!("Usage:");
    println!(
        "  gnd [check] [PATH] [OPTIONS]      check is the default — `gnd PATH` means `gnd check PATH`"
    );
    println!(
        "  gnd <COMMAND> [ARGS] [OPTIONS]    run `gnd <COMMAND> --help` for that command's options"
    );
    println!();
    println!("Commands:");
    println!("  check    Validate every reference in a repo (the default).        e.g. gnd .");
    println!(
        "  show     Print one declaration body for agent context.            e.g. gnd show FS-login.3"
    );
    println!(
        "  list     The ID catalog: every declared ID, path:line, title.     e.g. gnd list --kind FS"
    );
    println!(
        "  refs     List every citation of an ID, as path:line.              e.g. gnd refs FS-login"
    );
    println!(
        "  cover    Group the citation graph by scanned file.                e.g. gnd cover --format json"
    );
    println!(
        "  fmt      Rewrite `$$` triggers to `§`; --marker upgrades cites.   e.g. gnd fmt --check"
    );
    println!(
        "  name     Next conflict-free ID for a new KIND declaration.        e.g. gnd name FS \"user login\""
    );
    println!(
        "  init     Scaffold agents.md + .agents/gnd.toml; idempotent.       e.g. gnd init --docs"
    );
    println!(
        "  config   Validate or show the effective .agents/gnd.toml.         e.g. gnd config show"
    );
    println!(
        "  agent-setup-instructions  Print AI setup guide.                   e.g. gnd agent-setup-instructions"
    );
    println!(
        "  completions  Print shell completion scripts.                      e.g. gnd completions bash"
    );
    println!();
    println!(
        "Options:  --format text|json   output shape; text is the default (where it applies)."
    );
    println!("          --version, -V        print version.       --help, -h   show this screen.");
    println!("Help and version go to stdout and exit 0.   Docs: docs/functional-spec/");
}

/// Per-subcommand `--help` / `help <subcommand>` page (§FS-cli.2, §FS-cli.3): what
/// it takes, every flag with a one-line example, the exit codes, and the common
/// recovery path. Goes to stdout, exit 0 — help is never an error.
fn print_subcommand_help(cmd: &str) {
    match cmd {
        "check" => {
            println!(
                "gnd check — validate every ID citation across the repo (the default subcommand)."
            );
            println!();
            println!("Usage:  gnd [check] [PATH] [--require-grounding] [--format text|json]");
            println!();
            println!(
                "PATH defaults to `.`; config (`.agents/gnd.toml`) is discovered by walking up from it."
            );
            println!(
                "With no config, gnd scans `docs/`, `e2e/`, and `src/`; set `[scan] include` to widen it."
            );
            println!("Pointing gnd at an explicit PATH scans exactly that file or directory.");
            println!("`gnd PATH` is shorthand for `gnd check PATH` — byte-for-byte equivalent.");
            println!();
            println!("Options:");
            println!(
                "  --format text|json   text (default) prints `path:line: message`; json emits NDJSON."
            );
            println!(
                "  --require-grounding  also require every source file to cite a declared ID ([reference] require_grounding)."
            );
            println!();
            println!(
                "Exit:  0 clean · 1 dangling / duplicate / unknown-section / ungrounded findings · 2 unreadable tree or CLI error."
            );
            println!();
            println!("Examples:");
            println!("  gnd                    # check the whole repo");
            println!("  gnd docs/              # check one subtree");
            println!("  gnd --format json      # machine-readable diagnostics for CI");
        }
        "show" => {
            println!(
                "gnd show — print one declaration's body by ID, so an agent pulls a single fact"
            );
            println!("into context without loading the whole document.");
            println!();
            println!(
                "Usage:  gnd show <ID>[.<section>] [PATH] [--section S] [--head|--full] [--format text|md|json] [--path PATH]"
            );
            println!();
            println!("Options:");
            println!("  --section S            show only that section path, e.g. --section 3.1");
            println!(
                "  --head                 first paragraph only       e.g. gnd show --head FS-login"
            );
            println!(
                "  --full                 the whole declaration       e.g. gnd show --full FS-login"
            );
            println!(
                "  --format text|md|json  text (default) is the body; md keeps the heading; json wraps it"
            );
            println!("  --path PATH            repo or subtree to resolve the ID in (default `.`)");
            println!();
            println!(
                "Exit:  0 printed · 1 ID not found / ambiguous / broken stub / unknown section · 2 CLI error."
            );
            println!();
            println!("Examples:");
            println!("  gnd show FS-login              # the whole declaration body");
            println!("  gnd show FS-login.3.1          # just that nested section");
            println!();
            println!(
                "ID not found? `gnd list` shows every declared ID; `gnd name <KIND> \"…\"` proposes a new one."
            );
        }
        "list" => {
            println!("gnd list — the ID catalog: every declared ID in the repo, with where it's");
            println!("declared and its one-line title. The complement of `gnd refs` (which lists");
            println!("the citations of one ID) — `list` is the index of what you can `gnd show`.");
            println!();
            println!("Usage:  gnd list [PATH] [--kind KIND] [--unused] [--format text|json]");
            println!();
            println!(
                "Output is one line per declared ID, `<ID>  <path>:<line>  <title>`, sorted by ID."
            );
            println!(
                "Stub-and-inline pairs collapse to one line; a duplicate-declared ID gets a line per home."
            );
            println!();
            println!("Options:");
            println!(
                "  --kind KIND          only IDs of that kind                e.g. gnd list --kind FS"
            );
            println!(
                "  --unused             only declarations nothing cites yet  e.g. gnd list --unused"
            );
            println!(
                "  --format text|json   text (default) is the table on stdout; json emits NDJSON (adds `refs` count)."
            );
            println!();
            println!(
                "Exit:  0 scan succeeded (an empty catalog prints nothing) · 2 unreadable tree, or an unknown --kind."
            );
            println!();
            println!("Examples:");
            println!("  gnd list                      # the whole catalog");
            println!("  gnd list --kind AS docs/      # architectural-spec IDs under docs/");
            println!("  gnd list --unused             # declarations no citation points at");
            println!(
                "  gnd list --unused --kind FS   # uncited specs only (--unused alone also lists uncited E2E cases)"
            );
        }
        "refs" => {
            println!("gnd refs — list every citation of an ID, as `path:line`, so you can see who");
            println!("depends on a declaration before you change it.");
            println!();
            println!("Usage:  gnd refs <ID>[.<section>] [PATH] [--section S] [--format text|json]");
            println!();
            println!(
                "PATH defaults to `.`. With a `.<section>` (or --section), only citations of that"
            );
            println!(
                "exact section are listed. An ID with no citations prints nothing and exits 0."
            );
            println!();
            println!("Options:");
            println!(
                "  --section S          list only citations of that section path   e.g. gnd refs FS-login --section 3"
            );
            println!(
                "  --format text|json   text (default) prints `path:line: <citation>`; json emits NDJSON."
            );
            println!();
            println!(
                "Text citation lines go to stderr (the `check` diagnostic stream — redirect `2>&1`"
            );
            println!("to pipe them); `--format json` emits NDJSON on stdout instead.");
            println!();
            println!(
                "Exit:  0 scan succeeded (with or without hits) · 2 unreadable tree or CLI error."
            );
            println!();
            println!("Examples:");
            println!("  gnd refs FS-login             # every citation of FS-login");
            println!("  gnd refs FS-login.3           # only citations of section 3");
        }
        "cover" => {
            println!("gnd cover — group the citation graph by scanned file.");
            println!();
            println!("Usage:  gnd cover [PATH] [--format text|json]");
            println!();
            println!("PATH defaults to `.`. The command runs the same scan as `check` and `refs`,");
            println!("then prints one file record with the citations found in that file.");
            println!();
            println!("Options:");
            println!(
                "  --format text|json   text (default) groups citations by file; json emits one record per file."
            );
            println!();
            println!("Exit:  0 scan succeeded · 2 unreadable tree, incomplete scan, or CLI error.");
            println!();
            println!("Examples:");
            println!("  gnd cover src/                # source files and their spec citations");
            println!("  gnd cover --format json       # machine-readable coverage index");
        }
        "fmt" => {
            println!(
                "gnd fmt — normalize citation syntax: rewrite the `$$` trigger to the `§` marker,"
            );
            println!("and optionally upgrade bare ID tokens to marker-prefixed ones.");
            println!();
            println!("Usage:  gnd fmt [PATH] [--check | --write] [--marker] [--md-links]");
            println!();
            println!("Options:");
            println!(
                "  --check        report what would change, exit 1 if anything would   e.g. gnd fmt --check"
            );
            println!(
                "  --write        apply the changes in place                           e.g. gnd fmt --write"
            );
            println!(
                "  --marker       also prefix bare `<ID>` tokens with the marker        e.g. gnd fmt --write --marker"
            );
            println!(
                "  --md-links     also wrap citations as Markdown links to their target e.g. gnd fmt --write --md-links"
            );
            println!();
            println!(
                "With neither --check nor --write, fmt prints the would-be changes and exits 1 if any (a dry run)."
            );
            println!(
                "--write prints `rewrote N references:` then one `  <path> (count)` line per file touched."
            );
            println!();
            println!(
                "Exit:  0 nothing to do, or --write succeeded · 1 changes pending (dry run / --check) · 2 unreadable tree or CLI error."
            );
        }
        "name" => {
            println!("gnd name — emit the next conflict-free ID for a new declaration of a kind.");
            println!();
            println!(
                "Usage:  gnd name <KIND> \"<title>\" [PATH] [--width N] [--explain] [--format text|json]"
            );
            println!();
            println!(
                "KIND is one of the configured prefixes (default G, FS, AS, DF, DA, E2E, RM). The title is"
            );
            println!(
                "slugified deterministically; the number is `max(existing) + 1` (holes are never filled)."
            );
            println!();
            println!("Options:");
            println!(
                "  --width N      minimum digit width for the number (default 3)   e.g. gnd name FS \"x\" --width 4"
            );
            println!(
                "  --explain      also print where to put the declaration file     e.g. gnd name FS \"x\" --explain"
            );
            println!(
                "  --format text|json   text (default) is the bare ID on stdout; json adds kind/number/slug/folder."
            );
            println!();
            println!(
                "Exit:  0 ID emitted · 1 empty slug / collision · 2 unknown kind, scan, or CLI error."
            );
            println!();
            println!("Examples:");
            println!(
                "  gnd name FS \"User can log in\"          # -> FS-007-user-can-log-in (or FS-user-can-log-in)"
            );
            println!(
                "  ID=$(gnd name FS \"User can log in\"); $EDITOR \"docs/functional-spec/$ID.md\""
            );
        }
        "init" => {
            println!(
                "gnd init — scaffold `agents.md` + `.agents/gnd.toml` (and, with --docs, the docs/ and e2e/ layout)."
            );
            println!(
                "Idempotent: re-running updates the managed `agents.md` block in place and leaves your edits alone."
            );
            println!();
            println!("Usage:  gnd init [PATH] [--docs] [--name NAME] [--force | --append]");
            println!();
            println!("Options:");
            println!(
                "  --docs         also write docs/ (raison-detre, goals, roadmap, changelog, spec READMEs) and e2e/"
            );
            println!(
                "  --name NAME    project name to interpolate (default: derived from the directory)"
            );
            println!("  --force        overwrite existing files with the canonical version");
            println!(
                "  --append       append the managed agents.md block instead of replacing an older one"
            );
            println!();
            println!(
                "Exit:  0 written / updated / already current · 2 missing target, --force+--append, or unsupported newer block."
            );
            println!();
            println!("Examples:");
            println!("  gnd init --docs                  # full first-time scaffold");
            println!("  gnd init --name \"My Service\"      # just agents.md + .agents/gnd.toml");
        }
        "config" => {
            println!(
                "gnd config — inspect the effective `.agents/gnd.toml` discovered from a path."
            );
            println!();
            println!("Usage:  gnd config <show | validate> [PATH]");
            println!();
            println!(
                "  show       print the effective config as TOML (defaults filled in for keys you didn't set)."
            );
            println!(
                "  validate   parse the discovered config and report the first error; exit 0 if it's well-formed."
            );
            println!();
            println!("PATH defaults to `.`; config is discovered by walking up from that path.");
            println!(
                "There is no `--config <file>` override — config is discovered, not pointed at (FS-cli.6)."
            );
            println!();
            println!(
                "Exit:  0 well-formed / printed · 1 `validate` found an error · 2 no subcommand, or `show` couldn't read the config."
            );
        }
        "completions" => {
            println!("gnd completions — print a shell completion script for gnd.");
            println!();
            println!("Usage:  gnd completions <bash|zsh|fish>");
            println!();
            println!("The generated scripts complete subcommands and complete declared IDs for");
            println!("`gnd show <ID>` and `gnd refs <ID>` by calling the hidden helper:");
            println!("`gnd complete ids --prefix <word>`.");
            println!();
            println!("Install examples:");
            println!("  source <(gnd completions bash)");
            println!("  gnd completions zsh > ~/.zfunc/_gnd");
            println!("  gnd completions fish > ~/.config/fish/completions/gnd.fish");
            println!();
            println!("Exit:  0 script printed · 2 unsupported shell.");
        }
        "agent-setup-instructions" => {
            println!(
                "gnd agent-setup-instructions — print the guided setup instructions for AI agents."
            );
            println!();
            println!("Usage:  gnd agent-setup-instructions");
            println!();
            println!(
                "The output is the same Markdown source shipped as `skills/gnd-init/SKILL.md`,"
            );
            println!("embedded in the binary so installed agents can discover the setup workflow");
            println!("without access to the source tree.");
            println!();
            println!("Exit:  0 instructions printed · 2 unexpected arguments.");
        }
        _ => print_help(),
    }
}

fn command_agent_setup_instructions(args: &[String]) -> ExitCode {
    if !args.is_empty() {
        eprintln!("error: agent-setup-instructions takes no arguments");
        return ExitCode::from(2);
    }
    print!("{AGENT_SETUP_INSTRUCTIONS}");
    ExitCode::SUCCESS
}

/// Restore the default `SIGPIPE` disposition (Unix only).
///
/// Rust ignores `SIGPIPE` at startup, which turns a closed downstream pipe
/// (`gnd list | head`) into an `EPIPE` on the next write — and `println!`
/// panics on a write error. A CLI in a pipeline should instead die quietly,
/// the way `ls | head` does. This is a no-op off Unix.
#[cfg(unix)]
fn restore_default_sigpipe() {
    // SIGPIPE == 13 and SIG_DFL == (void(*)(int))0 on Linux, macOS, and the BSDs.
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    unsafe extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

/// The CLI entry point: parse `argv`, dispatch to the matching `command_*`, and
/// return its `ExitCode` (§FS-cli). `gnd` with no subcommand — or with a leading
/// flag or a path — is `gnd check` (§FS-cli.1); `--version`/`--help` short-circuit
/// to stdout, exit 0 (§FS-cli.2); an unknown command exits 2 and lists the known
/// ones (§FS-cli.4). The exit-code mapping (0/1/2) is fixed (§FS-cli.5).
pub fn main_entry() -> ExitCode {
    restore_default_sigpipe();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("gnd {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let first = args.first().map(|arg| arg.as_str());
    // `gnd help [<subcommand>]` — the top-level page with no argument, that
    // subcommand's page with one, an error for an unknown name (§FS-cli.2).
    if first == Some("help") {
        return match args.get(1).map(String::as_str) {
            None => {
                print_help();
                ExitCode::SUCCESS
            }
            Some(cmd) if SUBCOMMANDS.contains(&cmd) => {
                print_subcommand_help(cmd);
                ExitCode::SUCCESS
            }
            Some(other) => {
                eprintln!("error: unknown command: {other}");
                eprintln!("known commands: {}", SUBCOMMANDS.join(", "));
                ExitCode::from(2)
            }
        };
    }
    // `--help` / `-h` short-circuits before any work; with a known subcommand
    // first it prints that subcommand's page, otherwise the top-level one.
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        match first {
            Some(cmd) if SUBCOMMANDS.contains(&cmd) => print_subcommand_help(cmd),
            _ => print_help(),
        }
        return ExitCode::SUCCESS;
    }
    match first {
        None => command_check(&[]),
        Some("check") => command_check(&args[1..]),
        Some("show") => command_show(&args[1..]),
        Some("list") => command_list(&args[1..]),
        Some("refs") => command_refs(&args[1..]),
        Some("cover") => command_cover(&args[1..]),
        Some("fmt") => command_fmt(&args[1..]),
        Some("name") => command_name(&args[1..]),
        Some("init") => command_init(&args[1..]),
        Some("config") => command_config(&args[1..]),
        Some("agent-setup-instructions") => command_agent_setup_instructions(&args[1..]),
        Some("completions") => command_completions(&args[1..]),
        Some("complete") => command_complete(&args[1..]),
        Some(other) if other.starts_with('-') => command_check(&args),
        // Any first argument that is not a known subcommand is a path argument:
        // `gnd <path>` ≡ `gnd check <path>` (§FS-cli.1). When that path doesn't
        // exist the message names both readings, so a mistyped subcommand isn't
        // misreported as a missing file (§FS-cli.1, §FS-cli.4).
        Some(other) => {
            if !Path::new(other).exists() {
                eprintln!("error: unknown command or missing path: {other}");
                eprintln!("known commands: {}", SUBCOMMANDS.join(", "));
                eprintln!(
                    "(a bare path is shorthand for `gnd check <path>`; run `gnd --help` for commands)"
                );
                return ExitCode::from(2);
            }
            command_check(&args)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_root(name: &str) -> PathBuf {
        let unique = format!(
            "{}-{}-{:?}",
            name,
            std::process::id(),
            std::thread::current().id()
        );
        let dir = std::env::temp_dir().join("gnd-lib-tests").join(unique);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create test root");
        dir
    }

    fn write(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        std::fs::write(path, text).expect("write fixture");
    }

    fn current_block() -> String {
        render_agents_append_block("demo", &Config::default_for(PathBuf::from(".")))
    }

    #[test]
    fn explicit_file_scope_ignores_unrelated_findings() {
        let root = test_root("explicit_file_scope_ignores_unrelated_findings");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );
        write(
            &root.join("docs/functional-spec/FS-002-beta.md"),
            "# FS-002-beta: Beta\n\nMentions FS-999-missing.\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(
            &config,
            Some(&root.join("docs/functional-spec/FS-001-alpha.md")),
            true,
        )
        .expect("scan scoped file");
        let report = check(&findings, &config);

        assert!(
            report.errors.is_empty(),
            "unrelated dangling citation should not be reported"
        );
    }

    #[test]
    fn scanner_ignores_bare_source_citations_inside_strings() {
        let root = test_root("scanner_ignores_bare_source_citations_inside_strings");
        write(
            &root.join("src/app.rs"),
            "fn main() {\n    let value = \"FS-999-missing\";\n}\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) =
            scan_tree(&config, Some(&root.join("src/app.rs")), true).expect("scan source file");
        let report = check(&findings, &config);

        assert!(
            report.errors.is_empty(),
            "string literal must not be a citation"
        );
    }

    #[test]
    fn require_grounding_off_by_default() {
        let root = test_root("require_grounding_off_by_default");
        write(&root.join("src/util.rs"), "pub fn helper() {}\n");

        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check(&findings, &config);

        assert!(
            !report.errors.iter().any(|e| e.code == "ungrounded"),
            "grounding is opt-in: an uncited source file is not an error by default"
        );
    }

    #[test]
    fn require_grounding_flags_uncited_source_file() {
        let root = test_root("require_grounding_flags_uncited_source_file");
        write(
            &root.join("docs/functional-spec/FS-001-login.md"),
            "# FS-001-login: Login\n",
        );
        write(
            &root.join("src/auth.rs"),
            "// §FS-001-login\npub fn login() {}\n",
        );
        write(&root.join("src/util.rs"), "pub fn helper() {}\n");

        let mut config = Config::default_for(root.clone());
        config.require_grounding = true;
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check(&findings, &config);

        let ungrounded: Vec<_> = report
            .errors
            .iter()
            .filter(|e| e.code == "ungrounded")
            .map(|e| e.path.as_deref().unwrap().to_path_buf())
            .collect();
        assert_eq!(
            ungrounded,
            vec![root.join("src/util.rs")],
            "only the uncited source file is flagged; the one citing §FS-001-login is grounded"
        );
    }

    #[test]
    fn require_grounding_accepts_inline_declaration() {
        let root = test_root("require_grounding_accepts_inline_declaration");
        write(
            &root.join("src/router.rs"),
            "// # AS-001-router: Router\n//\n// ## 1. Shape\npub struct Router;\n",
        );

        let mut config = Config::default_for(root.clone());
        config.require_grounding = true;
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check(&findings, &config);

        assert!(
            !report.errors.iter().any(|e| e.code == "ungrounded"),
            "a file that declares a spec inline is grounded in the spec it is"
        );
    }

    #[test]
    fn require_grounding_ignores_markdown() {
        let root = test_root("require_grounding_ignores_markdown");
        write(
            &root.join("docs/notes.md"),
            "# Notes\n\nNothing cited here.\n",
        );

        let mut config = Config::default_for(root.clone());
        config.require_grounding = true;
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check(&findings, &config);

        assert!(
            !report.errors.iter().any(|e| e.code == "ungrounded"),
            "the grounding rule applies to source files, not Markdown"
        );
    }

    #[test]
    fn require_grounding_treats_dangling_only_file_as_ungrounded() {
        let root = test_root("require_grounding_treats_dangling_only_file_as_ungrounded");
        write(
            &root.join("src/app.rs"),
            "// §FS-001-missing\npub fn run() {}\n",
        );

        let mut config = Config::default_for(root.clone());
        config.require_grounding = true;
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check(&findings, &config);

        assert!(
            report.errors.iter().any(|e| e.code == "dangling"),
            "the dangling citation is still its own error"
        );
        let app = root.join("src/app.rs");
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.code == "ungrounded" && e.path.as_deref() == Some(app.as_path())),
            "a file whose only citation resolves to nothing is not grounded"
        );
    }

    #[test]
    fn scanner_uses_configured_comment_prefixes() {
        let root = test_root("scanner_uses_configured_comment_prefixes");
        let mut config = Config::default_for(root.clone());
        config.comment_prefixes = vec!["//".to_string()];
        config.rebuild_grammar().expect("rebuild grammar");
        write(
            &root.join("src/router.rs"),
            "// # AS-001-router: Router\n//\n// ## 1. Shape\n",
        );

        let (findings, _) =
            scan_tree(&config, Some(&root.join("src/router.rs")), true).expect("scan source file");

        assert!(
            findings.declarations.contains_key(&Id {
                kind: "AS".to_string(),
                num: Some(1),
                slug: Some("router".to_string())
            }),
            "configured // prefix should allow inline declarations"
        );
    }

    #[test]
    fn diagnostics_render_custom_id_format() {
        let root = test_root("diagnostics_render_custom_id_format");
        write(
            &root.join(".agents/gnd.toml"),
            r#"gnd_config_version = 1

[id]
format = "{kind}_{number}_{slug}"
section_separator = "."
number_pattern = "\\d+"
slug_pattern = "[a-z0-9][a-z0-9-]*"
"#,
        );
        write(
            &root.join("docs/functional-spec/FS_001_alpha.md"),
            "# FS_001_alpha: Alpha\n\nMentions §FS_999_missing.\n",
        );
        let config = load_config(&root).expect("load config");
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check(&findings, &config);

        assert!(
            report
                .errors
                .iter()
                .any(|error| error.message == "unknown reference FS_999_missing"),
            "diagnostic should use configured ID rendering: {:?}",
            report.errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn agents_update_appends_managed_block_when_missing() {
        let (updated, result) =
            update_agents_text("# Existing agents\n", &current_block(), "agents.md")
                .expect("append block");

        assert_eq!(result, AgentsUpdateResult::Appended);
        assert!(updated.starts_with("# Existing agents\n\n"));
        assert_eq!(updated.matches(AGENTS_APPEND_BEGIN).count(), 1);
    }

    #[test]
    fn agents_update_does_not_append_current_block_twice() {
        let existing = current_block();
        let (updated, result) =
            update_agents_text(&existing, &current_block(), "agents.md").expect("current block");

        assert_eq!(result, AgentsUpdateResult::AlreadyCurrent);
        assert_eq!(updated, existing);
        assert_eq!(updated.matches(AGENTS_APPEND_BEGIN).count(), 1);
    }

    #[test]
    fn agents_update_replaces_older_block_in_place() {
        let old_block = current_block()
            .replace("gnd:init:agents:v1 begin", "gnd:init:agents:v0 begin")
            .replace("gnd:init:agents:v1 end", "gnd:init:agents:v0 end");
        let existing = format!("# Existing agents\n\n{old_block}\n\n# Local notes\n");
        let (updated, result) =
            update_agents_text(&existing, &current_block(), "agents.md").expect("update old block");

        assert_eq!(result, AgentsUpdateResult::Updated);
        assert!(updated.starts_with("# Existing agents\n\n"));
        assert!(updated.ends_with("\n\n# Local notes\n"));
        assert_eq!(updated.matches(AGENTS_APPEND_BEGIN).count(), 1);
        assert!(!updated.contains("gnd:init:agents:v0"));
    }

    #[test]
    fn agents_update_keeps_current_block_in_middle_position() {
        // §FS-init.2.3.1: a v1 block that already sits between user-authored
        // sections must be recognized as `AlreadyCurrent` and the file must not be
        // rewritten — the position of the block within the file is preserved.
        let existing = format!(
            "# Existing agents\n\n{}\n\n# Local notes\n",
            current_block()
        );
        let (updated, result) = update_agents_text(&existing, &current_block(), "agents.md")
            .expect("non-EOF current block");

        assert_eq!(result, AgentsUpdateResult::AlreadyCurrent);
        assert_eq!(
            updated, existing,
            "file must be byte-identical when current"
        );
        assert!(updated.starts_with("# Existing agents\n\n"));
        assert!(updated.ends_with("\n\n# Local notes\n"));
        assert_eq!(updated.matches(AGENTS_APPEND_BEGIN).count(), 1);
    }

    #[test]
    fn agents_update_handles_crlf_line_endings() {
        // §FS-init.2.3.2: a CRLF-encoded agents.md with a v0 block sandwiched
        // between user-authored sections must still be detected and updated, with
        // CRLF preserved outside the managed block.
        let v0_lf = current_block()
            .replace("gnd:init:agents:v1 begin", "gnd:init:agents:v0 begin")
            .replace("gnd:init:agents:v1 end", "gnd:init:agents:v0 end");
        let v0_crlf = v0_lf.replace('\n', "\r\n");
        let existing = format!("# Existing agents\r\n\r\n{v0_crlf}\r\n\r\n# Local notes\r\n");
        let (updated, result) = update_agents_text(&existing, &current_block(), "agents.md")
            .expect("update CRLF v0 block");

        assert_eq!(result, AgentsUpdateResult::Updated);
        assert!(
            updated.starts_with("# Existing agents\r\n\r\n"),
            "CRLF prefix must be preserved verbatim"
        );
        assert!(
            updated.ends_with("\r\n\r\n# Local notes\r\n"),
            "CRLF suffix must be preserved verbatim"
        );
        assert_eq!(updated.matches(AGENTS_APPEND_BEGIN).count(), 1);
        assert!(!updated.contains("gnd:init:agents:v0"));
    }

    #[test]
    fn discovers_known_companion_agent_entrypoints() {
        let root = test_root("discovers_known_companion_agent_entrypoints");
        write(&root.join("AGENTS.md"), "# Codex notes\n");
        write(&root.join("AGENTS.override.md"), "# Codex override notes\n");
        write(&root.join("CLAUDE.md"), "# Claude notes\n");
        write(&root.join(".claude/CLAUDE.md"), "# Claude project notes\n");
        write(&root.join("GEMINI.md"), "# Gemini notes\n");
        write(
            &root.join(".github/copilot-instructions.md"),
            "# Copilot notes\n",
        );

        let companions = companion_agent_entrypoints(&root).expect("discover companions");
        let rels = companions
            .iter()
            .map(|path| {
                path.strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert_eq!(
            rels,
            vec![
                "AGENTS.md",
                "AGENTS.override.md",
                "CLAUDE.md",
                ".claude/CLAUDE.md",
                "GEMINI.md",
                ".github/copilot-instructions.md"
            ]
        );
    }

    #[test]
    fn check_ignores_companion_agent_entrypoints_without_canonical_agents_md() {
        let root =
            test_root("check_ignores_companion_agent_entrypoints_without_canonical_agents_md");
        write(&root.join("AGENTS.md"), "# Project agent notes\n");
        write(
            &root.join("docs/functional-spec/FS-001-alpha.md"),
            "# FS-001-alpha: Alpha\n",
        );

        let config = Config::default_for(root.clone());
        let (findings, _) = scan_tree(&config, Some(&root), true).expect("scan root");
        let report = check(&findings, &config);

        assert!(
            report
                .errors
                .iter()
                .all(|error| error.code != "agents-init"),
            "project-owned AGENTS.md should not require a managed block without canonical agents.md"
        );
    }

    #[cfg(unix)]
    #[test]
    fn claude_symlink_to_agents_is_not_a_companion_entrypoint() {
        let root = test_root("claude_symlink_to_agents_is_not_a_companion_entrypoint");
        write(&root.join("agents.md"), &current_block());
        std::os::unix::fs::symlink("agents.md", root.join("CLAUDE.md"))
            .expect("create CLAUDE.md symlink");

        let companions = companion_agent_entrypoints(&root).expect("discover companions");

        assert!(
            companions.is_empty(),
            "CLAUDE.md symlinked to agents.md should be covered by agents.md"
        );
    }
}

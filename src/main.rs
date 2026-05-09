use anyhow::{Context, Result, anyhow};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use walkdir::WalkDir;

const SEC_GROUP: &str = r"(?P<sec>\d+(?:\.\d+)*)";
const COMMENT_PREFIX: &str = r"(?://[/!]?|#|;|--|\*|/\*)";

static SECTION_HEADING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"^\s*{}?\s*#+\s+{}\.?\s+\S",
        COMMENT_PREFIX, SEC_GROUP
    ))
    .unwrap()
});

static STUB_LINK_HEADING: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*:\s*\[[^\]]*\]\(\s*(?P<path>[^)\s]+)\s*\)\s*$").unwrap());
static AGENTS_BLOCK_BEGIN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<!--\s*gnd:init:agents:v(?P<version>\d+)\s+begin\s*-->").unwrap());
static AGENTS_BLOCK_END: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<!--\s*gnd:init:agents:v\d+\s+end\s*-->").unwrap());

/// ID grammar compiled from [id].format + [[kinds]] — the single place that knows the
/// shape of a declaration heading or a citation. Built once per config load.
#[derive(Clone)]
struct Grammar {
    decl_re: Regex,
    citation_re: Regex,
    id_input_re: Regex,
}

impl Grammar {
    fn build(
        format: &str,
        kinds: &[String],
        number_pattern: &str,
        slug_pattern: &str,
        section_separator: &str,
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

        let sep_quoted = regex::escape(section_separator);
        let sec_suffix = format!(r"(?:{}{})?", sep_quoted, SEC_GROUP);

        let decl_re = Regex::new(&format!(
            r"^\s*{}?\s*#+\s+{}\b",
            COMMENT_PREFIX, id_pat
        ))?;
        let citation_re = Regex::new(&format!(r"\b{}{}", id_pat, sec_suffix))?;
        let id_input_re = Regex::new(&format!(r"^{}{}$", id_pat, sec_suffix))?;

        Ok(Self {
            decl_re,
            citation_re,
            id_input_re,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct Id {
    kind: String,
    num: Option<u32>,
    slug: Option<String>,
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)?;
        if let Some(n) = self.num {
            write!(f, "-{:03}", n)?;
        }
        if let Some(s) = &self.slug {
            write!(f, "-{}", s)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct Declaration {
    id: Id,
    file: PathBuf,
    line: usize,
    sections: BTreeSet<String>,
    is_stub: bool,
    defined_in: Option<PathBuf>,
}

#[derive(Debug)]
struct Citation {
    id: Id,
    section: Option<String>,
    file: PathBuf,
    line: usize,
}

#[derive(Default)]
struct Findings {
    declarations: BTreeMap<Id, Vec<Declaration>>,
    citations: Vec<Citation>,
}

#[derive(Clone)]
struct Config {
    marker: String,
    trigger: String,
    strict: bool,
    include: Option<Vec<String>>,
    exclude: Vec<String>,
    extensions: Vec<String>,
    output_format: String,
    id_format: String,
    section_separator: String,
    number_pattern: String,
    slug_pattern: String,
    kinds: Vec<String>,
    grammar: Grammar,
}

const DEFAULT_KINDS: &[&str] = &["G", "FS", "AS", "DA", "DF", "E2E"];
const DEFAULT_ID_FORMAT: &str = "{kind}-{number}-{slug}";
const DEFAULT_SECTION_SEPARATOR: &str = ".";
const DEFAULT_NUMBER_PATTERN: &str = r"\d+";
const DEFAULT_SLUG_PATTERN: &str = r"[a-z0-9][a-z0-9-]*";

impl Config {
    fn default_for(_root: PathBuf) -> Self {
        let kinds: Vec<String> = DEFAULT_KINDS.iter().map(|s| s.to_string()).collect();
        let grammar = Grammar::build(
            DEFAULT_ID_FORMAT,
            &kinds,
            DEFAULT_NUMBER_PATTERN,
            DEFAULT_SLUG_PATTERN,
            DEFAULT_SECTION_SEPARATOR,
        )
        .expect("default grammar must compile");
        Self {
            marker: "§".to_string(),
            trigger: "$$".to_string(),
            strict: false,
            include: None,
            exclude: vec![
                ".git".into(),
                "target".into(),
                "node_modules".into(),
                "dist".into(),
                "build".into(),
                ".next".into(),
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
                "jsx".into(),
                "py".into(),
                "c".into(),
                "h".into(),
                "cpp".into(),
                "hpp".into(),
                "swift".into(),
                "scala".into(),
                "rb".into(),
                "sh".into(),
                "yaml".into(),
                "yml".into(),
                "toml".into(),
            ],
            output_format: "text".into(),
            id_format: DEFAULT_ID_FORMAT.into(),
            section_separator: DEFAULT_SECTION_SEPARATOR.into(),
            number_pattern: DEFAULT_NUMBER_PATTERN.into(),
            slug_pattern: DEFAULT_SLUG_PATTERN.into(),
            kinds,
            grammar,
        }
    }

    fn rebuild_grammar(&mut self) -> Result<()> {
        self.grammar = Grammar::build(
            &self.id_format,
            &self.kinds,
            &self.number_pattern,
            &self.slug_pattern,
            &self.section_separator,
        )?;
        Ok(())
    }
}

#[derive(Default)]
struct Report {
    errors: Vec<String>,
    warnings: Vec<String>,
}

fn parse_id(caps: &regex::Captures) -> Option<Id> {
    let kind = caps.name("kind")?.as_str().to_string();
    let num = match caps.name("num") {
        Some(m) => Some(m.as_str().parse().ok()?),
        None => None,
    };
    let slug = caps.name("slug").map(|m| m.as_str().to_string());
    Some(Id { kind, num, slug })
}

fn parse_id_arg(raw: &str, grammar: &Grammar) -> Result<(Id, Option<String>)> {
    let caps = grammar
        .id_input_re
        .captures(raw)
        .ok_or_else(|| anyhow!("invalid ID `{raw}`"))?;
    let id = parse_id(&caps).ok_or_else(|| anyhow!("invalid ID `{raw}`"))?;
    Ok((id, caps.name("sec").map(|m| m.as_str().to_string())))
}

fn load_config(start: &Path) -> Result<Config> {
    let start_dir = if start.is_file() {
        start.parent().unwrap_or(Path::new(".")).to_path_buf()
    } else {
        start.to_path_buf()
    };
    let mut cursor = Some(start_dir.as_path());
    while let Some(dir) = cursor {
        let candidate = dir.join(".agents").join("gnd.toml");
        if candidate.exists() {
            let mut config = Config::default_for(dir.to_path_buf());
            parse_config_file(&candidate, &mut config)?;
            return Ok(config);
        }
        cursor = dir.parent();
    }
    Ok(Config::default_for(start_dir))
}

fn parse_config_file(path: &Path, config: &mut Config) -> Result<()> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut section = String::new();
    let mut id_seen = false;
    let mut parsed_kinds: Vec<String> = Vec::new();
    let mut current_kind: Option<String> = None;
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
                "reference" | "scan" | "output" | "id" => {}
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
                    current_kind = Some(String::new());
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
            ("id", "format") => {
                config.id_format = parse_string(path, line_no, value)?;
                id_seen = true;
            }
            ("id", "section_separator") => {
                config.section_separator = parse_string(path, line_no, value)?;
                id_seen = true;
            }
            ("id", "number_pattern") => {
                config.number_pattern = parse_string(path, line_no, value)?;
                id_seen = true;
            }
            ("id", "slug_pattern") => {
                config.slug_pattern = parse_string(path, line_no, value)?;
                id_seen = true;
            }
            ("kinds", "prefix") => {
                let prefix = parse_string(path, line_no, value)?;
                if let Some(slot) = current_kind.as_mut() {
                    *slot = prefix;
                } else {
                    bail_config(
                        path,
                        line_no,
                        "`prefix` outside of [[kinds]] block".to_string(),
                    )?;
                }
            }
            ("kinds", "folder" | "title") => {
                parse_string(path, line_no, value)?;
            }
            ("scan", "include") => config.include = Some(parse_string_list(path, line_no, value)?),
            ("scan", "exclude") => config.exclude = parse_string_list(path, line_no, value)?,
            ("scan", "extensions") => config.extensions = parse_string_list(path, line_no, value)?,
            ("scan", "comment_prefixes") => {
                parse_string_list(path, line_no, value)?;
            }
            ("scan", "docstring_python" | "respect_gitignore") => {
                parse_bool(path, line_no, value)?;
            }
            ("output", "format") => config.output_format = parse_string(path, line_no, value)?,
            ("output", "color") => {
                parse_string(path, line_no, value)?;
            }
            ("output", "relative_paths") => {
                parse_bool(path, line_no, value)?;
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
        // [[kinds]] replaces defaults entirely, per FS-config.3.4.
        if parsed_kinds.iter().any(|p| p.is_empty()) {
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
        // (FS-config.3.4 — would make tokenization ambiguous).
        for (i, a) in parsed_kinds.iter().enumerate() {
            for (j, b) in parsed_kinds.iter().enumerate() {
                if i != j && a.len() <= b.len() && b.starts_with(a.as_str()) {
                    return Err(anyhow!(
                        "{}: kinds `{}` and `{}` collide (one is a prefix of the other)",
                        path.display(),
                        a,
                        b
                    ));
                }
            }
        }
        config.kinds = parsed_kinds;
    }
    if id_seen || kinds_block_seen {
        config
            .rebuild_grammar()
            .with_context(|| format!("{}: invalid [id] grammar", path.display()))?;
    }
    Ok(())
}

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

fn skip_dir(path: &Path, config: &Config) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    name.starts_with('.') || config.exclude.iter().any(|excluded| excluded == name)
}

fn scan_file(path: &Path, config: &Config, findings: &mut Findings) -> Result<()> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
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
        if is_py && (trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''")) {
            in_py_docstring = !in_py_docstring;
            continue;
        }
        let scan_line = if in_py_docstring {
            line.trim_start()
        } else {
            line
        };

        if let Some(caps) = config.grammar.decl_re.captures(scan_line) {
            if let Some(id) = parse_id(&caps) {
                if let Some(prev) = current.take() {
                    findings
                        .declarations
                        .entry(prev.id.clone())
                        .or_default()
                        .push(prev);
                }
                let mut is_stub = false;
                let mut defined_in = None;
                if is_md && in_docs {
                    let tail = &scan_line[caps.get(0).unwrap().end()..];
                    if let Some(link_caps) = STUB_LINK_HEADING.captures(tail) {
                        is_stub = true;
                        defined_in = Some(PathBuf::from(link_caps.name("path").unwrap().as_str()));
                    }
                }
                current = Some(Declaration {
                    id,
                    file: path.to_path_buf(),
                    line: lineno,
                    sections: BTreeSet::new(),
                    is_stub,
                    defined_in,
                });
                continue;
            }
        }

        if let Some(caps) = SECTION_HEADING.captures(scan_line) {
            if let Some(decl) = current.as_mut() {
                if let Some(sec) = caps.name("sec") {
                    decl.sections.insert(sec.as_str().to_string());
                }
            }
        }

        for caps in config.grammar.citation_re.captures_iter(scan_line) {
            let Some(full) = caps.get(0) else { continue };
            let has_marker = scan_line[..full.start()].ends_with(&config.marker);
            if config.strict && !has_marker {
                continue;
            }
            let Some(id) = parse_id(&caps) else { continue };
            if let Some(decl) = current.as_ref() {
                if decl.line == lineno && decl.id == id {
                    continue;
                }
            }
            findings.citations.push(Citation {
                id,
                section: caps.name("sec").map(|m| m.as_str().to_string()),
                file: path.to_path_buf(),
                line: lineno,
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

fn scan_tree(root: &Path, config: &Config) -> Result<Findings> {
    let mut findings = Findings::default();
    let roots = if let Some(include) = &config.include {
        include
            .iter()
            .map(|path| root.join(path))
            .collect::<Vec<_>>()
    } else {
        vec![root.to_path_buf()]
    };
    for scan_root in roots {
        if !scan_root.exists() {
            continue;
        }
        let walker = WalkDir::new(scan_root).into_iter().filter_entry(|e| {
            if e.depth() == 0 {
                return true;
            }
            if e.file_type().is_dir() {
                return !skip_dir(e.path(), config);
            }
            true
        });
        for entry in walker {
            let entry = entry?;
            if !entry.file_type().is_file() || !is_scannable(entry.path(), config) {
                continue;
            }
            scan_file(entry.path(), config, &mut findings)?;
        }
    }
    Ok(findings)
}

fn check(root: &Path, findings: &Findings, config: &Config) -> Report {
    let mut report = Report::default();
    check_agents_block_version(root, &mut report);

    for (id, decls) in &findings.declarations {
        let duplicate_homes: Vec<&Declaration> = decls
            .iter()
            .filter(|decl| !is_stub_for_inline_decl(root, decl, decls))
            .collect();
        if duplicate_homes.len() > 1 {
            let mut locs: Vec<String> = duplicate_homes
                .iter()
                .map(|d| format!("{}:{}", d.file.display(), d.line))
                .collect();
            locs.sort();
            for loc in locs {
                report
                    .errors
                    .push(format!("{}: duplicate declaration of {}", loc, id));
            }
        }
    }

    for cite in &findings.citations {
        let Some(decls) = findings.declarations.get(&cite.id) else {
            report.errors.push(format!(
                "{}:{}: unknown reference {}",
                cite.file.display(),
                cite.line,
                cite.id
            ));
            continue;
        };
        if let Some(sec) = &cite.section {
            let any_match = decls.iter().any(|d| d.sections.contains(sec));
            if !any_match {
                report.errors.push(format!(
                    "{}:{}: missing section {}.{}",
                    cite.file.display(),
                    cite.line,
                    cite.id,
                    sec
                ));
            }
        }
    }

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
                root.join(target)
            };
            if !resolved.exists() {
                report.errors.push(format!(
                    "{}:{}: stub link target missing: {}",
                    decl.file.display(),
                    decl.line,
                    target.display()
                ));
                continue;
            }
            let inline_ok = if resolved.is_file() && is_scannable(&resolved, config) {
                file_declares(&resolved, id, &config.grammar).unwrap_or(false)
            } else {
                false
            };
            if !inline_ok {
                report.errors.push(format!(
                    "{}:{}: stub link target lacks {}: {}",
                    decl.file.display(),
                    decl.line,
                    id,
                    target.display()
                ));
            }
        }
    }

    let cited: BTreeSet<&Id> = findings.citations.iter().map(|c| &c.id).collect();
    for id in findings.declarations.keys() {
        if !cited.contains(id) {
            report
                .warnings
                .push(format!("declared but never cited: {}", id));
        }
    }

    report.errors.sort();
    report.warnings.sort();
    report
}

fn check_agents_block_version(root: &Path, report: &mut Report) {
    let path = root.join("agents.md");
    if !path.exists() {
        return;
    }
    let Ok(text) = fs::read_to_string(&path) else {
        report
            .errors
            .push(format!("{}:1: cannot read agents.md", path.display()));
        return;
    };
    if let Some(block) = find_agents_block(&text) {
        let line = line_for_byte_index(&text, block.start);
        if block.version < AGENTS_BLOCK_VERSION {
            report.errors.push(format!(
                "{}:{}: outdated gnd init block v{} (run `gnd init` to update to v{})",
                path.display(),
                line,
                block.version,
                AGENTS_BLOCK_VERSION
            ));
        } else if block.version > AGENTS_BLOCK_VERSION {
            report.errors.push(format!(
                "{}:{}: unsupported gnd init block v{} (this gnd supports v{})",
                path.display(),
                line,
                block.version,
                AGENTS_BLOCK_VERSION
            ));
        }
        return;
    }
    if AGENTS_BLOCK_BEGIN.is_match(&text) {
        let line = AGENTS_BLOCK_BEGIN
            .find(&text)
            .map(|m| line_for_byte_index(&text, m.start()))
            .unwrap_or(1);
        report.errors.push(format!(
            "{}:{}: malformed gnd init block",
            path.display(),
            line
        ));
    } else {
        report.errors.push(format!(
            "{}:1: missing gnd init block v{}",
            path.display(),
            AGENTS_BLOCK_VERSION
        ));
    }
}

fn line_for_byte_index(text: &str, byte_index: usize) -> usize {
    text[..byte_index]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

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

fn file_declares(path: &Path, id: &Id, grammar: &Grammar) -> Result<bool> {
    let text = fs::read_to_string(path)?;
    for line in text.lines() {
        if let Some(caps) = grammar.decl_re.captures(line) {
            if let Some(found) = parse_id(&caps) {
                if &found == id {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn print_report(report: &Report) {
    if report.errors.is_empty() && report.warnings.is_empty() {
        return;
    }
    for w in &report.warnings {
        eprintln!("warning: {}", w);
    }
    for e in &report.errors {
        eprintln!("{}", e);
    }
}

fn print_json_report(report: &Report) {
    let errors = report
        .errors
        .iter()
        .map(|message| format!("{{\"message\":\"{}\"}}", json_escape(message)))
        .collect::<Vec<_>>()
        .join(",");
    let warnings = report
        .warnings
        .iter()
        .map(|message| format!("{{\"message\":\"{}\"}}", json_escape(message)))
        .collect::<Vec<_>>()
        .join(",");
    println!("{{\"errors\":[{}],\"warnings\":[{}]}}", errors, warnings);
}

fn json_escape(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('"', "\\\"")
}

fn command_check(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut format_override = None;
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
            other => path = PathBuf::from(other),
        }
        idx += 1;
    }
    let config = match load_config(&path) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err:#}");
            return ExitCode::from(2);
        }
    };
    let findings = match scan_tree(&path, &config) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("scan failed: {:#}", e);
            return ExitCode::from(2);
        }
    };
    let report = check(&path, &findings, &config);
    let format = format_override.unwrap_or_else(|| config.output_format.clone());
    if format == "json" {
        print_json_report(&report);
    } else {
        print_report(&report);
    }
    if report.errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn command_show(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("error: show requires an ID");
        return ExitCode::from(2);
    }
    let mut id_arg = None;
    let mut path = PathBuf::from(".");
    let mut head = false;
    let mut section_override = None;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--head" => head = true,
            "--full" => head = false,
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
            }
            "--format" => idx += 1,
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other if id_arg.is_none() => id_arg = Some(other.to_string()),
            other => path = PathBuf::from(other),
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
            eprintln!("{err:#}");
            return ExitCode::from(2);
        }
    };
    let (id, inline_section) = match parse_id_arg(&id_arg, &config.grammar) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("{err:#}");
            return ExitCode::FAILURE;
        }
    };
    let section = section_override.or(inline_section);
    let findings = match scan_tree(&path, &config) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("scan failed: {:#}", e);
            return ExitCode::from(2);
        }
    };
    match show_declaration(&path, &findings, &id, section.as_deref(), head, &config.grammar) {
        Ok(body) => {
            print!("{body}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err:#}");
            ExitCode::FAILURE
        }
    }
}

fn show_declaration(
    root: &Path,
    findings: &Findings,
    id: &Id,
    section: Option<&str>,
    head: bool,
    grammar: &Grammar,
) -> Result<String> {
    let decls = findings
        .declarations
        .get(id)
        .ok_or_else(|| anyhow!("ID not found: {id}"))?;
    let homes: Vec<&Declaration> = decls
        .iter()
        .filter(|decl| !is_stub_for_inline_decl(root, decl, decls))
        .collect();
    if homes.len() > 1 {
        let mut sites: Vec<String> = homes
            .iter()
            .map(|d| format!("{}:{}", d.file.display(), d.line))
            .collect();
        sites.sort();
        return Err(anyhow!(
            "ambiguous ID: {} (declared at {})",
            id,
            sites.join(", ")
        ));
    }
    let decl = decls.iter().find(|decl| decl.is_stub).unwrap_or(&decls[0]);
    let file = if let Some(target) = &decl.defined_in {
        if target.is_absolute() {
            target.clone()
        } else {
            root.join(target)
        }
    } else {
        decl.file.clone()
    };
    extract_declaration_body(&file, id, section, head, grammar)
}

fn extract_declaration_body(
    path: &Path,
    id: &Id,
    section: Option<&str>,
    head: bool,
    grammar: &Grammar,
) -> Result<String> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let is_md = path.extension().and_then(|e| e.to_str()) == Some("md");
    let mut in_decl = false;
    let mut found_section = section.is_none();
    let mut target_depth = usize::MAX;
    let mut lines = Vec::new();

    for line in text.lines() {
        if let Some(caps) = grammar.decl_re.captures(line) {
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
        if !is_md && !is_comment_body_line(line) && !line.trim().is_empty() {
            break;
        }
        if let Some(caps) = SECTION_HEADING.captures(line) {
            let sec = caps.name("sec").map(|m| m.as_str()).unwrap_or("");
            if head {
                break;
            }
            if let Some(target) = section {
                let depth = sec.split('.').count();
                if sec == target {
                    found_section = true;
                    target_depth = depth;
                    continue;
                }
                if found_section && depth <= target_depth {
                    break;
                }
            }
        }
        if found_section {
            lines.push(clean_body_line(line, is_md));
        }
    }

    if !in_decl {
        return Err(anyhow!("ID not found: {id}"));
    }
    if !found_section {
        return Err(anyhow!(
            "section not found: {}.{}",
            id,
            section.unwrap_or("")
        ));
    }
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!("{}\n", lines.join("\n")))
    }
}

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

fn is_comment_body_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    ["///", "//!", "//", "#", "*", "/*", "*/"]
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
}

fn command_fmt(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut write = false;
    let mut marker = false;
    for arg in args {
        match arg.as_str() {
            "--check" => {}
            "--write" => write = true,
            "--marker" => marker = true,
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other => path = PathBuf::from(other),
        }
    }
    let config = match load_config(&path) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err:#}");
            return ExitCode::from(2);
        }
    };
    let changes = match fmt_tree(&path, &config, marker, write) {
        Ok(changes) => changes,
        Err(err) => {
            eprintln!("{err:#}");
            return ExitCode::from(2);
        }
    };
    if !write {
        for (path, line) in &changes {
            eprintln!("{}:{}: would rewrite reference", path.display(), line);
        }
    }
    if changes.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn fmt_tree(
    root: &Path,
    config: &Config,
    add_marker: bool,
    write: bool,
) -> Result<Vec<(PathBuf, usize)>> {
    let mut changes = Vec::new();
    let walker = WalkDir::new(root).into_iter().filter_entry(|e| {
        if e.depth() == 0 {
            return true;
        }
        if e.file_type().is_dir() {
            return !skip_dir(e.path(), config);
        }
        true
    });
    for entry in walker {
        let entry = entry?;
        if !entry.file_type().is_file() || !is_scannable(entry.path(), config) {
            continue;
        }
        let path = entry.path();
        let original =
            fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
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
            let new_line = fmt_line(line, config, add_marker);
            if new_line != line {
                changes.push((path.to_path_buf(), idx + 1));
                changed = true;
            }
            changed_lines.push(new_line);
        }
        if write && changed {
            let mut output = changed_lines.join("\n");
            if original.ends_with('\n') {
                output.push('\n');
            }
            fs::write(path, output).with_context(|| format!("write {}", path.display()))?;
        }
    }
    Ok(changes)
}

fn fmt_line(line: &str, config: &Config, add_marker: bool) -> String {
    let triggered = replace_trigger(line, config);
    if add_marker {
        add_markers(&triggered, config)
    } else {
        triggered
    }
}

fn replace_trigger(line: &str, config: &Config) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    while let Some(relative) = line[cursor..].find(&config.trigger) {
        let start = cursor + relative;
        let after = start + config.trigger.len();
        if let Some(found) = config.grammar.citation_re.find_at(line, after) {
            if found.start() == after {
                output.push_str(&line[cursor..start]);
                output.push_str(&config.marker);
                cursor = after;
                continue;
            }
        }
        output.push_str(&line[cursor..after]);
        cursor = after;
    }
    output.push_str(&line[cursor..]);
    output
}

fn add_markers(line: &str, config: &Config) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    for found in config.grammar.citation_re.find_iter(line) {
        if line[..found.start()].ends_with(&config.marker) {
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

fn command_config(args: &[String]) -> ExitCode {
    match args.first().map(|arg| arg.as_str()) {
        Some("validate") => {
            let path = args.get(1).map(PathBuf::from).unwrap_or_else(|| ".".into());
            match load_config(&path) {
                Ok(_) => ExitCode::SUCCESS,
                Err(err) => {
                    eprintln!("{err:#}");
                    ExitCode::from(2)
                }
            }
        }
        Some("show") => {
            let path = args.get(1).map(PathBuf::from).unwrap_or_else(|| ".".into());
            match load_config(&path) {
                Ok(config) => {
                    println!("[reference]");
                    println!("marker = \"{}\"", config.marker);
                    println!("trigger = \"{}\"", config.trigger);
                    println!("strict = {}", config.strict);
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("{err:#}");
                    ExitCode::from(2)
                }
            }
        }
        _ => {
            eprintln!("error: expected `config validate` or `config show`");
            ExitCode::from(2)
        }
    }
}

const AGENTS_TEMPLATE: &str = include_str!("../templates/agents.md");
const GND_TOML_TEMPLATE: &str = include_str!("../templates/gnd.toml");
const RAISON_DETRE_TEMPLATE: &str = include_str!("../templates/raison-detre.md");
const GOALS_TEMPLATE: &str = include_str!("../templates/goals.md");
const E2E_README_TEMPLATE: &str = include_str!("../templates/e2e-README.md");
const FS_README_TEMPLATE: &str = include_str!("../templates/functional-spec-README.md");
const AS_README_TEMPLATE: &str = include_str!("../templates/architectural-spec-README.md");
const GITKEEP_TEMPLATE: &str = include_str!("../templates/gitkeep.md");
const AGENTS_BLOCK_VERSION: u32 = 1;
const AGENTS_APPEND_BEGIN: &str = "<!-- gnd:init:agents:v1 begin -->";
const AGENTS_APPEND_END: &str = "<!-- gnd:init:agents:v1 end -->";

fn render_agents_md(name: &str) -> String {
    AGENTS_TEMPLATE.replace("{NAME}", name)
}

fn render_agents_append_block(name: &str) -> String {
    let rendered = render_agents_md(name);
    let start = rendered
        .find(AGENTS_APPEND_BEGIN)
        .expect("agents template must contain append block start marker");
    let end = rendered
        .find(AGENTS_APPEND_END)
        .map(|index| index + AGENTS_APPEND_END.len())
        .expect("agents template must contain append block end marker");
    format!("{}\n", rendered[start..end].trim_end())
}

fn render_gnd_toml(name: &str) -> String {
    GND_TOML_TEMPLATE.replace("{NAME}", &escape_toml_basic(name))
}

fn escape_toml_basic(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('"', "\\\"")
}

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

    let mut files: Vec<(&'static str, String)> = vec![
        ("agents.md", render_agents_md(&resolved_name)),
        (".agents/gnd.toml", render_gnd_toml(&resolved_name)),
    ];
    if docs {
        files.extend(docs_scaffold());
    }

    for (rel, contents) in &files {
        let dest = target.join(rel);
        if !force && dest.exists() {
            if *rel == "agents.md" {
                match update_agents_block(&dest, &render_agents_append_block(&resolved_name)) {
                    Ok(AgentsUpdateResult::Appended) => eprintln!("appended {rel}"),
                    Ok(AgentsUpdateResult::Updated) => eprintln!("updated {rel}"),
                    Ok(AgentsUpdateResult::AlreadyCurrent) => eprintln!("exists {rel}"),
                    Err(err) => {
                        eprintln!("error: update {}: {err}", dest.display());
                        return ExitCode::from(2);
                    }
                }
            } else {
                eprintln!("exists {rel}");
            }
            continue;
        }
        if let Some(parent) = dest.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                eprintln!("error: create {}: {err}", parent.display());
                return ExitCode::from(2);
            }
        }
        if let Err(err) = fs::write(&dest, contents) {
            eprintln!("error: write {}: {err}", dest.display());
            return ExitCode::from(2);
        }
        eprintln!("wrote {rel}");
    }

    ExitCode::SUCCESS
}

#[derive(Debug, Eq, PartialEq)]
enum AgentsUpdateResult {
    Appended,
    Updated,
    AlreadyCurrent,
}

fn update_agents_block(dest: &Path, block: &str) -> Result<AgentsUpdateResult> {
    let existing = fs::read_to_string(dest)?;
    let (updated, result) = update_agents_text(&existing, block)?;
    if result == AgentsUpdateResult::AlreadyCurrent {
        return Ok(result);
    }
    fs::write(dest, updated)?;
    Ok(result)
}

fn update_agents_text(existing: &str, block: &str) -> Result<(String, AgentsUpdateResult)> {
    if let Some(existing_block) = find_agents_block(&existing) {
        if existing_block.version == AGENTS_BLOCK_VERSION {
            return Ok((existing.to_string(), AgentsUpdateResult::AlreadyCurrent));
        }
        if existing_block.version > AGENTS_BLOCK_VERSION {
            return Err(anyhow!(
                "agents.md contains newer gnd init block v{}; this binary supports v{}",
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

    if AGENTS_BLOCK_BEGIN.is_match(&existing) {
        return Err(anyhow!(
            "agents.md contains a gnd init block start without a matching end"
        ));
    }

    let separator = if existing.is_empty() {
        ""
    } else if existing.ends_with("\n\n") {
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

struct AgentsBlock {
    start: usize,
    end: usize,
    version: u32,
}

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

fn derive_default_name(target: &Path) -> Result<String> {
    let absolute =
        fs::canonicalize(target).with_context(|| format!("resolve {}", target.display()))?;
    absolute
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_string())
        .ok_or_else(|| anyhow!("cannot derive project name from {}", absolute.display()))
}

fn docs_scaffold() -> Vec<(&'static str, String)> {
    vec![
        ("docs/raison-detre.md", RAISON_DETRE_TEMPLATE.to_string()),
        ("docs/goals/goals.md", GOALS_TEMPLATE.to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn current_block() -> String {
        render_agents_append_block("demo")
    }

    #[test]
    fn agents_update_appends_managed_block_when_missing() {
        let (updated, result) =
            update_agents_text("# Existing agents\n", &current_block()).expect("append block");

        assert_eq!(result, AgentsUpdateResult::Appended);
        assert!(updated.starts_with("# Existing agents\n\n"));
        assert_eq!(updated.matches(AGENTS_APPEND_BEGIN).count(), 1);
    }

    #[test]
    fn agents_update_does_not_append_current_block_twice() {
        let existing = current_block();
        let (updated, result) =
            update_agents_text(&existing, &current_block()).expect("current block");

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
            update_agents_text(&existing, &current_block()).expect("update old block");

        assert_eq!(result, AgentsUpdateResult::Updated);
        assert!(updated.starts_with("# Existing agents\n\n"));
        assert!(updated.ends_with("\n\n# Local notes\n"));
        assert_eq!(updated.matches(AGENTS_APPEND_BEGIN).count(), 1);
        assert!(!updated.contains("gnd:init:agents:v0"));
    }

    #[test]
    fn agents_update_keeps_current_block_in_middle_position() {
        // FS-init.2.3.1: a v1 block that already sits between user-authored
        // sections must be recognized as `AlreadyCurrent` and the file must not be
        // rewritten — the position of the block within the file is preserved.
        let existing = format!(
            "# Existing agents\n\n{}\n\n# Local notes\n",
            current_block()
        );
        let (updated, result) =
            update_agents_text(&existing, &current_block()).expect("non-EOF current block");

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
        // FS-init.2.3.2: a CRLF-encoded agents.md with a v0 block sandwiched
        // between user-authored sections must still be detected and updated, with
        // CRLF preserved outside the managed block.
        let v0_lf = current_block()
            .replace("gnd:init:agents:v1 begin", "gnd:init:agents:v0 begin")
            .replace("gnd:init:agents:v1 end", "gnd:init:agents:v0 end");
        let v0_crlf = v0_lf.replace('\n', "\r\n");
        let existing = format!("# Existing agents\r\n\r\n{v0_crlf}\r\n\r\n# Local notes\r\n");
        let (updated, result) =
            update_agents_text(&existing, &current_block()).expect("update CRLF v0 block");

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
}

fn print_help() {
    println!("Usage:");
    println!("  gnd [check] [path] [--format text|json]");
    println!("  gnd show <ID> [path] [--section <section>] [--head]");
    println!("  gnd fmt [path] [--check] [--marker] [--write]");
    println!("  gnd init [path] [--name <name>] [--docs] [--force] [--append]");
    println!("  gnd config validate [path]");
    println!("  gnd config show [path]");
}

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(|arg| arg.as_str()) {
        None => command_check(&[]),
        Some("--help") | Some("-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("check") => command_check(&args[1..]),
        Some("show") => command_show(&args[1..]),
        Some("fmt") => command_fmt(&args[1..]),
        Some("init") => command_init(&args[1..]),
        Some("config") => command_config(&args[1..]),
        Some(other) if other.starts_with('-') => command_check(&args),
        Some(other) if Path::new(other).exists() => command_check(&args),
        Some(other) => {
            eprintln!("error: unknown subcommand `{other}`");
            ExitCode::from(2)
        }
    }
}

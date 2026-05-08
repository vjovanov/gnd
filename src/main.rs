use anyhow::{Context, Result, anyhow};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use walkdir::WalkDir;

const KIND_GROUP: &str = r"(?P<kind>FS|AS|DA|DF|G|E2E)";
const NUM_GROUP: &str = r"(?P<num>\d+)";
const SLUG_GROUP: &str = r"(?P<slug>[a-z0-9][a-z0-9-]*)";
const SEC_GROUP: &str = r"(?P<sec>\d+(?:\.\d+)*)";
const COMMENT_PREFIX: &str = r"(?://[/!]?|#|;|--|\*|/\*)";

static DECL_HEADING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"^\s*{}?\s*#+\s+{}-{}-{}\b",
        COMMENT_PREFIX, KIND_GROUP, NUM_GROUP, SLUG_GROUP
    ))
    .unwrap()
});

static SECTION_HEADING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"^\s*{}?\s*#+\s+{}\.?\s+\S",
        COMMENT_PREFIX, SEC_GROUP
    ))
    .unwrap()
});

static CITATION_CORE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"\b{}-{}-{}(?:\.{})?",
        KIND_GROUP, NUM_GROUP, SLUG_GROUP, SEC_GROUP
    ))
    .unwrap()
});

static ID_INPUT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"^{}-{}-{}(?:\.{})?$",
        KIND_GROUP, NUM_GROUP, SLUG_GROUP, SEC_GROUP
    ))
    .unwrap()
});

static DEFINED_IN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*Defined-in:\s*(?P<path>\S.*?)\s*$").unwrap());

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct Id {
    kind: String,
    num: u32,
    slug: String,
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{:03}-{}", self.kind, self.num, self.slug)
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
}

impl Config {
    fn default_for(_root: PathBuf) -> Self {
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
        }
    }
}

#[derive(Default)]
struct Report {
    errors: Vec<String>,
    warnings: Vec<String>,
}

fn parse_id(caps: &regex::Captures) -> Option<Id> {
    Some(Id {
        kind: caps.name("kind")?.as_str().to_string(),
        num: caps.name("num")?.as_str().parse().ok()?,
        slug: caps.name("slug")?.as_str().to_string(),
    })
}

fn parse_id_arg(raw: &str) -> Result<(Id, Option<String>)> {
    let caps = ID_INPUT
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
        let candidate = dir.join("gnd.toml");
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
    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line.trim_matches(['[', ']']).to_string();
            match section.as_str() {
                "reference" | "scan" | "output" => {}
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
            ("reference", "marker") => config.marker = parse_string(path, line_no, value)?,
            ("reference", "trigger") => config.trigger = parse_string(path, line_no, value)?,
            ("reference", "strict") => config.strict = parse_bool(path, line_no, value)?,
            ("scan", "include") => config.include = Some(parse_string_list(path, line_no, value)?),
            ("scan", "exclude") => config.exclude = parse_string_list(path, line_no, value)?,
            ("scan", "extensions") => config.extensions = parse_string_list(path, line_no, value)?,
            ("output", "format") => config.output_format = parse_string(path, line_no, value)?,
            _ => bail_config(path, line_no, format!("unknown config key `{key}`"))?,
        }
    }
    if config.strict && config.marker.is_empty() {
        return Err(anyhow!(
            "{}: reference.strict requires a non-empty marker",
            path.display()
        ));
    }
    Ok(())
}

fn bail_config<T>(path: &Path, line: usize, message: String) -> Result<T> {
    Err(anyhow!("{}:{}: {}", path.display(), line, message))
}

fn parse_string(path: &Path, line: usize, value: &str) -> Result<String> {
    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        Ok(value[1..value.len() - 1].to_string())
    } else {
        bail_config(path, line, "expected string".to_string())
    }
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

        if let Some(caps) = DECL_HEADING.captures(scan_line) {
            if let Some(id) = parse_id(&caps) {
                if let Some(prev) = current.take() {
                    findings
                        .declarations
                        .entry(prev.id.clone())
                        .or_default()
                        .push(prev);
                }
                current = Some(Declaration {
                    id,
                    file: path.to_path_buf(),
                    line: lineno,
                    sections: BTreeSet::new(),
                    is_stub: false,
                    defined_in: None,
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

        if is_md && in_docs {
            if let Some(caps) = DEFINED_IN.captures(scan_line) {
                if let Some(decl) = current.as_mut() {
                    let raw = caps.name("path").unwrap().as_str();
                    let target = raw
                        .split_whitespace()
                        .next()
                        .unwrap_or(raw)
                        .trim_end_matches(|c: char| matches!(c, ',' | ';' | '.'));
                    decl.is_stub = true;
                    decl.defined_in = Some(PathBuf::from(target));
                }
            }
        }

        for caps in CITATION_CORE.captures_iter(scan_line) {
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

fn check(root: &Path, findings: &Findings) -> Report {
    let mut report = Report::default();

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
                    "{}:{}: Defined-in path missing: {}",
                    decl.file.display(),
                    decl.line,
                    target.display()
                ));
                continue;
            }
            let inline_ok = if resolved.is_file()
                && is_scannable(&resolved, &Config::default_for(root.to_path_buf()))
            {
                file_declares(&resolved, id).unwrap_or(false)
            } else {
                false
            };
            if !inline_ok {
                report.errors.push(format!(
                    "{}:{}: Defined-in target lacks {}: {}",
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

fn file_declares(path: &Path, id: &Id) -> Result<bool> {
    let text = fs::read_to_string(path)?;
    let target = id.to_string();
    for line in text.lines() {
        if let Some(caps) = DECL_HEADING.captures(line) {
            if let Some(found) = parse_id(&caps) {
                if found.to_string() == target {
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
    let report = check(&path, &findings);
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
    let (id, inline_section) = match parse_id_arg(&id_arg) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("{err:#}");
            return ExitCode::FAILURE;
        }
    };
    let section = section_override.or(inline_section);
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
    match show_declaration(&path, &findings, &id, section.as_deref(), head) {
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
) -> Result<String> {
    let decls = findings
        .declarations
        .get(id)
        .ok_or_else(|| anyhow!("ID not found: {id}"))?;
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
    extract_declaration_body(&file, id, section, head)
}

fn extract_declaration_body(
    path: &Path,
    id: &Id,
    section: Option<&str>,
    head: bool,
) -> Result<String> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let is_md = path.extension().and_then(|e| e.to_str()) == Some("md");
    let mut in_decl = false;
    let mut found_section = section.is_none();
    let mut target_depth = usize::MAX;
    let mut lines = Vec::new();

    for line in text.lines() {
        if let Some(caps) = DECL_HEADING.captures(line) {
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
            if in_fence || DECL_HEADING.is_match(line) {
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
        if let Some(found) = CITATION_CORE.find_at(line, after) {
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
    for found in CITATION_CORE.find_iter(line) {
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

fn print_help() {
    println!("Usage:");
    println!("  gnd [check] [path] [--format text|json]");
    println!("  gnd show <ID> [path] [--section <section>] [--head]");
    println!("  gnd fmt [path] [--check] [--marker] [--write]");
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
        Some("config") => command_config(&args[1..]),
        Some(other) if other.starts_with('-') => command_check(&args),
        Some(other) if Path::new(other).exists() => command_check(&args),
        Some(other) => {
            eprintln!("error: unknown subcommand `{other}`");
            ExitCode::from(2)
        }
    }
}

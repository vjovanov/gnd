/// A dotfile or dot-directory — same convention used by the scanner walker
/// and by `expand_workspace_members` to skip `.git`, `.agents`, `.cache`, etc.
fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.'))
}

/// Whether a file is one the scanner reads: a non-hidden name with an extension in
/// `[scan] extensions` (§FS-config.3.5, §AR-scanner.1).
fn is_scannable(path: &Path, config: &Config) -> bool {
    if is_hidden(path) {
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
///
/// In workspace mode the caller passes a non-empty `workspace_targets` so a
/// `§<alias>/<ID>` token parses with the target project's grammar inline —
/// one disk read for both unqualified and qualified citations
/// (§AR-workspace.5.1). An empty slice falls back to the loose qualified
/// parser used by member-local scans (§FS-workspace.5).
fn scan_file(
    path: &Path,
    config: &Config,
    findings: &mut Findings,
    workspace_targets: &[WorkspaceCitationTarget],
) -> Result<()> {
    let text = fs::read_to_string(path)?;
    let is_md = path.extension().and_then(|e| e.to_str()) == Some("md");
    let is_py = path.extension().and_then(|e| e.to_str()) == Some("py");
    let inline_sites = inline_citation_sites(&text, is_md, is_py, config, workspace_targets);
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

        if let Some(caps) = declaration_captures(&config.grammar, scan_line, in_py_docstring, is_md)
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
        {
            let heading_level = heading_level_for_line(scan_line, is_md || in_py_docstring, &caps);
            if heading_level > decl.heading_level {
                decl.sections.insert(
                    sec.as_str().to_string(),
                    SectionInfo {
                        title: section_anchor_text(scan_line, sec.as_str()),
                        line: lineno,
                        heading_level,
                    },
                );
            }
        }

        let workspace_mode = !workspace_targets.is_empty();
        let mut qualified_marker_starts = BTreeSet::new();
        for caps in config.grammar.citation_re.captures_iter(scan_line) {
            let Some(full) = caps.get(0) else { continue };
            let namespace = caps.name("namespace").map(|m| m.as_str().to_string());
            // In workspace mode, the qualified branch is parsed below with the
            // target's grammar — let that pass own every `§<alias>/...` hit so
            // we never emit one with the citing project's grammar.
            if workspace_mode && namespace.is_some() {
                continue;
            }
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
            let start = if has_marker {
                full.start().saturating_sub(config.marker.len())
            } else {
                full.start()
            };
            if namespace.is_some()
                && has_marker
                && (is_inside_inline_code(scan_line, start)
                    || (!is_md && is_inside_string_literal(scan_line, start)))
            {
                continue;
            }
            if let Some(decl) = current.as_ref()
                && decl.line == lineno
                && decl.id == id
            {
                continue;
            }
            let text = scan_line[start..full.end()].to_string();
            if namespace.is_some() && has_marker {
                qualified_marker_starts.insert(start);
            }
            findings.citations.push(Citation {
                namespace,
                id,
                section: caps.name("sec").map(|m| m.as_str().to_string()),
                file: path.to_path_buf(),
                line: lineno,
                column: start + 1,
                has_marker,
                text,
                inline_site: inline_sites.get(&lineno).cloned(),
            });
        }
        if workspace_mode {
            scan_workspace_qualified_pass(
                scan_line,
                lineno,
                path,
                config,
                is_md,
                workspace_targets,
                &inline_sites,
                findings,
            );
        } else {
            scan_fallback_qualified_citations(
                scan_line,
                lineno,
                path,
                config,
                is_md,
                &qualified_marker_starts,
                &inline_sites,
                findings,
            );
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

#[derive(Clone)]
enum CommentBlockKind {
    Line(String),
    Block,
    PythonDocstring,
}

/// Locate the source-comment blocks that can host inline citation sites
/// (§FS-inline-citation-style.1). Markdown prose is deliberately out of scope.
fn inline_citation_sites(
    text: &str,
    is_md: bool,
    is_py: bool,
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
) -> BTreeMap<usize, InlineCitationSite> {
    let mut sites = BTreeMap::new();
    if is_md {
        return sites;
    }
    let lines = text.lines().collect::<Vec<_>>();
    let mut index = 0;
    while index < lines.len() {
        let Some(kind) = comment_block_kind(lines[index], is_py, config) else {
            index += 1;
            continue;
        };
        let start = index;
        let end = match &kind {
            CommentBlockKind::Line(marker) => {
                let mut end = index;
                while end + 1 < lines.len()
                    && matches!(
                        comment_block_kind(lines[end + 1], is_py, config),
                        Some(CommentBlockKind::Line(next)) if next == *marker
                    )
                {
                    end += 1;
                }
                end
            }
            CommentBlockKind::Block => {
                let mut end = index;
                while end + 1 < lines.len() && !lines[end].contains("*/") {
                    end += 1;
                }
                end
            }
            CommentBlockKind::PythonDocstring => {
                let quote = python_docstring_quote(lines[index]).unwrap_or("\"\"\"");
                let mut end = index;
                while end + 1 < lines.len()
                    && !python_docstring_closes(lines[end], quote, end == start)
                {
                    end += 1;
                }
                end
            }
        };
        if !block_declares_id(&lines[start..=end], matches!(kind, CommentBlockKind::PythonDocstring), config) {
            let site = InlineCitationSite {
                first_line: start + 1,
                last_line: end + 1,
                max_columns: lines[start..=end]
                    .iter()
                    .map(|line| line.len())
                    .max()
                    .unwrap_or(0),
                has_note: block_has_inline_note(&lines[start..=end], config, workspace_targets),
            };
            for line in (start + 1)..=(end + 1) {
                sites.insert(line, site.clone());
            }
        }
        index = end + 1;
    }
    sites
}

fn comment_block_kind(line: &str, is_py: bool, config: &Config) -> Option<CommentBlockKind> {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    if is_py && python_docstring_quote(line).is_some() {
        return Some(CommentBlockKind::PythonDocstring);
    }
    if trimmed.starts_with("/*") {
        return Some(CommentBlockKind::Block);
    }
    line_comment_marker(trimmed, config).map(CommentBlockKind::Line)
}

fn line_comment_marker(trimmed: &str, config: &Config) -> Option<String> {
    for marker in ["///", "//!", "//"] {
        if config.comment_prefixes.iter().any(|prefix| prefix == "//")
            && trimmed.starts_with(marker)
        {
            return Some(marker.to_string());
        }
    }
    let mut prefixes = config
        .comment_prefixes
        .iter()
        .filter(|prefix| !matches!(prefix.as_str(), "" | "//" | "*" | "/*"))
        .collect::<Vec<_>>();
    prefixes.sort_by_key(|prefix| std::cmp::Reverse(prefix.len()));
    prefixes
        .into_iter()
        .find(|prefix| trimmed.starts_with(prefix.as_str()))
        .map(|prefix| prefix.to_string())
}

fn python_docstring_quote(line: &str) -> Option<&'static str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("\"\"\"") {
        Some("\"\"\"")
    } else if trimmed.starts_with("'''") {
        Some("'''")
    } else {
        None
    }
}

fn python_docstring_closes(line: &str, quote: &str, is_opening_line: bool) -> bool {
    let trimmed = line.trim_start();
    let search = if is_opening_line {
        trimmed.strip_prefix(quote).unwrap_or(trimmed)
    } else {
        trimmed
    };
    search.contains(quote)
}

fn block_declares_id(lines: &[&str], in_py_docstring: bool, config: &Config) -> bool {
    lines.iter().any(|line| {
        let scan_line = if in_py_docstring {
            line.trim_start()
        } else {
            *line
        };
        declaration_captures(&config.grammar, scan_line, in_py_docstring, false)
            .and_then(|caps| parse_id(&caps))
            .is_some()
    })
}

fn block_has_inline_note(
    lines: &[&str],
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
) -> bool {
    lines.iter().any(|line| {
        let tokenless = remove_inline_citation_tokens(line, config, workspace_targets);
        let clean = strip_comment_tokens(&tokenless);
        !clean.trim().is_empty()
    })
}

fn remove_inline_citation_tokens(
    line: &str,
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
) -> String {
    let mut ranges = citation_token_ranges(line, config, workspace_targets);
    ranges.sort_unstable();
    ranges.dedup();
    let mut out = String::with_capacity(line.len());
    let mut cursor = 0;
    for (start, end) in ranges {
        if start < cursor {
            continue;
        }
        out.push_str(&line[cursor..start]);
        cursor = end;
    }
    out.push_str(&line[cursor..]);
    out
}

fn citation_token_ranges(
    line: &str,
    config: &Config,
    workspace_targets: &[WorkspaceCitationTarget],
) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    for caps in config.grammar.citation_re.captures_iter(line) {
        let Some(full) = caps.get(0) else { continue };
        let namespace = caps.name("namespace");
        let has_marker = line[..full.start()].ends_with(&config.marker);
        if namespace.is_some() && !has_marker {
            continue;
        }
        if config.strict && !has_marker {
            continue;
        }
        if !has_marker && is_inside_string_literal(line, full.start()) {
            continue;
        }
        let start = if has_marker {
            full.start().saturating_sub(config.marker.len())
        } else {
            full.start()
        };
        if namespace.is_some()
            && has_marker
            && (is_inside_inline_code(line, start) || is_inside_string_literal(line, start))
        {
            continue;
        }
        ranges.push((start, full.end()));
    }

    if config.marker.is_empty() {
        return ranges;
    }
    for (marker_start, _) in line.match_indices(&config.marker) {
        if ranges.iter().any(|(start, _)| *start == marker_start)
            || is_inside_string_literal(line, marker_start)
        {
            continue;
        }
        let token_start = marker_start + config.marker.len();
        let Some(rest) = line.get(token_start..) else {
            continue;
        };
        let Some(prefix) = QUALIFIED_CITATION_PREFIX.captures(rest) else {
            continue;
        };
        let Some(alias) = prefix.name("namespace").map(|m| m.as_str()) else {
            continue;
        };
        let id_start = token_start + prefix.get(0).unwrap().end();
        let Some(id_rest) = line.get(id_start..) else {
            continue;
        };
        let parsed = if workspace_targets.is_empty() {
            parse_loose_qualified_id_prefix(id_rest).map(|(_, _, len)| len)
        } else {
            match workspace_targets.iter().find(|target| target.alias == alias) {
                Some(target) => parse_longest_id_prefix(id_rest, &target.config.grammar),
                None => workspace_targets
                    .iter()
                    .find_map(|target| parse_longest_id_prefix(id_rest, &target.config.grammar)),
            }
            .map(|(_, _, len)| len)
        };
        let Some(id_len) = parsed else {
            continue;
        };
        ranges.push((marker_start, id_start + id_len));
    }
    ranges
}

fn strip_comment_tokens(line: &str) -> String {
    let marker_start = line
        .char_indices()
        .find_map(|(idx, ch)| (!ch.is_whitespace()).then_some(idx))
        .unwrap_or(line.len());
    let (_, body) = line.split_at(marker_start);
    let mut rest = body;
    for prefix in ["/**", "/*", "///", "//!", "//", "*/", "#", ";", "--", "*", "\"\"\"", "'''"] {
        if let Some(stripped) = rest.strip_prefix(prefix) {
            rest = stripped.strip_prefix(' ').unwrap_or(stripped);
            break;
        }
    }
    let trimmed_end = rest.trim_end();
    let rest = trimmed_end
        .strip_suffix("*/")
        .or_else(|| trimmed_end.strip_suffix("\"\"\""))
        .or_else(|| trimmed_end.strip_suffix("'''"))
        .unwrap_or(trimmed_end);
    rest.to_string()
}

/// §FS-workspace.5: a member-local scan must still recognize marker-qualified
/// citations before the member's own ID grammar is applied. Without this
/// fallback, `§root/FS-root` in a default member can disappear just because the
/// root uses `{kind}-{slug}`.
///
/// The fallback parses the ID tail with the conventional `KIND[-NUM]-SLUG`
/// shape (`parse_loose_qualified_id_prefix`), not the citing or any target
/// project's configured `[id] format`. Member-local scans have no workspace
/// catalogue, so the target's grammar is unreachable here. The tradeoff:
/// non-default ID grammars (lowercase kinds, slug-only shapes that don't
/// separate on `-`/`_`, kinds with characters outside `[A-Z0-9]`) won't be
/// recognised as qualified citations at member scope and will fall through
/// to be diagnosed at the workspace-root run instead. Workspace-root and
/// workspace-aware paths use the target's actual grammar via
/// `scan_workspace_qualified_pass`.
fn scan_fallback_qualified_citations(
    scan_line: &str,
    lineno: usize,
    path: &Path,
    config: &Config,
    is_md: bool,
    already_seen: &BTreeSet<usize>,
    inline_sites: &BTreeMap<usize, InlineCitationSite>,
    findings: &mut Findings,
) {
    if config.marker.is_empty() {
        return;
    }
    for (marker_start, _) in scan_line.match_indices(&config.marker) {
        if already_seen.contains(&marker_start) {
            continue;
        }
        if is_inside_inline_code(scan_line, marker_start) {
            continue;
        }
        if !is_md && is_inside_string_literal(scan_line, marker_start) {
            continue;
        }
        let token_start = marker_start + config.marker.len();
        let Some(rest) = scan_line.get(token_start..) else {
            continue;
        };
        let Some(prefix) = QUALIFIED_CITATION_PREFIX.captures(rest) else {
            continue;
        };
        let Some(alias) = prefix.name("namespace").map(|m| m.as_str()) else {
            continue;
        };
        let id_start = token_start + prefix.get(0).unwrap().end();
        let Some(id_rest) = scan_line.get(id_start..) else {
            continue;
        };
        let Some((id, section, id_len)) = parse_loose_qualified_id_prefix(id_rest) else {
            continue;
        };
        let token_end = id_start + id_len;
        findings.citations.push(Citation {
            namespace: Some(alias.to_string()),
            id,
            section,
            file: path.to_path_buf(),
            line: lineno,
            column: marker_start + 1,
            has_marker: true,
            text: scan_line[marker_start..token_end].to_string(),
            inline_site: inline_sites.get(&lineno).cloned(),
        });
    }
}

/// The member-local fallback ID parser (§FS-workspace.5). Recognises the
/// conventional `KIND[-NUM]-SLUG` shape — uppercase-or-digit kind, optional
/// numeric middle component, non-empty slug — because the member has no
/// access to the citing or target project's `[id] format` at this point.
/// A workspace-root run uses `parse_longest_id_prefix` with the target's
/// grammar (`scan_workspace_qualified_pass`) and is not affected by this
/// fallback's assumptions.
fn parse_loose_qualified_id_prefix(raw: &str) -> Option<(Id, Option<String>, usize)> {
    let mut end = raw
        .char_indices()
        .find(|(_, ch)| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.')))
        .map(|(idx, _)| idx)
        .unwrap_or(raw.len());
    while end > 0
        && raw[..end]
            .chars()
            .next_back()
            .is_some_and(|ch| matches!(ch, '.' | ',' | ';' | ':' | '!' | '?'))
    {
        end -= raw[..end].chars().next_back().map(char::len_utf8).unwrap_or(1);
    }
    let token = raw.get(..end)?;
    let (id_text, section) = split_loose_section(token);
    let (kind, rest) = id_text
        .split_once(['-', '_'])
        .filter(|(kind, rest)| !kind.is_empty() && !rest.is_empty())?;
    if !kind.chars().all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit()) {
        return None;
    }
    let (num, slug) = match rest.split_once(['-', '_']) {
        Some((maybe_num, slug)) if maybe_num.chars().all(|ch| ch.is_ascii_digit()) => {
            (maybe_num.parse::<u32>().ok(), slug)
        }
        _ => (None, rest),
    };
    if slug.is_empty() {
        return None;
    }
    Some((
        Id {
            kind: kind.to_string(),
            num,
            slug: Some(slug.to_string()),
        },
        section.map(str::to_string),
        end,
    ))
}

fn split_loose_section(token: &str) -> (&str, Option<&str>) {
    let suffix_start = token
        .char_indices()
        .rev()
        .find(|(_, ch)| !(ch.is_ascii_digit() || *ch == '.'))
        .map(|(idx, ch)| idx + ch.len_utf8())
        .unwrap_or(0);
    let suffix = &token[suffix_start..];
    let Some(section) = suffix.strip_prefix('.') else {
        return (token, None);
    };
    if section.is_empty()
        || !section
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
    {
        return (token, None);
    }
    (&token[..suffix_start], Some(section))
}

/// One line's worth of marker-qualified workspace citations: a `§<alias>/<ID>`
/// token whose ID tail parses with the target project's grammar
/// (§FS-workspace.1, §AR-workspace.2). Runs inline during `scan_file` in
/// workspace mode so the file is read once, not twice.
fn scan_workspace_qualified_pass(
    scan_line: &str,
    lineno: usize,
    path: &Path,
    config: &Config,
    is_md: bool,
    targets: &[WorkspaceCitationTarget],
    inline_sites: &BTreeMap<usize, InlineCitationSite>,
    findings: &mut Findings,
) {
    if config.marker.is_empty() || targets.is_empty() {
        return;
    }
    for (marker_start, _) in scan_line.match_indices(&config.marker) {
        if is_inside_inline_code(scan_line, marker_start) {
            continue;
        }
        if !is_md && is_inside_string_literal(scan_line, marker_start) {
            continue;
        }
        let token_start = marker_start + config.marker.len();
        let Some(rest) = scan_line.get(token_start..) else {
            continue;
        };
        let Some(prefix) = QUALIFIED_CITATION_PREFIX.captures(rest) else {
            continue;
        };
        let Some(alias) = prefix.name("namespace").map(|m| m.as_str()) else {
            continue;
        };
        let id_start = token_start + prefix.get(0).unwrap().end();
        let Some(id_rest) = scan_line.get(id_start..) else {
            continue;
        };
        let parsed = match targets.iter().find(|target| target.alias == alias) {
            Some(target) => parse_longest_id_prefix(id_rest, &target.config.grammar),
            None => targets
                .iter()
                .find_map(|target| parse_longest_id_prefix(id_rest, &target.config.grammar)),
        };
        let Some((id, section, id_len)) = parsed else {
            continue;
        };
        let token_end = id_start + id_len;
        findings.citations.push(Citation {
            namespace: Some(alias.to_string()),
            id,
            section,
            file: path.to_path_buf(),
            line: lineno,
            column: marker_start + 1,
            has_marker: true,
            text: scan_line[marker_start..token_end].to_string(),
            inline_site: inline_sites.get(&lineno).cloned(),
        });
    }
}

fn parse_longest_id_prefix(raw: &str, grammar: &Grammar) -> Option<(Id, Option<String>, usize)> {
    let search_end = raw
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace())
        .map(|(idx, _)| idx)
        .unwrap_or(raw.len());
    // `char_indices` already yields strictly increasing, unique byte offsets,
    // and `search_end` is strictly greater than the last char-start it can
    // emit — so the chained list is sorted and unique without further work.
    let ends = raw[..search_end]
        .char_indices()
        .map(|(idx, _)| idx)
        .chain(std::iter::once(search_end))
        .filter(|end| *end > 0)
        .collect::<Vec<_>>();
    for end in ends.into_iter().rev() {
        if let Ok((id, section)) = parse_id_arg(&raw[..end], grammar) {
            return Some((id, section, end));
        }
    }
    None
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
/// `grund E2E-<name>` renders (§FS-show.2.4).
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
    // Code-form declarations (§DF-code-declarations-drop-hash) match the branch
    // that has no `#+`, so no heading group is set; default to depth 1.
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
        // syscall, no allocation in the hot path. `strip_prefix` only removes
        // the root, so the descendant suffix is invariant under symlink
        // resolution — comparing against `scan_root_for_filter` works even if
        // `scan_root` itself is a symlink.
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
                if is_hidden(e.path()) {
                    return false;
                }
                let Some(name) = e.path().file_name().and_then(|name| name.to_str()) else {
                    return true;
                };
                return !excluded.iter().any(|item| item == name);
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
/// so `check` can report them and keep going (§FS-check.2). The wrapper around
/// the workspace-aware variant with no targets — single-project scans and
/// member-local scans share this path.
fn scan_tree(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
) -> Result<(Findings, Vec<ScanError>)> {
    scan_tree_with_workspace(config, scope, explicit_scope, &[])
}

/// Workspace-aware tree walk: `§<alias>/<ID>` citations parse with each
/// target's grammar inline, so the workspace layer (§FS-workspace.1,
/// §AR-workspace.2) never needs to re-read the files the initial scan
/// already read.
fn scan_tree_with_workspace(
    config: &Config,
    scope: Option<&Path>,
    explicit_scope: bool,
    workspace_targets: &[WorkspaceCitationTarget],
) -> Result<(Findings, Vec<ScanError>)> {
    let mut findings = Findings::default();
    let mut errors = Vec::new();
    for file in walk_scannable_files(config, scope, explicit_scope)? {
        match scan_file(&file, config, &mut findings, workspace_targets) {
            Ok(()) => findings.scanned_files.push(file),
            Err(err) => errors.push((file, format!("{err:#}"))),
        }
    }
    if let Err(err) = scan_e2e_cases(config, scope, explicit_scope, &mut findings) {
        errors.push((config.root.join("e2e/cases"), format!("{err:#}")));
    }
    // §FS-workspace.1: when the citing-grammar pass and the target-grammar
    // pass both fire on the same line they emit in source order *per pass*;
    // sort once at the end so a workspace scan's per-line citation order
    // matches the single-project scan's left-to-right invariant.
    if !workspace_targets.is_empty() {
        findings.citations.sort_by(|a, b| {
            (sort_path_key(&a.file), a.line, a.column).cmp(&(
                sort_path_key(&b.file),
                b.line,
                b.column,
            ))
        });
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

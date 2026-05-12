# FS-show: grund reads a single declaration body by ID

The `show` subcommand prints just the body of a declaration, given an ID — or, with `--head` / `--outline` / `--brief`, a cheaper slice of it. It exists so an agent — human or AI — can pull a single grounded fact into context without loading the whole file. Serves [§GOAL-friendliness-first](../goals/goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) and [§GOAL-token-economy](../goals/goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file).

## 1. Inputs

```
grund show <ID> [<path>] [--section <s>] [--head | --outline | --brief | --full] [--format <text|md|json>]
```

- `<ID>` — the full ID without the marker (e.g. `FS-check`). May include an inline section (`FS-check.3.1`). The dotted form uses the configured `[id] section_separator` ([§FS-config.3.2](FS-config.md#32-id--id-grammar)). When the separator is non-default (e.g., `:` or `#`) the inline form may collide with the slug grammar; use `--section` instead.
- `<path>` — directory or file whose tree is scanned to resolve the ID. Defaults to `.`. Discovery is the same as every other subcommand (walk up to `.agents/grund.toml`, else defaults — [§FS-config.1](FS-config.md#1-file-location-and-discovery)). `--path <path>` is an accepted alias for scripts that prefer to pass it as a flag; the two forms are equivalent.
- `--section <s>` — alternative way to specify a section path (`3.1`). Mutually exclusive with the dotted form. Required when `[id] section_separator` makes the dotted form ambiguous. Combined with `--head` it prints the selected section heading plus only the lead prose of that section (§2.1.1); combined with `--outline` it prints only the sub-headings nested under that section, not the selected section heading itself (§2.1.2).
- `--head` — print only the top of the context: the heading line and the prose up to the first numbered subsection. The cheapest "what is this about" view (§2.1.1).
- `--outline` — print only the declaration's numbered section headings, one per line — the *map* an agent reads to choose which `<ID>.<section>` to fetch, without paying for any section body (§2.1.2).
- `--brief` — print `--head` then `--outline`: the lead prose plus the section map. The recommended first read when a citation is a bare `§<ID>` with no section coordinate (§2.1.3).
- `--full` — print the entire body. The default, and the escalation path when `--head`, `--outline`, `--brief`, or a section slice is not enough.
- `--head`, `--outline`, `--brief`, and `--full` are mutually exclusive — each picks one point on the "how much of the body" axis: lead prose only → section map → both → everything.
- `--format` — output shape; defaults to `text` (just the body, no headers).

## 2. Behavior

### 2.1 Whole declaration (default, or `--full`)

`grund show FS-check` prints from the heading of `FS-check` to the start of the next ID heading (or end of file). The opening heading is omitted in `text` format and included in `md`. This is the default and stays the default: the `--head` / `--outline` / `--brief` slices below (§2.1.1–§2.1.3) are strictly additive — they never change what a bare `grund show <ID>` prints — decided in [§DF-show-token-cheap-reads](../decisions/functional/DF-show-token-cheap-reads.md#df-show-token-cheap-reads-grund-show-keeps-the-full-body-default-token-cheap-slices-are-opt-in).

### 2.1.1 Head only (`--head`)

`grund show --head FS-check` prints only the top of the context: the prose between the ID heading line and the first numbered section heading (`## 1. ...`). This is the "what is this about" view — typically a paragraph or two — meant for quick scanning, hover previews, and agent prompts where the section structure isn't needed.

If a declaration has no lead paragraph (its body opens directly with `## 1. ...`), `--head` prints **nothing** and exits `0`. This is not an error: the declaration simply has no head. Callers (IDE hovers, agents) can detect this case by the empty output and decide whether to fall back to the full body. We do not auto-fall-back; the caller knows what it wants.

`grund show --head FS-check.3.1` applies the same rule one level down, but keeps the selected section heading so the slice is self-labeled: it prints section heading `### 3.1 ...` and the prose up to the first numbered heading nested under it (`#### 3.1.1 ...`). If the section opens directly with a sub-subsection, the output is just the selected section heading. A section that does not exist is still a `section not found` error regardless of `--head`.

### 2.1.2 Outline only (`--outline`)

`grund show --outline FS-check` prints only the declaration's numbered section headings — one per line, in document order, each at the depth it was written (`## 1. Inputs`, `### 2.1 Whole declaration`, `### 2.1.1 Head only`, …): no lead prose, no section bodies. This is the *map* an agent reads to decide which `§FS-check.<section>` is worth fetching, at a fraction of the whole-body cost. The heading lines are emitted verbatim — the same bytes `--full` would show for those lines — so the section numbers the reader needs are right there to feed back into `grund show FS-check.<n>`. No generated summary, ever: `--outline` is a structural slice of the heading tree, as deterministic as `--head` ([§FS-errors.4](FS-errors.md#4-determinism)).

A declaration whose body has no numbered `## N.` headings — an E2E manifest (§2.4), or any short declaration that is all lead prose — prints **nothing** and exits `0`. Like `--head`'s empty case (§2.1.1) this is not an error: there is no outline to show, and the caller decides whether to fall back.

`grund show --outline FS-check.3.1` (equivalently `--section 3.1 --outline`) restricts the map to headings **nested under** the selected section: it prints `#### 3.1.1 …`, `#### 3.1.2 …`, and so on, stopping at the next sibling-or-shallower heading — the same slice boundary §2.2 uses, headings only. The selected section heading itself (`### 3.1 …`) is not printed by `--outline`; fetch `grund show --head FS-check.3.1` when the selected section needs to be self-labeled. A selected section with no nested headings prints nothing and exits `0`. A section that does not exist is still a `section not found` error regardless of `--outline`.

### 2.1.3 Brief (`--brief`)

`grund show --brief FS-check` prints the `--head` slice (§2.1.1), then a blank line, then the `--outline` slice (§2.1.2): the "what is this about" paragraph followed by the section map. This is the read an agent should reach for **first** when it lands on a bare `§<ID>` citation with no section coordinate — it answers "is this the right declaration?" and "which section do I actually need?" in one cheap call, after which the targeted move is `grund show <ID>.<section>` and, only if that slice is still not enough, `grund show <ID>` for the full body. (When the citation already names a section — `§<ID>.<s>` — skip `--brief` and fetch `grund show <ID>.<s>` directly; the map is only useful when the section is not yet known.)

The blank line separator is emitted only when both halves are non-empty. If `--head` is empty but `--outline` is not, `--brief` prints the outline with no leading blank line; if `--outline` is empty but `--head` is not, `--brief` prints the head with no trailing blank line. If the declaration has neither a lead paragraph nor numbered sections, `--brief` prints **nothing** and exits `0` — the union of the two empty cases above, and not an error. With `--section` / the dotted form it composes the same way: the section's `--head` output (including the selected section heading), then the separator only if there are nested headings, then the section's `--outline` output (nested headings only, so the selected section heading is not duplicated).

### 2.2 Section

`grund show FS-check.3.1` prints the selected section heading (`### 3.1 ...`) and its contents within the declaration body, stopping at the next sibling-or-shallower heading. Nested deeper headings (e.g., `#### 3.1.2`) are included in the output — they end at the next `### 3.x` (sibling) or `## N.` (shallower) heading. Arbitrary nesting depth is supported per [§FS-config.3.3](FS-config.md#33-section-paths--arbitrary-nesting-depth).

### 2.2.1 Ambiguous ID

If an ID has more than one home — the duplicate-declaration error from [§FS-check.3.3](FS-check.md#33-duplicate-declaration) — `show` does not pick one. A stub paired with the inline declaration it points at is *one* home, not two; ambiguity means two or more independent declarations remain after that pairing collapses. When ambiguous, `show` exits 1 with a single bare stderr line (no `<path>:<line>:` prefix, since there is no single site to point at):

```
ambiguous ID: <ID> (declared at <path>:<line>, <path>:<line>[, ...])
```

Sites are listed in lexicographic `path:line` order so the message is stable across runs. The repo must be fixed (run `grund check` first) before `show` will return a body. This shape matches the bare-message form used for `ID not found` and `section not found` ([§FS-show.3](FS-show.md#3-outputs)): all three are queries that found something other than exactly one body.

### 2.3 Inline declarations in code and doc-comments

When the ID's home is in code (per [§FS-check.3.4](FS-check.md#34-broken-inline-spec-stub) stub semantics), `show` extracts the comment block surrounding the inline declaration, strips comment markers, and prints the resulting prose. The same section logic applies — and so do the `--head` / `--outline` / `--brief` slices, computed over the stripped block exactly as over a `.md` body (the lead prose is what precedes the first `## N.` heading inside the comment; the outline is the numbered headings recorded within it, per §2.3.3).

The scanner recognizes the same doc-comment forms enumerated in [§AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments) — Javadoc, JSDoc/TSDoc, Doxygen, KDoc, Scaladoc, PHPDoc, Rustdoc (`///`, `//!`, `/** … */`), C# XML doc comments, Go's `// …` doc blocks, Ruby `#` comments, and Python `""" … """` docstrings. This means an architectural spec can live directly in the class-level Javadoc, and `grund show AR-<event-bus>` returns the rendered Javadoc body — same content the optional LSP server shows on hover ([§FS-lsp.1.2](FS-lsp.md#12-hover-preview)). The stub at `docs/architecture/AR-<event-bus>.md` is a single-line H1 — `# AR-<event-bus>: [<path>](<path>)` — pointing at the file.

#### 2.3.1 What counts as the "comment block"

Extraction is precisely defined so that the implementation has no freedom and the same input produces the same output across editor, CLI, and binding callers.

A declaration is found on a "declaration line" — a line that matches the declaration regex from [§AR-scanner.2.1](../architecture/AR-scanner.md#21-declaration-detection) *and* sits inside a comment or docstring. The block surrounding it is computed as follows:

1. **Find the open boundary.** Walk **backwards** from the declaration line over consecutive lines that are part of the same comment construct:
   - For line-style comments (`//`, `///`, `//!`, `#`, `;`, `--`): consecutive lines whose first non-whitespace character matches the same comment prefix family. A blank line ends the block. A line whose first non-whitespace character is not a comment marker ends the block.
   - For block-style comments (`/* … */`, `/** … */`): walk backward until the opener is found (`/*` or `/**`). The opener line itself is part of the block.
   - For Python triple-quoted docstrings: walk backward until the opening `"""` (or `'''`). The opener line is part of the block.

2. **Find the close boundary.** Walk **forwards** from the declaration line by the symmetric rules:
   - Line-style: until a blank line or a non-comment line.
   - Block-style: until the closing `*/`. The closer line is part of the block.
   - Python docstring: until the matching `"""` or `'''`. The closer line is part of the block.

3. **Terminate early on another declaration.** Walking in **either direction**, if another declaration line of any ID is encountered, the block ends at the line before it. This is what allows two adjacent inline declarations to live in the same comment without bleeding into each other — backward termination keeps a later declaration's block from absorbing the previous declaration's tail; forward termination keeps the previous declaration's block from absorbing the next declaration's head.

#### 2.3.2 Stripping comment markers

After the block is selected, comment markers are removed line-by-line so the output is plain prose:

- Leading whitespace is preserved up to the comment marker, then the marker is dropped, then a single space following the marker is dropped if present. The remainder of the line is kept verbatim.
- For block-style continuation lines, a leading ` * ` (with surrounding spaces) is removed if present. Lines that do not have it are kept as-is.
- For Python docstrings, no marker is stripped — docstring content is plain text already; only the surrounding `"""` lines are skipped.
- Trailing comment-close markers (`*/`) on their own line are dropped entirely.
- Blank lines inside the block are preserved.

The result is the markdown that the declaration's author wrote, identical to what would have lived in a `.md` file had the spec been doc-resident instead of inline. This is the property that makes [§FS-show.2.3](FS-show.md#23-inline-declarations-in-code-and-doc-comments) round-trip-stable across the in-docs and in-code homes.

#### 2.3.3 Section selection inside a doc-comment

Section selection (`AR-<event-bus>.2`) works the same way inside a doc-comment as inside a markdown file: the scanner records the numbered subsection headings declared within the doc-comment block and `show` slices to the requested section. Section depth is measured relative to the declaration's heading level exactly as in markdown ([§AR-scanner.2.2](../architecture/AR-scanner.md#22-section-detection)) — a `# AR-<event-bus>` heading inside a `///` block is "level 1", so `## 1.` is a depth-1 section. The comment-stripping pass leaves these headings intact.

#### 2.3.4 Broken stub

If the ID's only home is a stub (`# <ID>: [<text>](<path>)`) whose link is broken — the `<path>` does not exist, or the file at `<path>` contains no inline declaration of `<ID>` (the [§FS-check.3.4](FS-check.md#34-broken-inline-spec-stub) error) — `show` has no body to extract. It exits `1` with a bare query-result line ([§FS-errors.2.3](FS-errors.md#23-bare-query-failure)), not a `path:line:` finding:

```
broken stub: <ID> (stub at <path>:<line> points at <target>, which does not exist)
broken stub: <ID> (stub at <path>:<line> points at <target>, which contains no inline declaration of <ID>)
```

This is the same "found something other than exactly one body" family as `ID not found` and `ambiguous ID` (§3). Run `grund check` to see the error in located form; fix the stub or the target before `show` will return a body.

### 2.4 E2E cases

`grund show E2E-<name>` returns the case's manifest ([§AR-scanner.6](../architecture/AR-scanner.md#6-e2e-case-declarations)) in three parts:

```
grund <args…>
expected exit: <code>
fixtures:
- <relative path>
- <relative path>
…
```

The first line is the invocation (`grund check` when the case has no `command.args`); then an `expected exit: <code>` line; then a `fixtures:` line followed by one `- <path>` line per file in the case directory, paths relative to that directory, sorted lexicographically — deterministic for a given tree. `text`, `md`, and `--full` all produce this same output: the manifest has no heading to include or strip. This is the "the test *is* the body" view — enough for an agent to understand what the case proves without opening every fixture. `--head` prints only the first line (the invocation). Section paths are not defined for E2E cases (the manifest is not a numbered-heading tree); `grund show E2E-<name>.1` is a section-not-found error, and `--outline E2E-<name>` prints nothing and exits `0` (no heading tree to map — the §2.1.2 empty case), so `--brief E2E-<name>` is just the `--head` line. `--format=json` emits a single object `{"id":"E2E-<name>","kind":"E2E","path":"e2e/cases/<name>","args":[…],"expected_exit":<code>,"fixtures":[…]}` — `args` is the parsed `command.args` (empty when there is none), `fixtures` the same sorted relative-path list; `--outline` / `--brief` over a case do not change this object (the `sections`/`head` additions of §3.1 have nothing to add — the manifest has no headings or lead prose).

## 3. Outputs

- `0` — printed successfully.
- `1` — ID not found, ambiguous ID (multiple homes — [§FS-show.2.2.1](FS-show.md#221-ambiguous-id)), broken stub ([§FS-show.2.3.4](FS-show.md#234-broken-stub)), or section not found in declaration.
- `2` — I/O error.

Stdout carries the body (or, with `--format=json`, the result object — one JSON object, never NDJSON, per [§FS-errors.5](FS-errors.md#5-json-format)). Stderr carries errors. Stdout is empty on error.

A failed query (`1`) prints the bare result line and, where the next step is obvious, one extra `hint:` line on stderr below it — never on stdout. With `--format=json`, stderr instead carries one diagnostic JSON object per [§FS-errors.5](FS-errors.md#5-json-format), with `path` and `line` set to `null` because the failure has no single source location:

- `ID not found: <ID>` → `hint: run \`grund list\` to see every declared ID, or \`grund id <KIND> "<title>"\` to propose a new one`
- `section not found: <ID>.<s>` → `hint: run \`grund show <ID>\` to print the whole declaration with its section numbers`
- a `<ID>` argument that does not match the configured `[id] format` ([§FS-config.3.2](FS-config.md#32-id--id-grammar)) is rejected before the scan with `invalid ID \`<arg>\``, followed by `hint: this repo's [id] format is \`<format>\` (run \`grund config show\`); \`grund list\` shows the IDs that exist` — this is the common surprise in a repo whose format differs from the `{kind}-{slug}` `grund` itself uses.

`ambiguous ID` and `broken stub` get no hint: the fix (run `grund check`, then edit the duplicate or the stub) is already stated in §2.2.1 / §2.3.4 and the message names the sites.

### 3.1 Format variants

- `text` (default) — the body only: for a whole markdown declaration, the lines after the declaration heading line through the end of the body; for a selected section, the selected section heading and its body (§2.2); for `--head`, the lead prose only (§2.1.1); for `--outline`, the numbered heading lines only (§2.1.2); for `--brief`, the non-empty `--head` and `--outline` slices joined by one blank line, with no leading or trailing separator when either half is empty (§2.1.3); for an inline-source declaration, the comment-stripped prose (§2.3.2); for an E2E case, the manifest (§2.4). The whole-declaration opening heading line is **omitted** — in `--head`, `--outline`, and `--brief` too. A `grund fmt --cross-refs` link wrapper around a citation (`[§FS-<x>.1](FS-<x>.md#1-y)`) is flattened back to the bare `§FS-<x>.1` — §3.2.
- `md` — same as `text` but the opening declaration heading line is **included** verbatim, so the output is a self-contained markdown fragment, and `--cross-refs` link wrappers are kept as written — that is the renderable form (§3.2). This holds for the `--head` / `--outline` / `--brief` slices too: in `md` each is prefixed with the declaration's H1, while selected-section heading behavior otherwise stays the same as `text` (`--head` and `--brief` include it through the head slice; section `--outline` does not, because it intentionally lists only nested headings per §2.1.2). The kind's `[[kinds]] title` ([§FS-config.3.4](FS-config.md#34-kinds--recognized-prefixes)) is *not* injected — it is metadata exposed only in `json`. For an inline-source declaration the included heading is the one written in the doc-comment (`# AR-<event-bus>: In-process event broadcaster`), comment-markers stripped.
- `json` — a single object on stdout: `{"id":<ID>,"section":<section-path or null>,"body":<string>,"path":<declaring file or case dir>,"line":<1-indexed>}`. `body` is the same text `text` prints — `--cross-refs` wrappers flattened (§3.2). `section` is `null` when the whole declaration was requested. With `--outline` the object additionally carries `sections` — one `{"path":<section path>,"title":<heading text>,"depth":<integer>}` per numbered heading in the selected outline slice, in document order; with `--brief` it carries both `sections` and `head` (the `--head` slice as a string). For E2E cases the object is the §2.4 shape instead. The wire form is stable per [§GOAL-no-silent-breakage.1](../goals/goals.md#1-what-counts-as-user-visible).

Verbose `show --format=json` examples, including failed-query stream behavior, live in [§FS-output-shapes](FS-output-shapes.md#fs-output-shapes-machine-readable-output-shapes).

### 3.2 Cross-reference links are flattened in `text` and `json`

A repo that has run `grund fmt --cross-refs` ([§FS-fmt.6](FS-fmt.md#6-cross-reference-emission-with---cross-refs)) carries each citation in its `.md` files as a Markdown link *wrapping* the citation — `[§FS-check.1](FS-check.md#1-inputs)` instead of `§FS-check.1`. That wrapper is a rendered-view convenience ([§DF-md-link-emission](../decisions/functional/DF-md-link-emission.md#df-md-link-emission-grund-fmt-may-emit-clickable-markdown-links-alongside--prefixed-citations)), not the canonical form; for an agent pulling a fact into context it is noise, and the relative path inside it is the wrong pointer — the consumer should resolve the citation with `grund show <ID>`, not open the file.

So when `show` prints a body in `text` or in the `json` `body` field, it **flattens** every such wrapper back to the bare citation: a `[` immediately before a marker-prefixed citation token and `](…)` immediately after it — exactly the wrap shape `grund fmt --cross-refs` emits and re-derives ([§FS-fmt.6.3](FS-fmt.md#63-idempotency-and-re-derive)) — collapses to just the `§<ID>[.<section>]` text. Nothing else changes: an ordinary Markdown link in the prose, a citation that is not wrapped, a body extracted from a source-code doc-comment (cross-references never run on source — [§FS-fmt.6.1](FS-fmt.md#61-scope)), and a `grund show --format md` body (the self-contained markdown fragment, §3.1) are all left exactly as written. The flattening is purely textual — it does not resolve the citation, so a dangling one is flattened just the same and `grund check` still reports it. Decided in [§DF-show-cross-ref-flattening](../decisions/functional/DF-show-cross-ref-flattening.md#df-show-cross-ref-flattening-grund-show-flattens-cross-reference-link-wrappers).

## 4. Why this matters

Without `show`, an agent retrieving a spec section either loads the whole file (token-expensive) or reimplements the parser. With `show`, the canonical way to pull `§FS-check.3.1` into a prompt is exactly:

```
grund show FS-check.3.1
```

And when the citation is a bare `§FS-check` with no section, the cheap first move is `grund show FS-check --brief` — the lead paragraph plus the section numbers — then `grund show FS-check.<n>` for the one section that matters, with `grund show FS-check` (the full body) held in reserve for when the slice is not enough. On this repo that ladder is roughly 1 KB → 1–2 KB → 15 KB: an agent that grounds itself this way pays for the fact it needs, not the file it lives in. `grund show FS-check --outline` alone — just the section map — is the move when the agent already knows what it is reading and only needs to find the right `<section>`.

This is the agent-grounding loop: declarations live in one place, and any agent — at any time — can fetch one, or just its map, with a single command.

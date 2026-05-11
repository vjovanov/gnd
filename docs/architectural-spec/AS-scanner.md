# AS-scanner: how gnd discovers declarations and citations

The scanner is the single tree-walk that produces all of gnd's input data. Every check in [§FS-check](../functional-spec/FS-check.md#fs-check-gnd-validates-every-reference-in-a-repo) and every retrieval in [§FS-show](../functional-spec/FS-show.md#fs-show-gnd-reads-a-single-declaration-body-by-id) derives from what the scanner finds. Speed ([§G-fast-feedback](../goals/goals.md#g-fast-feedback-gnd-must-be-as-fast-as-possible)) is set here.

## 1. Tree walk

Recursive walk from a root path using the `ignore` crate (the same walker that powers `ripgrep`). The walker is chosen specifically because it gives us `.gitignore` support for free — see 1.1 below.

Directory-level skip rules:

- Hidden directories (any name starting with `.`) are skipped — this already covers `.next`, `.venv`, and friends.
- Build/output directories named in the skip list (`target`, `node_modules`, `.git`, `dist`, `build`, `.venv` by default — [§FS-config.3.5](../functional-spec/FS-config.md#35-scan--what-gets-walked)) are skipped at any depth.
- The skip list is configurable per [§G-configurable](../goals/goals.md#g-configurable-every-default-is-overridable) and [§FS-config.3.5](../functional-spec/FS-config.md#35-scan--what-gets-walked).

Files are filtered by extension to those that can plausibly contain specs or inline declarations: `.md` and a curated list of source-file extensions.

### 1.1 Respecting `.gitignore` and friends

By default, the walker honors every form of ignore file the `ignore` crate recognizes:

- `.gitignore` files at any depth (nearest-wins precedence, as `git` itself does).
- `.git/info/exclude`.
- The global `core.excludesFile` configured in `git config`.
- `.ignore` files (the ripgrep convention) for `gnd`-specific exclusions that are not appropriate for `git`.

This means `gnd` will not scan files that `git` would not commit. Generated artefacts, secrets, and vendored dependencies are skipped automatically without any `gnd.toml` configuration. A repo's existing `.gitignore` is the source of truth.

The behavior is overridable via `[scan] respect_gitignore` in `gnd.toml` (default `true`). Set to `false` only when you genuinely need to scan ignored paths — e.g., a repo that commits both `node_modules/` and its own specs in unusual layouts.

The directory-level skip lists in 1 above are applied **in addition** to ignore-file rules, never instead of them.

## 2. Per-file scan

A single linear pass over each file's lines, performing three jobs simultaneously:

### 2.1 Declaration detection

A regex matches heading lines of the form `<comment-prefix>? #{1,N} <ID>[:…]` where `<ID>` is the configured `[id]` grammar ([§FS-config.3.2](../functional-spec/FS-config.md#32-id--id-grammar)) with `{kind}` drawn from a configured `[[kinds]]` prefix. The heading may sit at any markdown level: file-form `FS`/`AS`/`DF`/`DA` declarations are H1 (`# FS-… :`), `G` declarations are H2 inside `docs/goals/goals.md`, and an inline declaration in a doc-comment is whatever level the author wrote (`# AS-… ` is "level 1" *within* the comment block). When the regex matches, the line opens a new "current declaration" context and the **declaration heading level** `L` (the count of `#`) is recorded on it. (`E2E` declarations are the exception — they are directories, not heading lines; see §6.)

### 2.2 Section detection

Within a declaration context whose heading is at level `L`, a numbered subsection heading is a line of the form `#{L+1,} <n₁.n₂.….n_d>[.] <title>` — at least one `#` more than the declaration heading, then a dotted number of one or more components, an **optional** trailing `.`, whitespace, and the heading text. The line is recorded on the current declaration as the section path `n₁.n₂.….n_d` together with its `<title>` text (the heading text is needed by [§FS-fmt.6](../functional-spec/FS-fmt.md#6-markdown-link-emission-with---md-links) / [§DF-md-link-anchor-strategy](../decisions/functional/DF-md-link-anchor-strategy.md#df-md-link-anchor-strategy-heading-text-slugs-re-derived-on-every-fmt-pass) and by `gnd show --format=md`). It is the **dotted number, not the `#` count, that fixes the section's depth and tree position**: `1.1` is a child of `1` whether it is written `## 1.1` or `### 1.1`, and `gnd show`'s slicing ([§FS-show.2.2](../functional-spec/FS-show.md#22-section)) walks that number tree, not the heading levels. The `#` count only has to be deeper than the declaration heading so the line reads as a heading at all. Examples for a declaration whose heading is `# §FS-check:` (`L = 1`): `## 1. Inputs` → section `1`; `## 1.1 Recognized citations` and `### 2.1 Report format` → sections `1.1` and `2.1`; `#### 3.1.2.7.4 …` → section `3.1.2.7.4`. Nesting depth is unbounded ([§FS-config.3.3](../functional-spec/FS-config.md#33-section-paths--arbitrary-nesting-depth)); the recorded set is what [§AS-checker.2.3](AS-checker.md#23-missing-sections-fs-check32) validates citations against.

### 2.3 Citation detection

The citation regex matches the configured marker ([§DF-reference-marker](../decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger); default `§`) immediately followed by an `<ID>` token, with an optional `<sep><section-path>` suffix, anywhere in the file. In default (non-strict) mode it **additionally** matches bare ID tokens — but, in source files (every extension except `md`), a bare token whose start column lies inside a string literal is **not** treated as a citation, applying the same deterministic left-to-right quote-tracking rule as [§FS-fmt.2.3.1](../functional-spec/FS-fmt.md#231-string-literal-exclusion-rule). This keeps an ID-shaped substring inside a runtime string from raising a false dangling-ref. Marker-prefixed citations are recognized regardless of string context — a `§` in a string is a deliberate citation. Markdown files have no string literals, so the carve-out does not apply there. In `[reference] strict = true` mode only marker-prefixed citations are recognized at all. A declaration's own heading line is never counted as a citation of the ID it declares.

## 3. Output

The scanner produces a `Findings` struct containing:

- `declarations: BTreeMap<Id, Vec<Declaration>>` — keyed by ID, with file/line, stub-info, and the recorded sections (each section path paired with its heading text — §2.2) per declaration. An `E2E` declaration (§6) carries its case-directory path, fixture list, invocation, and expected exit code instead.
- `citations: Vec<Citation>` — each with the referenced ID, optional section, file, line, and start column, plus whether it was written marker-prefixed or bare.

This is the only structured output the scanner produces. Everything downstream (checking, showing, IDE diagnostics) operates on this data structure.

## 4. Inline declarations in language doc-comments

The scanner is designed so that an inline declaration — most commonly an `AS-NNN-<slug>` for an architectural spec — can live inside the **class, method, module, or package doc-comment** of any major language. This makes class-level documentation a first-class place to put architecture specs: the spec body sits with the code it describes, and a stub under `docs/architectural-spec/` points at it through a single-line H1 of the form `# <ID>: [<path>](<path>)`.

The recognized doc-comment forms (matched as comment prefixes preceding the heading line):

| Language(s)              | Doc-comment form                                  | How the regex sees it                |
|--------------------------|---------------------------------------------------|--------------------------------------|
| Java, Kotlin, Scala      | `/** … */` (Javadoc / KDoc / Scaladoc)            | `/*` opens; ` * ` on continuation    |
| C, C++                   | `/** … */` (Doxygen) or `/// …`                   | `/*` or `//` (covers `///`)          |
| C#                       | `/// <summary>…</summary>` (XML doc)              | `//` (covers `///`)                  |
| Rust                     | `/// …` outer, `//! …` inner, `/** … */` block    | `//` covers `///` and `//!`; `/*` for block |
| TypeScript, JavaScript   | `/** … */` (JSDoc / TSDoc)                        | `/*` opens; ` * ` on continuation    |
| Go                       | `// …` block immediately above the declaration    | `//`                                 |
| Swift                    | `/// …` or `/** … */`                             | `//` or `/*`                          |
| PHP                      | `/** … */` (PHPDoc)                               | `/*` opens; ` * ` on continuation    |
| Ruby                     | `# …` lines (RDoc / YARD)                         | `#` (see note 4.1)                    |
| Python                   | `""" … """` triple-quoted docstring               | special-cased (see note 4.2)         |

This table documents the doc-comment *conventions* for the languages `gnd` is built to serve. It is not the gate: the gate is the `[scan] comment_prefixes` list ([§FS-config.3.5](../functional-spec/FS-config.md#35-scan--what-gets-walked)), whose default also recognizes `;` (Lisp / Scheme / Clojure), `--` (SQL / Haskell / Lua / Ada), and bare `*` / `/*` block-comment lines. Any line whose first non-whitespace run is a configured prefix can host a declaration heading or a citation; a language not in the table still works as long as its comment marker is in `comment_prefixes`.

A canonical example — a Java class whose Javadoc *is* the architectural spec:

```java
/**
 * # AS-event-bus: Asynchronous event distribution
 *
 * ## 1. Responsibilities
 * The event bus owns subscription state and …
 *
 * ## 2. Threading model
 * Single-writer, multi-reader …
 */
public final class EventBus { … }
```

Matched by the matching stub `docs/architectural-spec/AS-<event-bus>.md`:

```
# AS-event-bus: [src/main/java/com/example/EventBus.java](src/main/java/com/example/EventBus.java)
```

### 4.1 Ruby and Python edge cases

- **Ruby** uses `#` as the only comment marker, which collides with markdown heading characters. The scanner requires the heading hashes to follow a clear comment prefix and at least one space, so `# # AS-<event-bus>` (Ruby comment, then a level-1 heading) is the canonical form. A bare `## AS-014-…` line in a `.rb` file is treated as a level-2 heading (markdown-style) and recognized as a declaration. Both work; the Ruby form is preferred for clarity.
- **Python** docstrings are not comments but string literals (`""" … """`). The scanner has a small docstring mode for `.py`: when a triple-quoted string opens, lines inside it are scanned the same way as comment continuation lines until the matching close. This lets a Python class or module docstring be a fully-featured spec home.

## 5. Why regex, not a parser

Specs live in markdown *and* in source-file doc-comments across half a dozen languages. A real parser per language would be far more code and far slower than a single line-oriented regex pass. The scheme is deliberately designed to be regex-recognizable: the heading shape is unambiguous and the citation shape is anchored on word boundaries.

The trade-off: we cannot reason about the surrounding code structure. We do not need to — IDs are syntactic, not semantic. The link in the stub heading is the only structural pointer between a stub and the code that hosts the inline spec, and it is verified by [§AS-checker.2.4](AS-checker.md#24-broken-inline-spec-stubs-fs-check34).

The marker character recognized in citations follows [§DF-reference-marker](../decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger); the regex shape changes when the marker is reconfigured per [§G-configurable](../goals/goals.md#g-configurable-every-default-is-overridable).

## 6. E2E case declarations

`E2E` is reserved in the default `[[kinds]]` set, and case-directory discovery follows the directory contract below.

The `E2E` kind is the one kind not declared by a heading line. An `E2E` declaration is a **case directory** directly under the `E2E` kind's `[[kinds]] folder` (default `e2e/cases`). The directory's name is the declared ID with the leading `{kind}` placeholder and its following literal stripped — under the default `[id] format = "{kind}-{number}-{slug}"`, a directory `007-login` declares `E2E-007-login`; under `{kind}-{slug}`, `login` declares `E2E-<login>`; under `{kind}-{number}`, `007` declares `E2E-007`. The directory name must match the format with the kind portion removed; directories that do not (e.g. `.gitkeep`, or a folder with no `expected.exit`) are skipped, so `e2e/cases/` itself never becomes a declaration.

A case directory is recognized as a declaration only if it contains an `expected.exit` file (the minimal marker of a real case). The `Declaration` recorded for it carries the directory path with `line = 1`, an empty section set (the fixture file set is not a numbered-heading tree, so any section-bearing citation of an `E2E` ID — a `.2` suffix and so on — is a missing-section error per [§AS-checker.2.3](AS-checker.md#23-missing-sections-fs-check32)), and the deterministic, sorted list of the case's fixture files plus the invocation (`command.args` contents, or the implicit `gnd check` when absent) and the expected exit code — this is the "body" [§FS-show.2.4](../functional-spec/FS-show.md#24-e2e-cases) prints. E2E declarations are never stubs, are never hosted in code, and are not reported as unused when no spec cites them.

Citations of an `E2E` ID resolve like any other: an `E2E-<name>` cite from a spec ("proven by …") is a dangling-ref error ([§AS-checker.2.2](AS-checker.md#22-dangling-citations-fs-check31)) when no `e2e/cases/<name>/` case directory exists.

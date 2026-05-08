# AS-001-scanner: how gnd discovers declarations and citations

The scanner is the single tree-walk that produces all of gnd's input data. Every check in FS-001-check and every retrieval in FS-002-show derives from what the scanner finds. Speed (G-002-fast-feedback) is set here.

## 1. Tree walk

Recursive walk from a root path using the `ignore` crate (the same walker that powers `ripgrep`). The walker is chosen specifically because it gives us `.gitignore` support for free — see 1.1 below.

Directory-level skip rules:

- Hidden directories (any name starting with `.`) are skipped.
- Build/output directories (`target`, `node_modules`, `dist`, `build`, `.next`, `.venv`) are skipped.
- The skip list is configurable per G-006-configurable and FS-006-config.3.5.

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

A regex matches heading lines of the form `<comment-prefix>? #+ <KIND>-<NNN>-<slug>`. When it matches, the line opens a new "current declaration" context.

### 2.2 Section detection

Within a declaration context, a regex matches numbered subsection headings (`## 3.`, `### 3.1`, etc.) and records the section path on the current declaration.

### 2.3 Citation detection

The citation regex matches every bare ID token in the file (whether on heading lines or in prose). The declaration line itself is excluded from being its own citation.

## 3. Output

The scanner produces a `Findings` struct containing:

- `declarations: BTreeMap<Id, Vec<Declaration>>` — keyed by ID, with file/line/sections/stub-info per declaration.
- `citations: Vec<Citation>` — each with the referenced ID, optional section, file, and line.

This is the only structured output the scanner produces. Everything downstream (checking, showing, IDE diagnostics) operates on this data structure.

## 4. Inline declarations in language doc-comments

The scanner is designed so that an inline declaration — most commonly an `AS-NNN-<slug>` for an architectural spec — can live inside the **class, method, module, or package doc-comment** of any major language. This makes class-level documentation a first-class place to put architecture specs: the spec body sits with the code it describes, and a stub under `docs/architectural-spec/` points at it via `Defined-in:`.

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

A canonical example — a Java class whose Javadoc *is* the architectural spec:

```java
/**
 * # AS-014-event-bus: Asynchronous event distribution
 *
 * ## 1. Responsibilities
 * The event bus owns subscription state and …
 *
 * ## 2. Threading model
 * Single-writer, multi-reader …
 */
public final class EventBus { … }
```

Matched by the matching stub `docs/architectural-spec/AS-014-event-bus.md`:

```
# AS-014-event-bus

Defined-in: src/main/java/com/example/EventBus.java
```

### 4.1 Ruby and Python edge cases

- **Ruby** uses `#` as the only comment marker, which collides with markdown heading characters. The scanner requires the heading hashes to follow a clear comment prefix and at least one space, so `# # AS-014-event-bus` (Ruby comment, then a level-1 heading) is the canonical form. A bare `## AS-014-…` line in a `.rb` file is treated as a level-2 heading (markdown-style) and recognized as a declaration. Both work; the Ruby form is preferred for clarity.
- **Python** docstrings are not comments but string literals (`""" … """`). The scanner has a small docstring mode for `.py`: when a triple-quoted string opens, lines inside it are scanned the same way as comment continuation lines until the matching close. This lets a Python class or module docstring be a fully-featured spec home.

## 5. Why regex, not a parser

Specs live in markdown *and* in source-file doc-comments across half a dozen languages. A real parser per language would be far more code and far slower than a single line-oriented regex pass. The scheme is deliberately designed to be regex-recognizable: the heading shape is unambiguous and the citation shape is anchored on word boundaries.

The trade-off: we cannot reason about the surrounding code structure. We do not need to — IDs are syntactic, not semantic. The `Defined-in:` pointer is the only structural link between a stub and the code that hosts the inline spec, and it is verified by AS-002-checker.2.4.

The marker character recognized in citations follows DF-001-reference-marker; the regex shape changes when the marker is reconfigured per G-006-configurable.

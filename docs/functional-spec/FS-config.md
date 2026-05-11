# FS-config: gnd reads a TOML config file under .agents/

`gnd` is zero-config out of the box ([§G-zero-config](../goals/goals.md#g-zero-config-works-on-any-conformant-tree)) and fully configurable when a project's conventions diverge ([§G-configurable](../goals/goals.md#g-configurable-every-default-is-overridable)). This spec defines the contract: where the config lives, what it contains, what it overrides, and how malformed configs are reported.

## 1. File location and discovery

The config file is **`.agents/gnd.toml`** — the `gnd.toml` lives inside an `.agents/` directory at the repo root. Discovery walks upward from the working directory until a directory containing `.agents/gnd.toml` is found, mirroring how `cargo` finds `Cargo.toml`. The directory **containing `.agents/`** is the **config root**; relative paths inside the config are resolved against it (not against `.agents/`).

`.agents/` is a single-purpose directory: it holds agent-facing tooling configuration that does not belong at the repo root next to the project's own metadata files. Other agent tools may colocate their configuration here in the future; `gnd` only owns `.agents/gnd.toml`.

If no `.agents/gnd.toml` is found, `gnd` runs with the built-in defaults defined in this spec. The defaults are the canonical `gnd` grammar — they are not stored in any file.

## 2. Precedence

CLI flags > `gnd.toml` > built-in defaults. Layering is shallow: a value present in `gnd.toml` overrides the entire corresponding default; CLI flags override individual leaf values.

## 3. Schema

The config file is TOML. Every key is optional; omitted keys take the default value. Unknown keys are an **error**, not a warning, per [§G-friendliness-first](../goals/goals.md#g-friendliness-first-as-user--and-agent-friendly-as-possible) — typos in config files are bugs and gnd surfaces them loudly.

The recognized surface is the line-oriented subset that the schema below uses: one `key = value` per line, basic (double-quoted) strings, booleans, integers, and single-line `["…", "…"]` arrays of basic strings; `#` comments; `[table]` and `[[array.of.tables]]` headers. Multi-line arrays, inline `{ … }` tables, and other TOML constructs are not parsed — keep each value on one line. A line that does not fit this shape is reported as an error pointing at the offending line, per §4.3.

Top-level keys:

```toml
gnd_config_version = 1
project_name = "Example" # optional metadata written by `gnd init`
```

`project_name` is metadata only. `gnd` accepts and preserves it in generated config, but no checker, scanner, formatter, or query behavior depends on it.

### 3.1 `[reference]` — citation form

```toml
[reference]
marker            = "§"      # default; rare character that prefixes a citation in prose
trigger           = "$$"     # default; typed sequence rewritten to marker by IDE plugin and `gnd fmt`
strict            = false    # default; if true, bare citations are NOT recognized
require_grounding = false    # default; if true, `check` flags source files that cite no declared ID
```

Per [§DF-reference-marker](../decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger). `strict = true` requires a non-empty `marker`.

`require_grounding = true` adds the ungrounded-source-file error ([§FS-check.3.6](FS-check.md#36-ungrounded-source-file-opt-in)): every scanned non-Markdown file must carry at least one resolving citation, or declare an ID inline. `gnd check --require-grounding` forces it on for one run. Per [§DF-require-grounding](../decisions/functional/DF-require-grounding.md#df-require-grounding-an-opt-in-check-that-every-source-file-cites-a-spec); off by default so adopting the discipline is a deliberate step, like `strict`.

### 3.2 `[id]` — ID grammar

```toml
[id]
format             = "{kind}-{number}-{slug}"
section_separator  = "."
number_pattern     = "\\d+"
slug_pattern       = "[a-z0-9][a-z0-9-]*"
```

`format` is a template: `{kind}`, `{number}`, `{slug}` are placeholders; everything else is literal. `{kind}` is required. `{number}` and `{slug}` are individually optional — but **at least one** of them must appear, because a bare kind would not identify a declaration. The literal characters between placeholders may be anything — `-`, `_`, `.`, `:`, etc.

The three canonical shapes:

| `format`                       | Example ID            | Disambiguator                |
|--------------------------------|-----------------------|------------------------------|
| `"{kind}-{number}-{slug}"`     | `FS-NNN-<slug>`       | number; slug is descriptive  |
| `"{kind}-{number}"`            | `FS-NNN`              | number                       |
| `"{kind}-{slug}"`              | `FS-<slug>`           | slug must be unique per kind |

When `{number}` is omitted, slugs must be unique within each kind — two declarations sharing a kind and slug collide on the same ID and are reported as duplicate declarations (per [§FS-check.3](FS-check.md#3-errors-detected)). When `{number}` is present, slugs are descriptive only and may repeat across declarations with different numbers.

`section_separator` must not collide lexically with any literal in `format` or with `slug_pattern`. gnd validates this on load and refuses ambiguous configs.

The chosen format is repo-wide. Mixing styles in one tree (some IDs numbered, others slug-only) is not supported — citations would look identical but resolve differently. Pick one shape per repo and keep it stable.

### 3.3 Section paths — arbitrary nesting depth

Section coordinates are **dotted paths of arbitrary depth**. There is no maximum nesting level. All of the following are valid section references when the corresponding heading exists in the declaration:

```
§FS-check.3
§FS-check.3.1
§FS-check.3.1.2
§FS-check.3.1.2.7.4
```

Section depth in the citation must match a heading at that depth in the declaration. The scanner records every numbered heading inside a declaration body and validates citations against the recorded set, so a project that wants four-deep nesting (`## 1.`, `### 1.1`, `#### 1.1.1`, `##### 1.1.1.1`) is supported with no config changes — the dotted path simply grows.

The default `section_separator` is `.`. Projects that prefer `:` (`§FS-check:3.1.2`) or `#` (`RFC-42#3.1.2`) override it; the dotted **components** stay separated by `.` regardless of the outer separator. Example with `section_separator = "#"`:

```
§FS-check#3.1.2     ← outer separator is `#`, intra-section separator is `.`
```

This split keeps the section grammar regular at any depth.

### 3.4 `[[kinds]]` — recognized prefixes

One `[[kinds]]` table per allowed prefix. `folder` is the conventional home for declarations of this kind — used by `gnd id` ([§FS-id.2.2](FS-id.md#22---format-json) emits it as the `folder` field) and by editor "create new declaration" / "go to home folder" actions; it is **not** enforced by the checker — declarations are recognized wherever they appear. `title` is human-readable metadata: it surfaces in `gnd show --format=json`, `gnd refs --format=json`, and IDE hover previews, and is **not** injected into `gnd show --format=md` text (which is the declaration verbatim — [§FS-show.3](FS-show.md#3-outputs)).

The defaults declare the canonical seven, in this order:

```toml
[[kinds]]
prefix = "G"
folder = "docs/goals"
title  = "Goal"

[[kinds]]
prefix = "FS"
folder = "docs/functional-spec"
title  = "Functional spec"

[[kinds]]
prefix = "AS"
folder = "docs/architectural-spec"
title  = "Architectural spec"

[[kinds]]
prefix = "DF"
folder = "docs/decisions/functional"
title  = "Functional decision"

[[kinds]]
prefix = "DA"
folder = "docs/decisions/architectural"
title  = "Architectural decision"

[[kinds]]
prefix = "E2E"
folder = "e2e/cases"
title  = "End-to-end test"

[[kinds]]
prefix = "RM"
folder = "docs"
title  = "Roadmap milestone"
```

`G` declarations are H2 headings inside the single file `docs/goals/goals.md` (one file, all goals inline); `RM` declarations are likewise H2 headings inside the single file `docs/roadmap.md` (one file, all milestones inline) — `folder` is `docs` because that file lives at the top of `docs/`; `FS`, `AS`, `DF`, and `DA` declarations are the H1 of a file in their `folder` (an `AS` declaration may instead live inline in a source doc-comment with an optional stub in `folder` — [§AS-scanner.4](../architectural-spec/AS-scanner.md#4-inline-declarations-in-language-doc-comments)); `E2E` declarations are case directories under `folder` rather than heading lines — [§AS-scanner.6](../architectural-spec/AS-scanner.md#6-e2e-case-declarations). A project that overrides this list replaces the defaults entirely — there is no merge. To extend rather than replace, copy the defaults and add to them.

Prefix sets must be unambiguous: no kind's `prefix` may itself be a prefix of another kind's `prefix`. For example, `prefix = "DA"` and `prefix = "DAT"` together are invalid because a token starting with `DAT-` would parse as either kind. gnd validates this on load and refuses ambiguous configs with a single error pointing at the offending pair (per §4.3).

### 3.5 `[scan]` — what gets walked

```toml
[scan]
include            = ["docs", "e2e", "src"]
exclude            = ["target", "node_modules", ".git", "dist", "build", ".venv"]
extensions         = ["md", "rs", "go", "java", "kt", "ts", "tsx", "js", "py", "c", "cpp", "swift", "scala", "rb", "php", "cs"]
comment_prefixes   = ["//", "#", ";", "--", "*", "/*"]
docstring_python   = true
respect_gitignore  = true
```

`include` is the set of paths walked **from the config root** — the directory containing `.agents/`, or, when no `.agents/gnd.toml` was discovered, the current working directory (never a subdirectory that merely happened to be passed as `gnd`'s path argument). So in a config-less repo `gnd` (no path) and `gnd check .` both walk `docs/`, `e2e/`, `src/` relative to the cwd, while `gnd check src/foo` or `gnd check lib/` scans exactly the file or directory it is handed — an explicit path argument overrides `include` rather than being filtered by it. A walk that ends up reading no files at all is reported, not silently passed ([§FS-check.2.2](FS-check.md#22-empty-scan)). `exclude` is the set of directory names skipped at any depth. `extensions` filters which files are read. `comment_prefixes` are the markers recognized when looking for inline declarations and citations in source files. `docstring_python` enables Python triple-quoted-string scanning in addition to `#` comments.

The default `comment_prefixes` set is broader than the languages tabulated in [§AS-scanner.4](../architectural-spec/AS-scanner.md#4-inline-declarations-in-language-doc-comments): it also covers `;` (Lisp / Scheme / Clojure), `--` (SQL / Haskell / Lua / Ada), and `*` / `/*` (block-comment continuation and opener). Any line whose first non-whitespace run is a configured prefix is eligible to host a declaration heading or a citation; the [§AS-scanner.4](../architectural-spec/AS-scanner.md#4-inline-declarations-in-language-doc-comments) table documents the doc-comment *conventions* for the major languages, not the full set of recognized prefixes.

`respect_gitignore` (default `true`) makes the scanner honor every form of ignore file the `ignore` crate recognizes — `.gitignore` at any depth, `.git/info/exclude`, the global `core.excludesFile`, and `.ignore` files. Set to `false` only when you genuinely need to scan ignored paths. The directory-level `exclude` list above is applied **in addition** to ignore-file rules, never instead of them. See [§AS-scanner.1.1](../architectural-spec/AS-scanner.md#11-respecting-gitignore-and-friends).

### 3.6 `[output]` — report format

```toml
[output]
format         = "text"   # text | json
color          = "auto"   # auto | always | never
relative_paths = true     # show paths relative to config root in reports
```

`relative_paths = true` (default) renders every `<path>` in a report relative to the config root (§1). `relative_paths = false` renders them relative to the path argument passed on the command line — or to the current working directory when no path is given — i.e. the same base `gnd` uses when no `.agents/gnd.toml` is discovered. Either way `gnd` **never** emits an absolute path or a path that escapes above the chosen base; this is what keeps the report deterministic per [§FS-errors.4](FS-errors.md#4-determinism). `color` controls ANSI styling once the colored-output feature lands ([§FS-errors.3](FS-errors.md#3-message-text)); until then output is plain bytes regardless of this value, and a change to that default goes through the [§G-no-silent-breakage](../goals/goals.md#g-no-silent-breakage-changes-ship-through-a-deprecation-path) path.

### 3.7 `[fmt.cross_refs]` — cross-reference emission

```toml
[fmt.cross_refs]
enabled       = false      # default; --cross-refs overrides per-invocation
anchor_format = "github"   # default; one of github | gitlab | mkdocs | pandoc | none
```

The full contract for this block — what `enabled` does, the named `anchor_format` profiles, and when the cross-reference pass runs — lives in [§FS-fmt.6.7](FS-fmt.md#67-configurability) and [§DF-md-link-anchor-strategy](../decisions/functional/DF-md-link-anchor-strategy.md#df-md-link-anchor-strategy-heading-text-slugs-re-derived-on-every-fmt-pass). It is part of the schema here because the generated `.agents/gnd.toml` ([§FS-init.2.4](FS-init.md#24-generated-agentsgndtoml)) writes every key in this section explicitly. `[fmt.cross_refs]` is the home for cross-reference settings; today `gnd fmt --cross-refs` only emits the Markdown inline-link form ([§FS-fmt.6](FS-fmt.md#6-cross-reference-emission-with---cross-refs)), so `anchor_format` is the only knob — a future markup family adds its settings under this same block ([§FS-fmt.6.7](FS-fmt.md#67-configurability)), additively, with no `gnd_config_version` bump (§5).

## 4. Validation and inspection

### 4.1 `gnd config validate [path]`

Loads the config discovered by walking up from `path` (or `.` when omitted), checks the schema, and reports problems. Exits 0 on success, 1 on validation errors — the error in the same `error: <path>:<line>: <message>` shape §4.3 defines. No tree scan is performed.

### 4.2 `gnd config show [path]`

Prints the **effective** configuration — defaults merged with the config discovered by walking up from `path` (or `.` when omitted), plus CLI flags — as TOML. Useful for debugging "why did gnd recognize this citation" or "what does my config actually evaluate to."

### 4.3 Invalid config behavior

A `gnd.toml` that fails validation causes every `gnd` subcommand to exit with code 2 (code 1 for `gnd config validate` itself, §4.1) and a single error message pointing at the first problem, in the form `error: <path>:<line>: <message>` on stderr ([§FS-errors.2.2](FS-errors.md#22-cli-level-message), [§FS-check.2.1.1](FS-check.md#211-cli-level-messages)) — the `error:` prefix marks it a CLI-level failure, the `<path>:<line>:` inside the text points at the offending key or line. Subsequent problems are not reported until the first is fixed — this avoids cascading errors that obscure the root cause.

## 5. Schema versioning

The TOML file may include a top-level `gnd_config_version = N`. The current version is **1**. Future incompatible schema changes increment this; gnd refuses to load a config whose version is greater than the gnd binary's known maximum, with an error suggesting an upgrade. Configs with no version key are interpreted as version 1.

## 6. What is NOT configured here

Per [§G-friendliness-first](../goals/goals.md#g-friendliness-first-as-user--and-agent-friendly-as-possible), the following are deliberately **not** configurable, to avoid the trap of every gnd repo behaving differently in surprising ways:

- The set of severity levels (only `error` and `warning` exist).
- The exit code mapping (`0`/`1`/`2` per [§FS-check.2](FS-check.md#2-outputs)).
- The ordering of the report (always deterministic).
- Anything that would let two correctly-configured gnd installs disagree on whether a given repo is well-formed.

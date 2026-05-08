# FS-006-config: gnd reads a TOML config file under .agents/

`gnd` is zero-config out of the box (G-003-zero-config) and fully configurable when a project's conventions diverge (G-006-configurable). This spec defines the contract: where the config lives, what it contains, what it overrides, and how malformed configs are reported.

## 1. File location and discovery

The config file is **`.agents/gnd.toml`** — the `gnd.toml` lives inside an `.agents/` directory at the repo root. Discovery walks upward from the working directory until a directory containing `.agents/gnd.toml` is found, mirroring how `cargo` finds `Cargo.toml`. The directory **containing `.agents/`** is the **config root**; relative paths inside the config are resolved against it (not against `.agents/`).

`.agents/` is a single-purpose directory: it holds agent-facing tooling configuration that does not belong at the repo root next to the project's own metadata files. Other agent tools may colocate their configuration here in the future; `gnd` only owns `.agents/gnd.toml`.

If no `.agents/gnd.toml` is found, `gnd` runs with the built-in defaults defined in this spec. The defaults are the canonical `gnd` grammar — they are not stored in any file.

## 2. Precedence

CLI flags > `gnd.toml` > built-in defaults. Layering is shallow: a value present in `gnd.toml` overrides the entire corresponding default; CLI flags override individual leaf values.

## 3. Schema

The config file is TOML. Every key is optional; omitted keys take the default value. Unknown keys are an **error**, not a warning, per G-005-friendliness-first — typos in config files are bugs and gnd surfaces them loudly.

### 3.1 `[reference]` — citation form

```toml
[reference]
marker  = "§"      # default; rare character that prefixes a citation in prose
trigger = "$$"     # default; typed sequence rewritten to marker by IDE plugin and `gnd fmt`
strict  = false    # default; if true, bare citations are NOT recognized
```

Per DF-001-reference-marker. `strict = true` requires a non-empty `marker`.

### 3.2 `[id]` — ID grammar

```toml
[id]
format             = "{kind}-{number}-{slug}"
section_separator  = "."
number_pattern     = "\\d+"
slug_pattern       = "[a-z0-9][a-z0-9-]*"
```

`format` is a template: `{kind}`, `{number}`, `{slug}` are placeholders; everything else is literal. `{slug}` is optional; `{kind}` and `{number}` are required (without them gnd cannot disambiguate). The literal characters between placeholders may be anything — `-`, `_`, `.`, `:`, etc.

`section_separator` must not collide lexically with any literal in `format` or with `slug_pattern`. gnd validates this on load and refuses ambiguous configs.

### 3.3 Section paths — arbitrary nesting depth

Section coordinates are **dotted paths of arbitrary depth**. There is no maximum nesting level. All of the following are valid section references when the corresponding heading exists in the declaration:

```
§FS-001-check.3
§FS-001-check.3.1
§FS-001-check.3.1.2
§FS-001-check.3.1.2.7.4
```

Section depth in the citation must match a heading at that depth in the declaration. The scanner records every numbered heading inside a declaration body and validates citations against the recorded set, so a project that wants four-deep nesting (`## 1.`, `### 1.1`, `#### 1.1.1`, `##### 1.1.1.1`) is supported with no config changes — the dotted path simply grows.

The default `section_separator` is `.`. Projects that prefer `:` (`FS-001-check:3.1.2`) or `#` (`RFC-42#3.1.2`) override it; the dotted **components** stay separated by `.` regardless of the outer separator. Example with `section_separator = "#"`:

```
§FS-001-check#3.1.2     ← outer separator is `#`, intra-section separator is `.`
```

This split keeps the section grammar regular at any depth.

### 3.4 `[[kinds]]` — recognized prefixes

```toml
[[kinds]]
prefix = "FS"
folder = "docs/functional-spec"
title  = "Functional spec"
```

One `[[kinds]]` table per allowed prefix. `folder` is the conventional home for declarations of this kind, used by `gnd new` (future) and by editor "go to home folder" actions; it is not enforced by the checker — declarations are recognized wherever they appear. `title` is human-readable text shown by `gnd show --format=md` and IDE hover previews.

Defaults declare the canonical six: `G`, `FS`, `AS`, `DA`, `DF`, `E2E`. A project that overrides this list replaces the defaults entirely — there is no merge. To extend rather than replace, copy the defaults and add to them.

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

`include` is the set of paths walked from the config root. `exclude` is the set of directory names skipped at any depth. `extensions` filters which files are read. `comment_prefixes` are the markers recognized when looking for inline declarations in source files (see AS-001-scanner.4 for full doc-comment handling). `docstring_python` enables Python triple-quoted-string scanning in addition to `#` comments.

`respect_gitignore` (default `true`) makes the scanner honor every form of ignore file the `ignore` crate recognizes — `.gitignore` at any depth, `.git/info/exclude`, the global `core.excludesFile`, and `.ignore` files. Set to `false` only when you genuinely need to scan ignored paths. The directory-level `exclude` list above is applied **in addition** to ignore-file rules, never instead of them. See AS-001-scanner.1.1.

### 3.6 `[output]` — report format

```toml
[output]
format         = "text"   # text | json
color          = "auto"   # auto | always | never
relative_paths = true     # show paths relative to config root in reports
```

## 4. Validation and inspection

### 4.1 `gnd config validate [path]`

Loads the config (or the one at `path`), checks the schema, and reports problems. Exits 0 on success, 1 on validation errors. No tree scan is performed.

### 4.2 `gnd config show`

Prints the **effective** configuration — defaults merged with `gnd.toml` merged with CLI flags — as TOML. Useful for debugging "why did gnd recognize this citation" or "what does my config actually evaluate to."

### 4.3 Invalid config behavior

A `gnd.toml` that fails validation causes every `gnd` subcommand to exit with code 2 and a single error message pointing at the first problem. Subsequent problems are not reported until the first is fixed — this avoids cascading errors that obscure the root cause.

## 5. Schema versioning

The TOML file may include a top-level `gnd_config_version = N`. The current version is **1**. Future incompatible schema changes increment this; gnd refuses to load a config whose version is greater than the gnd binary's known maximum, with an error suggesting an upgrade. Configs with no version key are interpreted as version 1.

## 6. What is NOT configured here

Per G-005-friendliness-first, the following are deliberately **not** configurable, to avoid the trap of every gnd repo behaving differently in surprising ways:

- The set of severity levels (only `error` and `warning` exist).
- The exit code mapping (`0`/`1`/`2` per FS-001-check.2).
- The ordering of the report (always deterministic).
- Anything that would let two correctly-configured gnd installs disagree on whether a given repo is well-formed.

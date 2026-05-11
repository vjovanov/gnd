---
name: gnd-init
description: Use when bootstrapping or adopting gnd in a repository, especially when the user wants an interactive guided setup for gnd init, .agents/gnd.toml, AGENTS.md, docs scaffolding, citation format, scan scope, output format, or Markdown link settings.
---

# gnd init

Guide the user through `gnd` adoption. `gnd init` itself is non-interactive, so this skill acts as the interactive wrapper: inspect the repository, recommend suitable settings, ask the user to confirm or override every option, write `.agents/gnd.toml`, run `gnd init`, then validate.

## Workflow

1. Inspect the target repo before asking questions. Find existing specs, artifact types, roadmaps, changelogs, decisions, plans, tests, and agent instruction files before recommending anything.
2. Present a short "detected repo shape" summary and recommended setup.
3. Ask each setup/config question below. For every question, include the recommended value, repo evidence, pros, cons, and when to choose something else.
4. Write `.agents/gnd.toml` from the analysis before running `gnd init`, so generated guidance reflects the repository's actual grammar, marker, strict mode, kinds, artifact folders, and scan scope.
5. Run `gnd init [path] [--name NAME] [--append|--force]`, adding `--docs` only when the repo is fresh or explicitly needs the canonical docs/e2e scaffold.
6. Run `gnd config validate [path]` and `gnd check [path]`.
7. Summarize generated files, validation results, existing specs/artifacts found, and any follow-up cleanup.

## Repo Analysis First

Use `rg` and `rg --files` first. Prefer evidence from existing files over generic defaults.

Analyze:

- Existing `AGENTS.md`, `.agents/gnd.toml`, root `gnd.toml`, and gnd-style citations.
- Documentation layout: `docs/`, `e2e/`, `spec/`, `rfcs/`, `adr/`, `decisions/`, `roadmap`, `changelog`.
- Existing artifact types and their homes: specifications, requirements, RFCs, ADRs/decisions, roadmaps, changelogs, plans, end-to-end fixtures, examples, package READMEs, generated reports, and runtime logs. Use these to choose `[[kinds]]`, `[scan].include`, and `[scan].exclude`; do not add generic folders when the repo already has project-specific artifact homes.
- Source layout: `src/`, `lib/`, `crates/`, `packages/`, `apps/`, `services/`, `cmd/`, `internal/`, `pkg/`, `test/`, `tests/`.
- Languages from file extensions and manifests such as `Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`, `pom.xml`, `build.gradle`, `.csproj`, `Package.swift`, `Gemfile`, `composer.json`, `build.sbt`, `CMakeLists.txt`, `dbt_project.yml`.
- Ignore/build/vendor directories from `.gitignore` and common generated paths.
- Existing ID/citation patterns, including headings and tokens that look like `FS-001-login`, `FS-login`, `ADR-001`, `RFC-42`, or `§FS-...`.
- Existing rendered-doc target: GitHub default, GitLab, MkDocs, Pandoc, or unknown.
- Whether this is a fresh repo, a docs-heavy repo, or an existing codebase adopting gnd.

## Asking Style

Do not ask as a blank preference survey. Ask the user to confirm or change a recommendation.

Each prompt must include:

- Recommended value.
- Evidence from repo analysis.
- Pros.
- Cons.
- When to choose a different value.

The user should be able to accept the full recommendation set quickly, but still see and override every option.

## Init Questions

Ask first:

- Target path: default `.`
- Project name: default target directory basename
- Scaffold docs/e2e with `--docs`: default no for existing repos, yes for fresh repos
- Existing file behavior: append/update default, or `--force`

## Config Questions

Ask these in order.

### Top-level

`gnd_config_version = 1`

Do not ask unless the user is migrating schemas. Keep `1`.

`project_name`

Pros of explicit name: stable display name for agents/tools.
Cons: one more metadata value to maintain if repo is renamed.
Default: derived from target directory.

### `[reference]`

`marker`

Default: `§`.
Pros: visually distinct, avoids false positives.
Cons: awkward to type without trigger/editor help.
Alternatives: `@`, `#`, or `ref:` only if the team has strong conventions.

`trigger`

Default: `$$`.
Pros: easy typing path to `§`.
Cons: may conflict with math-heavy Markdown or template languages.
Recommend changing only if `$$` is common in the repo.

`strict`

Default: `false`.
Pros false: easier adoption; bare `FS-001-login` references work.
Cons false: more accidental matches.
Pros true: citations are intentional and explicit.
Cons true: migration requires adding markers everywhere.
Recommend false for new/easy adoption, true for mature repos.

`require_grounding`

Default: `false`.
Pros true: every scanned source file must cite or declare a grounding ID.
Cons true: high adoption cost and noisy until coverage discipline exists.
Recommend false initially; enable later or in CI once coverage is deliberate.

### `[id]`

`format`

Default: `{kind}-{number}-{slug}`.

```
       ┌──────────── citation ──────────┐
           ┌────────── ID ──────────┐
  [§] KIND - [number -] slug [.section]
   │   │       │         │       │
   │   │       │         │       └─ optional dotted path, arbitrary depth (.3, .3.1, .3.1.5, …)
   │   │       │         └───────── [a-z][a-z0-9-]*
   │   │       └─────────────────── optional ordinal (e.g. 001)
   │   └─────────────────────────── G│FS│AS│DA│DF│E2E│RM│DISC
   └─────────────────────────────── citation marker (writing only)
```

Pick one per repo and keep it stable — mixing is unsupported because citations would look identical but resolve under different rules.

| Scheme                                     | Example             | Benefit                                                                                                          | Trade-off                                                                |
|--------------------------------------------|---------------------|------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------|
| `{kind}-{number}-{slug}` *(default)*       | `FS-014-user-login` | Number is the stable identifier; slug is descriptive and can be **renamed freely** without breaking citations.   | Two tokens to type; needs `gnd id` to allocate the next number.        |
| `{kind}-{number}` (RFC-style)              | `FS-014`            | Maximally stable — no slug to drift. Familiar from RFCs/PEPs/JEPs/ADRs.                                          | Opaque at the call site: `§FS-014` tells you nothing without `gnd show`. |
| `{kind}-{slug}` *(`gnd` itself uses this)* | `FS-user-login`     | Self-describing — reads like English in prose and code. No number to allocate.                                   | Renaming a slug rewrites every citation. Slug must be unique per kind.   |

Rule of thumb: pick `{kind}-{slug}` until rename churn or ID count starts to hurt; switch to `{kind}-{number}-{slug}` when it does.

If existing IDs are detected, prefer matching them over the canonical default.

`section_separator`

Default: `.`.
Pros: natural `FS-login.3.1` syntax.
Cons: can collide with custom ID formats or slug rules.
Change only for existing conventions, e.g. `#` or `:`.

`number_pattern`

Default: `\d+`.
Pros: simple numbered IDs.
Cons: does not enforce fixed width like `001`.
Use `\d{3}` only if the team wants strict padded numbers.

`slug_pattern`

Default: `[a-z0-9][a-z0-9-]*`.
Pros: URL-friendly, portable, predictable.
Cons: excludes uppercase and underscores.
Relax only to match existing IDs.

### `[[kinds]]`

Default kinds: `G`, `FS`, `AS`, `DF`, `DA`, `E2E`, `RM`.

Ask whether to keep defaults, remove unused kinds, or add project-specific kinds.

Pros of defaults: matches gnd docs and generated scaffold.
Cons: some repos may not need all categories.
Pros of custom kinds: adapts to existing taxonomy.
Cons: replacing defaults means the full list must be copied; no merge.

For each kind ask: `prefix`, `folder`, `title`.

### `[scan]`

`include`

Default: `["docs", "e2e", "src"]`.
Pros: focused, avoids scanning root clutter.
Cons: misses specs/citations outside these dirs.
Base the recommendation on actual directories. Do not include paths that do not exist unless `--docs` will create them.

`exclude`

Default: `["target", "node_modules", ".git", "dist", "build", ".venv"]`.
Pros: skips generated/vendor-heavy trees.
Cons: can hide intentional generated docs if stored there.
Usually keep defaults and add repo-specific build/cache dirs.

`extensions`

Default includes common Markdown and source extensions.
Pros: polyglot coverage.
Cons: scanning more extensions costs time and may surface noise.
Recommend only extensions found in the repo plus Markdown, unless the repo is fresh.

`comment_prefixes`

Default: `["//", "#", ";", "--", "*", "/*"]`.
Pros: broad language support.
Cons: may match prose-like comments in some languages.
Usually keep defaults, or narrow to the detected language set for established repos.

`docstring_python`

Default: `true`.
Pros: Python docstrings can carry inline declarations/citations.
Cons: docstring scanning can surface intentional prose examples.
Recommend true if Python files exist.

`respect_gitignore`

Default: `true`.
Pros: avoids ignored/generated/vendor files.
Cons: ignored files with real specs will not be scanned.
Keep true unless the repo intentionally stores tracked specs in ignored paths.

### `[output]`

`format`

Default: `text`.
Pros text: readable locally and in CI logs.
Cons text: harder for tools to consume.
Use `json` for automation dashboards or custom CI parsing.

`color`

Default: `auto`.
Pros: readable terminal output without polluting non-TTY logs.
Cons: not fully meaningful until colored output lands.
Keep `auto`.

`relative_paths`

Default: `true`.
Pros: deterministic, repo-root-relative reports.
Cons: less convenient when running from a subdirectory and expecting local paths.
Keep true for CI and shared logs.

### `[fmt.cross_refs]`

`enabled`

Default: `false`.
Pros true: Markdown citations can become normal links for rendered docs.
Cons true: extra churn and renderer-specific anchors.
Recommend false initially; use `gnd fmt --cross-refs` explicitly.

`anchor_format`

Default: `github`.
Options: `github`, `gitlab`, `mkdocs`, `pandoc`, `none`.

Pros of matching renderer: links work in published docs.
Cons: wrong profile creates broken anchors.
Use `none` if only file links are desired.

## Recommendation Heuristics

### `--docs`

Recommend `--docs = true` when the repo has little or no `docs/` or `e2e/` structure.

Recommend `--docs = false` when the repo already has meaningful docs or tests, and suggest adding only missing gnd files.

### `[reference].strict`

Recommend `strict = false` for first adoption or when many bare ID-like tokens already exist.

Recommend `strict = true` when the repo already uses explicit marker citations, has noisy ID-like tokens, or wants deliberate citation hygiene.

### `[reference].require_grounding`

Recommend `false` for initial adoption.

Recommend `true` only when the repo already has broad spec-to-code citation coverage or the user explicitly wants a strict co-change discipline.

### `[id].format`

Recommend `{kind}-{number}-{slug}` for fresh repos and teams that want stable IDs with readable names.

Recommend `{kind}-{number}` when existing docs already use ADR/RFC-style numbered IDs.

Recommend `{kind}-{slug}` when existing docs are title/slug based and stable numeric allocation would feel artificial.

### `[[kinds]]`

Start from the default kinds.

Recommend adding custom kinds when the repo already has clear categories such as `ADR`, `RFC`, `REQ`, `API`, `SEC`, or `OPS`.

Recommend removing defaults only when they would clearly confuse the project.

### `[scan].include`

Base this on actual directories:

- Rust workspace: include `["docs", "e2e", "src", "crates", "tests"]` when present.
- JS monorepo: include `["docs", "e2e", "src", "packages", "apps", "tests"]` when present.
- Go repo: include `["docs", "e2e", "cmd", "internal", "pkg", "tests"]` when present.

### `[fmt.cross_refs].anchor_format`

Recommend:

- `github` when the repo is hosted on GitHub or no renderer is evident.
- `gitlab` for GitLab repos.
- `mkdocs` when `mkdocs.yml` exists.
- `pandoc` when Pandoc config/build scripts are evident.
- `none` when Markdown links should only point to files without section anchors.

## Language And Repo Shape Examples

Use these examples to turn repo evidence into recommended `[scan]` settings. Include only directories that exist, unless `--docs` will create them.

### Rust

Evidence: `Cargo.toml`, `Cargo.lock`, `crates/`, `src/**/*.rs`.

```toml
[scan]
include = ["docs", "e2e", "src", "crates", "tests"]
extensions = ["md", "rs"]
exclude = ["target", ".git"]
comment_prefixes = ["//", "/*", "*"]
```

Pros: covers workspace crates, integration tests, and Rust doc comments.
Cons: may miss generated docs outside `docs/`.

### TypeScript / JavaScript

Evidence: `package.json`, `pnpm-workspace.yaml`, `tsconfig.json`, `src/`, `apps/`, `packages/`.

```toml
include = ["docs", "e2e", "src", "apps", "packages", "tests"]
extensions = ["md", "ts", "tsx", "js", "jsx"]
exclude = ["node_modules", "dist", "build", "coverage", ".next", ".turbo", ".git"]
comment_prefixes = ["//", "/*", "*"]
```

Pros: works for frontend apps and monorepos.
Cons: broad monorepos may need narrower package selection.

### Python

Evidence: `pyproject.toml`, `setup.py`, `requirements.txt`, `src/`, package dirs, `tests/`.

```toml
include = ["docs", "e2e", "src", "tests"]
extensions = ["md", "py"]
exclude = [".venv", "__pycache__", ".pytest_cache", ".mypy_cache", "build", "dist", ".git"]
comment_prefixes = ["#"]
docstring_python = true
```

Pros: supports citations in comments and docstrings.
Cons: docstring scanning can surface intentional prose examples.

### Go

Evidence: `go.mod`, `cmd/`, `internal/`, `pkg/`, `*.go`.

```toml
include = ["docs", "e2e", "cmd", "internal", "pkg", "tests"]
extensions = ["md", "go"]
exclude = ["vendor", "dist", "build", ".git"]
comment_prefixes = ["//", "/*", "*"]
```

Pros: matches common Go project layout.
Cons: single-package repos may only need `["docs", "src"]` or `["docs", "."]` if code is at root.

### Java / Kotlin / Gradle

Evidence: `pom.xml`, `build.gradle`, `settings.gradle`, `src/main`, `src/test`.

```toml
include = ["docs", "e2e", "src"]
extensions = ["md", "java", "kt"]
exclude = ["target", "build", ".gradle", ".git"]
comment_prefixes = ["//", "/*", "*"]
```

Pros: covers Maven and Gradle conventions.
Cons: multi-module builds may need module directories added explicitly.

### C / C++

Evidence: `CMakeLists.txt`, `Makefile`, `src/`, `include/`, `lib/`, `tests/`.

```toml
include = ["docs", "e2e", "src", "include", "lib", "tests"]
extensions = ["md", "c", "cpp", "h", "hpp"]
exclude = ["build", "dist", "out", ".git"]
comment_prefixes = ["//", "/*", "*"]
```

Pros: covers implementation and public headers.
Cons: vendored headers should be excluded if present.

### C# / .NET

Evidence: `*.csproj`, `*.sln`, `src/`, `test/`, `tests/`.

```toml
include = ["docs", "e2e", "src", "test", "tests"]
extensions = ["md", "cs"]
exclude = ["bin", "obj", "TestResults", ".git"]
comment_prefixes = ["//", "/*", "*"]
```

Pros: covers normal solution layout.
Cons: generated code folders may need extra excludes.

### Ruby / Rails

Evidence: `Gemfile`, `app/`, `lib/`, `spec/`, `test/`.

```toml
include = ["docs", "e2e", "app", "lib", "spec", "test"]
extensions = ["md", "rb"]
exclude = ["vendor", "tmp", "log", "coverage", ".git"]
comment_prefixes = ["#"]
```

Pros: covers Rails and library conventions.
Cons: Rails apps may need to skip generated schema or fixture-heavy paths.

### PHP

Evidence: `composer.json`, `src/`, `app/`, `tests/`.

```toml
include = ["docs", "e2e", "src", "app", "tests"]
extensions = ["md", "php"]
exclude = ["vendor", "var", "cache", "build", ".git"]
comment_prefixes = ["//", "#", "/*", "*"]
```

Pros: works for Composer apps and frameworks.
Cons: framework cache dirs vary; inspect before finalizing.

### Swift

Evidence: `Package.swift`, `Sources/`, `Tests/`.

```toml
include = ["docs", "e2e", "Sources", "Tests"]
extensions = ["md", "swift"]
exclude = [".build", "DerivedData", ".git"]
comment_prefixes = ["//", "/*", "*"]
```

Pros: matches Swift Package Manager.
Cons: Xcode projects may have different app/test directories.

### Scala

Evidence: `build.sbt`, `src/main/scala`, `src/test/scala`.

```toml
include = ["docs", "e2e", "src"]
extensions = ["md", "scala"]
exclude = ["target", "project/target", ".bloop", ".metals", ".git"]
comment_prefixes = ["//", "/*", "*"]
```

Pros: covers sbt source layout.
Cons: generated sources may need explicit exclusion.

### SQL / Data Projects

Evidence: `db/`, `migrations/`, `models/`, `*.sql`, `dbt_project.yml`.

```toml
include = ["docs", "e2e", "db", "migrations", "models", "tests"]
extensions = ["md", "sql", "py", "yml", "yaml"]
exclude = ["target", "logs", ".venv", ".git"]
comment_prefixes = ["--", "#"]
```

Pros: covers dbt and migration-heavy repos.
Cons: YAML comments are line-only; generated dbt target dirs must stay excluded.

### Polyglot Monorepo

Evidence: multiple manifests and top-level `apps/`, `packages/`, `services/`, `libs/`, `tools/`.

```toml
include = ["docs", "e2e", "apps", "packages", "services", "libs", "tools", "tests"]
extensions = ["md", "rs", "go", "java", "kt", "ts", "tsx", "js", "py", "c", "cpp", "cs", "rb", "php"]
exclude = ["target", "node_modules", ".git", "dist", "build", ".venv", "coverage", ".next", ".turbo"]
comment_prefixes = ["//", "#", ";", "--", "*", "/*"]
docstring_python = true
```

Pros: broad coverage for adoption across teams.
Cons: can be noisy; recommend narrowing after the first `gnd check`.

When multiple language examples apply, merge them conservatively: union the real include dirs, union the extensions actually present, and union generated/cache excludes. Prefer a narrower first config that passes cleanly over an over-broad config that floods the user with findings.

## Validation

After writing config, run:

```bash
gnd config validate .
gnd init .
gnd check .
```

If custom config affects `AGENTS.md`, ensure `.agents/gnd.toml` exists before `gnd init` so the generated managed block reflects the selected ID grammar, marker, strict mode, kinds, and existing artifact layout.

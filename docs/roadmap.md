# Roadmap

What `gnd` plans to ship next, in priority order. Each item has a stable ID (`RM-NNN-slug`) and may be cited from anywhere — commits, PRs, the changelog, other specs. Done items move to `docs/changelog.md`; cancelled items stay here with a `~~strikethrough~~` and a one-line reason so the citation does not dangle.

`gnd` is moving from a working checker toward a tool that is **fast enough to invoke on every keystroke, friendly enough for agents to use unaided, and portable enough to ship as a CI dependency in any ecosystem**. The near-term arc is: harden the engine with an e2e corpus, expand the surface from *checking* to *retrieval* (`gnd show`), make every default overridable through configuration, and then carry the same engine onto npm, PyPI, and into editors. The IDed milestones below project that arc onto reviewable units of work.

## RM-007-self-host: restore a passing self-host loop

`cargo run -- .` against this repository must exit zero with empty stdout. As of writing it exits 1 with over a hundred errors, mostly from descending into `e2e/cases/*` fixture trees and from illustrative IDs (`FS-user-login`, `AS-event-bus`, `AS-authoring`) being scanned as real citations.

### 1. What

Three fixes, smallest blast radius first: (a) extend the default exclude list — or the scan rules — so `e2e/cases/` fixture repos do not pollute the host project's report; (b) make illustrative IDs explicitly non-semantic — fenced Markdown blocks must already be skipped by `gnd fmt` (FS-fmt), apply the same skip in the scanner; (c) suppress the trailing "declared but never cited" warnings on the success path so a passing repo emits nothing on stdout (G-friendliness-first.1.6).

### 2. Why now

Self-host is the load-bearing demonstration of G-no-dangling-refs and G-fast-feedback. Every other roadmap item assumes the engine is correct on its own corpus; until that holds, regressions in any other feature are masked by pre-existing failures.

### 3. Measurable

CI on every push to `main` runs `cargo run -- .` and asserts exit 0 with empty stdout. An e2e fixture exercises a tree with nested fixture directories and asserts they are not scanned by default.

## RM-001-e2e-corpus: ship a complete e2e fixture set and wire CI

The check engine has no regression net until every rule it enforces has at least one positive and one negative fixture in `e2e/`, and CI runs the suite on every push.

### 1. What

Positive and negative fixtures under `e2e/cases/` covering each error class in FS-check; harness wired to `cargo test` and to `.github/workflows/ci.yml`.

### 2. Why now

Without this, every other change risks silent regressions. Blocks RM-002 through RM-006.

### 3. Measurable

CI green on `main`; coverage report shows at least one fixture per FS-check error class.

## RM-002-show: implement gnd show

Per FS-show. Makes `gnd` useful as an agent retrieval tool, not just a checker.

### 1. What

`gnd show <ID>` returns the declaration body; supports `--head`, `--format=json`, and section-path lookup.

### 2. Why now

Agents need grounded retrieval more than they need stricter checks; the engine already locates declarations as part of FS-check, so the marginal cost is small.

### 3. Measurable

Output under 200 lines for the common case (G-friendliness-first.1); e2e fixtures cover stub redirection, missing IDs, and section paths.

## RM-003-config: implement .agents/gnd.toml loading

Per FS-config. Once this lands, every knob becomes overridable.

### 1. What

TOML parser, schema validation with strict-unknown-key behavior, and `gnd config show` / `gnd config validate` subcommands. `gnd_config_version` gating per FS-config.5.

### 2. Why now

DF-reference-marker and FS-fmt depend on the marker/strict toggles living in config.

### 3. Measurable

E2E fixture with non-default `[reference]`, `[scan]`, and `[ids]` blocks passes (G-configurable.3); unknown-key fixture fails with the right line.

## RM-004-marker-fmt: reference marker plus gnd fmt

Per DF-reference-marker and FS-fmt.

### 1. What

Scanner recognizes `§`-prefixed citations alongside bare ones; `gnd fmt` rewrites the typed trigger (`$$` by default) to the marker. Strict mode (per FS-config.3.1) recognizes only marked citations.

### 2. Why now

The marker is the load-bearing primitive for IDE/agent ergonomics; until it ships, FS-ide-plugins is blocked.

### 3. Measurable

Round-trip e2e: a tree with `$$` triggers, after `gnd fmt`, equals the same tree with `§` markers; strict-mode fixture passes only when every citation is marked.

## RM-008-core-cli-split: split gnd-core from gnd-cli

Workspace split before bindings ship. `src/main.rs` is currently a single 1518-line binary that mixes scanner, checker, show, fmt, init, config parsing, argument handling, and rendering.

### 1. What

`gnd-core` library crate: config loading, scanner, checker, `show` body extraction, `fmt` planning, report data structures. `gnd-cli` binary crate: argument parsing, rendering (text/JSON), exit-code mapping, help text. Today's `src/main.rs` is decomposed into these two crates with no behavior change.

### 2. Why now

RM-005 publishes three bindings from one engine; bindings need a library, not a binary. Splitting first also makes the e2e harness call into the engine directly and keeps CLI concerns (exit codes, rendering) from leaking into scanner internals.

### 3. Measurable

`gnd-core` compiles standalone; `gnd-cli`, the planned `gnd-node`, and `gnd-py` all consume it without duplicating scanner or checker code. The full e2e suite passes byte-identical reports before and after the split.

## RM-009-distribution-naming: verify package names before first publish

Pre-release sanity check: the registry names claimed across the docs may not still be available, and the docs already disagree with each other. FS-distribution and DA-reference-checker-name reach different conclusions about the Python name; this must be reconciled before publishing plans harden.

### 1. What

A pre-release CI step that queries crates.io, npm, and PyPI for each claimed package name and fails if any claimed-available name is in fact taken or owned by another project. Docs are corrected so they no longer claim a name is free unless the project owns it. Where a registry name is unavailable, an explicit alternate package name is chosen and recorded in FS-distribution.

### 2. Why now

A doc contradiction at release time is a release blocker. The check is cheap to run and cheaper to wire before the publish workflow exists than after.

### 3. Measurable

The release pipeline queries each registry and proceeds only if every claimed name resolves to either "available" or "owned by this project." FS-distribution and DA-reference-checker-name agree on every package name they mention.

## RM-005-distribution: cargo + npm + pypi from one engine

Per FS-distribution and AS-bindings. Workspace split into `gnd-core`, `gnd-cli`, `gnd-node`, `gnd-py`.

### 1. What

Workspace restructure; napi-rs binding for npm; PyO3 binding for PyPI; CI publish jobs for all three registries.

### 2. Why now

`gnd` is only viable as a CI dependency for non-Rust projects once it ships on their native package manager (G-multi-language).

### 3. Measurable

Integration test runs the same spec corpus through all three bindings and asserts byte-identical reports (G-multi-language.3).

## RM-006-ide-plugins: first-party editor integrations

Per FS-ide-plugins. One LSP server, multiple editor wrappers.

### 1. What

`gnd-lsp` server providing diagnostics, go-to-definition, and live `$$ → §` transform. Wrappers for VSCode (first), IntelliJ IDEA, Vim/Neovim, and Emacs.

### 2. Why now

The check loop is only as fast as the developer's feedback path. IDE diagnostics close the loop without a manual `gnd .` invocation (G-fast-feedback).

### 3. Measurable

VSCode extension installs from the marketplace; opening a conformant repo shows diagnostics within the speed budget of G-fast-feedback.1.

# Roadmap

What `gnd` plans to ship next, in priority order. Each item has a stable ID (`RM-NNN-slug`) and may be cited from anywhere — commits, PRs, the changelog, other specs. Done items move to `docs/changelog.md`; cancelled items stay here with a `~~strikethrough~~` and a one-line reason so the citation does not dangle.

The check engine, the retrieval surface (`gnd show`, `gnd refs`), bulk normalization (`gnd fmt`, including `--marker` and `--md-links`), config loading (`.agents/gnd.toml` plus `gnd config show` / `gnd config validate`), `gnd init`, `gnd name`, and the e2e corpus are all shipped — see `docs/changelog.md`. What remains is the **distribution arc**: split the single binary into a `gnd-core` library plus thin frontends, verify the package names, publish on npm and PyPI alongside cargo, ship the optional LSP server, and add `gnd check --watch`. The IDed milestones below project that arc onto reviewable units of work.

## RM-007-self-host: guard the self-host loop in CI

`cargo run -- .` against this repository already exits zero with empty stdout — the fenced-block skip in the scanner and this repo's slug-only `[id] format` keep the `e2e/cases/*` fixture trees and the illustrative IDs out of the host report. What is missing is the CI guard and an explicit fixture for the fixture-tree case.

### 1. What

Two pieces: (a) a CI step on every push to `main` that runs `cargo run -- .` and fails on a non-zero exit or non-empty stdout; (b) an e2e fixture exercising a tree with nested fixture directories under a canonical *default* config (numbered IDs, non-strict) and asserting they do not pollute the outer report — the default `[scan] exclude` plus scan rules must keep nested case dirs out without relying on a particular `[id] format`.

### 2. Why now

Self-host is the load-bearing demonstration of §G-no-dangling-refs and §G-fast-feedback. The loop passes today, but without the CI guard a future change could break it silently, and the current pass on this repo leans on a config coincidence rather than a guaranteed scan rule.

### 3. Measurable

CI on every push runs `cargo run -- .` and asserts exit 0 with empty stdout. A new e2e fixture proves nested fixture directories are not scanned under the default config.

## RM-008-core-cli-split: split gnd-core from gnd-cli

Workspace split before bindings ship. `src/lib.rs` is currently a single module that mixes scanner, checker, show, fmt, init, config parsing, argument handling, and rendering, with a thin `src/main.rs` calling into it.

### 1. What

`gnd-core` library crate: config loading, scanner, checker, `show` body extraction, `fmt` planning, `refs` filtering, report data structures. `gnd-cli` binary crate: argument parsing, rendering (text/JSON), exit-code mapping, help text. Today's `src/lib.rs` / `src/main.rs` are decomposed into these two crates with no behavior change.

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

Per FS-distribution and AS-bindings. Builds on the workspace split (§RM-008) and the name verification (§RM-009).

### 1. What

napi-rs binding for npm; PyO3 binding for PyPI; CI publish jobs for all three registries (`gnd-core` first, in dependency order).

### 2. Why now

`gnd` is only viable as a CI dependency for non-Rust projects once it ships on their native package manager (G-multi-language).

### 3. Measurable

Integration test runs the same spec corpus through all three bindings and asserts byte-identical reports (G-multi-language.3).

## RM-006-lsp: ship the optional LSP server

Per §FS-lsp, §AS-lsp, and §DA-lsp-optional. Adds `crates/gnd-lsp/` to the workspace and publishes it as a separate package on cargo, npm, and PyPI. No first-party per-editor wrappers ship; editor configuration is the user's one-time work, with example snippets in the README.

### 1. What

A `gnd-lsp` binary that speaks LSP over stdio and serves the four capabilities pinned in §FS-lsp.1: diagnostics, hover preview, go-to-definition, and the live `$$ → §` transform (the bulk form of which already ships in `gnd fmt`). Holds an in-memory `Findings` per workspace; full re-scan strategy on every change for v1 (§AS-lsp.3.1). Parity with the CLI is enforced by an e2e harness that drives the LSP through the same `e2e/cases/*` corpus and asserts byte-equivalent output (§AS-lsp.5).

Distribution: separate package on each registry (§FS-distribution.1). The CLI install does not pull in `gnd-lsp` transitively. README gains a section with example LSP-client snippets for Helix, Neovim, Zed, Emacs, VSCode (generic LSP client extension), and IntelliJ via LSP4IJ.

### 2. Why now

The reframed §raison-detre.2 keeps Markdown links peripheral and centers verify/refactor-safe/extract — three pillars all satisfied by CLI-shaped surfaces. Editor integration is then a UX layer over those, and the cheapest non-zero answer is one LSP server every editor can talk to. Shipping this after §RM-005 (bindings) and §RM-008 (workspace split) means the engine is already factored as a library and the registries are already wired.

### 3. Depends on

- §RM-008-core-cli-split must land first; without `gnd-core` as a library, `gnd-lsp` has nothing to depend on.

### 4. Measurable

`gnd-lsp` installs from each registry. An editor pointed at the binary receives diagnostics, hover bodies, and definition jumps for any conformant repo, and parity tests assert byte-equivalence with `gnd check` and `gnd show` across the e2e corpus. Diagnostic latency on file change is within §G-fast-feedback.1's per-scan budget.

## RM-012-watch: implement gnd check --watch

Per §FS-check.6. The editor-less "every save" loop §G-fast-feedback exists for — re-run `gnd check` on every change under the scanned tree, clearing prior output each run.

### 1. What

`--watch` on `gnd check` (and `gnd --watch` as shorthand): filesystem-notification-driven, debounced, no polling and no configurable interval. Each run is byte-identical to a plain `gnd check` on the tree's current state; on Ctrl-C the process exits with the last completed run's exit code. Non-interactive — no TUI, no key bindings (§FS-non-goals.10), no network (§FS-non-goals.11).

### 2. Why now

`gnd-lsp` (§RM-006) covers editor users; `--watch` covers everyone else with zero editor configuration, and it is small once the engine is a library (§RM-008). Sequenced after §RM-008 so the watcher calls `gnd-core::scan`/`check` rather than re-implementing the walk.

### 3. Measurable

An e2e fixture starts `gnd check --watch` on a clean fixture (asserts silent first run), writes a file that introduces a dangling ref (asserts the next run prints it), removes the bad citation (asserts the run goes silent again), then sends SIGINT (asserts exit code matches the last run). A second fixture asserts `--format=json` emits one self-contained report per run.

# Roadmap

What `gnd` plans to ship next, in priority order. Each item has a stable ID — `RM-<slug>` under this repo's `[id] format` (§FS-config.3.2); `RM` is a configured `[[kinds]]` prefix (§FS-config.3.4), so `gnd check` validates `§RM-…` citations like any other. Items may be cited from anywhere — commits, PRs, the changelog, other specs. Shipped items move their detail to `docs/changelog.md` and keep a one-line pointer in §"Shipped milestones" below so the citation does not dangle; cancelled items stay in place with a `~~strikethrough~~` title and a one-line reason.

The check engine, the retrieval surface (`gnd show`, `gnd refs`, including E2E case manifests), bulk normalization (`gnd fmt`, including `--marker` and `--md-links`), config loading (`.agents/gnd.toml` plus `gnd config show` / `gnd config validate`), `gnd init`, `gnd name`, and the e2e corpus are all shipped — see `docs/changelog.md`. What remains is the **distribution arc**: split the single binary into a `gnd-core` library plus thin frontends, verify the package names, publish on npm and PyPI alongside cargo, ship the optional LSP server, and add `gnd check --watch`. The IDed milestones below project that arc onto reviewable units of work.

## RM-self-host: guard the self-host loop in CI

`cargo run -- .` against this repository already exits zero with empty stdout — the fenced-block skip in the scanner and this repo's slug-only `[id] format` keep the `e2e/cases/*` fixture trees and the illustrative IDs out of the host report. What is missing is the CI guard and an explicit fixture for the fixture-tree case.

### 1. What

Two pieces: (a) a CI step on every push to `main` that runs `cargo run -- .` and fails on a non-zero exit or non-empty stdout; (b) an e2e fixture exercising a tree with nested fixture directories under a canonical *default* config (numbered IDs, non-strict) and asserting they do not pollute the outer report — the default `[scan] exclude` plus scan rules must keep nested case dirs out without relying on a particular `[id] format`.

### 2. Why now

Self-host is the load-bearing demonstration of §G-no-dangling-refs and §G-fast-feedback. The loop passes today, but without the CI guard a future change could break it silently, and the current pass on this repo leans on a config coincidence rather than a guaranteed scan rule.

### 3. Measurable

CI on every push runs `cargo run -- .` and asserts exit 0 with empty stdout. A new e2e fixture proves nested fixture directories are not scanned under the default config.

## RM-core-cli-split: split gnd-core from gnd-cli

Workspace split before bindings ship. `src/lib.rs` is currently a single module that mixes scanner, checker, show, fmt, init, config parsing, argument handling, and rendering, with a thin `src/main.rs` calling into it.

### 1. What

`gnd-core` library crate: config loading, scanner, checker, `show` body extraction, `fmt` planning, `refs` filtering, report data structures. `gnd-cli` binary crate: argument parsing, rendering (text/JSON), exit-code mapping, help text. Today's `src/lib.rs` / `src/main.rs` are decomposed into these two crates with no behavior change.

### 2. Why now

RM-distribution publishes three bindings from one engine; bindings need a library, not a binary. Splitting first also makes the e2e harness call into the engine directly and keeps CLI concerns (exit codes, rendering) from leaking into scanner internals.

### 3. Measurable

`gnd-core` compiles standalone; `gnd-cli`, the planned `gnd-node`, and `gnd-py` all consume it without duplicating scanner or checker code. The full e2e suite passes byte-identical reports before and after the split.

## RM-distribution-naming: verify package names before first publish

Pre-release sanity check: the registry names claimed across the docs may not still be available, and the docs already disagree with each other. FS-distribution and DA-reference-checker-name reach different conclusions about the Python name; this must be reconciled before publishing plans harden.

### 1. What

A pre-release CI step that queries crates.io, npm, and PyPI for each claimed package name and fails if any claimed-available name is in fact taken or owned by another project. Docs are corrected so they no longer claim a name is free unless the project owns it. Where a registry name is unavailable, an explicit alternate package name is chosen and recorded in FS-distribution.

### 2. Why now

A doc contradiction at release time is a release blocker. The check is cheap to run and cheaper to wire before the publish workflow exists than after.

### 3. Measurable

The release pipeline queries each registry and proceeds only if every claimed name resolves to either "available" or "owned by this project." FS-distribution and DA-reference-checker-name agree on every package name they mention.

## RM-distribution: cargo + npm + pypi from one engine

Per FS-distribution and AS-bindings. Builds on the workspace split (§RM-core-cli-split) and the name verification (§RM-distribution-naming).

### 1. What

napi-rs binding for npm; PyO3 binding for PyPI; CI publish jobs for all three registries (`gnd-core` first, in dependency order).

### 2. Why now

`gnd` is only viable as a CI dependency for non-Rust projects once it ships on their native package manager (G-multi-language).

### 3. Measurable

Integration test runs the same spec corpus through all three bindings and asserts byte-identical reports (G-multi-language.3).

## RM-lsp: ship the optional LSP server

Per §FS-lsp, §AS-lsp, and §DA-lsp-optional. Adds `crates/gnd-lsp/` to the workspace and publishes it as a separate package on cargo, npm, and PyPI. No first-party per-editor wrappers ship; editor configuration is the user's one-time work, with example snippets in the README.

### 1. What

A `gnd-lsp` binary that speaks LSP over stdio and serves the four capabilities pinned in §FS-lsp.1: diagnostics, hover preview, go-to-definition, and the live `$$ → §` transform (the bulk form of which already ships in `gnd fmt`). Holds an in-memory `Findings` per workspace; full re-scan strategy on every change for v1 (§AS-lsp.3.1). Parity with the CLI is enforced by an e2e harness that drives the LSP through the same `e2e/cases/*` corpus and asserts byte-equivalent output (§AS-lsp.5).

Distribution: separate package on each registry (§FS-distribution.1). The CLI install does not pull in `gnd-lsp` transitively. README gains a section with example LSP-client snippets for Helix, Neovim, Zed, Emacs, VSCode (generic LSP client extension), and IntelliJ via LSP4IJ.

### 2. Why now

The reframed §raison-detre.2 keeps Markdown links peripheral and centers verify/refactor-safe/extract — three pillars all satisfied by CLI-shaped surfaces. Editor integration is then a UX layer over those, and the cheapest non-zero answer is one LSP server every editor can talk to. Shipping this after §RM-distribution (bindings) and §RM-core-cli-split (workspace split) means the engine is already factored as a library and the registries are already wired.

### 3. Depends on

- §RM-core-cli-split must land first; without `gnd-core` as a library, `gnd-lsp` has nothing to depend on.

### 4. Measurable

`gnd-lsp` installs from each registry. An editor pointed at the binary receives diagnostics, hover bodies, and definition jumps for any conformant repo, and parity tests assert byte-equivalence with `gnd check` and `gnd show` across the e2e corpus. Diagnostic latency on file change is within §G-fast-feedback.1's per-scan budget.

## RM-watch: implement gnd check --watch

Per §FS-check.6. The editor-less "every save" loop §G-fast-feedback exists for — re-run `gnd check` on every change under the scanned tree, clearing prior output each run.

### 1. What

`--watch` on `gnd check` (and `gnd --watch` as shorthand): filesystem-notification-driven, debounced, no polling and no configurable interval. Each run is byte-identical to a plain `gnd check` on the tree's current state; on Ctrl-C the process exits with the last completed run's exit code. Non-interactive — no TUI, no key bindings (§FS-non-goals.10), no network (§FS-non-goals.11).

### 2. Why now

`gnd-lsp` (§RM-lsp) covers editor users; `--watch` covers everyone else with zero editor configuration, and it is small once the engine is a library (§RM-core-cli-split). Sequenced after §RM-core-cli-split so the watcher calls `gnd-core::scan`/`check` rather than re-implementing the walk.

### 3. Measurable

An e2e fixture starts `gnd check --watch` on a clean fixture (asserts silent first run), writes a file that introduces a dangling ref (asserts the next run prints it), removes the bad citation (asserts the run goes silent again), then sends SIGINT (asserts exit code matches the last run). A second fixture asserts `--format=json` emits one self-contained report per run.

## Shipped milestones

Done milestones leave their full record in `docs/changelog.md` (the `Implemented` block of the latest release). They keep a one-line declaration here so existing `§RM-…` citations still resolve — the changelog has the detail.

## RM-e2e-corpus: the e2e/cases/* corpus and CI harness

Shipped. The `e2e/cases/*` corpus, `tests/e2e.rs`, `tests/init.rs`, the per-error-class fixtures, and the byte-for-byte determinism sweep — see `docs/changelog.md`.

## RM-show: gnd show <ID>

Shipped. Whole declaration, `--head`, `--section` and dotted-inline section paths, `--full`, `--format text|md|json`, inline-source extraction, ambiguous-ID / broken-stub query forms — see `docs/changelog.md` and §FS-show.

## RM-config: .agents/gnd.toml discovery, parsing, and inspection

Shipped. The line-oriented TOML subset, unknown-key errors with `path:line:` pointers, `gnd_config_version` gating, every documented block, plus `gnd config validate` / `gnd config show` — see `docs/changelog.md` and §FS-config.

## RM-marker-fmt: the § marker, the $$ trigger, and gnd fmt

Shipped. `gnd fmt` with `--check` / `--write` / `--marker`, the deterministic string-literal carve-out, declaration-heading and fenced-block exemptions, and `[reference] strict = true` — see `docs/changelog.md`, §FS-fmt, and §DF-reference-marker.

## RM-md-link-emission: gnd fmt --md-links

Shipped. Wraps marker-prefixed citations in `.md` files only, heading-text anchor slugs per the `github` / `gitlab` / `mkdocs` / `pandoc` / `none` profiles, re-derived each pass, fenced/inline-code-span and dangling-citation skips, idempotent — see `docs/changelog.md`, §FS-fmt.6, and §DF-md-link-anchor-strategy.

## RM-refs: gnd refs <ID>

Shipped. Over the same scan as `check`, sorted by `(path, line, column)`, honouring strict mode and the string-literal carve-out, NDJSON on stdout for `--format=json` — see `docs/changelog.md` and §FS-refs.

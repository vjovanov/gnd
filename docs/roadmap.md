# Roadmap

What `gnd` plans to ship next, in priority order. Each item has a stable ID — `RM-<slug>` under this repo's `[id] format` ([§FS-config.3.2](functional-spec/FS-config.md#32-id--id-grammar)); `RM` is a configured `[[kinds]]` prefix ([§FS-config.3.4](functional-spec/FS-config.md#34-kinds--recognized-prefixes)), so `gnd check` validates `§RM-…` citations like any other. Items may be cited from anywhere — commits, PRs, the changelog, other specs. Shipped items move their detail to `docs/changelog.md` and keep a one-line pointer in §"Shipped milestones" below so the citation does not dangle; cancelled items stay in place with a `~~strikethrough~~` title and a one-line reason.

The check engine, the retrieval surface (`gnd show`, `gnd refs`, including E2E case manifests), the coverage index (`gnd cover`), bulk normalization (`gnd fmt`, including `--marker` and `--cross-refs`), config loading (`.agents/gnd.toml` plus `gnd config show` / `gnd config validate`), `gnd init`, `gnd id`, the opt-in grounding floor ([§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in)), and the e2e corpus are all shipped — see `docs/changelog.md`. Two arcs remain. The **distribution arc**: split the single binary into a `gnd-core` library plus thin frontends, verify the package names, publish on npm and PyPI alongside cargo, ship the optional LSP server, and add `gnd check --watch`. And the **grounding arc** (the third layer of [§G-agent-grounding.1](goals/goals.md#1-the-three-layers), diff-gated enforcement): build on [§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in) and [§FS-cover](functional-spec/FS-cover.md#fs-cover-gnd-groups-citations-by-scanned-file) toward a diff-aware co-change gate — implementation cannot change without the spec it grounds in and without a test of it — via a pre-commit / CI recipe that consumes `gnd cover` ([§RM-cochange-gate](roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test)). One standalone item ([§RM-benchmarks](roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-g-fast-feedback-budgets)) captures a performance baseline against today's single-binary build before the distribution arc starts moving the engine around. The IDed milestones below project both arcs onto reviewable units of work.

## RM-self-host: guard the self-host loop in CI

`cargo run -- .` against this repository exits zero with empty stdout, and CI now enforces that self-host loop on every push and pull request. The fenced-block skip in the scanner and this repo's slug-only `[id] format` keep the `e2e/cases/*` fixture trees and the illustrative IDs out of the host report. What is still missing is an explicit fixture for the fixture-tree case.

### 1. What

One remaining piece: an e2e fixture exercising a tree with nested fixture directories under a canonical *default* config (numbered IDs, non-strict) and asserting they do not pollute the outer report — the default `[scan] exclude` plus scan rules must keep nested case dirs out without relying on a particular `[id] format`.

### 2. Why now

Self-host is the load-bearing demonstration of [§G-no-dangling-refs](goals/goals.md#g-no-dangling-refs-every-cited-id-resolves-to-a-declaration) and [§G-fast-feedback](goals/goals.md#g-fast-feedback-gnd-must-be-as-fast-as-possible). The CI guard catches future regressions in this repo; the remaining fixture closes the gap for default-config fixture trees, where the pass should not lean on this repo's slug-only ID format.

### 3. Measurable

A new e2e fixture proves nested fixture directories are not scanned under the default config.

## RM-benchmarks: a benchmark harness for the §G-fast-feedback budgets

Per [§G-fast-feedback.1](goals/goals.md#1-performance-targets) and [§G-fast-feedback.3](goals/goals.md#3-measurable). The budgets are written down — under 100 ms on this repo, under 1 s on a 10k-file repo — but nothing measures them yet, so "CI fails on regression" is currently a promise without a meter. Capture a baseline against the current 0.1.0 single-binary build before [§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-gnd-core-from-gnd-cli) moves the engine into a library and [§RM-distribution](roadmap.md#rm-distribution-cargo--npm--pypi-from-one-engine) adds two more frontends.

### 1. What

A `cargo bench` harness (criterion) over `gnd check` on two inputs: this repo, and a generated large synthetic fixture — the "large conformant repo" fixture [§G-small-and-large.5](goals/goals.md#5-measurable) calls for, sized to fit the CI budget. The harness reports wall-clock per run; CI records both numbers per commit and fails when either crosses the [§G-fast-feedback.1](goals/goals.md#1-performance-targets) budget, which is what [§G-fast-feedback.3](goals/goals.md#3-measurable) ("CI tracks the number across commits and fails on regression") asks for. The 0.1.0 figures land in `docs/changelog.md` (or `docs/benchmarks.md`) as the recorded baseline. Allocation-count assertions ([§G-fast-feedback.1](goals/goals.md#1-performance-targets)'s "single allocation per file at most") are in scope if cheap to wire; otherwise they are a follow-up.

### 2. Why now

[§G-fast-feedback](goals/goals.md#g-fast-feedback-gnd-must-be-as-fast-as-possible) is one of the two ordering principles, and [§G-fast-feedback.1](goals/goals.md#1-performance-targets) says CI must fail on regression — but there is no harness, so the budget is unenforced. Establishing the baseline against today's code, before [§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-gnd-core-from-gnd-cli) splits the engine out and [§RM-distribution](roadmap.md#rm-distribution-cargo--npm--pypi-from-one-engine) wraps it in napi-rs and PyO3 bindings, means any slowdown those refactors introduce shows up as a diff against a known-good number rather than going unnoticed. It also shares the synthetic-large-repo fixture with [§RM-self-host](roadmap.md#rm-self-host-guard-the-self-host-loop-in-ci)'s remaining nested-fixture-tree case and with [§G-small-and-large](goals/goals.md#g-small-and-large-start-small-configure-for-big), so the generator is written once.

### 3. Measurable

`cargo bench` produces a stable per-run figure for `gnd check` on this repo and on the 10k-file synthetic fixture; CI records both, fails when either crosses the [§G-fast-feedback.1](goals/goals.md#1-performance-targets) budget, and the 0.1.0 baseline is committed alongside the harness.

## RM-core-cli-split: split gnd-core from gnd-cli

Workspace split before bindings ship. `src/lib.rs` is currently a single module that mixes scanner, checker, show, fmt, init, config parsing, argument handling, and rendering, with a thin `src/main.rs` calling into it.

### 1. What

`gnd-core` library crate: config loading, scanner, checker, `show` body extraction, `fmt` planning, `refs` filtering, report data structures. `gnd-cli` binary crate: argument parsing, rendering (text/JSON), exit-code mapping, help text. Today's `src/lib.rs` / `src/main.rs` are decomposed into these two crates with no behavior change.

### 2. Why now

[§RM-distribution](roadmap.md#rm-distribution-cargo--npm--pypi-from-one-engine) publishes three bindings from one engine; bindings need a library, not a binary. Splitting first also makes the e2e harness call into the engine directly and keeps CLI concerns (exit codes, rendering) from leaking into scanner internals.

### 3. Measurable

`gnd-core` compiles standalone; `gnd-cli`, the planned `gnd-node`, and `gnd-py` all consume it without duplicating scanner or checker code. The full e2e suite passes byte-identical reports before and after the split.

## RM-distribution-naming: verify package names before first publish

Pre-release sanity check: the registry names claimed across the docs may not still be available, and the future LSP slots have not been reserved. [§FS-distribution](functional-spec/FS-distribution.md#fs-distribution-gnd-distribution-targets) and [§DA-reference-checker-name](decisions/architectural/DA-reference-checker-name.md#da-reference-checker-name-name-for-the-spec-reference-checker-tool) must stay aligned before publishing plans harden.

### 1. What

The `scripts/check-registry-names.sh` pre-release guard and the manual **Release registry names** workflow query crates.io, npm, and PyPI for each claimed package name and fail if any claimed-available name is in fact taken or owned by another project. Docs are corrected so they no longer claim a name is free unless the project owns it. Where a registry name is unavailable, an explicit alternate package name is chosen and recorded in [§FS-distribution](functional-spec/FS-distribution.md#fs-distribution-gnd-distribution-targets).

### 2. Why now

A doc contradiction at release time is a release blocker. The check is cheap to run and cheaper to wire before the publish workflow exists than after.

### 3. Measurable

The release workflow queries each registry and proceeds only if every claimed name resolves to either "available" or "owned by this project." [§FS-distribution](functional-spec/FS-distribution.md#fs-distribution-gnd-distribution-targets) and [§DA-reference-checker-name](decisions/architectural/DA-reference-checker-name.md#da-reference-checker-name-name-for-the-spec-reference-checker-tool) agree on every package name they mention.

## RM-distribution: cargo + npm + pypi from one engine

Per [§FS-distribution](functional-spec/FS-distribution.md#fs-distribution-gnd-distribution-targets) and [§AS-bindings](architectural-spec/AS-bindings.md#as-bindings-target-shape-for-exposing-the-rust-engine-on-three-platforms). Builds on the workspace split ([§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-gnd-core-from-gnd-cli)) and the name verification ([§RM-distribution-naming](roadmap.md#rm-distribution-naming-verify-package-names-before-first-publish)).

### 1. What

napi-rs binding for npm; PyO3 binding for PyPI; CI publish jobs for all three registries (`gnd-core` first, in dependency order).

### 2. Why now

`gnd` is only viable as a CI dependency for non-Rust projects once it ships on their native package manager ([§G-multi-language](goals/goals.md#g-multi-language-same-engine-three-platforms)).

### 3. Measurable

Integration test runs the same spec corpus through all three bindings and asserts byte-identical reports ([§G-multi-language.3](goals/goals.md#3-measurable)).

## RM-lsp: ship the optional LSP server

Per [§FS-lsp](functional-spec/FS-lsp.md#fs-lsp-gnd-will-ship-an-optional-lsp-server), [§AS-lsp](architectural-spec/AS-lsp.md#as-lsp-how-the-lsp-server-is-built), and [§DA-lsp-optional](decisions/architectural/DA-lsp-optional.md#da-lsp-optional-lsp-server-ships-as-a-separate-optional-binary). Adds `crates/gnd-lsp/` to the workspace and publishes it as a separate package on cargo, npm, and PyPI. No first-party per-editor wrappers ship; editor configuration is the user's one-time work, with example snippets in the README.

### 1. What

A `gnd-lsp` binary that speaks LSP over stdio and serves the four capabilities pinned in [§FS-lsp.1](functional-spec/FS-lsp.md#1-capabilities): diagnostics, hover preview, go-to-definition, and the live `$$ → §` transform (the bulk form of which already ships in `gnd fmt`). Holds an in-memory `Findings` per workspace; full re-scan strategy on every change for v1 ([§AS-lsp.3.1](architectural-spec/AS-lsp.md#31-full-re-scan-on-every-change-v1)). Parity with the CLI is enforced by an e2e harness that drives the LSP through the same `e2e/cases/*` corpus and asserts byte-equivalent output ([§AS-lsp.5](architectural-spec/AS-lsp.md#5-determinism-and-parity-tests)).

Distribution: separate package on each registry ([§FS-distribution.1](functional-spec/FS-distribution.md#1-targets)). The CLI install does not pull in `gnd-lsp` transitively. README gains a section with example LSP-client snippets for Helix, Neovim, Zed, Emacs, VSCode (generic LSP client extension), and IntelliJ via LSP4IJ.

### 2. Why now

The reframed §raison-detre.2 keeps Markdown links peripheral and centers verify/refactor-safe/extract — three pillars all satisfied by CLI-shaped surfaces. Editor integration is then a UX layer over those, and the cheapest non-zero answer is one LSP server every editor can talk to. Shipping this after [§RM-distribution](roadmap.md#rm-distribution-cargo--npm--pypi-from-one-engine) (bindings) and [§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-gnd-core-from-gnd-cli) (workspace split) means the engine is already factored as a library and the registries are already wired.

### 3. Depends on

- [§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-gnd-core-from-gnd-cli) must land first; without `gnd-core` as a library, `gnd-lsp` has nothing to depend on.

### 4. Measurable

`gnd-lsp` installs from each registry. An editor pointed at the binary receives diagnostics, hover bodies, and definition jumps for any conformant repo, and parity tests assert byte-equivalence with `gnd check` and `gnd show` across the e2e corpus. Diagnostic latency on file change is within [§G-fast-feedback.1](goals/goals.md#1-performance-targets)'s per-scan budget.

## RM-watch: implement gnd check --watch

Per [§FS-check.6](functional-spec/FS-check.md#6-watch-mode---watch). The editor-less "every save" loop [§G-fast-feedback](goals/goals.md#g-fast-feedback-gnd-must-be-as-fast-as-possible) exists for — re-run `gnd check` on every change under the scanned tree, clearing prior output each run.

### 1. What

`--watch` on `gnd check` (and `gnd --watch` as shorthand): filesystem-notification-driven, debounced, no polling and no configurable interval. Each run is byte-identical to a plain `gnd check` on the tree's current state; on Ctrl-C the process exits with the last completed run's exit code. Non-interactive — no TUI, no key bindings ([§FS-non-goals.10](functional-spec/FS-non-goals.md#10-interactive-mode)), no network ([§FS-non-goals.11](functional-spec/FS-non-goals.md#11-network-access-during-a-check)).

### 2. Why now

`gnd-lsp` ([§RM-lsp](roadmap.md#rm-lsp-ship-the-optional-lsp-server)) covers editor users; `--watch` covers everyone else with zero editor configuration, and it is small once the engine is a library ([§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-gnd-core-from-gnd-cli)). Sequenced after [§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-gnd-core-from-gnd-cli) so the watcher calls `gnd-core::scan`/`check` rather than re-implementing the walk.

### 3. Measurable

An e2e fixture starts `gnd check --watch` on a clean fixture (asserts silent first run), writes a file that introduces a dangling ref (asserts the next run prints it), removes the bad citation (asserts the run goes silent again), then sends SIGINT (asserts exit code matches the last run). A second fixture asserts `--format=json` emits one self-contained report per run.

## RM-cochange-gate: a pre-commit / CI recipe — no impl change without spec and test

The strong form of the discipline ([§G-agent-grounding.1](goals/goals.md#1-the-three-layers), diff-gated enforcement): a changed source file must be grounded ([§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in)), and the change must also touch the spec it cites *or* a test of it, with an explicit escape hatch for refactors. This is diff-aware — a function of `(tree, base ref, config)`, not `(tree, config)` — and it leans on `gnd cover` ([§FS-cover](functional-spec/FS-cover.md#fs-cover-gnd-groups-citations-by-scanned-file)) plus a git diff, so it lives in the recipe layer, **not** in `gnd-core` (a third first-party surface is out of scope, [§FS-non-goals.12](functional-spec/FS-non-goals.md#12-surfaces-outside-gnd-core-and-the-lsp-transport); the engine reads no history, [§FS-non-goals.6](functional-spec/FS-non-goals.md#6-decision-database-audit-log-history-tracking)). Tiering rationale in [§DF-require-grounding](decisions/functional/DF-require-grounding.md#df-require-grounding-an-opt-in-check-that-every-source-file-cites-a-spec).

### 1. What

A documented pre-commit hook / CI step (a recipe alongside the `gnd check` hook in the README, and a worked example under `examples/`), not a shipped binary. Given a base ref it: (a) lists changed source files; (b) for each, gets its cited IDs from `gnd cover` and fails `ungrounded change` if a changed hunk falls under no citation; (c) requires the diff to also touch the declaring file of one of those IDs *or* a test / `§E2E-` case that cites one of them; (d) honours an escape hatch — a commit trailer (e.g. `Gnd-Cochange: refactor`) or a `gnd:no-cochange` pragma on a hunk — for legitimate refactors, kept greppable so a reviewer sees every waiver. Which paths count as "source" vs. "test", whether (c) needs spec *and* test or *either*, and how the base ref is chosen are knobs the repo sets in the recipe, not in `gnd-core` — so the "two installs agree" contract ([§FS-non-goals.13](functional-spec/FS-non-goals.md#13-anything-that-would-let-two-gnd-installs-disagree)) and the no-config-on-severity rule ([§FS-non-goals.9](functional-spec/FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization)) are untouched.

### 2. Why now

[§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in) makes "every file is grounded" true at rest; this makes "every change stays grounded, and ships with a test" true at the diff. It is unsound by construction — without an AST it cannot tell a behavioral hunk from a cosmetic one — so the escape hatch is mandatory and the gate is advisory-strict, not a proof. That trade is the reason it is a recipe a repo opts into, not engine behavior.

### 3. Measurable

The recipe, run in this repo's CI on a synthetic branch, fails a commit that edits a `src/` file without touching its spec or a test, passes the same commit once a `Gnd-Cochange:` trailer is added, and passes a commit that edits the spec and a test together. The `examples/` worked example carries golden output the e2e harness can diff.

## Shipped milestones

Done milestones leave their full record in `docs/changelog.md` (the `Implemented` block of the latest release). They keep a one-line declaration here so existing `§RM-…` citations still resolve — the changelog has the detail.

## RM-require-grounding: the opt-in grounding floor

Shipped. `[reference] require_grounding` (and `gnd check --require-grounding`), the `ungrounded source file` error class, the inline-declaration exemption, Markdown skipped — see `docs/changelog.md`, [§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in), [§FS-config.3.1](functional-spec/FS-config.md#31-reference--citation-form), and [§DF-require-grounding](decisions/functional/DF-require-grounding.md#df-require-grounding-an-opt-in-check-that-every-source-file-cites-a-spec). The diff-aware co-change recipe is [§RM-cochange-gate](roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test).

## RM-e2e-corpus: the e2e/cases/* corpus and CI harness

Shipped. The `e2e/cases/*` corpus, `tests/e2e.rs`, `tests/init.rs`, the per-error-class fixtures, and the byte-for-byte determinism sweep — see `docs/changelog.md`.

## RM-show: gnd show <ID>

Shipped. Whole declaration, `--head`, `--section` and dotted-inline section paths, `--full`, `--format text|md|json`, inline-source extraction, ambiguous-ID / broken-stub query forms — see `docs/changelog.md` and [§FS-show](functional-spec/FS-show.md#fs-show-gnd-reads-a-single-declaration-body-by-id).

## RM-config: .agents/gnd.toml discovery, parsing, and inspection

Shipped. The line-oriented TOML subset, unknown-key errors with `path:line:` pointers, `gnd_config_version` gating, every documented block, plus `gnd config validate` / `gnd config show` — see `docs/changelog.md` and [§FS-config](functional-spec/FS-config.md#fs-config-gnd-reads-a-toml-config-file-under-agents).

## RM-marker-fmt: the § marker, the $$ trigger, and gnd fmt

Shipped. `gnd fmt` with `--check` / `--write` / `--marker`, the deterministic string-literal carve-out, declaration-heading and fenced-block exemptions, and `[reference] strict = true` — see `docs/changelog.md`, [§FS-fmt](functional-spec/FS-fmt.md#fs-fmt-gnd-normalizes-references-in-bulk), and [§DF-reference-marker](decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger).

## RM-md-link-emission: gnd fmt --cross-refs

Shipped. Wraps marker-prefixed citations in `.md` files only, heading-text anchor slugs per the `github` / `gitlab` / `mkdocs` / `pandoc` / `none` profiles, re-derived each pass, fenced/inline-code-span and dangling-citation skips, idempotent — see `docs/changelog.md`, [§FS-fmt.6](functional-spec/FS-fmt.md#6-cross-reference-emission-with---cross-refs), and [§DF-md-link-anchor-strategy](decisions/functional/DF-md-link-anchor-strategy.md#df-md-link-anchor-strategy-heading-text-slugs-re-derived-on-every-fmt-pass).

## RM-refs: gnd refs <ID>

Shipped. Over the same scan as `check`, sorted by `(path, line, column)`, honouring strict mode and the string-literal carve-out, NDJSON on stdout for `--format=json` — see `docs/changelog.md` and [§FS-refs](functional-spec/FS-refs.md#fs-refs-gnd-lists-every-citation-of-an-id).

## RM-cover: gnd cover

Shipped. Groups the scanner's citation graph by file, emits text on stdout or one JSON record per scanned file, includes files with zero citations, and stays git/policy-free for the co-change recipe — see `docs/changelog.md` and [§FS-cover](functional-spec/FS-cover.md#fs-cover-gnd-groups-citations-by-scanned-file).

# Roadmap

What `grund` plans to ship next, in priority order. Each item has a stable ID — `RM-<slug>` under this repo's `[id] format` ([§FS-config.3.2](functional-spec/FS-config.md#32-id--id-grammar)); `RM` is a configured `[[kinds]]` prefix ([§FS-config.3.4](functional-spec/FS-config.md#34-kinds--recognized-prefixes)), so `grund check` validates `§RM-…` citations like any other. Items may be cited from anywhere — commits, PRs, the changelog, other specs. Shipped items move their detail to `docs/changelog.md` and keep a one-line pointer in §"Shipped milestones" below so the citation does not dangle; cancelled items stay in place with a `~~strikethrough~~` title and a one-line reason.

The check engine, the retrieval surface (`grund show`, `grund refs`, including E2E case manifests), the coverage index (`grund cover`), bulk normalization (`grund fmt`, including `--marker` and `--cross-refs`), config loading (`.agents/grund.toml` plus `grund config show` / `grund config validate`), `grund init`, `grund id`, the opt-in grounding floor ([§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in)), the token-cheap read surfaces ([§RM-token-cheap-grounding](roadmap.md#rm-token-cheap-grounding-token-cheap-read-surfaces-for-agents)), and the e2e corpus are all shipped — see `docs/changelog.md`. Two arcs remain. The **distribution arc**: split the single binary into a `grund-core` library plus thin frontends, verify the package names, publish on npm and PyPI alongside cargo, ship the optional LSP server, and add `grund check --watch`. And the **grounding arc** (the third layer of [§GOAL-agent-grounding.1](goals.md#1-the-three-layers), diff-gated enforcement): build on [§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in) and [§FS-cover](functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file) toward a diff-aware co-change gate — implementation cannot change without the spec it grounds in and without a test of it — via a pre-commit / CI recipe that consumes `grund cover` ([§RM-cochange-gate](roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test)). Four standalone items sit outside both arcs: [§RM-benchmarks](roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-goal-fast-feedback-budgets) captures a performance baseline against today's single-binary build before the distribution arc starts moving the engine around, [§RM-declaration-near-miss](roadmap.md#rm-declaration-near-miss-warn-on-a-heading-that-looks-like-a-declaration-but-does-not-match-id-format) adds a warning for a heading that looks like a declaration but does not match the configured `[id] format`, [§RM-init-workspace-members](roadmap.md#rm-init-workspace-members-init-mentions-workspace-members) extends `grund init`'s managed block to mention sibling project namespaces when the effective config declares a `[workspace]`, and [§RM-positioning](roadmap.md#rm-positioning-the-lychee-contrast-and-the-instruction-count-framing-in-readme-and-landing-copy) keeps the README/landing pitch paired with the benchmark story. The IDed milestones below project both arcs onto reviewable units of work.

## RM-self-host: guard the self-host loop in CI

`cargo run -- .` against this repository exits zero with the clean text `success` marker, and CI now enforces that self-host loop on every push and pull request. The fenced-block skip in the scanner and this repo's slug-only `[id] format` keep the `e2e/cases/*` fixture trees and the illustrative IDs out of the host report. What is still missing is an explicit fixture for the fixture-tree case.

### 1. What

One remaining piece: an e2e fixture exercising a tree with nested fixture directories under a canonical *default* config (numbered IDs, non-strict) and asserting they do not pollute the outer report — the default `[scan] exclude` plus scan rules must keep nested case dirs out without relying on a particular `[id] format`.

### 2. Why now

Self-host is the load-bearing demonstration of [§GOAL-no-dangling-refs](goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) and [§GOAL-fast-feedback](goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible). The CI guard catches future regressions in this repo; the remaining fixture closes the gap for default-config fixture trees, where the pass should not lean on this repo's slug-only ID format.

### 3. Measurable

A new e2e fixture proves nested fixture directories are not scanned under the default config.

## RM-benchmarks: a benchmark harness for the §GOAL-fast-feedback budgets

Per [§GOAL-fast-feedback.1](goals.md#1-performance-targets) and [§GOAL-fast-feedback.3](goals.md#3-measurable). The budgets are written down — under 100 ms on this repo, under 1 s on a 10k-file repo. The instruction-counting `cargo bench` harness over the hot commands on this repo is in place ([§AR-benchmarks](architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands), decision in [§DA-benchmark-instruction-counting](decisions/architectural/DA-benchmark-instruction-counting.md#da-benchmark-instruction-counting-the-performance-harness-counts-instructions-not-wall-clock-seconds)) and CI runs it ([§AR-ci.5](architecture/AR-ci.md#5-benchmark-job)), so the per-commit number now exists; what is still missing is the 10k-file input, the committed baseline, and the build-failing threshold — so "CI fails on regression" is still a promise. Land the rest against the current 0.1.0 single-binary build before [§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-grund-core-from-grund-cli) moves the engine into a library and [§RM-distribution](roadmap.md#rm-distribution-cargo--npm--pypi-from-one-engine) adds two more frontends.

### 1. What

Done: a `cargo bench` harness at `benches/instructions.rs` (gated behind a `bench` Cargo feature) that runs the built `grund` binary under Callgrind for the commands agents and CI run most — `check`, `list`, `show`, `refs`, `cover`, `fmt --check` — over this repo, reporting a deterministic instruction count per invocation; and a CI `bench` job that installs Valgrind + `iai-callgrind-runner` and runs it on every push ([§AR-benchmarks](architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands), [§AR-ci.5](architecture/AR-ci.md#5-benchmark-job)). Instruction count rather than wall-clock so the figure does not flake on a loaded runner — [§DA-benchmark-instruction-counting](decisions/architectural/DA-benchmark-instruction-counting.md#da-benchmark-instruction-counting-the-performance-harness-counts-instructions-not-wall-clock-seconds).

Remaining: a generated large synthetic fixture — the "large conformant repo" fixture [§GOAL-small-and-large.5](goals.md#5-measurable) calls for, sized to fit the CI budget — added as a second input to the harness; the 0.1.0 instruction-count figures (and the recorded wall-clock ms headline) committed to `docs/changelog.md` (or `docs/benchmarks.md`) as the baseline; and the CI job comparing against that baseline and failing the build when a count crosses a regression threshold, which is what [§GOAL-fast-feedback.3](goals.md#3-measurable) ("CI tracks the number across commits and fails on regression") asks for. Allocation-count assertions ([§GOAL-fast-feedback.1](goals.md#1-performance-targets)'s "single allocation per file at most") are in scope if cheap to wire; otherwise they are a follow-up.

### 2. Why now

[§GOAL-fast-feedback](goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) is one of the two ordering principles, and [§GOAL-fast-feedback.1](goals.md#1-performance-targets) says CI must fail on regression — but there is no harness, so the budget is unenforced. Establishing the baseline against today's code, before [§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-grund-core-from-grund-cli) splits the engine out and [§RM-distribution](roadmap.md#rm-distribution-cargo--npm--pypi-from-one-engine) wraps it in napi-rs and PyO3 bindings, means any slowdown those refactors introduce shows up as a diff against a known-good number rather than going unnoticed. It also shares the synthetic-large-repo fixture with [§RM-self-host](roadmap.md#rm-self-host-guard-the-self-host-loop-in-ci)'s remaining nested-fixture-tree case and with [§GOAL-small-and-large](goals.md#goal-small-and-large-start-small-configure-for-big), so the generator is written once.

### 3. Measurable

`cargo bench` produces a stable per-run figure for `grund check` on this repo and on the 10k-file synthetic fixture; CI records both, fails when either crosses the [§GOAL-fast-feedback.1](goals.md#1-performance-targets) budget, and the 0.1.0 baseline is committed alongside the harness.

## RM-core-cli-split: split grund-core from grund-cli

Workspace split before bindings ship. The first boundary is in place: `crates/grund-core` is a real workspace crate and the root `grund` binary depends on it. The remaining cleanup is to move CLI-only argument parsing, rendering, and exit-code mapping out of `grund-core` into a dedicated `grund-cli` frontend crate.

### 1. What

Done: `grund-core` library crate exists and owns the current scanner/checker/show/fmt/config implementation. The published root `grund` package is a thin binary that calls into it. Remaining: `grund-cli` frontend crate for argument parsing, rendering (text/JSON), exit-code mapping, and help text, leaving `grund-core` as the engine-only library.

### 2. Why now

[§RM-distribution](roadmap.md#rm-distribution-cargo--npm--pypi-from-one-engine) publishes three bindings from one engine; bindings need a library, not a binary. Splitting first also makes the e2e harness call into the engine directly and keeps CLI concerns (exit codes, rendering) from leaking into scanner internals.

### 3. Measurable

`grund-core` compiles standalone; `grund-cli`, the planned `grund-node`, and `grund-py` all consume it without duplicating scanner or checker code. The full e2e suite passes byte-identical reports before and after the split.

## RM-distribution-naming: verify package names before first publish

Pre-release sanity check: the registry names claimed across the docs may not still be available, and the future LSP slots have not been reserved. [§FS-distribution](functional-spec/FS-distribution.md#fs-distribution-grund-distribution-targets) and [§DA-reference-checker-name](decisions/architectural/DA-reference-checker-name.md#da-reference-checker-name-name-for-the-spec-reference-checker-tool) must stay aligned before publishing plans harden.

### 1. What

The `scripts/check-registry-names.sh` pre-release guard and the manual **Pre-release checks** workflow query crates.io, npm, and PyPI for each claimed package name and fail if any claimed-available name is in fact taken or owned by another project. The same workflow also runs the PGO release-binary build pinned by [§DA-pgo-release](decisions/architectural/DA-pgo-release.md#da-pgo-release-distributed-binaries-are-pgo-built-trained-on-the-benchmark-workload), so package-name drift and a broken optimized release build both block publish. Docs are corrected so they no longer claim a name is free unless the project owns it. Where a registry name is unavailable, an explicit alternate package name is chosen and recorded in [§FS-distribution](functional-spec/FS-distribution.md#fs-distribution-grund-distribution-targets).

### 2. Why now

A doc contradiction at release time is a release blocker. The check is cheap to run and cheaper to wire before the publish workflow exists than after.

### 3. Measurable

The release workflow queries each registry and proceeds only if every claimed name resolves to either "available" or "owned by this project." [§FS-distribution](functional-spec/FS-distribution.md#fs-distribution-grund-distribution-targets) and [§DA-reference-checker-name](decisions/architectural/DA-reference-checker-name.md#da-reference-checker-name-name-for-the-spec-reference-checker-tool) agree on every package name they mention.

## RM-distribution: cargo + npm + pypi from one engine

Per [§FS-distribution](functional-spec/FS-distribution.md#fs-distribution-grund-distribution-targets) and [§AR-bindings](architecture/AR-bindings.md#ar-bindings-target-shape-for-exposing-the-rust-engine-on-three-platforms). Builds on the workspace split ([§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-grund-core-from-grund-cli)) and the name verification ([§RM-distribution-naming](roadmap.md#rm-distribution-naming-verify-package-names-before-first-publish)).

### 1. What

napi-rs binding for npm; PyO3 binding for PyPI; CI publish jobs for all three registries (`grund-core` first, in dependency order). Each publish job builds the CLI binary with profile-guided optimization via `scripts/pgo-build.sh` ([§DA-pgo-release](decisions/architectural/DA-pgo-release.md#da-pgo-release-distributed-binaries-are-pgo-built-trained-on-the-benchmark-workload), [§FS-distribution.4](functional-spec/FS-distribution.md#4-release-process)) — wired for the crates.io `grund` binary already, extended to the prebuilt npm and PyPI binaries here.

### 2. Why now

`grund` is only viable as a CI dependency for non-Rust projects once it ships on their native package manager ([§GOAL-multi-language](goals.md#goal-multi-language-same-engine-three-platforms)).

### 3. Measurable

Integration test runs the same spec corpus through all three bindings and asserts byte-identical reports ([§GOAL-multi-language.3](goals.md#3-measurable)).

## RM-lsp: ship the optional LSP server

Per [§FS-lsp](functional-spec/FS-lsp.md#fs-lsp-grund-will-ship-an-optional-lsp-server), [§AR-lsp](architecture/AR-lsp.md#ar-lsp-how-the-lsp-server-is-built), and [§DA-lsp-optional](decisions/architectural/DA-lsp-optional.md#da-lsp-optional-lsp-server-ships-as-a-separate-optional-binary). Adds `crates/grund-lsp/` to the workspace and publishes it as a separate package on cargo, npm, and PyPI. No first-party per-editor wrappers ship; editor configuration is the user's one-time work, with example snippets in the README.

### 1. What

A `grund-lsp` binary that speaks LSP over stdio and serves the four capabilities pinned in [§FS-lsp.1](functional-spec/FS-lsp.md#1-capabilities): diagnostics, hover preview, go-to-definition, and the live `$$ → §` transform (the bulk form of which already ships in `grund fmt`). Holds an in-memory `Findings` per workspace; full re-scan strategy on every change for v1 ([§AR-lsp.3.1](architecture/AR-lsp.md#31-full-re-scan-on-every-change-v1)). Parity with the CLI is enforced by an e2e harness that drives the LSP through the same `e2e/cases/*` corpus and asserts byte-equivalent output ([§AR-lsp.5](architecture/AR-lsp.md#5-determinism-and-parity-tests)).

Distribution: separate package on each registry ([§FS-distribution.1](functional-spec/FS-distribution.md#1-targets)). The CLI install does not pull in `grund-lsp` transitively. README gains a section with example LSP-client snippets for Helix, Neovim, Zed, Emacs, VSCode (generic LSP client extension), and IntelliJ via LSP4IJ.

### 2. Why now

[§GND-grund.1](grund.md#1-what-grund-does-about-it) keeps Markdown links peripheral and centers verify/refactor-safe/extract — three pillars all satisfied by CLI-shaped surfaces. Editor integration is then a UX layer over those, and the cheapest non-zero answer is one LSP server every editor can talk to. Shipping this after [§RM-distribution](roadmap.md#rm-distribution-cargo--npm--pypi-from-one-engine) (bindings) and [§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-grund-core-from-grund-cli) (workspace split) means the engine is already factored as a library and the registries are already wired.

### 3. Depends on

- [§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-grund-core-from-grund-cli) must land first; without `grund-core` as a library, `grund-lsp` has nothing to depend on.

### 4. Measurable

`grund-lsp` installs from each registry. An editor pointed at the binary receives diagnostics, hover bodies, and definition jumps for any conformant repo, and parity tests assert byte-equivalence with `grund check` and `grund show` across the e2e corpus. Diagnostic latency on file change is within [§GOAL-fast-feedback.1](goals.md#1-performance-targets)'s per-scan budget.

## RM-watch: implement grund check --watch

Per [§FS-check.6](functional-spec/FS-check.md#6-watch-mode---watch). The editor-less "every save" loop [§GOAL-fast-feedback](goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) exists for — re-run `grund check` on every change under the scanned tree, clearing prior output each run.

### 1. What

`--watch` on `grund check` (and `grund --watch` as shorthand): filesystem-notification-driven, debounced, no polling and no configurable interval. Each run is byte-identical to a plain `grund check` on the tree's current state; on Ctrl-C the process exits with the last completed run's exit code. Non-interactive — no TUI, no key bindings ([§FS-non-goals.10](functional-spec/FS-non-goals.md#10-interactive-mode)), no network ([§FS-non-goals.11](functional-spec/FS-non-goals.md#11-network-access-during-a-check)).

### 2. Why now

`grund-lsp` ([§RM-lsp](roadmap.md#rm-lsp-ship-the-optional-lsp-server)) covers editor users; `--watch` covers everyone else with zero editor configuration, and it is small once the engine is a library ([§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-grund-core-from-grund-cli)). Sequenced after [§RM-core-cli-split](roadmap.md#rm-core-cli-split-split-grund-core-from-grund-cli) so the watcher calls `grund-core::scan`/`check` rather than re-implementing the walk.

### 3. Measurable

An e2e fixture starts `grund check --watch` on a clean fixture (asserts silent first run), writes a file that introduces a dangling ref (asserts the next run prints it), removes the bad citation (asserts the run goes silent again), then sends SIGINT (asserts exit code matches the last run). A second fixture asserts `--format=json` emits one self-contained report per run.

## RM-cochange-gate: a pre-commit / CI recipe — no impl change without spec and test

The strong form of the discipline ([§GOAL-agent-grounding.1](goals.md#1-the-three-layers), diff-gated enforcement): a changed source file must be grounded ([§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in)), and the change must also touch the spec it cites *or* a test of it, with an explicit escape hatch for refactors. This is diff-aware — a function of `(tree, base ref, config)`, not `(tree, config)` — and it leans on `grund cover` ([§FS-cover](functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file)) plus a git diff, so it lives in the recipe layer, **not** in `grund-core` (a third first-party surface is out of scope, [§FS-non-goals.12](functional-spec/FS-non-goals.md#12-surfaces-outside-grund-core-and-the-lsp-transport); the engine reads no history, [§FS-non-goals.6](functional-spec/FS-non-goals.md#6-decision-database-audit-log-history-tracking)). Tiering rationale in [§DF-require-grounding](decisions/functional/DF-require-grounding.md#df-require-grounding-an-opt-in-check-that-every-source-file-cites-a-spec).

### 1. What

A documented pre-commit hook / CI step (a recipe alongside the `grund check` hook in the README, and a worked example under `examples/`), not a shipped binary. Given a base ref it: (a) lists changed source files; (b) for each, gets its cited IDs from `grund cover` and fails `ungrounded change` if a changed hunk falls under no citation; (c) requires the diff to also touch the declaring file of one of those IDs *or* a test / `§E2E-` case that cites one of them; (d) honours an escape hatch — a commit trailer (e.g. `Grund-Cochange: refactor`) or a `grund:no-cochange` pragma on a hunk — for legitimate refactors, kept greppable so a reviewer sees every waiver. Which paths count as "source" vs. "test", whether (c) needs spec *and* test or *either*, and how the base ref is chosen are knobs the repo sets in the recipe, not in `grund-core` — so the "two installs agree" contract ([§FS-non-goals.13](functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)) and the no-config-on-severity rule ([§FS-non-goals.9](functional-spec/FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization)) are untouched.

### 2. Why now

[§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in) makes "every file is grounded" true at rest; this makes "every change stays grounded, and ships with a test" true at the diff. It is unsound by construction — without an AST it cannot tell a behavioral hunk from a cosmetic one — so the escape hatch is mandatory and the gate is advisory-strict, not a proof. That trade is the reason it is a recipe a repo opts into, not engine behavior.

### 3. Measurable

The recipe, run in this repo's CI on a synthetic branch, fails a commit that edits a `src/` file without touching its spec or a test, passes the same commit once a `Grund-Cochange:` trailer is added, and passes a commit that edits the spec and a test together. The `examples/` worked example carries golden output the e2e harness can diff.

## RM-declaration-near-miss: warn on a heading that looks like a declaration but does not match `[id] format`

A non-heuristic onboarding aid: a heading shaped like `# <KIND>-…: <title>` whose ID does not match the configured `[id] format` ([§FS-config.3.2](functional-spec/FS-config.md#32-id--id-grammar)) is silently *not* a declaration today — it does not appear in `grund list`, `grund check` says nothing, and `§…` citations of that shape go unrecognised. The classic stumble is writing `# FS-login: …` under the default `{kind}-{number}-{slug}` (forgetting the `-NNN-`). Serves [§GOAL-zero-config](goals.md#goal-zero-config-works-on-any-conformant-tree) and [§GOAL-friendliness-first](goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) — a fresh adopter should not have to discover the grammar by its silence.

### 1. What

`grund check` emits one **warning** (not an error — exit code unchanged, [§FS-check.4](functional-spec/FS-check.md#4-warnings)) at `path:line:` for a heading whose first token after `# ` is `<KIND>-…` for a configured `[[kinds]]` prefix but whose remainder fails the `[id] format` regex — message names the heading, the configured format, and the nearest valid shape. This is a fact about the tree, not a guess at intent: it does not propose the corrected ID ([§FS-non-goals.3](functional-spec/FS-non-goals.md#3-code-ast-parsing) / no heuristics, [§GOAL-agent-grounding.3](goals.md#3-what-this-rules-out)), it only points out that something heading-shaped is being ignored. A line-oriented opt-out (a `grund:not-a-declaration` pragma, or a config glob) for files that legitimately use `# <KIND>-…:` headings as prose is in scope if the warning proves noisy.

### 2. Why now

Caught in the 0.1.0 product review as a real sharp edge: the strict-grammar design is correct, but its failure mode is invisible. The warning is the smallest fix that stays inside the no-guessing rule — it surfaces the mismatch and lets the contributor decide.

### 3. Measurable

An e2e fixture with a numbered-format config and a `# FS-login: …` heading gets exactly one warning naming the heading and the format, `grund check` still exits `0`, and a sibling fixture whose heading *does* match the format gets none. `grund list` is unchanged in both (a near-miss heading is still not a declaration).

## RM-init-workspace-members: `init` mentions workspace members

`grund init` is workspace-blind today. When the effective `.agents/grund.toml` declares `[workspace] members = [...]` ([§FS-workspace.2](functional-spec/FS-workspace.md#2-workspace-configuration)), the generated `AGENTS.md` says nothing about sibling project namespaces, so an agent landing at the root cannot tell that `§api/FS-foo`-shaped citations exist without scanning the toml itself. Discussion concluded in [§DISC-init-workspace-members](discussions/proposals/2026-05-17-init-workspace-members.md#disc-init-workspace-members-have-init-mention-workspace-members) — *mention*, do not *configure*: surface the resolved members in the existing managed block without prompts or inference. Serves [§GOAL-friendliness-first](goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) by making the cross-project citation scope visible at the entrypoint an agent reads first.

### 1. What

Extend the managed block written by `grund init` ([§FS-init.2.3.4.4](functional-spec/FS-init.md#2344-project-map)) with a sibling `### Workspace members` section that renders whenever the effective config has a `[workspace]` table. The section emits one discoverability line — `Cross-project citations use §alias/<ID>.` — and one bullet per resolved member, sorted by alias, with the member root joined to `AGENTS.md` when the file exists or marked `*(not yet initialized)*` and pointing at the member root when it does not. Globs in `members = […]` expand the same way `grund check` already expands them ([§FS-workspace.2](functional-spec/FS-workspace.md#2-workspace-configuration)); aliases follow [§FS-workspace.3](functional-spec/FS-workspace.md#3-aliases). The same list is emitted whether `init` runs at the workspace root or inside a member whose effective config inherits `[workspace]` — explicit in the spec so the symmetry is not a surprise. `init` still does not prompt, does not infer workspace topology, does not add a `[workspace]` block to a config that lacks one, and does not write under any member directory. Out of scope for v1: a reciprocal member → root pointer, and any collapse-to-alias-only behavior for very large workspaces.

### 2. Why now

Preserves the no-prompts, no-surprises contract of [§FS-init](functional-spec/FS-init.md#fs-init-grund-bootstraps-a-new-grund-conformant-repo) while closing a real visibility gap exposed by [§FS-workspace](functional-spec/FS-workspace.md#fs-workspace-grund-validates-cross-project-citations-in-a-workspace): the workspace namespace exists in config but is invisible to an agent reading the generated entrypoint. The change is small and additive — no Project Map format change, no member-directory writes — and lands cleanly without waiting on the distribution or grounding arcs. Pairs with the recent workspace-citation work on this branch by closing the discoverability side of the same loop.

### 3. Measurable

An e2e fixture with a `[workspace]` root and at least one initialized and one uninitialized member: `grund init` at the root emits a `### Workspace members` block with the syntax line, members sorted by alias, the initialized member linked to `…/AGENTS.md`, and the uninitialized member marked `*(not yet initialized)*` pointing at its member root. A second fixture runs `grund init` inside a member and asserts byte-identical output for that section. A third fixture asserts a non-workspace config produces no `### Workspace members` section and the existing Project Map is unchanged.

## RM-positioning: the Lychee contrast and the instruction-count framing in README and landing copy

`grund` lives next to `lychee` in CI, not against it ([§FS-non-goals.1](functional-spec/FS-non-goals.md#1-markdown-link-validation), [§AR-ci.3](architecture/AR-ci.md#3-current-hooks)). The README now says that in product terms — "Lychee is the link checker; `grund` is the intent checker" — and carries the instruction-count-not-stopwatch framing beside the benchmark badge. What remains is pairing that framing with the committed instruction-count baseline once [§RM-benchmarks](roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-goal-fast-feedback-budgets) lands it. This milestone ships words, not code.

### 1. What

Done: a short "vs. a link checker" block in the README:

- Lychee checks whether Markdown links still open; `grund` checks whether your code still knows why it exists.
- Lychee catches dead links; `grund` catches dead grounding.
- Lychee validates the web of pages; `grund` validates the web of intent.
- Lychee says "this URL broke"; `grund` says "this implementation lost its spec."
- Use Lychee for links out; use `grund` for reasons in.
- Lychee guards navigation; `grund` guards meaning.

…landing on the closing line: **Lychee is the link checker; `grund` is the intent checker. Both belong in CI; they guard different failure modes.**

Partially done: the README states the benchmark framing next to the local throughput badge. Once [§RM-benchmarks](roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-goal-fast-feedback-budgets) commits the baseline figure, place the explicit number beside this line: **`grund` measures performance by instruction count, not stopwatch time — same binary, same repo, same number — which gives CI a stable regression meter instead of a noisy timing guess** ([§DA-benchmark-instruction-counting](decisions/architectural/DA-benchmark-instruction-counting.md#da-benchmark-instruction-counting-the-performance-harness-counts-instructions-not-wall-clock-seconds), [§GOAL-fast-feedback.3](goals.md#3-measurable)).

### 2. Why now

The 0.1.0 product review found the README explained the *mechanism* well and the *pitch* thinly: a reader who already runs `lychee` could not tell in one line what `grund` adds beside it ([§GND-grund.1](grund.md#1-what-grund-does-about-it)). The framing is cheap to write and pays off on every landing. It pairs with [§RM-benchmarks](roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-goal-fast-feedback-budgets) because the instruction-count line earns its full place once there is a committed figure to attach it to.

### 3. Measurable

The README (and landing page, if any) carries a "vs. link checkers" block whose closing line is the "link checker / intent checker" pair. When [§RM-benchmarks](roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-goal-fast-feedback-budgets) commits the 0.1.0 baseline, the benchmark section states the instruction-count-not-wall-clock framing alongside that number. `grund check` stays clean.

## Shipped milestones

Done milestones leave their full record in `docs/changelog.md` (the `Implemented` block of the latest release). They keep a one-line declaration here so existing `§RM-…` citations still resolve — the changelog has the detail.

## RM-require-grounding: the opt-in grounding floor

Shipped. `[reference] require_grounding` (and `grund check --require-grounding`), the `ungrounded source file` error class, the inline-declaration exemption, Markdown skipped — see `docs/changelog.md`, [§FS-check.3.6](functional-spec/FS-check.md#36-ungrounded-source-file-opt-in), [§FS-config.3.1](functional-spec/FS-config.md#31-reference--citation-form), and [§DF-require-grounding](decisions/functional/DF-require-grounding.md#df-require-grounding-an-opt-in-check-that-every-source-file-cites-a-spec). The diff-aware co-change recipe is [§RM-cochange-gate](roadmap.md#rm-cochange-gate-a-pre-commit--ci-recipe--no-impl-change-without-spec-and-test).

## RM-e2e-corpus: the e2e/cases/* corpus and CI harness

Shipped. The `e2e/cases/*` corpus, `tests/e2e.rs`, `tests/init.rs`, the per-error-class fixtures, and the byte-for-byte determinism sweep — see `docs/changelog.md`.

## RM-show: grund show <ID>

Shipped. Lead-default declaration reads, `--brief`, `--toc`, `--section` and dotted-inline section paths, `--full`, `--format text|md|json`, inline-source extraction, ambiguous-ID / broken-stub query forms — see `docs/changelog.md` and [§FS-show](functional-spec/FS-show.md#fs-show-grund-reads-a-single-declaration-body-by-id).

## RM-token-cheap-grounding: token-cheap read surfaces for agents

Shipped. The lead-default `grund show <ID>` read, `grund show --brief`, `grund show --toc`, `grund refs --summary`, multi-kind `grund list --kind FS,AR`, `grund list --summary`, and the generated `AGENTS.md` guidance block — see `docs/changelog.md`, [§FS-show.2.1](functional-spec/FS-show.md#21-whole-declaration-default), [§FS-show.2.1.1](functional-spec/FS-show.md#211-brief---brief), [§FS-show.2.1.2](functional-spec/FS-show.md#212-section-map---toc), [§FS-refs.3.3](functional-spec/FS-refs.md#33---summary), [§FS-list.3.3](functional-spec/FS-list.md#33---summary), and [§DF-show-default-token-cheap](decisions/functional/DF-show-default-token-cheap.md#df-show-default-token-cheap-grund-show-defaults-to-the-cheap-read-the-full-body-is-opt-in).

## RM-config: .agents/grund.toml discovery, parsing, and inspection

Shipped. The line-oriented TOML subset, unknown-key errors with `path:line:` pointers, `grund_config_version` gating, every documented block, plus `grund config validate` / `grund config show` — see `docs/changelog.md` and [§FS-config](functional-spec/FS-config.md#fs-config-grund-reads-a-toml-config-file-under-agents).

## RM-marker-fmt: the § marker, the $$ trigger, and grund fmt

Shipped. `grund fmt` with `--check` / `--write` / `--marker`, the deterministic string-literal carve-out, declaration-heading and fenced-block exemptions, and `[reference] strict = true` — see `docs/changelog.md`, [§FS-fmt](functional-spec/FS-fmt.md#fs-fmt-grund-normalizes-references-in-bulk), and [§DF-reference-marker](decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger).

## RM-md-link-emission: grund fmt --cross-refs

Shipped. Wraps marker-prefixed citations in `.md` files only, heading-text anchor slugs per the `github` / `gitlab` / `mkdocs` / `pandoc` / `none` profiles, re-derived each pass, fenced/inline-code-span and dangling-citation skips, idempotent — see `docs/changelog.md`, [§FS-fmt.6](functional-spec/FS-fmt.md#6-cross-reference-emission-with---cross-refs), and [§DF-md-link-anchor-strategy](decisions/functional/DF-md-link-anchor-strategy.md#df-md-link-anchor-strategy-heading-text-slugs-re-derived-on-every-fmt-pass).

## RM-refs: grund refs <ID>

Shipped. Over the same scan as `check`, sorted by `(path, line, column)`, honouring strict mode and the string-literal carve-out, NDJSON on stdout for `--format=json` — see `docs/changelog.md` and [§FS-refs](functional-spec/FS-refs.md#fs-refs-grund-lists-every-citation-of-an-id).

## RM-cover: grund cover

Shipped. Groups the scanner's citation graph by file, emits text on stdout or one JSON record per scanned file, includes files with zero citations, and stays git/policy-free for the co-change recipe — see `docs/changelog.md` and [§FS-cover](functional-spec/FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file).

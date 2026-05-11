# Goals

What `grund` measures itself against. If a change does not advance one of these, it is not worth doing. Goals are declared inline below so a human can read the whole picture top-to-bottom; each declaration is a stable ID and may be cited from anywhere in the repo.

Current goals:

- [§GOAL-agent-grounding](goals.md#goal-agent-grounding-agents-stay-cited-as-they-work)
- [§GOAL-no-dangling-refs](goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration)
- [§GOAL-polyglot-citation](goals.md#goal-polyglot-citation-ids-cite-cleanly-from-anywhere-they-are-useful)
- [§GOAL-fast-feedback](goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible)
- [§GOAL-zero-config](goals.md#goal-zero-config-works-on-any-conformant-tree)
- [§GOAL-multi-language](goals.md#goal-multi-language-same-engine-three-platforms)
- [§GOAL-friendliness-first](goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible)
- [§GOAL-configurable](goals.md#goal-configurable-every-default-is-overridable)
- [§GOAL-no-silent-breakage](goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path)
- [§GOAL-small-and-large](goals.md#goal-small-and-large-start-small-configure-for-big)

## GOAL-agent-grounding: agents stay cited as they work

The point of citations is to keep specs, decisions, and code coupled while the project evolves. That coupling only holds if every contributor — human or AI — leaves the tree cited *as they go*, not in a retro-fit pass at the end. `grund` must make grounded work the path of least resistance: an agent should learn the rules without reading source, feel the rules while editing, and be stopped by the rules before a bad diff lands.

This is the **headline** goal — the one [§GND-grund](../grund.md#gnd-grund-agents-stay-grounded-in-the-spec) cashes out directly; every other goal in this file exists in service of it. [§GOAL-no-dangling-refs](goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) guarantees the resolver is right; [§GOAL-fast-feedback](goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) keeps it cheap enough to run in the agent loop; [§GOAL-friendliness-first](goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) shapes the output so an agent can act on it; [§GOAL-polyglot-citation](goals.md#goal-polyglot-citation-ids-cite-cleanly-from-anywhere-they-are-useful) lets a citation live wherever the agent edits. Grounding is the *outcome*; the rest is mechanism.

### 1. The three layers

Grounding is enforced at three escalating layers; each catches what the one above it lets through.

- **Instruction.** `grund init` writes a managed block into `AGENTS.md` (and the language-specific aliases — `CLAUDE.md`, etc.) that names the citation grammar, the `grund show` / `grund refs` workflow, the rule that an agent re-reads cited specs (via `grund show <ID>`) before editing the code that realizes them, and the rule that every new behavior carries an ID. An agent that reads its entry-point file at session start arrives already taught — the "faster onboarding, cheaper LLM context" promise of [§GND-grund](../grund.md#gnd-grund-agents-stay-grounded-in-the-spec), paid in a few hundred tokens at session open instead of a discovery walk through every spec. Per [§GOAL-no-silent-breakage](goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path), the block is versioned and refreshed by `grund init --force`.
- **Verification at rest.** `grund check` over the whole tree is the steady-state guarantee — every cited ID resolves, every declaration is reachable, nothing dangles. This is the property [§GOAL-no-dangling-refs](goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) already locks in; this goal commits to keeping it cheap enough ([§GOAL-fast-feedback](goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible)) that an agent can run it between edits, not just in CI.
- **Diff-gated enforcement.** A `--since <ref>` mode (or equivalent) reports only what *changed* in the working tree relative to a base — new declarations missing from the index, new code without a citation to the spec it realizes, new specs without an e2e test under [§GOAL-no-dangling-refs](goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration)'s contract. This is the layer that closes the agent loop: a coding agent runs it before claiming a task is done and gets a punch list back, in the same shape as `grund check`'s normal output.

### 2. What "grounded" requires of a diff

A change is grounded when:

- Every new heading of the form `# <KIND>-<slug>: …` is reachable by a walk from the repo root under the configured scan scope.
- Every new code unit (function, class, module) that realizes a named behavior carries a `§<ID>` citation on its doc-comment or an inline citation beside the clause that enforces it.
- Every new decision record under `docs/decisions/` is cited from the spec or architecture doc whose shape it changed (per the existing "no dangling decisions" rule in `CLAUDE.md`).
- Every new e2e case carries `spec.refs` naming the FS IDs it proves (per the existing e2e convention).

The diff-gated mode reports the absences; it does not invent citations.

### 3. What this rules out

- Heuristic "you probably meant to cite X" suggestions in `grund check`. The tool reports facts about the tree; choosing the right `<ID>` is the contributor's call. (Composition with [§GOAL-friendliness-first.2](goals.md#2-what-this-rules-out): no severity knobs, no auto-fix, no guessing.)
- A separate "lint" command parallel to `check`. Diff-gated reporting is a *mode* of `check`, sharing one resolver, one output schema, one exit-code mapping ([§GOAL-no-silent-breakage.1](goals.md#1-what-counts-as-user-visible)).
- Hard-coding what "a code unit that realizes a behavior" means. The detector is a configurable scan rule ([§GOAL-configurable](goals.md#goal-configurable-every-default-is-overridable)), not a built-in heuristic that two repos cannot agree on.

### 4. Composition with other goals

- [§GOAL-no-dangling-refs](goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) is the *correctness* contract for citations at rest; [§GOAL-agent-grounding](goals.md#goal-agent-grounding-agents-stay-cited-as-they-work) is the *adoption* contract for citations as work proceeds. Together they say: the tree is always cited, and stays cited under change.
- [§GOAL-friendliness-first.1](goals.md#1-hard-requirements) ("errors point at the line") applies unchanged — a diff-mode finding reports `path:line: <message>` so an agent or editor jumps straight to the source.
- [§GOAL-fast-feedback](goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) is what makes layer 3 viable. A diff-gated check that takes longer than a save-cycle is one an agent will route around.
- [§GOAL-zero-config](goals.md#goal-zero-config-works-on-any-conformant-tree) holds: the instruction block (`grund init`) and diff-mode default both assume the canonical layout; projects that diverge configure per [§GOAL-configurable](goals.md#goal-configurable-every-default-is-overridable).

### 5. Measurable

- `grund init` writes the managed `AGENTS.md` block on a fresh repo; re-running with `--force` refreshes it to the current `grund` version without clobbering surrounding prose. E2E fixtures cover both paths (some already exist under `e2e/cases/init-*`).
- A diff-gated mode of `grund check` exists, runs within the [§GOAL-fast-feedback](goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) budget on the working tree, and reports uncited new declarations / code / decisions / e2e cases on the lines they were introduced. An e2e fixture stages a deliberately ungrounded diff and asserts that the mode catches each missing citation.
- A "happy path" fixture stages a fully grounded diff and asserts the mode exits clean with the [§GOAL-friendliness-first.1](goals.md#1-hard-requirements) fixed success marker.

## GOAL-no-dangling-refs: every cited ID resolves to a declaration

A repo that passes `grund` has zero dangling references and zero broken section coordinates. False negatives are bugs. This is the load-bearing promise; everything else exists in service of this one.

### 1. What "resolves" means

A citation `§FS-<user-login>.3.1` resolves when:

- A declaration of `FS-<user-login>` exists somewhere in the scanned tree.
- The declaration body contains a numbered section `3.1` (recursively, at any depth — see [§FS-config.3.3](../functional-spec/FS-config.md#33-section-paths--arbitrary-nesting-depth)).
- If the declaration is a stub (H1 of the form `# <ID>: [<text>](<path>)`), the pointed-at file contains an inline declaration of the same ID.

### 2. Measurable

The e2e suite includes deliberately broken inputs (missing declarations, missing sections, broken stubs); `grund` must catch each one and report it on the right line. Any uncaught case is a regression.

## GOAL-polyglot-citation: IDs cite cleanly from anywhere they are useful

A `grund` citation is valid in a Markdown file, a Java doc-comment, a Rust `///` line, a Python docstring, a Go doc block, a TypeScript JSDoc, or any other source-comment form enumerated in [§AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments) — and `grund` verifies it the same way in every one. This is the property that off-the-shelf Markdown link checkers (`lychee`, `markdown-link-check`) cannot offer, and it is the load-bearing reason `grund` exists alongside them rather than competing with them.

### 1. What "cleanly" means

- One citation grammar across all hosts. A citation like `§FS-<user-login>.3.1` reads, parses, and resolves identically whether the file is `.md`, `.rs`, `.java`, `.py`, `.go`, `.ts`, or any other extension on the configured scan list.
- One marker. The same `§` (or whatever [§DF-reference-marker](../decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger) resolves to in the project's config) is recognized in every file type; no per-language escape rules.
- One section grammar. The trailing `.3.1` resolves to a heading inside the declaration body the same way regardless of which file type the *declaration* lives in (`.md` page, inline Rustdoc, Javadoc, Python docstring).
- One resolver. Citations cross the docs/code boundary in both directions: a Markdown spec under `docs/` may cite an architectural ID whose home is a Java class doc-comment, and the Java class doc-comment may cite a functional ID back. Both are validated by the same `grund check` walk.

### 2. Why this is a goal, not a side effect

Markdown links degrade the moment a citation crosses the docs/code boundary: source files are not rendered, anchor slugs are not produced, and a path-relative link from `src/bus.rs` into `docs/` is fragile under refactor. The polyglot property is what makes IDs strictly stronger than links for the cases that matter to spec-driven projects — and it is what justifies the existence of a separate tool. Treating it as a load-bearing goal forces every other design choice (scanner, resolver, config, error format) to keep the property intact.

### 3. Composition with other goals

- [§GOAL-no-dangling-refs](goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) is the *correctness* contract; [§GOAL-polyglot-citation](goals.md#goal-polyglot-citation-ids-cite-cleanly-from-anywhere-they-are-useful) is the *coverage* contract. Together they say: every cited ID resolves, no matter where the citation lives.
- [§GOAL-multi-language](goals.md#goal-multi-language-same-engine-three-platforms) is about the *engine* shipping on three registries (cargo / npm / PyPI). [§GOAL-polyglot-citation](goals.md#goal-polyglot-citation-ids-cite-cleanly-from-anywhere-they-are-useful) is about the *citations themselves* spanning languages. The two are independent — one is about distribution, the other about the reference grammar.
- [§GOAL-friendliness-first.1](goals.md#1-hard-requirements) ("errors point at the line") applies in every host: a dangling cite in a Javadoc reports `<path>:<line>` exactly the way a dangling cite in a Markdown file does.

### 4. Measurable

The e2e suite includes positive and negative fixtures for every supported doc-comment form in [§AR-scanner.4](../architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments) — Javadoc, JSDoc/TSDoc, Doxygen, KDoc, Scaladoc, Rustdoc (`///`, `//!`, `/** … */`), Go `//` blocks, Python `""" … """` docstrings, C# XML doc, Ruby `#` lines. Each fixture exercises a citation crossing the docs/code boundary in both directions. A regression in any host is a release blocker.

## GOAL-fast-feedback: grund must be as fast as possible

Speed is not a target — it is an **ordering principle**. When a design choice trades clarity, generality, or features for speed, speed wins. `grund` exists to be invoked on every keystroke (IDE), every save (watcher), every commit (CI). Anything slower than human reflex breaks the loop it is meant to enable.

### 1. Performance targets

These are the targets the implementation is designed around. As of 0.1.0 they are met by a wide margin in practice (`grund .` on this repo runs in tens of milliseconds). The instruction-counting benchmark harness that turns the targets into a number CI records — `cargo bench` over the hot commands, run under Callgrind so the figure does not flake on a loaded runner — is [§AR-benchmarks](../architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands); the committed baseline and the build-failing regression threshold on top of it are the remaining work under [§RM-benchmarks](../roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-goal-fast-feedback-budgets). Until that threshold lands, CI runs the harness for the numbers and carries the cheap catastrophic-regression guard in §3.

- Under **100 ms** on the `grund` repo itself. The self-host loop must be invisible.
- Under **1 s** on a 10k-file repo.
- Single allocation per file at most; zero allocations on the hot regex path where possible.

### 2. How we get there

- Linear pass per file. No second walks for second-stage checks.
- Streaming line scan, not full-file buffering, on large files.
- Parallel walk using `rayon` once the single-thread version stops winning.
- Compiled regexes shared across all files via `once_cell`.
- Skip directories that obviously cannot contain specs — see [§FS-config.3.5](../functional-spec/FS-config.md#35-scan--what-gets-walked).

### 3. Measurable

Manual timing on this repo and on a synthetic 10k-file fixture should stay within the targets above. The `cargo bench` instruction-counting harness over the commands agents and CI run most ([§AR-benchmarks](../architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands), run in CI by the [§AR-ci.5](../architecture/AR-ci.md#5-benchmark-job) job) is what records the per-commit number; turning a committed baseline into a release-blocking regression threshold is the remaining work in [§RM-benchmarks](../roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-goal-fast-feedback-budgets). Alongside it CI runs the built `grund .` under a generous timeout ([§AR-ci.4](../architecture/AR-ci.md#4-performance-smoke-guard)) so a catastrophic regression — an accidental quadratic walk, a re-read pass — still fails the build outright.

## GOAL-zero-config: works on any conformant tree

No config file, no flags required for the canonical layout. Discovery is by walking from the supplied root. The default behavior is the canonical `grund` reference grammar — that is the contract.

### 1. What "canonical layout" means

A repo whose layout follows the canonical `grund` conventions: `AGENTS.md` at the root; `docs/` containing `functional-spec/`, `architecture/`, `decisions/{architectural,functional}/`, `goals/`; `e2e/` for end-to-end tests; sources under `src/`; IDs in the canonical grammar. For such a repo, `grund .` Just Works — with no config, the walk covers `docs/`, `e2e/`, and `src/` (the default `[scan] include`, [§FS-config.3.5](../functional-spec/FS-config.md#35-scan--what-gets-walked)). A project whose sources or specs live elsewhere is one `[scan] include` line away from the same experience (`grund init` writes the file for editing), and `grund check <path>` always scans exactly the path it is handed regardless of the default scope. A walk that ends up reading nothing says so rather than exiting `0` silently ([§FS-check.2.2](../functional-spec/FS-check.md#22-empty-scan)) — the "any conformant tree" promise fails loud, never quiet.

### 2. Measurable

`grund <repo>` works on any canonical-layout repo without additional setup. The e2e suite includes a "minimal conformant repo" fixture; `grund` must report zero errors with no flags and no `grund.toml`. A repo whose content sits outside the default scope and carries no config gets the empty-scan notice of [§FS-check.2.2](../functional-spec/FS-check.md#22-empty-scan), not a misleading clean exit.

### 3. Composition with [§GOAL-configurable](goals.md#goal-configurable-every-default-is-overridable)

Zero-config and configurable are not in tension — they compose. Out-of-the-box, `grund` matches the canonical defaults; for projects that diverge, every assumption is overridable per [§FS-config](../functional-spec/FS-config.md#fs-config-grund-reads-a-toml-config-file-under-agents). There is no middle ground where defaults are weird.

## GOAL-multi-language: same engine, three platforms

Cargo, npm, and PyPI ship the same engine, with idiomatic API surfaces on each. The check command behaves identically on all three. This is what makes `grund` viable as a dependency for projects whose CI pipelines, editor tooling, or test harnesses are written in JavaScript or Python — not just Rust.

### 1. Identical behavior

The same input — a tree plus an optional `grund.toml` — produces a byte-identical report regardless of which binding called the engine.

### 2. Idiomatic surfaces

Each binding fits its host. Rust returns `Result<T, E>`; Node returns Promises; Python returns values and raises exceptions. Names follow each ecosystem's conventions. Behavior is identical; surface fits each. See [§FS-distribution](../functional-spec/FS-distribution.md#fs-distribution-grund-distribution-targets) and [§AR-bindings](../architecture/AR-bindings.md#ar-bindings-target-shape-for-exposing-the-rust-engine-on-three-platforms) for the implementation.

### 3. Measurable

An integration test runs the same spec corpus through each binding and asserts byte-identical reports. Any diff between bindings is a release blocker.

## GOAL-friendliness-first: as user- and agent-friendly as possible

Friendliness is the second **ordering principle** (alongside speed, [§GOAL-fast-feedback](goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible)). When a design choice trades raw capability or terseness for legibility, legibility wins. `grund` is used by humans in terminals and IDEs *and* by AI agents through stdout pipelines — both audiences must be served.

### 1. Hard requirements

- **Errors point at the line.** Every error message includes `path:line: <message>`, so editors and agents can jump to the source unmodified.
- **Output is parseable.** A `--format=json` flag emits a stable JSON shape suitable for LLM consumption and editor integration.
- **Show is grounded.** `grund show <ID>` returns just the declaration body — no surrounding context, no scrolling, no token waste — under 200 lines for the common case.
- **Help is actionable.** `grund --help` is one screen; every flag has a one-line example.
- **No surprises.** Same input → same output, byte-for-byte. Order of files in the report is deterministic.
- **Explicit success.** A passing text `grund check` prints exactly `success` on stdout; machine-readable `--format=json` stays diagnostics-only.

### 2. What this rules out

By accepting friendliness as an ordering principle, we rule out designs that would compromise it for marginal gain: configurable severity levels (would let two installs disagree on whether a repo passes), configurable report ordering (would break editor integrations), per-flag interactive prompts (would block CI).

### 3. Measurable

Typical `grund show` output under 200 lines; `grund --format=json` validates against a stable schema in `e2e/`; `grund --help` fits in 24 lines; round-trip determinism is enforced by an e2e test that runs `grund` twice and diffs the output.

## GOAL-configurable: every default is overridable

Zero-config by default ([§GOAL-zero-config](goals.md#goal-zero-config-works-on-any-conformant-tree)); configurable when a project's conventions diverge. Users must be able to write references **the way they like**.

### 1. What is configurable

Per [§FS-config](../functional-spec/FS-config.md#fs-config-grund-reads-a-toml-config-file-under-agents), a `grund.toml` at the repo root can override the set of `KIND` prefixes, the ID format itself, the reference marker and typing trigger, strict vs optional marker mode, the set of folders that are scanned and skipped, the supported comment prefixes for inline specs, and the output format defaults.

### 2. What is NOT configurable

Per [§GOAL-friendliness-first.2](goals.md#2-what-this-rules-out), the severity model, exit-code mapping, report ordering, and other invariants that would let two correctly-configured installs disagree on a repo's well-formedness are deliberately **not** configurable.

### 3. Measurable

An e2e fixture with a non-default `grund.toml` (custom kinds, alternate section delimiter) passes. The default config — applied implicitly when no `grund.toml` exists — produces canonical `grund` grammar.

## GOAL-no-silent-breakage: changes ship through a deprecation path

A repo that worked yesterday must work today. Every user-visible change to `grund` either stays backwards-compatible or ships through a deprecation path that names the removal horizon. Silent semantic changes — output shape, exit codes, config schema, grammar — are release blockers, not features. This goal extends [§GOAL-friendliness-first.1](goals.md#1-hard-requirements)'s "no surprises" from within-run determinism to cross-version stability.

### 1. What counts as user-visible

- CLI surface: subcommands, flags, and the exit-code mapping (frozen per [§GOAL-friendliness-first.2](goals.md#2-what-this-rules-out) and [§FS-non-goals.9](../functional-spec/FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization)).
- Output bytes: stdout and stderr shapes that tools, editors, and agents pipe — including the `--format=json` schema ([§GOAL-friendliness-first.1](goals.md#1-hard-requirements)).
- `grund.toml` schema ([§FS-config.3](../functional-spec/FS-config.md#3-schema)) and the `grund_config_version` ([§FS-config.5](../functional-spec/FS-config.md#5-schema-versioning)).
- Reference grammar: KIND set, ID format, marker, trigger ([§DF-reference-marker](../decisions/functional/DF-reference-marker.md#df-reference-marker-use--as-the-reference-marker-with--as-the-typing-trigger)).
- The `AGENTS.md` init block content and its version markers ([§FS-init.2.3](../functional-spec/FS-init.md#23-generated-agent-entrypoints)).

Internal refactors that leave every item above byte-identical are out of scope — they are not "changes" in the sense this goal covers.

### 2. The deprecation path

A change crosses at least two releases:

1. **Release N** introduces the new form. The old form continues to work and emits a one-line warning to stderr that names the change, the new form, and the release in which the old form will stop working. The changelog entry (FS — see `docs/changelog.md` §1) links the warning to its specification.
2. **Release ≥ N+1**, after the named horizon, removes the old form. A schema-version bump (`grund_config_version`, `AGENTS.md` block) **is** the horizon for that surface — `grund` refuses to load the old version with a message that points at the migration.

A change that cannot be expressed as a deprecation path (e.g. a security fix that requires immediate semantic change) is documented in `docs/decisions/architectural/` with the reason the goal is being broken, in advance of the release.

### 3. Measurable

- Every removal or semantic change in `docs/changelog.md` cites the prior release that introduced the deprecation warning. Releases that remove behavior with no prior entry are blocked.
- An e2e fixture per deprecated surface asserts that the deprecated form still works and emits the warning, and that the new form is accepted in parallel.
- Tombstone fixtures (per the e2e deprecation convention) keep the old behavior's executable proof alive across the deprecation window so a regression in the legacy path fails CI.

## GOAL-small-and-large: start small, configure for big

`grund` serves both ends of the size spectrum without forking into two tools. A solo project with nine specs and a multi-team monorepo with thousands use the same binary, the same citation grammar, the same defaults — only the *layout* deepens. The shape of the tree adapts; the contract does not.

### 1. Small-repo promise

A repo with a handful of specs at the root of `docs/functional-spec/` works with zero ceremony. No components required, no synthetic buckets, no `meta/` directory created to satisfy a schema. This is today's `grund` repo and it must keep working unchanged as the project grows.

### 2. Large-repo promise

A repo with hundreds of specs across many components organizes them into a component tree (`docs/functional-spec/<comp>/<sub>/...`) without changing citation syntax or breaking the resolver's invariants. The layout grows by adding component paths, not by stretching a single flat bucket — adding a spec in one component does not coordinate with any other.

### 3. Layout knobs live in config

Per [§FS-config](../functional-spec/FS-config.md#fs-config-grund-reads-a-toml-config-file-under-agents), the layout differences between small and large repos are exposed as keys in `grund.toml`, defaulting to small. Repos opt in to component-required mode (and any other layout commitments needed for scale) when flat stops working for them. Both modes are first-class; neither is a degraded form of the other.

### 4. Composition with [§GOAL-zero-config](goals.md#goal-zero-config-works-on-any-conformant-tree) and [§GOAL-configurable](goals.md#goal-configurable-every-default-is-overridable)

The default — flat, component-optional — keeps zero-config intact for the small case ([§GOAL-zero-config](goals.md#goal-zero-config-works-on-any-conformant-tree)). Configurability picks up where flat stops scaling ([§GOAL-configurable](goals.md#goal-configurable-every-default-is-overridable)). The two ordering principles compose: out-of-the-box behavior is canonical small-repo `grund`; scale is opted into, not bolted on.

### 5. Measurable

- An e2e fixture for a "tiny conformant repo" (handful of specs, flat, no `grund.toml`) passes.
- An e2e fixture for a "large conformant repo" (synthetic, sized to fit CI budget, with components, sub-components, and tree-form specs) passes with the appropriate `grund.toml` and meets the [§GOAL-fast-feedback](goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) budget for a 10k-file repo.
- The large fixture, with its `grund.toml` removed, fails — proving that scale features are opt-in, not implicit.

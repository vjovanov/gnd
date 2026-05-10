# Changelog

Records every notable change to `gnd`. Versions follow semver; the **latest release is inline** in this file, and **older releases live one-per-file under `docs/changelog/`** so a reader (human or agent) only loads the history they ask for. Each entry cites the FS/AS/G/DF IDs it touches, so the changelog is itself part of the conformant tree (`gnd .` validates the citations).

Schema-version bumps are called out explicitly: `gnd_config_version` (§FS-config.5) and the `agents.md` init block version (§FS-init.2). A bump to either is a breaking change for the consumer and must appear under **Changed** with a migration note.

## 1. Conventions

### 1.1 Sections per release

`Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`. Omit any section with no entries.

### 1.2 Schema version callouts

Any change to `gnd_config_version` or the `agents.md` block version goes under **Changed** with the prefix `**Schema:**` and a one-line migration pointer.

### 1.3 Entry style

One bullet per change, present tense, leading with the affected ID. Example: `§FS-show: add --head mode for truncated output`.

### 1.4 Progressive discovery

Only **Unreleased** and the **most recent release** are inline. When a new release ships, the previous "latest" section is moved verbatim to `docs/changelog/<version>.md` and a one-line link is added under [§4 Older releases](#4-older-releases). The most recent release stays inline so the common reader and agent path — "what changed lately?" — is one file deep.

## Unreleased

### Added

- §FS-check.3.6: `[reference] require_grounding` (off by default) and `gnd check --require-grounding` — when on, `check` reports an `ungrounded source file` error for every scanned non-Markdown file that carries no resolving citation and declares no ID inline. Shipped under §RM-require-grounding; the grounding floor of §DF-require-grounding; the diff-aware tiers are tracked under §RM-cover and §RM-cochange-gate. `gnd config show` now prints `require_grounding`; `templates/gnd.toml` carries `require_grounding = false`. Content change within `gnd_config_version = 1` — a v1 config without the key is unaffected.
- §FS-config.3.1: document the `require_grounding` key.

## 2. [0.1.0] — 2026-05-10

First published binary. The CLI covers the full subcommand surface the specs describe (§2.7); the distribution arc — the `gnd-core`/`gnd-cli` workspace split, the npm/PyPI bindings, the optional `gnd-lsp` server, and `gnd check --watch` — is tracked in `docs/roadmap.md`.

### 2.1 Baseline

- Initial scheme in place: `gnd_config_version = 1` (§FS-config.5), `agents.md` init block at **v1** (§FS-init.2).
- `gnd check` implemented against the canonical grammar (§FS-check), with the full e2e corpus and the byte-for-byte determinism sweep (§FS-non-goals.13).
- Decision records in scope: §DA-reference-checker-name and §DF-reference-marker.

### 2.2 Changed

- §FS-init: drop `docs/state-and-direction.md` from the `--docs` scaffold; the soft direction folds into `docs/roadmap.md` and the project-specific change rules move to `agents.md`. The `agents.md` v1 block's `docs/` table is updated to list `roadmap.md` and `changelog.md`. Content change within v1; no schema bump.
- raison-detre: reframe around the polyglot pitch and the three pillars (verify in source comments, refactor-safe IDs, extract). No surface change; sharpens what `gnd` is for vs. off-the-shelf Markdown link checkers (`lychee`, `markdown-link-check`).
- §FS-non-goals.6: drop the `gnd graph` forward-reference (it was "planned in roadmap" but never was); state that ID-graph visualisation is not a committed feature, and point the reverse-lookup need at the now-specified §FS-refs. The §6 non-goal (no decision database / no history view) is unchanged.
- §FS-config.3.4: enumerate the canonical `[[kinds]]` tables in full (G/FS/AS/DF/DA/E2E/RM with `folder` and `title`) instead of showing only `FS` — §FS-init.2.4 says the generated `.agents/gnd.toml` matches this section exactly, so it had to be exact. Drop the `gnd new` (future) reference; clarify `title` is metadata surfaced in `--format=json` and hover, not injected into `gnd show --format=md` text.
- §FS-config.3.4 / §FS-distribution: add `RM` ("Roadmap milestone") to the canonical `[[kinds]]` set (now seven, `folder = "docs"`) — `RM` declarations are H2 headings inside the single file `docs/roadmap.md`, parallel to how `G` is declared inside `docs/goals/goals.md`, so `gnd check` validates `§RM-…` citations like any other reference instead of silently ignoring them. This is an *additive* grammar change (a repo that overrides `[[kinds]]` is unaffected; a default-config repo with a `§RM-…` token in prose now gets it checked, which is the intent of the marker); `gnd_config_version` and the `agents.md` block version are unchanged. `templates/gnd.toml` (and therefore the embedded `gnd init` config) gains the `RM` table.
- §FS-show.1 / §FS-show.2.1.1: document that `gnd show <ID>` also takes a positional `[<path>]` (with `--path` as an accepted alias), and that `--head` combined with a section path prints the lead prose of *that* section — previously the spec listed neither.
- §FS-errors.3: the worked example for the "lowercase first letter" rule used `dangling citation: <ID>`, which is not the text `gnd` emits (`unknown reference <ID>`); the example now matches the shipped message.
- §FS-distribution.1: stop claiming the PyPI name `gnd` is "free" — §DA-reference-checker-name records it as a dormant squat; the text now matches that record and points at §RM-distribution-naming as the gate that re-verifies every name (and picks an explicit alternate if a name assumed dormant turns out live) before the first publish.
- README: drop `--watch` from the `gnd check` synopsis row (it is specified in §FS-check.6 but not yet implemented — `gnd check --watch` errors as an unknown flag); add `docs/discussions/` to the project-layout list.
- `.agents/gnd.toml` (this repo): add the `[fmt.md_links]` block so the on-disk config matches what `gnd init` now writes (§FS-config.3.7 / §FS-init.2.4 require the generated config to carry every documented key).
- §FS-config.3.5 / 3.6: note that the default `comment_prefixes` set is broader than the §AS-scanner.4 doc-comment table (`;`, `--`, `*`, `/*`); define `relative_paths = false` (paths relative to the path argument / cwd, never absolute — keeps §FS-errors.4 intact); cross-reference `color` to §FS-errors.3.
- §AS-scanner.2.1–2.3: pin the heading grammar that section resolution depends on — declaration heading level `L` is recorded; a depth-`d` section heading is `#{L+d}` followed by a `d`-component dotted number with an optional trailing `.`; section heading text is recorded alongside the path. Add the string-literal carve-out for **bare** citations in source files (mirrors `gnd fmt` §FS-fmt.2.3.1); marker-prefixed citations are still recognised everywhere.
- §FS-check.1.1 / 2: document the source-file string-literal carve-out for `check`, and the partial-scan semantics — a per-file read/decode failure mid-walk is reported `error: <path>: <reason>`, the walk continues, collected findings still print, and the run exits `2` (incomplete view).
- §FS-name: fix the §2.1/§2.2 examples to use the documented default `[id] format` (`FS-008-…`, with a parallel `{kind}-{slug}` example matching gnd's own repo); add §4.1 defining `gnd name` under number-less and slug-less ID formats (`--width` ignored, JSON `number` is `null`, collision check matters more).
- §FS-show: clarify `text` vs `md` vs `json` output (§3.1 — `md` includes the heading verbatim, `title` is not injected; `json` is one object, `section` is `null` for a whole declaration); §2.3.4 defines `show` on a broken stub (exit 1, bare query line); §2.3.3 spells out relative section depth inside doc-comments.
- §FS-init.2.1 / 2.3: add `docs/roadmap.md` and `docs/changelog.md` to the `--docs` scaffold (the generated `agents.md` block links to them); restate that the canonical `agents.md` block text is embedded in the binary (reference copy `templates/agents.md`), versioned by the `vN` marker — `gnd check` validates the markers and version, not a byte-diff.
- §FS-errors: add §FS-refs and §FS-cli to the cross-cutting list; §2.1 now covers `gnd refs` lines, §2.2 covers §FS-cli's unknown-subcommand / bad-flag errors.
- §FS-config.3.7: add the `[fmt.md_links]` block (`enabled`, `anchor_format`) to the documented schema. The detailed contract still lives in §FS-fmt.6.7 and §DF-md-link-anchor-strategy; §FS-config.3.7 exists because §FS-init.2.4 writes every key in it, so the generated `.agents/gnd.toml` (and `templates/gnd.toml`) now includes it.
- §FS-init.2.1: rewrite the `--docs` scaffold description to match what `gnd init` actually emits — richer starter files embedded in the binary (`raison-detre.md` with its three H2 sections, the spec READMEs with an empty ID/Subject table, `e2e/README.md` with its one-line note, `roadmap.md` / `changelog.md` with the H1 plus a placeholder line) rather than bare H1-plus-placeholder stubs. The "byte-identical at the same `gnd` version" guarantee (§FS-non-goals.13) is unchanged.
- §FS-init: the scaffolded `functional-spec/` and `architectural-spec/` README templates no longer claim "`gnd check` enforces it; missing links are errors" about the index — `gnd check` has no such rule (it is not in §FS-check.3); the templates now state README linkage as a convention.
- §FS-name: an unknown `<KIND>` is now an exit-`2` CLI-level error (an `error:` line naming the kind, plus a `known kinds: …` line), matching `gnd list --kind <unknown>` — previously it exited `1`. The remaining query failures (empty slug, collision) keep exit `1` but drop the `error:` prefix, so an `error:`-prefixed stderr line reliably means an exit-`2` launch-time failure (§FS-cli.4). §FS-name.6, §FS-name.3, and §FS-name.5 updated.
- §FS-show.2.4: the `gnd show E2E-<name>` manifest is documented as it is emitted — the invocation line, an `expected exit: <code>` line, then a `fixtures:` line followed by one `- <path>` line per fixture file (sorted); `--head` prints the invocation line only. JSON shape `{"id","kind","path","args","expected_exit","fixtures"}` confirmed.
- README: add the `agents.md` init-block check (§FS-check.3.5) to the "What it Checks" list; add `RM` to the `[id]` kinds list; refresh the Subcommands table so it lists `show --section`/`--full`/`--format`, `fmt --check`/`--write`/`--md-links`, and the `[path]`/`--format` arguments other commands take; replace the `src/scanner.rs` / `{kind}-{number}-{slug}` worked examples in "Verifying what a file refers to" with ones that run against this repo's actual layout and slug-only `[id] format`.
- Packaging: `Cargo.toml` `description` rewritten to the README's pitch ("the polyglot reference checker …") instead of the stale "league-of-code style docs" phrasing; `keywords` and `categories` added for the crates.io listing. The `examples/parallel-spec-review-example/` directory (an unrelated workflow demo, never referenced from `examples/README.md`) is removed so it no longer ships in the published crate.
- §FS-init.2.3: the generated `agents.md` v1 block now follows the repo's effective config — the ID-shape line, the `## References` ID-scheme line, the worked declaration heading, the `KIND ∈ {…}` set, and the marker / `$$`-trigger / strict-vs-bare-tokens notes are rendered from `.agents/gnd.toml` (or the defaults `init` is about to write) instead of a fixed `<KIND>-<NNN>-<slug>` boilerplate. A `{kind}-{slug}` repo (like this one) now gets a `<KIND>-<slug>` description. Content change within v1; no schema bump (`gnd check` still validates the block markers and version, not a byte-diff). `templates/agents.md` carries the new placeholders.
- `agents.md` / `templates/agents.md`: the `gnd` project's own repository URL was `github.com/anthropics/gnd` in the agents template; corrected to `github.com/vjovanov/gnd` to match `Cargo.toml` `repository`/`homepage`. This repo's `agents.md` is regenerated from the corrected template (it had also accumulated a duplicated copy of the v1 block above the block).
- CLI: restore the default `SIGPIPE` disposition at startup so a closed downstream pipe (`gnd list | head`, `gnd refs … | head`) ends the process quietly the way `ls | head` does, instead of panicking on the next `println!` with `failed printing to stdout: Broken pipe`. Unix only; a no-op elsewhere.
- §FS-check.4.1: state that the unused-declaration warning skips `E2E` declarations — an end-to-end case is exercised by being run, not by being cited, so a never-cited `§E2E-…` is not a warning. README "What it Checks" updated to match.

### 2.3 Added

- §G-polyglot-citation: new goal making explicit that one citation grammar resolves identically across `.md` and every supported source-comment form in §AS-scanner.4. Pairs with §G-no-dangling-refs (correctness × coverage).
- §FS-fmt.6: new section spec'ing an opt-in `--md-links` mode that wraps marker-prefixed citations in clickable Markdown links inside `.md` files. Source files are never touched.
- §DF-md-link-emission: decision record for the wrap-the-citation form and the reconciliation with §FS-non-goals.1 (link validation stays out of scope) and §FS-non-goals.5 (no rendered docs). The §2.2 "Anchor format" section is superseded by §DF-md-link-anchor-strategy below.
- §DF-md-link-anchor-strategy: decision record picking heading-text slugs (per a configurable renderer profile) re-derived on every `gnd fmt --md-links` pass. Retracts the placeholder section-coordinate anchor format from §DF-md-link-emission's first draft, which proved factually wrong about renderer behavior on review. Updates §FS-fmt.6.2 (anchor bullet), §FS-fmt.6.3 (re-derive supersedes "leave URLs alone"), and §FS-fmt.6.7 (named profiles `github`/`gitlab`/`mkdocs`/`pandoc`/`none`).
- §RM-md-link-emission: roadmap item that owns the implementation. Sequenced after §RM-marker-fmt (marker + fmt) and before §RM-distribution (bindings) so the link form stabilizes once across all three registries.
- §FS-lsp: new functional spec for the optional LSP server (`gnd-lsp`). Covers the four v1 capabilities (diagnostics, hover, go-to-definition, live `$$` → `§` trigger via `textDocument/onTypeFormatting`), opt-in install, and the policy that no first-party per-editor wrappers ship.
- §AS-lsp: new architectural spec describing the server's relationship to `gnd-core`, in-memory `Findings` cache, full-rescan strategy for v1 with incremental as a follow-on, and stdio-only transport.
- §DA-lsp-optional: new architectural decision pinning the LSP as a separate published package on every registry — not a bundled subcommand, not a Cargo feature, not a second binary in the CLI crate. Records the dependency-cost, CI-binary-size, and industry-parallel reasoning.
- §FS-non-goals.12.2: new non-goal entry — first-party per-editor plugins (VSCode, IntelliJ, Vim, Emacs wrappers) are out of scope. Reorganizes §12 into a parent header about "surfaces outside `gnd-core` and the LSP transport"; the engine-plugins non-goal becomes §12.1, the editor-wrappers non-goal §12.2. The §13 bright line is unchanged.
- §FS-refs: new functional spec for `gnd refs <ID>` — the reverse of `gnd show`: list every citation site of an ID (located-finding shape; NDJSON for tooling), sharing the scanner with `gnd check` so the two never disagree on what counts as a citation. Closes the agent-grounding loop (read the body / know the blast radius) without telling agents to `grep`.
- §FS-cli: new cross-cutting spec pinning the command-line surface — `gnd` / `gnd <path>` default to `gnd check`; `gnd --version` (alias `-V`) and `gnd --help` (alias `-h`) are handled before any scan; `--format` is the cross-subcommand flag; unknown subcommand / bad flag → `error:` + exit 2; the exit-code mapping is frozen; `gnd graph` / `gnd new` are explicitly absent.
- §FS-check.6: new section spec'ing `gnd check --watch` — stay resident, re-check on every change under the scanned tree, clearing prior output each run; filesystem-notification-driven and debounced (no polling, no interval knob); each run byte-identical to a plain `gnd check`; non-interactive, no network. The editor-less counterpart to §FS-lsp.
- §AS-scanner.6: new section spec'ing `E2E` declaration discovery — the one kind discovered by directory structure (a case directory under `e2e/cases/` containing `expected.exit`), not a heading line; the directory name is the ID with the `{kind}` portion stripped; the recorded "body" is the case manifest (invocation, expected exit, fixture list) that §FS-show.2.4 prints. `§E2E-<name>` citations resolve like any other.
- §FS-show.2.4: `gnd show E2E-<name>` returns the case manifest; `--head` prints the invocation line; section paths are not defined for E2E cases.
- §RM-refs: roadmap item owning `gnd refs` implementation (cheap — citations are already in `Findings`).
- §RM-watch: roadmap item owning `gnd check --watch`; sequenced after §RM-core-cli-split so the watcher calls `gnd-core` rather than re-implementing the walk.

### 2.4 Removed

- `FS-ide-plugins`: the multi-editor first-party-plugins spec is deleted; replaced by §FS-lsp and §FS-non-goals.12.2. References in §FS-errors, §FS-name, §FS-fmt, §FS-show, §AS-checker, §DF-reference-marker, and the roadmap have been swept to point at §FS-lsp instead.

### 2.5 Renamed

- `RM-ide-plugins` → `§RM-lsp`. Scope shrinks: ship the `gnd-lsp` crate and per-registry packages; document one-time editor setup snippets in the README; do not maintain per-editor wrappers.
- Roadmap milestone IDs reslugged to fit this repo's slug-only `[id] format` now that `RM` is a checked kind: `RM-001-e2e-corpus` → `§RM-e2e-corpus`, `RM-002-show` → `§RM-show`, `RM-003-config` → `§RM-config`, `RM-004-marker-fmt` → `§RM-marker-fmt`, `RM-005-distribution` → `§RM-distribution`, `RM-006-lsp` → `§RM-lsp`, `RM-007-self-host` → `§RM-self-host`, `RM-008-core-cli-split` → `§RM-core-cli-split`, `RM-009-distribution-naming` → `§RM-distribution-naming`, `RM-010-md-link-emission` → `§RM-md-link-emission`, `RM-011-refs` → `§RM-refs`, `RM-012-watch` → `§RM-watch`. The shipped milestones (`§RM-e2e-corpus`, `§RM-show`, `§RM-config`, `§RM-marker-fmt`, `§RM-md-link-emission`, `§RM-refs`) keep a one-line declaration under `docs/roadmap.md` §"Shipped milestones" so their citations resolve; the detail stays in §2.7 below.

### 2.6 Distribution and bindings

- §FS-distribution.1: adds `gnd-lsp` as a separate package on cargo, npm, and PyPI. The CLI install on each registry does not transitively pull in `gnd-lsp`. §FS-distribution.4 release process publishes the LSP alongside the CLI in lockstep.
- §AS-bindings.1: workspace gains a `crates/gnd-lsp/` member; sections renumbered (§3 CLI, §4 LSP, §5 Node, §6 Python, §7 Why this shape) with no external citations affected.

### 2.7 Implemented

The CLI now covers the full subcommand surface the specs describe. The remaining work — the `gnd-core`/`gnd-cli` workspace split, the npm/PyPI bindings, the optional `gnd-lsp` server, and `gnd check --watch` — is tracked in `docs/roadmap.md`.

- §RM-self-host: the self-host loop passes — `cargo run -- .` on this repo exits `0` with empty stdout — and CI now enforces it on every push and pull request. The scanner skips fenced Markdown blocks (§FS-fmt's existing exemption applied to the scanner), so illustrative IDs in fenced examples are inert. Remaining under §RM-self-host: a default-config nested-fixtures e2e fixture.
- §RM-e2e-corpus: the `e2e/cases/*` corpus plus `tests/e2e.rs` and `tests/init.rs` run on every push (`.github/workflows/ci.yml`), with positive and negative fixtures per §FS-check error class and a byte-for-byte determinism sweep (§FS-non-goals.13).
- §RM-show: `gnd show <ID>` per §FS-show — whole declaration, `--head`, `--section` and dotted-inline section paths, `--full`, `--format text|md|json`, inline-source extraction with comment-marker stripping (§FS-show.2.3), E2E case manifests (§FS-show.2.4), and the ambiguous-ID / broken-stub query-result forms.
- §RM-config: `.agents/gnd.toml` discovery and parsing per §FS-config — the line-oriented TOML subset, unknown-key errors with `path:line:` pointers, `gnd_config_version` gating, `[reference]` / `[id]` / `[[kinds]]` / `[scan]` / `[output]` / `[fmt.md_links]`, plus `gnd config validate` and `gnd config show`.
- §RM-marker-fmt: the `§` marker and `$$` trigger per §DF-reference-marker; `gnd fmt` with `--check` / `--write` / `--marker`, the deterministic string-literal carve-out (§FS-fmt.2.3.1), declaration-heading and fenced-block exemptions, and `[reference] strict = true` (this repo runs in strict mode).
- §RM-md-link-emission: `gnd fmt --md-links` per §FS-fmt.6 and §DF-md-link-anchor-strategy — wraps marker-prefixed citations in `.md` files only, heading-text anchor slugs per the `github` / `gitlab` / `mkdocs` / `pandoc` / `none` profiles, re-derived each pass, fenced/inline-code-span and dangling-citation skips, idempotent, and the `[fmt.md_links] enabled` opt-in.
- §RM-refs: `gnd refs <ID> [path] [--section <s>] [--format text|json]` per §FS-refs — over the same scan as `check`, sorted by `(path, line, column)`, honouring strict mode and the string-literal carve-out; NDJSON on stdout for `--format=json`, exit `0` always on a successful scan.
- `gnd init` per §FS-init — `agents.md` + `.agents/gnd.toml`, the `--docs` scaffold, the versioned managed block with append / in-place update / position preservation / CRLF handling, `--name` / `--force` / `--append`, and idempotent `exists ` reporting.
- `gnd name <KIND> "<title>" [path]` per §FS-name — deterministic NFKD slugging, next-number derivation, `--width`, the collision check, number-less / slug-less `[id] format` handling, `--format text|json`.
- `gnd` CLI surface per §FS-cli and §FS-errors — default-subcommand routing, `--version` / `-V`, `--help` / `-h`, the cross-subcommand `--format`, the three message shapes, `error:`-prefixed CLI errors, and the frozen `0`/`1`/`2` exit mapping.

### 2.8 Fixed

- §FS-show: `gnd show <ID>.<section> --head` (and `--head --section S`) reported `section not found` for a section that exists — the `--head` short-circuit fired before the section was matched. It now prints the lead prose of the requested section; a missing section is still a `section not found` error. New e2e case `show-head-section-markdown` covers it.

## 3. [0.0.0] — 2026-05-08

Initial commit. Scheme, e2e fixtures, and harness; no published binary yet.

## 4. Older releases

None yet. When 0.0.0 is no longer the latest, it moves to `docs/changelog/0.0.0.md` and is linked from here.

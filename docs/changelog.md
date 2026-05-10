# Changelog

Records every notable change to `gnd`. Versions follow semver; the **latest release is inline** in this file, and **older releases live one-per-file under `docs/changelog/`** so a reader (human or agent) only loads the history they ask for. Each entry cites the FS/AS/G/DF IDs it touches, so the changelog is itself part of the conformant tree (`gnd .` validates the citations).

Schema-version bumps are called out explicitly: `gnd_config_version` (FS-config.5) and the `agents.md` init block version (FS-init.2). A bump to either is a breaking change for the consumer and must appear under **Changed** with a migration note.

## 1. Conventions

### 1.1 Sections per release

`Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`. Omit any section with no entries.

### 1.2 Schema version callouts

Any change to `gnd_config_version` or the `agents.md` block version goes under **Changed** with the prefix `**Schema:**` and a one-line migration pointer.

### 1.3 Entry style

One bullet per change, present tense, leading with the affected ID. Example: `FS-show: add --head mode for truncated output`.

### 1.4 Progressive discovery

Only **Unreleased** and the **most recent release** are inline. When a new release ships, the previous "latest" section is moved verbatim to `docs/changelog/<version>.md` and a one-line link is added under [§4 Older releases](#4-older-releases). The most recent release stays inline so the common reader and agent path — "what changed lately?" — is one file deep.

## 2. [Unreleased]

### 2.1 Baseline

- Initial scheme in place: `gnd_config_version = 1` (FS-config.5), `agents.md` init block at **v1** (FS-init.2).
- Working `gnd check` prototype against the canonical grammar (FS-check).
- Decision records in scope: §DA-reference-checker-name and §DF-reference-marker.

### 2.2 Changed

- FS-init: drop `docs/state-and-direction.md` from the `--docs` scaffold; the soft direction folds into `docs/roadmap.md` and the project-specific change rules move to `agents.md`. The `agents.md` v1 block's `docs/` table is updated to list `roadmap.md` and `changelog.md`. Content change within v1; no schema bump.
- raison-detre: reframe around the polyglot pitch and the three pillars (verify in source comments, refactor-safe IDs, extract). No surface change; sharpens what `gnd` is for vs. off-the-shelf Markdown link checkers (`lychee`, `markdown-link-check`).
- FS-non-goals.6: drop the `gnd graph` forward-reference (it was "planned in roadmap" but never was); state that ID-graph visualisation is not a committed feature, and point the reverse-lookup need at the now-specified §FS-refs. The §6 non-goal (no decision database / no history view) is unchanged.
- FS-config.3.4: enumerate the canonical six `[[kinds]]` tables in full (G/FS/AS/DF/DA/E2E with `folder` and `title`) instead of showing only `FS` — FS-init.2.4 says the generated `.agents/gnd.toml` matches this section exactly, so it had to be exact. Drop the `gnd new` (future) reference; clarify `title` is metadata surfaced in `--format=json` and hover, not injected into `gnd show --format=md` text.
- FS-config.3.5 / 3.6: note that the default `comment_prefixes` set is broader than the §AS-scanner.4 doc-comment table (`;`, `--`, `*`, `/*`); define `relative_paths = false` (paths relative to the path argument / cwd, never absolute — keeps §FS-errors.4 intact); cross-reference `color` to §FS-errors.3.
- AS-scanner.2.1–2.3: pin the heading grammar that section resolution depends on — declaration heading level `L` is recorded; a depth-`d` section heading is `#{L+d}` followed by a `d`-component dotted number with an optional trailing `.`; section heading text is recorded alongside the path. Add the string-literal carve-out for **bare** citations in source files (mirrors `gnd fmt` §FS-fmt.2.3.1); marker-prefixed citations are still recognised everywhere.
- FS-check.1.1 / 2: document the source-file string-literal carve-out for `check`, and the partial-scan semantics — a per-file read/decode failure mid-walk is reported `error: <path>: <reason>`, the walk continues, collected findings still print, and the run exits `2` (incomplete view).
- FS-name: fix the §2.1/§2.2 examples to use the documented default `[id] format` (`FS-008-…`, with a parallel `{kind}-{slug}` example matching gnd's own repo); add §4.1 defining `gnd name` under number-less and slug-less ID formats (`--width` ignored, JSON `number` is `null`, collision check matters more).
- FS-show: clarify `text` vs `md` vs `json` output (§3.1 — `md` includes the heading verbatim, `title` is not injected; `json` is one object, `section` is `null` for a whole declaration); §2.3.4 defines `show` on a broken stub (exit 1, bare query line); §2.3.3 spells out relative section depth inside doc-comments.
- FS-init.2.1 / 2.3: add `docs/roadmap.md` and `docs/changelog.md` to the `--docs` scaffold (the generated `agents.md` block links to them); restate that the canonical `agents.md` block text is embedded in the binary (reference copy `templates/agents.md`), versioned by the `vN` marker — `gnd check` validates the markers and version, not a byte-diff.
- FS-errors: add FS-refs and FS-cli to the cross-cutting list; §2.1 now covers `gnd refs` lines, §2.2 covers FS-cli's unknown-subcommand / bad-flag errors.
- §FS-config.3.7: add the `[fmt.md_links]` block (`enabled`, `anchor_format`) to the documented schema. The detailed contract still lives in §FS-fmt.6.7 and §DF-md-link-anchor-strategy; §FS-config.3.7 exists because §FS-init.2.4 writes every key in it, so the generated `.agents/gnd.toml` (and `templates/gnd.toml`) now includes it.
- §FS-init.2.1: rewrite the `--docs` scaffold description to match what `gnd init` actually emits — richer starter files embedded in the binary (`raison-detre.md` with its three H2 sections, the spec READMEs with an empty ID/Subject table, `e2e/README.md` with its one-line note, `roadmap.md` / `changelog.md` with the H1 plus a placeholder line) rather than bare H1-plus-placeholder stubs. The "byte-identical at the same `gnd` version" guarantee (§FS-non-goals.13) is unchanged.
- §FS-init: the scaffolded `functional-spec/` and `architectural-spec/` README templates no longer claim "`gnd check` enforces it; missing links are errors" about the index — `gnd check` has no such rule (it is not in §FS-check.3); the templates now state README linkage as a convention.

### 2.3 Added

- §G-polyglot-citation: new goal making explicit that one citation grammar resolves identically across `.md` and every supported source-comment form in §AS-scanner.4. Pairs with §G-no-dangling-refs (correctness × coverage).
- §FS-fmt.6: new section spec'ing an opt-in `--md-links` mode that wraps marker-prefixed citations in clickable Markdown links inside `.md` files. Source files are never touched.
- §DF-md-link-emission: decision record for the wrap-the-citation form and the reconciliation with §FS-non-goals.1 (link validation stays out of scope) and §FS-non-goals.5 (no rendered docs). The §2.2 "Anchor format" section is superseded by §DF-md-link-anchor-strategy below.
- §DF-md-link-anchor-strategy: decision record picking heading-text slugs (per a configurable renderer profile) re-derived on every `gnd fmt --md-links` pass. Retracts the placeholder section-coordinate anchor format from §DF-md-link-emission's first draft, which proved factually wrong about renderer behavior on review. Updates §FS-fmt.6.2 (anchor bullet), §FS-fmt.6.3 (re-derive supersedes "leave URLs alone"), and §FS-fmt.6.7 (named profiles `github`/`gitlab`/`mkdocs`/`pandoc`/`none`).
- §RM-010-md-link-emission: roadmap item that owns the implementation. Sequenced after §RM-004 (marker + fmt) and before §RM-005 (bindings) so the link form stabilizes once across all three registries.
- §FS-lsp: new functional spec for the optional LSP server (`gnd-lsp`). Covers the four v1 capabilities (diagnostics, hover, go-to-definition, live `$$` → `§` trigger via `textDocument/onTypeFormatting`), opt-in install, and the policy that no first-party per-editor wrappers ship.
- §AS-lsp: new architectural spec describing the server's relationship to `gnd-core`, in-memory `Findings` cache, full-rescan strategy for v1 with incremental as a follow-on, and stdio-only transport.
- §DA-lsp-optional: new architectural decision pinning the LSP as a separate published package on every registry — not a bundled subcommand, not a Cargo feature, not a second binary in the CLI crate. Records the dependency-cost, CI-binary-size, and industry-parallel reasoning.
- §FS-non-goals.12.2: new non-goal entry — first-party per-editor plugins (VSCode, IntelliJ, Vim, Emacs wrappers) are out of scope. Reorganizes §12 into a parent header about "surfaces outside `gnd-core` and the LSP transport"; the engine-plugins non-goal becomes §12.1, the editor-wrappers non-goal §12.2. The §13 bright line is unchanged.
- §FS-refs: new functional spec for `gnd refs <ID>` — the reverse of `gnd show`: list every citation site of an ID (located-finding shape; NDJSON for tooling), sharing the scanner with `gnd check` so the two never disagree on what counts as a citation. Closes the agent-grounding loop (read the body / know the blast radius) without telling agents to `grep`.
- §FS-cli: new cross-cutting spec pinning the command-line surface — `gnd` / `gnd <path>` default to `gnd check`; `gnd --version` (alias `-V`) and `gnd --help` (alias `-h`) are handled before any scan; `--format` is the cross-subcommand flag; unknown subcommand / bad flag → `error:` + exit 2; the exit-code mapping is frozen; `gnd graph` / `gnd new` are explicitly absent.
- §FS-check.6: new section spec'ing `gnd check --watch` — stay resident, re-check on every change under the scanned tree, clearing prior output each run; filesystem-notification-driven and debounced (no polling, no interval knob); each run byte-identical to a plain `gnd check`; non-interactive, no network. The editor-less counterpart to §FS-lsp.
- §AS-scanner.6: new section spec'ing `E2E` declaration discovery — the one kind discovered by directory structure (a case directory under `e2e/cases/` containing `expected.exit`), not a heading line; the directory name is the ID with the `{kind}` portion stripped; the recorded "body" is the case manifest (invocation, expected exit, fixture list) that §FS-show.2.4 prints. `§E2E-<name>` citations resolve like any other.
- §FS-show.2.4: `gnd show E2E-<name>` returns the case manifest; `--head` prints the invocation line; section paths are not defined for E2E cases.
- §RM-011-refs: roadmap item owning `gnd refs` implementation (cheap — citations are already in `Findings`).
- §RM-012-watch: roadmap item owning `gnd check --watch`; sequenced after §RM-008 so the watcher calls `gnd-core` rather than re-implementing the walk.

### 2.4 Removed

- `FS-ide-plugins`: the multi-editor first-party-plugins spec is deleted; replaced by §FS-lsp and §FS-non-goals.12.2. References in §FS-errors, §FS-name, §FS-fmt, §FS-show, §AS-checker, §DF-reference-marker, and the roadmap have been swept to point at §FS-lsp instead.

### 2.5 Renamed

- `RM-006-ide-plugins` → `§RM-006-lsp`. Scope shrinks: ship the `gnd-lsp` crate and per-registry packages; document one-time editor setup snippets in the README; do not maintain per-editor wrappers.

### 2.6 Distribution and bindings

- §FS-distribution.1: adds `gnd-lsp` as a separate package on cargo, npm, and PyPI. The CLI install on each registry does not transitively pull in `gnd-lsp`. §FS-distribution.4 release process publishes the LSP alongside the CLI in lockstep.
- §AS-bindings.1: workspace gains a `crates/gnd-lsp/` member; sections renumbered (§3 CLI, §4 LSP, §5 Node, §6 Python, §7 Why this shape) with no external citations affected.

### 2.7 Implemented

The CLI now covers the full subcommand surface the specs describe. The remaining work — the `gnd-core`/`gnd-cli` workspace split, the npm/PyPI bindings, the optional `gnd-lsp` server, `gnd check --watch`, and `E2E` directory discovery — is tracked in `docs/roadmap.md`.

- §RM-007-self-host: the self-host loop passes — `cargo run -- .` on this repo exits `0` with empty stdout. The scanner skips fenced Markdown blocks (§FS-fmt's existing exemption applied to the scanner), so illustrative IDs in fenced examples are inert. Remaining under §RM-007: a CI guard for the loop and a default-config nested-fixtures e2e fixture.
- §RM-001-e2e-corpus: the `e2e/cases/*` corpus plus `tests/e2e.rs` and `tests/init.rs` run on every push (`.github/workflows/ci.yml`), with positive and negative fixtures per §FS-check error class and a byte-for-byte determinism sweep (§FS-non-goals.13).
- §RM-002-show: `gnd show <ID>` per §FS-show — whole declaration, `--head`, `--section` and dotted-inline section paths, `--full`, `--format text|md|json`, inline-source extraction with comment-marker stripping (§FS-show.2.3), and the ambiguous-ID / broken-stub query-result forms. `gnd show E2E-<name>` (§FS-show.2.4) waits on `E2E` directory discovery.
- §RM-003-config: `.agents/gnd.toml` discovery and parsing per §FS-config — the line-oriented TOML subset, unknown-key errors with `path:line:` pointers, `gnd_config_version` gating, `[reference]` / `[id]` / `[[kinds]]` / `[scan]` / `[output]` / `[fmt.md_links]`, plus `gnd config validate` and `gnd config show`.
- §RM-004-marker-fmt: the `§` marker and `$$` trigger per §DF-reference-marker; `gnd fmt` with `--check` / `--write` / `--marker`, the deterministic string-literal carve-out (§FS-fmt.2.3.1), declaration-heading and fenced-block exemptions, and `[reference] strict = true` (this repo runs in strict mode).
- §RM-010-md-link-emission: `gnd fmt --md-links` per §FS-fmt.6 and §DF-md-link-anchor-strategy — wraps marker-prefixed citations in `.md` files only, heading-text anchor slugs per the `github` / `gitlab` / `mkdocs` / `pandoc` / `none` profiles, re-derived each pass, fenced/inline-code-span and dangling-citation skips, idempotent, and the `[fmt.md_links] enabled` opt-in.
- §RM-011-refs: `gnd refs <ID> [path] [--section <s>] [--format text|json]` per §FS-refs — over the same scan as `check`, sorted by `(path, line, column)`, honouring strict mode and the string-literal carve-out; NDJSON on stdout for `--format=json`, exit `0` always on a successful scan.
- `gnd init` per §FS-init — `agents.md` + `.agents/gnd.toml`, the `--docs` scaffold, the versioned managed block with append / in-place update / position preservation / CRLF handling, `--name` / `--force` / `--append`, and idempotent `exists ` reporting.
- `gnd name <KIND> "<title>" [path]` per §FS-name — deterministic NFKD slugging, next-number derivation, `--width`, the collision check, number-less / slug-less `[id] format` handling, `--format text|json`.
- `gnd` CLI surface per §FS-cli and §FS-errors — default-subcommand routing, `--version` / `-V`, `--help` / `-h`, the cross-subcommand `--format`, the three message shapes, `error:`-prefixed CLI errors, and the frozen `0`/`1`/`2` exit mapping.

## 3. [0.0.0] — 2026-05-08

Initial commit. Scheme, e2e fixtures, and harness; no published binary yet.

## 4. Older releases

None yet. When 0.0.0 is no longer the latest, it moves to `docs/changelog/0.0.0.md` and is linked from here.

# Changelog

Records every notable change to `grund`. Versions follow semver; the **latest release is inline** in this file, and **older releases live one-per-file under `docs/changelog/`** so a reader (human or agent) only loads the history they ask for. Each entry cites the FS/AR/G/DF IDs it touches, so the changelog is itself part of the conformant tree (`grund .` validates the citations).

Schema-version bumps are called out explicitly: `grund_config_version` ([§FS-config.5](functional-spec/FS-config.md#5-schema-versioning)) and the `AGENTS.md` init block version ([§FS-init.2](functional-spec/FS-init.md#2-outputs)). A bump to either is a breaking change for the consumer and must appear under **Changed** with a migration note.

## 1. Conventions

### 1.1 Sections per release

`Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security` — the Keep-a-Changelog set; omit any with no entries. A large entry (the first release, folding in pre-history, is the case in point) may add narrative subsection headings — e.g. `Baseline`, `Renamed`, `Implemented`, `Distribution and bindings` — for readability when the standard six would bury the structure; the semver-relevant changes still live under the standard names.

### 1.2 Schema version callouts

Any change to `grund_config_version` or the `AGENTS.md` block version goes under **Changed** with the prefix `**Schema:**` and a one-line migration pointer.

### 1.3 Entry style

One bullet per change, present tense, leading with the affected ID. Example: `§FS-show: add --head mode for truncated output`.

### 1.4 Progressive discovery

Only **Unreleased** and the **most recent release** are inline. When a new release ships, the previous "latest" section is moved verbatim to `docs/changelog/<version>.md` and a one-line link is added under [§3 Older releases](#3-older-releases). The most recent release stays inline so the common reader and agent path — "what changed lately?" — is one file deep.

## Unreleased

### Added

- [§FS-inline-citation-style](functional-spec/FS-inline-citation-style.md#fs-inline-citation-style-configurable-shape-of-inline-code-comment-citations) / [§FS-config.3.1](functional-spec/FS-config.md#31-reference--citation-form): add configurable inline citation style enforcement for source comments. `grund check` can now reject citation-only violations and hard-cap overlong inline notes, optionally warning on soft-cap overruns; generated agent entrypoints render the same house-style guidance. PR #13.

### Changed

- [§FS-check.3.9](functional-spec/FS-check.md#39-section-heading-level-mismatch) / [§FS-config.3.3](functional-spec/FS-config.md#33-section-paths--arbitrary-nesting-depth): numbered section headings are now checked against their dotted section paths. The default `strict` mode reports mismatched Markdown depth, while `warn` and `loose` give repos migration paths without a `grund_config_version` bump. PR #13.
- [§FS-fmt.6.6](functional-spec/FS-fmt.md#66-why-generated-configs-enable-cross-references) / [§FS-fmt.6.7](functional-spec/FS-fmt.md#67-configurability): generated configs now enable Markdown cross-reference emission by default, and `grund fmt --write` runs the link pass automatically for scopes that include Markdown files. Repos can opt out with `[fmt.cross_refs] enabled = false`; `--cross-refs` remains the one-run override. PR #13.
- [§FS-init.2.4](functional-spec/FS-init.md#24-generated-agentsgrundtoml): generated `.agents/grund.toml` templates now document constrained option sets inline, and init fixture specs cite the generated config contract directly. PR #14.

### Fixed

- [§FS-distribution.4](functional-spec/FS-distribution.md#4-release-process) / [§AR-ci.7](architecture/AR-ci.md#7-pull-request-changelog-gate): pull-request CI now requires the `## Unreleased` changelog body to mention the current PR number, so release notes stay mapped to the PRs they include. PR #16.
- [§AR-scanner.4](architecture/AR-scanner.md#4-inline-declarations-in-language-doc-comments) / [§FS-inline-citation-style.1](functional-spec/FS-inline-citation-style.md#1-scope): inline citation style checks now use scanner-normalized source-comment blocks, including stripped block-comment continuation markers, so diagnostics measure the author-facing comment text instead of parser syntax. PR #13.
- [§AR-scanner.2.2](architecture/AR-scanner.md#22-section-detection): normalize the scanner cross-reference for recorded section heading data to the exact section-detection point. PR #13.

## 2. [0.3.0] — 2026-05-18

Default-show release. The main user-visible change is that `grund <ID>` now resolves a declaration directly, making the short grounding command the default read path while moving whole-tree validation to the explicit `grund check` spelling.

### Changed

- [§FS-cli.1](functional-spec/FS-cli.md#1-the-default-subcommand) / [§FS-show.1](functional-spec/FS-show.md#1-inputs): the default subcommand flips from `check` to `show`. `grund <ID>` is now shorthand for `grund show <ID>` (and `grund <ID>.<section>`, `grund <ID> --toc`, `grund --toc <ID>` likewise), so an agent resolving a bare `§<ID>` runs the shortest possible command. `grund` with no arguments prints the top-level help instead of running a check on `.`.
- [§FS-completions.1](functional-spec/FS-completions.md#1-user-facing-command): bash, zsh, and fish completion now offer declared IDs in the first-argument position alongside the subcommand list, so `grund FS-<TAB>` completes IDs the same way `grund show FS-<TAB>` already did. The ID lookup is gated on an uppercase or empty prefix to keep `grund <lowercase-typo><TAB>` fast (subcommands are all lowercase).

### Removed

- [§FS-cli.1](functional-spec/FS-cli.md#1-the-default-subcommand) / [§FS-check](functional-spec/FS-check.md#fs-check-grund-validates-every-reference-in-a-repo): `grund <path>` is no longer shorthand for `grund check <path>`, and bare `grund` no longer runs `grund check .`. Validation is now spelled `grund check [<path>]` in every invocation; the bare first-argument slot belongs to `show`. **Migration:** existing scripts that ran `grund` or `grund <path>` for CI should be updated to `grund check` or `grund check <path>`. A bare path is now rejected with a diagnostic that names both readings (`invalid ID '<path>'` plus a `hint: run grund check <path> to validate a path` breadcrumb), so the migration surface is loud rather than silent.

### Fixed

- [§FS-distribution.4](functional-spec/FS-distribution.md#4-release-process): the patch and minor release-bump helpers now rotate `docs/changelog.md` automatically as part of the release candidate commit, failing if `## Unreleased` has no curated bullet entries. `release.yml` extracts the promoted section for GitHub release notes instead of publishing a static note body.

## 3. Older releases

- [0.2.0](changelog/0.2.0.md) — 2026-05-17: Workspace and agent-entrypoint release.
- [0.1.0](changelog/0.1.0.md) — 2026-05-14: first published release and baseline CLI surface.

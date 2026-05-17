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

No entries yet.

## 2. [0.2.0] — 2026-05-17

Workspace and agent-entrypoint release. The main user-visible change is that `grund` now understands a monorepo as a set of independent project namespaces: a root `.agents/grund.toml` can declare `[workspace]`, each member keeps its own local IDs, and cross-project citations use the explicit `§alias/<ID>` form. Single-project repos keep the same zero-config path; no `grund_config_version` or `AGENTS.md` block-version bump.

### Added

- [§FS-workspace](functional-spec/FS-workspace.md#fs-workspace-grund-validates-cross-project-citations-in-a-workspace) / [§DF-subproject-namespaces](decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos): `grund` now validates workspace-qualified citations. A root config can declare workspace members with explicit paths or single-segment globs, choose whether the root participates with `include_root`, and derive stable aliases from `project_name` or member directory names. `grund check` at the workspace root checks the root and every member under their own configs, treats member directories as namespace boundaries, resolves `§alias/ID[.section]` against the named project, and reports unknown aliases, missing declarations, and missing sections at the citation site. Unqualified citations still resolve only in the current project, so existing single-project IDs and member-local citations do not grow prefixes.
- [§FS-workspace.8](functional-spec/FS-workspace.md#8-other-commands) / [§AR-workspace](architecture/AR-workspace.md#ar-workspace-how-the-resolver-config-loader-and-scanner-compose-across-projects): workspace awareness is shared by the read/query surfaces. `grund show alias/ID` reads a declaration in another project; `grund refs alias/ID` gives the whole-workspace blast radius, including both qualified citations from other projects and local citations inside the target project; `grund list` at a workspace root emits qualified `alias/ID` rows and supports project filtering; completions understand aliases; and `grund fmt --cross-refs` can derive Markdown links across member boundaries when run at the workspace root. Member-local command runs remain intentionally local: they do not silently resolve sibling aliases without the workspace root context.
- [§FS-init.2.3.4.15](functional-spec/FS-init.md#23415-workspace-members) / [§RM-init-workspace-members](roadmap.md#rm-init-workspace-members-init-mentions-workspace-members): `grund init` now emits a `### Workspace members` section in the generated `AGENTS.md` whenever the effective `.agents/grund.toml` declares `[workspace]`. The section lists every resolved project (root + members, subject to `include_root`) sorted by alias, with one discoverability line — `Cross-project citations use §alias/<ID>.` — and uniform `` `alias` → [path](path) `` bullets. Members whose `AGENTS.md` does not yet exist are marked `*(not yet initialized)*` and link to the member root rather than a 404. `init` invoked inside a member walks up to the workspace root so the same section appears in the member's `AGENTS.md`, with link paths recomputed relative to the member.
- [§FS-init.1](functional-spec/FS-init.md#1-inputs) / [§FS-init.2.2](functional-spec/FS-init.md#22-stdout--stderr): `grund init --dry-run` previews a run without writing any file. Every line a real run would print as `wrote `, `appended `, or `updated ` is reported with the `would-write `, `would-append `, or `would-update ` prefix instead; `exists ` lines and the `next:` block are unchanged.
- [§FS-init.1](functional-spec/FS-init.md#1-inputs) / [§FS-init.2.1](functional-spec/FS-init.md#21-files-written-updated-or-left-in-place): `grund init` now ships with companion entrypoints for Cursor (`.cursor/rules/grund.mdc`, plus the legacy `.cursorrules`), Windsurf (`.windsurfrules`), and Zed (`.rules`), each with a matching `--cursor` / `--windsurf` / `--zed` flag. `.cursor/` and `.zed/` trigger automatic alias creation; `.windsurfrules` is existing-file-only in automatic mode because there is no workspace directory to key off. `.rules` is workspace- or flag-gated only — its filename is too generic to attribute to Zed by file existence alone.

### Changed

- [§FS-init.1](functional-spec/FS-init.md#1-inputs) / [§FS-init.2.1](functional-spec/FS-init.md#21-files-written-updated-or-left-in-place) / [§FS-init.2.3](functional-spec/FS-init.md#23-generated-agent-entrypoints): `grund init` now preserves an existing repo's agent-entrypoint choice by default. A repo with only `CLAUDE.md` gets `CLAUDE.md` updated and no new `AGENTS.md`; if no known entrypoint exists, `.claude/` creates `CLAUDE.md` and `.claude/CLAUDE.md`, `.gemini/` creates `GEMINI.md`, `.cursor/` creates `.cursor/rules/grund.mdc`, `.zed/` creates `.rules`, and otherwise `AGENTS.md` is the fallback. Explicit flags (`--agents-md`, `--claude`, `--gemini`, `--copilot`, `--cursor`, `--windsurf`, `--zed`) create or update multiple requested entrypoints. `AGENTS.override.md`, `.github/copilot-instructions.md`, `.cursorrules`, and `.windsurfrules` remain automatic existing-file-only.
- [§FS-init.2.2](functional-spec/FS-init.md#22-stdout--stderr): `grund init` now suppresses the trailing `next:` guidance block when every reported path is `exists ` — the user already has a complete grund setup, so there is no next step to teach.

### Removed

- [§FS-init.1](functional-spec/FS-init.md#1-inputs): `grund init --append` is removed. It was an explicit no-op flag that only stated the default existing-entrypoint behavior; scripts that passed it no longer need it. The `--codex` alias of `--agents-md` is also removed — Codex uses `AGENTS.md` (the canonical fallback), not a Codex-specific file, so the alias suggested an asymmetry that did not exist. `--agents-md` remains the single flag name.

### Fixed

- [§FS-distribution.4](functional-spec/FS-distribution.md#4-release-process): the minor and patch release-bump helpers now update the checked `grund --version` e2e fixture together with the Cargo manifests and lockfile, so the version bump commit they push to `main` stays e2e-clean before it dispatches `release.yml`.

## 3. Older releases

- [0.1.0](changelog/0.1.0.md) — 2026-05-14: first published release and baseline CLI surface.

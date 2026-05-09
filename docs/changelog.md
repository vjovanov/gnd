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

## 3. [0.0.0] — 2026-05-08

Initial commit. Scheme, e2e fixtures, and harness; no published binary yet.

## 4. Older releases

None yet. When 0.0.0 is no longer the latest, it moves to `docs/changelog/0.0.0.md` and is linked from here.

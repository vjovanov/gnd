# Functional spec

This is the external behavior of `gnd` — *what* it does, not how it's built. Each spec lives in its own file. The H1 of that file declares an `FS-NNN-<slug>` ID, and the body is its contract. Anywhere else in the tree, a citation like `§FS-NNN-<slug>.<section>` resolves back into one of these files.

## CLI commands

The subcommands a user runs on the command line.

- [§FS-check](FS-check.md) — validates every reference in a repo
- [§FS-show](FS-show.md) — reads a single declaration body by ID
- [§FS-fmt](FS-fmt.md) — normalizes references in bulk
- [§FS-init](FS-init.md) — bootstraps a new `gnd`-conformant repo
- [§FS-name](FS-name.md) — proposes IDs for new declarations

## IDE plugins

The editor surface — a parallel front-end to the CLI.

- [§FS-ide-plugins](FS-ide-plugins.md) — first-party editor integrations

## Packaging

How `gnd` is shipped.

- [§FS-distribution](FS-distribution.md) — ships on cargo, npm, and PyPI with a native API on each

## Cross-cutting

Behavior every subcommand inherits.

- [§FS-errors](FS-errors.md) — the shape and style of every message `gnd` prints

## Configuration and scope

- [§FS-config](FS-config.md) — reads a TOML config file under `.agents/`
- [§FS-non-goals](FS-non-goals.md) — what `gnd` will deliberately not do

---

This index is navigational only. Citations should target the spec ID directly, never this file.

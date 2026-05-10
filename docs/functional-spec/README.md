# Functional spec

This is the external behavior of `gnd` — *what* it does, not how it's built. Each spec lives in its own file. The H1 of that file declares an `FS-NNN-<slug>` ID, and the body is its contract. Anywhere else in the tree, a citation like `§FS-NNN-<slug>.<section>` resolves back into one of these files.

## CLI commands

The subcommands a user runs on the command line.

- [§FS-check](§FS-check.md) — validates every reference in a repo
- [§FS-show](§FS-show.md) — reads a single declaration body by ID
- [§FS-list](§FS-list.md) — lists every declared ID (the ID catalog)
- [§FS-refs](§FS-refs.md) — lists every citation of an ID
- [§FS-fmt](§FS-fmt.md) — normalizes references in bulk
- [§FS-init](§FS-init.md) — bootstraps a new `gnd`-conformant repo
- [§FS-name](§FS-name.md) — proposes IDs for new declarations
- [§FS-completions](§FS-completions.md) — shell completion for declared IDs

## Editor integration

The editor surface — an optional LSP server that any LSP-aware editor can talk to. No first-party per-editor plugins ship; configuration is the user's one-time work.

- [§FS-lsp](§FS-lsp.md) — optional LSP server (`gnd-lsp`)

## Packaging

How `gnd` is shipped.

- [§FS-distribution](§FS-distribution.md) — ships on cargo, npm, and PyPI with a native API on each

## Cross-cutting

Behavior every subcommand inherits.

- [§FS-cli](§FS-cli.md) — the command-line surface: default subcommand, `--version`/`--help`, exit-code mapping
- [§FS-errors](§FS-errors.md) — the shape and style of every message `gnd` prints

## Configuration and scope

- [§FS-config](§FS-config.md) — reads a TOML config file under `.agents/`
- [§FS-non-goals](§FS-non-goals.md) — what `gnd` will deliberately not do

---

This index is navigational only. Citations should target the spec ID directly, never this file.

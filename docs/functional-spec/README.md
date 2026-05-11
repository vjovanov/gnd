# Functional spec

This is the external behavior of `grund` — *what* it does, not how it's built. Each spec lives in its own file. The H1 of that file declares an `FS-<slug>` ID, and the body is its contract. Anywhere else in the tree, a citation like `§FS-<slug>.<section>` resolves back into one of these files.

## CLI commands

The subcommands a user runs on the command line.

- [§FS-check](FS-check.md#fs-check-grund-validates-every-reference-in-a-repo) — validates every reference in a repo
- [§FS-show](FS-show.md#fs-show-grund-reads-a-single-declaration-body-by-id) — reads a single declaration body by ID
- [§FS-list](FS-list.md#fs-list-grund-lists-every-declared-id) — lists every declared ID (the ID catalog)
- [§FS-refs](FS-refs.md#fs-refs-grund-lists-every-citation-of-an-id) — lists every citation of an ID
- [§FS-cover](FS-cover.md#fs-cover-grund-groups-citations-by-scanned-file) — groups citations by scanned file
- [§FS-fmt](FS-fmt.md#fs-fmt-grund-normalizes-references-in-bulk) — normalizes references in bulk
- [§FS-init](FS-init.md#fs-init-grund-bootstraps-a-new-grund-conformant-repo) — bootstraps a new `grund`-conformant repo
- [§FS-id](FS-id.md#fs-id-grund-proposes-ids-for-new-declarations) — proposes IDs for new declarations
- [§FS-completions](FS-completions.md#fs-completions-grund-completes-declared-ids-in-shells) — shell completion for declared IDs

## Editor integration

The editor surface — an optional LSP server that any LSP-aware editor can talk to. No first-party per-editor plugins ship; configuration is the user's one-time work.

- [§FS-lsp](FS-lsp.md#fs-lsp-grund-will-ship-an-optional-lsp-server) — optional LSP server (`grund-lsp`)

## Packaging

How `grund` is shipped.

- [§FS-distribution](FS-distribution.md#fs-distribution-grund-distribution-targets) — ships on cargo, npm, and PyPI with a native API on each

## Cross-cutting

Behavior every subcommand inherits.

- [§FS-cli](FS-cli.md#fs-cli-grunds-command-line-surface-conventions) — the command-line surface: default subcommand, `--version`/`--help`, exit-code mapping
- [§FS-errors](FS-errors.md#fs-errors-grund-emits-messages-in-fixed-shapes) — the shape and style of every message `grund` prints

## Configuration and scope

- [§FS-config](FS-config.md#fs-config-grund-reads-a-toml-config-file-under-agents) — reads a TOML config file under `.agents/`
- [§FS-non-goals](FS-non-goals.md#fs-non-goals-what-grund-will-deliberately-not-do) — what `grund` will deliberately not do

---

This index is navigational only. Citations should target the spec ID directly, never this file.

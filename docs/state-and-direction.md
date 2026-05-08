# State and direction

## 1. Where we are now

A working Rust prototype lives in `src/main.rs`. It implements FS-001-check end-to-end: walks a tree, identifies declarations and citations, and reports dangling references, missing sections, duplicate declarations, and broken inline-spec stubs. It does not yet have an `e2e/` corpus and is not yet wired to CI.

The repo dogfoods its own scheme: this `docs/` tree is `gnd`-conformant and is intended to pass `gnd .` once the e2e corpus is in place.

## 2. Where we are going

In rough priority order:

1. **E2E corpus.** Build out `e2e/` with positive and negative fixtures covering each rule. Wire CI.
2. **FS-002-show.** Implement the `gnd show <ID>` subcommand — read a single declaration body, with optional section path and `--head` mode. This is what makes `gnd` useful as an *agent retrieval* tool, not just a checker.
3. **FS-006-config.** Implement `gnd.toml` parsing, validation, and the `gnd config show` / `gnd config validate` subcommands. Once this lands, every other knob in the system becomes overridable.
4. **DF-001-reference-marker / FS-005-fmt.** Add scanner support for the `§`-marked citation form. Implement `gnd fmt` for trigger-to-marker normalization. Make the marker optional (default) and strict (opt-in via `gnd.toml`).
5. **FS-004-distribution.** Restructure into a workspace: `gnd-core` (library), `gnd-cli` (binary), `gnd-node` (napi-rs binding for npm), `gnd-py` (PyO3 binding for PyPI). Set up CI to publish to all three registries.
6. **FS-003-ide-plugins.** First-party integrations for VSCode, IntelliJ IDEA, Vim/Neovim, and Emacs — all sharing a single `gnd-lsp` server. Live `$$ → §` transform on type. VSCode ships first; the rest follow as the LSP server stabilizes.

## 3. How to get there

- Each step lands as a separate, reviewable change with its own e2e tests.
- The scheme itself does not change without a corresponding decision under `docs/decisions/architectural/`.
- Self-host check stays green from day one: `cargo run -- .` against this repo must report zero errors after every change.

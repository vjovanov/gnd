# AR-core-module-layout: core implementation is split by category

The core implementation lives in `crates/grund-core/src/`, while the root `src/main.rs` is the thin published `grund` CLI entrypoint described by [§AR-bindings](AR-bindings.md#ar-bindings-target-shape-for-exposing-the-rust-engine-on-three-platforms). Inside `grund-core`, the source layout should match the same category boundaries the later LSP and binding frontends need. A single large crate root hides ownership and makes spec-to-code citations harder to place.

## 1. Module categories

`crates/grund-core/src/lib.rs` stays the crate entrypoint and re-export surface for the CLI-facing `main_entry`, while implementation code lives in smaller category files under `crates/grund-core/src/`.

The categories are:

- **model** — shared data types and tiny helpers used across commands.
- **config** — defaults, config discovery, config parsing, and TOML rendering helpers.
- **scanner** — tree walking, per-file scanning, e2e case discovery, and scan error handling.
- **checker** — validation rules that turn scanner findings into diagnostics.
- **output** — shared path formatting, JSON escaping, diagnostics, and report rendering.
- **show** — declaration and section retrieval/rendering.
- **refs** — reverse-reference query rendering.
- **cover** — per-file citation coverage query rendering.
- **list** — declaration catalog query rendering.
- **fmt** — citation normalization and cross-reference planning/writing.
- **id** — ID allocation, slug derivation, and ID rendering.
- **init** — scaffold/template rendering and managed agent-entrypoint updates.
- **completions** — shell completion scripts and dynamic completion helpers.
- **cli** — command dispatch, help text, exit-code mapping, and signal setup.

## 2. Refactor boundary

Splitting `crates/grund-core/src/lib.rs` by these categories is an architectural refactor only: it must not change CLI output, diagnostics, scan behavior, template bytes, or public entrypoints. The first split may keep crate-private implementation details crate-private; exposing a stable library API is a separate distribution step under [§AR-bindings](AR-bindings.md#ar-bindings-target-shape-for-exposing-the-rust-engine-on-three-platforms).

## 3. File size

Each implementation file under `src/` stays below 500 lines of code. If a category grows past that limit, split it into smaller category subfiles, or into a category directory with submodules, rather than letting a new monolith form.

## 4. Citation placement

Code moved into a category file keeps the same behavior citations it carried before. When a whole category implements an architectural behavior, the file or module-level comment may cite this spec; narrower functional clauses remain cited on the specific function or branch that implements them.

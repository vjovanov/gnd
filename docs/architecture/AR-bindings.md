# AR-bindings: target shape for exposing the Rust engine on three platforms

Implements the planned distribution shape in [§FS-distribution](../functional-spec/FS-distribution.md#fs-distribution-grund-distribution-targets). Target state: the repo is a Cargo workspace with one core library and four frontends — three for batch use (CLI, Node, Python) and one for editor use (LSP). The release-blocking boundary is now in place: `grund-core` is the shared engine crate, and `crates/grund-cli` is the published Cargo package named `grund`. The later frontend crates (`grund-lsp`, `grund-node`, `grund-py`) build on that boundary.

## 1. Target workspace layout

Current shipped split:

```
grund/
├── Cargo.toml          # virtual workspace root
├── crates/
│   ├── grund-core/     # scanner + checker + show + fmt + config + public Rust API
│   └── grund-cli/      # package `grund`; binary entrypoint, help, and top-level dispatch
├── docs/
└── e2e/
```

This split keeps CLI behavior byte-identical while giving `grund-lsp` and the language bindings a library package they can depend on. `grund-core` exposes the embedding API (`check`, `show`, `Report`, `Findings`, `ShowOpts`) and still carries a small set of command-adapter functions for the existing CLI surfaces during the transition; the user-facing binary, help text, version handling, SIGPIPE setup, and top-level command dispatch live in `grund-cli`.

Final frontend layout:

```
grund/
├── crates/
│   ├── grund-core/   # the engine: scanner + checker + show + fmt + config. Pure Rust. No I/O policy.
│   ├── grund-cli/    # the CLI binary. Command parsing, exit codes, terminal formatting. Published to cargo as `grund`.
│   ├── grund-lsp/    # the LSP server binary. Speaks LSP over stdio. Published as `grund-lsp` on every registry.
│   ├── grund-node/   # napi-rs binding. Published to npm as `grund-cli` (with the prebuilt CLI binary).
│   └── grund-py/     # PyO3 binding. Published to PyPI as `grund`.
├── docs/
└── e2e/
```

All four frontend crates depend on `grund-core` and only on `grund-core` for engine logic. None depend on each other. This is the property that lets [§DA-lsp-optional](../decisions/architectural/DA-lsp-optional.md#da-lsp-optional-lsp-server-ships-as-a-separate-optional-binary) hold: `grund-cli`'s dependency tree contains no async runtime, no JSON-RPC machinery, and no LSP types, because none of those reach `grund-core`.

## 2. grund-core: the only place logic lives

Every check, every show, every regex, every walker invocation lives in `grund-core`. The crate exposes:

- `grund_core::scan(root: &Path) -> Result<Findings>`
- `grund_core::check(root: &Path) -> Result<Report>`
- `grund_core::show(id: &str, opts: ShowOpts) -> Result<ShowOutput>`
- `grund::refs(findings: &Findings, id: &str, section: Option<&str>) -> Vec<Citation>` ([§FS-refs](../functional-spec/FS-refs.md#fs-refs-grund-lists-every-citation-of-an-id))
- The `Findings`, `Declaration`, `Citation`, `Report` data types.

The embedding API returns data; callers decide what to do with it. CLI compatibility adapters remain inside `grund-core` until each subcommand has a stable data-returning API.

## 3. grund-cli: the CLI binary

The Cargo package named `grund`. It imports `grund-core`, owns the installed binary, prints help/version output, restores SIGPIPE, and routes top-level commands to the command adapters. This is what `cargo install grund` produces and what the npm/PyPI packages wrap. Synchronous; no async runtime, no LSP types, no JSON-RPC.

## 4. grund-lsp: the LSP server binary

Speaks LSP over stdio (per [§AR-lsp.4](AR-lsp.md#4-transport)). Imports `grund-core` for scan/check/show/fmt; imports `tower-lsp` (or equivalent) plus `tokio` for the protocol surface. Publishes as `grund-lsp` on every registry per [§FS-distribution.1](../functional-spec/FS-distribution.md#1-targets) and [§DA-lsp-optional](../decisions/architectural/DA-lsp-optional.md#da-lsp-optional-lsp-server-ships-as-a-separate-optional-binary). Independent of `grund-cli` — neither pulls the other in. The full architecture lives in [§AR-lsp](AR-lsp.md#ar-lsp-how-the-lsp-server-is-built).

## 5. grund-node: the napi-rs binding

Re-exports the same operations as Promise-returning Node functions. The npm `grund-cli` package ships:

- The `grund` binary (so `npx grund-cli` works).
- A small JS module re-exporting `check`, `show`, etc. against the napi binding (so `import { check } from 'grund-cli'` works).

Prebuilt platform binaries are uploaded as separate npm packages (`@grund-cli/linux-x64`, etc.) per the `napi-rs` convention; the main package picks the right one at install time.

## 6. grund-py: the PyO3 binding

Same operations, exposed as Python functions. Built and packaged via `maturin`. Wheels are produced by `cibuildwheel` in CI for each release. Source distributions are also uploaded so unsupported platforms can build from source.

## 7. Why this shape

- **One source of truth for behavior.** Bug fixes and new rules land in `grund-core` and reach all three ecosystems on the next release.
- **No re-implementation.** Neither Node nor Python developers need to maintain a parallel parser or a parallel rule set.
- **Fast everywhere.** The compiled engine is the same in all three. The bindings add only a thin marshalling layer.
- **Independent release cadence per crate when needed.** A Node-only fix in `grund-node` does not require a `grund-core` version bump.

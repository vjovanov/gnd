# AR-bindings: target shape for exposing the Rust engine on three platforms

Implements the planned distribution shape in [§FS-distribution](../functional-spec/FS-distribution.md#fs-distribution-grund-distribution-targets). Target state: the repo is a Cargo workspace with one core library and four frontends — three for batch use (CLI, Node, Python) and one for editor use (LSP). The split starts with the release-blocking boundary: `grund-core` is a real workspace crate, and the published `grund` CLI package depends on it. The later frontend crates (`grund-lsp`, `grund-node`, `grund-py`) build on that boundary.

## 1. Target workspace layout

Current shipped split:

```
grund/
├── Cargo.toml          # workspace root and published `grund` CLI package
├── src/main.rs         # thin binary entrypoint; delegates to `grund-core`
├── crates/
│   └── grund-core/     # scanner + checker + show + fmt + config + shared CLI dispatcher
├── docs/
└── e2e/
```

This first step keeps the existing CLI behavior byte-identical while giving `grund-lsp` a package it can depend on. `grund-core` temporarily still carries the shared CLI dispatcher and text/JSON rendering helpers because those are the stable API the current `grund` binary calls. The follow-up `grund-cli` crate move is a cleanup: it moves argument parsing, `println!`/`eprintln!`, and exit-code mapping out of `grund-core` without changing the public CLI.

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

- `grund::scan(root: &Path) -> Findings`
- `grund::check(findings: &Findings, root: &Path) -> Report`
- `grund::show(id: &str, opts: ShowOpts) -> Result<String>`
- `grund::refs(findings: &Findings, id: &str, section: Option<&str>) -> Vec<Citation>` ([§FS-refs](../functional-spec/FS-refs.md#fs-refs-grund-lists-every-citation-of-an-id))
- The `Findings`, `Declaration`, `Citation`, `Report` data types.

The crate has no `println!`, no `eprintln!`, no `process::exit`. It returns data; callers decide what to do with it.

## 3. grund-cli: the CLI binary

Argument parsing (likely `clap`), terminal formatting, exit-code mapping. Imports `grund-core` and translates results into stdout/stderr text. This is what `cargo install grund` produces and what the npm package wraps. Synchronous; no async runtime, no LSP types, no JSON-RPC.

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

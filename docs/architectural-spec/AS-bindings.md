# AS-bindings: target shape for exposing the Rust engine on three platforms

Implements the planned distribution shape in §FS-distribution. Target state: the repo is a Cargo workspace with one core library and four frontends — three for batch use (CLI, Node, Python) and one for editor use (LSP). The current implementation is still a single Rust crate; this architectural spec describes the split that must happen before the LSP and language bindings ship.

## 1. Target workspace layout

```
gnd/
├── crates/
│   ├── gnd-core/   # the engine: scanner + checker + show + fmt + config. Pure Rust. No I/O policy.
│   ├── gnd-cli/    # the CLI binary. Command parsing, exit codes, terminal formatting. Published to cargo as `gnd`.
│   ├── gnd-lsp/    # the LSP server binary. Speaks LSP over stdio. Published as `gnd-lsp` on every registry.
│   ├── gnd-node/   # napi-rs binding. Published to npm as `gnd-cli` (with the prebuilt CLI binary).
│   └── gnd-py/     # PyO3 binding. Published to PyPI as `gnd`.
├── docs/
└── e2e/
```

All four frontend crates depend on `gnd-core` and only on `gnd-core` for engine logic. None depend on each other. This is the property that lets §DA-lsp-optional hold: `gnd-cli`'s dependency tree contains no async runtime, no JSON-RPC machinery, and no LSP types, because none of those reach `gnd-core`.

## 2. gnd-core: the only place logic lives

Every check, every show, every regex, every walker invocation lives in `gnd-core`. The crate exposes:

- `gnd::scan(root: &Path) -> Findings`
- `gnd::check(findings: &Findings, root: &Path) -> Report`
- `gnd::show(id: &str, opts: ShowOpts) -> Result<String>`
- `gnd::refs(findings: &Findings, id: &str, section: Option<&str>) -> Vec<Citation>` (§FS-refs)
- The `Findings`, `Declaration`, `Citation`, `Report` data types.

The crate has no `println!`, no `eprintln!`, no `process::exit`. It returns data; callers decide what to do with it.

## 3. gnd-cli: the CLI binary

Argument parsing (likely `clap`), terminal formatting, exit-code mapping. Imports `gnd-core` and translates results into stdout/stderr text. This is what `cargo install gnd` produces and what the npm package wraps. Synchronous; no async runtime, no LSP types, no JSON-RPC.

## 4. gnd-lsp: the LSP server binary

Speaks LSP over stdio (per §AS-lsp.4). Imports `gnd-core` for scan/check/show/fmt; imports `tower-lsp` (or equivalent) plus `tokio` for the protocol surface. Publishes as `gnd-lsp` on every registry per §FS-distribution.1 and §DA-lsp-optional. Independent of `gnd-cli` — neither pulls the other in. The full architecture lives in §AS-lsp.

## 5. gnd-node: the napi-rs binding

Re-exports the same operations as Promise-returning Node functions. The npm `gnd-cli` package ships:

- The `gnd` binary (so `npx gnd-cli` works).
- A small JS module re-exporting `check`, `show`, etc. against the napi binding (so `import { check } from 'gnd-cli'` works).

Prebuilt platform binaries are uploaded as separate npm packages (`@gnd-cli/linux-x64`, etc.) per the `napi-rs` convention; the main package picks the right one at install time.

## 6. gnd-py: the PyO3 binding

Same operations, exposed as Python functions. Built and packaged via `maturin`. Wheels are produced by `cibuildwheel` in CI for each release. Source distributions are also uploaded so unsupported platforms can build from source.

## 7. Why this shape

- **One source of truth for behavior.** Bug fixes and new rules land in `gnd-core` and reach all three ecosystems on the next release.
- **No re-implementation.** Neither Node nor Python developers need to maintain a parallel parser or a parallel rule set.
- **Fast everywhere.** The compiled engine is the same in all three. The bindings add only a thin marshalling layer.
- **Independent release cadence per crate when needed.** A Node-only fix in `gnd-node` does not require a `gnd-core` version bump.

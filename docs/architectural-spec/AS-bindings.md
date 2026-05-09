# AS-bindings: how the same Rust engine is exposed on three platforms

Implements FS-distribution. The repo is a Cargo workspace with one core library and three frontends.

## 1. Workspace layout

```
gnd/
├── crates/
│   ├── gnd-core/   # the engine: scanner + checker + show. Pure Rust. No I/O policy.
│   ├── gnd-cli/    # the binary. Command parsing, exit codes, terminal formatting.
│   ├── gnd-node/   # napi-rs binding. Published to npm as gnd-cli (with the binary).
│   └── gnd-py/     # PyO3 binding. Published to PyPI as gnd.
├── docs/
└── e2e/
```

## 2. gnd-core: the only place logic lives

Every check, every show, every regex, every walker invocation lives in `gnd-core`. The crate exposes:

- `gnd::scan(root: &Path) -> Findings`
- `gnd::check(findings: &Findings, root: &Path) -> Report`
- `gnd::show(id: &str, opts: ShowOpts) -> Result<String>`
- The `Findings`, `Declaration`, `Citation`, `Report` data types.

The crate has no `println!`, no `eprintln!`, no `process::exit`. It returns data; callers decide what to do with it.

## 3. gnd-cli: the binary

Argument parsing (likely `clap`), terminal formatting, exit-code mapping. Imports `gnd-core` and translates results into stdout/stderr text. This is what `cargo install gnd` produces and what the npm package wraps.

## 4. gnd-node: the napi-rs binding

Re-exports the same operations as Promise-returning Node functions. The npm `gnd-cli` package ships:

- The `gnd` binary (so `npx gnd-cli` works).
- A small JS module re-exporting `check`, `show`, etc. against the napi binding (so `import { check } from 'gnd-cli'` works).

Prebuilt platform binaries are uploaded as separate npm packages (`@gnd-cli/linux-x64`, etc.) per the `napi-rs` convention; the main package picks the right one at install time.

## 5. gnd-py: the PyO3 binding

Same operations, exposed as Python functions. Built and packaged via `maturin`. Wheels are produced by `cibuildwheel` in CI for each release. Source distributions are also uploaded so unsupported platforms can build from source.

## 6. Why this shape

- **One source of truth for behavior.** Bug fixes and new rules land in `gnd-core` and reach all three ecosystems on the next release.
- **No re-implementation.** Neither Node nor Python developers need to maintain a parallel parser or a parallel rule set.
- **Fast everywhere.** The compiled engine is the same in all three. The bindings add only a thin marshalling layer.
- **Independent release cadence per crate when needed.** A Node-only fix in `gnd-node` does not require a `gnd-core` version bump.

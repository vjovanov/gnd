# FS-004-distribution: gnd ships on cargo, npm, and PyPI with a native API on each

`gnd` is written in Rust but is distributed on **all three** major language ecosystems, with idiomatic API bindings on each. The check engine is a single shared library; the surfaces differ. Serves G-004-multi-language and G-005-friendliness-first.

## 1. Targets

| Registry | Package name        | Contents                                                                  |
|----------|---------------------|---------------------------------------------------------------------------|
| cargo    | `gnd`               | Library crate (`gnd-core`) + binary (`gnd`).                              |
| npm      | `gnd-cli`           | Prebuilt binary + thin Node API surface (via `napi-rs`).                  |
| PyPI     | `gnd`               | Prebuilt wheel + Python API surface (via `PyO3` / `maturin`).             |

The PyPI package name `gnd` is free; using it avoids a `pip install gnd-cli` mismatch with the Python convention. The npm package uses `gnd-cli` because the unscoped `gnd` is held by an unrelated dormant package (see DA-001-reference-checker-name).

## 2. CLI parity

The `gnd` binary behaves identically regardless of how it was installed: the same flags, the same exit codes, the same byte-for-byte report format. Users on any platform who run `gnd .` get the same answer.

## 3. API surfaces

Each binding exposes the same conceptual operations as the CLI subcommands, plus a programmatic check-and-iterate path so the engine can be embedded inside test runners and editor servers.

### 3.0 Language-neutral data shapes

Every binding returns the same data, only spelled idiomatically. The conceptual shapes are:

```
Report {
  errors:   [Finding]
  warnings: [Finding]
}

Finding {
  severity: "error" | "warning"
  code:     "dangling" | "missing-section" | "duplicate" | "broken-stub" | "unused" | "io"
  path:     string         // relative to config root (FS-006-config.3.6)
  line:     u32            // 1-indexed
  message:  string         // the human-readable text
  sites:    [{ path, line }]?  // present for multi-site findings (e.g. duplicates)
}

ShowOpts {
  section: string?    // dotted section path, e.g. "3.1.2"
  head:    bool       // mutually exclusive with `full`
  full:    bool
  format:  "text" | "md" | "json"
}
```

These fields are normative. The byte-for-byte JSON form emitted by `gnd --format=json` and consumed by IDE/agent integrations follows the same shape and is the cross-binding equivalence test (G-004.1).

### 3.1 Rust (`gnd-core` crate)

```rust
let report = gnd::check(&path)?;
let body = gnd::show("FS-001-check", ShowOpts::default())?;
```

`Report` and the underlying `Findings` are exposed as plain data structures so callers can iterate, filter, or render their own output.

### 3.2 Node (`gnd-cli` npm package)

```js
import { check, show } from 'gnd-cli';

const report = await check('./repo');
const body = await show('FS-001-check', { head: true });
```

The Node binding is built with `napi-rs`. Native binaries are prebuilt for the platforms covered by `napi-rs` (macOS arm64/x64, Linux x64/arm64, Windows x64). Source builds are supported as a fallback.

### 3.3 Python (`gnd` PyPI package)

```python
from gnd import check, show

report = check("./repo")
body = show("FS-001-check", head=True)
```

The Python binding is built with `PyO3` and packaged with `maturin`. Wheels are built for CPython 3.10+ across the platforms covered by `cibuildwheel`.

## 4. Release process

A single release tag triggers parallel jobs that:

1. Publish the crate to crates.io.
2. Build per-platform Node binaries and publish `gnd-cli` to npm.
3. Build per-platform Python wheels and publish `gnd` to PyPI.

All three artifacts must succeed for a release to be considered complete. Versions are kept in lockstep across registries.

## 5. What we do not promise

- 100% identical APIs across languages. Each binding is idiomatic to its host (camelCase for Node, snake_case for Python, `Result<T,E>` for Rust). The *behavior* is identical; the surface fits each ecosystem.
- Stable ABI for the C-level FFI. Bindings link against the Rust core at compile time; we do not ship a separate C library.

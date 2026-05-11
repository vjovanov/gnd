# FS-distribution: gnd distribution targets

`gnd` is written in Rust; the target distribution is **all three** major language ecosystems — cargo, npm, and PyPI — with idiomatic API bindings on each. The check engine stays a single shared library; only the surfaces differ. Today the Cargo CLI is implemented and installable from git; registry publication, the npm and PyPI bindings, and the optional `gnd-lsp` server are tracked in `docs/roadmap.md` and gated by §RM-distribution-naming. Serves §G-multi-language and §G-friendliness-first.

## 1. Targets

| Registry | Package name        | Contents                                                                  |
|----------|---------------------|---------------------------------------------------------------------------|
| cargo    | `gnd`               | Library crate (`gnd-core`) + binary (`gnd`).                              |
| cargo    | `gnd-lsp`           | Optional LSP server binary (§FS-lsp). Depends on `gnd-core`.              |
| npm      | `gnd-cli`           | Prebuilt CLI binary + thin Node API surface (via `napi-rs`).              |
| npm      | `gnd-lsp`           | Optional LSP server binary, prebuilt per platform.                        |
| PyPI     | `gnd-cli`           | Prebuilt CLI wheel + Python API surface (via `PyO3` / `maturin`).         |
| PyPI     | `gnd-lsp`           | Optional LSP server, distributed via wheel (`pipx install gnd-lsp`).      |

The PyPI name `gnd` is held by an unrelated package, so PyPI uses the explicit alternate `gnd-cli` per §DA-pypi-package-name. The installed binary is still `gnd`, and the Python import module is still intended to be `gnd`. The npm package also uses `gnd-cli` because the unscoped `gnd` is likewise held by an unrelated dormant package (see §DA-reference-checker-name). The remaining registry slots — including `gnd-lsp` — are re-verified by §RM-distribution-naming before first publish.

The CLI install on each registry does **not** transitively pull in `gnd-lsp` — they are independent published packages, per §DA-lsp-optional. A user who only runs `gnd check` in CI installs the CLI alone; a user who wants editor integration installs `gnd-lsp` separately and configures their editor to launch it (§FS-lsp.2).

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
  code:     "dangling" | "missing-section" | "duplicate" | "broken-stub" | "unused" | "agents-init" | "io"
  path:     string?        // relative to config root (FS-config.3.6); null for a CLI-level error
  line:     u32?           // 1-indexed; null for a file-level finding with no line (e.g. an unreadable file, FS-check.2)
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

These fields are normative. The byte-for-byte JSON form emitted by `gnd --format=json` and consumed by IDE/agent integrations follows the same shape and is the cross-binding equivalence test (§G-multi-language.3).

### 3.1 Rust (`gnd-core` crate)

```rust
let report = gnd::check(&path)?;
let body = gnd::show("FS-check", ShowOpts::default())?;
```

`Report` and the underlying `Findings` are exposed as plain data structures so callers can iterate, filter, or render their own output.

### 3.2 Node (`gnd-cli` npm package)

```js
import { check, show } from 'gnd-cli';

const report = await check('./repo');
const body = await show('FS-check', { head: true });
```

The Node binding is built with `napi-rs`. Native binaries are prebuilt for the platforms covered by `napi-rs` (macOS arm64/x64, Linux x64/arm64, Windows x64). Source builds are supported as a fallback.

### 3.3 Python (`gnd-cli` PyPI package)

```python
from gnd import check, show

report = check("./repo")
body = show("FS-check", head=True)
```

The Python binding is built with `PyO3` and packaged with `maturin`. Wheels are built for CPython 3.10+ across the platforms covered by `cibuildwheel`. The distribution package is named `gnd-cli`; the import module is `gnd` (§DA-pypi-package-name).

## 4. Release process

A single release tag triggers parallel jobs that:

1. Publish the `gnd-core`, `gnd`, and `gnd-lsp` crates to crates.io (in dependency order: `gnd-core` first).
2. Build per-platform Node binaries and publish `gnd-cli` and `gnd-lsp` to npm.
3. Build per-platform Python wheels and publish `gnd-cli` and `gnd-lsp` to PyPI.

All artifacts must succeed for a release to be considered complete. Versions across the CLI and the LSP move together within a release; `gnd-lsp` pins its `gnd-core` dependency to the same version the CLI ships, so a CLI/LSP version mismatch in editors is structurally impossible.

## 5. What we do not promise

- 100% identical APIs across languages. Each binding is idiomatic to its host (camelCase for Node, snake_case for Python, `Result<T,E>` for Rust). The *behavior* is identical; the surface fits each ecosystem.
- Stable ABI for the C-level FFI. Bindings link against the Rust core at compile time; we do not ship a separate C library.

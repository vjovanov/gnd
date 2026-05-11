# FS-distribution: grund distribution targets

`grund` is written in Rust; the target distribution is **all three** major language ecosystems — cargo, npm, and PyPI — with idiomatic API bindings on each. The check engine stays a single shared library; only the surfaces differ. Today the Cargo CLI is implemented and installable from git; registry publication, the npm and PyPI bindings, and the optional `grund-lsp` server are tracked in `docs/roadmap.md` and gated by [§RM-distribution-naming](../roadmap.md#rm-distribution-naming-verify-package-names-before-first-publish). Serves [§GOAL-multi-language](../goals/goals.md#goal-multi-language-same-engine-three-platforms) and [§GOAL-friendliness-first](../goals/goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible).

## 1. Targets

| Registry | Package name        | Contents                                                                  |
|----------|---------------------|---------------------------------------------------------------------------|
| cargo    | `grund`               | Library crate (`grund-core`) + binary (`grund`).                              |
| cargo    | `grund-lsp`           | Optional LSP server binary ([§FS-lsp](FS-lsp.md#fs-lsp-grund-will-ship-an-optional-lsp-server)). Depends on `grund-core`.              |
| npm      | `grund-cli`           | Prebuilt CLI binary + thin Node API surface (via `napi-rs`).              |
| npm      | `grund-lsp`           | Optional LSP server binary, prebuilt per platform.                        |
| PyPI     | `grund-cli`           | Prebuilt CLI wheel + Python API surface (via `PyO3` / `maturin`).         |
| PyPI     | `grund-lsp`           | Optional LSP server, distributed via wheel (`pipx install grund-lsp`).      |

On crates.io the crate is `grund` — the library (`grund-core`) plus the `grund` binary — alongside `grund-lsp`. On npm and PyPI the published CLI is `grund-cli`: one name that reads identically on both registries and as "the package that installs the `grund` command", with `grund-lsp` as the server slot on each. The installed binary is `grund` no matter how it was installed, and the Python import module is `grund`. The tool was renamed from its pre-release working title `gnd` to `grund` ([§DA-rename-to-grund](../decisions/architectural/DA-rename-to-grund.md#da-rename-to-grund-rename-gnd-to-grund-before-first-publish)); that rename also voids the registry-collision reasoning in [§DA-reference-checker-name](../decisions/architectural/DA-reference-checker-name.md#da-reference-checker-name-name-for-the-spec-reference-checker-tool) and [§DA-pypi-package-name](../decisions/architectural/DA-pypi-package-name.md#da-pypi-package-name-pypi-uses-gnd-cli-as-the-package-name), which were about the old name (`grund` itself is clean on crates.io and PyPI; the unscoped `grund` on npm is a dormant low-use squat). Every one of these names — and the still-unreserved LSP slots — is re-verified against the live registries by [§RM-distribution-naming](../roadmap.md#rm-distribution-naming-verify-package-names-before-first-publish) before the first publish, which may collapse the PyPI package to the bare `grund` if it is still free.

The CLI install on each registry does **not** transitively pull in `grund-lsp` — they are independent published packages, per [§DA-lsp-optional](../decisions/architectural/DA-lsp-optional.md#da-lsp-optional-lsp-server-ships-as-a-separate-optional-binary). A user who only runs `grund check` in CI installs the CLI alone; a user who wants editor integration installs `grund-lsp` separately and configures their editor to launch it ([§FS-lsp.2](FS-lsp.md#2-installation-and-lifecycle)).

## 2. CLI parity

The `grund` binary behaves identically regardless of how it was installed: the same flags, the same exit codes, the same byte-for-byte report format. Users on Linux, macOS, and Windows who run `grund .` against the same repo get the same answer.

CLI reports use repo-relative logical paths with `/` as the separator, even on Windows. This applies to text reports, JSON fields, `sites`, `grund show` e2e fixture lists, stub-link targets, and generated cross-reference URLs; native platform paths may appear only in launch-time errors about paths outside the scanned repo, where there is no repo-relative path to print. The CI build/test matrix is the proof for this contract: every normal e2e case must pass on Linux, macOS, and Windows.

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
  code:     // a `check` finding — "dangling" | "missing-section" | "duplicate" | "broken-stub"
            //                   | "unused" | "ungrounded" | "agents-init" | "empty-scan" | "io"
            // — or, on a failed `grund show` query (FS-show.3, rendered with this same shape on stderr,
            //   path/line null) — "not-found" | "missing-section" | "broken-stub" | "ambiguous"
            //                   | "invalid-id" | "query-failed"
  path:     string?        // relative to config root (FS-config.3.6); null for a CLI-level error
  line:     u32?           // 1-indexed; null for a file-level finding with no line (e.g. an unreadable file, FS-check.2)
  message:  string         // the human-readable text
  sites:    [{ path, line }]?  // null for a single-site diagnostic; a list naming every site for a multi-site
                               // finding (a duplicate declaration) or an ambiguous-ID query failure
}

ShowOpts {
  section: string?    // dotted section path, e.g. "3.1.2"
  head:    bool       // mutually exclusive with `full`
  full:    bool
  format:  "text" | "md" | "json"
}
```

These fields are normative. The byte-for-byte JSON form emitted by `grund --format=json` and consumed by IDE/agent integrations follows the same shape and is the cross-binding equivalence test ([§GOAL-multi-language.3](../goals/goals.md#3-measurable)).

### 3.1 Rust (`grund-core` crate)

```rust
let report = grund::check(&path)?;
let body = grund::show("FS-check", ShowOpts::default())?;
```

`Report` and the underlying `Findings` are exposed as plain data structures so callers can iterate, filter, or render their own output.

### 3.2 Node (`grund-cli` npm package)

```js
import { check, show } from 'grund-cli';

const report = await check('./repo');
const body = await show('FS-check', { head: true });
```

The Node binding is built with `napi-rs`. Native binaries are prebuilt for the platforms covered by `napi-rs` (macOS arm64/x64, Linux x64/arm64, Windows x64). Source builds are supported as a fallback.

### 3.3 Python (`grund-cli` PyPI package)

```python
from grund import check, show

report = check("./repo")
body = show("FS-check", head=True)
```

The Python binding is built with `PyO3` and packaged with `maturin`. Wheels are built for CPython 3.10+ across the platforms covered by `cibuildwheel`. The distribution package is named `grund-cli`; the import module is `grund` ([§DA-pypi-package-name](../decisions/architectural/DA-pypi-package-name.md#da-pypi-package-name-pypi-uses-gnd-cli-as-the-package-name) — that record predates the rename and used the old name; [§RM-distribution-naming](../roadmap.md#rm-distribution-naming-verify-package-names-before-first-publish) re-confirms the live names).

## 4. Release process

A single release tag triggers parallel jobs that:

1. Publish the `grund-core`, `grund`, and `grund-lsp` crates to crates.io (in dependency order: `grund-core` first).
2. Build per-platform Node binaries and publish `grund-cli` and `grund-lsp` to npm.
3. Build per-platform Python wheels and publish `grund-cli` and `grund-lsp` to PyPI.

All artifacts must succeed for a release to be considered complete. Versions across the CLI and the LSP move together within a release; `grund-lsp` pins its `grund-core` dependency to the same version the CLI ships, so a CLI/LSP version mismatch in editors is structurally impossible.

The distributed `grund` binary is profile-guided-optimized: each release build runs `scripts/pgo-build.sh`, which builds an instrumented binary, runs the [§AR-benchmarks](../architecture/AR-benchmarks.md#ar-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) workload (the commands agents and CI invoke most) against `grund`'s own conformant tree to record a profile, then rebuilds against it. The rationale, and why the benchmark workload is also the PGO training corpus, is [§DA-pgo-release](../decisions/architectural/DA-pgo-release.md#da-pgo-release-distributed-binaries-are-pgo-built-trained-on-the-benchmark-workload). This is wired for the crates.io `grund` binary today and extends to the prebuilt npm and PyPI CLI binaries as those land. The manual **Pre-release checks** workflow must pass before publish; it runs the PGO build and self-checks the resulting release binary. PGO is not part of development builds or push/PR CI; it is a release-packaging step, and an explicit benchmarking step when comparing the optimized release artifact. A `cargo install grund` from source is LTO-optimized but not PGO'd — `cargo install` runs no custom build step — and is byte-for-byte behavior-identical to the distributed binary; only its performance differs.

## 5. What we do not promise

- 100% identical APIs across languages. Each binding is idiomatic to its host (camelCase for Node, snake_case for Python, `Result<T,E>` for Rust). The *behavior* is identical; the surface fits each ecosystem.
- Stable ABI for the C-level FFI. Bindings link against the Rust core at compile time; we do not ship a separate C library.

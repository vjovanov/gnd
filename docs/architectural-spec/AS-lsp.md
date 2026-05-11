# AS-lsp: how the LSP server is built

Implements [§FS-lsp](../functional-spec/FS-lsp.md). The LSP server is a separate crate (`gnd-lsp`) in the workspace defined by [§AS-bindings.1](AS-bindings.md#1-target-workspace-layout), depending only on `gnd-core`. It has no shared runtime with `gnd-cli`, no shared state with the bindings, and no own engine logic — everything it does delegates to `gnd-core`.

## 1. Crate boundary

`gnd-lsp` is a binary crate with one job: speak LSP over stdio and translate each request into a `gnd-core` call. The crate has:

- No scanner, no checker, no `show` extraction, no `fmt` planning. All four are imports from `gnd-core`.
- No `tokio`/`tower-lsp`/`lsp-types` references in `gnd-core`. The async runtime and the LSP machinery live entirely in `gnd-lsp`. `gnd-cli` continues to be synchronous and pulls none of this in.
- No filesystem walking outside what `gnd-core::scan` already does. The LSP server does not invent its own walker.

This is the architectural shape that lets the LSP be optional ([§DA-lsp-optional](../decisions/architectural/DA-lsp-optional.md)): the dependency cost stays in `gnd-lsp`, and a user installing only `gnd` (the CLI) pays none of it.

## 2. State

The server holds an in-memory `Findings` (the same struct [§AS-scanner.3](AS-scanner.md#3-output) produces) per workspace:

- On `initialize`, the server records the workspace root from the client's `rootUri`.
- On `initialized`, the server runs a full scan and stores the resulting `Findings`.
- On `textDocument/didChange`, the server updates the in-memory copy of the changed file (LSP delivers the new text), then re-runs the scan over the workspace.
- On `textDocument/didSave`, the server reconciles the in-memory copy against disk (handles cases where another tool wrote the file).
- On `workspace/didChangeWatchedFiles`, the server re-runs the scan to pick up creates and deletes the editor reported.

The `Findings` is the cache for everything else: hover, definition, and diagnostics all answer from it.

## 3. Scan strategy

### 3.1 Full re-scan on every change (v1)

Initial implementation: every `didChange` triggers `gnd-core::scan(workspace_root)` and a fresh `gnd-core::check`. This is simple and correct. Per [§G-fast-feedback.1](../goals/goals.md#1-performance-targets), a scan completes in under 100 ms on the gnd repo and under 1 s on a 10k-file repo — fast enough that a full re-scan per keystroke is invisible on small and medium projects, and acceptable per-save on large ones.

### 3.2 Incremental scan (v2, when budget breaks)

When the full-scan budget breaks (typically: large monorepos, slow disks, or per-keystroke debounce too tight), switch to incremental: rescan only the changed file and re-validate citations whose targets touch the changed file's declarations. This is the same gradient [§G-fast-feedback.2](../goals/goals.md#2-how-we-get-there) endorses for the CLI's parallel walk — incremental is added when the simple version stops winning, not before.

The incremental path keeps the single source of truth in `gnd-core::scan`; `gnd-lsp` adds a thin "what changed" diff over scan inputs and reuses the rest.

## 4. Transport

LSP over **stdio only**. No TCP, no Unix socket, no named pipe. Reasoning: stdio is what every LSP-aware editor expects by default, has no port-conflict surface, and avoids the need for any local listener that could be reached by another process. The server is invoked by the editor's LSP client as a child process and reads/writes JSON-RPC framed messages on stdin/stdout. Diagnostic logging goes to stderr in the LSP-canonical `[LEVEL] message` form; editors that surface server logs render it as-is.

## 5. Determinism and parity tests

The LSP must produce the same diagnostics for the same workspace state as `gnd check` does — byte-for-byte on the message text, position-for-position on the line numbers ([§FS-non-goals.13](../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-gnd-installs-disagree)). Parity is enforced by an e2e harness:

- A test driver spawns `gnd-lsp` as a child process, sends `initialize` with the workspace root pointing at an `e2e/cases/<id>/repo/` fixture, and asserts the published diagnostics match the case's `expected.stdout` and `expected.stderr` after format-translation (LSP diagnostic shape vs. CLI text shape).
- A second sweep sends `textDocument/hover` for each citation in the fixture and asserts the hover body matches `gnd show` for that ID.
- A third sweep sends `textDocument/definition` and asserts the resolved `path:line` matches the declaration recorded in the fixture's `Findings`.

This is what makes the LSP "the same engine with a different transport" rather than a parallel implementation that could drift.

## 6. What this does not contain

- No editor-specific code. Per [§FS-lsp](../functional-spec/FS-lsp.md) and [§FS-non-goals](../functional-spec/FS-non-goals.md), no first-party VSCode/IntelliJ/Vim/Emacs wrappers ship; this crate is the only editor-facing surface.
- No process supervision. The editor owns the lifecycle ([§FS-lsp.2.2](../functional-spec/FS-lsp.md#22-lifecycle)); `gnd-lsp` does not respawn itself, does not background, does not write a PID file.
- No telemetry, no auto-update, no crash reporter ([§FS-non-goals.11](../functional-spec/FS-non-goals.md#11-network-access-during-a-check) — no network I/O).

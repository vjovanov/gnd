# Architectural spec

Internals — *how* `gnd` is built. One file per spec; each H1 is the declaration of an `AS-<slug>` ID and the body is its contract. Citations from elsewhere in the tree (`§AS-<slug>.<section>`) resolve into these files.

An architectural spec may live inline in the class- or module-level doc-comment of the file it describes. A one-line stub here whose H1 is `# AS-<slug>: [<path>](<path>)` is **optional** — add it when you want the inline spec listed in this index alongside the file-form ones; omit it when the doc-comment alone is enough. `gnd show` resolves the ID either way; with a stub it follows the link and strips comment markers. See `§AS-scanner.4` for the supported doc-comment forms. `§AS-checker` is the worked example: it lives in the doc-comment of `fn check` in [`src/lib.rs`](../../src/lib.rs), with the one-line stub at [`AS-checker.md`](AS-checker.md).

| ID | Subject |
|---|---|
| [§AS-scanner](AS-scanner.md#as-scanner-how-gnd-discovers-declarations-and-citations) | how `gnd` discovers declarations and citations |
| [§AS-checker](../../src/lib.rs) | how `gnd` validates the scanner's findings — declared inline in `src/lib.rs` (stub: `AS-checker.md`) |
| [§AS-bindings](AS-bindings.md#as-bindings-target-shape-for-exposing-the-rust-engine-on-three-platforms) | how the same Rust engine is exposed on three platforms |
| [§AS-lsp](AS-lsp.md#as-lsp-how-the-lsp-server-is-built) | how the optional LSP server is built |
| [§AS-ci](AS-ci.md#as-ci-ci-mirrors-the-local-pre-commit-gate) | how CI mirrors the local pre-commit gate |
| [§AS-benchmarks](AS-benchmarks.md#as-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands) | the instruction-counting benchmark harness for the [§G-fast-feedback](../goals/goals.md#g-fast-feedback-gnd-must-be-as-fast-as-possible) budgets |

This index is navigational — citations should target the spec ID directly, never this file.

# Architectural spec

Internals — *how* `gnd` is built. One file per spec; each H1 is the declaration of an `AS-NNN-<slug>` ID and the body is its contract. Citations from elsewhere in the tree (`§AS-NNN-<slug>.<section>`) resolve into these files.

An architectural spec may live inline in the class- or module-level doc-comment of the file it describes. A one-line stub here whose H1 is `# AS-NNN-<slug>: [<path>](<path>)` is **optional** — add it when you want the inline spec listed in this index alongside the file-form ones; omit it when the doc-comment alone is enough. `gnd show` resolves the ID either way; with a stub it follows the link and strips comment markers. See `§AS-scanner.4` for the supported doc-comment forms.

| ID | Subject |
|---|---|
| [§AS-scanner](AS-scanner.md) | how `gnd` discovers declarations and citations |
| [§AS-checker](AS-checker.md) | how `gnd` validates the scanner's findings |
| [§AS-bindings](AS-bindings.md) | how the same Rust engine is exposed on three platforms |
| [§AS-lsp](AS-lsp.md) | how the optional LSP server is built |

This index is navigational — citations should target the spec ID directly, never this file.

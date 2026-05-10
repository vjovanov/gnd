# Architectural spec

Internals — *how* this project is built. One file per spec; each H1 declares an `AS-NNN-<slug>` ID and the body is its contract. Citations from elsewhere in the tree (`§AS-NNN-<slug>.<section>`) resolve into these files.

An architectural spec may live inline in the class- or module-level doc-comment of the file it describes. A one-line stub here whose H1 is `# AS-NNN-<slug>: [<path>](<path>)` is **optional** — add it when you want the inline spec to appear in the index below; omit it when the doc-comment alone is enough. `gnd show` resolves the ID either way.

By convention every file under this directory — each file-form spec and each stub you chose to write — is linked from this README. Inline specs without a stub are not listed here and that is fine. Extra prose, recommended reading order, and conceptual groupings are welcome around the link set.

| ID | Subject |
|---|---|

This index is navigational — citations should target the spec ID directly, never this file.

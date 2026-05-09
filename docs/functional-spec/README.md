# Functional spec

External behavior — *what* `gnd` does. One file per spec; each H1 is the declaration of an `FS-NNN-<slug>` ID and the body is its contract. Citations from elsewhere in the tree (`§FS-NNN-<slug>.<section>`) resolve into these files.

| ID | Subject |
|---|---|
| [FS-check](FS-check.md) | `gnd` validates every reference in a repo |
| [FS-show](FS-show.md) | `gnd` reads a single declaration body by ID |
| [FS-ide-plugins](FS-ide-plugins.md) | `gnd` ships first-party editor integrations |
| [FS-distribution](FS-distribution.md) | `gnd` ships on cargo, npm, and PyPI with a native API on each |
| [FS-fmt](FS-fmt.md) | `gnd` normalizes references in bulk |
| [FS-config](FS-config.md) | `gnd` reads a TOML config file under `.agents/` |
| [FS-non-goals](FS-non-goals.md) | what `gnd` will deliberately not do |
| [FS-init](FS-init.md) | `gnd` bootstraps a new `gnd`-conformant repo |
| [FS-name](FS-name.md) | `gnd` proposes IDs for new declarations |

This index is navigational — citations should target the spec ID directly, never this file.

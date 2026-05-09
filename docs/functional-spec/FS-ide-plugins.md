# FS-ide-plugins: gnd ships first-party editor integrations

`gnd` ships first-party integrations for the four editors that cover the vast majority of working developers and AI-pair-programming setups: **VSCode**, **IntelliJ IDEA**, **Vim/Neovim**, and **Emacs**. Each integration makes IDs first-class navigable artifacts inside the editor — the same way file paths and symbols already are. Together with FS-show they realize G-friendliness-first for editor-resident agents.

The slug remains `ide-plugins` for stable referencing; "IDE" here is read in the broad sense of "interactive editing environment" so as to include Vim and Emacs.

## 1. Capabilities

### 1.1 Follow-the-link

Cmd-click (VSCode) / Ctrl-click (IntelliJ) on a citation `FS-check.3.1` jumps to the declaring heading in the right file, scrolled to the right section.

### 1.2 Inline preview without navigation

Hovering on a citation pops up a panel showing the body of the cited declaration (or section) — same content as `gnd show <ID>`. The reader does not need to leave their current file to absorb the cited material. This is the editor counterpart of the agent's `gnd show` workflow.

### 1.3 Diagnostics

Dangling references, missing sections, and other errors detected by FS-check appear as in-editor squiggles with the same `path:line: message` shape as the CLI. Quick-fix actions offer to insert a stub declaration when an ID is cited but not yet declared.

### 1.4 Completion

Typing a partial ID (e.g. `FS-`) offers an autocompletion list of declared IDs in the workspace, with the declaration's title shown alongside.

### 1.5 Trigger-to-marker live transform

Per DF-reference-marker, the plugins watch for the configured trigger sequence (default `$$`) and replace it with the marker (default `§`) the moment it is followed by a token matching the configured ID grammar's leading `{kind}` placeholder and a digit (FS-config.3.2). Type the trigger before `FS-check`; see `§FS-check`. This is what makes the marker practical to use without leaving the keyboard.

The trigger, marker, and ID grammar are read from `gnd.toml` so the editor experience matches the project's choices. If no config is present, the defaults from DF-reference-marker and FS-config apply.

## 2. Implementation strategy

All four first-party editor integrations share a single Language Server Protocol (LSP) server (`gnd-lsp`) backed by `gnd-core`. The LSP server runs as a child process. Each editor-side integration is a thin shell: editor-specific glue, distribution packaging, and any UI that LSP does not cover natively.

### 2.1 VSCode (first to ship)

A `gnd-vscode` extension published to the VS Code Marketplace and Open VSX. Bundles the `gnd-lsp` binary for the host platform; activates on workspaces that contain a `gnd.toml` or any `*.md` file with a recognized declaration. UI uses native LSP hover, diagnostics, code-action, and definition surfaces.

### 2.2 IntelliJ IDEA (second)

A `gnd-intellij` plugin published to the JetBrains Marketplace. Targets IntelliJ IDEA Ultimate and Community editions; works in other IntelliJ-platform IDEs (PyCharm, GoLand, RustRover, etc.) by virtue of the shared platform. Uses IntelliJ's built-in LSP support to host `gnd-lsp` without bespoke platform code; adds IntelliJ-specific UI for inline-preview popups (FS-ide-plugins.1.2).

### 2.3 Vim and Neovim

A `gnd.vim` (Vim 9) / `gnd.nvim` (Neovim) plugin distributed via standard plugin managers (`vim-plug`, `lazy.nvim`, `packer.nvim`). Configures the editor's LSP client to attach `gnd-lsp` for `markdown`, files whose extension matches the configured `[scan] extensions` set (FS-config.3.5), and projects with a `gnd.toml`. Native LSP support in Neovim is the primary integration path; in Vim 9, the `vim-lsp` ecosystem is supported. Live trigger transform (FS-ide-plugins.1.5) is implemented as a buffer-local `InsertCharPre` autocommand backed by the LSP server.

### 2.4 Emacs

A `gnd.el` package distributed via MELPA. Provides minor modes that integrate with both `lsp-mode` and `eglot` — the user's choice of LSP client controls which path is taken. Hover previews use the LSP `textDocument/hover` surface; jump-to-definition uses `xref`. Live trigger transform is implemented as a `post-self-insert-hook` calling out to `gnd-lsp` for the rewrite.

### 2.5 Other LSP-capable editors

Once `gnd-lsp` ships, Helix, Zed, Sublime, Kakoune, etc. can adopt `gnd` by pointing their LSP client at the same binary. We do not maintain first-party configs for them, but we do publish a generic LSP config snippet that translates cleanly. PRs adding first-party support for additional editors are welcome but not on the roadmap.

### 2.6 When to split this spec

This single FS describes four integrations that share a server and most behavior. Split into separate FS entries (`FS-vscode-plugin`, `FS-intellij-plugin`, `FS-vim-plugin`, `FS-emacs-plugin`) when any of the following becomes true:

- An integration's marketplace/distribution rules force divergent packaging or signing flows that need their own decisions.
- An integration grows features that the others cannot or should not implement.
- An integration acquires its own maintainer with an independent release cadence.

Until then, the merged form keeps the four shells in lockstep and avoids duplicated prose. The split itself, when it happens, is recorded as its own architectural decision so the trigger is auditable.

## 3. Out of scope (for now)

- Refactoring (renaming an ID across the workspace). The scheme says IDs are forever; renaming requires a `Supersedes:` workflow that is better expressed as a deliberate edit than an automated refactor.
- Inline editing of declaration bodies from the hover popup. Editors already do this well; we do not need to.

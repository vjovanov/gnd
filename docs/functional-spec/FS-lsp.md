# FS-lsp: gnd will ship an optional LSP server

Status: planned. The current shipped surface is the `gnd` CLI; this file is the contract for the optional Language Server Protocol server when `gnd-lsp` lands.

`gnd` will ship an optional Language Server Protocol server, `gnd-lsp`, as a separate binary that any LSP-aware editor can talk to: VSCode, Neovim, Emacs (eglot or lsp-mode), Helix, Zed, Sublime Text, and the IntelliJ family via LSP4IJ. Users who want editor integration install `gnd-lsp` and configure their editor once; users who do not — CI pipelines, pre-commit hooks, contributors who only run `gnd check` — install nothing extra and pay no dependency cost. The architectural choice (separate binary rather than a Cargo feature or a bundled library) is decided in §DA-lsp-optional.

`gnd` does not ship per-editor wrappers. The only first-party editor surface is the LSP server; per-editor configuration is one-time work the user does, with example snippets in the README. See §FS-non-goals for the non-goal that pins this.

## 1. Capabilities

The minimum viable set — everything the server speaks at version 1.0.

### 1.1 Diagnostics

`textDocument/publishDiagnostics` pushes `gnd check` results as the user edits. Each unknown reference, missing section, duplicate declaration, and broken stub becomes a diagnostic with the same `path:line: <message>` content the CLI prints to stderr (§FS-errors). Severity follows the engine's severity model (§FS-non-goals.9 — not configurable). The diagnostic position is the citation's start column on the line; precise column information is computed once per scan and reused across the open editor session.

### 1.2 Hover preview

`textDocument/hover` on a citation returns the body `gnd show <ID>` would print (§FS-show.2.1), or the body of the requested section if the citation includes one (§FS-show.2.2). When the declaration's home is in source code (a stub points at `src/bus.rs`), the hover body is the comment-stripped prose per §FS-show.2.3.2 — the same content the CLI returns. There is no separate "IDE-only" rendering; hover and `gnd show` produce the same bytes.

### 1.3 Go-to-definition

`textDocument/definition` on a citation jumps to the declaration's `path:line`. For a stub-and-inline-source pair (§FS-check.3.4), the server follows the stub's link and lands on the inline declaration line directly — the user does not stop at the stub.

### 1.4 Live trigger transform

`textDocument/onTypeFormatting` watches the configured trigger sequence (default `$$`, per §DF-reference-marker.2.2) and replaces it with the marker (default `§`) the moment the trigger is followed by a token matching `<KIND>-<digit>`. This is the live counterpart to `gnd fmt`'s bulk trigger pass (§FS-fmt.2.1) and is what makes the marker practical to type without leaving the keyboard.

The trigger, marker, and recognized `KIND` set are read from `.agents/gnd.toml` so the editor experience matches the project's choices. If no config is present, the defaults from §DF-reference-marker and §FS-config apply.

### 1.5 Capabilities reserved for later

These are out of scope for the first version but compatible with the architecture:

- `textDocument/completion` — autocomplete `§F` to declared `FS-…` IDs from the workspace.
- `textDocument/codeAction` — quick fixes for "unknown reference" (suggest similarly-named IDs) and "section not found" (suggest sibling sections).
- `workspace/symbol` — fuzzy-find IDs across the project.

Each addition is a separate roadmap item if and when it is taken on.

## 2. Installation and lifecycle

### 2.1 Install

`gnd-lsp` is a separate package on each registry per §FS-distribution: `cargo install gnd-lsp`, `npm install -g gnd-lsp`, `pipx install gnd-lsp`. None of these are pulled in by the corresponding CLI install (`cargo install gnd` and friends do not transitively install `gnd-lsp`). A user with no editor integration installs the CLI alone.

### 2.2 Lifecycle

Users do not run `gnd-lsp` directly. The editor's LSP client spawns it as a child process when a relevant file (markdown or any extension in the configured `[scan] extensions`) is opened in a workspace containing `.agents/gnd.toml` or `agents.md`, and kills it when the workspace closes. The server speaks LSP over stdio; there is no daemon, no socket, no background service. CI pipelines that happen to have `gnd-lsp` installed never invoke it — the only entry point in batch contexts is the CLI.

### 2.3 Editor configuration (one-time, per editor)

The README ships example LSP-client snippets for the editors most contributors use:

- **Helix** — three lines in `languages.toml`.
- **Neovim** — `nvim-lspconfig` snippet (or zero local config once the server is upstreamed there).
- **Zed** — central LSP registry entry; one config block locally if not yet upstreamed.
- **Emacs** — `eglot-server-programs` or `lsp-mode` registration (~5 lines).
- **VSCode** — install a generic LSP client extension and point it at `gnd-lsp`. A first-party VSCode extension is **not** shipped (§FS-non-goals).
- **IntelliJ family** — LSP4IJ plugin with a `gnd-lsp` server registration.

Adding a new editor's snippet to the README is a small contribution; it does not require a release.

## 3. Configuration

The server reads `.agents/gnd.toml` via the same discovery logic as `gnd check` (§FS-config), walking up from the workspace root supplied by the editor's LSP `initialize` request. There is no separate LSP config; one source of truth drives both the CLI and the LSP. A workspace without `.agents/gnd.toml` falls back to the canonical defaults (§G-zero-config).

Editor-side LSP configuration (server arguments, workspace folders) is the user's responsibility per §2.3 and is not part of `gnd.toml`.

## 4. Determinism and parity with the CLI

Same input + same config → same diagnostics, same hover body, same definition target, byte-for-byte (§FS-non-goals.13). An e2e fixture per LSP capability runs the same `e2e/cases/*` corpus through the LSP and the CLI and asserts the LSP's published diagnostics match the CLI's report and the LSP's hover body matches `gnd show`.

The LSP server does not have an "interactive" mode, a confirmation prompt, or any user-visible state that the CLI lacks (§FS-non-goals.10). It is the same engine with a different transport.

## 5. Out of scope

- **Per-editor wrappers**: VSCode/IntelliJ/Vim/Emacs first-party plugins are not shipped (§FS-non-goals). The LSP server is the surface; editor configuration is the user's.
- **Refactoring (rename ID)**: `gnd` does not rename IDs; the scheme says IDs are forever (§FS-non-goals.4).
- **Inline editing of declaration bodies from the hover popup**: editors already do this well; `gnd-lsp` does not implement it.
- **Network access**: the server performs no network I/O (§FS-non-goals.11). All scanning is local.

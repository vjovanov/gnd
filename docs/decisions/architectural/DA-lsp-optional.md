# DA-lsp-optional: LSP server ships as a separate, optional binary

**Status:** Accepted
**Date:** 2026-05-09

## 1. Context

`gnd`'s editor-integration story (§FS-lsp) is delivered through the Language Server Protocol. There are several plausible ways to package the LSP capability into the project:

1. **Bundle into `gnd-cli`.** A single `gnd lsp` subcommand on the existing CLI; one binary, one install.
2. **Cargo feature flag on `gnd-cli`.** `cargo install gnd --features lsp` opts in; default install excludes it.
3. **Separate binary in the same crate.** `gnd-cli` produces both `gnd` and `gnd-lsp` from one Cargo manifest.
4. **Separate crate, separate binary, separate published package.** `gnd-lsp` is its own crate in the workspace; `cargo install gnd-lsp` is the only way to get it.

This decision picks among the four and pins the consequences for §FS-distribution and §AS-bindings.

## 2. Decision

**Option (4): separate crate, separate binary, separate published package on every registry.**

`gnd-lsp` lives at `crates/gnd-lsp/` in the workspace defined by §AS-bindings.1, depends only on `gnd-core`, and is published as a standalone package on cargo (`gnd-lsp`), npm (`gnd-lsp`), and PyPI (`gnd-lsp`). The CLI install path (`cargo install gnd`, `npm install gnd-cli`, `pipx install gnd`) does not transitively pull in the LSP server; users who want editor integration install it explicitly.

## 3. Why this shape

### 3.1 Dependency cost

`gnd-lsp` depends on `tower-lsp`, `tokio`, `lsp-types`, and `serde_json` — collectively tens of transitive dependencies plus an async runtime. None of those are needed by `gnd check`, `gnd show`, `gnd fmt`, or `gnd init`, all of which are synchronous batch operations. Bundling (option 1) or feature-flagging on the CLI crate (option 2) leaks those dependencies into the resolved dependency graph in some installs (option 1) or risks accidental inclusion (option 2 — feature unification across a workspace can force `lsp` features on the CLI even when no consumer asked for them). A separate crate keeps the CLI's dependency tree small for the audience that runs `gnd` in CI without ever opening an editor — the largest audience by usage volume.

### 3.2 CI binary size and start-up

CI pipelines and pre-commit hooks call `gnd check` thousands of times more often than any editor opens `gnd-lsp`. Keeping the CLI binary small and synchronous matters for both raw start-up cost (every invocation pays binary-load overhead) and download size (every CI cache pull moves the binary across the network). A bundled binary that includes a LSP server most users will never invoke is a tax on the common case. §G-fast-feedback explicitly endorses paying engineering cost to keep the CLI fast; this is part of that pattern.

### 3.3 Distribution parallel to industry practice

`rust-analyzer`, `gopls`, `pyright`, and `typescript-language-server` are all distributed as separate binaries, not subcommands of the language toolchain. This is the established expectation: editors point their LSP client at a binary named by convention, not at a subcommand of a CLI. Following the convention means contributors who already know how to wire up `rust-analyzer` know how to wire up `gnd-lsp` — no new mental model.

### 3.4 Composition with §FS-non-goals.12.1

§FS-non-goals.12.1 forbids a plugin system inside the engine. Keeping `gnd-lsp` as a separate process that talks to the engine through a defined transport (LSP over stdio) — rather than as a feature flag that links against the engine in-process — preserves the "no plugins inside" property at the architectural level. The LSP is a *consumer* of `gnd-core`, on equal footing with `gnd-cli`, `gnd-node`, and `gnd-py`.

### 3.5 Composition with §G-no-silent-breakage

A separate package on every registry means the LSP's release cadence can decouple from the CLI's when needed. A bug fix to the LSP (say, a hover formatting change) does not require a CLI version bump. Conversely, a CLI surface change that breaks the LSP's assumptions surfaces as a build break in the LSP crate before release, not as a runtime mismatch in editors. The compile-time link from `gnd-lsp` to a pinned `gnd-core` version is what enforces this.

## 4. Consequences

- The workspace gains a third crate: `crates/gnd-lsp/`. §AS-bindings.1 is updated to list it alongside `gnd-core`, `gnd-cli`, `gnd-node`, and `gnd-py`.
- Three new packages are published, one per registry: `gnd-lsp` on cargo, npm, and PyPI. §FS-distribution.1 is updated to list them.
- §FS-lsp.2.1 ("Install") states that the CLI install does not pull in `gnd-lsp` transitively; the inverse is also true (`gnd-lsp` does not pull in the CLI binary).
- The roadmap item §RM-006-lsp owns shipping the crate and the published packages. It depends on §RM-008-core-cli-split (the workspace split) — that prerequisite must land first so `gnd-lsp` has a `gnd-core` to depend on.
- §FS-non-goals adds an explicit entry: first-party per-editor plugins (VSCode/IntelliJ/Vim/Emacs wrappers) are out of scope. The LSP server is the only editor surface; editor configuration is the user's one-time work.

## 5. Alternatives considered

| Option | Why rejected |
|---|---|
| **(1) Bundle into `gnd-cli` as a `gnd lsp` subcommand.** Single binary, single install; the simplest install story. | Forces every CLI install — including CI — to carry the LSP's transitive dependencies (tower-lsp, tokio, lsp-types). Increases binary size, slows start-up, and pushes async runtime into a synchronous tool. The largest audience pays for a feature it never invokes. |
| **(2) Cargo feature on `gnd-cli`.** `cargo install gnd --features lsp` opts in; default install excludes the LSP. Avoids the dependency cost for the default case. | Cargo's feature unification can force LSP features on the CLI when a consumer in the same workspace requests them — a transitive `gnd-cli` dependency with `features = ["lsp"]` would impose them on every consumer of `gnd-cli`. Not airtight isolation. Also loses the per-binary version cadence in §3.5. |
| **(3) Separate binary in the same crate.** One Cargo manifest, two `[[bin]]` entries (`gnd` and `gnd-lsp`). Avoids a new crate. | The binaries would share a Cargo manifest, which means they share a dependency declaration — `tokio` and friends would be present in the manifest even if optional. `cargo install gnd` would still need to compile the dependency graph including the LSP's deps to produce only the CLI binary. Separating crates is the one mechanism Cargo provides for fully partitioning compiled artifacts. |
| **(4) — chosen** | See §2 and §3 above. |

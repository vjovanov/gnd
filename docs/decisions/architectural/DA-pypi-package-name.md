# DA-pypi-package-name: PyPI uses gnd-cli as the package name

**Status:** Accepted
**Date:** 2026-05-11
**Supersedes:** §DA-reference-checker-name for the PyPI package name only.

## 1. Context

§DA-reference-checker-name chose `gnd` for the binary and repository, and treated the existing PyPI `gnd` package as a dormant squat that could be ignored later. The final launch review re-checked the registry and found that the name is not available to this project: PyPI already serves `GND` releases, while `gnd-cli` is available on PyPI and is already the planned npm package name.

The product must not ask users to install through a name owned by an unrelated project. The binary can still be `gnd`; package-manager names do not have to be identical when a registry namespace is occupied.

## 2. Decision

Use `gnd-cli` as the PyPI package name for the CLI and Python binding. The installed command remains `gnd`, and the Python import module remains `gnd` unless the binding implementation later proves that impossible.

Keep `gnd-lsp` as the planned PyPI package name for the optional LSP server.

## 3. Consequences

- `pip install gnd-cli` is the documented PyPI install command once Python wheels ship.
- `from gnd import check, show` remains the intended Python API surface.
- §FS-distribution records `gnd-cli` for PyPI instead of `gnd`.
- The cargo crate, binary, repository, and npm package naming decisions from §DA-reference-checker-name are unchanged.

## 4. Alternatives considered

| Option | Why not |
|---|---|
| Keep planning to publish `gnd` on PyPI | It depends on an unrelated owner and makes the launch path ambiguous. |
| Rename the whole product | Cargo and the binary name are already coherent, and the collision is registry-specific. |
| Use `gnd-check` on PyPI | It is available, but `gnd-cli` already matches the npm package name and reads as the package that installs the command. |

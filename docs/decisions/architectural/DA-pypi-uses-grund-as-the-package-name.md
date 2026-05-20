# DA-pypi-uses-grund-as-the-package-name: PyPI uses grund as the package name

**Status:** Accepted
**Date:** 2026-05-14
**Supersedes:** The PyPI-package part of [§DA-rename-to-grund](DA-rename-to-grund.md#da-rename-to-grund-rename-gnd-to-grund-before-first-publish), which left the final `grund` vs. `grund-cli` choice to the pre-release registry-name check.

## 1. Context

[§DA-rename-to-grund](DA-rename-to-grund.md#da-rename-to-grund-rename-gnd-to-grund-before-first-publish) renamed the product before first publish and kept `grund-cli` as the npm/PyPI CLI package name pending a final live-registry check. That was conservative: npm's unscoped `grund` package is occupied, while PyPI `grund` appeared free but still needed verification.

The final pre-release check found PyPI `grund` available and npm `grund` still externally occupied. This is the last point where the PyPI name can be made friendlier without migration cost: no `grund` wheel has shipped yet.

## 2. Decision

Use `grund` as the PyPI package name for the CLI and Python binding. The installed command remains `grund`, and the Python import module is `grund`.

Keep `grund-cli` on npm, where the bare name is not available to this project. Keep `grund-lsp` as the optional LSP-server package on both npm and PyPI.

## 3. Consequences

- [§FS-distribution](../../functional-spec/FS-distribution.md#fs-distribution-grund-distribution-targets) records PyPI `grund` instead of `grund-cli`.
- `scripts/check-registry-names.sh` treats PyPI `grund` as a claimed name, so a later external claim blocks release instead of printing a notice.
- The registry names are asymmetric (`npm install grund-cli`, `pipx install grund`) because the registries are asymmetric. This is preferable to making PyPI users type an avoidable `-cli` suffix just to match npm.

## 4. Alternatives considered

| Option | Why not |
|---|---|
| Keep PyPI `grund-cli` for npm/PyPI symmetry | Symmetry would be artificial: PyPI has the better name free, npm does not. The installed command and Python import both being `grund` matters more for friendliness. |
| Publish both `grund` and `grund-cli` on PyPI | Splits ownership and docs before there is a compatibility reason to do so. A single package name is cleaner for first publish. |

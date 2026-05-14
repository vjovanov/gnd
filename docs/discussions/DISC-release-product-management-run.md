# DISC-release-product-management-run: Final release product management run

Date: 2026-05-14

## 1. Product frame

The release is valuable only if it advances the reason `grund` exists: keep agents grounded in stable specs across docs and source, with cheap retrieval and mechanical enforcement. That is the product center of gravity from [§GND-grund](../grund.md#gnd-grund-agents-stay-grounded-in-the-spec) and the headline goal in [§GOAL-agent-grounding](../goals/goals.md#goal-agent-grounding-agents-stay-cited-as-they-work). For this run, every finding is judged against that frame first, then against registry and packaging mechanics.

## 2. Verdict

Do not publish from `6a4d9d94a9`. The implementation intent is right, but the pushed CI run exposed a cross-platform failure in the new stub-resolution coverage. That is release-blocking because [§GOAL-multi-language.1](../goals/goals.md#1-identical-behavior) requires ordinary reports to stay platform-neutral across Linux, macOS, and Windows, and [§GOAL-agent-grounding](../goals/goals.md#goal-agent-grounding-agents-stay-cited-as-they-work) depends on the checker being trusted in the agent loop.

With the local path-equivalence fix applied, the release candidate becomes suitable for a fresh CI run. If that run is green, the cargo side can publish in dependency order: `grund-core` first, then `grund`. The release should still be positioned as the Rust CLI plus shared-core foundation for the later LSP and npm/PyPI bindings, not as completion of the full three-ecosystem promise in [§GOAL-multi-language](../goals/goals.md#goal-multi-language-same-engine-three-platforms).

## 3. Findings

### 3.1 Release blocker found: cross-platform stub equivalence

The pushed `main` CI failed on macOS and Windows in `stub_resolution_keeps_repo_root_fallback_for_old_stubs`. The failure was an ambiguous duplicate declaration between the Markdown stub and the inline source declaration. Locally on Linux the same test passed.

Root cause: the checker compared the scanned source path and the resolved stub target by raw `PathBuf` equality. On macOS, temporary paths can canonicalize through `/private/var` while the configured root keeps `/var`, so the same file compared unequal. That breaks the "stub plus inline declaration counts as one home" rule in [§AR-checker.2.1](../../crates/grund-core/src/checker.rs) / [§FS-check.3.3](../functional-spec/FS-check.md#33-duplicate-declaration).

Status: fixed locally by comparing canonicalized paths when deciding whether a stub points at the inline declaration. The focused stub tests and full workspace tests pass after the fix.

### 3.2 Distribution shape is product-correct

The `grund-core` split serves the release strategy: one shared engine, frontends later. That directly supports [§GOAL-multi-language](../goals/goals.md#goal-multi-language-same-engine-three-platforms) and the workspace shape in [§AR-bindings.1](../architecture/AR-bindings.md#1-target-workspace-layout). The current implementation now gives `grund-lsp` a real dependency target without bundling LSP dependencies into the CLI path, which preserves the fast common case demanded by [§GOAL-fast-feedback](../goals/goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible) and the optional-LSP boundary in [§DA-lsp-optional](../decisions/architectural/DA-lsp-optional.md#da-lsp-optional-lsp-server-ships-as-a-separate-optional-binary).

Product positioning should be precise: this release reserves and proves the shared engine boundary. It does not yet fulfill the full npm/PyPI binding promise in [§FS-distribution.1](../functional-spec/FS-distribution.md#1-targets), so launch copy should not claim JavaScript or Python API availability until those frontends exist.

### 3.3 Registry names are aligned with the docs

The live registry guard passed for the claimed release slots: crates.io `grund-core`, `grund`, `grund-lsp`; npm `grund-cli`, `grund-lsp`; PyPI `grund`, `grund-lsp`. It also reported npm `grund` as externally occupied, which matches the documented asymmetric naming decision in [§DA-pypi-uses-grund-as-the-package-name](../decisions/architectural/DA-pypi-uses-grund-as-the-package-name.md#da-pypi-uses-grund-as-the-package-name-pypi-uses-grund-as-the-package-name) and [§FS-distribution.1](../functional-spec/FS-distribution.md#1-targets).

This is important product hygiene: the final names match the story users will read. There is no last-minute naming contradiction to resolve before publishing.

### 3.4 Cargo publish order is mandatory

`cargo package -p grund-core --locked` succeeds on a clean tree. `cargo package -p grund --locked` fails until `grund-core` exists in the crates.io index, because the root `grund` crate depends on `grund-core = 0.1.0`. That is expected and matches [§FS-distribution.4](../functional-spec/FS-distribution.md#4-release-process): publish `grund-core` first, then publish `grund`.

Release managers should not treat the `grund` package failure as a code defect before `grund-core` is published. They should treat it as a sequencing gate.

### 3.5 Local release gates after the fix

Passed locally after the path-equivalence fix:

- `cargo fmt --all -- --check`
- `cargo test -p grund-core --lib stub_resolution --locked`
- `cargo test --workspace --all-targets --locked`
- `cargo run --locked --quiet -- check . --format json`
- `lychee --no-progress --include-fragments README.md docs examples`
- `scripts/check-registry-names.sh`
- `bash scripts/pgo-build.sh`
- `target/release/grund check . --format json`

The remaining release gate is remote CI on the fix commit, because the product promise includes platform parity, not just local Linux success ([§GOAL-multi-language.1](../goals/goals.md#1-identical-behavior)).

## 4. Recommendation

Ship after one more commit and green CI:

1. Commit the path-equivalence fix and this report.
2. Push and require the full GitHub CI matrix to pass on Linux, macOS, and Windows.
3. Run the manual **Pre-release checks** workflow.
4. Publish `grund-core` to crates.io.
5. Publish `grund` to crates.io after the `grund-core` index entry is visible.
6. Keep npm/PyPI/LSP messaging in "planned/next" language until the actual `grund-node`, `grund-py`, and `grund-lsp` crates/packages land.

This release is product-worthy once the remote matrix is green: it strengthens the core grounding loop from [§GND-grund](../grund.md#gnd-grund-agents-stay-grounded-in-the-spec), keeps the check fast enough for agents per [§GOAL-fast-feedback](../goals/goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible), and creates the shared engine boundary required for [§GOAL-multi-language](../goals/goals.md#goal-multi-language-same-engine-three-platforms) without over-claiming what has shipped.

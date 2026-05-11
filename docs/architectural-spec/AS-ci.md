# AS-ci: CI mirrors the local pre-commit gate

The CI workflow is the remote form of the local pre-commit gate. Anything that can abort a local commit must also abort CI, so a contributor cannot bypass the repository's local checks by skipping hooks or editing through a web UI. This supports [§G-no-silent-breakage](../goals/goals.md#g-no-silent-breakage-changes-ship-through-a-deprecation-path) and keeps the link-checking boundary from [§FS-non-goals.1](../functional-spec/FS-non-goals.md#1-markdown-link-validation) enforced alongside `gnd check`.

## 1. Pre-commit is the source of truth

The hook list lives in `.pre-commit-config.yaml`. CI must invoke that list directly with `pre-commit run --all-files`, rather than hand-copying each hook into separate workflow steps. The workflow may install hook prerequisites first, but the set of checks is defined by the pre-commit config.

When a new pre-commit hook is added, the same change must ensure CI can run it. If the hook needs an external binary, the CI workflow installs that binary before the pre-commit step. If the hook is intentionally local-only, it does not belong in `.pre-commit-config.yaml`; put it in a developer-local hook instead.

## 2. Platform scope

The full Rust build and test matrix still runs on every configured operating system. The pre-commit gate may run on one representative CI platform when the hooks are platform-independent, because its job is policy parity with local commits, not cross-platform behavior coverage. Platform-specific behavior belongs in the build and test jobs.

## 3. Current hooks

The current pre-commit gate runs `gnd check`, including the grounding floor from [§FS-check.3.6](../functional-spec/FS-check.md#36-ungrounded-source-file-opt-in), and `lychee` for Markdown links. `gnd` owns ID citations across docs and source; `lychee` owns regular Markdown links and URLs. Running both in CI preserves that boundary.

## 4. Performance smoke guard

CI carries a cheap floor on [§G-fast-feedback](../goals/goals.md#g-fast-feedback-gnd-must-be-as-fast-as-possible) that does not depend on any benchmarking toolchain: the `gnd .` self-check step runs the already-built binary under a generous wall-clock `timeout` — long enough never to flake on a loaded runner, short enough to fail the build on a catastrophic regression such as an accidental quadratic walk or a second read pass over every file. It is not the budget itself (the budget is tens of milliseconds; the ceiling is tens of seconds) — it is the difference between "we'd notice eventually" and "the build is red on the commit that did it". The precise per-commit meter is the §5 benchmark job; once [§RM-benchmarks](../roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-g-fast-feedback-budgets) adds a committed baseline and a regression threshold to that job it becomes the authoritative check, and this timeout stays as a backstop.

## 5. Benchmark job

A separate `bench` job runs the instruction-counting harness ([§AS-benchmarks](AS-benchmarks.md#as-benchmarks-instruction-counting-benchmarks-for-the-hot-cli-commands)). Because the harness runs `gnd` under Callgrind, the job first installs `valgrind` and `cargo install`s `iai-callgrind-runner` pinned to the same version as the `iai-callgrind` dev-dependency, then runs `cargo bench --features bench`. It runs on one representative platform (the harness measures instruction counts, not cross-platform behavior — §2's reasoning applies) and on every push and pull request, so the numbers are recorded per commit. It does not block the build today; turning the recorded baseline into a build-failing regression threshold is the remaining [§RM-benchmarks](../roadmap.md#rm-benchmarks-a-benchmark-harness-for-the-g-fast-feedback-budgets) work, at which point this job is the authoritative [§G-fast-feedback](../goals/goals.md#g-fast-feedback-gnd-must-be-as-fast-as-possible) meter (§4's timeout becoming the backstop). The harness body is gated behind the `bench` Cargo feature, so the regular build and test matrix compiles only a no-op bench target and stays free of the Valgrind dependency — only this job needs it; that gate is also why §1's "CI installs hook prerequisites" pattern is followed here as a dedicated job rather than folded into the test matrix.

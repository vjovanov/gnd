# Goals

What `gnd` measures itself against. If a change does not advance one of these, it is not worth doing. Goals are declared inline below so a human can read the whole picture top-to-bottom; each declaration is a stable ID and may be cited from anywhere in the repo.

## G-001-no-dangling-refs: every cited ID resolves to a declaration

A repo that passes `gnd` has zero dangling references and zero broken section coordinates. False negatives are bugs. This is the load-bearing promise; everything else exists in service of this one.

### 1. What "resolves" means

A citation `§FS-042-user-login.3.1` resolves when:

- A declaration of `FS-042-user-login` exists somewhere in the scanned tree.
- The declaration body contains a numbered section `3.1` (recursively, at any depth — see FS-006-config.3.3).
- If the declaration is a stub with `Defined-in:`, the pointed-at file contains an inline declaration of the same ID.

### 2. Measurable

The e2e suite includes deliberately broken inputs (missing declarations, missing sections, broken stubs); `gnd` must catch each one and report it on the right line. Any uncaught case is a regression.

## G-002-fast-feedback: gnd must be as fast as possible

Speed is not a target — it is an **ordering principle**. When a design choice trades clarity, generality, or features for speed, speed wins. `gnd` exists to be invoked on every keystroke (IDE), every save (watcher), every commit (CI). Anything slower than human reflex breaks the loop it is meant to enable.

### 1. Hard budgets

These must hold; CI fails on regression.

- Under **100 ms** on the `gnd` repo itself. The self-host loop must be invisible.
- Under **1 s** on a 10k-file repo.
- Single allocation per file at most; zero allocations on the hot regex path where possible.

### 2. How we get there

- Linear pass per file. No second walks for second-stage checks.
- Streaming line scan, not full-file buffering, on large files.
- Parallel walk using `rayon` once the single-thread version stops winning.
- Compiled regexes shared across all files via `once_cell`.
- Skip directories that obviously cannot contain specs — see FS-006-config.3.5.

### 3. Measurable

`time gnd .` on a synthetic 10k-file fixture; CI tracks the number across commits and fails on regression.

## G-003-zero-config: works on any conformant tree

No config file, no flags required for the common case. Discovery is by walking from the supplied root. The default behavior is the canonical `gnd` reference grammar — that is the contract.

### 1. What "common case" means

A repo whose layout follows the canonical `gnd` conventions: `agents.md` at the root; `docs/` containing `functional-spec/`, `architectural-spec/`, `decisions/{architectural,functional}/`, `goals/`; `e2e/` for end-to-end tests; IDs in the canonical grammar. For such a repo, `gnd .` Just Works.

### 2. Measurable

`gnd <repo>` works on any conformant repo without additional setup. The e2e suite includes a "minimal conformant repo" fixture; `gnd` must report zero errors with no flags and no `gnd.toml`.

### 3. Composition with G-006-configurable

Zero-config and configurable are not in tension — they compose. Out-of-the-box, `gnd` matches the canonical defaults; for projects that diverge, every assumption is overridable per FS-006-config. There is no middle ground where defaults are weird.

## G-004-multi-language: same engine, three platforms

Cargo, npm, and PyPI ship the same engine, with idiomatic API surfaces on each. The check command behaves identically on all three. This is what makes `gnd` viable as a dependency for projects whose CI pipelines, editor tooling, or test harnesses are written in JavaScript or Python — not just Rust.

### 1. Identical behavior

The same input — a tree plus an optional `gnd.toml` — produces a byte-identical report regardless of which binding called the engine.

### 2. Idiomatic surfaces

Each binding fits its host. Rust returns `Result<T, E>`; Node returns Promises; Python returns values and raises exceptions. Names follow each ecosystem's conventions. Behavior is identical; surface fits each. See FS-004-distribution and AS-003-bindings for the implementation.

### 3. Measurable

An integration test runs the same spec corpus through each binding and asserts byte-identical reports. Any diff between bindings is a release blocker.

## G-005-friendliness-first: as user- and agent-friendly as possible

Friendliness is the second **ordering principle** (alongside speed, G-002-fast-feedback). When a design choice trades raw capability or terseness for legibility, legibility wins. `gnd` is used by humans in terminals and IDEs *and* by AI agents through stdout pipelines — both audiences must be served.

### 1. Hard requirements

- **Errors point at the line.** Every error message includes `path:line: <message>`, so editors and agents can jump to the source unmodified.
- **Output is parseable.** A `--format=json` flag emits a stable JSON shape suitable for LLM consumption and editor integration.
- **Show is grounded.** `gnd show <ID>` returns just the declaration body — no surrounding context, no scrolling, no token waste — under 200 lines for the common case.
- **Help is actionable.** `gnd --help` is one screen; every flag has a one-line example.
- **No surprises.** Same input → same output, byte-for-byte. Order of files in the report is deterministic.
- **Zero noise on success.** A passing repo prints nothing on stdout.

### 2. What this rules out

By accepting friendliness as an ordering principle, we rule out designs that would compromise it for marginal gain: configurable severity levels (would let two installs disagree on whether a repo passes), configurable report ordering (would break editor integrations), per-flag interactive prompts (would block CI).

### 3. Measurable

Typical `gnd show` output under 200 lines; `gnd --format=json` validates against a stable schema in `e2e/`; `gnd --help` fits in 24 lines; round-trip determinism is enforced by an e2e test that runs `gnd` twice and diffs the output.

## G-006-configurable: every default is overridable

Zero-config by default (G-003-zero-config); configurable when a project's conventions diverge. Users must be able to write references **the way they like**.

### 1. What is configurable

Per FS-006-config, a `gnd.toml` at the repo root can override the set of `KIND` prefixes, the ID format itself, the reference marker and typing trigger, strict vs optional marker mode, the set of folders that are scanned and skipped, the supported comment prefixes for inline specs, and the output format defaults.

### 2. What is NOT configurable

Per G-005-friendliness-first.2, the severity model, exit-code mapping, report ordering, and other invariants that would let two correctly-configured installs disagree on a repo's well-formedness are deliberately **not** configurable.

### 3. Measurable

An e2e fixture with a non-default `gnd.toml` (custom kinds, alternate section delimiter) passes. The default config — applied implicitly when no `gnd.toml` exists — produces canonical `gnd` grammar.

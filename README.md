# grund

[![CI](https://github.com/vjovanov/grund/actions/workflows/ci.yml/badge.svg)](https://github.com/vjovanov/grund/actions/workflows/ci.yml)
[![grund check: ~360k LoC/s](https://img.shields.io/badge/grund%20check-~360k%20LoC%2Fs-brightgreen.svg)](docs/benchmarks.md)
[![crates.io](https://img.shields.io/crates/v/grund.svg)](https://crates.io/crates/grund)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

> **Keep your agents grounded** — specs, docs, and code as one knowledge graph, always in sync.

`grund` is built around one workflow:

0. **Specify your intent.** Declare the goal, spec, or decision as a `# <ID>: …` heading before any code or doc cites it.
1. **Cite as you write.** Every code unit carries a `§<ID>` back to the spec section it implements.
2. **Re-read before you edit.** `grund show <ID>.<section>` pulls just that subsection into context — no full-file reads, no token bloat.
3. **No dangling pointers.** `grund check` validates that every cited ID resolves — in `.md`, Rust `///`, Java doc-comments, Python docstrings, Go `//`, JSDoc, every doc-comment form `grund` knows about.

Off-the-shelf Markdown link checkers (`lychee`, `markdown-link-check`) only handle `.md` and only validate `[text](url)`. A `§FS-events.4` cited from `src/bus.rs` is invisible to them. That gap is what `grund` exists to close.

## 0. Specify your intent

Before anything can be cited, the target has to exist. A declaration is a heading whose first token is the ID — `grund`'s own reason for being lives at [`docs/grund.md`](docs/grund.md):

```markdown
# GND-grund: agents stay grounded in the spec

Keep agents grounded in the spec — fewer bugs, cheaper LLM context,
faster onboarding. …
```

That heading lives in the configured home for its kind (`GND` → `docs/grund.md`, `FS` → `docs/functional-spec/`, `GOAL` → `docs/goals/`, and so on — see [§4](#4-the-structure-that-gets-cited)). Once it's declared, any code, doc, or test can cite `§GND-grund` and `grund check` will resolve it.

## 1. Cite as you write

When code realizes a named behavior, it carries a `§<ID>` citation — on its doc-comment for a whole behavior, or inline beside the line that enforces one clause:

```rust
// src/bus.rs

/// # AR-event-bus: In-process event broadcaster
///
/// Implements the publish-subscribe contract from §FS-events.
pub struct EventBus {
    receivers: Vec<Receiver<Event>>, // §FS-events.4 — slow receivers are dropped silently
}
```

`grund` doesn't invent these citations — that's the contributor's call. What `grund` does is make sure the ones you wrote *resolve*, and tell you when a diff added a new code unit without one (`[reference] require_grounding = true`).

## 2. Re-read before you edit

A citation is a pointer to a fact, not a file path. Resolve it without opening files:

```bash
$ grund show FS-events.4
A receiver that falls behind the broadcaster is disconnected, not blocked.
The sender never waits on a slow consumer.
```

`grund show` returns *just* that subsection — well under 200 lines for the common case — so the agent pulls one fact into context instead of an entire file. Its companions:

- `grund show <ID>` — the whole declaration body
- `grund show <ID> --head` — the lead paragraph only
- `grund show <ID> --format json` — for tooling

That's the "cheap grounding" half of the workflow: every agent fetches the same bytes for the same ID, every time.

## 3. Check for dangling pointers

Rename the heading `FS-events` to `FS-event-stream` and `grund check` flags both sides of the boundary in one resolver:

```
$ grund check
src/bus.rs:5: unknown reference FS-events
src/bus.rs:7: unknown reference FS-events.4
```

`grund <path>` scans `<path>`; with no path it scans the canonical layout (`docs/`, `e2e/`, `src/`). In the scanned tree it enforces:

1. Every cited ID resolves to a declaration. *(dangling references)*
2. Every section coordinate (`.3.1`) resolves to a heading inside the declaration. *(missing sections)*
3. No ID is declared in two places. *(duplicates)*
4. Every stub heading `# <ID>: [<text>](<path>)` points at a file containing the inline declaration. *(broken stubs)*
5. The `AGENTS.md` / `CLAUDE.md` entry-point block is up to date. *(stale init)*
6. Declared-but-uncited IDs are flagged. *(unused — warning, not error; `E2E-` cases are exempt)*
7. *(opt-in)* With `[reference] require_grounding = true`: every source file carries at least one citation. *(ungrounded source file)*

A passing text check prints `success` and exits 0. Findings go to stdout as `<path>:<line>: <message>` so editors and agents jump straight to the source, and `grund check | …` / `grund check --format=json | jq` work without redirection (the linter convention — only run-level `error:` lines, like an unreadable path, go to stderr). JSON output remains diagnostics-only, so a clean `grund check --format=json` prints nothing.

`grund` does **not** check Markdown links, URLs, spelling, or grammar. Use [`lychee`](https://github.com/lycheeverse/lychee), `vale`, etc. for those.

## 4. The structure that gets cited

Every fact in a `grund` repo has a stable ID. The default kinds (configurable):

| Kind   | What it is              | Where it lives                                 |
|--------|-------------------------|------------------------------------------------|
| `GND`  | the project's reason for being (the *grund*) | `docs/grund.md` (one declaration, all of it inline) |
| `GOAL` | goal                    | `docs/goals/goals.md` (one file, all goals inline) |
| `FS`   | functional spec         | `docs/functional-spec/` — external behavior    |
| `AR`   | architectural spec      | `docs/architecture/` — **or inline in a class / module doc-comment** |
| `DF`   | functional decision     | `docs/decisions/functional/` (append-only)     |
| `DA`   | architectural decision  | `docs/decisions/architectural/` (append-only)  |
| `E2E`  | end-to-end test         | `e2e/cases/<id>/` (the test *is* the body)     |
| `RM`   | roadmap milestone       | `docs/roadmap.md`                              |

**Architectural specs can live inline in source.** Drop a one-line stub in `docs/architecture/AR-foo.md` whose H1 is `# AR-foo: [src/foo.rs](src/foo.rs)`, then declare the spec in the class doc-comment:

```rust
/// # AR-event-bus: In-process event broadcaster
///
/// ## 1. Topology
/// One sender, many receivers. Senders never block.
pub struct EventBus { /* … */ }
```

`grund show AR-event-bus` follows the stub, strips the `///` markers, and prints the Rustdoc prose. The same goes for Javadoc, JSDoc, Python docstrings, Go doc blocks, KDoc, Doxygen — every comment form enumerated in `grund`'s scanner spec.

`grund` does this itself: `§AR-checker` lives in the doc-comment of `fn check` in [`src/checker.rs`](src/checker.rs), with the one-line stub at [`docs/architecture/AR-checker.md`](docs/architecture/AR-checker.md) — `grund show AR-checker` prints it.

**ID format:**

```plaintext
     ┌──────────── citation ───────────┐
           ┌────────── ID ─────────────┐
  [§] KIND - [number -] slug [.section]
   │   │       │         │       │
   │   │       │         │       └─ optional dotted path, arbitrary depth (.3, .3.1, …)
   │   │       │         └───────── [a-z0-9][a-z0-9-]*  (default slug_pattern)
   │   │       └─────────────────── optional ordinal (e.g., 001)
   │   └─────────────────────────── G│FS│AR│DA│DF│E2E│RM│DISC
   └─────────────────────────────── citation marker (writing only)
```

Three schemes are supported. Pick one per repo and keep it stable — mixing is unsupported because citations would look identical but resolve under different rules. Each has a runnable tiny repo under [`examples/`](examples/).

| Scheme                                     | Example             | Benefit                                                                                                          | Trade-off                                                                |
|--------------------------------------------|---------------------|------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------|
| `{kind}-{number}-{slug}` *(default)*       | `FS-014-user-login` | Number is the stable identifier; slug is descriptive and can be **renamed freely** without breaking citations.   | Two tokens to type; needs `grund id` to allocate the next number.        |
| `{kind}-{number}` (RFC-style)              | `FS-014`            | Maximally stable — no slug to drift. Familiar from RFCs/PEPs/JEPs/ADRs.                                          | Opaque at the call site: `§FS-014` tells you nothing without `grund show`. |
| `{kind}-{slug}` *(`grund` itself uses this)* | `FS-user-login`     | Self-describing — reads like English in prose and code. No number to allocate.                                   | Renaming a slug rewrites every citation. Slug must be unique per kind.   |

Rule of thumb: pick `{kind}-{slug}` until rename churn or ID count starts to hurt; switch to `{kind}-{number}-{slug}` when it does.

Citations use the marker `§`, e.g. `§FS-user-login.3.1`. Type `$$` in a `grund`-aware editor and it's rewritten to `§` automatically. Both marker and trigger are configurable in `.agents/grund.toml`.

## 5. Reviewing code

Before changing or removing a declaration, see what leans on it:

```bash
$ grund refs FS-events.4
docs/architecture/AR-event-bus.md:6: §FS-events.4
src/bus.rs:7: §FS-events.4
```

(The citation list goes to stdout — pipe it like `grund list`. Add `--format=json` for NDJSON.)

Before reviewing a diff, group the citation graph by file so you can join changed files to the specs they touch:

```bash
$ grund cover --format json | jq -c 'select(.path | startswith("src/bus"))'
```

(`grund cover --format json` is NDJSON — one `{"path":…,"citations":[…]}` record per scanned file.)

For an agent reviewing a code change, the loop is mechanical: list the `§…` citations in the changed files, run `grund show` on each, and ask "does the code still match what the spec claims?"

## Install

```bash
cargo install --git https://github.com/vjovanov/grund
```

That puts the `grund` binary on your `PATH`. npm and PyPI bindings are planned — see [`FS-distribution`](docs/functional-spec/FS-distribution.md).

## Set up a repo

```bash
grund init           # writes AGENTS.md and .agents/grund.toml in the cwd
grund init --docs    # also scaffolds docs/ and e2e/ trees
```

`init` is non-interactive and idempotent: re-running never errors on existing files. See [`FS-init`](docs/functional-spec/FS-init.md) for the full state table.

## Pre-commit

This repo ships a ready-to-install [.pre-commit-config.yaml](.pre-commit-config.yaml) — `grund check` for citations, `lychee` for Markdown links:

```bash
pip install pre-commit && cargo install lychee && pre-commit install
```

## Commands

`grund --help` is one screen; `grund <command> --help` is one page with flags, examples, and exit codes.

- **`grund check`** — validate every reference in the tree.
- **`grund show <ID>[.<section>]`** — print one declaration body, for pulling spec content into agent prompts.
- **`grund list`** — the ID catalog.
- **`grund refs <ID>`** — list every citation of a declaration.
- **`grund cover`** — group the citation graph by file, for git-diff recipes.
- **`grund fmt`** — normalize citation syntax (`$$` → `§`, optional Markdown link wrapping).
- **`grund id <KIND> "<title>"`** — emit the next conflict-free ID for a new declaration.
- **`grund init`** — scaffold `AGENTS.md` and `.agents/grund.toml`.
- **`grund config`** — validate or print the effective `.agents/grund.toml`.
- **`grund completions`** — print bash, zsh, or fish completion scripts.
- **`grund agent-setup-instructions`** — print the guided setup workflow for AI agents.

Full surface (flags, JSON shapes, exit codes) in [`docs/functional-spec/`](docs/functional-spec/).

## Agent prompt pattern

The grounding loop, distilled to one rule for an AI agent's system prompt:

> When you see `§<ID>` or `§<ID>.<section>` in any file you are reading, run `grund show <ID>[.<section>]` and treat the output as the authoritative definition. Do not paraphrase or guess — quote what `show` returned, or cite the ID and move on.

That rule plus a clean `grund check` is the whole contract: every reference resolves, every agent fetches the same bytes for the same ID.

## Project layout

`grund` follows its own scheme. Start at [`AGENTS.md`](AGENTS.md), then read down through [`docs/`](docs/):

- [`docs/grund.md`](docs/grund.md) — why this exists
- [`docs/goals/`](docs/goals/) — what we measure ourselves against
- [`docs/roadmap.md`](docs/roadmap.md) — what's next
- [`docs/changelog.md`](docs/changelog.md) — what changed
- [`docs/functional-spec/`](docs/functional-spec/) — external behavior
- [`docs/architecture/`](docs/architecture/) — internals, including the core file layout in `§AR-core-module-layout`
- [`docs/decisions/`](docs/decisions/) — how we got here
- [`e2e/`](e2e/) — executable proof that the spec holds

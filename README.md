# gnd — Ground Your Agents in the Spec

> A small, fast Rust CLI that validates ID-based citations everywhere they live — in your docs *and* in your code's doc-comments — so every agent (human or AI) points at the same facts.

`lychee` and `markdown-link-check` walk `.md` files and validate `[text](url)`. They can't see a `§FS-events.4` cited from `src/bus.rs`. **That gap is what `gnd` exists to close.**

> Status: 0.1.0 Cargo CLI. The core command surface is implemented and self-hosted on this repo; npm / PyPI bindings, the optional LSP server, and watch mode are tracked in [`docs/roadmap.md`](docs/roadmap.md).

## See it in 30 seconds

A spec can live in the doc-comment of the class it describes:

```rust
// src/bus.rs

/// # AS-event-bus: In-process event broadcaster
///
/// Implements the publish-subscribe contract from §FS-events.
/// Slow receivers are dropped silently as required by §FS-events.4.
pub struct EventBus { /* … */ }
```

Rename the spec heading from `FS-events` to `FS-event-stream` and `gnd` flags both citations on the line, across the docs/code boundary, with one resolver:

```
$ gnd check
src/bus.rs:5: unknown reference FS-events
src/bus.rs:6: unknown reference FS-events.4
```

The same citation grammar (`§<ID>` or `§<ID>.<section>`) reads, parses, and resolves identically in Markdown, Rust `///`, Java doc-comments, Python docstrings, Go `//` blocks, JSDoc — every doc-comment form `gnd` knows about. (`FS-events` here is illustrative; `gnd list` shows this repo's real catalogue.)

## Install

```bash
cargo install --git https://github.com/vjovanov/gnd
```

That puts the `gnd` binary on your `PATH`. npm and PyPI bindings are planned — see [`FS-distribution`](docs/functional-spec/FS-distribution.md).

## Set up a repo

```bash
gnd init           # writes agents.md and .agents/gnd.toml in the cwd
gnd init --docs    # also scaffolds docs/ and e2e/ trees
```

`init` is non-interactive and idempotent: re-running never errors on existing files. See [`FS-init`](docs/functional-spec/FS-init.md) for the full state table.

## What it checks

`gnd <path>` scans `<path>`; with no path it scans the canonical layout (`docs/`, `e2e/`, `src/`). In the scanned tree:

1. Every cited ID resolves to a declaration. *(dangling references)*
2. Every section coordinate (`.3.1`) resolves to a heading inside the declaration. *(missing sections)*
3. No ID is declared in two places. *(duplicates)*
4. Every stub heading `# <ID>: [<text>](<path>)` points at a file containing the inline declaration. *(broken stubs)*
5. The `agents.md` / `CLAUDE.md` / `AGENTS.md` entry-point block is up to date. *(stale init)*
6. Declared-but-uncited IDs are flagged. *(unused — warning, not error; `E2E-` cases are exempt)*
7. *(opt-in)* With `[reference] require_grounding = true`: every source file carries at least one citation. *(ungrounded source file)*

A passing repo prints nothing on stdout and exits 0. Errors go to stderr as `<path>:<line>: <message>` so editors and agents jump straight to the source.

`gnd` does **not** check Markdown links, URLs, spelling, or grammar. Use [`lychee`](https://github.com/lycheeverse/lychee), `vale`, etc. for those.

## Pre-commit

This repo ships a ready-to-install [.pre-commit-config.yaml](.pre-commit-config.yaml) — `gnd check` for citations, `lychee` for Markdown links:

```bash
pip install pre-commit && cargo install lychee && pre-commit install
```

## Commands

`gnd --help` is one screen; `gnd <command> --help` is one page with flags, examples, and exit codes. The headline subcommands:

- **`gnd check`** — validate every reference in the tree.
- **`gnd show <ID>[.<section>]`** — print one declaration body, for pulling spec content into agent prompts.
- **`gnd refs <ID>`** — list every citation of a declaration, so you know what leans on it before you change it.
- **`gnd list`** — the ID catalog.
- **`gnd name <KIND> "<title>"`** — emit the next conflict-free ID for a new declaration.
- **`gnd fmt`** — normalize citation syntax (`$$` → `§`, optional Markdown link wrapping).
- **`gnd cover`** — group the citation graph by file, for git-diff recipes.
- **`gnd init`** — scaffold `agents.md` and `.agents/gnd.toml`.

Full surface (flags, JSON shapes, exit codes) in [`docs/functional-spec/`](docs/functional-spec/).

## ID format

```
<KIND>-<NNN>-<slug>[.<section>]
```

`KIND` is one of (configurable):

- **`G`** — goal (declared in `docs/goals/goals.md`)
- **`FS`** — functional spec (external behavior)
- **`AS`** — architectural spec (internals; can live inline in a class doc-comment)
- **`DF`** / **`DA`** — functional / architectural decision records
- **`E2E`** — end-to-end test
- **`RM`** — roadmap milestone

Three ID schemes are supported — `{kind}-{number}-{slug}`, `{kind}-{number}` (RFC-style), `{kind}-{slug}` (`gnd` itself uses this). Each has a runnable tiny repo under [`examples/`](examples/).

Citations are written with the marker `§`, e.g. `§FS-user-login.3.1`. Type `$$` in a `gnd`-aware editor and it's rewritten to `§` automatically. Both marker and trigger are configurable in `.agents/gnd.toml`.

## Agent prompt pattern

The grounding loop for an AI agent is one rule in its system prompt:

> When you see `§<ID>` or `§<ID>.<section>` in any file you are reading, run `gnd show <ID>[.<section>]` and treat the output as the authoritative definition. Do not paraphrase or guess — quote what `show` returned, or cite the ID and move on.

That rule plus a clean `gnd check` is the entire contract: every reference resolves, every agent fetches the same bytes for the same ID.

## Project layout

`gnd` follows its own scheme. Start at [`agents.md`](agents.md), then read down through [`docs/`](docs/):

- [`docs/raison-detre.md`](docs/raison-detre.md) — why this exists
- [`docs/goals/`](docs/goals/) — what we measure ourselves against
- [`docs/roadmap.md`](docs/roadmap.md) — what's next
- [`docs/changelog.md`](docs/changelog.md) — what changed
- [`docs/functional-spec/`](docs/functional-spec/) — external behavior
- [`docs/architectural-spec/`](docs/architectural-spec/) — internals
- [`docs/decisions/`](docs/decisions/) — how we got here
- [`e2e/`](e2e/) — executable proof that the spec holds

## License

[MIT](LICENSE).

# gnd: Ground Your Agents in the Spec

**The polyglot reference checker.** A small, fast Rust CLI that validates ID-based citations everywhere they live — in your docs, *and* in your code's doc-comments — so every agent (human or AI) is pointing at the same facts.

A citation like `§FS-user-login.3.1` works the same way in a Markdown spec, a Rust `///` line, a Java doc-comment, a Python docstring, or a Go doc block. `gnd` walks all of them with one resolver. `lychee` and `markdown-link-check` can't — they only read `.md` and only validate `[text](url)`. That gap is what `gnd` is for.

The three things `gnd` does that plain Markdown links can't:

1. **Verify** citations across the docs/code boundary. A `§FS-events.4` cited from `src/bus.rs` is checked the same as one cited from `docs/`.
2. **Survive refactors.** IDs are location-independent — rename a file, reword a heading, move a module, citations keep resolving. Markdown anchors break on the first heading edit.
3. **Extract** a single declaration body. `gnd show FS-user-login.3.1` returns just that subsection — under 200 lines — so an LLM agent can pull a fact into context without loading the file.

> Status: 0.1.0 Cargo CLI. The core command-line surface is implemented and self-hosted; npm/PyPI bindings, the optional LSP server, and watch mode are tracked in `docs/roadmap.md`.

## What it Checks

When you run `gnd <path>`:

1. Every cited ID resolves to a declaration. *(dangling references)*
2. Every section coordinate (`.3.1`) resolves to a heading inside the declaration. *(missing sections)*
3. No ID is declared in two places. *(duplicates)*
4. Every stub heading (`# <ID>: [<text>](<path>)`) points at a file that actually contains the inline declaration. *(broken stubs)*
5. If `agents.md` or a known standalone companion entrypoint such as `AGENTS.md`, `AGENTS.override.md`, `CLAUDE.md`, `.claude/CLAUDE.md`, `GEMINI.md`, or `.github/copilot-instructions.md` is present, it carries an up-to-date `gnd init` block. *(uninitialized / stale agent entry point — run `gnd init`)*
6. Declared-but-uncited IDs are flagged. *(unused — warning, not error; `E2E-` cases are exempt — a test is used by being run, not cited)*
7. *(opt-in)* With `[reference] require_grounding = true` (or `gnd check --require-grounding`): every source file carries at least one citation to a declared ID — or declares one inline. *(ungrounded source file — so a reviewer changing a spec sees every file that leans on it, because they all must cite it)*

It does **not** check markdown links, URLs, spelling, or grammar. Use [`lychee`](https://github.com/lycheeverse/lychee), `vale`, etc. for those.

## ID format

```
<KIND>-<NNN>-<slug>[.<section>]
```

`KIND` is one of (configurable):

- `G` — **goal.** What we measure ourselves against. Declared as a heading inside `docs/goals/goals.md` (one file, all goals inline). Resolves to a short, observable success criterion — typically a paragraph or two.
- `FS` — **functional spec.** External behavior — what the system does. Declared as the H1 of a file in `docs/functional-spec/`. Resolves to that file's body: numbered sections describing inputs, outputs, behavior, exit codes.
- `AS` — **architectural spec.** Internals — components, boundaries, data flow. Declared as the H1 of a file in `docs/architectural-spec/`, **or** inline in a class/module-level doc-comment with a one-line stub at `docs/architectural-spec/AS-….md` whose H1 is `# <ID>: [<path>](<path>)`. Resolves to the spec body, with comment markers stripped when the home is in code.
- `DF` — **functional decision.** Append-only record of a choice that shaped the *what*. Declared as the H1 of a file in `docs/decisions/functional/`. Resolves to a dated entry: options considered, chosen path, rationale.
- `DA` — **architectural decision.** Append-only record of a choice that shaped the *how*. Declared as the H1 of a file in `docs/decisions/architectural/`. Resolves to the same shape as `DF`.
- `E2E` — **end-to-end test.** Executable proof that a functional spec holds. Declared per case directory under `e2e/cases/<id>/`. Resolves to the case's fixtures (`repo/`, `expected.stdout`, `expected.stderr`, `expected.exit`) — the test *is* the body.
- `RM` — **roadmap milestone.** A reviewable unit of upcoming work. Declared as an H2 heading inside `docs/roadmap.md` (one file, all milestones inline), parallel to how `G` lives in `docs/goals/goals.md`. Resolves to that milestone's *what / why now / measurable* block. Shipped milestones keep a one-line declaration so `§RM-…` citations from commits and the changelog don't dangle.

Section coordinates (`.3.1`) resolve to a heading **inside** the declaration body — `### 3.1 …` for a markdown declaration, or the same heading shape inside a doc-comment for an inline one. `gnd show <ID>.<section>` returns just that subtree.

Citations are written prefixed by the marker `§`, e.g. `§FS-user-login.3.1`. Type `$$` in a `gnd`-aware editor and it's rewritten to `§` automatically. Both marker and trigger are configurable in `.agents/gnd.toml`.

### Supported ID schemes

`format` in `[id]` (see [`FS-config`](docs/functional-spec/FS-config.md#32-id--id-grammar)) selects which ID shape the repo uses. Pick one per repo and keep it stable — citations look identical across schemes but resolve under different rules.

| `format`                       | Example                  | Best when…                                                                  | Trade-off                                                                       | Runnable example                                                  |
|--------------------------------|--------------------------|-----------------------------------------------------------------------------|---------------------------------------------------------------------------------|-------------------------------------------------------------------|
| `{kind}-{number}-{slug}`       | `FS-014-event-bus`       | You want stable refs *and* a topic hint readable in prose.                  | Two facts to maintain — a title edit leaves the slug stale until re-slugged.    | [`examples/scheme-numbered-slug/`](examples/scheme-numbered-slug/) |
| `{kind}-{number}`              | `RFC-014`, `FS-002`      | You want the shortest possible ID and don't mind opacity (RFC-/JEP-style).  | Citations are unreadable without `gnd show` — punishes drive-by review.         | [`examples/scheme-numbered/`](examples/scheme-numbered/)           |
| `{kind}-{slug}`                | `FS-event-bus`           | You want self-describing IDs with no number bookkeeping. (`gnd` itself uses this.) | Slugs must be unique within a kind; renaming a spec breaks existing cites.      | [`examples/scheme-slug/`](examples/scheme-slug/)                   |

Each subfolder is a tiny self-contained repo plus golden `expected.*` files — `gnd examples/scheme-slug/repo` prints nothing and exits 0. See [`examples/README.md`](examples/README.md) for the full list.

## Install

```bash
cargo install gnd                                  # from crates.io
cargo install --path .                             # from a clone
cargo install --git https://github.com/vjovanov/gnd  # pin to a git ref
```

This puts the `gnd` binary on your `PATH`. The npm and PyPI packages are planned — see [Distribution](#distribution) below.

## Try it

From a clone, without installing:

```bash
cargo build --release
./target/release/gnd .
```

A passing repo prints nothing on stdout and exits 0. Errors go to stderr as `<path>:<line>: <message>` so editors and agents can jump straight to the source.

## Set up a repo

```bash
gnd init                       # writes agents.md and .agents/gnd.toml in the cwd
gnd init --docs                # also scaffolds docs/ and e2e/ trees in the cwd
gnd init --docs path/to/repo   # any form accepts a target path; default is .
gnd init --force               # rewrite agents.md and .agents/gnd.toml from the canonical templates
```

`init` is non-interactive and **idempotent**: re-running it never errors on existing files. If `agents.md` is already present, `init` either appends a versioned `<!-- gnd:init:agents:v1 ... -->` block (when none is there yet), updates an older block in place, or leaves a current block untouched. Every other target file (`.agents/gnd.toml`, the `--docs` scaffolds) is reported `exists` and left as-is unless `--force` is passed. See [`FS-init`](docs/functional-spec/FS-init.md) for the full state table.

### Pre-commit hook

Run `gnd check` before each commit so dangling references never land on `main`, and run `lychee` beside it so normal Markdown links stay valid. This repo carries a ready-to-install [.pre-commit-config.yaml](.pre-commit-config.yaml):

```bash
pip install pre-commit
cargo install lychee
pre-commit install
```

Or as a plain `.git/hooks/pre-commit`:

```bash
#!/bin/sh
set -e
gnd check
lychee --no-progress README.md docs
```

`gnd check` exits 0 on a clean repo and non-zero on the first dangling, duplicate, broken stub, ungrounded source file, or stale init block. `lychee` exits non-zero on broken Markdown links or URLs.

## Subcommands

Commands with machine-readable result modes document `--format text|json` in their synopsis. `gnd <command> --help` prints that command's page — flags, examples, exit codes.

| Command                    | What it does                                                                       |
| -------------------------- | ---------------------------------------------------------------------------------- |
| `gnd check [path] [--require-grounding]` | Validate references. The default — `gnd <path>` is shorthand. `--require-grounding` adds the opt-in check that every source file cites a declared ID (§3.6 of [`FS-check`](docs/functional-spec/FS-check.md); also settable as `[reference] require_grounding`). (`--watch`, a resident re-check on every change, is specified in `FS-check` §6 but not yet implemented.) |
| `gnd init [path] [--docs] [--name N] [--force\|--append]` | Scaffold `agents.md` and `.agents/gnd.toml`; idempotent by default — appends/updates the managed block in an existing `agents.md`, reports `exists` for other files. `--docs` also seeds `docs/` and `e2e/`; `--name` sets the project name; `--force` overwrites. |
| `gnd show <ID>[.<section>] [path] [--section S] [--head\|--full] [--format text\|md\|json]` | Print just the body of a declaration (or one of its sections), for pulling spec content into agent prompts. `--head` is the lead paragraph only; `--full` (default) is the whole body; `md` keeps the heading line. |
| `gnd list [path] [--kind K] [--unused] [--format text\|json]` | The ID catalog — every declared ID, `<ID>  path:line  title`, sorted by ID. `--kind` filters by prefix; `--unused` shows declarations nothing cites yet; `json` adds a `refs` count. The thing `gnd show` reads from. |
| `gnd refs <ID>[.<section>] [path] [--section S] [--format text\|json]` | List every citation of an ID — `path:line: <citation>` — so you know what leans on a declaration before you change it. `--section` narrows to citations of one section. |
| `gnd cover [path] [--format text\|json]` | Group the citation graph by scanned file. JSON emits one record per file, including files with no citations, so git-diff recipes can join changed files to the specs they cite. |
| `gnd fmt [path] [--check\|--write] [--marker] [--md-links]` | Normalize citation syntax: rewrite the `$$` trigger to `§`. `--marker` also upgrades bare `<ID>` tokens to `§<ID>`; `--md-links` also wraps citations in `.md` files as clickable links to the declaration. Default is a dry run (`--check`); `--write` applies the changes. |
| `gnd name <KIND> "<title>" [path] [--width N] [--explain] [--format text\|json]` | Emit the next conflict-free ID for a new declaration (e.g. `FS-008-user-login`, or `FS-user-login` under a number-less `[id] format`). Pure function from `(kind, title, tree)` to `id`; no files are written. `--explain` adds a one-line "where to put the file" hint on stderr (stdout stays the bare ID). |
| `gnd config (validate\|show) [path]` | `validate` checks the discovered `.agents/gnd.toml` against the schema; `show` prints the effective config (defaults + file) as TOML. |
| `gnd agent-setup-instructions` | Print the AI-agent setup guide embedded from `skills/gnd-init/SKILL.md`, so agents with only the installed binary can still perform guided `gnd init` adoption. |
| `gnd completions <bash\|zsh\|fish>` | Print a shell completion script; generated scripts complete declared IDs for `gnd show <ID>` and `gnd refs <ID>`. |
| `gnd --version` / `gnd --help` / `gnd help <command>` | Print the version, the one-screen top-level usage, or one command's page (its flags, examples, exit codes); all handled before any scan. |

Full surface in [`docs/functional-spec/`](docs/functional-spec/).

## Distribution

Today, `gnd` ships as a single cargo crate. The plan is one engine across three registries with an idiomatic API on each (tracked in [`docs/roadmap.md`](docs/roadmap.md)):

- **cargo** — `gnd` (library + binary) — *shipping now* (`cargo install gnd`)
- **npm** — `gnd-cli` (prebuilt binary + Node API via `napi-rs`) — *planned*
- **PyPI** — `gnd` (Python API via PyO3, wheels via `maturin`) — *planned; package name re-verified before first publish per `RM-distribution-naming`*

See [`FS-distribution`](docs/functional-spec/FS-distribution.md).

## Example

Two spec files in a small `gnd` repo, citing each other:

```markdown
# docs/functional-spec/FS-check.md
# FS-check: gnd validates every reference in a repo

Walks a repo and reports every violation. Companion read path is
FS-show. Tracked under G-clarity.

## 1. Inputs

Optional path argument; defaults to the current directory.
```

```markdown
# docs/functional-spec/FS-show.md
# FS-show: gnd reads a single declaration body by ID

Prints the body of a declaration, given an ID. Default path matches
FS-check.1.
```

`gnd .` reports the one dangling citation and exits non-zero:

```
docs/functional-spec/FS-check.md:4: unknown reference G-clarity
```

`FS-show` resolves to the second file. `FS-check.1` resolves to the `## 1. Inputs` heading. Only `G-clarity` has no declaration anywhere in the tree.

## Example: spec in code

An architectural spec often reads better next to the class it describes. `gnd` lets the spec live inline in the class doc-comment, with a one-line stub in `docs/` so the ID is still discoverable from the tree.

The stub under `docs/`:

```markdown
# docs/architectural-spec/AS-event-bus.md
# AS-event-bus: [src/bus.rs](src/bus.rs)
```

The class doc-comment in code, citing back into the functional spec:

```rust
// src/bus.rs

/// # AS-event-bus: In-process event broadcaster
///
/// Implements the publish-subscribe contract from §FS-events.
/// Slow receivers are dropped silently as required by §FS-events.4.
///
/// ## 1. Topology
///
/// One sender, many receivers. Senders never block.
pub struct EventBus { /* … */ }
```

Two references cross the docs/code boundary in opposite directions:

- **Doc → code:** `AS-event-bus.md` declares the ID; the link in `# AS-event-bus: [src/bus.rs](src/bus.rs)` tells `gnd` the body lives in the Rustdoc on `EventBus`. `gnd show AS-event-bus` strips the `///` markers and prints the Rustdoc prose.
- **Code → doc:** the Rustdoc cites `§FS-events` and `§FS-events.4`. Those have to resolve to a markdown declaration under `docs/functional-spec/`. If `FS-events` is renamed or deleted, `gnd check` flags `src/bus.rs` immediately — even though the spec lives in `.md` and the cite lives in `.rs`.

The same `gnd check` walks both files, treats the doc-comment as prose, and validates every citation in either direction.

## Reading a reference

When an agent (human or AI) sees a citation in code or docs — say `§FS-check.1` in a comment — it pulls the grounded body with `gnd show`:

```bash
$ gnd show FS-check.1
Optional path argument; defaults to the current directory.
```

Skim the lead paragraph of a declaration without loading sections:

```bash
$ gnd show --head FS-show
Prints the body of a declaration, given an ID. Default path matches
FS-check.1.
```

The whole declaration:

```bash
$ gnd show FS-check
# FS-check: gnd validates every reference in a repo

Walks a repo and reports every violation. Companion read path is
FS-show. Tracked under G-clarity.

## 1. Inputs

Optional path argument; defaults to the current directory.
```

`show` prints prose verbatim — it does not strip cites that `check` would flag. A dangling citation in a fetched body is information, not noise: the agent sees what the spec actually claims.

The same works when the declaration lives inline in source — a Rustdoc, Javadoc, or Python docstring containing `# AS-event-bus: …`. `gnd show AS-event-bus` strips the comment markers and returns the prose, identical to what an IDE hover would render. The stub at `docs/architectural-spec/AS-event-bus.md` is a single-line H1 — `# AS-event-bus: [<path>](<path>)` — and `show` follows the link.

JSON for tooling:

```bash
$ gnd show --format json FS-check.1
{"id":"FS-check","section":"1","body":"Optional path argument; defaults to the current directory.\n","path":"docs/functional-spec/FS-check.md","line":7}
```

Errors are bare lines on stderr with empty stdout — exit `1` for a missing ID or missing section, exit `1` with `ambiguous ID: …` if duplicates exist (run `gnd check` first). See [`FS-show`](docs/functional-spec/FS-show.md).

### Agent prompt pattern

The grounding loop for an AI agent is a single rule in its system prompt:

> When you see `§<ID>` or `§<ID>.<section>` in any file you are reading, run `gnd show <ID>[.<section>]` and treat the output as the authoritative definition. Do not paraphrase or guess — quote what `show` returned, or cite the ID and move on.

That rule plus a clean `gnd check` is the entire contract: every reference resolves, and every agent fetches the same bytes for the same ID.

## Verifying what a file refers to

Before changing a file, an agent typically wants to know two things: *which specs does this file claim to be grounded in*, and *do those claims still hold*. Both are mechanical. The walkthrough below uses the same hypothetical `src/bus.rs` from the [spec-in-code example](#example-spec-in-code) — a file whose Rustdoc declares `AS-event-bus` and cites `§FS-events` back into the docs — because it is small enough to read in one screen. For a real, much larger instance, `gnd`'s own `src/lib.rs` carries hundreds of `§…` doc-comment citations into `docs/`, and this repo runs with `[reference] require_grounding = true` (§FS-check.3.6) so *every* source file under `[scan] include` is required to carry at least one — `gnd cover src/`, `gnd refs <ID>`, and `gnd show <ID>` all work against this repo as-is. (The remaining step is the engine split into per-component modules each owning its own `# AS-…:` inline declaration — tracked as `RM-core-cli-split` in `docs/roadmap.md`.)

### List the citations in a file

The `§…-…` grammar is the same across schemes, so one `grep` finds every cite regardless of whether the repo numbers its IDs:

```bash
$ grep -oE '§[A-Z][A-Z0-9]*-[a-z0-9-]+(\.[0-9.]+)?' src/bus.rs | sort -u
§FS-events
§FS-events.4
§G-fast-feedback
```

Three cites. The agent now knows exactly which declarations the file leans on.

### Validate those references

`gnd check .` walks the whole tree, so it resolves a cite in `src/bus.rs` against a declaration anywhere under `docs/` and reports the ones that don't:

```bash
$ gnd check .
$ echo $?
0
```

Silent + exit 0 means every cite in the repo — including the ones in `src/bus.rs` — resolves. A regression in that file looks like:

```bash
$ gnd check .
src/bus.rs:14: unknown reference FS-events.9.9
src/bus.rs:18: section 4.7 not found in FS-events
```

(`gnd check` also accepts a narrower path — `gnd check src/bus.rs` or `gnd check docs/` — which scans just that subtree. That is faster, but it only resolves cites whose declarations also live under the scanned path; for a file that cites specs elsewhere in the repo, the whole-tree `gnd check .` is the one that proves the cites hold.)

### Fetch each cited body and compare against the code

This is the verification step — the agent pulls the spec text and checks the code matches:

```bash
$ for id in $(grep -oE '§[A-Z][A-Z0-9]*-[a-z0-9-]+(\.[0-9.]+)?' src/bus.rs | tr -d '§' | sort -u); do
    echo "=== $id ==="
    gnd show "$id"
  done
```

Now the agent can answer concrete questions: does `src/bus.rs` actually drop slow receivers the way `FS-events.4` requires? If the spec says receivers that fall behind are disconnected but the code blocks the sender instead, the file's claim to ground itself in `§FS-events.4` is a lie — and the verification surfaces it.

### Find which file owns a declaration

The reverse direction — *who declares this ID?* — is a single command and uses the JSON shape so the agent can program against it:

```bash
$ gnd show --format json AS-event-bus | jq -r '.path + ":" + (.line|tostring)'
src/bus.rs:42
```

If a stub at `docs/architectural-spec/AS-event-bus.md` reads `# AS-event-bus: [src/bus.rs](src/bus.rs)`, `show` follows the link; the path printed is the inline declaration's home, not the stub. An agent verifying that a spec's prose still matches its implementation always lands on the file it actually needs to read.

### Find every file that cites a declaration

The other reverse direction — *who depends on this ID?*, the question to ask before changing or removing a declaration — is `gnd refs`. It shares the scanner with `gnd check`, so it sees exactly the citations the checker validates (respects `strict` mode, skips ID-shaped substrings inside string literals, reaches into block doc-comments) — things a bare `grep` cannot:

```bash
$ gnd refs FS-events.4
docs/architectural-spec/AS-event-bus.md:6: §FS-events.4
src/bus.rs:14: §FS-events.4
```

Empty output, exit 0 means nothing cites it yet (`gnd check` will also warn about that). See [`FS-refs`](docs/functional-spec/FS-refs.md).

## Project layout

`gnd` follows its own scheme. Start at [`agents.md`](agents.md), then read down through [`docs/`](docs/):

- [`docs/raison-detre.md`](docs/raison-detre.md) — why this exists
- [`docs/goals/`](docs/goals/) — what we measure ourselves against
- [`docs/roadmap.md`](docs/roadmap.md) — what's next, with IDed milestones
- [`docs/changelog.md`](docs/changelog.md) — what changed, latest release inline
- [`docs/functional-spec/`](docs/functional-spec/) — external behavior
- [`docs/architectural-spec/`](docs/architectural-spec/) — internals
- [`docs/decisions/`](docs/decisions/) — how we got here
- [`docs/discussions/`](docs/discussions/) — ideas still open, as `DISC-*` proposals (`DISC` is a project-local `[[kinds]]` entry in this repo's `.agents/gnd.toml`, not one of the canonical defaults)
- [`e2e/`](e2e/) — executable proof that the spec holds

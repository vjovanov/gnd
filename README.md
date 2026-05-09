# gnd: Ground Your Agents in the Spec

**Agent grounding tool.** A small, fast Rust CLI that keeps every agent — human or AI — pointing at the same facts.

`gnd` validates an ID-based reference scheme across your docs, tests, and source. Things like `§FS-user-login.3.1` aren't markdown links and aren't URLs, so `lychee` can't check them. `gnd` does.

> Status: early. The spec under `docs/` is the source of truth; the binary is being implemented against it. `gnd` is its own first user — running `gnd .` at the repo root checks the project's own specs.

## What it Checks

When you run `gnd <path>`:

1. Every cited ID resolves to a declaration. *(dangling references)*
2. Every section coordinate (`.3.1`) resolves to a heading inside the declaration. *(missing sections)*
3. No ID is declared in two places. *(duplicates)*
4. Every stub heading (`# <ID>: [<text>](<path>)`) points at a file that actually contains the inline declaration. *(broken stubs)*
5. Declared-but-uncited IDs are flagged. *(unused — warning, not error)*

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

Section coordinates (`.3.1`) resolve to a heading **inside** the declaration body — `### 3.1 …` for a markdown declaration, or the same heading shape inside a doc-comment for an inline one. `gnd show <ID>.<section>` returns just that subtree.

Citations are written prefixed by the marker `§`, e.g. `§FS-user-login.3.1`. Type `$$` in a `gnd`-aware editor and it's rewritten to `§` automatically. Both marker and trigger are configurable in `.agents/gnd.toml`.

## Try it

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

Run `gnd check` before each commit so dangling references never land on `main`. With [pre-commit](https://pre-commit.com/), add to `.pre-commit-config.yaml`:

```yaml
- repo: local
  hooks:
    - id: gnd-check
      name: gnd check
      entry: gnd check
      language: system
      pass_filenames: false
```

Or as a plain `.git/hooks/pre-commit`:

```bash
#!/bin/sh
exec gnd check
```

`gnd check` exits 0 on a clean repo and non-zero on the first dangling, duplicate, or broken stub — the commit is aborted with the offending `<path>:<line>: <message>` on stderr.

## Subcommands

| Command                    | What it does                                                                       |
| -------------------------- | ---------------------------------------------------------------------------------- |
| `gnd check [path]`         | Validate references. The default — `gnd <path>` is shorthand.                      |
| `gnd init [path] [--docs] [--force]` | Scaffold `agents.md` and `.agents/gnd.toml`; idempotent by default — appends/updates the managed block in an existing `agents.md`, reports `exists` for other files. `--docs` also seeds `docs/` and `e2e/`; `--force` overwrites. |
| `gnd show <ID> [--head]`   | Print just the body of a declaration, for pulling spec content into agent prompts. |
| `gnd fmt [path]`           | Rewrite `$$` triggers to `§`; with `--marker`, also upgrade bare citations.        |

Full surface in [`docs/functional-spec/`](docs/functional-spec/).

## Distribution (planned)

One engine, three registries, idiomatic API on each:

- **cargo** — `gnd` (library + binary)
- **npm** — `gnd-cli` (prebuilt binary + Node API via `napi-rs`)
- **PyPI** — `gnd` (Python API via PyO3, wheels via `maturin`)

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

Before changing a file, an agent typically wants to know two things: *which specs does this file claim to be grounded in*, and *do those claims still hold*. Both are mechanical.

### List the citations in a file

```bash
$ grep -oE '§[A-Z]+-[0-9]+-[a-z0-9-]+(\.[0-9.]+)?' src/scanner.rs | sort -u
§AS-scanner.2.1
§AS-scanner.4
§FS-check.1.1
§G-fast-feedback
```

Four cites. The agent now knows exactly which declarations the file leans on.

### Validate just those references

`gnd check` accepts a path. Scoping it to the file under review is faster than a whole-repo scan and proves no cite in *this* file is dangling:

```bash
$ gnd check src/scanner.rs
$ echo $?
0
```

Silent + exit 0 means every cite resolves. A regression looks like:

```bash
$ gnd check src/scanner.rs
src/scanner.rs:142: unknown reference FS-check.9.9
src/scanner.rs:201: section 4.7 not found in AS-scanner
```

### Fetch each cited body and compare against the code

This is the verification step — the agent pulls the spec text and checks the code matches:

```bash
$ for id in $(grep -oE '§[A-Z]+-[0-9]+-[a-z0-9-]+(\.[0-9.]+)?' src/scanner.rs | tr -d '§' | sort -u); do
    echo "=== $id ==="
    gnd show "$id"
  done
```

Now the agent can answer concrete questions: does `src/scanner.rs` actually implement the doc-comment forms enumerated in `AS-scanner.4`? If the spec lists Javadoc, JSDoc, Rustdoc, Python docstrings, and Go `//` blocks, but the code only handles three of them, the file's claim to ground itself in `§AS-scanner.4` is a lie — and the verification surfaces it.

### Find which file owns a declaration

The reverse direction — *who declares this ID?* — is a single command and uses the JSON shape so the agent can program against it:

```bash
$ gnd show --format json AS-event-bus | jq -r '.path + ":" + (.line|tostring)'
src/bus.rs:42
```

If a stub at `docs/architectural-spec/AS-event-bus.md` reads `# AS-event-bus: [src/bus.rs](src/bus.rs)`, `show` follows the link; the path printed is the inline declaration's home, not the stub. An agent verifying that a spec's prose still matches its implementation always lands on the file it actually needs to read.

## Project layout

`gnd` follows its own scheme. Start at [`agents.md`](agents.md), then read down through [`docs/`](docs/):

- [`docs/raison-detre.md`](docs/raison-detre.md) — why this exists
- [`docs/goals/`](docs/goals/) — what we measure ourselves against
- [`docs/roadmap.md`](docs/roadmap.md) — what's next, with IDed milestones
- [`docs/changelog.md`](docs/changelog.md) — what changed, latest release inline
- [`docs/functional-spec/`](docs/functional-spec/) — external behavior
- [`docs/architectural-spec/`](docs/architectural-spec/) — internals
- [`docs/decisions/`](docs/decisions/) — how we got here
- [`e2e/`](e2e/) — executable proof that the spec holds

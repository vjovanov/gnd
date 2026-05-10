<!-- gnd:init:agents:v1 begin -->
# {NAME} — agent instructions

This file is the entry point for any agent (human or AI) working on **{NAME}**. Read it first, then read the declared artifacts it points to before making changes.

This project uses the [`gnd`](https://github.com/vjovanov/gnd) reference scheme: every spec, goal, decision, and end-to-end test has a stable ID of the form `{ID_SHAPE}`, declared as a heading inside its home file. Citations are written prefixed by the marker `{MARKER}`, e.g. `{CITE_EXAMPLE}` (the `{ID_EXAMPLE}` here is an illustration of the *shape*, not a real ID in this repo). Section paths can be arbitrary depth — `.3`, `.3.1`, `.3.1.2` are all valid as long as a heading at that depth exists in the declaration. Run `gnd check` to validate every citation; run `gnd list` to see every declared ID; run `gnd show <ID>` to print just the body of one declaration. (`gnd` documents its own `check`, `list`, and `show` contract under [`docs/functional-spec/`](https://github.com/vjovanov/gnd/tree/main/docs/functional-spec) in the `gnd` repo — that's `gnd`'s spec, not this project's; only IDs declared *in this repo* resolve with `gnd show` here.)

## Grounding yourself in the spec

A `{MARKER}<ID>` — or `{MARKER}<ID>.<section>` — is a pointer to a fact, not a file path. When a doc, a code comment, or a review note cites one, resolve it with `gnd` instead of opening the file and skimming:

- `gnd show <ID>` — the full declaration body (a spec file's contents, a goal's success criterion, a decision record).
- `gnd show <ID>.<section>` — just that subsection, so you pull one fact into context without loading the whole file. This is the cheap, precise move — prefer it.
- `gnd show <ID> --head` — the lead paragraph only, for a quick "what is this about" before deciding whether to read more.
- `gnd list` — every declared ID, when you need to discover the right `<ID>`. `gnd refs <ID>` — every place that cites it, so you know what leans on a declaration before you change it.

## How to use this file

1. Start with the project knowledge map below, then open the relevant declaration with `gnd show <ID>` before making changes.
2. When you learn something new about *why*, *where*, *what*, *how*, or *how we got here* — write it down in the matching declared artifact. Don't hoard context.
3. When you make a non-obvious decision, add it to the configured decision or record location and cite the ID from the spec, code, or test it affects.
4. Behavior is proven by executable tests or cases, not by prose alone. Every functional change ships with a matching proof in the repo's configured test artifacts.

## Project knowledge map

`gnd` scans: {SCAN_SCOPE}.

Configured declaration homes:

{DECLARATION_TABLE}

These paths come from `.agents/gnd.toml`. If this repo's specs, roadmaps, changelogs, decisions, plans, tests, or examples live somewhere else, update the config first and re-run `gnd init` so this guidance stays aligned with the repository instead of introducing a parallel layout.

## References

The `gnd` ID scheme in this repo: `{ID_SHAPE_SEC}`, where `KIND` ∈ `{KINDS_SET}` — the kinds and the ID/marker syntax are configurable in `.agents/gnd.toml` (run `gnd config show` to see the effective settings). Citations are written prefixed by the marker `{MARKER}` — type `{TRIGGER}` in a `gnd`-aware editor and it becomes `{MARKER}` automatically. {BARE_TOKEN_NOTE}

Declarations are heading lines: `# {ID_EXAMPLE}: A player can log in …` in a markdown file (again, `{ID_EXAMPLE}` is just the shape), or the same shape inside a code doc-comment (Javadoc, JSDoc, Rustdoc, Python docstring, Go `//` block, etc.). A declaration can also live directly in a class-level or module-level doc-comment, with a one-line stub in that kind's configured home whose H1 is `# <ID>: [<path>](<path>)` (a markdown link to the file with the inline declaration). The exhaustive list of supported doc-comment forms is in [`gnd`'s own architectural spec](https://github.com/vjovanov/gnd/tree/main/docs/architectural-spec).

Code back-references the spec it realizes. When a function, class, or block implements a behavior, it carries a `{MARKER}<ID>` citation — on its doc-comment for a whole behavior, or on an inline comment beside the line that enforces one clause (`{MARKER}<ID>.2.1`) or honors one decision. Cite at the granularity you implement: the behavior on the doc-comment, the clause on the `if` that checks it, the decision ID on the literal it pinned. Each citation is one more edge `gnd refs <ID>` reports, so a reviewer changing a spec sees exactly which code leans on it — closing the loop goals ← specs ← architecture ← code, alongside specs ← executable tests.

## Rules for agents

- **Citations climb toward the goals.** Specs cite goals. Architecture cites specs. Code cites the specs it implements. Executable tests or cases cite the behavior they verify.
- **Refresh cited specs before editing code.** Before editing code that carries a `{MARKER}<ID>` or `{MARKER}<ID>.<section>` citation, run `gnd show <ID>` or `gnd show <ID>.<section>` for the cited behavior and keep that output in context while making the change.
- **No dangling decisions.** Every decision record is cited from the spec or architecture doc it shaped, at the point where the choice applies — so a reader lands on the *why* without searching. A decision may also cite back into a spec; what it may not be is uncited — `gnd check` flags that as unused.
- **Decisions are append-only.** Never rewrite decision history. If a decision is reversed, add a new entry that supersedes the old one and link both ways.
- **Cross-link everything via IDs.** Use the ID. No markdown links between docs.
- **Executable tests are the source of truth for behavior.** When the spec and the executable proof disagree, one of them is wrong — fix both in the same change.
- **Run `gnd check` before you commit.** A dangling reference is a stop-the-line bug; `gnd check`'s output names the file and line for each one.

This managed agent guidance block and the accompanying `.agents/gnd.toml` were generated by `gnd init`. Re-run `gnd init --force` to refresh them at the current `gnd` version.
<!-- gnd:init:agents:v1 end -->

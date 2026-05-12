<!-- grund:init:agents:v2 begin -->
# repo — agent instructions

This file is the entry point for any agent (human or AI) working on **repo**. Read it first, then read the declared artifacts it points to before making changes.

This project uses the [`grund`](https://github.com/vjovanov/grund) reference scheme: every spec, goal, decision, and end-to-end test has a stable ID of the form `<KIND>-<NNN>-<slug>`, declared as a heading inside its home file. Citations are written prefixed by the marker `§`, e.g. `§FS-042-user-login.3.1` (the `FS-042-user-login` here is an illustration of the *shape*, not a real ID in this repo). Section paths can be arbitrary depth — `.3`, `.3.1`, `.3.1.2` are all valid as long as a heading at that depth exists in the declaration. Run `grund check` to validate every citation; `grund show <ID> --brief` to read the lead prose plus section map for a bare cited ID; `grund show <ID>.<section>` to print one section; `grund list` or `grund list --kind FS,AR` to discover IDs; `grund refs <ID> --summary` to see the file-level blast radius before you change or move a declaration. (`grund` documents its own `check`, `show`, `list`, and `refs` contract under [`docs/functional-spec/`](https://github.com/vjovanov/grund/tree/main/docs/functional-spec) in the `grund` repo — that's `grund`'s spec, not this project's; only IDs declared *in this repo* resolve with `grund show` here.)

## Grounding yourself in the spec

A `§<ID>` — or `§<ID>.<section>` — is a pointer to a fact, not a file path. When a doc, a code comment, or a review note cites one, resolve it with `grund` instead of opening the file and skimming:

- `grund show <ID> --brief` — for a bare cited `§<ID>`, read the lead prose plus section map first.
- `grund show <ID>.<section>` — for a citation that already names a section, read just that subsection.
- `grund show <ID>` — the full declaration body, when the brief or section slice is not enough.
- `grund show <ID> --outline` — just the numbered section map, when you already know the declaration and only need the right section coordinate.
- `grund list` / `grund list --kind FS,AR` — every declared ID, or just the spec/architecture slice when you need to discover the right `<ID>`.
- `grund refs <ID> --summary` / `grund refs <ID>` — the file-level blast radius first, then every citation site when you need exact edits.

## How to use this file

1. Start with the project knowledge map below, then open the relevant declaration with `grund show <ID> --brief` (or `grund show <ID>.<section>` when the citation names a section) before making changes.
2. When you learn something new about *why*, *where*, *what*, *how*, or *how we got here* — write it down in the matching declared artifact. Don't hoard context.
3. When you make a non-obvious decision, add it to the configured decision or record location and cite the ID from the spec, code, or test it affects.
4. Behavior is proven by executable tests or cases, not by prose alone. Every functional change ships with a matching proof in the repo's configured test artifacts.

## Project knowledge map

`grund` scans: `docs`, `e2e`, `src`; excluded directories: `target`, `node_modules`, `.git`, `dist`, `build`, `.venv`.

Configured declaration homes:

| Kind | Home | Purpose |
|---|---|---|
| `G` | `docs/goals` | Goal |
| `FS` | `docs/functional-spec` | Functional spec |
| `AR` | `docs/architecture` | Architectural spec |
| `DF` | `docs/decisions/functional` | Functional decision |
| `DA` | `docs/decisions/architectural` | Architectural decision |
| `E2E` | `e2e/cases` | End-to-end test |
| `RM` | `docs` | Roadmap milestone |

These paths come from `.agents/grund.toml`. If this repo's specs, roadmaps, changelogs, decisions, plans, tests, or examples live somewhere else, update the config first and re-run `grund init` so this guidance stays aligned with the repository instead of introducing a parallel layout.

## References

The `grund` ID scheme in this repo: `<KIND>-<NNN>-<slug>[.<section>]`, where `KIND` ∈ `{G, FS, AR, DF, DA, E2E, RM}` — the kinds and the ID/marker syntax are configurable in `.agents/grund.toml` (run `grund config show` to see the effective settings). Citations are written prefixed by the marker `§` — type `$$` in a `grund`-aware editor and it becomes `§` automatically. Bare ID-shaped tokens are also recognized as citations for backward compatibility; set `[reference] strict = true` in `.agents/grund.toml` to require the `§` marker (run `grund fmt --marker` first to upgrade existing bare citations).

Declarations are heading lines: `# FS-042-user-login: A player can log in …` in a markdown file (again, `FS-042-user-login` is just the shape), or the same shape inside a code doc-comment (Javadoc, JSDoc, Rustdoc, Python docstring, Go `//` block, etc.). A declaration can also live directly in a class-level or module-level doc-comment, with a one-line stub in that kind's configured home whose H1 is `# <ID>: [<path>](<path>)` (a markdown link to the file with the inline declaration). The exhaustive list of supported doc-comment forms is in [`grund`'s own architectural spec](https://github.com/vjovanov/grund/tree/main/docs/architecture).

Code back-references the spec it realizes. When a function, class, or block implements a behavior, it carries a `§<ID>` citation — on its doc-comment for a whole behavior, or on an inline comment beside the line that enforces one clause (`§<ID>.2.1`) or honors one decision. Cite at the granularity you implement: the behavior on the doc-comment, the clause on the `if` that checks it, the decision ID on the literal it pinned. Each citation is one more edge `grund refs <ID>` reports, so a reviewer changing a spec sees exactly which code leans on it — closing the loop goals ← specs ← architecture ← code, alongside specs ← executable tests.

## Rules for agents

- **Citations climb toward the goals.** Specs cite goals. Architecture cites specs. Code cites the specs it implements. Executable tests or cases cite the behavior they verify.
- **Refresh cited specs before editing code.** Before editing code that carries a `§<ID>` or `§<ID>.<section>` citation, run `grund show <ID> --brief` for a bare ID or `grund show <ID>.<section>` for a section citation; escalate to `grund show <ID>` only when the slice is not enough.
- **Know what leans on a declaration before you change it.** Before editing, renaming, or moving any declaration, run `grund refs <ID> --summary` first, then `grund refs <ID>` when you need every citation site, so you update or relocate them in the same change and nothing dangles after.
- **No dangling decisions.** Every decision record is cited from the spec or architecture doc it shaped, at the point where the choice applies — so a reader lands on the *why* without searching. A decision may also cite back into a spec; what it may not be is uncited — `grund check` flags that as unused.
- **Decisions are append-only.** Never rewrite decision history. If a decision is reversed, add a new entry that supersedes the old one and link both ways.
- **Cross-link everything via IDs.** Use the ID. No markdown links between docs.
- **Executable tests are the source of truth for behavior.** When the spec and the executable proof disagree, one of them is wrong — fix both in the same change.
- **Run `grund check` before you commit.** A dangling reference is a stop-the-line bug; `grund check`'s output names the file and line for each one.

This managed agent guidance block and the accompanying `.agents/grund.toml` were generated by `grund init`. Re-run `grund init --force` to refresh them at the current `grund` version.
<!-- grund:init:agents:v2 end -->

# grund — agent instructions

## Agent instructions (grund-agents v3)

This project uses [`grund`](https://github.com/vjovanov/grund): every spec, goal, decision, and end-to-end test has a stable ID `<KIND>-<slug>[.<section>]` (`KIND ∈ {GND, GOAL, FS, AR, DF, DA, E2E, RM, DISC}`), cited with the marker `§` — e.g. `§FS-user-login.3.1` (the `FS-user-login` here is a shape illustration, not a real ID in this repo). Type `$$` in a grund-aware editor and it becomes `§`. Bare ID-shaped tokens are ignored — `[reference] strict = true` is set in `.agents/grund.toml`, so only `§`-prefixed citations are checked.

### Grounding from a citation

A `§<ID>` is a pointer to a fact, not a file path. Resolve it with `grund` and climb only as far as needed:

- `grund show <ID>` — the lead (heading-less, cut at the first child section). The cheap first read for a bare `§<ID>` citation.
- `grund show <ID> --toc` — the lead plus the nested section map. Use to choose which subsection to fetch next.
- `grund show <ID> --full` — the entire body. Escalate to this when narrower reads aren't enough.
- `grund show <ID> --brief` — heading + first paragraph only.
- `grund refs <ID>` — every site that cites the ID; add `--summary` for one line per file. Run before renaming or moving a declaration.
- `grund list` / `grund list --kind FS,AR` — discover IDs if you get lost

### Project map

`grund` scans: `docs`, `e2e`, `src`; excluded directories: `target`, `node_modules`, `.git`, `dist`, `build`, `.venv`, `repo`, `expected.repo`.

| Kind | Home | Purpose |
|---|---|---|
| `GND` | `docs` | Grund |
| `GOAL` | `docs/goals` | Goal |
| `FS` | `docs/functional-spec` | Functional spec |
| `AR` | `docs/architecture` | Architectural spec |
| `DF` | `docs/decisions/functional` | Functional decision |
| `DA` | `docs/decisions/architectural` | Architectural decision |
| `E2E` | `e2e/cases` | End-to-end test |
| `RM` | `docs` | Roadmap milestone |
| `DISC` | `docs/discussions` | Discussion |

### Declarations and citations

Declarations are heading lines `# FS-user-login: …` in markdown, or the same shape in a doc-comment (Javadoc, JSDoc, Rustdoc, Python docstring, Go `//`). An inline source declaration can be represented by a one-line stub in the configured kind home: `# <ID>: [<path>](<path>)`.

### Rules

- **Spec first.** For behavior or design changes, write or update the most-specific spec point before code.
- **Cite as you write.** Place `§<ID>` at the point a claim or behavior is made — on the doc-comment for a whole behavior, inline beside the clause it enforces.
- **Always cite the most-specific point.**
- **Citations climb to reasons (grund.md).** Goals cite reasons, specs cite goals; architecture cites specs; code and executable tests cite specs.

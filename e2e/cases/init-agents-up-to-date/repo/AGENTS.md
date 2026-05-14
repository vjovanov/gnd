# repo — agent instructions

## Grounding with grund (v1)

This project uses [`grund`](https://github.com/vjovanov/grund): every spec, goal, decision, and end-to-end test has a stable ID `<KIND>-<NNN>-<slug>[.<section>]` (`KIND ∈ {G, FS, AR, DF, DA, E2E, RM}`), cited with the marker `§` — e.g. `§FS-042-user-login.3.1` (the `FS-042-user-login` here is a shape illustration, not a real ID in this repo). Type `$$` in a grund-aware editor and it becomes `§`. Bare ID-shaped tokens are also recognized as citations for backward compatibility; set `[reference] strict = true` in `.agents/grund.toml` to require the `§` marker (run `grund fmt --marker` first to upgrade existing bare citations).

### Grounding from a citation

A `§<ID>` is a pointer to a fact, not a file path. Resolve it with `grund` and climb only as far as needed:

- `grund show <ID>` — the lead (heading-less, cut at the first child section). The cheap first read for a bare `§<ID>` citation.
- `grund show <ID> --toc` — the lead plus the nested section map. Use to choose which subsection to fetch next.
- `grund show <ID> --full` — the entire body. Escalate to this when narrower reads aren't enough.
- `grund show <ID> --brief` — heading + first paragraph only.
- `grund refs <ID>` — every site that cites the ID; add `--summary` for one line per file. Run before renaming or moving a declaration.
- `grund list` / `grund list --kind FS,AR` — discover IDs if you get lost

### Project map

| Kind | Home | Purpose |
|---|---|---|
| `G` | `docs/goals` | Goal |
| `FS` | `docs/functional-spec` | Functional spec |
| `AR` | `docs/architecture` | Architectural spec |
| `DF` | `docs/decisions/functional` | Functional decision |
| `DA` | `docs/decisions/architectural` | Architectural decision |
| `E2E` | `e2e/cases` | End-to-end test |
| `RM` | `docs` | Roadmap milestone |

### Declarations and citations

Declarations are heading lines `# FS-042-user-login: …` in markdown. In a code doc-comment (Rustdoc, Javadoc, JSDoc, Python docstring, Go `//`, …) drop the `#` — write `/// FS-042-user-login: …` directly. One doc-comment may declare multiple IDs (e.g. an `AR-` and an `FS-` on the same class) — each gets its own body. An inline source declaration is reachable from the configured kind home via a one-line stub: `# <ID>: [<path>](<path>)`.

### Rules

- **Spec first.** For behavior or design changes, write or update the most-specific spec point before code.
- **Cite as you write.** Place `§<ID>` at the point a claim or behavior is made — on the doc-comment for a whole behavior, inline beside the clause it enforces.
- **Always cite the most-specific point.**
- **Citations climb to reasons (grund.md).** Goals cite reasons, specs cite goals; architecture cites specs; code and executable tests cite specs.

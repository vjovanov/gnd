# gnd — agents.md

This file is the entry point for any agent (human or AI) working on **gnd**. Read it first, then read the docs it points to — in order — before making changes.

This project uses the [`gnd`](https://github.com/anthropics/gnd) reference scheme: every spec, goal, decision, and end-to-end test has a stable ID of the form `<KIND>-<NNN>-<slug>`, declared as a heading inside its home file. Citations are written prefixed by the marker `§`, e.g. `§FS-user-login.3.1`. Section paths can be arbitrary depth — `.3`, `.3.1`, `.3.1.2` are all valid as long as a heading at that depth exists in the declaration. Run `gnd check` to validate every citation; run `gnd list` to see every declared ID; run `gnd show <ID>` to print just the body of one declaration. See `gnd`'s own §FS-check, §FS-list, and §FS-show for the contract.

## Grounding yourself in the spec

A `§<ID>` — or `§<ID>.<section>` — is a pointer to a fact, not a file path. When a doc, a code comment, or a review note cites one, resolve it with `gnd` instead of opening the file and skimming:

- `gnd show <ID>` — the full declaration body (a spec file's contents, a goal's success criterion, a decision record).
- `gnd show <ID>.<section>` — just that subsection, so you pull one fact into context without loading the whole file. This is the cheap, precise move — prefer it.
- `gnd show <ID> --head` — the lead paragraph only, for a quick "what is this about" before deciding whether to read more.
- `gnd list` — every declared ID, when you need to discover the right `<ID>`. `gnd refs <ID>` — every place that cites it, so you know what leans on a declaration before you change it.

## How to use this file

1. Start at the top of `docs/` and read down. Each document answers one question.
2. When you learn something new about *why*, *where*, *what*, *how*, or *how we got here* — write it down in the matching doc. Don't hoard context.
3. When you make a non-obvious decision, add an entry under `docs/decisions/` (architectural vs. functional) with date, options considered, and chosen path.
4. Behavior is proven by end-to-end tests in `e2e/`, not by prose. Every functional change ships with an e2e test.

## The docs/ folder

| Document | Question it answers |
|---|---|
| `docs/raison-detre.md` | **Why does this exist?** The problem we are solving and who it is for. |
| `docs/goals/goals.md` | **What do we measure?** Concrete, observable goals declared inline in a single file so a human can read them top-to-bottom. |
| `docs/roadmap.md` | **What's next?** Forward-looking, IDed milestones (`RM-NNN`) with a soft direction paragraph. |
| `docs/changelog.md` | **What changed?** Latest release inline; older releases under `docs/changelog/`. |
| `docs/functional-spec/` | **How does the system behave to achieve the goals?** External behavior — the *what*. |
| `docs/architectural-spec/` | **How is the system built?** Components, boundaries, data flow — the *how*. |
| `docs/decisions/` | **How did we get to the state we are in?** Append-only decision records, split into `architectural/` and `functional/`. |
| `docs/discussions/` | **What ideas are still open?** Discussion notes and proposals, declared with the project-local `DISC-*` kind. |

## The e2e/ folder

End-to-end tests live in `e2e/`. They are not documentation — they are executable proof that the functional spec holds.

- Every behavior described in `docs/functional-spec/` has at least one e2e test, per `gnd`'s §G-no-dangling-refs.
- When the spec and the tests disagree, one of them is wrong — fix both in the same change.
- New features are not "done" until an e2e test covers them.

## References

The `gnd` ID scheme: `<KIND>-<slug>[.<section>]` in this repo, where `KIND` ∈ `{G, FS, AS, DA, DF, E2E, RM, DISC}` per `.agents/gnd.toml` (configurable per `gnd`'s §FS-config). Citations are written prefixed by the marker `§` (per `gnd`'s §DF-reference-marker). Bare tokens are ignored because `[reference] strict = true` is set in `.agents/gnd.toml`.

Declarations are heading lines: `# FS-user-login: A player can log in …` in a markdown file, or the same shape inside a code doc-comment (Javadoc, JSDoc, Rustdoc, Python docstring, Go `//` block, etc.). An architectural spec, in particular, can live directly in the class-level doc-comment of the class it describes, with a one-line stub under `docs/architectural-spec/` whose H1 is `# <ID>: [<path>](<path>)` (a markdown link to the file with the inline declaration). See `gnd`'s §AS-scanner.4 for the exhaustive list of supported doc-comment forms.

## Rules for agents

- **Citations climb toward the goals.** Specs cite goals. Architecture cites specs. E2E tests cite the FS they verify.
- **No dangling decisions.** Every decision record is cited from the spec or architecture doc it shaped, at the point where the choice applies — so a reader lands on the *why* without searching. A decision may also cite back into a spec; what it may not be is uncited — `gnd check` flags that as unused.
- **Decisions are append-only.** Never rewrite history under `docs/decisions/`. If a decision is reversed, add a new entry that supersedes the old one and link both ways.
- **Cross-link everything via IDs.** Use the ID. No markdown links between docs.
- **E2E tests are the source of truth for behavior.** When the spec and the e2e tests disagree, one of them is wrong — fix both, in the same change.
- **Run `gnd check` before you commit.** A dangling reference is a stop-the-line bug — see `gnd`'s §FS-check.3 for the rules enforced.

## Rules for change

These are project-specific (this repo, the `gnd` tool itself); they don't apply to downstream users of `gnd`:

- Each step lands as a separate, reviewable change with its own e2e tests.
- The scheme itself does not change without a corresponding decision under `docs/decisions/architectural/`.
- Self-host check stays green from day one: `cargo run -- .` against this repo must report zero errors after every change.

This `agents.md` and the accompanying `.agents/gnd.toml` were generated by `gnd init`. Re-run `gnd init --force` to refresh them at the current `gnd` version.

<!-- gnd:init:agents:v1 begin -->
# gnd — agents.md

This file is the entry point for any agent (human or AI) working on **gnd**. Read it first, then read the docs it points to — in order — before making changes.

This project uses the [`gnd`](https://github.com/anthropics/gnd) reference scheme: every spec, goal, decision, and end-to-end test has a stable ID of the form `<KIND>-<NNN>-<slug>`, declared as a heading inside its home file. Citations are written prefixed by the marker `§`, e.g. `§FS-user-login.3.1`. Section paths can be arbitrary depth — `.3`, `.3.1`, `.3.1.2` are all valid as long as a heading at that depth exists in the declaration. Run `gnd check` to validate every citation; run `gnd list` to see every declared ID; run `gnd show <ID>` to print just the body of one declaration. See `gnd`'s own §FS-check, §FS-list, and §FS-show for the contract.

## Grounding yourself in the spec

A `§<ID>` — or `§<ID>.<section>` — is a pointer to a fact, not a file path. When a doc, a code comment, or a review note cites one, resolve it with `gnd` instead of opening the file and skimming:

- `gnd show <ID>` — the full declaration body (a spec file's contents, a goal's success criterion, a decision record).
- `gnd show <ID>.<section>` — just that subsection, so you pull one fact into context without loading the whole file. This is the cheap, precise move — prefer it.
- `gnd show <ID> --head` — the lead paragraph only, for a quick "what is this about" before deciding whether to read more.
- `gnd list` — every declared ID, when you need to discover the right `<ID>`. `gnd refs <ID>` — every place that cites it, so you know what leans on a declaration before you change it.

## How to use this file

1. Start at the top of `docs/` and read down. Each document answers one question.
2. When you learn something new about *why*, *where*, *what*, *how*, or *how we got here* — write it down in the matching doc. Don't hoard context.
3. When you make a non-obvious decision, add an entry under `docs/decisions/` (architectural vs. functional) with date, options considered, and chosen path.
4. Behavior is proven by end-to-end tests in `e2e/`, not by prose. Every functional change ships with an e2e test.

## The docs/ folder

| Document | Question it answers |
|---|---|
| `docs/raison-detre.md` | **Why does this exist?** The problem we are solving and who it is for. |
| `docs/goals/goals.md` | **What do we measure?** Concrete, observable goals declared inline in a single file so a human can read them top-to-bottom. |
| `docs/roadmap.md` | **What's next?** Forward-looking, IDed milestones (`RM-NNN`) with a soft direction paragraph. |
| `docs/changelog.md` | **What changed?** Latest release inline; older releases under `docs/changelog/`. |
| `docs/functional-spec/` | **How does the system behave to achieve the goals?** External behavior — the *what*. |
| `docs/architectural-spec/` | **How is the system built?** Components, boundaries, data flow — the *how*. |
| `docs/decisions/` | **How did we get to the state we are in?** Append-only decision records, split into `architectural/` and `functional/`. |
| `docs/discussions/` | **What ideas are still open?** Discussion notes and proposals, declared with the project-local `DISC-*` kind. |

## The e2e/ folder

End-to-end tests live in `e2e/`. They are not documentation — they are executable proof that the functional spec holds.

- Every behavior described in `docs/functional-spec/` has at least one e2e test, per `gnd`'s §G-no-dangling-refs.
- When the spec and the tests disagree, one of them is wrong — fix both in the same change.
- New features are not "done" until an e2e test covers them.

## References

The `gnd` ID scheme: `<KIND>-<slug>[.<section>]` in this repo, where `KIND` ∈ `{G, FS, AS, DA, DF, E2E, RM, DISC}` per `.agents/gnd.toml` (configurable per `gnd`'s §FS-config). Citations are written prefixed by the marker `§` (per `gnd`'s §DF-reference-marker). Bare tokens are ignored because `[reference] strict = true` is set in `.agents/gnd.toml`.

Declarations are heading lines: `# FS-user-login: A player can log in …` in a markdown file, or the same shape inside a code doc-comment (Javadoc, JSDoc, Rustdoc, Python docstring, Go `//` block, etc.). An architectural spec, in particular, can live directly in the class-level doc-comment of the class it describes, with a one-line stub under `docs/architectural-spec/` whose H1 is `# <ID>: [<path>](<path>)` (a markdown link to the file with the inline declaration). See `gnd`'s §AS-scanner.4 for the exhaustive list of supported doc-comment forms.

## Rules for agents

- **Citations climb toward the goals.** Specs cite goals. Architecture cites specs. E2E tests cite the FS they verify.
- **No dangling decisions.** Every decision record is cited from the spec or architecture doc it shaped, at the point where the choice applies — so a reader lands on the *why* without searching. A decision may also cite back into a spec; what it may not be is uncited — `gnd check` flags that as unused.
- **Decisions are append-only.** Never rewrite history under `docs/decisions/`. If a decision is reversed, add a new entry that supersedes the old one and link both ways.
- **Cross-link everything via IDs.** Use the ID. No markdown links between docs.
- **E2E tests are the source of truth for behavior.** When the spec and the e2e tests disagree, one of them is wrong — fix both, in the same change.
- **Run `gnd check` before you commit.** A dangling reference is a stop-the-line bug — see `gnd`'s §FS-check.3 for the rules enforced.

This `agents.md` and the accompanying `.agents/gnd.toml` were generated by `gnd init`. Re-run `gnd init --force` to refresh them at the current `gnd` version.
<!-- gnd:init:agents:v1 end -->

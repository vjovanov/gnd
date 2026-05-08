# Agents

This file is the entry point for any agent (human or AI) working on **gnd**. Read it first. Then read the docs it points to — in order — before making changes.

`gnd` is the **agent grounding tool** — it keeps every agent (human or AI) grounded in the same shared facts, by validating an ID-based reference scheme across docs, tests, and source code. No agent has to hold the whole picture in their head, and no claim floats free of the declaration that supports it. `gnd` dogfoods this: the repo's own docs use `gnd`-style IDs, and `gnd` is run against itself in CI.

## How to use this file

1. Start at the top of `docs/` and read down. Each document answers one question.
2. When you learn something new about *why*, *where*, *what*, *how*, or *how we got here* — write it down in the matching doc. Don't hoard context.
3. When you make a non-obvious decision, add an entry under `docs/decisions/` (architectural vs. functional) with date, options considered, and chosen path.
4. Behavior is proven by end-to-end tests in [`e2e/`](e2e/), not by prose. Every functional change ships with an e2e test.

## The docs/ folder

| Document | Question it answers |
|---|---|
| [`docs/raison-detre.md`](docs/raison-detre.md) | **Why does this exist?** The problem we are solving and who it is for. |
| [`docs/state-and-direction.md`](docs/state-and-direction.md) | **Where are we now, and how do we get there?** Current state and the path forward. |
| [`docs/goals/goals.md`](docs/goals/goals.md) | **What do we measure?** Concrete, observable goals declared inline in a single file so a human can read them top-to-bottom. |
| [`docs/functional-spec/`](docs/functional-spec/) | **How does the system behave to achieve the goals?** External behavior — the *what*. |
| [`docs/architectural-spec/`](docs/architectural-spec/) | **How is the system built?** Components, boundaries, data flow — the *how*. |
| [`docs/decisions/`](docs/decisions/) | **How did we get to the state we are in?** Append-only decision records, split into `architectural/` and `functional/`. |

## The e2e/ folder

End-to-end tests live in their own top-level folder: [`e2e/`](e2e/). They are not documentation — they are executable proof that the functional spec holds.

- Every behavior described in `docs/functional-spec/` has at least one e2e test.
- When the spec and the tests disagree, one of them is wrong — fix both in the same change.
- New features are not "done" until an e2e test covers them.

## References

The `gnd` ID scheme: `<KIND>-<NNN>-<slug>[.<section>]`, where `KIND` ∈ `{G, FS, AS, DA, DF, E2E}` by default (configurable per FS-006-config). Citations are written prefixed by the marker `§` (per DF-001-reference-marker) — for example `§FS-001-check.3.1`. Section paths can be **arbitrary depth** — `.3`, `.3.1`, `.3.1.2`, `.3.1.2.7.4` are all valid as long as a heading at that depth exists in the declaration. Bare tokens are also recognized for backward compatibility unless `[reference] strict = true` is set in `gnd.toml`.

Declarations are heading lines: `# FS-042-user-login: A player can log in …` in a markdown file, or the same shape inside a code doc-comment (Javadoc, JSDoc, Rustdoc, Python docstring, Go `//` block, etc.). An architectural spec, in particular, can live directly in the class-level doc-comment of the class it describes, with a one-line stub under `docs/architectural-spec/` containing `Defined-in: <path>`. See AS-001-scanner.4 for the exhaustive list of supported doc-comment forms.

Self-hosting: `cargo run --release -- .` (run from this repo root) checks all of the IDs in this repo's `docs/`, `e2e/`, and `src/`. CI fails the build if any reference is dangling. The same check runs locally as a pre-commit hook (see README "Pre-commit hook"), so a broken citation never reaches a commit.

Bootstrapping a new repo: `gnd init` writes a canonical `agents.md` and `gnd.toml`; `gnd init --docs` additionally scaffolds the `docs/` and `e2e/` trees described above. See [`FS-008-init`](docs/functional-spec/FS-008-init.md).

## Rules for agents

- **One direction of truth.** Specs cite goals. Architecture cites specs. Decisions cite whichever they shaped. E2E tests cite the FS they verify.
- **Decisions are append-only.** Never rewrite history under `docs/decisions/`. If a decision is reversed, add a new entry that supersedes the old one and link both ways.
- **Cross-link everything via IDs.** Use the ID. No markdown links between docs.
- **E2E tests are the source of truth for behavior.** When the spec and the e2e tests disagree, one of them is wrong — fix both, in the same change.
- **Self-host.** Any change to the spec scheme must keep this repo's own docs valid. If you can't ground gnd with gnd, you broke gnd.
- **Run `gnd check` before you commit.** The pre-commit hook does it automatically; do not bypass it with `--no-verify`. A dangling reference is a stop-the-line bug.

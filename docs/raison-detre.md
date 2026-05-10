# Raison d'être

## 1. The problem

Software projects increasingly involve many agents — humans and AIs — collaborating on the same codebase over long horizons. Each agent has limited context. Without a shared, mechanically-checked reference frame, knowledge silently fragments: specs drift from code, decisions are forgotten, e2e tests prove the wrong things, and "the why" lives in someone's head until that someone moves on.

The **gnd reference scheme** addresses this by giving every spec, goal, decision, and test a stable ID, and forbidding agents from hoarding context outside the docs. But the scheme is only as strong as its enforcement: a dangling `FS-user-login.3.1` in prose is invisible until something breaks.

Citations live wherever they are useful — including inside Java doc-comments, Rust `///` lines, Python docstrings, Go doc blocks, TypeScript JSDoc — not only inside Markdown. Off-the-shelf Markdown link checkers (`lychee`, `markdown-link-check`) cannot help with those: they walk `.md` files, validate `[text](url)`, and return. A `FS-events.4` cited from `src/bus.rs` is invisible to them. That is the gap `gnd` exists to close.

## 2. What gnd does about it

`gnd` is the mechanical conscience of the scheme. It walks the repo — `.md` files **and** source files in every major language (§AS-scanner.4) — and does three things plain Markdown links cannot:

1. **Verify** every cited ID across both prose and code. A citation in a Javadoc, Rustdoc, or Python docstring is checked the same way as a citation in `docs/`. Dangling refs, broken section coordinates, duplicate declarations, and broken stub links are all errors.
2. **Survive refactors.** IDs are location-independent: `FS-user-login.3.1` keeps resolving when the file is renamed or moved. Markdown anchors (`#some-heading-slug`) break the moment a heading is reworded. Citations written once stay correct.
3. **Extract** a single declaration body for grounded reading. `gnd show FS-user-login.3.1` returns just that subsection — under 200 lines per §G-friendliness-first.1 — so a human or an LLM agent can pull a fact into context without loading entire files.

Markdown links cover navigation in rendered docs. The three above are the load-bearing ones, and they are what `gnd` is for.

This serves §G-no-dangling-refs, §G-fast-feedback, §G-friendliness-first, and §G-polyglot-citation.

## 3. Who it is for

- **Codebases that adopt the specification-driven design** — to verify the spec stays intact across changes, *and across the docs/code boundary*.
- **Polyglot projects** whose specs are cited from source as well as docs — the case off-the-shelf link checkers cannot serve.
- **Agents (human and AI) working in those codebases** — to retrieve grounded spec content cheaply.
- **CI systems** — as a fast pre-merge check.

If a project does not use a gnd-style ID scheme, `gnd` has nothing to offer it. We deliberately do not generalize.

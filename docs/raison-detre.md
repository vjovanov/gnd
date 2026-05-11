# Raison d'être

**Keep agents grounded in the spec — fewer bugs, cheaper LLM context, faster onboarding.** On a long-lived codebase with many humans and AIs, the spec drifts: citations rot, decisions get forgotten, e2e tests prove the wrong things, and "the why" lives in someone's head until that someone moves on. Each agent has limited context, and without a shared, mechanically-checked reference frame, knowledge silently fragments.

The **gnd reference scheme** — shipped and enforced by the `gnd` tool — addresses this by giving every spec, goal, decision, and test a stable ID, and forbidding agents from hoarding context outside the docs. But the scheme is only as strong as its enforcement: a dangling `§FS-<user-login>.3.1` in prose is invisible until something breaks.

Citations live wherever they are useful — including inside Java doc-comments, Rust `///` lines, Python docstrings, Go doc blocks, TypeScript JSDoc — not only inside Markdown. Off-the-shelf Markdown link checkers (`lychee`, `markdown-link-check`) cannot help with those: they walk `.md` files, validate `[text](url)`, and return. A `§FS-<events>.4` cited from `src/bus.rs` is invisible to them. That is the gap `gnd` exists to close.

## 1. What gnd does about it

`gnd` owns the scheme end to end. It defines the IDs and citation grammar, ships the config in `.agents/gnd.toml`, and scans every `.md` file and every source file in the repo ([§AS-scanner.4](architectural-spec/AS-scanner.md#4-inline-declarations-in-language-doc-comments)) to guarantee three things:

1. **No dangling reference ships.** Every cited ID is checked across prose and code alike — Javadoc, Rustdoc, Python docstrings, Go blocks, JSDoc. Dangling refs, broken section coordinates, duplicate declarations, and broken stub links all fail the build.
2. **Citations survive refactors.** IDs are location-independent: `§FS-<user-login>.3.1` keeps resolving when files move or headings reword. Markdown anchors break; gnd citations don't.
3. **Grounding is cheap.** `gnd show FS-<user-login>.3.1` returns just that subsection — under 200 lines per [§G-friendliness-first.1](goals/goals.md#1-hard-requirements) — so a human or LLM pulls one fact into context instead of a whole file.

This serves [§G-agent-grounding](goals/goals.md#g-agent-grounding-agents-stay-cited-as-they-work) — the headline goal that every other goal exists in service of — and the mechanisms that make it viable: [§G-no-dangling-refs](goals/goals.md#g-no-dangling-refs-every-cited-id-resolves-to-a-declaration), [§G-fast-feedback](goals/goals.md#g-fast-feedback-gnd-must-be-as-fast-as-possible), [§G-friendliness-first](goals/goals.md#g-friendliness-first-as-user--and-agent-friendly-as-possible), and [§G-polyglot-citation](goals/goals.md#g-polyglot-citation-ids-cite-cleanly-from-anywhere-they-are-useful).

## 2. Who it is for

- **Codebases that adopt the specification-driven design** — to verify the spec stays intact across changes, *and across the docs/code boundary*.
- **Polyglot projects** whose specs are cited from source as well as docs — the case off-the-shelf link checkers cannot serve.
- **Agents (human and AI) working in those codebases** — to retrieve grounded spec content cheaply.
- **CI systems** — as a fast pre-merge check.

If a project does not use a gnd-style ID scheme, `gnd` has nothing to offer it. We deliberately do not generalize.

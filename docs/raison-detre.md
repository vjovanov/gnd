# Raison d'être

## 1. The problem

Software projects increasingly involve many agents — humans and AIs — collaborating on the same codebase over long horizons. Each agent has limited context. Without a shared, mechanically-checked reference frame, knowledge silently fragments: specs drift from code, decisions are forgotten, e2e tests prove the wrong things, and "the why" lives in someone's head until that someone moves on.

The **gnd reference scheme** addresses this by giving every spec, goal, decision, and test a stable ID, and forbidding agents from hoarding context outside the docs. But the scheme is only as strong as its enforcement: a dangling `FS-042-user-login.3.1` in prose is invisible until something breaks.

## 2. What gnd does about it

`gnd` is the mechanical conscience of that scheme. It walks the repo, finds every declared and cited ID, and fails the build when:

- a citation points at a declaration that doesn't exist;
- a section coordinate (`.3.1`) points at a heading that doesn't exist;
- two files claim the same ID;
- an inline-spec stub points at code that doesn't declare the ID.

It also helps agents *use* the scheme: `gnd show <ID>` prints just the declaration body, so an agent can pull a single spec section into context without loading entire files.

This serves G-001-no-dangling-refs and G-002-fast-feedback.

## 3. Who it is for

- **Codebases that adopt the specification-driven design** — to verify the spec stays intact across changes.
- **Agents (human and AI) working in those codebases** — to retrieve grounded spec content cheaply.
- **CI systems** — as a fast pre-merge check.

If a project does not use a gnd-style ID scheme, `gnd` has nothing to offer it. We deliberately do not generalize.

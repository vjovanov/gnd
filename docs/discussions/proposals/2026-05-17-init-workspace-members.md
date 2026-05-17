# DISC-init-workspace-members: Have `init` mention workspace members

## Status

Discussion.

## Context

`grund init` is workspace-blind today. It writes a single `AGENTS.md` (plus
companion entrypoints) and a single `.agents/grund.toml` for the directory it is
invoked in, and the managed block's Project Map
([§FS-init.2.3.4.4](../../functional-spec/FS-init.md#2-3-4-4-project-map))
lists only the declaration homes from that one project's effective config.

That is fine for a single-project repo and matches §FS-init's commitment to "no
prompts, no surprises" rooted in
[§GOAL-friendliness-first](../../goals.md#goal-friendliness-first). It is
weaker for a workspace, though: at a root whose `.agents/grund.toml` declares
`[workspace] members = [...]`
([§FS-workspace.2](../../functional-spec/FS-workspace.md#2-workspace-configuration)),
nothing in the generated `AGENTS.md` tells a reading agent that sibling
project namespaces exist or where to climb to read their rules. An agent
arriving at the root therefore cannot tell — without scanning the toml itself —
that `§api/FS-foo` style cross-project citations are even possible, or which
aliases are in scope.

Making `init` *configure* the workspace is the heavier alternative discussed
upstream of this note and was set aside: it requires either prompts or
inference about which directories are members, both of which violate the
no-prompts rule. This proposal is the lighter version — *mention*, do not
*configure*.

## Proposed shape

When the effective `.agents/grund.toml` at the init target has a `[workspace]`
section, append a small "Workspace members" list to the managed block, derived
purely from the existing config:

```markdown
### Workspace members

- `api` → `apps/api/AGENTS.md`
- `core` → `packages/core/AGENTS.md`
- `ui`   → `packages/ui/AGENTS.md`
```

The alias on the left is the member's `project_name` (or the directory's
canonical alias per [§FS-workspace.3](../../functional-spec/FS-workspace.md#3-aliases));
the path on the right is the member root joined with `AGENTS.md`. Globs from
`members = ["packages/*"]` are expanded the same way `grund check` already
expands them ([§FS-workspace.2](../../functional-spec/FS-workspace.md#2-workspace-configuration)),
so the list reflects the actual filesystem at init time.

The list is emitted from existing configuration only — `init` still does not
prompt, does not infer workspace topology, and does not write to member
directories. Bootstrapping a member's own `AGENTS.md` remains a separate
`grund init` invocation inside that member.

## Boundaries

- Do not write or modify any file under a member directory. This stays a
  root-only edit, scoped to the same managed block §FS-init already maintains.
- Do not add a `[workspace]` block to a config that does not already have one.
  The section appears only when the config already declares members.
- Do not change the Project Map format
  ([§FS-init.2.3.4.4](../../functional-spec/FS-init.md#2-3-4-4-project-map)) —
  the members list is a sibling section, not an extension of the existing list,
  so the "declaration homes" semantics stay clean.
- Do not surface `include_root = false` as anything other than the absence of
  the root alias from the members list; the flag's effect on `check` scope is
  not something an agent reads inline.

## Open questions

- Should members whose `AGENTS.md` does not yet exist be listed anyway (more
  honest, surfaces the missing entrypoint) or filtered out (avoids pointing at
  broken paths)? Listing them matches how the Project Map already lists homes
  without checking they are populated.
- Should the section also restate the cross-project citation syntax
  (`§alias/<ID>`), or rely on the reader to follow the link into the member's
  own `AGENTS.md`? The token-cheap-grounding direction
  ([§DISC-token-cheap-grounding](2026-05-12-token-cheap-grounding.md#disc-token-cheap-grounding-token-cheap-grounding-surfaces))
  argues for the briefest possible inline content.
- For very large workspaces (dozens of members), should the list collapse to
  the alias-only form (`- api`, `- core`, …) with a single pointer to "see
  each member's `AGENTS.md`", or stay fully linked?
- Does the member-side `init` need any reciprocal change — e.g. a one-line
  "this project is a member of the workspace at `../..`" pointer — or is the
  one-way root → members link enough?

## Opinions

Overall, this is a strong, low-risk proposal. Adding a "Workspace members" list to the root `AGENTS.md` solves the discoverability problem for AI agents in a multi-project repository while strictly adhering to the "no prompts, no surprises" goal.

Regarding the open questions:

- **Should members without an `AGENTS.md` be listed?** Yes. Listing them accurately reflects the `grund.toml` state and correctly surfaces missing entrypoints, which might prompt the agent to initialize them.
- **Should the syntax be restated?** No. Adhere to token-cheap-grounding. Providing the alias-to-path mapping is sufficient; agents should already know or learn the citation syntax independently.
- **Should large workspaces collapse the list?** No. Keep it fully linked. The mapping of namespaces to paths is the primary value. Stripping paths defeats the purpose, and 50 lines of Markdown is cheap in modern context windows.
- **Does member-side `init` need a reciprocal change?** Yes. A one-line pointer to the workspace root (e.g., `Part of the workspace defined at ../../AGENTS.md`) is highly valuable for agents invoked directly inside a member project.

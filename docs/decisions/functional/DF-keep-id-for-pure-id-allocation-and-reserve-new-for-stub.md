# DF-keep-id-for-pure-id-allocation-and-reserve-new-for-stub: Keep `id` for pure ID allocation and reserve `new` for stub creation

**Status:** Accepted
**Date:** 2026-05-12

## 1. Context

After the project rename to `grund` ([§DA-rename-to-grund](../architectural/DA-rename-to-grund.md#da-rename-to-grund-rename-gnd-to-grund-before-first-publish)), we reconsidered whether the declaration allocator should be named `grund id` or `grund new`.

`grund id` already has a narrow contract: it emits one conflict-free declaration ID and changes nothing ([§FS-id](../../functional-spec/FS-id.md#fs-id-grund-proposes-ids-for-new-declarations)). That contract is important for shell composition, agent workflows, and editor integrations: callers can run `ID=$(grund id FS "Title")` knowing stdout is exactly the proposed ID and the tree is untouched ([§FS-id.2.1](../../functional-spec/FS-id.md#21---format-text-default), [§FS-id.7](../../functional-spec/FS-id.md#7-what-id-does-not-do)).

`grund new` is more inviting, especially now that the binary name reads as a real word. But in a CLI, `new` usually promises creation: a new file, a new stub, or a new initialized object. Reusing that name for a command that only prints text would make the friendlier spelling less precise, and would blur the mutation boundary that [§FS-id.7](../../functional-spec/FS-id.md#7-what-id-does-not-do) currently makes explicit.

## 2. Decision

Keep `grund id <KIND> "<title>"` as the pure ID-allocation command.

Do **not** rename it to `grund new`, and do **not** add `new` as a synonym while it only emits an ID. Reserve `grund new` for a future higher-level declaration-creation workflow, if one is added: that command should create a declaration file or stub, and may call the same allocator internally.

## 3. Rationale

- **The name matches the output.** `id` returns an ID. It does not promise a file, a stub, or a write.
- **The scripting surface stays sharp.** `grund id` is the primitive that humans, agents, and the optional LSP can all share ([§FS-id.8](../../functional-spec/FS-id.md#8-why-this-exists)). A higher-level `new` command can be added later without weakening that primitive.
- **`new` should mean mutation.** If a user types `grund new FS "Title"`, the natural expectation is that something new appears in the tree. Holding that name back keeps the CLI honest.
- **The rename to `grund` does not change the command contract.** [§DA-rename-to-grund](../architectural/DA-rename-to-grund.md#da-rename-to-grund-rename-gnd-to-grund-before-first-publish) says every subcommand and flag is otherwise unchanged; keeping `id` respects that scope.

## 4. Consequences

- [§FS-id](../../functional-spec/FS-id.md#fs-id-grund-proposes-ids-for-new-declarations) remains the specification for `grund id`.
- Help text, completions, init hints, examples, and e2e fixtures continue to spell the pure allocator as `id`.
- A future `grund new` needs its own functional spec or an extension to [§FS-id](../../functional-spec/FS-id.md#fs-id-grund-proposes-ids-for-new-declarations), because it would cross the non-mutating boundary in [§FS-id.7](../../functional-spec/FS-id.md#7-what-id-does-not-do).

## 5. Alternatives considered

| Option | Why rejected |
|---|---|
| Rename `grund id` to `grund new` before first release | Better surface language, but wrong behavior for the name: it would still only print an ID. The user-visible command would imply creation where none happens. |
| Add `grund new` as an alias for `grund id` | Avoids breaking `id`, but creates two spellings for the same primitive and teaches users that `new` may be non-mutating. That makes it harder to add a real creator later. |
| Replace `id` with a mutating `new` now | Too much behavior for the current need. File placement, overwrite behavior, stub body, editor integration, and dry-run semantics would all need specification and tests. |
| Rename to `alloc` or `mint` | More technically precise than `new`, but less obvious than `id` and not materially better for a one-token frequent command ([§GOAL-friendliness-first.1](../../goals.md#1-hard-requirements)). |

# FS-workspace: grund validates cross-project citations in a workspace

`grund` can treat a repository as a workspace of independent project namespaces.
Local citations stay unchanged, while cross-project citations name a stable alias
before the local ID. This keeps the zero-config single-project path intact
([§GOAL-zero-config](../goals.md#goal-zero-config-works-on-any-conformant-tree)) and gives larger repos an explicit resolver for sub-projects
without forcing project names into every ID ([§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable)). The alias
syntax is chosen in [§DF-subproject-namespaces](../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos).

## 1. Citation syntax

A normal citation still resolves inside the current project:

```text
§FS-login
§FS-login.2.1
```

A cross-project citation writes the target project alias, a slash, then the
target ID:

```text
§api/FS-login
§root/GOAL-compatibility
§payments/FS-refunds.3.2
```

The grammar is:

```text
§[alias/]ID[.section]
```

`alias` is a lowercase slug: it starts with a letter and then uses lowercase
letters, digits, or `-`. The slash is part of the citation namespace, not part of
the ID. The `ID[.section]` part uses the same ID and section grammar as local
citations in this first workspace implementation.

## 2. Workspace configuration

A workspace is declared in the root project's `.agents/grund.toml`:

```toml
project_name = "root"

[workspace]
members = ["apps/api", "packages/*"]
include_root = true
```

`members` is a list of paths or single-segment trailing globs, resolved relative
to the config root. `packages/*` means every direct child directory under
`packages/`; recursive `**` globs are not part of v1. `include_root` defaults to
`true`; when false, `grund check` at the workspace root checks only member
projects.

Each member is a separate project namespace. If a member has its own
`.agents/grund.toml`, that file configures the member. If it does not, the
canonical defaults apply with the member directory as the config root.

## 3. Aliases

The root alias is the root config's `project_name`, or `root` when omitted.
A member alias is the member config's `project_name`, or the member directory's
basename when omitted.

Aliases must be unique across the workspace and must match the lowercase slug
grammar in §1. A duplicate or invalid alias is a launch-time error, because a
qualified citation would otherwise have two possible targets.

## 4. Resolution

During `grund check`:

- `<§>ID` resolves only against declarations in the current project.
- `<§>alias/ID` resolves only against declarations in the named workspace project.
- `<§>alias/ID.section` additionally requires that the target declaration contain
  the cited section.
- an unknown alias is an error at the citation site;
- a known alias with no matching declaration is an error at the citation site.

Cross-project references are deliberately never resolved by path syntax such as
`../FS-login` or `packages/api/FS-login`; aliases are the stable handles.

## 5. Command scope

`grund check` run at a workspace root checks the root project and all configured
members, aggregates diagnostics, and prints the same `path:line: message` shape
as a normal check ([§FS-check.2.1](FS-check.md#21-report-format)). Paths are rendered relative to the workspace
root when `[output] relative_paths = true`.

`grund check <member>` (or `grund check` invoked from inside a member tree)
discovers the member's own config first and validates it as an independent
project. Qualified citations such as `§root/<ID>` or `§sibling/<ID>` cannot be
resolved without the workspace context; each such citation produces an
`unknown project alias <name>` error at the citation site
([§FS-check.3.8](FS-check.md#38-cross-project-citation-failure)). This matches [§DF-subproject-namespaces](../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos) §3.6 — silent skipping
would let a passing member check ship a cross-project reference that no longer
resolves at the workspace root, which violates [§GOAL-no-dangling-refs](../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration). Run
`grund check` at the workspace root to validate cross-project citations.

A relaxed standalone mode (downgrade `unknown project alias` from error to
warning on a member-only run) is deferred follow-up; see
[§DF-subproject-namespaces](../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos) §3.6.

## 6. Nested project boundary

Workspace members are namespace boundaries. When the root project is checked as
part of a workspace, the root scan does not scan member project directories or
their descendants, even if the root project's `[scan] include` names a path
inside a member. The member is scanned separately under its own config and alias.

This prevents a child project declaration from accidentally becoming a duplicate
or dependency of the root namespace.

A member config that itself carries a `[workspace]` block is a load-time
error in v1 — nested workspaces are not supported. The resolver semantics
they would require (alias priority across nesting levels, which
`current_alias` a checker run uses, parent-of-parent reachability) are not
pinned, and the safer default is to refuse the configuration loudly rather
than silently flatten it ([§AR-workspace.6.1](../architecture/AR-workspace.md#61-nested-workspaces-are-rejected)).

## 7. Neighboring repos

Neighboring repositories in the same organization should use the same external
syntax, for example `<§>payments/FS-refunds`, but external repository resolution
is not part of this first implementation. It requires an explicit cache or
lockfile so ordinary `grund check` remains offline, deterministic, and fast
([§GOAL-fast-feedback](../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible)). Until that cache layer exists, aliases are
workspace-local.

## 8. Other commands

The v1 implementation applies to `grund check`. Query commands such as
`grund show`, `grund refs`, `grund list`, shell completions, and formatter
cross-reference rewriting remain project-local until their qualified-ID behavior
is specified separately.

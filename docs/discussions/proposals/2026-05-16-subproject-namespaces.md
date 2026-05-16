# DISC-subproject-namespaces: Sub-project and cross-repo citation namespaces

## Status

Resolved by [§DF-subproject-namespaces](../../decisions/functional/DF-subproject-namespaces.md#df-subproject-namespaces-alias-namespace-model-for-sub-projects-and-external-repos) and specified for the first implementation
slice in [§FS-workspace](../../functional-spec/FS-workspace.md#fs-workspace-grund-validates-cross-project-citations-in-a-workspace). This note remains as the exported design conversation.

## Context

`grund` currently treats one discovered `.agents/grund.toml` as one project
namespace: local IDs resolve inside the scanned tree, config discovery walks
upward from the command path, and the checker validates one declaration graph.
Any sub-project design must preserve the local citation ergonomics of
[§GOAL-zero-config](../../goals.md#goal-zero-config-works-on-any-conformant-tree) while keeping configurable layouts possible under
[§GOAL-configurable](../../goals.md#goal-configurable-every-default-is-overridable). It must also avoid false negatives in cross-project
resolution, because [§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration) is the core correctness promise.

The design pressure from the discussion:

- Sub-projects should be able to reference their super-project rarely.
- Super-projects should be able to reference sub-projects.
- Neighboring repos in the same organization should also be referenceable.
- Most citations should remain local and short.
- Cross-project references should be explicit and stable under file or repo
  moves.
- `grund check` should stay fast and deterministic per [§GOAL-fast-feedback](../../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible).

## Claude prompt

Please review the design discussion below and give an independent opinion. Focus
on syntax, resolver semantics, workspace/external-repo boundaries, ambiguity,
and whether the proposal fits `grund`'s existing goals:

- [§GOAL-zero-config](../../goals.md#goal-zero-config-works-on-any-conformant-tree): canonical repos should work without ceremony.
- [§GOAL-configurable](../../goals.md#goal-configurable-every-default-is-overridable): divergent layouts and naming schemes should be explicit
  config, not hard-coded.
- [§GOAL-no-dangling-refs](../../goals.md#goal-no-dangling-refs-every-cited-id-resolves-to-a-declaration): a passing check must not miss broken references.
- [§GOAL-fast-feedback](../../goals.md#goal-fast-feedback-grund-must-be-as-fast-as-possible): normal checks should avoid network and remain cheap.

Specific questions:

- Is `<§>alias/ID.section` the right syntax, or is `§ALIAS-ID.section` better?
- Should external neighboring repos use the same namespace syntax as workspace
  sub-projects?
- Should root/super-project references reserve `root`, or should the root alias
  be configurable?
- Should cross-project dependency policy be part of v1, or deferred until the
  resolver works?
- What should `grund check <subproject>` do with references to `root` or sibling
  projects when run outside the workspace root?

## Transcript

### Initial question

User:

> How can we support sub-projects elegantly?

Assistant:

> Support sub-projects as **independent grund namespaces orchestrated by a
> workspace**, not by baking project names into every ID.
>
> Recommended model:
>
> ```toml
> # .agents/grund.toml at repo root
> [workspace]
> members = ["apps/*", "packages/*"]
> include_root = true
> ```
>
> Each member is its own grund project root. Local citations stay short:
>
> ```md
> <§>FS-login
> <§>AR-auth
> ```
>
> Cross-project citations are explicit only when needed:
>
> ```md
> §@api/FS-login
> §@web/AR-routing.2.1
> ```
>
> Why this shape works:
>
> - **Local ergonomics stay unchanged.** A sub-project can have `FS-login`
>   without forcing `api-FS-login` everywhere.
> - **Duplicate IDs are fine across projects.** `apps/web` and `packages/api`
>   can both declare `§FS-config` in their own namespace.
> - **Root checks become orchestration.** `grund check` at the workspace root
>   runs each member with its own effective config, aggregates diagnostics, and
>   preserves normal `path:line:` output.
> - **Nested config is a boundary.** A parent scan should not accidentally
>   absorb a child project's `docs/` and `src/` under the parent namespace.
> - **Ambiguity is explicit.** At workspace root, `grund show FS-login` can work
>   only if unique; otherwise error with candidates and accept
>   `grund show @api/FS-login` or `--project api`.
>
> I would avoid "one giant global namespace" and avoid adding project prefixes
> to the normal ID grammar. That would make every citation noisier and would
> fight the current design where IDs are stable local handles.
>
> A clean implementation path:
>
> 1. Add `FS-workspace` / `DA-subproject-namespaces` first, grounded in
>    `§GOAL-zero-config`, `§GOAL-configurable`, and `§GOAL-no-dangling-refs`.
> 2. Add a `Workspace` layer above `Config`; keep `Config`, `Findings`, and
>    `check()` project-local.
> 3. Make `check` workspace-aware first; defer `show`, `refs`, `list`, and
>    completions until the namespace rules are proven.
> 4. Add qualified reference parsing later as `§@project/ID[.section]`, leaving
>    existing `§ID[.section]` untouched.
>
> The key design rule: **a sub-project is not a folder filter; it is a separate
> citation universe with an optional workspace resolver above it.**

### Super-project and sub-project references

User:

> So sub-projects can reference super projects (rarely), and super-projects can
> reference sub-projects.

Assistant:

> Yes. That fits the workspace model well.
>
> I would make cross-project references **explicit and symmetric**:
>
> ```md
> # in root docs
> The API package implements session validation in §@api/FS-session-validation.
>
> # in apps/api docs or code
> This package follows the repo-wide compatibility goal §@root/GOAL-no-silent-breakage.
> ```
>
> Local references stay unqualified:
>
> ```md
> <§>FS-session-validation
> ```
>
> The resolver rule would be:
>
> - `<§>FS-x` resolves inside the current project only.
> - `§@root/GOAL-x` resolves in the named super-project.
> - `§@api/FS-x` resolves in the named sub-project.
> - `grund check` at workspace root validates all local and cross-project
>   citations.
> - `grund check apps/api` can still validate references to `@root` if it can
>   discover the workspace root.
>
> I would avoid relative citation syntax like `§../GOAL-x` or
> `§./packages/api/FS-x`. Stable aliases are better than paths because projects
> move.
>
> Config shape could be:
>
> ```toml
> project_name = "root"
>
> [workspace]
> members = ["apps/api", "apps/web", "packages/*"]
> ```
>
> And each sub-project can either inherit its alias from path or declare:
>
> ```toml
> project_name = "api"
> ```
>
> Then:
>
> ```md
> <§>api/FS-login
> <§>payments/FS-refunds
> ```
>
> The important invariant: **cross-project references are always qualified**.
> That prevents a child accidentally binding to a parent's `FS-login`, or the
> root accidentally binding to a child's `FS-login`.
>
> I would probably start with "all workspace projects may reference each other"
> and add optional dependency restrictions later only if needed, for example:
>
> ```toml
> [workspace.references]
> api = ["root", "db"]
> root = ["api", "web"]
> ```
>
> But that policy layer should come after the resolver. The core feature is just
> multiple citation namespaces plus explicit qualified references between them.

### Neighboring repos

User:

> What if they are accross the org (neighboring repos)?

Assistant:

> For neighboring repos, treat them as **external project registries**, not
> workspace members.
>
> Same citation idea, but alias points to a repo identity:
>
> ```md
> §@payments/FS-refunds
> §@identity/GOAL-session-security
> ```
>
> Config at the consuming repo:
>
> ```toml
> [projects.payments]
> source = "git@github.com:org/payments.git"
> ref = "main"
>
> [projects.identity]
> source = "https://github.com/org/identity.git"
> ref = "v2.4.1"
> ```
>
> Resolver behavior:
>
> - `<§>FS-x` resolves locally.
> - `§@api/FS-x` resolves to a workspace sibling/sub-project.
> - `§@payments/FS-x` resolves to a registered external repo.
> - `grund check` should validate external refs only against a **local
>   cache/lockfile**, not hit the network by default. That preserves
>   `§GOAL-fast-feedback` and keeps checks deterministic.
> - A separate command updates the cache, e.g. `grund sync` or
>   `grund project fetch`.
>
> I would add a lockfile:
>
> ```toml
> # .agents/grund.lock
> [[projects]]
> alias = "payments"
> source = "git@github.com:org/payments.git"
> ref = "main"
> resolved = "a1b2c3d..."
> ```
>
> Then CI can run in two modes:
>
> ```bash
> grund check          # uses lockfile/cache, no network
> grund sync --check   # verifies lockfile is current
> ```
>
> This gives you three scopes with one mental model:
>
> | Citation | Scope |
> |---|---|
> | `<§>FS-login` | current project |
> | `§@root/GOAL-x` | workspace parent |
> | `§@api/FS-x` | workspace sibling/sub-project |
> | `§@payments/FS-x` | external registered repo |
>
> The important rule stays the same: **anything outside the current project is
> explicitly qualified by stable alias, never by filesystem path or URL**. That
> keeps references readable and survives repo moves.

### Syntax refinement

User:

> Can we make the syntax nicer?

Assistant:

> Yes. I would drop the `@` and use a **namespace path before the ID**:
>
> ```md
> <§>FS-login               # local project
> <§>root/GOAL-compatibility  # super-project
> <§>api/FS-session           # sub-project
> <§>payments/FS-refunds      # neighboring repo
> ```
>
> That is cleaner than:
>
> ```md
> §@payments/FS-refunds
> ```
>
> Recommended grammar:
>
> ```text
> §[namespace/]ID[.section]
> ```
>
> Examples:
>
> ```md
> <§>api/FS-session.2.1
> <§>web/AR-routing
> <§>root/GND-grund
> <§>identity/DF-token-lifetime
> ```
>
> Why `/` is a good separator:
>
> - It is visually light and familiar.
> - It does not interfere with current `<§>FS-id.3.1` section syntax.
> - It keeps the ID itself unchanged.
> - It scales from sub-projects to neighboring repos with the same shape.
> - It is still explicit enough that cross-project references cannot happen
>   accidentally.
>
> I would constrain namespace aliases to lowercase slugs:
>
> ```toml
> project_name = "api"
>
> [workspace]
> members = ["apps/api", "apps/web"]
>
> [projects.payments]
> source = "git@github.com:org/payments.git"
> ref = "main"
> ```
>
> Then:
>
> ```md
> <§>api/FS-login
> <§>payments/FS-refunds
> ```
>
> For the root project, reserve `root` as the canonical alias. Local references
> remain unqualified, so most citations stay as nice as they are today.

### Dash-style syntax

User:

> Can it be with a - PM-FS?

Assistant:

> Yes, but I would only do it with a clear reserved grammar.
>
> For example:
>
> ```md
> §PM-FS-refunds
> §API-GOAL-compatibility
> §ROOT-GND-grund
> ```
>
> Meaning:
>
> ```text
> §<PROJECT>-<ID>
> ```
>
> So `§PM-FS-refunds` means project `PM`, ID `FS-refunds`.
>
> The tradeoff is ambiguity. Today `FS-refunds` is itself an ID. With
> `PM-FS-refunds`, grund must know that `PM` is a project alias, not a kind. That
> is manageable if we add rules:
>
> - Project aliases are declared in config: `alias = "PM"`.
> - Cross-project parsing tries project aliases first.
> - Project aliases may not collide with kind prefixes like `FS`, `AR`, `GOAL`.
> - Local citations stay unchanged: `<§>FS-refunds`.
> - Cross-project citations use the prefix: `§PM-FS-refunds`.
>
> I would prefer uppercase aliases if you want this style:
>
> ```toml
> [workspace.projects.PM]
> path = "packages/payments"
>
> [projects.ID]
> source = "git@github.com:org/identity.git"
> ref = "main"
> ```
>
> Then citations read compactly:
>
> ```md
> §PM-FS-refunds.2.1
> §ID-AR-token-validation
> §ROOT-GOAL-no-silent-breakage
> ```
>
> This is nicer than `<§>payments/FS-refunds`, but less visually explicit. I think
> it is acceptable if the alias registry is mandatory and collisions are rejected
> at config load.

### Syntax preference

User:

> What is nicer?

Assistant:

> I would choose:
>
> ```md
> <§>pm/FS-refunds
> <§>api/AR-routing
> <§>root/GOAL-compatibility
> ```
>
> over:
>
> ```md
> §PM-FS-refunds
> ```
>
> The slash form is nicer because it keeps the two concepts visually separate:
>
> ```text
> project / local-id
> ```
>
> That means the local ID remains exactly `FS-refunds`, and `pm` is clearly the
> namespace. It also avoids parser ambiguity with today's `KIND-slug` grammar.
>
> The compact version I would recommend is lowercase short aliases plus `/`:
>
> ```md
> <§>pm/FS-refunds.2.1
> <§>id/GOAL-session-security
> <§>web/E2E-login
> ```
>
> So the rule becomes:
>
> ```text
> §ID                 local project
> <§>alias/ID           another project
> <§>alias/ID.section   another project section
> ```
>
> `§PM-FS-refunds` is workable, but it reads like one long ID rather than
> "payments project, FS-refunds." For a grounding tool, that extra clarity is
> worth the single `/`.

## Current recommendation to review

Use:

```text
§ID
<§>alias/ID
<§>alias/ID.section
```

with lowercase stable aliases:

```md
<§>FS-login
<§>root/GOAL-no-silent-breakage
<§>api/FS-session.2.1
<§>pm/FS-refunds
```

`alias` may name:

- a workspace root, sub-project, or sibling project;
- an external neighboring repo registered in config and resolved via a local
  cache or lockfile.

The resolver should treat each project as a separate citation namespace. Local
unqualified references resolve only in the current project. Qualified references
resolve only through declared aliases. Normal `check` should not perform network
I/O; external repositories should be synced by an explicit command and checked
from local cached metadata.

## Open design questions

- Should `root` be reserved by default, or should the root project need to
  declare `project_name = "root"` explicitly?
- Should alias uniqueness be workspace-wide and external-project-wide in one
  table, or should local workspace aliases be allowed to shadow external aliases?
- Should `show`, `refs`, `list`, and completions accept qualified IDs in v1, or
  should only `check` support them first?
- Should diagnostics for broken external references include the external alias
  and resolved commit/ref?
- Should `grund sync` cache full source trees, exported declaration indexes, or
  both?

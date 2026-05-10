# DISC-external-ticket-resolvers: External ticket resolvers

## Status

Discussion.

## Context

Project discussions often need to reference issue trackers alongside local specs:
GitHub issues, Jira tickets, Linear issues, or internal trackers. Those references
are useful in the same prose where `gnd` IDs appear, but they are not local
declarations and should not behave like `FS-*` or `AS-*` specs.

The design tension is that `gnd check` is intentionally deterministic and
offline, while validating a ticket's existence usually requires network access
and authentication.

## Proposed shape

Add a separate resolver concept for external references. These references would
have configured syntax and URL expansion, but no local declaration body.

Illustrative config:

```toml
[[external_refs]]
prefix = "GH"
format = "{number}"
url = "https://github.com/acme/project/issues/{number}"

[[external_refs]]
prefix = "JIRA"
format = "{project}-{number}"
url = "https://jira.acme.com/browse/{project}-{number}"
```

Illustrative prose:

```text
GH-1234 tracks the rollout work.
JIRA-PLAT-812 records the migration blocker.
```

In the future, the marker-prefixed form might become available once the scanner
can distinguish local declaration kinds from external resolver kinds without
creating false "unknown reference" errors.

## Semantics

- Local IDs resolve to local declarations and support `gnd show`.
- External tickets resolve to configured URLs and do not support `gnd show`.
- Default `gnd check` validates syntax and resolver configuration only.
- Default `gnd check` does not perform network calls.
- A later opt-in command could validate ticket existence online, for example
  `gnd external check --online`, but that should be separate from the normal
  offline pass.

## Why not use normal `[[kinds]]`

Ticket schemes usually have different grammar from local spec IDs. A repo may use
slug-only local IDs such as `§FS-config`, while GitHub issues are numeric and Jira
issues combine project keys with numbers. Trying to force these into the current
repo-wide `[id]` grammar would either make local IDs worse or make tickets too
constrained.

External resolvers should therefore be a new table rather than an extension of
local declaration kinds.

## Open questions

- Should ticket references require the same marker as local citations, or should
  they use a separate marker to avoid confusing them with `gnd show`-able IDs?
- Should URL expansion support only simple placeholders, or a small named set of
  provider presets such as GitHub, Jira, and Linear?
- Should online validation live in `gnd` at all, or should `gnd` only emit URLs
  and leave validation to tracker-specific tooling?

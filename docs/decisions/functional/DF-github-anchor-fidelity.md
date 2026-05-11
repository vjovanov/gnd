# DF-github-anchor-fidelity: the github anchor profile reproduces github-slugger exactly

**Status:** Accepted
**Date:** 2026-05-11
**Corrects:** [§DF-md-link-anchor-strategy.2.3](DF-md-link-anchor-strategy.md#23-renderer-profiles) — the `github` profile's slug algorithm

## 1. Context

[§DF-md-link-anchor-strategy.2.3](DF-md-link-anchor-strategy.md#23-renderer-profiles) introduced the named `[fmt.md_links] anchor_format` profiles and described the default `github` one as *"lowercase, strip punctuation, replace whitespace runs with `-`, collapse consecutive `-`."* The "whitespace **runs**" / "collapse consecutive `-`" half is wrong about what GitHub does, and `anchor_slug_github` followed the prose.

GitHub's `github-slugger` does not collapse anything. It lowercases the heading's text, deletes every character that is not a letter, digit, `_`, or `-` — each deletion **in place**, so the characters on either side become adjacent — and then replaces **each** remaining space with one `-`. Consequences:

- `## A — B` → `#a--b` — the em dash is deleted; the two spaces that flanked it survive and each becomes `-`.
- `` ## 6. Watch mode (`--watch`) `` → `#6-watch-mode---watch` — `(`, the back-ticks, and `)` are deleted; the space before `(` becomes `-` and the literal `--` of `--watch` survives, so the join is `-` + `--`.
- `## 3.2 [id] — ID grammar` → `#32-id--id-grammar`.

gnd was emitting `#a-b`, `#6-watch-mode-watch`, `#32-id-id-grammar` — anchors that resolve nowhere on GitHub. For a feature whose entire purpose is "clickable in the rendered doc" ([§FS-fmt.6.1](../../functional-spec/FS-fmt.md#61-scope)), shipping a `#fragment` that GitHub does not render is exactly the silent breakage the project promises not to ship ([§G-no-silent-breakage](../../goals/goals.md)). It went unnoticed because `gnd check` validates the *citation inside the brackets*, never the `#fragment` after the path ([§FS-non-goals.1](../../functional-spec/FS-non-goals.md#1-markdown-link-validation)); only a renderer — or `lychee --include-fragments` — exercises the anchor.

## 2. Decision

The `github` profile reproduces `github-slugger`: lowercase; keep letters, digits, `_`, and `-`; delete everything else in place; turn each remaining space into one `-`; no run-collapsing, no trailing-`-` trimming. `gitlab` continues to alias it — "similar to GitHub with minor Unicode-handling differences" per [§DF-md-link-anchor-strategy.2.3](DF-md-link-anchor-strategy.md#23-renderer-profiles), and identical for the ASCII headings gnd's own specs use. `mkdocs` and `pandoc` are unaffected: Python-Markdown's TOC slugger genuinely collapses `[-\s]+`, so the `mkdocs` profile's collapse is correct as written, and `pandoc`'s profile is unchanged.

Fidelity is held by two checks: the curated-heading e2e fixture [§FS-fmt.6.8](../../functional-spec/FS-fmt.md#68-measurable) demands for each profile (the `github` case now includes headings whose punctuation closes up into runs), and — repo-locally — `lychee --include-fragments` over `docs/` in the pre-commit hook ([§FS-fmt.4](../../functional-spec/FS-fmt.md#4-why-this-exists)), which resolves every emitted `#fragment` against the heading it points at the way GitHub would.

## 3. Why this fits gnd's goals

- [§G-no-silent-breakage](../../goals/goals.md) — the point of the change: the emitted anchor now resolves where the feature says it does.
- [§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-gnd-installs-disagree) (two installs agree) — still byte-deterministic on `(tree, config)`; the change is which bytes, not whether they are fixed.
- [§FS-non-goals.1](../../functional-spec/FS-non-goals.md#1-markdown-link-validation) (no link validation) — `gnd check` still does not validate `#fragment`s. The fix is in *emission*, where gnd computes the URL and so owns its correctness; the pre-commit `lychee --include-fragments` step is a repo-local belt-and-braces, not a new `gnd check` rule.

## 4. Consequences

- `anchor_slug_github` in `src/lib.rs` is rewritten to the algorithm above; its doc-comment cites this record.
- [§DF-md-link-anchor-strategy.2.3](DF-md-link-anchor-strategy.md#23-renderer-profiles)'s `github`/`gitlab` bullet is annotated to point here; its "collapse consecutive `-`" wording is corrected by this record rather than rewritten in place (decisions are append-only).
- [§FS-fmt.6.2](../../functional-spec/FS-fmt.md#62-form)'s anchor bullet states the no-collapse rule and cites this record; [§FS-fmt.6.8](../../functional-spec/FS-fmt.md#68-measurable)'s curated-heading note calls out the punctuation-closes-up cases.
- New e2e case `fmt-md-links-profile-github` covers the curated `github` heading set.
- This repo's pre-commit hook gains `gnd fmt --md-links --write` and switches `lychee` to `--include-fragments`, so anchor drift in `docs/` is caught at commit time.

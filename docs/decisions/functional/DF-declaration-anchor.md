# DF-declaration-anchor: a bare-ID Markdown link points at the declaration's heading anchor

**Status:** Accepted
**Date:** 2026-05-11
**Amends:** [§FS-fmt.6.2](../../functional-spec/FS-fmt.md#62-form) — the "no section → no anchor" rule; extends [§DF-md-link-anchor-strategy.2.1](DF-md-link-anchor-strategy.md#21-strategy)

## 1. Context

[§DF-md-link-anchor-strategy](DF-md-link-anchor-strategy.md#df-md-link-anchor-strategy-heading-text-slugs-re-derived-on-every-fmt-pass) gave `grund fmt --cross-refs` heading-text slug anchors, re-derived every pass. But [§FS-fmt.6.2](../../functional-spec/FS-fmt.md#62-form) wired them in only for citations that carry a `.<section>` suffix — *"When the citation has no section, the `#…` part is omitted"* — so a bare-ID citation like `§GOAL-agent-grounding` wraps to `[§GOAL-agent-grounding](goals.md)`, a link to the *file*, not to the declaration. In a single-file home like `docs/goals/goals.md` (every `§GOAL-…` declaration is a heading there, §G is declared inline) or `docs/roadmap.md` (the `§RM-…` declarations), the reader who clicks lands at the top of the file and has to scroll to find the declaration they asked for. The link is technically correct and useless — the same "too thin to ship" failure mode [§DF-md-link-anchor-strategy.5](DF-md-link-anchor-strategy.md#5-alternatives-considered) rejected the `none` profile as the default for.

A declaration in Markdown *is* a heading (`# G-agent-grounding: agents stay cited as they work`), and a heading has a renderer anchor (`#g-agent-grounding-agents-stay-cited-as-they-work`). The machinery to compute that anchor — heading-text slug per the configured renderer profile — already exists for sections; it just was never pointed at the declaration heading.

## 2. Decision

When `grund fmt --cross-refs` wraps a bare-ID citation (no `.<section>`) and the declaration's home is a Markdown file, the link gets a `#<anchor>` derived from that declaration's heading text, using the same renderer-profile slugger and the same rendered-text reduction (`[text](url)` → `text`) sections already use ([§DF-github-anchor-fidelity](DF-github-anchor-fidelity.md#df-github-anchor-fidelity-the-github-anchor-profile-reproduces-github-slugger-exactly)). The heading text is `<ID>` rendered per `[id] format`, then `: <title>` if the heading carries one.

This does **not** apply when:

- the home is a source file — a stub `# AS-x: [the scanner](../../src/lib.rs)` points at `src/lib.rs`, and a renderer will not jump inside a doc-comment, so the link stays a bare file link (the rule [§DF-md-link-emission.2.3](DF-md-link-emission.md#23-source-file-links) already states for source-file declarations);
- the active `anchor_format` is `none` — that profile is "no fragment, ever" by definition ([§DF-md-link-anchor-strategy.2.3](DF-md-link-anchor-strategy.md#23-renderer-profiles)).

Citations remain the source of truth; this is still a presentation layer ([§DF-md-link-emission](DF-md-link-emission.md#df-md-link-emission-grund-fmt-may-emit-clickable-markdown-links-alongside--prefixed-citations)). It does not inject anchors into headings (the invasive option [§DF-md-link-anchor-strategy.5](DF-md-link-anchor-strategy.md#5-alternatives-considered) rejected) — it derives the anchor a renderer already produces for the heading the author wrote.

## 3. Why this fits grund's goals

- [§GND-grund.1](../../grund.md#1-what-grund-does-about-it) — Markdown links are the "cover navigation in rendered docs" layer; a link that drops you at the file top barely covers it. This makes the bare-ID link land where the section-ID link already lands.
- [§GOAL-no-silent-breakage](../../goals/goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) — same conditional-on-`fmt` guarantee as every other `--cross-refs` rewrite: a declaration heading rename produces a one-line `fmt` diff on the next pass ([§DF-md-link-anchor-strategy.2.2](DF-md-link-anchor-strategy.md#22-re-derive-on-every-pass-supersede-fs-fmt63)), not a silently-dead anchor.
- [§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree) (two installs agree) — still byte-deterministic on `(tree, config)`.
- [§FS-non-goals.1](../../functional-spec/FS-non-goals.md#1-markdown-link-validation) (no link validation) — `grund check` still does not validate `#fragment`s; the anchor is correct-by-construction at emission, and `lychee --include-fragments` in this repo's pre-commit hook ([§FS-fmt.4](../../functional-spec/FS-fmt.md#4-why-this-exists)) is the repo-local cross-check.
- [§GOAL-zero-config](../../goals/goals.md#goal-zero-config-works-on-any-conformant-tree) — opt-in via `--cross-refs` / `[fmt.cross_refs] enabled`; the `github` default fits the common host, so it works out of the box for the majority ([§DF-md-link-emission.2.4](DF-md-link-emission.md#24-opt-in-never-default)).

## 4. Consequences

- `markdown_link_target` in `src/lib.rs` derives a declaration-heading anchor for a sectionless citation to a Markdown home; new helper `declaration_heading_text`.
- [§FS-fmt.6.2](../../functional-spec/FS-fmt.md#62-form)'s anchor bullet is rewritten: the `#<anchor>` is present whenever the home is Markdown and the profile is not `none` — the section heading for a `.<section>` citation, the declaration heading for a bare ID. The "no section → no anchor" sentence is replaced.
- [§FS-fmt.6.8](../../functional-spec/FS-fmt.md#68-measurable)'s curated e2e set gains a bare-ID citation whose link carries the declaration-heading anchor.
- Running `grund fmt --cross-refs --write` over this repo rewrites every bare-ID `.md` citation from `(<file>)` to `(<file>#<decl-anchor>)`.

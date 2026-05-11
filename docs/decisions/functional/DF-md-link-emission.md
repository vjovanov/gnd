# DF-md-link-emission: gnd fmt may emit clickable Markdown links alongside §-prefixed citations

**Status:** Accepted
**Date:** 2026-05-09

## 1. Context

The gnd reference scheme cites IDs in the form `§<KIND>-<slug>.<section>`. In rendered Markdown (GitHub, MkDocs, an IDE preview), these citations are not clickable — they are plain text. A reader scanning a doc has to mentally resolve the citation to a path, navigate there, and scroll to the section. `gnd show <ID>` answers the question on the command line, but a reader already inside the rendered doc wants the link there too.

Off-the-shelf Markdown links solve the click problem but lose every other property an ID has — they are path-coupled (renames break them), anchor-coupled (heading rewrites break them), and they do not work in source comments (`§G-polyglot-citation`). The whole point of `gnd` is that none of those losses are necessary.

We want to keep IDs as the source of truth and *also* deliver clickable links in rendered Markdown — without the losses, and without the user maintaining two forms by hand.

## 2. Decision

Extend `gnd fmt` (per [§FS-fmt.6](../../functional-spec/FS-fmt.md#6-markdown-link-emission-with---md-links)) with an opt-in `--md-links` mode that, in `.md` files only, wraps each marker-prefixed citation in a Markdown link to the declaration body. The unwrapped citation remains the canonical, source-of-truth form; the wrap is a derived presentation layer that `fmt` regenerates idempotently.

### 2.1 Form

Wrap-the-citation. Given an illustrative citation, the rewrite is:

```text
before:  §FS-foo.3.1
after:   [§FS-foo.3.1](../functional-spec/FS-foo.md#3-1)
```

The citation text inside the brackets is preserved verbatim — a reader sees the same characters as before, only now they form a link.

Why wrap, and not the alternatives we considered:

| Form | Why rejected |
|---|---|
| Parenthetical link — citation text plus a sidecar arrow link | Two artifacts per citation; visually noisy at scale; the arrow becomes meaningless to readers who do not learn the convention. |
| Reference-style — `[citation][label]` with footnote definitions collected at file bottom | Splits the citation across the file; harder to grep; reference-definition collection at file bottom drifts as citations move. |
| Auto-link only without text — the bare URL form `<path#anchor>` | Loses the human-readable citation form; readers can no longer skim a paragraph and identify the citation by its ID. |

Wrap-the-citation keeps one artifact per citation, keeps the cite human-readable, and produces clean diffs (only the brackets and parenthetical change).

### 2.2 Anchor format

**Superseded by [§DF-md-link-anchor-strategy](DF-md-link-anchor-strategy.md#df-md-link-anchor-strategy-heading-text-slugs-re-derived-on-every-fmt-pass).** This DR's first draft proposed a section-coordinate anchor (`.3.1` → `#3-1`) on a "stability under heading edits" argument. On review the proposed format proved factually wrong about renderer behavior — GitHub's slugger strips punctuation rather than converting it, so `### 3.1 Inputs` produces `#31-inputs`, not `#3-1`. [§DF-md-link-anchor-strategy](DF-md-link-anchor-strategy.md#df-md-link-anchor-strategy-heading-text-slugs-re-derived-on-every-fmt-pass) picks the actual strategy: heading-text slugs per a configurable renderer profile, re-derived on every `gnd fmt` pass. The "stability" framing is retracted there in favor of a normalize-on-each-run rule that matches the existing trigger→marker pass.

### 2.3 Source-file links

When the citation's declaration lives in source code (a stub of the form `# <ID>: [<src/path>](<src/path>)` points the ID at a source file), the wrapped link targets the source file with no anchor — e.g., `[§<ID>](../../src/path.rs)`. The host renderer will not jump inside a Rustdoc, but the reader lands on the right file. This is the best available answer until renderers learn doc-comment fragments — the alternative (skip emission entirely) would punish the very polyglot case the project is built around.

### 2.4 Opt-in, never default

`--md-links` is opt-in per invocation; `[fmt.md_links] enabled = true` opts a repo in globally. Three reasons (per [§FS-fmt.6.6](../../functional-spec/FS-fmt.md#66-why---md-links-is-opt-in)): paths in links are coupled to the file's location and rebase noisily under heavy refactor; alternative renderers (Pandoc, etc.) need different anchor formats; and treating wrapped form as canonical would imply the rendered Markdown view is the source of truth, which it is not.

## 3. Reconciliation with non-goals

This decision sits close to two [§FS-non-goals](../../functional-spec/FS-non-goals.md#fs-non-goals-what-gnd-will-deliberately-not-do) entries; both stand and this is consistent with them.

### 3.1 [§FS-non-goals.1](../../functional-spec/FS-non-goals.md#1-markdown-link-validation) — gnd does not validate Markdown links

`fmt --md-links` *emits* a link; it does not *validate* a link. Once the link is on disk it is a normal Markdown link and `lychee` is the right tool to validate it. `gnd check` continues to validate only the underlying citation — the part inside the brackets — same as before. If a contributor hand-edits the URL inside `(...)` to point somewhere wrong, `gnd` will not catch it; that is `lychee`'s job, exactly as [§FS-non-goals.1](../../functional-spec/FS-non-goals.md#1-markdown-link-validation) says.

### 3.2 [§FS-non-goals.5](../../functional-spec/FS-non-goals.md#5-documentation-generation) — gnd does not generate rendered documentation

The link emission is a transformation on the source `.md` file (a sibling of the existing trigger→marker rewrite in [§FS-fmt.2.1](../../functional-spec/FS-fmt.md#21-trigger-to-marker)), not a render to a different format. The output is still `.md`; the file tree is unchanged in shape; no HTML, PDF, or static site is produced. A reader who never opens a renderer sees the same citation as before, now wrapped in link syntax. [§FS-non-goals.5](../../functional-spec/FS-non-goals.md#5-documentation-generation) prevents `gnd` from owning the publishing pipeline; this decision keeps `gnd` upstream of any such pipeline.

## 4. Consequences

- A new `--md-links` flag on `gnd fmt` and a `[fmt.md_links]` config block in `gnd.toml` ([§FS-fmt.6.7](../../functional-spec/FS-fmt.md#67-configurability)).
- A new roadmap item [§RM-md-link-emission](../../roadmap.md#rm-md-link-emission-gnd-fmt---md-links) carries the implementation.
- The [§G-polyglot-citation](../../goals/goals.md#g-polyglot-citation-ids-cite-cleanly-from-anywhere-they-are-useful) goal explicitly states that the polyglot grammar is the canonical form; this decision is the sanctioned exception that adds a presentation-layer view in `.md` only.
- Repos that adopt `--md-links` should run `gnd fmt --md-links --write` as a pre-commit hook so the generated links stay in sync with file moves and citation edits. CI should run `gnd fmt --md-links --check` to flag drift.
- The [§G-no-silent-breakage](../../goals/goals.md#g-no-silent-breakage-changes-ship-through-a-deprecation-path) path: shipping the flag does not change any existing behavior; `--md-links` and `[fmt.md_links] enabled = true` are both off by default, so a repo that does not opt in sees no diff.

## 5. Alternatives considered

| Approach | Why rejected |
|---|---|
| Markdown links as the source of truth (delete the ID grammar) | Loses the polyglot property entirely — the reason `gnd` exists per [§G-polyglot-citation](../../goals/goals.md#g-polyglot-citation-ids-cite-cleanly-from-anywhere-they-are-useful) and the raison-detre. Source comments cannot host clickable Markdown. |
| Render IDs to links at publish time, in a downstream tool | Pushes the work to every consumer — MkDocs plugin, GitHub Action, IDE preview — and gives each one a chance to disagree on the link target. Doing it once in `gnd fmt` keeps two installs in agreement ([§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-gnd-installs-disagree)). |
| Always emit links (no opt-in) | Surprises existing repos with a large mechanical diff on first upgrade; couples every `.md` to its current path layout; violates [§G-no-silent-breakage](../../goals/goals.md#g-no-silent-breakage-changes-ship-through-a-deprecation-path). |
| Heading-text slugs for anchors | Brittle under heading edits; would force `fmt` to rewrite anchors whenever prose changes; runs counter to "IDs survive refactors" — the property this project exists to defend. |

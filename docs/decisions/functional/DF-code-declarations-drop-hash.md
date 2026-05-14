# DF-code-declarations-drop-hash: code-resident declarations may drop the `#` prefix

**Status:** Accepted
**Date:** 2026-05-14

## 1. Context

A declaration is a line whose first token (after an optional comment marker) is `#`, followed by an ID and a colon: `# AR-router: In-process router` in markdown, `/// # AR-router: In-process router` in a Rust doc-comment, ` * # AR-router: …` in a JSDoc block, and so on. The `#` does double duty: in markdown it makes the line a real H1 (which gives the file a TOC, anchor links, and natural readability in any viewer); inside a code doc-comment it does *nothing useful* — the surrounding doc-comment marker (`///`, `*`, `// `, the enclosing `"""`) already signals "this is documentation prose", and the class/function the doc-comment is attached to already provides the structural anchor. The `#` was carried over from markdown for grammar uniformity, not because anyone in a code editor benefits from it.

The cost is visible:

- The form `/// # AR-router: …` looks like the author is starting an H1 *inside* a Rustdoc comment, which is what rustdoc renders. The struct's own name is already the H1 in the rendered docs; the second one is redundant.
- Multi-declaration doc-comments — co-locating an `AR-` and an `FS-` in the same comment block, which the scanner already supports per [§FS-show.2.3.1](../../functional-spec/FS-show.md#231-what-counts-as-the-comment-block) — read as two stacked H1s, even though they're labels on prose blocks, not section starts.
- Discovery cost: agents reading the spec see `# AR-router:` and assume the `#` is part of the convention rather than a markdown-rendering accident, and propagate the noise into new declarations.

Markdown specs *do* benefit from the `#`: a `.md` file with `# FS-foo: title` opens as a structured document in any renderer, gets anchor links from the heading slug ([§DF-declaration-anchor](DF-declaration-anchor.md#df-declaration-anchor-a-bare-id-markdown-link-points-at-the-declarations-heading-anchor)), and reads naturally for a human browsing the repo on GitHub. Dropping `#` in markdown would break all of that.

So the asymmetry is real: `#` is load-bearing in `.md`, ceremonial in code. The decision is to let code-resident declarations drop it.

## 2. Decision

A code-resident declaration may be written *either* as `<comment-marker> # <ID>: <title>` (the historical form) or as `<comment-marker> <ID>: <title>` (the new, preferred form). The scanner accepts both; the [§DF-show-default-token-cheap](DF-show-default-token-cheap.md#df-show-default-token-cheap-grund-show-defaults-to-the-cheap-read-the-full-body-is-opt-in) `--brief` / default / `--toc` / `--full` ladder treats them identically. Markdown declarations are unchanged: they still require `# <ID>:` because a `.md` file needs its H1.

| File kind | Declaration form |
|---|---|
| `.md` | `# <ID>: <title>` — unchanged; the `#` is the markdown H1 |
| Rust doc-comment (`///`, `//!`) | `/// <ID>: <title>` — preferred; `/// # <ID>: <title>` still accepted |
| JSDoc / KDoc / Doxygen / Javadoc / Scaladoc / PHPDoc (`/** */`) | ` * <ID>: <title>` — preferred; ` * # <ID>: <title>` still accepted |
| Go (`// `) | `// <ID>: <title>` — preferred |
| C# XML doc (`///`) | `/// <ID>: <title>` — preferred |
| Python docstring (`""" """`) | `<ID>: <title>` (bare, inside the docstring) — preferred |
| Ruby (`# `) | `# <ID>: <title>` — the `#` is the comment marker, not a heading marker |

### 2.1 Multi-declaration doc-comments

The scanner already terminates a declaration's block at the next declaration line in either direction ([§FS-show.2.3.1](../../functional-spec/FS-show.md#231-what-counts-as-the-comment-block)). So two declarations may share a single doc-comment, each with its own body:

```rust
/// AR-router: In-process event router
///
/// Implements the publish-subscribe contract from §FS-events.
///
/// FS-router-priority: Routes are matched in declared priority order
///
/// Ties broken by registration order; see §DF-router-tiebreak.
pub struct Router { ... }
```

`grund show AR-router` and `grund show FS-router-priority` resolve independently — each gets its own body. This was already supported but had no example in the spec or fixtures; the new form makes it visually obvious (two label-and-prose pairs, rather than two stacked H1s).

### 2.2 Backward compatibility

The historical `<comment-marker> # <ID>: <title>` form keeps parsing. No repo on the old form needs to migrate to read its specs; `grund show`, `grund check`, `grund refs`, and `grund list` continue to work. The migration is purely cosmetic — a planned `grund fmt --strip-decl-hash` *(follow-up)* will mechanize the rewrite for repos that want to adopt the new form in bulk; until then, the conversion is a one-line sed against `<comment-marker> # <ID>` patterns.

### 2.3 `grund show` preserves the comment marker in `text` *(follow-up)*

Today `grund show`'s `text` format strips the declaration heading line entirely. A planned refinement: in `text` format, keep the heading line with its comment marker (`/// AR-router: …`, ` * AR-router: …`) as a provenance hint, so an agent can tell at a glance that the body came from a code doc-comment. The `md` format would synthesize a proper `# <ID>: <title>` line so the slice remains valid markdown.

This is **not** landing with the initial change because it would update every existing `show-inline-*` and `show-python-*` e2e fixture; the new declaration form works without it. Tracked as a follow-up to this decision.

### 2.4 What this is *not*

- **Not a change to markdown declarations.** `.md` files keep `#` because the markdown reader needs it.
- **Not a change to section headings inside a doc-comment.** A code-resident declaration that has its own sections still uses `## 1. …`, `### 1.1 …`, etc. — those are real markdown headings inside the rendered doc-comment, and they continue to anchor [§FS-show](../../functional-spec/FS-show.md#fs-show-grund-reads-a-single-declaration-body-by-id)'s section selection. Only the *declaration* heading drops the `#`.
- **Not a forced migration.** Old form keeps working; `grund check` does not warn on the `#`.

## 3. Why this fits grund's goals

- [§GOAL-friendliness-first](../../goals/goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) — the new form reads as what it is (a label on a prose block) rather than as a misplaced markdown heading. The `#` was confusing exactly the audience the goal is for.
- [§GND-grund](../../grund.md#gnd-grund-agents-stay-grounded-in-the-spec) — co-locating multiple declarations in one doc-comment (an `AR-` and an `FS-` on the same class) was already legal; the new form makes it readable, which makes it likelier to be used, which improves grounding.
- [§GOAL-no-silent-breakage](../../goals/goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) — both forms parse. No repo breaks. Migration is opt-in.

## 4. Consequences

Landing now:

- [§AR-scanner.2.1](../../architecture/AR-scanner.md#21-declaration-detection) describes both regex branches and how heading-level is inferred when the declaration line has no `#+` to count (it defaults to depth `1`).
- [§FS-show.2.3](../../functional-spec/FS-show.md#23-inline-declarations-in-code-and-doc-comments) documents the multi-declaration-per-comment shape with a Rust example, surfacing a capability the scanner already had.
- New e2e fixtures cover (a) a single declaration in code in the new form (`show-inline-rust-no-hash`), (b) two declarations in one doc-comment (`show-inline-rust-multi-decl`).
- The project's own [crates/grund-core/src/checker.rs](../../../crates/grund-core/src/checker.rs) declaration is migrated to the new form as the first dogfood.

Follow-ups (each tracked above as *(follow-up)*):

- §2.3: `grund show` text-format marker preservation on the heading line.
- §2: `grund fmt --strip-decl-hash` migration helper — once landed, [§FS-fmt](../../functional-spec/FS-fmt.md#fs-fmt-grund-normalizes-references-in-bulk) gains the mode.
- A worked example of code-form declarations in the [§FS-config.3.2](../../functional-spec/FS-config.md#32-id--id-grammar) prose, if the spec needs to surface the grammar branch separately from [§AR-scanner](../../architecture/AR-scanner.md#ar-scanner-how-grund-discovers-declarations-and-citations).

## 5. Alternatives considered

| Approach | Why rejected |
|---|---|
| Drop `#` in markdown too (one uniform "no-`#`" rule across all source kinds) | Breaks `.md` files' natural heading structure, anchor links, and "open it in any renderer" property. Reverses [§DF-declaration-anchor](DF-declaration-anchor.md#df-declaration-anchor-a-bare-id-markdown-link-points-at-the-declarations-heading-anchor) and forces a different anchoring strategy. The cost is far higher than the symmetry gain. |
| Keep the historical form and just document multi-declaration comments | The reading cost (two stacked H1s in a Rustdoc) is the actual friction. Documentation alone wouldn't address it. |
| Synthesize the missing `#` at scan time so the internal representation is uniform | The internal representation already normalizes on the `Id`; the friction is at the *authoring* surface, not the data model. Adding a normalization layer doesn't help the author. |
| Forced migration: warn (or error) on the old form | Punishes existing repos for no real win. `grund fmt` is the right place for cosmetic conversions; the checker stays semantic. |

# DF-md-link-anchor-strategy: heading-text slugs, re-derived on every fmt pass

**Status:** Accepted
**Date:** 2026-05-09

## 1. Context

§DF-md-link-emission decided that `gnd fmt --md-links` wraps citations in clickable Markdown links inside `.md` files. That decision left §DF-md-link-emission.2.2 with a placeholder anchor format — section-coordinate slugs of the shape `#3-1` derived from `.3.1` — and an "idempotency" rule (§FS-fmt.6.3) that told `fmt` to leave existing wrap URLs alone once written.

Both placeholders are wrong on review. The section-coordinate slug does not match what standard Markdown renderers actually produce: GitHub's slugger strips punctuation rather than converting it (`### 3.1 Inputs` → `#31-inputs`, not `#3-1`); Pandoc's `auto_identifiers` algorithm differs further; MkDocs' TOC extension differs again. A repo emitting `#3-1` would render dead anchors in every renderer in common use. The "leave URLs alone" rule, in turn, lets the wrap go stale silently — a heading edit or a file move would invalidate every wrap pointing at it, and `fmt` would never repair them.

This DR picks the anchor strategy and tightens the idempotency rule.

## 2. Decision

### 2.1 Strategy

**Heading-text slugs**, derived per a configurable renderer profile, **re-derived on every `gnd fmt --md-links` pass**.

The scanner records each section heading's text alongside its section path (small extension to §AS-scanner.2.2). When `gnd fmt --md-links` emits a wrap, it looks up the heading text for the target section and slugifies it using the configured renderer profile. With the default `github` profile, a citation `§FS-fmt.6.2` (heading `### 6.2 Form`) becomes:

```text
[§FS-fmt.6.2](../functional-spec/FS-fmt.md#62-form)
```

The slug `#62-form` matches what GitHub actually produces — the anchor is clickable in the rendered doc.

### 2.2 Re-derive on every pass, supersede §FS-fmt.6.3

Every `gnd fmt --md-links` invocation recomputes the canonical URL inside each existing wrap and rewrites if it differs. The pass remains idempotent — a second run with no intervening edits is a no-op, because the URL on disk is now equal to the canonical URL — but the rule shifts from "preserve what is there" to "make what is there canonical." This is the same property the existing trigger→marker pass (§FS-fmt.2.1) already has: `fmt` is a normalizer, and normalizers do not preserve drift.

The consequence: heading edits and file moves that invalidate a wrap produce a one-line `fmt` diff on the next pass, instead of a silently-broken link. With `fmt --check` in CI and a pre-commit hook (§FS-fmt.4), the window between drift and re-derive is bounded by one commit.

### 2.3 Renderer profiles

`[fmt.md_links] anchor_format` ships with named profiles from day one:

- `github` (default) — GitHub's slugger: lowercase, strip punctuation, replace whitespace runs with `-`, collapse consecutive `-`. Covers GitHub, the most common host.
- `gitlab` — GitLab's slugger (similar to GitHub with minor Unicode-handling differences).
- `mkdocs` — MkDocs / Python-Markdown TOC extension's slugger.
- `pandoc` — Pandoc's `auto_identifiers` algorithm.
- `none` — no anchor; emit a file-level link with no fragment. Reader lands at the top of the target file. This is the same behavior §DF-md-link-emission.2.3 already specifies for source-file declarations.

A repo using a renderer with no matching profile selects `none` until a profile is added. Adding a new profile is a small contribution behind a focused e2e fixture.

## 3. Why this fits gnd's goals

The reframed §raison-detre.2 names three pillars — verify, refactor-safe, extract — and explicitly positions Markdown links as *not* a pillar: *"Markdown links cover navigation in rendered docs. The three above are the load-bearing ones."* `--md-links` is a peripheral convenience layer over the canonical citation grammar, not a load-bearing feature. That positioning is the test this decision is graded against.

- **§G-no-dangling-refs.** Untouched. Wraps are emitted from validated citations; the citation form §gnd checks is unchanged.
- **§G-polyglot-citation.** Untouched. The `§<KIND>-<slug>.<section>` grammar remains the canonical, source-of-truth form across `.md` and every supported source-comment host. Wrap is a presentation layer over `.md` only.
- **§G-fast-feedback.** `fmt` is not on the keystroke path (`check` is). The added scanner work is one extra string per declaration; the slugifier is a per-emission pure function.
- **§G-zero-config.** `--md-links` is opt-in (§DF-md-link-emission.2.4); the `github` default fits the most common hosting case, so opting in works out-of-the-box for the majority.
- **§G-friendliness-first.1's "no surprises" bullet (no surprises).** Same input + same config → same output bytes. No mutation of headings, no HTML injected into source `.md`. A reader scanning the source sees the same characters they wrote, only wrapped in `[…](…)`.
- **§G-configurable.** Renderer profiles are first-class and named.
- **§G-no-silent-breakage.** Holds *conditional on running `fmt`* — the same condition that already governs every other `fmt`-managed normalization in the project. Heading edits and file moves produce a one-line diff on the next `fmt` pass, not silent breakage.
- **§FS-non-goals.1 (no link validation).** `fmt` emits; it does not validate. The emitted URL is correct-by-construction (gnd computed it), but `gnd check` continues to validate only the citation inside the brackets, exactly as §DF-md-link-emission.3.1 says.
- **§FS-non-goals.5 (no rendered docs).** Output remains `.md`; no HTML is injected anywhere; the file tree is unchanged in shape.
- **§FS-non-goals.13 (two installs agree).** Same input + same config → byte-identical output. A renderer-side mismatch between `github` and `pandoc` profiles is not a disagreement between two correctly-configured installs — it is the project asking the user which profile is correct.

## 4. Consequences

- §FS-fmt.6.2's "Anchor format" bullet is rewritten to reference this DR and the heading-text strategy.
- §FS-fmt.6.3's idempotency rule changes from "leave wrapped URLs alone" to "recompute and rewrite if different." Idempotency itself holds — second-run-with-no-edits is a no-op.
- §FS-fmt.6.7's `anchor_format` config gains the named-profile shape from §2.3 above.
- §AS-scanner.2.2 is extended to record heading text per section, in addition to the existing section path.
- §DF-md-link-emission.2.2 is superseded by this DR. The section-coord stability framing in that section is retracted.
- §RM-md-link-emission's "What" section grows by one item: implement the renderer-profile slugifiers and the heading-text storage in `Findings`.

## 5. Alternatives considered

The four anchor strategies surveyed before this decision:

| Approach | Why rejected (or how folded in) |
|---|---|
| **(b) Anchor injection.** `gnd fmt` rewrites every section heading to embed an explicit `<a id="6-2"></a>` tag, then wraps cite as `#6-2`. Renderer-portable; immune to heading-text edits. | Two costs the project will not absorb for a peripheral convenience feature. (1) gnd would write literal HTML into source `.md` headings, which sits uncomfortably close to §FS-non-goals.5 ("does not generate rendered documentation") even on the strict reading; the optics of "gnd is editing my headings to add HTML" undermine the trust relationship. (2) §G-friendliness-first.1's "no surprises" bullet — opting in rewrites every heading in `docs/`, surprising the user with their own diff. The technically strongest answer; too invasive for what `--md-links` is. |
| **(c) Per-renderer format with no default.** User must pick a profile before `--md-links` works. | Not actually a separate option — it is how (a) gets shipped configurably. Without specifying the underlying slug strategy, this is a deferral, not a decision. Folded into (a) as the renderer-profile config in §2.3. |
| **(d) No anchors, file-only links.** Every wrap targets the file with no fragment; reader scrolls to the section. Renderer-universal, trivial implementation, contractually cleanest. | Delivers minimal value over what `gnd show <ID>.<section>` already provides at the CLI: the link takes you to the file, not the section. If the peripheral convenience adds nothing beyond `show`, the spec, the flag, the config block, and the e2e fixtures are not justified. Retained as the `none` profile in (a)'s config — a sane fallback when no renderer profile fits. |

The clinching argument for (a) over (b): the raison-detre frames `--md-links` as a "free convenience layer" — peripheral, not load-bearing. Peripheral features should not push on §FS-non-goals or surprise users in source markdown. (a) keeps gnd's machinery invisible in the source form (no HTML injection), accepts a brittleness that the project's own contract (run `fmt`) makes a non-event, and delivers section-level navigation in the renderer. (d) is contractually safer but the link's value collapses to "open the file," which is too thin to ship.

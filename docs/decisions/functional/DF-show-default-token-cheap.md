# DF-show-default-token-cheap: grund show defaults to the cheap read; the full body is opt-in

**Status:** Accepted
**Date:** 2026-05-14
**Supersedes:** [§DF-show-token-cheap-reads](DF-show-token-cheap-reads.md#df-show-token-cheap-reads-grund-show-keeps-the-full-body-default-token-cheap-slices-are-opt-in) (which pinned the full body as the default and kept the cheap slices opt-in).

## 1. Context

[§DF-show-token-cheap-reads](DF-show-token-cheap-reads.md#df-show-token-cheap-reads-grund-show-keeps-the-full-body-default-token-cheap-slices-are-opt-in) added cheap read modes (`--head`, `--outline`, `--brief`) as opt-in flags and left `grund show <ID>` printing the full body. The reasoning at the time: changing the default would be a user-visible output change, and humans on the terminal want the body. After living with the four-flag surface — `--head` / `--outline` / `--brief` / `--full` — the surface itself turned out to be the friction. Agents (the actual cost-bearers) kept reaching for the wrong slice: `--head` sounds like "summary" but it's "lead prose up to `## 1.`"; `--outline` sounds like "table of contents" but it's "headings only, no prose"; `--brief` is the recommended first read but its name doesn't make that obvious. And the human terminal use case the old decision protected is rarer than the agent one — humans reading specs read them in an editor, not by piping `grund show` to `less`.

This decision flips the default to the cheap read and renames the flags around an incremental mental model: each level adds to the previous one.

## 2. Decision

The `grund show <ID>` modes are an ordered four-level ladder, named by what each level adds:

| Flag | What it adds to the previous level | Typical use |
|---|---|---|
| `--brief` | declaration title + first paragraph | hover preview, "what is this about?" |
| (no flag, the new default) | + the rest of this section's prose, cut at the first child section heading | resolving a bare `§<ID>` citation |
| `--toc` | + the headings of nested subsections | choosing which `§<ID>.<sec>` to fetch next |
| `--full` | + the bodies of all nested subsections (full recursion) | reading the whole declaration |

The flags are mutually exclusive: each picks one point on the "how much" axis. Section-scoping (`grund show <ID>.<sec>`) re-roots the same four levels at a subsection — `--brief` on a section is "section heading + first paragraph", default is "lead prose down to the first sub-subsection", `--toc` adds the nested heading map, `--full` includes the bodies.

### 2.1 What changes from [§DF-show-token-cheap-reads](DF-show-token-cheap-reads.md#df-show-token-cheap-reads-grund-show-keeps-the-full-body-default-token-cheap-slices-are-opt-in)

- **Default flips.** `grund show <ID>` no longer prints the full body; it prints the section's lead prose (cut at the first child section heading). To get today's behavior, pass `--full` explicitly.
- **`--head` is removed.** Its semantics ("everything before the first `## 1.`") become the new default.
- **`--outline` is removed.** A pure-map mode (headings with no prose) is dropped from the surface. Agents that want just the section coordinates use `--toc` and pay for one paragraph of prose; the cost is small and the cognitive surface shrinks.
- **`--brief` is repurposed.** Today's `--brief` (head + outline) is replaced by `--toc` (new default + outline). The flag name `--brief` is reused for a new, strictly shorter slice: title + first blank-line-separated paragraph.
- **`--toc` is added.** It is the new "default plus the section map", which is the move when an agent needs to choose a subsection to fetch.

### 2.2 The naming principle: each flag names what it adds

Today's four flags name *what they show* (head, outline, brief, full) — orthogonal-sounding but not actually orthogonal, and a reader has to memorize each one. The new four flags name *what they add to the previous level*: `--brief` adds the title+1 paragraph; the default adds the rest of the lead; `--toc` adds the section headings; `--full` adds the subsection bodies. The order is the ladder, and the ladder is the recommended workflow: open with the cheapest slice that answers the question; escalate one rung at a time.

### 2.3 Slices stay structural, never generated

Unchanged from [§DF-show-token-cheap-reads.2.2](DF-show-token-cheap-reads.md#22-slices-are-structural-never-generated): every byte of every slice is a structural subset of bytes `grund` already parsed — heading lines, blank-line-terminated paragraphs, section maps. No model summarization, no paraphrase. Byte-determinism on `(tree, config)` is preserved ([§FS-non-goals.13](../../functional-spec/FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree), [§FS-non-goals.14](../../functional-spec/FS-non-goals.md#14-generated-summaries-token-saving-inside-check)).

### 2.4 `check` is not abridged

Unchanged from [§DF-show-token-cheap-reads.2.3](DF-show-token-cheap-reads.md#23-check-is-not-abridged): the token economy applies to the read/query surface only. `grund check` diagnostics stay complete.

## 3. Why this fits grund's goals

- [§GOAL-token-economy](../../goals/goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file) / [§GOAL-friendliness-first](../../goals/goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) — the cheapest read is now the default. An agent that reaches for `grund show <ID>` without thinking gets the right amount of spec, not 15 KB.
- [§GND-grund](../../grund.md#gnd-grund-agents-stay-grounded-in-the-spec) — grounding is more likely when grounding is cheap. The four-level ladder gives an agent a deliberate next move at every rung.
- [§GOAL-no-silent-breakage](../../goals/goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) — this is a user-visible output change. It ships through the managed `AGENTS.md` block version bump and a release note; the `--full` flag remains the way to get the prior default.

## 4. Consequences

- [§FS-show.1](../../functional-spec/FS-show.md#1-inputs) is rewritten around the new flag set; the `--head` / `--outline` / `--brief` / `--full` mutual-exclusion group becomes `--brief` / (none) / `--toc` / `--full`.
- [§FS-show.2.1](../../functional-spec/FS-show.md#21-whole-declaration-default) defines the new default semantics: lead prose, cut at the first child section heading. The `--head` and `--outline` subsections are removed; `--brief` and `--toc` get new subsections.
- [§FS-show.3.1](../../functional-spec/FS-show.md#31-format-variants) drops the `head` JSON field (no longer meaningful when the head *is* the body); `sections` now appears with `--toc` only.
- The managed `AGENTS.md` block (`templates/AGENTS.md`) advertises the new ladder; the `grund-agents` block version bumps and `init-agents-*` e2e fixtures refresh.
- The `show-head-*`, `show-outline-*`, and most `show-brief-*` e2e cases are renamed/replaced; new `show-toc-*` cases cover the new flag.
- The `--head` and `--outline` flags are removed without aliasing. A repo that scripted them sees a hard error; the upgrade is mechanical (`--head` → no flag, `--outline` → `--toc`, old `--brief` → `--toc`). This is a 0.x change; the cost of carrying deprecated aliases for a 0.x flag rename is higher than the cost of the rename.

## 5. Alternatives considered

| Approach | Why rejected |
|---|---|
| Keep the [§DF-show-token-cheap-reads](DF-show-token-cheap-reads.md#df-show-token-cheap-reads-grund-show-keeps-the-full-body-default-token-cheap-slices-are-opt-in) surface, just rename `--brief` → recommended in docs | The friction is the surface, not the docs. Four orthogonal-sounding flags that aren't actually orthogonal stay confusing no matter how the README frames them. |
| Add `--toc` as a new flag, keep `--head` / `--outline` / `--brief` as aliases | Preserves scripts but doubles the surface and keeps the four confusing names in `--help`. The point of the change is to *shrink* the surface, not extend it. |
| Keep `--outline` for pure-map agents | A pure-map mode saves a paragraph of prose. The reduction in surface area is worth more than the prose savings; `--toc` covers the "I need the section map" case at marginal cost. If telemetry later shows heavy `--outline` use we can revisit. |
| Two orthogonal flags (`--lead-only`, `--recurse`) instead of named modes | More configurable in theory; harder to teach, and the four useful combinations all lie on a single diagonal. Named modes for the useful points, with orthogonal flags reserved for if an off-diagonal use case ever lands. |

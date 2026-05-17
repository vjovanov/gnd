# DF-show-keep-explicit-form: grund keeps `show` as a subcommand alongside the bare-ID default

**Status:** Accepted
**Date:** 2026-05-18

## 1. Context

[§DF-show-default-token-cheap](DF-show-default-token-cheap.md#df-show-default-token-cheap-grund-show-defaults-to-the-cheap-read-the-full-body-is-opt-in) made the cheap lead the default for `grund show`. [§FS-cli.1](../../functional-spec/FS-cli.md#1-the-default-subcommand) then flipped the default subcommand so `grund <ID>` is shorthand for `grund show <ID>` — the shortest possible move for an agent resolving a bare `§<ID>` citation. That raises an obvious follow-on question: should the explicit `show` subcommand be removed altogether and the bare-ID form be the only spelling?

The case for removal is real. Two canonical spellings for the most important action — `grund FS-login` and `grund show FS-login` — invites the "two ways to do it" critique. The bare form linguistically reinforces the model "reading a cited fact is the default thing `grund` does." And there are no external users yet, so the migration cost is zero. The case for keeping `show` as a public subcommand turns out to be larger.

## 2. Decision

`show` remains a documented public subcommand. `grund <ID>` is the recommended cheap default ([§FS-cli.1](../../functional-spec/FS-cli.md#1-the-default-subcommand), [§FS-show.1](../../functional-spec/FS-show.md#1-inputs)); `grund show <ID>` is the byte-for-byte equivalent explicit form. Both spellings are supported indefinitely.

### 2.1 What the bare form cannot replace

Three concrete failures of a bare-only design — each independently sufficient to keep the explicit form:

- **Subcommand-level help.** `grund show --help` is the densest help page in the CLI: six flags (`--brief`, `--toc`, `--full`, `--section`, `--path`, `--format`) with mutual-exclusion rules, the ladder of "how much" rungs, and per-flag examples. Folding that page onto top-level `grund --help` busts the one-screen budget set in [§GOAL-friendliness-first.1](../../goals.md#1-hard-requirements). Replacing it with a free-floating `grund help read` topic would introduce a help-topic mechanism the CLI does not otherwise have, just to host one page. Subcommand `--help` pages already exist for every other command; `show` should keep the affordance the rest of the surface offers.
- **Scriptability.** `grund show "$id"` survives `$id` being empty, beginning with `-`, or lexing as a subcommand name. Bare `grund "$id"` does not: an empty `$id` prints help, a `-`-prefixed `$id` is read as a flag, an `$id` that happens to match a subcommand name dispatches there silently. Scripts that pipe IDs through `xargs`, generate them from `grund list`, or read them from user input want the explicit spelling. Removing the explicit form would push that brittleness onto every caller.
- **Grep-ability and review.** `grund show <ID>` is greppable across a repo of scripts and docs; bare `grund <ID>` is a substring of every other `grund` invocation. When reviewing an agent's actions, a CI script, or a contributor's docs PR, the explicit form names what it is doing.

### 2.2 The bare form is the shorthand, not the only form

The pattern is the same as `git log` ≡ `git log HEAD` and `make` running the first target without naming it: a documented shorthand for the overwhelmingly common case alongside an explicit form for scripts, help, and discoverability. The shorthand teaches the mental model — "`grund <ID>` is what you do" — and is what an agent reaches for after [§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints)'s `AGENTS.md` block teaches it the default. The explicit form supports the cases where the shorthand cannot reach. The two are not redundant; they are layered.

### 2.3 The spec ID `FS-show` stays

`FS-show` continues to declare the body-reading behavior. Its prose already names both spellings (the explicit `show` form and the bare `<ID>` default — see [§FS-show.1](../../functional-spec/FS-show.md#1-inputs)). Renaming it to `FS-read` for cosmetic alignment with the agent-facing default would carry a docs-wide ID rename — every citation, every `grund refs FS-show` site, the kind home filename — for no semantic gain.

## 3. Why this fits grund's goals

- [§GOAL-friendliness-first.1](../../goals.md#1-hard-requirements) — the top-level help stays one screen because the dense flag surface lives on `grund show --help`. Every other subcommand pays the same way; `show` should not be the outlier whose help has nowhere to go.
- [§GOAL-token-economy](../../goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file) — the bare form *is* the cheap default; the explicit form does not change the read economy. The agent-facing recommendation in [§FS-init.2.3](../../functional-spec/FS-init.md#23-generated-agent-entrypoints) teaches `grund <ID>`, and that is what an agent reaches for.
- [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) — keeping `show` means the explicit form already used in our own templates, examples, and `agent-setup-instructions` output stays working without a deprecation window. Removing it would force a coordinated rewrite of every doc snippet and every script that uses it, with no behavior gain on the other side.
- [§GND-grund](../../grund.md#gnd-grund-agents-stay-grounded-in-the-spec) — grounding is more likely when both an agent (cheap bare form) and a human writing a CI script (explicit form) have a comfortable way in.

## 4. Consequences

- [§FS-cli.1](../../functional-spec/FS-cli.md#1-the-default-subcommand) and [§FS-show.1](../../functional-spec/FS-show.md#1-inputs) keep their existing wording — both spellings are byte-for-byte equivalent, and the bare form is the recommended default. The wording predates this DF; the DF records why it stays.
- [§FS-completions.1](../../functional-spec/FS-completions.md#1-user-facing-command) keeps offering declared IDs after `grund show` *and* after a bare `grund` first word — completion serves both spellings.
- `grund --help` keeps listing `show` in its commands block (with `(default)` next to it). `grund show --help` keeps the dense flag page.
- No deprecation surface, no alias removal, no migration step: this DF documents the status quo the [§FS-cli.1](../../functional-spec/FS-cli.md#1-the-default-subcommand) flip established and locks it in.
- Future direction: if telemetry ever shows the explicit form is genuinely unused — and a deprecation path through [§GOAL-no-silent-breakage](../../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) is in place — this decision can be revisited.

## 5. Alternatives considered

| Approach | Why rejected |
|---|---|
| Remove `show` entirely; the bare-ID form becomes the only spelling | Loses `grund show --help` (six-flag dense page that cannot fold onto top-level help without busting [§GOAL-friendliness-first.1](../../goals.md#1-hard-requirements)); makes `grund "$id"` brittle when `$id` is empty, `-`-prefixed, or subcommand-shaped; reduces grep-ability across scripts and docs. The "two ways" cost is real but smaller than these three concrete losses, and `git log` / `make` show the shorthand-plus-explicit pattern works. |
| Remove `show` but add `grund help read` as a free-standing help topic | Introduces a help-topic mechanism the CLI does not otherwise have, just to host one page. Subcommand `--help` already covers this case at zero new mechanism. |
| Keep `show` working but hide it from `grund --help` (un-document it) | Hidden subcommands are worse than removed ones: scripts can still use them, but new readers cannot discover them. Worst of both worlds, and discoverability is half of [§GOAL-friendliness-first](../../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible). |
| Rename `show` → `read` for closer alignment with the agent-facing default | Trades one familiar name for another for no behavior change. The cost of renaming the `FS-show` spec ID and every citation is not worth it; the bare form, not the explicit form, is what an agent reaches for in practice. |

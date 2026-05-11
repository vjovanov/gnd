# DF-id-number-width: grund id zero-pads minted numbers to a default width of 3

**Status:** Accepted
**Date:** 2026-05-11

## 1. Context

A repo whose `[id] format` carries a `{number}` placeholder — the `grund init` default `{kind}-{number}-{slug}` — needs `grund id` to decide how many digits to pad the next number to. `FS-7` and `FS-007` are the same number; which one does the tool emit? The choice is small but, once a handful of IDs exist, sticky: an ID is an immutable handle ([§FS-non-goals.4](../../functional-spec/FS-non-goals.md#4-cross-workspace-id-renaming), [§FS-id.4](../../functional-spec/FS-id.md#4-next-number-derivation)), so a width chosen on day one is the width every ID of that kind below the rollover threshold wears forever.

(Repos with no `{number}` placeholder — `{kind}-{slug}`, the form `grund` itself uses — are unaffected: there is no number to pad, `--width` is inert, and the `--format json` `number` field is `null` — [§FS-id.4.1](../../functional-spec/FS-id.md#41-number-less-id-formats).)

## 2. Decision

`grund id`'s `--width` flag defaults to **3**. The next number is left-padded with zeros to at least three digits — `FS-001`, `FS-042`, `FS-999` — and a number that already has more digits is emitted as-is (`FS-1000`). It is a per-invocation CLI flag, **not** an `[id]` config key: there is no `[id] number_width`. A repo that wants a different repo-wide width passes `--width N` on each `grund id` call (see §5 for why that is deliberate, for now).

## 3. Why 3

- **It is the documented canonical shape.** Every worked example — the `AGENTS.md` scaffold, the README, the spec bodies, the `grund init` templates — writes a numbered ID as `<KIND>-<NNN>-<slug>`. `grund id` is the tool that mints those IDs; its output has to look like the convention it exists to enforce. Defaulting to 2 or 4 would mean either the tool contradicts the docs or the docs all gain or lose a digit of leading zeros. Serves [§GOAL-friendliness-first](../../goals/goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) (no surprises).
- **Zero-padding is what makes IDs sort numerically.** `FS-002 < FS-010 < FS-100` in `ls`, in `grund list`, in a file tree, in `git log --stat`. Unpadded, `FS-10` sorts before `FS-2`. So *some* fixed width is required; 3 is the smallest that keeps the sort correct for the first 999 declarations of a kind.
- **3 digits is headroom enough that you stop thinking about it.** These are hand-authored declarations — someone writes each spec, goal, or decision. A repo with more than 999 declarations of a *single* kind is vanishingly rare; for practically every project, all IDs of a kind are the same width forever, so the padding "settles" immediately and never churns.
- **It is a floor, not a cap.** Choosing 3 limits nothing — `FS-1000` is a perfectly valid ID. The default only decides the visual width *below* the rollover, so the cost of picking it "too small" is bounded (§4).

## 4. What happens at the rollover

When a kind crosses 999, `grund id` emits `FS-1000-…` and the repo is **permanently mixed-width**: `FS-001`…`FS-999` stay 3 digits, `FS-1000`+ are 4. `grund` never re-pads `FS-001` → `FS-0001` — an ID is an immutable handle ([§FS-non-goals.4](../../functional-spec/FS-non-goals.md#4-cross-workspace-id-renaming)); re-padding would silently break every `§FS-001` citation in prose and code and every external reference (PRs, chat, mirrored repos), and there is no `grund fmt --renumber` for the same reason. The fallout is entirely cosmetic: every command still resolves the IDs (the grammar is `number_pattern` — "any run of digits" — not "exactly three digits"); only `grund list`'s lexical sort gets a local wrinkle at the boundary (`FS-1000 < FS-101` as strings), and a consumer that wants true numeric order sorts on the `number` field of `grund list --format json`. So the rollover degrades gracefully — which is precisely why 3, not 4, is the right floor (§5).

## 5. Alternatives considered

- **Default width 2.** Rejected. The sort stays correct only to 99 — a threshold real spec sets reach — and the day you go from `FS-99` to `FS-100` you have a mixed-width repo and a cosmetic churn, early, for no benefit.
- **Default width 4.** Rejected. It does not *solve* the rollover, it relocates it from 999 to 9999 — a threshold even fewer repos reach. The price is a leading `0` on *every* ID in *every* repo, forever (`FS-0008-user-can-log-in`), plus a mismatch with the documented `-NNN-` form — a universal, unconditional cost paid to defend against an event that is both extremely rare and (per §4) harmless when it does occur. The asymmetry runs the wrong way: pay the tiny rare cost, not the small universal one.
- **An `[id] number_width` config key (default 3) that `grund id` reads when `--width` is omitted.** Not done now, but the door is left open: it is additive (a config without the key behaves exactly as today), needs no `grund_config_version` bump ([§FS-config.5](../../functional-spec/FS-config.md#5-schema-versioning)), and mirrors how `number_pattern` already lives in `[id]`. Its case is a repo that *knows* it will be large and wants the width pinned without every contributor remembering `--width` — the [§GOAL-small-and-large](../../goals/goals.md#goal-small-and-large-start-small-configure-for-big) / [§GOAL-configurable](../../goals/goals.md#goal-configurable-every-default-is-overridable) instinct. Until such a repo asks for it, a CLI flag plus a documented default is enough; adding the key now is config surface for a need no one has expressed.

## 6. Consequences

- `command_id` in `src/lib.rs` keeps `let mut width = 3usize;` as the default, overridable by `--width`.
- [§FS-id.1](../../functional-spec/FS-id.md#1-inputs)'s `--width` bullet carries the [§DF-id-number-width](DF-id-number-width.md#df-id-number-width-grund-id-zero-pads-minted-numbers-to-a-default-width-of-3) citation.
- No change to the `[id]` config schema, `templates/grund.toml`, or `grund config show` — the width is deliberately not a config key (§5).

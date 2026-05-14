# FS-cli: grund's command-line surface conventions

The behaviour that is not owned by any one subcommand — how `grund` is invoked with no subcommand, the two global flags that short-circuit before any work, and the cross-subcommand flags. Serves [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) (one screen of help, no surprises) and [§GOAL-no-silent-breakage](../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) (the CLI surface — subcommands, flags, exit-code mapping — is user-visible and frozen).

## 1. The default subcommand

- `grund` with no arguments is `grund check .`.
- `grund <path>` (where `<path>` is not a known subcommand) is `grund check <path>`.
- `grund <subcommand> …` dispatches to that subcommand: `check`, `show`, `list`, `refs`, `cover`, `fmt`, `id`, `init`, `config`, `agent-setup-instructions`, `completions`. The hidden `complete` subcommand is reserved for generated shell scripts ([§FS-completions.2](FS-completions.md#2-internal-dynamic-helper)).

A bare `grund <path>` and an explicit `grund check <path>` are byte-for-byte equivalent ([§FS-check](FS-check.md#fs-check-grund-validates-every-reference-in-a-repo)). With no path, bare `grund` and explicit `grund check` are both byte-for-byte equivalent to `grund check .`. The shorthand exists because `check` is the overwhelmingly common invocation; the other subcommands are always spelled out.

Because the first non-flag word is read as a subcommand *or* a path, a mistyped subcommand would otherwise be reported as a missing file. So when `grund <word>` is neither a known subcommand nor an existing path, the message names both readings rather than only the path one:

```
error: unknown command or missing path: bogus
known commands: check, show, list, refs, cover, fmt, id, init, config, agent-setup-instructions, completions
(a bare path is shorthand for `grund check <path>`; run `grund --help` for commands)
```

Empty stdout, exit `2` — a CLI-level error like any other unknown subcommand (§4). When `grund <word>` *is* an existing path, it is `grund check <word>` and any "path does not exist" comes from `check` itself, not from this routing step.

## 2. Global flags

These are recognised regardless of subcommand and are handled *before* any tree scan or file write:

- `grund --version` (alias `grund -V`) — prints `grund <semver>` on stdout and exits `0`. Nothing else is printed; the output is one line and is deterministic for a given build. This is the affordance the [§GOAL-no-silent-breakage](../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) deprecation path relies on — a warning that names "the release in which the old form will stop working" is only actionable if the user can ask which release they are on.
- `grund --help` (alias `grund -h`) — prints the top-level help on stdout and exits `0`. The page opens with a one-line statement of what `grund` is, then the two invocation forms (`grund [check] <path>` and `grund <command> …`), then a `Commands:` block — every subcommand on its own line with a one-line description and a sample invocation — then the cross-subcommand options. The whole page fits one screen (≤ 24 lines for the top-level page per [§GOAL-friendliness-first.1](../goals.md#1-hard-requirements)), so each description is a single terse line: the `show` line still gestures at *why* the command exists ("Print one declaration body for agent context."), with the full rationale on `grund show --help`. Every flag carries a one-line example. Help is never an error: it goes to stdout, exit `0`, so `grund --help | …` works.
- `grund help <subcommand>` and `grund <subcommand> --help` (and `grund <subcommand> -h`) print *that subcommand's* page on stdout, exit `0`: its usage line, its arguments, every flag with a one-line example, the exit-code meanings for that subcommand, and a one-line recovery hint where the common failure has an obvious next step (e.g. `show`'s page says how to find an ID; `id`'s page shows the `$EDITOR` follow-up). `grund help` with no argument is the top-level page; `grund help <unknown>` is the unknown-command error (§4). `--version` still outranks everything — `grund show --version` is the version line, not the `show` page.

When both a global flag and a subcommand are present, the global flag wins: `grund check --version` prints the version and exits `0` without scanning.

## 3. Cross-subcommand flags

- `--format text|json` — accepted by the subcommands with a machine-readable result or finding surface: `check`, `show`, `list`, `refs`, `cover`, and `id` ([§FS-errors.5](FS-errors.md#5-json-format)). `text` is the default; `json` opts into the stable machine shapes. The stream split is the same as the text form ([§FS-errors.1](FS-errors.md#1-streams), [§FS-distribution.3.0](FS-distribution.md#30-language-neutral-data-shapes)): the command's output — `grund check`'s findings as NDJSON when diagnostics exist, a query result as one object (or NDJSON for a list command) — goes to stdout, while a failed `grund show` query's diagnostic and any CLI-level `error:` go to stderr. `grund check --format=json` stays diagnostics-only and does not emit the text-mode `success` marker. It is not a global flag: operational commands whose output is human text or generated files (`fmt`, `init`, `config`, `agent-setup-instructions`, `completions`) reject `--format` unless their own help page says otherwise.
- A path argument, when a subcommand takes one, defaults to `.` and is resolved the same way everywhere (config discovery walks up from it — [§FS-config.1](FS-config.md#1-file-location-and-discovery)). Every path-taking subcommand accepts **at most one** path: a second positional — `grund check a b`, `grund show ID a b`, `grund refs ID a b`, `grund cover a b`, `grund fmt a b`, `grund list a b`, `grund id FS "t" a b` — is a CLI-level error (`error: <subcommand> takes at most one path argument`, exit `2`, §4), never a silent use of one and a quiet drop of the rest. `config` and `agent-setup-instructions` already enforce this; the rule is uniform across the surface, so a typo'd path is reported, never absorbed.

## 4. Errors with no source location

An unknown subcommand (including `grund help <unknown>`), an unknown or malformed flag, or mutually-exclusive flags (e.g. [§FS-init](FS-init.md#fs-init-grund-bootstraps-a-new-grund-conformant-repo)'s `--append` with `--force`) are CLI-level errors: `error: <message>` on stderr, empty stdout, exit `2` ([§FS-errors.2.2](FS-errors.md#22-cli-level-message), [§FS-check.2.1.1](FS-check.md#211-cli-level-messages)). CI scripts grep for the leading `error:` to distinguish a launch-time failure from a clean run that found findings on stdout. A bare-word first argument that is neither a known subcommand nor an existing path gets the dual-reading message from §1; any following lines (`known commands: …`, the parenthetical hint) are part of that diagnostic, not separate findings.

## 5. Exit-code mapping is fixed

`0` clean / printed, `1` findings or a failed query, `2` scan or CLI-level failure — the precise meaning per subcommand is in that subcommand's spec, but the *mapping* is frozen per [§GOAL-friendliness-first.2](../goals.md#2-what-this-rules-out) and [§FS-non-goals.9](FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization): it is not configurable, and a change to it goes through the [§GOAL-no-silent-breakage](../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) deprecation path.

## 6. What is deliberately absent

- No `--quiet` / `--verbose` knobs that change which findings print — severity is fixed ([§FS-non-goals.9](FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization)), and a passing text `grund check` already has a single fixed `success` line ([§GOAL-friendliness-first.1](../goals.md#1-hard-requirements)).
- No `--config <file>` override — config is discovered by walking up from the command path, not pointed at directly, to keep two installs on the same tree in agreement ([§FS-config.1](FS-config.md#1-file-location-and-discovery)). `grund config show [path]` reports what was discovered from that starting path.
- No interactive flags, no TUI, no prompts ([§FS-non-goals.10](FS-non-goals.md#10-interactive-mode)).
- No `grund graph`, no `grund new` — graph visualisation is not a committed feature ([§FS-non-goals.6](FS-non-goals.md#6-decision-database-audit-log-history-tracking)), and file creation for a new declaration is the caller's job after `grund id` ([§FS-id.7](FS-id.md#7-what-id-does-not-do)).

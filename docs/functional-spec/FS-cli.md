# FS-cli: gnd's command-line surface conventions

Status: the default-subcommand routing, the global `--help`/`-h` and `--version`/`-V`, the cross-subcommand `--format` flag, and the per-subcommand help pages (`gnd <subcommand> --help` / `gnd help <subcommand>`) are all implemented today.

The behaviour that is not owned by any one subcommand — how `gnd` is invoked with no subcommand, the two global flags that short-circuit before any work, and the cross-subcommand flags. Serves §G-friendliness-first (one screen of help, no surprises) and §G-no-silent-breakage (the CLI surface — subcommands, flags, exit-code mapping — is user-visible and frozen).

## 1. The default subcommand

- `gnd` with no arguments is `gnd check .`.
- `gnd <path>` (where `<path>` is not a known subcommand) is `gnd check <path>`.
- `gnd <subcommand> …` dispatches to that subcommand: `check`, `show`, `list`, `refs`, `cover`, `fmt`, `name`, `init`, `config`, `agent-setup-instructions`, `completions`. The hidden `complete` subcommand is reserved for generated shell scripts (§FS-completions.2).

A bare `gnd <path>` and an explicit `gnd check <path>` are byte-for-byte equivalent (§FS-check). With no path, bare `gnd` and explicit `gnd check` are both byte-for-byte equivalent to `gnd check .`. The shorthand exists because `check` is the overwhelmingly common invocation; the other subcommands are always spelled out.

Because the first non-flag word is read as a subcommand *or* a path, a mistyped subcommand would otherwise be reported as a missing file. So when `gnd <word>` is neither a known subcommand nor an existing path, the message names both readings rather than only the path one:

```
error: unknown command or missing path: bogus
known commands: check, show, list, refs, cover, fmt, name, init, config, agent-setup-instructions, completions
(a bare path is shorthand for `gnd check <path>`; run `gnd --help` for commands)
```

Empty stdout, exit `2` — a CLI-level error like any other unknown subcommand (§4). When `gnd <word>` *is* an existing path, it is `gnd check <word>` and any "path does not exist" comes from `check` itself, not from this routing step.

## 2. Global flags

These are recognised regardless of subcommand and are handled *before* any tree scan or file write:

- `gnd --version` (alias `gnd -V`) — prints `gnd <semver>` on stdout and exits `0`. Nothing else is printed; the output is one line and is deterministic for a given build. This is the affordance the §G-no-silent-breakage deprecation path relies on — a warning that names "the release in which the old form will stop working" is only actionable if the user can ask which release they are on.
- `gnd --help` (alias `gnd -h`) — prints the top-level help on stdout and exits `0`. The page opens with a one-line statement of what `gnd` is, then the two invocation forms (`gnd [check] <path>` and `gnd <command> …`), then a `Commands:` block — every subcommand on its own line with a one-line description and a sample invocation — then the cross-subcommand options. The whole page fits one screen (≤ 24 lines for the top-level page per §G-friendliness-first.1), so each description is a single terse line: the `show` line still gestures at *why* the command exists ("Print one declaration body for agent context."), with the full rationale on `gnd show --help`. Every flag carries a one-line example. Help is never an error: it goes to stdout, exit `0`, so `gnd --help | …` works.
- `gnd help <subcommand>` and `gnd <subcommand> --help` (and `gnd <subcommand> -h`) print *that subcommand's* page on stdout, exit `0`: its usage line, its arguments, every flag with a one-line example, the exit-code meanings for that subcommand, and a one-line recovery hint where the common failure has an obvious next step (e.g. `show`'s page says how to find an ID; `name`'s page shows the `$EDITOR` follow-up). `gnd help` with no argument is the top-level page; `gnd help <unknown>` is the unknown-command error (§4). `--version` still outranks everything — `gnd show --version` is the version line, not the `show` page.

When both a global flag and a subcommand are present, the global flag wins: `gnd check --version` prints the version and exits `0` without scanning.

## 3. Cross-subcommand flags

- `--format text|json` — accepted by every subcommand that emits messages (§FS-errors.5). `text` is the default; `json` opts into the stable machine shapes (diagnostic NDJSON on stderr, result object on stdout — §FS-errors.5, §FS-distribution.3.0).
- A path argument, when a subcommand takes one, defaults to `.` and is resolved the same way everywhere (config discovery walks up from it — §FS-config.1).

## 4. Errors with no source location

An unknown subcommand (including `gnd help <unknown>`), an unknown or malformed flag, or mutually-exclusive flags (e.g. §FS-init's `--append` with `--force`) are CLI-level errors: `error: <message>` on stderr, empty stdout, exit `2` (§FS-errors.2.2, §FS-check.2.1.1). CI scripts grep for the leading `error:` to distinguish a launch-time failure from a clean run that found findings. A bare-word first argument that is neither a known subcommand nor an existing path gets the dual-reading message from §1; any following lines (`known commands: …`, the parenthetical hint) are part of that diagnostic, not separate findings.

## 5. Exit-code mapping is fixed

`0` clean / printed, `1` findings or a failed query, `2` scan or CLI-level failure — the precise meaning per subcommand is in that subcommand's spec, but the *mapping* is frozen per §G-friendliness-first.2 and §FS-non-goals.9: it is not configurable, and a change to it goes through the §G-no-silent-breakage deprecation path.

## 6. What is deliberately absent

- No `--quiet` / `--verbose` knobs that change which findings print — severity is fixed (§FS-non-goals.9), and a passing repo is already silent (§G-friendliness-first.1).
- No `--config <file>` override — config is discovered by walking up from the command path, not pointed at directly, to keep two installs on the same tree in agreement (§FS-config.1). `gnd config show [path]` reports what was discovered from that starting path.
- No interactive flags, no TUI, no prompts (§FS-non-goals.10).
- No `gnd graph`, no `gnd new` — graph visualisation is not a committed feature (§FS-non-goals.6), and file creation for a new declaration is the caller's job after `gnd name` (§FS-name.7).

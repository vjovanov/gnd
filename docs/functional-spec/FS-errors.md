# FS-errors: gnd emits messages in fixed shapes

This spec defines the style every `gnd` subcommand uses when it speaks to a user or to a downstream tool. It is cross-cutting: [§FS-check](FS-check.md#fs-check-gnd-validates-every-reference-in-a-repo), [§FS-show](FS-show.md#fs-show-gnd-reads-a-single-declaration-body-by-id), [§FS-list](FS-list.md#fs-list-gnd-lists-every-declared-id), [§FS-refs](FS-refs.md#fs-refs-gnd-lists-every-citation-of-an-id), [§FS-cover](FS-cover.md#fs-cover-gnd-groups-citations-by-scanned-file), [§FS-fmt](FS-fmt.md#fs-fmt-gnd-normalizes-references-in-bulk), [§FS-init](FS-init.md#fs-init-gnd-bootstraps-a-new-gnd-conformant-repo), [§FS-id](FS-id.md#fs-id-gnd-proposes-ids-for-new-declarations), [§FS-config](FS-config.md#fs-config-gnd-reads-a-toml-config-file-under-agents), and [§FS-completions](FS-completions.md#fs-completions-gnd-completes-declared-ids-in-shells) all conform to it, and the global-flag behaviour in [§FS-cli](FS-cli.md#fs-cli-gnds-command-line-surface-conventions) routes its errors through §2.2 here. Serves [§G-friendliness-first.1](../goals/goals.md#1-hard-requirements) ("errors point at the line", "no surprises") and [§G-no-silent-breakage.1](../goals/goals.md#1-what-counts-as-user-visible) (the message shapes are user-visible output).

The shapes are **frozen** by the same logic as [§FS-non-goals.9](FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization): two correctly-configured installs must agree on what they print. A subcommand that needs to say something new picks one of the shapes below; it does not invent an ad hoc one.

## 1. Streams

`gnd` follows the **linter convention** (`eslint`, `ruff`, `shellcheck`, `golangci-lint`): a checker's findings *are* its output, so they go to **stdout** — `gnd check | grep …`, `gnd check > findings.txt`, and `gnd check --format=json | jq …` all work with no stream redirection. `stderr` is reserved for what the command says *about* the run, not *as* its output.

- **stdout** carries the command's output:
  - a query result — the body printed by `gnd show`, the catalog from `gnd list`, the citations from `gnd refs`, the file graph from `gnd cover`, the ID from `gnd id`, the config from `gnd config show`;
  - a checker report — every located finding from `gnd check` ([§FS-check.2.1](FS-check.md#21-report-format)), the text-mode `success` marker from a clean `gnd check`, and the would-change / did-change report from `gnd fmt` ([§FS-fmt.3](FS-fmt.md#3-outputs)).
  - `gnd check --format=json` is diagnostics-only: on success with nothing to report, stdout is empty.
- **stderr** carries everything else:
  - `error:` lines — a launch-time failure or an I/O failure that means the run could not do its job (§2.2), always with a non-zero exit;
  - `warning:` lines about the run itself, not its content — e.g. an empty scan ([§FS-check.2.2](FS-check.md#22-empty-scan)) — exit unchanged;
  - `note:` / `hint:` recovery breadcrumbs ([§FS-refs.2](FS-refs.md#2-behaviour), [§FS-show.3](FS-show.md#3-outputs));
  - the bare message a *failed query* prints when it has no result to put on stdout (§2.3 — `gnd show` on a missing ID);
  - `gnd init`'s file-by-file transcript (§6 — `init`'s real output is the scaffold on disk; the transcript is progress).
- The two are never mixed: `gnd check 2>/dev/null` shows you the findings and only the findings; `gnd check >/dev/null` shows you only the run-level errors; `gnd show <ID> | …` is the body and nothing else.

## 2. The Fixed Shapes

### 2.1 Located finding

A diagnostic that points at a specific source site:

```
<path>:<line>: <message>
```

- `<path>` is relative to the config root when a `gnd.toml` was discovered ([§FS-config.3.6](FS-config.md#36-output--report-format)), otherwise to the path passed on the command line.
- `<line>` is 1-indexed.
- `<message>` is a single line — no embedded newlines, no terminal period.
- The `<path>:<line>:` prefix is mandatory: editors and agents jump on this exact shape.

Emitted on **stdout** — it is the command's output (§1): every finding from `gnd check` ([§FS-check.2.1](FS-check.md#21-report-format)), every would-change line from `gnd fmt` ([§FS-fmt.3](FS-fmt.md#3-outputs)), and every citation from `gnd refs` ([§FS-refs.3.1](FS-refs.md#31---format-text-default)) wears this shape. The optional LSP server surfaces the same `<path>:<line>: <message>` content as editor diagnostics ([§FS-lsp.1.1](FS-lsp.md#11-diagnostics)). The `<path>:<line>:` prefix is what editors and agents jump on; for `check` and `fmt` a line is a complaint about the repo, for `refs` it is an answer to a query — same shape, the exit code and the command tell them apart.

### 2.2 CLI-level message

A line that has no source location — it is about the *run*, not a site in the repo:

```
error: <message>
warning: <message>
```

- On **stderr** (§1) — it is not the command's output.
- The literal `error: ` / `warning: ` prefix is what distinguishes a CLI-level message from a located finding (which has the `<path>:<line>:` prefix instead). CI scripts grep for the leading `error:` to tell a launch-time failure from a clean run that found findings on stdout.
- No `<path>:<line>:` is attached, even if a config file is the cause — the message text names the file when relevant (e.g. `error: invalid gnd.toml: ...`).
- `error:` always accompanies a non-zero exit (`2` for a launch / I/O failure, `1` for a failed slug/collision query). `warning:` leaves the exit code alone — it is a caution, not a failure.

Used by [§FS-cli.4](FS-cli.md#4-errors-with-no-source-location) (unknown subcommand / bad flag), [§FS-id.3](FS-id.md#3-slug-derivation) (empty slug), [§FS-id.5](FS-id.md#5-collision-check) (collision), [§FS-config.6](FS-config.md#6-what-is-not-configured-here) (config validation), [§FS-check.2.1.1](FS-check.md#211-cli-level-messages) (a malformed config or a per-file read failure mid-walk), [§FS-check.2.2](FS-check.md#22-empty-scan) (the empty-scan `warning:`), and any subcommand reporting a launch-time failure. A *launch-time* `error:` (bad flag, unreadable config, missing path) is printed as raw text and is never JSON-ified; a *mid-walk* per-file failure collected by `gnd check` is one of the report's diagnostics and is rendered in `--format=json` like the others (§5), still on stderr because it is not a finding about the spec graph.

### 2.3 Bare query failure

When a *query* (a subcommand whose job is to return one body) finds something other than exactly one body — `gnd show` on a missing ID, a missing section, or an ambiguous ID:

```
<message>
```

- No prefix at all, on **stderr**, exit `1`. There is no single site to point at and no body to return, so stdout is empty; this line plus the exit code is what tells the caller what happened.
- Ambiguity messages list every site in lexicographic `path:line` order ([§FS-show.2.2.1](FS-show.md#221-ambiguous-id)). A `hint:` line may follow on stderr where the next step is obvious (§1).
- Distinct from §2.2: there is no `error:` prefix, because this is not a launch/run failure — the query ran fine, it just did not find one body.

Used only by query commands (currently `gnd show`). `check` does not use this shape — every line it prints is a located finding (stdout) or a CLI-level message (stderr).

### 2.4 Text success marker

A text-mode `gnd check` run with zero errors and zero warnings prints exactly:

```
success
```

One trailing newline follows the line. The marker is on **stdout** because it is the command's output (§1), exits `0`, and appears only when the report has no diagnostics ([§FS-check.2.1](FS-check.md#21-report-format)). It is not emitted in `--format=json`, where stdout remains diagnostics-only.

## 3. Message text

The shape is structural; the text is human-readable. Style rules apply to every shape:

- **Lowercase first letter.** `unknown reference <ID>` — not `Unknown reference <ID>`.
- **No terminal period.** Messages do not end in `.` or `!`.
- **No ANSI colors by default.** A future `--color=auto` may add them ([§G-no-silent-breakage](../goals/goals.md#g-no-silent-breakage-changes-ship-through-a-deprecation-path) applies); plain bytes are the contract.
- **Stable phrasing.** The exact text of each message is part of the user-visible output covered by [§G-no-silent-breakage.1](../goals/goals.md#1-what-counts-as-user-visible): changing it goes through a deprecation path. Tools grep on it.
- **Quoted user input** appears in double quotes when the input could be confused with surrounding prose: `"<original title>"`, not `<original title>`.

Severity (`error` vs `warning`) is **implicit in the rule**, not in the line. [§FS-check.3](FS-check.md#3-errors-detected) is errors; [§FS-check.4](FS-check.md#4-warnings) is warnings; both render identically as located findings. Consumers that need machine-distinguishable severity use `--format=json` (§5).

## 4. Determinism

Two runs of the same subcommand on the same input must produce byte-identical stdout *and* stderr. This rules out:

- Wall-clock timestamps in messages.
- Process IDs, hostnames, or absolute paths outside the configured root.
- Non-deterministic ordering. Findings sort by `(path, line)` lexicographically; multi-site findings anchor at the lexicographically-first site ([§FS-check.2.1](FS-check.md#21-report-format)).

A message that would otherwise be non-deterministic (e.g. the order of duplicate-declaration sites) is sorted before printing.

## 5. JSON format

The subcommands with a machine-readable result or finding surface accept `--format=json`: `check`, `show`, `list`, `refs`, `cover`, and `id` ([§G-friendliness-first.1](../goals/goals.md#1-hard-requirements), [§FS-cli.3](FS-cli.md#3-cross-subcommand-flags)). Operational commands whose output is human text or generated files (`fmt`, `init`, `config`, `agent-setup-instructions`, `completions`) do not accept `--format` unless their own spec adds a JSON surface later. JSON follows the same stream split as the text form (§1):

- **On stdout — the command's output.** `gnd check --format=json` emits its findings as NDJSON, one object per line, in the binding-level shape from [§FS-distribution.3.0](FS-distribution.md#30-language-neutral-data-shapes) (`{ severity, path, line, code, message, sites }`); `severity` carries the `error`/`warning` distinction the text form leaves implicit (§3), and `sites` is `null` for an ordinary single-site finding, a `[{ path, line }]` list naming every site for a multi-site finding (a duplicate declaration, [§FS-check.3.3](FS-check.md#33-duplicate-declaration)). A clean JSON check emits no `success` object. Query subcommands emit their result on stdout too: one JSON object for a single-result command (`gnd show --format=json` — [§FS-show](FS-show.md#fs-show-gnd-reads-a-single-declaration-body-by-id); `gnd id --format=json` — [§FS-id](FS-id.md#fs-id-gnd-proposes-ids-for-new-declarations)), NDJSON — one object per row — for a list command (`gnd list` per declaration, `gnd refs` per citation, `gnd cover` per scanned file).
- **On stderr — what is not output.** A *failed query* (`gnd show`'s `ID not found` / `ambiguous` / `broken stub` / `section not found` / `invalid ID`, exit `1`) emits its one diagnostic object on stderr in the same `{ severity, path, line, code, message, sites }` shape, with `path` and `line` `null` — there is no single site, and there is no result, so nothing goes to stdout. For an ambiguous ID, `sites` carries the `[{ path, line }]` list of the competing declarations ([§FS-show.2.2.1](FS-show.md#221-ambiguous-id)); for the other query failures it is `null`. A *launch-time* CLI-level error (§2.2 — bad flag, unknown kind, unreadable config or path; exit `2`) stays as the `error: <message>` text line on stderr regardless of `--format` — it is a launch failure, not data. The empty-scan `warning:` ([§FS-check.2.2](FS-check.md#22-empty-scan)) and a per-file read failure collected mid-walk (a `line`-less diagnostic in `gnd check`'s report) are likewise on stderr in both forms — about the run, not findings about the graph.

So `gnd check --format=json | jq …`, `gnd show <ID> --format=json | jq …`, `gnd list --format=json | jq …` all work with no stream juggling, and `gnd show <missing> --format=json | jq …` does not choke because the diagnostic is on stderr where the pipe does not see it.

The text-form messages defined above remain the default. JSON is opt-in.

## 6. The `gnd init` transcript

`gnd init` ([§FS-init.2.2](FS-init.md#22-stdout--stderr)) writes status lines to **stderr** — `wrote AGENTS.md`, `appended .agents/gnd.toml`, `exists docs/...`, etc. — followed by the `next:` block. These are **not** the command's output: `init`'s output is the scaffold it wrote to disk; the transcript is progress, and nobody pipes `gnd init`. They use an init-specific shape (`<verb> <path>`) and are scoped to that command. This is the one carve-out from §1 — every other subcommand puts its output on stdout. In particular `gnd fmt --write` ([§FS-fmt.3](FS-fmt.md#3-outputs)) does **not** use this shape: its `rewrote N reference(s):` report is `fmt`'s output and goes to stdout, the same stream as its `--check` dry-run report. A subcommand with no output and no transcript stays silent and lets the exit code carry the verdict.

## 7. What this rules out

- Severity prefixes (`error:`, `warning:`) on located findings — see §3.
- Multi-line messages. A finding that wants to elaborate uses `--format=json` and a `code` plus a documentation link, not a wrapped paragraph.
- Interactive prompts, progress bars, spinners, or any byte that depends on terminal capabilities. Per [§FS-non-goals.10](FS-non-goals.md#10-interactive-mode), every subcommand is non-interactive.
- Localization. Messages are English; translation is downstream's problem.

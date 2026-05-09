# FS-errors: gnd emits messages in one of three fixed shapes

This spec defines the style every `gnd` subcommand uses when it speaks to a user or to a downstream tool. It is cross-cutting: FS-check, FS-show, FS-fmt, FS-init, and FS-name all conform to it. Serves G-friendliness-first.1 ("errors point at the line", "no surprises") and G-no-silent-breakage.1 (the message shapes are user-visible output).

The shapes are **frozen** by the same logic as FS-non-goals.9: two correctly-configured installs must agree on what they print. A subcommand that needs to say something new picks one of the shapes below; it does not invent a fourth.

## 1. Streams

- **stderr** carries every message defined in this spec — findings, CLI errors, deprecation warnings, status lines.
- **stdout** is reserved for the command's *result* (e.g. the body printed by `gnd show`). On success with no result to print, stdout is empty per G-friendliness-first.1.6.
- A subcommand never mixes a finding into stdout, so `gnd show <ID> | …` and `gnd check … 2>/dev/null` both work without surprise.

## 2. The three shapes

### 2.1 Located finding

A diagnostic that points at a specific source site:

```
<path>:<line>: <message>
```

- `<path>` is relative to the config root when a `gnd.toml` was discovered (FS-config.3.6), otherwise to the path passed on the command line.
- `<line>` is 1-indexed.
- `<message>` is a single line — no embedded newlines, no terminal period.
- The `<path>:<line>:` prefix is mandatory: editors and agents jump on this exact shape.

Used by every per-source diagnostic in `gnd check` (FS-check.2.1) and surfaced unchanged by the IDE plugins (FS-ide-plugins.1.1).

### 2.2 CLI-level error

A diagnostic that has no source location — unknown subcommand, malformed flag, invalid `gnd.toml` schema, scan I/O failure:

```
error: <message>
```

- The literal `error: ` prefix is what distinguishes a CLI-level failure from a located finding. CI scripts grep for it.
- No `<path>:<line>:` is attached, even if a config file is the cause — the message text names the file when relevant (e.g. `error: invalid gnd.toml: ...`).

Used by FS-check.2.1.1, FS-name.3 (empty slug), FS-name.5 (collision), FS-config.6 (config validation), and any subcommand reporting a launch-time failure.

### 2.3 Bare query result

When a *query* (a subcommand whose job is to return one body) finds something other than exactly one body — `gnd show` on a missing ID, a missing section, or an ambiguous ID:

```
<message>
```

- No prefix at all. There is no single site to point at, and the command never returns a body in this case, so the message *is* the result the caller reads.
- Ambiguity messages list every site in lexicographic `path:line` order (FS-show.2.2.1).

Used only by query commands (currently `gnd show`). `check` does not use this shape — it always has a site or is a CLI-level error.

## 3. Message text

The shape is structural; the text is human-readable. Style rules apply to every shape:

- **Lowercase first letter.** `dangling citation: FS-foo` — not `Dangling citation`.
- **No terminal period.** Messages do not end in `.` or `!`.
- **No ANSI colors by default.** A future `--color=auto` may add them (G-no-silent-breakage applies); plain bytes are the contract.
- **Stable phrasing.** The exact text of each message is part of the user-visible output covered by G-no-silent-breakage.1: changing it goes through a deprecation path. Tools grep on it.
- **Quoted user input** appears in double quotes when the input could be confused with surrounding prose: `"<original title>"`, not `<original title>`.

Severity (`error` vs `warning`) is **implicit in the rule**, not in the line. FS-check.3 is errors; FS-check.4 is warnings; both render identically as located findings. Consumers that need machine-distinguishable severity use `--format=json` (§5).

## 4. Determinism

Two runs of the same subcommand on the same input must produce byte-identical stderr. This rules out:

- Wall-clock timestamps in messages.
- Process IDs, hostnames, or absolute paths outside the configured root.
- Non-deterministic ordering. Findings sort by `(path, line)` lexicographically; multi-site findings anchor at the lexicographically-first site (FS-check.2.1).

A message that would otherwise be non-deterministic (e.g. the order of duplicate-declaration sites) is sorted before printing.

## 5. JSON format

Every subcommand that emits messages accepts `--format=json` (G-friendliness-first.1.2). The schema is the binding-level shape from FS-distribution.2 (`{ severity, path, line, code, message }`); the wire form is one JSON object per line (NDJSON) on stderr. Located findings, CLI-level errors, and bare query results all serialize into the same shape — `path` and `line` are `null` for the latter two.

The text-form messages defined above remain the default. JSON is opt-in.

## 6. Non-status-line output

`gnd init` (FS-init.2.2) writes status lines to stderr — `wrote agents.md`, `appended .agents/gnd.toml`, etc. These are **not** diagnostics; they are a transcript of side effects. They use a fourth, init-specific shape (`<verb> <path>`) and are scoped to that command. Other subcommands do not adopt this shape — when they need to report progress, they stay silent (G-friendliness-first.1.6) and let the exit code carry the verdict.

## 7. What this rules out

- Severity prefixes (`error:`, `warning:`) on located findings — see §3.
- Multi-line messages. A finding that wants to elaborate uses `--format=json` and a `code` plus a documentation link, not a wrapped paragraph.
- Interactive prompts, progress bars, spinners, or any byte that depends on terminal capabilities. Per FS-non-goals.10, every subcommand is non-interactive.
- Localization. Messages are English; translation is downstream's problem.

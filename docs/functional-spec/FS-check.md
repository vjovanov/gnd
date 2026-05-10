# FS-check: gnd validates every reference in a repo

The `check` command walks a repo and reports every violation of the gnd reference scheme. It is the default subcommand: `gnd <path>` and `gnd check <path>` are equivalent. Serves §G-no-dangling-refs and §G-fast-feedback.

## 1. Inputs

- Optional path argument; defaults to the current directory. May be a directory or a single file (`gnd check src/scanner.rs` scopes the scan to one file but still discovers `.agents/gnd.toml` by walking up — §FS-config.1).
- The walked tree may contain markdown (`.md`) and source files (Rust, Go, Java, TS, Python, etc.).
- Optional `.agents/gnd.toml` configuring marker, trigger, kinds, and skip lists per §G-configurable (§FS-config).
- `--watch` is reserved for the planned resident checker (§6) and is not accepted by the current CLI.
- `--require-grounding` — turn the grounding check (§3.6) on for this run regardless of `[reference] require_grounding` in `.agents/gnd.toml` (§FS-config.3.1). It only ever *adds* the check; it cannot switch off a config that already sets it.
- `--format text|json` — output shape, per §FS-errors.5. The global flags `--version` and `--help` are handled before any scan (§FS-cli).

## 1.1 Recognized citations

Per §DF-reference-marker, a citation is the marker followed by an ID, e.g. `§FS-check.3.1`. The default marker is `§`; configurable via `gnd.toml`.

In default mode, bare ID tokens are also recognized as citations for backward compatibility. In `[reference] strict = true` mode, only marker-prefixed citations are recognized — bare tokens are treated as plain text and do not trigger dangling-ref errors. New repos should adopt strict mode after running `gnd fmt --marker` (§FS-fmt) to convert existing bare citations.

Citations may appear in markdown prose, in source-file line/block comments, and in language doc-comments (Javadoc, JSDoc, Rustdoc, Python docstrings, etc.) — see §AS-scanner.2.3 and §AS-scanner.4 for the exact contexts. In source files, a **bare** ID-shaped token whose start column falls inside a string literal is not treated as a citation (the same deterministic quote-tracking rule `gnd fmt` uses — §FS-fmt.2.3.1, §AS-scanner.2.3), so an ID-shaped substring inside a runtime string does not raise a false dangling-ref. A marker-prefixed citation is recognized everywhere, string or not — the marker is the signal of intent. Markdown files have no string literals and the carve-out does not apply there. `E2E` citations (`§E2E-<name>`) resolve against case directories under `e2e/cases/` per §AS-scanner.6.

## 2. Outputs

A report on stderr, plus an exit code:

- `0` — no errors. Warnings allowed (they do not affect the exit code).
- `1` — at least one error.
- `2` — scan failure (I/O, malformed file, invalid `.agents/gnd.toml`).

An invalid `.agents/gnd.toml` aborts before any file is read (§FS-config.4.3): exit `2`, a single `error:` line, no findings. A per-file failure encountered *during* the walk (a file that cannot be read or decoded) is different: the offending file is reported as `error: <path>: <reason>` (the CLI-level shape, §FS-errors.2.2 — the file has no line to point at), the walk continues over the remaining files, every finding collected from the readable files is still printed in the normal `<path>:<line>:` form, and the run exits `2` because the view of the tree was incomplete. A `2` therefore always means "do not trust this report as complete"; the printed findings are still real.

### 2.1 Report format

Findings are written to stderr, one per line, in the form:

```
<path>:<line>: <message>
```

`<path>` is relative to the config root (§FS-config.3.6) when a `gnd.toml` was discovered, otherwise relative to the path passed on the command line. `<line>` is 1-indexed. The `<path>:<line>:` prefix is mandatory on every finding so editors and agents can jump unmodified — this is the contract from §G-friendliness-first.1.

Severity is implicit. Per-finding lines carry no `error:`/`warning:` prefix because the severity of a rule is fixed (§FS-check.3 vs §4) and the message text is what humans read. Consumers that need machine-distinguishable severity use `--format=json`.

When a finding inherently spans multiple sites (e.g., duplicate declarations, §FS-check.3.3), the message is anchored at the lexicographically-first site (sort by `path`, then `line`) and the other sites are listed parenthetically inside the message.

Stdout is always empty for `check`, including `--format=json`; diagnostics are emitted on stderr per §FS-errors.5. Stderr is empty when there are zero errors and zero warnings, satisfying §G-friendliness-first.1 ("zero noise on success"). There is no summary footer — the exit code is the machine-readable verdict, the per-finding lines are the human-readable detail.

#### 2.1.1 CLI-level errors

Errors that have no source location — unknown subcommand, malformed flag, invalid `gnd.toml` schema (when the config itself parses but a value is wrong) — are emitted on stderr as:

```
error: <message>
```

These never carry a `<path>:<line>:` prefix. The `error:` prefix is what distinguishes them from per-finding lines, and CI scripts can grep for the leading `error:` to detect launch-time failures. The same shape with a `warning:` prefix is the CLI-level *warning* form — used by §2.2; like other warnings it does not affect the exit code.

### 2.2 Empty scan

A walk that read **no scannable files** at all, and turned up no findings (no errors, no warnings — including the agent-entrypoint check of §3.5, which still runs and still reports even when nothing is scanned), is almost always a misconfigured scope rather than a clean repo. Rather than print nothing and exit `0` — which reads as "all clear" — `check` emits one CLI-level `warning:` line to stderr:

- when the scope is the repo root (no path argument, or `gnd check .`) and `[scan] include` is set: the message names the `include` list and points at `.agents/gnd.toml` / `gnd init`, since the usual cause is a project whose sources live outside the default `docs/`, `e2e/`, `src/`;
- when an explicit path was given: the message names that path and the recognized extensions, since the usual cause is pointing `gnd` at a tree with no `.md`/source files.

This is a warning, not an error: the exit code stays `0` (a genuinely empty tree is not a failure), `--format=json` emits the warning as one diagnostic JSON object on stderr, and a repo that *does* have a stale `agents.md` block or any other finding gets that finding and **no** empty-scan notice. This is the friendliness-first counterpart to "zero noise on success" (§G-friendliness-first.1): the run that scanned nothing is the one case where silence is the wrong answer.

## 3. Errors detected

Each of the following is an error and contributes to a non-zero exit code.

### 3.1 Dangling citation

A recognized citation (per §1.1) for which no declaration is found.

### 3.2 Missing section

A citation with a section suffix (`§FS-<user-login>.3.1`) where the declaration exists but the requested section heading does not.

### 3.3 Duplicate declaration

The same `<KIND>-<NNN>-<slug>` declared as a heading in more than one file. Reported per §2.1: one error anchored at the lexicographically-first site, with the remaining sites listed in the message.

### 3.4 Broken inline-spec stub

A `docs/` file whose H1 has the stub shape `# <ID>: [<text>](<path>)` where either the path does not exist, or the file at that path contains no inline declaration of the same ID.

### 3.5 Invalid agent entrypoint init block

If `<path>/agents.md` exists, `check` verifies the versioned `gnd init` block defined by §FS-init.2.3. It also verifies known companion agent entrypoints whenever they exist and are not symlinks to `agents.md`; for example, existing standalone `AGENTS.md`, `AGENTS.override.md`, `CLAUDE.md`, `.claude/CLAUDE.md`, `GEMINI.md`, and `.github/copilot-instructions.md` files must carry the same managed block, while `CLAUDE.md -> agents.md` is already covered by the canonical file. If `agents.md` does not exist, companion agent files are treated as project-owned instructions and are not validated by `gnd check`; this keeps config-only adoption from modifying or policing an existing agent setup. A missing block, malformed begin/end marker pair, older block version, or newer unsupported block version is an error in scaffolded-entrypoint mode. This lets CI catch repos whose managed agent entry points were initialized and later drifted or need to be refreshed with `gnd init`.

### 3.6 Ungrounded source file *(opt-in)*

Off by default. When `[reference] require_grounding = true` is set in `.agents/gnd.toml` (§FS-config.3.1) — or `gnd check --require-grounding` is passed (§1) — every scanned **source file** (a file the walk reads whose extension is not `.md`, §AS-scanner.1) must be *grounded*: it must contain at least one recognized citation (§1.1) whose ID resolves to a declaration, **or** it must itself declare an ID inline (a spec home is grounded in the spec it *is*, §AS-scanner.4). A source file that is neither is an error, anchored at line 1:

```
src/foo.rs:1: ungrounded source file: no § citation to a declared ID
```

The marker in the message is the configured one (§FS-config.3.1). A file whose only citation is dangling (§3.1) is *not* grounded — it gets both findings; fixing the citation clears both. Markdown files are never subject to this rule (they are documents, not implementation); use the unused-declaration warning (§4.1) and dangling/section errors for those.

This is a pure function of `(tree, config)` like every other `check` rule (§FS-non-goals.13): it reads no git history (§FS-non-goals.6) and parses no code (§FS-non-goals.3) — "source file" is decided by extension, "grounded" by the citations the scanner already collected. It is the floor of the grounding discipline; `gnd cover` exposes the citation graph (§FS-cover), and the diff-aware co-change gate is tracked under §RM-cochange-gate. Decided in §DF-require-grounding.

## 4. Warnings

### 4.1 Unused declaration

An ID that is declared but never cited. Reported as a warning, not an error — newly declared IDs may not yet have citations. Warnings never affect the exit code (§2).

`E2E` declarations (§AS-scanner.6) are exempt: an end-to-end case is exercised by being run, not by being cited, so a `§E2E-<name>` that nothing references is not a warning. Every other kind is subject to this rule. `gnd list --unused` (§FS-list) still lists uncited `E2E` declarations — the suppression is of the *check warning*, not of the catalog query.

## 5. What gnd does not check

See §FS-non-goals — in particular §FS-non-goals.1 (markdown links / URLs), §FS-non-goals.2 (spelling/grammar), and the convention that ID numbers are stable handles, not ordinal positions.

## 6. Watch mode (`--watch`)

Status: planned — implementation tracked under §RM-watch.

When implemented, `gnd check --watch [<path>]` will run the check once, then stay resident and re-run it whenever a file under the scanned tree (or `.agents/gnd.toml`) changes. It is the editor-less counterpart to the optional LSP server (§FS-lsp): the LSP integrates `gnd` into an editor's diagnostics; `--watch` is the plain-terminal "every save" loop that §G-fast-feedback exists for. Until §RM-watch lands, `gnd check --watch` is a CLI error (`error: unknown flag \`--watch\``, exit 2).

- **Change detection.** Filesystem notifications where the OS provides them; a debounce window coalesces a burst of writes into one re-check. No polling loop is required, and there is no configurable interval — the watcher reacts, it does not sample.
- **Each run is a plain `gnd check`.** Output and exit-status semantics of an individual run are exactly §2/§2.1 on the tree's state at that moment — byte-identical to what a non-`--watch` invocation would print (§FS-errors.4). Before each run the previous run's output is cleared so the terminal always shows the current report; with `--format=json` each run emits the same diagnostic NDJSON as non-watch mode, scoped to that run.
- **Lifecycle.** The process runs until interrupted (Ctrl-C / SIGINT). On interrupt it exits with the exit code of the most recently completed run (`0`/`1`/`2`), so `gnd check --watch &` followed by a later signal is still a meaningful CI-ish probe. There is no TUI, no key bindings, no prompt — it is non-interactive per §FS-non-goals.10, just a re-printing checker. No network I/O (§FS-non-goals.11); the only files touched are the ones the walk already reads.
- **Scope.** `--watch` will be a `check` flag (and `gnd --watch [<path>]` will be shorthand for `gnd check --watch [<path>]`, §FS-cli). Other subcommands will not take it; a one-shot `gnd fmt` or `gnd show` has nothing to keep watching.

# FS-001-check: gnd validates every reference in a repo

The `check` command walks a repo and reports every violation of the gnd reference scheme. It is the default subcommand: `gnd <path>` and `gnd check <path>` are equivalent. Serves G-001-no-dangling-refs and G-002-fast-feedback.

## 1. Inputs

- Optional path argument; defaults to the current directory.
- The walked tree may contain markdown (`.md`) and source files (Rust, Go, Java, TS, Python, etc.).
- Optional `gnd.toml` at the root configuring marker, trigger, kinds, and skip lists per G-006-configurable.

## 1.1 Recognized citations

Per DF-001-reference-marker, a citation is the marker followed by an ID, e.g. `§FS-001-check.3.1`. The default marker is `§`; configurable via `gnd.toml`.

In default mode, bare ID tokens are also recognized as citations for backward compatibility. In `[reference] strict = true` mode, only marker-prefixed citations are recognized — bare tokens are treated as plain text and do not trigger dangling-ref errors. New repos should adopt strict mode after running `gnd fmt --marker` (FS-005-fmt) to convert existing bare citations.

Citations may appear in markdown prose, in source-file line/block comments, and in language doc-comments (Javadoc, JSDoc, Rustdoc, Python docstrings, etc.) — see AS-001-scanner.2.3 and AS-001-scanner.4 for the exact contexts.

## 2. Outputs

A report on stderr, plus an exit code:

- `0` — no errors. Warnings allowed (they do not affect the exit code).
- `1` — at least one error.
- `2` — scan failure (I/O, malformed file, invalid `gnd.toml`).

### 2.1 Report format

Findings are written to stderr, one per line, in the form:

```
<path>:<line>: <message>
```

`<path>` is relative to the config root (FS-006-config.3.6) when a `gnd.toml` was discovered, otherwise relative to the path passed on the command line. `<line>` is 1-indexed. The `<path>:<line>:` prefix is mandatory on every finding so editors and agents can jump unmodified — this is the contract from G-005-friendliness-first.1.

Severity is implicit. Per-finding lines carry no `error:`/`warning:` prefix because the severity of a rule is fixed (FS-001-check.3 vs §4) and the message text is what humans read. Consumers that need machine-distinguishable severity use `--format=json`.

When a finding inherently spans multiple sites (e.g., duplicate declarations, FS-001-check.3.3), the message is anchored at the lexicographically-first site (sort by `path`, then `line`) and the other sites are listed parenthetically inside the message.

Stdout is always empty for `check`. Stderr is empty when there are zero errors and zero warnings, satisfying G-005-friendliness-first.1.6 ("zero noise on success"). There is no summary footer — the exit code is the machine-readable verdict, the per-finding lines are the human-readable detail.

#### 2.1.1 CLI-level errors

Errors that have no source location — unknown subcommand, malformed flag, invalid `gnd.toml` schema (when the config itself parses but a value is wrong) — are emitted on stderr as:

```
error: <message>
```

These never carry a `<path>:<line>:` prefix. The `error:` prefix is what distinguishes them from per-finding lines, and CI scripts can grep for the leading `error:` to detect launch-time failures.

## 3. Errors detected

Each of the following is an error and contributes to a non-zero exit code.

### 3.1 Dangling citation

A recognized citation (per §1.1) for which no declaration is found.

### 3.2 Missing section

A citation with a section suffix (`FS-042-user-login.3.1`) where the declaration exists but the requested section heading does not.

### 3.3 Duplicate declaration

The same `<KIND>-<NNN>-<slug>` declared as a heading in more than one file. Reported per §2.1: one error anchored at the lexicographically-first site, with the remaining sites listed in the message.

### 3.4 Broken inline-spec stub

A `docs/` file with a `Defined-in: <path>` line where either the path does not exist, or the file at that path contains no inline declaration of the same ID.

## 4. Warnings

### 4.1 Unused declaration

An ID that is declared but never cited. Reported as a warning, not an error — newly declared IDs may not yet have citations. Warnings never affect the exit code (§2).

## 5. What gnd does not check

See FS-007-non-goals — in particular FS-007-non-goals.1 (markdown links / URLs), FS-007-non-goals.2 (spelling/grammar), and the convention that ID numbers are stable handles, not ordinal positions.

# FS-005-fmt: gnd normalizes references in bulk

The `fmt` subcommand rewrites a tree to canonical form: trigger sequences become markers, and (optionally) bare citations become marker-prefixed. It is the batch counterpart to the live IDE transform from FS-003-ide-plugins.1.5. Implements DF-001-reference-marker.

## 1. Inputs

```
gnd fmt [<path>] [--check] [--marker] [--write]
```

- `<path>` — directory or file. Defaults to the current directory.
- `--check` — explicit form of the default behavior: report what would change; exit non-zero if any change would be made; do not write. Provided as a flag for CI clarity (a script that says `gnd fmt --check` is unambiguous about intent).
- `--marker` — also rewrite bare citations (`FS-001-check`) to marker-prefixed (`§FS-001-check`). Off by default to preserve existing repos that have not opted in.
- `--write` — write the transformed contents back to disk. Exit 0 even when changes were made (the changes were the requested operation, not a failure).

`--check` and `--write` are mutually exclusive. Without either, the default is `--check`.

## 2. Behavior

### 2.1 Trigger-to-marker

Wherever the configured trigger (default `$$`) is immediately followed by `<KIND>-<digit>`, replace the trigger with the configured marker (default `§`). Idempotent: running `gnd fmt` twice produces no further change.

### 2.2 Bare-to-marker (with `--marker`)

When `--marker` is given, every recognized bare citation is also rewritten to its marker-prefixed form. This is how a repo migrates from default mode to `[reference] strict = true`: run `gnd fmt --marker --write` once, then flip the strict flag.

### 2.3 What is never rewritten

- Declaration headings (the line that names the ID). The marker is for *citations*, not declarations.
- Citations inside string literals on a source line (where rewriting would change runtime behavior).
- Files outside the configured scan set.

#### 2.3.1 String-literal exclusion rule

The string-literal exclusion is deterministic, not heuristic. For every candidate transform site on a source-file line:

1. Walk the line left-to-right from column 0 up to the candidate's start column.
2. Track an open-quote state per `'`, `"`, and `` ` ``. Toggling rules: an unescaped (no immediately preceding `\`) quote of a given kind toggles its state, but only when no other kind is currently open.
3. If any quote state is open at the candidate's start column, the candidate is inside a string literal and is **not** rewritten.

Markdown files (`.md`) are not subject to this rule — they have no string literals. The rule applies only to files matched by the `extensions` list excluding `md`.

This gives two correctly-configured installs identical output on identical input (FS-007-non-goals.13).

## 3. Outputs

- `0` — no changes needed.
- `1` — changes made (or, with `--check`, would be made).
- `2` — I/O error.

With `--check`, the report lists `path:line: trigger → marker` for every transformed citation. With `--write`, the report is a summary count; the file system carries the change.

## 4. Why this exists

Three reasons:

1. **Onboarding.** Adopting the marker scheme on an existing repo requires rewriting hundreds of citations. `gnd fmt --marker --write` does it in seconds.
2. **CI safety net.** A contributor who bypasses the IDE plugin (e.g., edits via the GitHub web UI) leaves bare triggers in place. `gnd fmt --check` in CI catches it.
3. **Pre-commit hook.** Run on staged files; transform locally before commit. Keeps the canonical form in version control.

## 5. Configurability

Marker, trigger, and the recognized `KIND` set are read from `gnd.toml` per G-006-configurable. The defaults are `§` and `$$` as decided in DF-001-reference-marker.

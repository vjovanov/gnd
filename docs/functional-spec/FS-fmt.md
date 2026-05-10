# FS-fmt: gnd normalizes references in bulk

The `fmt` subcommand rewrites a tree to canonical form: trigger sequences become markers, and (optionally) bare citations become marker-prefixed. It is the batch counterpart to the optional LSP server's live trigger transform (§FS-lsp.1.4) and the always-available path: every install of `gnd` ships `fmt`, while the LSP server is opt-in. Implements §DF-reference-marker.

## 1. Inputs

```
gnd fmt [<path>] [--check] [--marker] [--md-links] [--write]
```

- `<path>` — directory or file. Defaults to the current directory.
- `--check` — explicit form of the default behavior: report what would change; exit non-zero if any change would be made; do not write. Provided as a flag for CI clarity (a script that says `gnd fmt --check` is unambiguous about intent).
- `--marker` — also rewrite bare citations (`FS-check`) to marker-prefixed (`§FS-check`). Off by default to preserve existing repos that have not opted in.
- `--md-links` — in `.md` files only, also wrap each marker-prefixed citation in a clickable Markdown link to the declaration body. Per §6. Off by default; opt-in because the link target is path-relative and not every repo wants the rewrite. Implements §DF-md-link-emission.
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
- Citations inside Markdown inline code spans (where rewriting would change a literal command, path, or example).
- Files outside the configured scan set.

#### 2.3.1 String-literal exclusion rule

The string-literal exclusion is deterministic, not heuristic. For every candidate transform site on a source-file line:

1. Walk the line left-to-right from column 0 up to the candidate's start column.
2. Track an open-quote state per `'`, `"`, and `` ` ``. Toggling rules: an unescaped (no immediately preceding `\`) quote of a given kind toggles its state, but only when no other kind is currently open.
3. If any quote state is open at the candidate's start column, the candidate is inside a string literal and is **not** rewritten.

Markdown files (`.md`) are not subject to this rule — they have no string literals. The rule applies only to files matched by the `extensions` list excluding `md`.

This gives two correctly-configured installs identical output on identical input (§FS-non-goals.13).

## 3. Outputs

- `0` — no changes needed, **or** `--write` succeeded (regardless of whether changes were made — they were the requested operation, not a failure).
- `1` — `--check` found at least one citation that would be rewritten. Never returned by `--write`.
- `2` — I/O error.

With `--check` (or no flag, the implicit dry run), the report lists one `path:line: <kind>` line per changed line, where `<kind>` names the rewrite: `trigger → marker` (a typed trigger sequence rewritten to the marker, §2.1), `bare → marker` (a bare citation marked, §2.2, with `--marker`), or `markdown link` (a citation wrapped or re-derived, §6, with `--md-links`). With `--write`, the report names what changed on disk: a `rewrote N reference(s):` summary line, then one `  <path> (<count>)` line per file touched, in lexicographic path order (an empty change set prints `rewrote 0 references` with no list). The file system carries the actual change; the summary is so a reviewer can see which files to re-inspect without diffing the whole tree.

## 4. Why this exists

Three reasons:

1. **Onboarding.** Adopting the marker scheme on an existing repo requires rewriting hundreds of citations. `gnd fmt --marker --write` does it in seconds.
2. **CI safety net.** A contributor who bypasses the IDE plugin (e.g., edits via the GitHub web UI) leaves bare triggers in place. `gnd fmt --check` in CI catches it.
3. **Pre-commit hook.** Run on staged files; transform locally before commit. Keeps the canonical form in version control.

## 5. Configurability

Marker, trigger, and the recognized `KIND` set are read from `gnd.toml` per §G-configurable. The defaults are `§` and `$$` as decided in §DF-reference-marker.

## 6. Markdown link emission (with `--md-links`)

A free convenience layer on top of the ID system: in rendered Markdown (GitHub, MkDocs, an IDE preview), wrap each citation in a clickable link to the declaration body — without giving up any of the polyglot, refactor-safe properties IDs already provide. Decided in §DF-md-link-emission.

### 6.1 Scope

`--md-links` runs **only on files with the `.md` extension** in the configured scan set. Source files are never touched: their host languages do not render Markdown links, and rewriting a comment in `src/bus.rs` to inject `[…](…)` syntax is at best noise and at worst a parse error. The polyglot citation grammar (`§G-polyglot-citation`) is the universal form; link emission is the rendered-Markdown view of it.

### 6.2 Form

Wrap the citation. A bare or marker-prefixed citation (illustrated as `§FS-<foo>.3.1`) becomes:

```
[§FS-<foo>.3.1](<relative-path>#<anchor>)
```

- `<relative-path>` — path from the file containing the citation to the file containing the declaration, in POSIX form (`../functional-spec/FS-<foo>.md`). When the declaration's home is in source code (a stub points at `src/foo.rs`), the link targets the source file directly with no anchor — the host renderer will not jump inside a doc-comment, but the link still leads to the right file.
- `#<anchor>` — section anchor when the citation has a `.<section>` part. The anchor is the heading text slugified per the configured renderer profile (§6.7) — for the default `github` profile, `### 6.2 Form` produces `#62-form`. The full strategy and profile list is decided in §DF-md-link-anchor-strategy. When the citation has no section, the `#…` part is omitted. When the active profile is `none`, the anchor is omitted regardless.

The citation text inside the brackets is preserved verbatim, including the marker. A reader scanning the rendered Markdown sees the citation exactly as before; only now it is clickable.

### 6.3 Idempotency and re-derive

Per §DF-md-link-anchor-strategy.2.2, every `gnd fmt --md-links` pass recomputes the canonical URL inside each existing wrap and rewrites if it differs. This makes `fmt` a normalizer, not a preserver: a heading rename or a file move that invalidates a wrap produces a one-line `fmt` diff on the next pass, instead of a silently-broken link.

Idempotency holds: a second run with no intervening edits is a no-op, because the URL on disk is now equal to the canonical URL.

Detection of an existing wrap, for both the rewrite and the no-double-wrap rules: the citation's immediately-preceding character is `[` and its immediately-following text begins `](`. When this matches, the wrapper computes the canonical URL and replaces the existing one if different. When it does not match, the citation is wrapped fresh.

### 6.4 What is never wrapped

In addition to the never-rewrite rules in §2.3:

- Citations inside fenced code blocks (the same skip used by §2.3 / `gnd fmt`'s existing trigger pass). Code samples often illustrate citations as plain tokens; rewriting them changes what the docs claim.
- Citations inside inline code spans (between backticks).
- Citations on a declaration heading line. The marker is for citations, not declarations (§2.3).
- Citations whose declaration cannot be located by the scanner. A dangling citation is a `gnd check` error; `fmt` does not paper over it by emitting a link to a nonexistent file. Report the unwrapped citation; let `check` flag the underlying problem.

### 6.5 Interaction with `--marker`

`--md-links` operates on marker-prefixed citations. When run together with `--marker`, the marker pass runs first (bare → marker), then the link pass wraps the now-marker-prefixed citations. When run without `--marker`, bare citations are left bare and unwrapped — wrapping only the marked ones gives a consistent, predictable output instead of two mixed forms.

### 6.6 Why `--md-links` is opt-in

Three reasons, all about preserving §G-no-silent-breakage:

1. The path inside the link is computed from the current location of citation and declaration. Repos that move files frequently (without running `gnd fmt`) would see noisy diffs as paths rebase.
2. Some projects render their Markdown through tools that produce different anchor slugs than `#3-1` (e.g., Pandoc). For those projects, the configurable anchor format (§6.7) is the right answer; until then, opting in is the conservative default.
3. Citations remain the source of truth. Wrapping them in links is a presentation-layer convenience; making it the default would imply that the rendered Markdown view is canonical, which it is not.

### 6.7 Configurability

```toml
[fmt.md_links]
enabled       = false      # default; --md-links overrides per-invocation
anchor_format = "github"   # default; named renderer profile per §DF-md-link-anchor-strategy.2.3
```

`anchor_format` accepts one of the named profiles defined in §DF-md-link-anchor-strategy.2.3:

- `github` (default) — GitHub's slugger; covers the most common host.
- `gitlab` — GitLab's slugger.
- `mkdocs` — MkDocs / Python-Markdown TOC extension's slugger.
- `pandoc` — Pandoc's `auto_identifiers` algorithm.
- `none` — emit no anchor; produce a file-level link with no fragment.

When `enabled = true`, the link pass runs on every `gnd fmt --write` invocation without requiring `--md-links`. This is for repos that have committed to the convention.

### 6.8 Measurable

E2E fixtures cover: wrap-on-first-run, no-op on second-run (idempotency), re-derive on heading rename (a wrap pointing at the old slug is rewritten to the new one in a single `fmt` pass), re-derive on file move, correct relative path across `docs/` subdirectories, source-file declaration link with no anchor, `anchor_format = "none"` produces file-only links, each named renderer profile (`github`, `gitlab`, `mkdocs`, `pandoc`) produces its expected slug for a curated heading set, fenced-block exemption, dangling-citation skipped, declaration-line skipped, `--md-links` without `--marker` on a tree containing both forms.

# FS-refs: gnd lists every citation of an ID

The `refs` subcommand answers the reverse of `gnd show`: not "what does this ID say?" but "who points at it?". An agent about to change a declaration — or delete one — needs to know what leans on it; `gnd refs FS-check` is that lookup, scheme-aware in the ways a `grep` cannot be. Serves §G-friendliness-first and the agent-grounding loop in the raison-detre (an agent verifying a change reads the cited bodies *and* the back-references).

## 1. Inputs

```
gnd refs <ID> [<path>] [--section <s>] [--format text|json]
```

- `<ID>` — the ID to look up, without the marker. May carry an inline section (`FS-check.3.1`) using the configured `[id] section_separator`; equivalently pass `--section 3.1`.
- `<path>` — directory or file whose tree is scanned. Defaults to `.`. Discovery is the same as every other subcommand (walk up to `.agents/gnd.toml`, else defaults — §FS-config.1).
- `--section <s>` — restrict to citations that reference exactly that section path. Without it, every citation of `<ID>` is listed regardless of section (including bare-ID citations with no section). Mutually exclusive with the dotted inline form.
- `--format text|json` — output shape (§3). Default `text`.

`refs` is a query, like `show` — non-interactive, no prompts (§FS-non-goals.10).

## 2. Behaviour

`refs` runs the same scan as `check` (§AS-scanner) and emits, for the requested `<ID>`, every recognised citation site — the same set of citations `check` would validate, so it honours `[reference] strict` (bare tokens are listed only in non-strict mode), the string-literal carve-out in source files (§AS-scanner.2.3), and citations inside doc-comments. It does **not** list the *declaration* of `<ID>` — that is `gnd show --format=json <ID>` (the README documents that one-liner). A `refs` lookup of an ID that has no declaration still works: the citations are listed (they are exactly the ones `check` flags as dangling), so `refs` is also the "what would break if I never create this ID" tool.

Output is sorted by `(path, line, column)` — deterministic per §FS-errors.4. An ID with zero citations produces empty output and exit `0` (not an error: an as-yet-uncited declaration is normal, and `check` already warns about it — §FS-check.4.1).

## 3. Outputs

### 3.1 `--format text` (default)

One line per citation site, in the located-finding shape (§FS-errors.2.1):

```
$ gnd refs FS-check.1
docs/functional-spec/FS-show.md:11: §FS-check.1
src/scanner.rs:142: FS-check.1
```

`<message>` is the citation token exactly as it appears in the source — marker-prefixed or bare, with its section suffix — so the reader sees the form on disk. Stdout is empty (this is diagnostic-shaped output to stderr, consistent with `check`); the result is the lines themselves. Exit `0` always when the scan succeeds, regardless of how many citations were found.

### 3.2 `--format json`

NDJSON on stdout — one object per citation, matching the `Citation` shape (§AS-scanner.3) plus the verbatim token:

```json
{"path":"docs/functional-spec/FS-show.md","line":11,"column":42,"id":"FS-check","section":"1","marker":true,"text":"§FS-check.1"}
{"path":"src/scanner.rs","line":142,"column":12,"id":"FS-check","section":"1","marker":false,"text":"FS-check.1"}
```

`section` is `null` for a bare-ID citation with no section coordinate.

## 4. Exit codes

- `0` — scan succeeded; the listed citations (possibly none) are the result.
- `2` — scan / I/O error (§FS-check.2 partial-scan semantics apply: an incomplete scan exits `2` and the lookup is not trustworthy as complete).

There is no `1`: `refs` is a query that always returns *its* answer (a possibly-empty list), never "found something other than one body" — unlike `show`, it has no single-result expectation to violate.

## 5. Why this exists

`grep -oE '§…'` gives a contributor a rough back-reference list but cannot: distinguish a real citation from an ID-shaped substring in a string literal; respect `strict` mode; reach citations inside block doc-comments without language-specific regex; or produce a stable, machine-shaped result for an agent to program against. `refs` is the scheme's own answer, sharing the scanner with `check` so the two never disagree on what counts as a citation. Together with `gnd show` it closes the loop: `show` reads the body an ID promises, `refs` enumerates the code and docs that took the promise.

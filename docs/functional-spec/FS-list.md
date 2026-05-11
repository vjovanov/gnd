# FS-list: gnd lists every declared ID

The `list` subcommand prints the repo's ID catalog: every declaration, where it lives, and its one-line title. It is the index that `gnd show` reads from and the broad counterpart of `gnd refs` — `refs` answers "who cites *this* ID?", `list` answers "what IDs are there?". An agent that has been told to ground itself with `gnd show <ID>` needs a way to discover the `<ID>`s; a human auditing a spec tree needs the same. Serves [§G-friendliness-first](../goals/goals.md#g-friendliness-first-as-user--and-agent-friendly-as-possible) (no `grep` for `^# [A-Z]+-` across the tree) and the agent-grounding loop in the raison-detre.

## 1. Inputs

```
gnd list [<path>] [--kind <KIND>] [--unused] [--format text|json]
```

- `<path>` — directory or file whose tree is scanned. Defaults to `.`. Discovery is the same as every other subcommand (walk up to `.agents/gnd.toml`, else defaults — [§FS-config.1](FS-config.md#1-file-location-and-discovery)).
- `--kind <KIND>` — list only declarations whose ID has that kind prefix (one of the configured `[[kinds]]` prefixes — [§FS-config.3.4](FS-config.md#34-kinds--recognized-prefixes)). An unknown kind is a CLI-level error (§4): a typo'd `--kind` must not silently produce an empty catalog.
- `--unused` — list only declarations that no recognised citation points at. This is a catalog query over `refs = 0`, not the same as `check`'s unused-declaration warning set: `check` suppresses uncited `E2E` warnings because a test is used by being run ([§FS-check.4.1](FS-check.md#41-unused-declaration)), but `list --unused --kind E2E` still lists uncited case declarations for inventory.
- `--format text|json` — output shape (§3). Default `text`.

`list` is a query, like `show` and `refs` — non-interactive, no prompts ([§FS-non-goals.10](FS-non-goals.md#10-interactive-mode)).

## 2. Behaviour

`list` runs the same scan as `check` ([§AS-scanner](../architectural-spec/AS-scanner.md#as-scanner-how-gnd-discovers-declarations-and-citations)) and emits, for every declaration the scan found, one catalog line. The set of declarations is exactly the set `check` validates and `show` can resolve, so the three never disagree on what exists.

- **Order.** Declarations come out sorted by ID — kind, then number, then slug — the same stable order `check` reports diagnostics in ([§FS-errors.4](FS-errors.md#4-determinism)). The result is deterministic for a given tree.
- **Stub-and-inline pairs collapse.** When an ID's home is an inline declaration in source code with a one-line stub under `docs/architectural-spec/` pointing at it (the [§FS-check.3.4](FS-check.md#34-broken-inline-spec-stub) / [§FS-show.2.3](FS-show.md#23-inline-declarations-in-code-and-doc-comments) arrangement), `list` shows **one** line for that ID, naming the source file where the body lives — not two lines, one for the stub and one for the inline declaration. A *broken* stub (its target missing, or the target has no matching inline declaration) is not paired with anything, so it does appear, listed at the stub's own location with a `→ <target>` note; `check` reports the breakage in located form.
- **Duplicate declarations.** When an ID is declared in more than one independent home — the [§FS-check.3.3](FS-check.md#33-duplicate-declaration) error — `list` prints one line per home, each flagged so the duplication is visible at a glance. `list` does not pick a winner; it shows the situation and leaves the located error to `check`.
- **What it is not.** `list` does not print declaration *bodies* (that is `gnd show`), and it does not list *citations* (that is `gnd refs <ID>`). It does not modify anything. An ID that is cited but never declared does **not** appear in `list` — it has no declaration to catalog; `gnd refs <ID>` and `gnd check` are where a dangling citation surfaces.

## 3. Outputs

### 3.1 `--format text` (default)

One line per catalog entry on **stdout** (this is a result a caller consumes and pipes, like `gnd show` / `gnd id` / `gnd config show`, not diagnostic output):

```
$ gnd list
AS-event-bus    src/bus.rs:14                 In-process event broadcaster
FS-check        docs/functional-spec/FS-check.md:1    gnd validates every reference in a repo
FS-login        docs/functional-spec/FS-login.md:1    A player can log in with email
G-no-dangling-refs  docs/goals/goals.md:7     every cited ID resolves to a declaration
```

The columns are: the ID (rendered in the repo's `[id] format`, left-padded so the column aligns — capped so one very long ID does not blow out the table), then `<path>:<line>` of the home declaration (for a collapsed stub-and-inline pair, the source file the body is in), then the title — the heading text the author wrote after `# <ID>:`. A declaration whose heading carries no `: <text>` tail has an empty title column. A broken stub shows `→ <target>` in place of a title. A duplicated ID's lines carry a `(duplicate declaration — gnd check)` note. With `--kind`, only that kind's lines appear; with `--unused`, only lines for declarations with zero inbound citations, including uncited `E2E` declarations even though `check` does not warn for them. An empty catalog (or an empty filter result) prints nothing — that is not an error.

Stderr is empty on success.

### 3.2 `--format json`

NDJSON on stdout — one object per catalog entry, same order as the text form:

```json
{"id":"AS-event-bus","kind":"AS","path":"src/bus.rs","line":14,"title":"In-process event broadcaster","stub":false,"defines":null,"refs":3,"duplicate":false}
{"id":"FS-login","kind":"FS","path":"docs/functional-spec/FS-login.md","line":1,"title":"A player can log in with email","stub":false,"defines":null,"refs":7,"duplicate":false}
```

Fields: `id` (rendered ID), `kind`, `path` and `line` of the home declaration, `title` (`null` when the heading has no title tail or the home is a broken stub), `stub` (true when this entry's home is a stub heading — only ever true for a *broken* stub, since a healthy one collapses into its inline declaration), `defines` (the `<target>` of a stub heading, else `null`), `refs` (the count of recognised citations of this ID across the scanned tree — the JSON form always carries it so a tool need not run `gnd refs` per ID just to find the uncited ones), and `duplicate` (true when the ID has more than one home). The wire form is stable per [§G-no-silent-breakage](../goals/goals.md#g-no-silent-breakage-changes-ship-through-a-deprecation-path).

## 4. Exit codes

- `0` — the scan succeeded; the listed catalog (possibly empty) is the result.
- `2` — scan / I/O error ([§FS-check.2](FS-check.md#2-outputs) partial-scan semantics apply: an incomplete scan exits `2` and the catalog may be short), an unknown `--kind`, an unsupported `--format`, or any other CLI-level error ([§FS-cli.4](FS-cli.md#4-errors-with-no-source-location)).

There is no `1`: `list` is a query that always returns *its* answer (a possibly-empty catalog), never "found something other than one body" — unlike `show`, it has no single-result expectation to violate.

## 5. Why this exists

`grep -RhoE '^#+ [A-Z]+-[a-z0-9-]+'` across `docs/` gives a contributor a rough list of declaration headings but cannot: reach inline declarations inside source-code doc-comments; collapse a stub onto the inline declaration it points at; honour the configured `[id]` grammar in a repo that customised it; tell which declarations are uncited; or produce a stable, machine-shaped result an agent can program against. `list` is the scheme's own answer, sharing the scanner with `check` so the catalog and the validator never disagree on what a declaration is. With `show` and `refs` it completes the read surface: `list` enumerates the IDs, `show` reads the body one promises, `refs` enumerates who took the promise.

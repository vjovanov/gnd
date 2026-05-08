# FS-009-name: gnd proposes IDs for new declarations

The `name` subcommand emits a single, conflict-free `<KIND>-<NNN>-<slug>` ID for a new declaration. Authors writing a new spec, agents drafting a new doc, and IDE plugins offering a "new declaration" action all call the same primitive — so the next number for a kind, and the canonical slug for a title, are computed in exactly one place. Serves G-005-friendliness-first (no human picks the next number by reading a directory listing) and G-001-no-dangling-refs (proposed IDs cannot collide with existing declarations).

## 1. Inputs

```
gnd name <KIND> "<title>" [<path>] [--width <N>] [--format text|json]
```

- `<KIND>` — required. One of the configured `[[kinds]]` prefixes (FS-006-config.3.4). Unknown kinds are an error (§4).
- `<title>` — required. Free-form human title for the new declaration; converted to a slug per §3.
- `<path>` — directory whose tree is scanned to determine "next free number." Defaults to the current directory. Discovery is the same as every other `gnd` command (walks up to find `gnd.toml`; falls back to defaults).
- `--width <N>` — minimum digit width for the number. Default `3`, matching the canonical form's `-NNN-`. The number is left-padded with zeros to at least this width; numbers that already exceed the width are emitted as-is.
- `--format text|json` — output shape (§2). Default `text`.

Per FS-007-non-goals.10, `name` is non-interactive: no prompt, no confirmation. The proposed ID is the only output.

## 2. Outputs

### 2.1 `--format text` (default)

A single line on stdout: the proposed ID, with no marker prefix, followed by a newline.

```
$ gnd name FS "User can log in with email"
FS-008-user-can-log-in-with-email
```

This is shaped for shell composition. A typical workflow:

```sh
ID=$(gnd name FS "User can log in with email")
$EDITOR "docs/functional-spec/${ID}.md"
```

Stderr is empty on success. The `path:line:` prefix from G-005-friendliness-first.1 does not apply — `name` synthesizes; it does not point at a source location.

### 2.2 `--format json`

```json
{"id":"FS-008-user-can-log-in-with-email","kind":"FS","number":8,"slug":"user-can-log-in-with-email","folder":"docs/functional-spec"}
```

`folder` is the configured `[[kinds]] folder` for the kind (FS-006-config.3.4) — the conventional home for declarations of this kind, included so editor "create new declaration" actions can place the file without a second lookup.

## 3. Slug derivation

The title is converted to a slug deterministically. Two `gnd name` calls with the same title and the same configured `slug_pattern` produce the same slug, on every platform, in every locale (FS-007-non-goals.13).

1. Unicode-normalize the title to NFKD, strip combining marks. (`Café log-in` → `Cafe log-in`.)
2. Lower-case. ASCII-only; non-ASCII letters that survive step 1 are passed through to step 3 unchanged and will be filtered there.
3. Replace every run of characters that does **not** match the configured `slug_pattern` character class (FS-006-config.3.2) with a single `-`. The default pattern is `[a-z0-9][a-z0-9-]*`, so spaces, punctuation, and quotes all collapse to `-`.
4. Trim leading and trailing `-`.
5. Collapse runs of two or more `-` into a single `-`.
6. Truncate to 60 characters at the nearest preceding `-` boundary (so a slug never ends mid-word).

If the resulting slug is empty (every character in the title was non-slug), `name` exits with code 1 and:

```
error: title produces empty slug after normalization: "<original title>"
```

The author is expected to provide a title that contains at least one slug-character. `name` does not invent a fallback — a meaningless slug is worse than an error.

## 4. Next-number derivation

The scan from FS-001-check runs across the tree (or the configured `[scan] include` paths from FS-006-config.3.5) and collects every declaration of the requested `<KIND>`. The proposed number is `max(existing numbers) + 1`, or `1` if the kind has no existing declarations.

Holes in the numbering (e.g., `FS-001`, `FS-002`, `FS-004` exists but `FS-003` does not) are **not** filled. Numbers are issued strictly above the maximum, never reused, never recycled. Reasoning: an ID that once existed and was removed may still be cited from external systems (PRs, chat, mirrored repos); reusing the number would silently change what those references point at. This is the same principle as FS-007-non-goals.4 (no rename) applied to allocation.

If the scan fails (I/O, malformed file), `name` exits 2 with the underlying error — it does **not** fall back to a guess. Allocating a number against an incomplete view of the tree could produce a collision.

## 5. Collision check

After deriving slug and number, `name` verifies the full proposed ID does not already appear as a declaration in the scanned tree. This is belt-and-suspenders against:

- A configured `slug_pattern` that admits ambiguity (e.g., a project that loosened the pattern after declarations were authored under the strict default).
- A `--width` change that re-zero-pads existing IDs into the candidate.

If a collision is detected, `name` exits 1 with:

```
error: proposed ID FS-008-user-login already declared at docs/functional-spec/FS-008-user-login.md:1
```

Authors disambiguate by editing the title.

## 6. Exit codes

- `0` — proposed ID emitted.
- `1` — kind unknown, slug empty, or collision detected.
- `2` — scan / I/O error.

## 7. What `name` does **not** do

- It does not create the file. Mechanically writing a stub is the caller's job — for an author, an `$EDITOR` invocation; for an IDE plugin, the `New file` action; for an agent, a follow-up `Write`. `name` stays a pure function from `(kind, title, tree)` to `id`. (Reasoning: the same primitive serves three callers with three different file-creation behaviors; baking any one of them in shrinks the surface.)
- It does not modify `gnd.toml`, the scanned tree, or any existing declaration.
- It does not allocate a number reservation. Two parallel `gnd name FS …` calls against the same tree may both propose `FS-008-…`; the loser sees the collision when their declaration is committed and `gnd check` runs. Reasoning: reservation is state, and `gnd` is stateless (FS-007-non-goals.6).

## 8. Why this exists

Three callers, one source of truth:

1. **Authors.** Picking the next free number means listing a directory and squinting; one typo creates a duplicate that `gnd check` will catch hours later. `name` removes the typo class.
2. **Agents.** An LLM proposing a new declaration cannot reliably read a directory listing and increment the right number — and even if it can, the answer drifts with the next file added. Calling `gnd name` is cheap, deterministic, and committed to the same regex grammar as the checker.
3. **IDE plugins.** The "new declaration" action in FS-003-ide-plugins needs the same number `gnd name` would compute; sharing the engine means there is exactly one allocator, not three subtly different ones.

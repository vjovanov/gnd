# DF-reference-marker: Use § as the reference marker, with $$ as the typing trigger

**Status:** Accepted
**Date:** 2026-05-08

## 1. Context

The gnd reference scheme cites IDs as bare tokens in prose: `FS-user-login.3.1`. This is token-cheap but ambiguous — a stray match in unrelated text (e.g. an issue tracker code, a regex example) can be picked up by the scanner as a citation, producing false positives and false dangling-ref errors.

We want a **single distinguishing character** in front of every citation that:

- Is rare enough that it will not appear by accident in code or prose followed by an ID-shaped token.
- Is aesthetically pleasing — citations are read often.
- Carries the right semantic weight (this is a *reference*, not random punctuation).

We also want a way for users to **type it without leaving the keyboard**, since the whole point of rarity is that no keyboard puts it on a key.

## 2. Decision

### 2.1 Marker

Use **`§`** (U+00A7, "section sign") as the reference marker. A canonical citation is:

```
§FS-user-login.3.1
```

Why:

- Semantic gold standard — the section sign means "section" in legal and academic citation.
- Aesthetically dignified, established typographic tradition.
- Almost never followed by `<KIND>-<digit>` in unrelated text; the regex `§<KIND>-\d+-` produces effectively zero false positives.
- Already supported in every modern font; renders crisply at any size.

### 2.2 Trigger

Default trigger sequence is **`$$`**, transformed to `§` whenever immediately followed by `<KIND>-<digit>`.

Why `$$`:

- Two same-keystrokes; both `$` are shift+4 on US layouts.
- Visually rhymes with `§` (curving stroke + central crossbar).
- The "only when followed by `<KIND>-<digit>`" constraint kills the LaTeX `$$` (display math) false positive entirely.

### 2.3 Trigger ownership

`gnd` owns the trigger transformation. It runs in two places:

- **Live, in the gnd IDE plugins (FS-ide-plugins).** Type `$$FS-check`; `$$` becomes `§` as soon as the regex matches.
- **Bulk, via `gnd fmt` (FS-fmt).** Walk files and rewrite `<trigger><ID>` to `<marker><ID>`. Idempotent. Used as a pre-commit hook and a CI safety net.

Editor-native input methods (snippets, Compose, OS Unicode entry) remain available for power users — they bypass the trigger and write `§` directly.

### 2.4 Strict vs optional

**Default: optional.** Bare `FS-user-login` is still a valid citation; gnd recognizes it. `§FS-user-login` is preferred; tooling and editor previews use the marker form.

**Opt-in strict mode.** Setting `[reference] strict = true` in `gnd.toml` makes the marker mandatory: bare tokens stop being treated as citations, eliminating false positives in repos that adopt the discipline fully. Strict mode is recommended once a repo has been migrated.

### 2.5 Configurability

Both marker and trigger are configurable per G-configurable:

```toml
[reference]
marker  = "§"     # default
trigger = "$$"    # default
strict  = false   # default; set true to require the marker
```

Other valid markers we considered: `※` (U+203B, Japanese reference mark), `‡` (U+2021, double dagger), `⁂` (U+2042, asterism). Any of these is a one-line config change.

## 3. Consequences

- The scanner (AS-scanner) recognizes both bare and marker-prefixed citations by default, and only marker-prefixed citations under `strict = true`.
- The IDE plugins (FS-ide-plugins) transform `$$<KIND>-<digit>` to `§<KIND>-<digit>` on the fly.
- A new functional spec, FS-fmt, defines `gnd fmt` for bulk transformation.
- Existing repos that use bare citations continue to work unchanged. Migration to marker-prefixed citations is mechanical: `gnd fmt --marker --check` reports unconverted citations; `gnd fmt --marker` rewrites them.
- The marker becomes the visible signal of a gnd citation. A reader scanning a file sees `§FS-...` and immediately knows: this is a reference, follow it.

## 4. Alternatives considered

| Marker | Why rejected |
|---|---|
| `※` (Japanese reference mark) | Strongest rarity, but unfamiliar to most Western readers; harder to learn what it means. |
| `‡`, `†` (daggers) | Established academic citation marks but evoke footnotes more than section refs; double dagger reads as "footnote of footnote." |
| `⁂` (asterism) | Beautiful but ornamental; reads as a section break, not a reference. |
| `¶` (pilcrow) | Word-processor-coded; less serious. |
| `[[...]]` (Obsidian-style) | Familiar but four extra characters per citation; loses token-cheap property. |
| No marker | Status quo; suffers the false-positive problem that motivates this decision. |

| Trigger | Why rejected |
|---|---|
| `::` | Reserved in Rust/C++/Haskell; false transforms in code. |
| `..` | Path operator (`../`) and sentence punctuation. |
| `[[FS-001]]` (full bracket form) | Verbose; defeats the goal of keeping citations short. |
| Editor-native only (no gnd trigger) | Inconsistent across editors; contributors on unfamiliar editors get no help. |

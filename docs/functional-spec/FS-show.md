# FS-show: gnd reads a single declaration body by ID

The `show` subcommand prints just the body of a declaration, given an ID. It exists so an agent — human or AI — can pull a single grounded fact into context without loading the whole file. Serves G-friendliness-first.

## 1. Inputs

```
gnd show <ID> [--section <s>] [--head | --full] [--format <text|md|json>]
```

- `<ID>` — the full ID (e.g. `FS-check`). May include an inline section (`FS-check.3.1`). The dotted form uses the configured `[id] section_separator` (FS-config.3.2). When the separator is non-default (e.g., `:` or `#`) the inline form may collide with the slug grammar; use `--section` instead.
- `--section <s>` — alternative way to specify a section path (`3.1`). Mutually exclusive with the dotted form. Required when `[id] section_separator` makes the dotted form ambiguous.
- `--head` — print only the top of the context: the heading line and the prose up to the first numbered subsection. Useful for skimming.
- `--full` — print the entire body (default).
- `--head` and `--full` are mutually exclusive.
- `--format` — output shape; defaults to `text` (just the body, no headers).

## 2. Behavior

### 2.1 Whole declaration (default, or `--full`)

`gnd show FS-check` prints from the heading of `FS-check` to the start of the next ID heading (or end of file). The opening heading is omitted in `text` format and included in `md`.

### 2.1.1 Head only (`--head`)

`gnd show --head FS-check` prints only the top of the context: the prose between the ID heading line and the first numbered section heading (`## 1. ...`). This is the "what is this about" view — typically a paragraph or two — meant for quick scanning, hover previews, and agent prompts where the section structure isn't needed.

If a declaration has no lead paragraph (its body opens directly with `## 1. ...`), `--head` prints **nothing** and exits `0`. This is not an error: the declaration simply has no head. Callers (IDE hovers, agents) can detect this case by the empty output and decide whether to fall back to the full body. We do not auto-fall-back; the caller knows what it wants.

### 2.2 Section

`gnd show FS-check.3.1` prints just the contents under section heading `### 3.1 ...` within the declaration body, stopping at the next sibling-or-shallower heading. Nested deeper headings (e.g., `#### 3.1.2`) are included in the output — they end at the next `### 3.x` (sibling) or `## N.` (shallower) heading. Arbitrary nesting depth is supported per FS-config.3.3.

### 2.2.1 Ambiguous ID

If an ID has more than one home — the duplicate-declaration error from FS-check.3.3 — `show` does not pick one. A stub paired with the inline declaration it points at is *one* home, not two; ambiguity means two or more independent declarations remain after that pairing collapses. When ambiguous, `show` exits 1 with a single bare stderr line (no `<path>:<line>:` prefix, since there is no single site to point at):

```
ambiguous ID: <ID> (declared at <path>:<line>, <path>:<line>[, ...])
```

Sites are listed in lexicographic `path:line` order so the message is stable across runs. The repo must be fixed (run `gnd check` first) before `show` will return a body. This shape matches the bare-message form used for `ID not found` and `section not found` (FS-show.3): all three are queries that found something other than exactly one body.

### 2.3 Inline declarations in code and doc-comments

When the ID's home is in code (per FS-check.3.4 stub semantics), `show` extracts the comment block surrounding the inline declaration, strips comment markers, and prints the resulting prose. The same section logic applies.

The scanner recognizes the same doc-comment forms enumerated in AS-scanner.4 — Javadoc, JSDoc/TSDoc, Doxygen, KDoc, Scaladoc, PHPDoc, Rustdoc (`///`, `//!`, `/** … */`), C# XML doc comments, Go's `// …` doc blocks, Ruby `#` comments, and Python `""" … """` docstrings. This means an architectural spec can live directly in the class-level Javadoc, and `gnd show AS-event-bus` returns the rendered Javadoc body — same content the IDE plugin shows on hover (FS-ide-plugins.1.2). The stub at `docs/architectural-spec/AS-event-bus.md` is a single-line H1 — `# AS-event-bus: [<path>](<path>)` — pointing at the file.

#### 2.3.1 What counts as the "comment block"

Extraction is precisely defined so that the implementation has no freedom and the same input produces the same output across editor, CLI, and binding callers.

A declaration is found on a "declaration line" — a line that matches the declaration regex from AS-scanner.2.1 *and* sits inside a comment or docstring. The block surrounding it is computed as follows:

1. **Find the open boundary.** Walk **backwards** from the declaration line over consecutive lines that are part of the same comment construct:
   - For line-style comments (`//`, `///`, `//!`, `#`, `;`, `--`): consecutive lines whose first non-whitespace character matches the same comment prefix family. A blank line ends the block. A line whose first non-whitespace character is not a comment marker ends the block.
   - For block-style comments (`/* … */`, `/** … */`): walk backward until the opener is found (`/*` or `/**`). The opener line itself is part of the block.
   - For Python triple-quoted docstrings: walk backward until the opening `"""` (or `'''`). The opener line is part of the block.

2. **Find the close boundary.** Walk **forwards** from the declaration line by the symmetric rules:
   - Line-style: until a blank line or a non-comment line.
   - Block-style: until the closing `*/`. The closer line is part of the block.
   - Python docstring: until the matching `"""` or `'''`. The closer line is part of the block.

3. **Terminate early on another declaration.** Walking in **either direction**, if another declaration line of any ID is encountered, the block ends at the line before it. This is what allows two adjacent inline declarations to live in the same comment without bleeding into each other — backward termination keeps a later declaration's block from absorbing the previous declaration's tail; forward termination keeps the previous declaration's block from absorbing the next declaration's head.

#### 2.3.2 Stripping comment markers

After the block is selected, comment markers are removed line-by-line so the output is plain prose:

- Leading whitespace is preserved up to the comment marker, then the marker is dropped, then a single space following the marker is dropped if present. The remainder of the line is kept verbatim.
- For block-style continuation lines, a leading ` * ` (with surrounding spaces) is removed if present. Lines that do not have it are kept as-is.
- For Python docstrings, no marker is stripped — docstring content is plain text already; only the surrounding `"""` lines are skipped.
- Trailing comment-close markers (`*/`) on their own line are dropped entirely.
- Blank lines inside the block are preserved.

The result is the markdown that the declaration's author wrote, identical to what would have lived in a `.md` file had the spec been doc-resident instead of inline. This is the property that makes FS-show.2.3 round-trip-stable across the in-docs and in-code homes.

#### 2.3.3 Section selection inside a doc-comment

Section selection (`AS-event-bus.2`) works the same way inside a doc-comment as inside a markdown file: the scanner records the numbered subsection headings declared within the doc-comment block and `show` slices to the requested section. Headings inside doc-comments use the same `## N. …`, `### N.M. …`, `#### N.M.O. …` convention as markdown — the comment-stripping pass leaves them intact.

## 3. Outputs

- `0` — printed successfully.
- `1` — ID not found, ambiguous ID (multiple homes — FS-show.2.2.1), or section not found in declaration.
- `2` — I/O error.

Stdout for the body. Stderr for errors. Empty stdout on error.

## 4. Why this matters

Without `show`, an agent retrieving a spec section either loads the whole file (token-expensive) or reimplements the parser. With `show`, the canonical way to pull `FS-check.3.1` into a prompt is exactly:

```
gnd show FS-check.3.1
```

This is the agent-grounding loop: declarations live in one place, and any agent — at any time — can fetch one with a single command.

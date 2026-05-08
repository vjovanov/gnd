# Project improvement proposal

This document is a current recommendation for improving `gnd` based on the repository as it stands now. The project has expanded from a reference checker into a broader grounding system: marker-prefixed citations, `fmt`, config, doc-comment specs, editor integrations, and multi-registry distribution are now all in scope.

The main risk is no longer lack of ambition. The main risk is that the spec surface is growing faster than the executable core.

## 1. Restore a passing self-host loop

The first milestone should still be: `cargo run -- .` against this repository exits zero.

Current blockers include:

- Some goals have moved to `docs/goals/`, but not every cited goal has a matching declaration file.
- `docs/goals.md` still duplicates or overlaps with split goal files, which makes the source of truth unclear.
- Example IDs such as `FS-042-user-login` are scanned as real citations.
- Example declarations inside prose and code blocks can be scanned as real declarations.
- The inline `AS-014-event-bus` example in the scanner spec is treated as a real declaration/stub and points at a non-existent sample file.
- Successful runs still print a summary, despite the zero-noise requirement.

Recommended fix: make examples explicitly non-semantic before adding new features. Fenced blocks, marked examples, and illustrative IDs should not participate in the real project graph.

## 2. Make one document layout authoritative

The project now has both a monolithic goals document and split files under `docs/goals/`.

Choose one of these:

- Keep split goal files and turn `docs/goals.md` into a short index with no live declarations.
- Keep the monolithic goals file and remove the split goal files.

The split-file model is probably better because it matches the retrieval story: `gnd show G-001-no-dangling-refs` should return one compact declaration body.

Do the same audit for examples in functional and architectural specs. If a document contains illustrative IDs, make sure the scanner can distinguish them from real citations.

## 3. Implement the marker-aware scanner before `fmt`

The `§` marker decision is now central. The current scanner still finds bare IDs only; it happens to match marker-prefixed IDs because the marker sits before a word boundary, not because marker semantics are implemented.

Implement this deliberately:

- Parse citations as either marker-prefixed or bare.
- Record whether a citation used the marker.
- Support optional mode first: marker-prefixed and bare citations both count.
- Support strict mode next: only marker-prefixed citations count.
- Make error messages preserve the user's spelling, including the marker when present.

This should land before `gnd fmt`, because `fmt` depends on a correct understanding of what is a citation.

## 4. Add the real e2e harness

`e2e/README.md` describes the right structure, but the actual corpus is not populated yet.

Add fixture directories and golden expectations for:

- valid minimal repo
- dangling citation
- missing section
- duplicate declaration
- broken `Defined-in:` path
- stub target exists but lacks the matching declaration
- fenced examples ignored
- marker-prefixed citation
- optional-mode bare citation
- strict-mode bare token ignored
- strict-mode marker citation accepted
- `fmt --check` reports pending trigger replacement
- `fmt --marker --check` reports pending bare-to-marker replacement

Run the binary from these tests. Unit tests can support parser edge cases, but the acceptance contract is CLI behavior.

## 5. Add command parsing now

The current binary treats the first argument as a path, so subcommands cannot work.

Add a real command surface:

```bash
gnd [check] [path]
gnd show <ID> [--section <section>] [--head | --full] [--format text|md|json]
gnd fmt [path] [--check] [--marker] [--write]
gnd config validate [path]
gnd config show
```

Use `clap` unless there is a strong reason not to. The CLI contract is now too large for ad hoc argument parsing.

## 6. Build config in phases

`FS-006-config` is intentionally broad, but implementing it all at once would put too much risk into the scanner.

Phase one should support only:

- config discovery
- marker
- trigger
- strict mode
- include paths
- exclude directories
- file extensions
- output format default

Defer these until the default grammar is stable:

- custom ID format templates
- alternate section separators
- custom number and slug regexes
- Python docstring mode
- per-kind folders and titles

The current spec can remain aspirational, but implementation should be phased and each phase should have fixtures.

## 7. Implement `show` before editor integrations

The editor integrations depend on the same retrieval semantics as `gnd show`. Do not start LSP work until `show` is correct.

Recommended implementation order:

1. Locate one declaration by ID.
2. Return the full declaration body from Markdown.
3. Return a numbered section body.
4. Add `--head`.
5. Add text output only.
6. Add Markdown and JSON output.
7. Add inline doc-comment extraction.

This gives agents the core retrieval primitive early and prevents every editor plugin from reimplementing slicing logic.

## 8. Treat doc-comment scanning as a second scanner milestone

The scanner spec now promises Javadoc, Rustdoc, JSDoc, Doxygen, KDoc, Scaladoc, PHPDoc, Go comments, Ruby comments, and Python docstrings.

That is useful, but it is a large parsing surface. Split it:

- First support line comments and Markdown.
- Then support block comments.
- Then support Python docstrings.
- Then verify each language form with fixtures.

Avoid claiming broad language support until fixture coverage exists for each promised form.

## 9. Implement `fmt` as a conservative rewriter

`gnd fmt` can damage source files if it rewrites inside strings or generated examples.

For the first version:

- Rewrite only files in the configured scan set.
- Skip fenced Markdown blocks.
- Rewrite trigger-to-marker everywhere a citation context is unambiguous.
- With `--marker`, rewrite only citations the scanner would recognize.
- Make `--check` the default behavior.
- Require `--write` for mutation.

Do not add aggressive language heuristics until the project has a regression corpus for source-file rewriting.

## 10. Split core and CLI before bindings

The distribution and editor specs assume a reusable `gnd-core`. The current implementation is a single `src/main.rs`.

Split before adding npm, Python, or LSP:

- `gnd-core`: config, scanner, checker, show, fmt planning, report data structures
- `gnd-cli`: argument parsing, rendering, exit codes

The split should not wait until distribution work. It will make e2e tests cleaner and prevent CLI concerns from leaking into the engine.

## 11. Recheck distribution naming before release

The distribution spec claims the Python package name is available, while the naming decision says it is already taken. Resolve this before publishing plans harden.

Recommended policy:

- Keep the binary name `gnd` if desired.
- Use explicit package names where registry names are unavailable.
- Recheck registry availability immediately before release.
- Avoid docs that claim a package name is free unless the project owns it.

This can wait until the CLI is stable, but the docs should not contradict each other.

## 12. Delay first-party editor plugins

The editor plugin spec now covers VSCode, IntelliJ, Vim/Neovim, and Emacs. That is a serious maintenance commitment.

Recommended sequencing:

1. Build `gnd-core`.
2. Build `gnd show`.
3. Build a small `gnd-lsp` with hover and go-to-definition.
4. Ship one thin editor integration first.
5. Add other editors after the LSP protocol behavior is stable.

The four-editor plan is reasonable as a direction, but it should not compete with the core checker/retriever milestones.

## Recommended order

1. Make self-hosting pass.
2. Pick the authoritative docs layout for goals.
3. Add command parsing.
4. Add the e2e harness and first fixtures.
5. Implement marker-aware scanning and strict mode.
6. Implement phase-one config.
7. Implement text-only `show`.
8. Split `gnd-core` from `gnd-cli`.
9. Implement conservative `fmt`.
10. Add JSON output and report schema tests.
11. Add doc-comment scanning beyond line comments.
12. Revisit npm, Python, LSP, and first-party editor plugins.


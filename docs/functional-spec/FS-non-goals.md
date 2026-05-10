# FS-non-goals: what gnd will deliberately not do

This spec exists to prevent feature creep. Every entry below is a thing `gnd` could plausibly grow into and is **deliberately out of scope**. When a contributor proposes one of these, the answer is "no" — not because it would be hard, but because the answer was decided.

A non-goal is not the same as "we'll do it later." Non-goals are commitments. To turn one into a goal requires a decision record under `docs/decisions/architectural/` overturning this spec, with a clear rationale.

## 1. Markdown link validation

`gnd` does **not** validate `[text](url)` links, anchor `#section` references inside markdown, or HTTP URLs. Use [`lychee`](https://github.com/lycheeverse/lychee) for those — it is fast, focused, and well-maintained. Reasoning: there is no token-cheap reason to merge two lints into one binary.

## 2. Spelling, grammar, prose quality

`gnd` reads spec text as opaque content between IDs. It does not lint English. Use any general-purpose linter — `vale`, `ltex-ls`, or a thousand others — alongside `gnd`.

## 3. Code AST parsing

`gnd` does not parse code. It does line-oriented regex over comments and doc-comments (per AS-scanner). It does not understand classes, methods, types, scopes, or call graphs. The stub-heading link is a file path, not a symbol reference. Reasoning: G-fast-feedback rules out per-language parsers, and IDs are syntactic by design.

## 4. Cross-workspace ID renaming

`gnd` does not provide a "rename ID" refactoring. The reference scheme says IDs are forever; renaming an ID is a deliberate edit (`Supersedes:` chain), not an automated operation. The optional LSP server (§FS-lsp) intentionally omits this affordance, and no first-party editor wrapper would add it (§FS-non-goals.12.1).

## 5. Documentation generation

`gnd` does not generate HTML, PDF, or any rendered documentation from specs. It reads, validates, and slices spec content; it does not publish. Static-site generators are the right tool for that downstream — `gnd show <ID>` is a building block they can use.

## 6. Decision database, audit log, history tracking

`gnd` does not store decisions in a database, render decision graphs over time, track who changed what when, or visualize the ID graph. Git is the audit log; `git log` is the time machine. The reverse-lookup the project does ship — `gnd refs <ID>` (§FS-refs) — answers "who cites this ID *now*" from a single scan; it is a query over the current tree, not a stored graph or a history view.

## 7. Inter-agent messaging or workflow

`gnd` is a checker and a retriever. It does not coordinate agents, queue work, route messages, or implement any kind of state machine. Tools like `rhei` exist for that.

## 8. Generalization beyond the ID scheme

`gnd` does not validate other kinds of references — RFC numbers in random codebases, bug tracker IDs, etc. — outside the configured `[[kinds]]`. If a project does not adopt the ID scheme, `gnd` has nothing to offer. Reasoning: see the raison-detre — generalization dilutes the promise without expanding the audience.

## 9. Severity, exit code, or report-ordering customization

Per G-friendliness-first.2 and FS-config.6, the severity model (`error`/`warning`), the exit-code mapping (`0`/`1`/`2`), and the deterministic report ordering are **not** configurable. Reasoning: two correctly-configured `gnd` installs must agree on whether a repo passes. Letting any of these vary by project breaks that contract.

## 10. Interactive mode

`gnd` does not have a TUI, an interactive prompt, or a confirmation step. Every subcommand is non-interactive and CI-friendly. Reasoning: G-friendliness-first — interactive flows block CI and complicate scripting.

## 11. Network access during a check

`gnd check` performs no network I/O. There is no "fetch this URL," no "validate against a remote schema," no telemetry. The only filesystem access is reading the scanned tree. Reasoning: `gnd` runs in CI, in pre-commit hooks, and on laptops offline; correctness must not depend on the network.

## 12. Surfaces outside `gnd-core` and the LSP transport

`gnd` ships exactly two kinds of surface over the engine: the bindings enumerated in §FS-distribution (cargo CLI, Node API, Python API, LSP server) and nothing else. Anything that would add a third — in-engine scripting, per-editor wrappers, marketplace plugins — is out of scope.

### 12.1 Plugins or scripting hooks inside the engine

`gnd-core` is a library. Bindings (§FS-distribution) are first-party. There is no plugin system, no Lua hook, no JavaScript user script that runs during a check. Custom behavior is achieved by calling the API from your own code; the core stays small. Reasoning: a plugin surface multiplies attack surface, breaks reproducibility, and undermines "two installs agree."

### 12.2 First-party per-editor plugins

`gnd` does not ship and does not maintain VSCode extensions, IntelliJ plugins, Vim/Neovim plugins, Emacs packages, or any other editor-specific wrapper. The optional LSP server (§FS-lsp) is the only first-party editor surface; configuring an editor to talk to it is the user's one-time work, with example snippets in the README. Reasoning: per-editor plugins multiply maintenance surface across release cadences we do not control (marketplace review, extension manifests, native UI APIs), and the LSP protocol already gives every modern editor the four capabilities `gnd-lsp` exposes (§FS-lsp.1) for free. Decided in §DA-lsp-optional. Reconsidering this entry would require an architectural decision record overturning §DA-lsp-optional.

## 13. Anything that would let two `gnd` installs disagree

Above all: any feature that could cause two correctly-configured `gnd` installs — same version, same `gnd.toml` — to disagree on whether a given repo is well-formed is permanently out of scope. This is the bright line.

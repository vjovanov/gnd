# AS-checker: how gnd validates the scanner's findings

The checker takes the `Findings` produced by §AS-scanner and produces a `Report`. It implements the rules in §FS-check.

## 1. Inputs and outputs

- Input: `Findings` from the scanner, plus the repo root (needed to resolve stub-link paths).
- Output: a `Report` containing two ordered lists: `errors` and `warnings`. Order is deterministic for §G-friendliness-first.

## 2. Rules

Each rule is a single pass over part of the findings. Rules are independent — adding a rule does not force re-scanning.

### 2.1 Duplicate declarations (§FS-check.3.3)

For each ID with more than one declaration, emit one error anchored at the lexicographically-first site (sort by `path`, then `line`); list every other site parenthetically in the message. This keeps the report's `path:line:` prefix invariant (§3, §FS-check.2.1) while still naming all sites.

### 2.2 Dangling citations (§FS-check.3.1)

For each citation whose ID has no declaration, emit one error at the citation site.

### 2.3 Missing sections (§FS-check.3.2)

For each citation with a section path, look up the section in the matching declaration's recorded sections. Missing → one error at the citation site.

### 2.4 Broken inline-spec stubs (§FS-check.3.4)

For each declaration whose H1 has the stub shape `# <ID>: [<text>](<path>)` (description after the colon is a single bare markdown link), extract the link target, resolve it against the repo root, verify the path exists, then re-scan that file for an inline declaration of the same ID. Either failure → one error at the stub site.

### 2.5 Unused declarations (§FS-check.4.1)

For each declared ID never cited, emit one warning. Warnings do not cause non-zero exit.

## 3. Error format

Every error and warning follows `<path>:<line>: <message>` so that editors and agents can jump to the source. There is no severity prefix, and there is no aggregate summary footer — the exit code is the machine-readable verdict. This is mandated by §G-friendliness-first and §FS-check.2.1.

Findings without a single source location (CLI launch errors, malformed configuration that prevents a scan from starting) are emitted on a separate path as `error: <message>` per §FS-check.2.1.1, distinguishable from per-finding lines by the leading `error:`.

## 4. Why a separate stage from the scanner

The scanner produces a complete view of the world; the checker enforces rules on that view. Keeping them separate means:

- New rules can be added without touching the scanner.
- The optional LSP server (§AS-lsp) can run a subset of checks (e.g., only dangling references on the active file's citations) against a cached scan.
- Tests can feed synthetic `Findings` directly to the checker without disk I/O.

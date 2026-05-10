# FS-init: gnd bootstraps a new gnd-conformant repo

The `init` subcommand writes the minimum set of files a project needs to start using `gnd` — an `agents.md` entry point at the repo root and a `.agents/gnd.toml` config — so that a fresh repo (or an existing repo adopting the scheme) becomes scannable in one command. The config lives under `.agents/` per §FS-config.1, so the repo root stays free of `gnd`-specific files. In a repo that already has `agents.md`, `init` preserves the existing file and appends or updates a versioned `gnd` block by default. Serves §G-zero-config (the emitted defaults are the canonical grammar) and §G-friendliness-first (no prompts, no surprises).

`init` is the only `gnd` subcommand that creates adoption scaffolding in the working tree. It may also update the managed `gnd` block inside an existing `agents.md`; it must not rewrite unrelated user-authored content in that file unless `--force` is passed.

## 1. Inputs

```
gnd init [<path>] [--name <name>] [--docs] [--force] [--append]
```

- `<path>` — directory in which to scaffold. Defaults to `.` (the current directory). Applies to every form of `init` — including `--docs` — and prefixes every emitted path in §2.1. Must exist; `init` does not create the target directory itself (a missing target is a user error, not something to silently paper over).
- `--name <name>` — human-readable project name baked into the generated `agents.md` heading and the `.agents/gnd.toml` `project_name` key. Defaults to the basename of `<path>` resolved to an absolute path.
- `--docs` — also scaffold the canonical `docs/` tree (stub `raison-detre.md`, `goals/goals.md`, `roadmap.md`, `changelog.md`, `functional-spec/`, `architectural-spec/`, `decisions/architectural/`, `decisions/functional/`) and an empty `e2e/` directory with a stub `README.md`. The `roadmap.md` and `changelog.md` stubs are scaffolded because the generated `agents.md` block's `docs/` table links to them (§2.3). Off by default — most adopters already have a `docs/` of some shape and want only the entry point and config. Composes with `<path>`: every scaffolded file lands under `<path>/`.
- `--force` — overwrite files that already exist at the target paths. Off by default. Mutually exclusive with `--append`: passing both is a CLI-level error and exits 2 without touching the working tree (per §4).
- `--append` — explicitly request the default `agents.md` behavior: keep existing content and append or update the managed `gnd` block. This flag is accepted for scripts that want to state intent, but it is not required.

Per §FS-non-goals.10, `init` is non-interactive: it never prompts. Every choice is a flag.

### 1.1 Usage examples

```
gnd init                       # cwd, no docs tree
gnd init path/to/repo          # appends/updates agents.md if it already exists
gnd init --docs                # cwd, with docs/ + e2e/ scaffolds
gnd init --docs path/to/repo   # explicit target, with docs/ + e2e/ scaffolds
gnd init --append path/to/repo # same agents.md behavior as the default
gnd init --docs --name acme path/to/repo  # full form
```

Argument order is flexible: positional `<path>` may appear before or after the flags.

## 2. Outputs

### 2.1 Files written, updated, or left in place

In the default form (no `--docs`):

- `<path>/agents.md` — written when absent; appended or updated when present; see §2.3.
- `<path>/.agents/gnd.toml` — see §2.4. The `.agents/` directory is created if missing.

With `--docs`, additionally:

- `<path>/docs/raison-detre.md`
- `<path>/docs/goals/goals.md`
- `<path>/docs/roadmap.md`
- `<path>/docs/changelog.md`
- `<path>/docs/functional-spec/README.md`
- `<path>/docs/architectural-spec/README.md`
- `<path>/docs/decisions/architectural/.gitkeep`
- `<path>/docs/decisions/functional/.gitkeep`
- `<path>/e2e/README.md`
- `<path>/e2e/cases/.gitkeep`

Each scaffolded markdown file is a minimal starter — enough structure to teach the layout, no real content:

- `raison-detre.md` — the canonical H1 plus the three H2 sections (`## 1. The problem`, `## 2. What this project does about it`, `## 3. Who it is for`), each with a one-line italic prompt to be replaced.
- `goals/goals.md` — the H1 plus a one-line note on how goals are declared inline (`# G-NNN-slug: …`).
- `roadmap.md`, `changelog.md` — the H1 plus a single `<!-- placeholder - replace with real content -->` line.
- `functional-spec/README.md`, `architectural-spec/README.md` — the H1, the navigational note about how `FS-`/`AS-` IDs declare into the directory and the convention that the index lists every spec, and an empty `| ID | Subject |` table to fill in.
- `e2e/README.md` — the H1 (`# e2e`) plus a one-line note that every behaviour under `docs/functional-spec/` has at least one case.

The exact bytes for a given `gnd` version are embedded in the binary; reference copies live under `templates/` in the `gnd` source tree, and two `gnd init --docs` runs at the same version with the same `--name` produce byte-identical scaffolds (§FS-non-goals.13). `gnd check` is clean against the freshly-scaffolded tree. The `.gitkeep` files exist solely so the empty directories survive a `git add`; their content is a single line: `# placeholder — replace this directory's contents with real declarations`.

### 2.2 Stdout / stderr

On success, stderr lists every path touched, one per line:

```
wrote agents.md
wrote .agents/gnd.toml
```

The prefix is `wrote ` for a newly written or overwritten file, `appended ` when the versioned `gnd` block was added to an existing `agents.md`, `updated ` when an older managed block was replaced with the current version, and `exists ` when an existing file was left unchanged.

After the file list, stderr prints a short `next:` block — a blank line, then numbered first steps, then `see agents.md for the full workflow.` — so the user is not left at a bare list of paths wondering what to do. The steps are: run `gnd check` (a fresh tree is clean); allocate an ID with `ID=$(gnd name FS "…")` and write `docs/functional-spec/$ID.md` with the `# <ID>: …` H1; cite it as `§<ID>` from the docs and e2e tests that depend on it. When `--docs` was *not* passed, step 2 instead points at re-running with `--docs` (or creating `docs/` and `e2e/` by hand). This block is guidance, not a finding; it is part of the success output and does not change the exit code.

Paths are relative to `<path>`. Stdout is always empty (consistent with §G-friendliness-first.1 — output that other tools might pipe stays clean).

### 2.3 Generated `agents.md`

The emitted `agents.md` content is a canonical managed block: it explains the ID grammar, points at the `docs/` layout (including `roadmap.md` and `changelog.md`), tells an agent how to resolve a marker-citation on demand (`gnd show <ID>` / `gnd show <ID>.<section>` / `gnd list` / `gnd refs`) rather than re-reading whole files, instructs the agent to refresh cited specs with `gnd show` before editing code that carries those citations, instructs the agent to back-reference the spec from the implementation — a marker-citation in the doc-comment or inline comment of the function, class, or block that realizes a behavior, at the granularity it implements — so `gnd refs <ID>` enumerates the code that leans on a declaration, and lists the rules for agents (mirrors the rules in this repo's own `agents.md`). The canonical text for a given block version `vN` is embedded in the `gnd` binary; the reference copy lives at `templates/agents.md` in the `gnd` source tree, and the `vN` marker (§2.3) is what versions it under §G-no-silent-breakage.

Two things in the block are *substituted in* rather than fixed for that `vN`, so the file describes the repo it is in: the project name from `--name` (interpolated into the H1 and the opening sentence), and the **effective ID grammar** — taken from the `.agents/gnd.toml` `init` leaves governing the target (an existing config in the target, or the defaults `init` is about to write, never an ancestor's). From that config the block fills in the ID shape (`<KIND>-<NNN>-<slug>`, `<KIND>-<slug>`, …, derived from `[id].format`), one worked example ID and citation, the `[id].section_separator`, the marker and `$$`-trigger from `[reference]`, the `KIND ∈ {…}` set from `[[kinds]]`, and a sentence on whether bare ID-shaped tokens count as citations (driven by `[reference].strict`). The contract this spec makes is the *determinism and versioning*, not a literal transcript: two `gnd init` runs at the same `gnd` version against trees with the same `--name` and the same effective config produce byte-identical managed blocks (§FS-non-goals.13), and `gnd check`'s `agents.md` validation (§FS-check.3.5) checks the begin/end marker pair and the version, not a byte-diff against the canonical text.

One content rule the canonical text *does* commit to, because getting it wrong sets a trap: references to `gnd`'s own specification — the `check`/`show` contract, the enumerated doc-comment forms, the marker decision — are written as prose and links to the `gnd` repository, never as `§<ID>` citations. In the user's repo only IDs declared *in that repo* resolve with `gnd show`; an `agents.md` that tells the reader "run `gnd show <ID>`" and then cites `§FS-…` IDs that belong to `gnd` itself would have the reader chase a dangling reference. The `§<KIND>-<…>` tokens that do appear in the block are explicitly flagged as shape illustrations, not real IDs.

The managed block is wrapped in version markers:

```
<!-- gnd:init:agents:v1 begin -->
...
<!-- gnd:init:agents:v1 end -->
```

The integer after `v` is the `agents.md` block schema version. A fresh `agents.md` consists of this block. If `agents.md` already exists and contains no `gnd:init:agents` block, `init` appends the current block after the existing content. If the file already contains the current block version, `init` does not append another copy (and so a change to `.agents/gnd.toml` that would reflow the substituted ID-grammar text — §2.3 — does not propagate to an existing same-version block until `gnd init --force` is run, which rewrites `agents.md` from the template). If the file contains an older block version, `init` replaces only the bytes between the begin and end markers with the current block and leaves all content before and after the block untouched. If the file contains a newer block version than the running binary supports, `init` exits 2 and leaves the file unchanged.

#### 2.3.1 Position invariants

The managed block's **position within `agents.md` is preserved on update**. `init` does not move the block: a v0 block sandwiched between user-authored sections is replaced in place with the current v1 block, with the surrounding sections — both before and after — left byte-identical. The same rule applies to a current-version block: a v1 block in the middle of the file is recognized as already current and the file is not rewritten at all.

#### 2.3.2 Line endings

`init` does not normalize line endings. When updating or reading an existing `agents.md`:

- The bytes outside the managed block (everything before `<!-- gnd:init:agents:v… begin -->` and everything after `<!-- gnd:init:agents:v… end -->`) are preserved byte-for-byte, including CRLF (`\r\n`) and lone-CR endings.
- The block-recognition regex tolerates any whitespace (including `\r`) between the marker tokens, so a CRLF-encoded file with a v0 block is detected and updated correctly.
- The freshly-written block uses LF endings (the bytes embedded in the binary). On a CRLF-encoded host file the result is mixed line endings inside the managed region and CRLF outside; this is intentional. Normalizing the rest of the file would violate the "leave content alone" guarantee.

The generated file uses the canonical `gnd` reference grammar even before any IDs exist in the repo — citations of `§G-no-dangling-refs`, `§FS-check`, etc. inside the boilerplate point at the **gnd project's own** documentation, not the new repo's. This is intentional: the boilerplate is a teaching surface, and the IDs anchor the teaching to a stable source. The generated `.agents/gnd.toml` (§2.4) sets `[scan] include = ["docs", "e2e", "src"]` so the boilerplate's pedagogical citations in `agents.md` are not themselves scanned (the file lives at the repo root, outside `include`) — the citations remain inert text in the host repo and never flow into its findings.

### 2.4 Generated `.agents/gnd.toml`

Every key written matches the built-in default for that key. The file is a teaching surface, not an override surface — `init` writes the defaults explicitly so a new user can see the schema they will be editing. Top of file is `gnd_config_version = 1` per §FS-config.5. The schema written is exactly the one in §FS-config.3 (no extra keys, no missing keys).

A single `project_name = "<name>"` key appears at the top above the section tables. This key is metadata only — it is not consumed by any other `gnd` subcommand and exists so downstream tooling (IDE status bars, CI dashboards) can read it without re-deriving the name.

## 3. Refusing to clobber

Without `--force`, `init` never overwrites an existing file. Existing `agents.md` is handled by the append/update rules in §2.3. Every other existing target file is left unchanged and reported with `exists `:

```
exists .agents/gnd.toml
```

This makes repeated `gnd init` runs idempotent and safe for existing repos. With `--force`, existing files are overwritten in place; their previous contents are not preserved (the user has git for that, per §FS-non-goals.6 — `gnd` does not maintain its own history).

The `--docs` mode applies the same rule across the docs tree: existing scaffold files are reported as `exists ` and left unchanged; missing scaffold files are written.

## 4. Exit codes

- `0` — every requested file was written, appended, updated, or already current.
- `2` — I/O error (target path does not exist, permission denied, disk full, etc.); a CLI-level error such as `--append` and `--force` together (§1); or an existing `agents.md` contains a managed block whose schema version is newer than the running binary supports (§2.3), in which case the file is left unchanged.

Exit-code mapping is fixed per §G-friendliness-first.2 and §FS-non-goals.9.

## 5. Why this exists

Without `init`, the on-ramp to `gnd` is a copy-paste of someone else's `agents.md`, with the resulting drift between projects. `init` collapses the on-ramp to one command and freezes the canonical entry point at the `gnd` version that wrote it — when the canonical form evolves, `gnd init --force` re-emits it (§G-configurable still applies for downstream edits).

`init` is also the only safe place to demonstrate the ID grammar before any IDs exist: a repo that has not yet authored its first `FS-001-…` declaration still gets a literate `agents.md` from `init` that teaches the grammar by example.

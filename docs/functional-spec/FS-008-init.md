# FS-008-init: gnd bootstraps a new gnd-conformant repo

The `init` subcommand writes the minimum set of files a project needs to start using `gnd` — an `agents.md` entry point and a `gnd.toml` config — so that a fresh repo (or an existing repo adopting the scheme) becomes scannable in one command. Serves G-003-zero-config (the emitted defaults are the canonical grammar) and G-005-friendliness-first (no prompts, no surprises).

`init` is the only `gnd` subcommand that **writes new files into the working tree**. Every other subcommand reads, validates, or rewrites existing references; `init` creates the scaffolding those subcommands operate against.

## 1. Inputs

```
gnd init [<path>] [--name <name>] [--docs] [--force]
```

- `<path>` — directory in which to scaffold. Defaults to `.` (the current directory). Applies to every form of `init` — including `--docs` — and prefixes every emitted path in §2.1. Must exist; `init` does not create the target directory itself (a missing target is a user error, not something to silently paper over).
- `--name <name>` — human-readable project name baked into the generated `agents.md` heading and the `gnd.toml` `project_name` key. Defaults to the basename of `<path>` resolved to an absolute path.
- `--docs` — also scaffold the canonical `docs/` tree (empty `raison-detre.md`, `state-and-direction.md`, `goals/goals.md`, `functional-spec/`, `architectural-spec/`, `decisions/architectural/`, `decisions/functional/`) and an empty `e2e/` directory with a stub `README.md`. Off by default — most adopters already have a `docs/` of some shape and want only the entry point and config. Composes with `<path>`: every scaffolded file lands under `<path>/`.
- `--force` — overwrite files that already exist at the target paths. Off by default; without it, an existing file is an error (see §3).

Per FS-007-non-goals.10, `init` is non-interactive: it never prompts. Every choice is a flag.

### 1.1 Usage examples

```
gnd init                       # cwd, no docs tree
gnd init --docs                # cwd, with docs/ + e2e/ scaffolds
gnd init path/to/repo          # explicit target, no docs tree
gnd init --docs path/to/repo   # explicit target, with docs/ + e2e/ scaffolds
gnd init --docs --name acme path/to/repo  # full form
```

Argument order is flexible: positional `<path>` may appear before or after the flags.

## 2. Outputs

### 2.1 Files written

In the default form (no `--docs`):

- `<path>/agents.md` — see §2.3.
- `<path>/gnd.toml` — see §2.4.

With `--docs`, additionally:

- `<path>/docs/raison-detre.md`
- `<path>/docs/state-and-direction.md`
- `<path>/docs/goals/goals.md`
- `<path>/docs/functional-spec/.gitkeep`
- `<path>/docs/architectural-spec/.gitkeep`
- `<path>/docs/decisions/architectural/.gitkeep`
- `<path>/docs/decisions/functional/.gitkeep`
- `<path>/e2e/README.md`
- `<path>/e2e/cases/.gitkeep`

The `.gitkeep` files exist solely so the empty directories survive a `git add`. Their content is a single line: `# placeholder — replace this directory's contents with real declarations`.

### 2.2 Stdout / stderr

On success, stderr lists every path written, one per line, prefixed `wrote `:

```
wrote agents.md
wrote gnd.toml
```

Paths are relative to `<path>`. Stdout is always empty (consistent with G-005-friendliness-first.1.6 — output that other tools might pipe stays clean).

### 2.3 Generated `agents.md`

The emitted `agents.md` is the canonical entry point: it explains the ID grammar, points at the `docs/` layout, and lists the rules for agents (mirrors the rules in this repo's own `agents.md`). The `<name>` from `--name` is interpolated into the H1 and the opening sentence; everything else is fixed boilerplate. The boilerplate content is part of this spec — two `gnd init` runs at the same `gnd` version with the same `--name` produce byte-identical `agents.md`. (FS-007-non-goals.13.)

The generated file uses the canonical `gnd` reference grammar even before any IDs exist in the repo — citations of `G-001-no-dangling-refs`, `FS-001-check`, etc. inside the boilerplate point at the **gnd project's own** documentation, not the new repo's. This is intentional: the boilerplate is a teaching surface, and the IDs anchor the teaching to a stable source. The generated `gnd.toml` (§2.4) sets `[scan] include = ["docs", "e2e", "src"]` so the boilerplate's pedagogical citations in `agents.md` are not themselves scanned (the file lives at the repo root, outside `include`). See AS-004-authoring.1 for why this is safe.

### 2.4 Generated `gnd.toml`

Every key written matches the built-in default for that key. The file is a teaching surface, not an override surface — `init` writes the defaults explicitly so a new user can see the schema they will be editing. Top of file is `gnd_config_version = 1` per FS-006-config.5. The schema written is exactly the one in FS-006-config.3 (no extra keys, no missing keys).

A single `project_name = "<name>"` key appears at the top above the section tables. This key is metadata only — it is not consumed by any other `gnd` subcommand and exists so downstream tooling (IDE status bars, CI dashboards) can read it without re-deriving the name.

## 3. Refusing to clobber

If any target file already exists, `init` exits with code 1 and writes one error per existing file to stderr:

```
error: agents.md already exists (use --force to overwrite)
```

No file is written when any conflict exists — `init` is all-or-nothing. With `--force`, existing files are overwritten in place; their previous contents are not preserved (the user has git for that, per FS-007-non-goals.6 — `gnd` does not maintain its own history).

The `--docs` mode applies the same rule across the docs tree: if any one of the scaffold paths exists and `--force` is absent, none are written and every conflict is reported.

## 4. Exit codes

- `0` — every requested file was written.
- `1` — at least one conflict (file already exists, `--force` not given).
- `2` — I/O error (target path does not exist, permission denied, disk full, etc.).

Exit-code mapping is fixed per G-005-friendliness-first.2 and FS-007-non-goals.9.

## 5. Why this exists

Without `init`, the on-ramp to `gnd` is a copy-paste of someone else's `agents.md`, with the resulting drift between projects. `init` collapses the on-ramp to one command and freezes the canonical entry point at the `gnd` version that wrote it — when the canonical form evolves, `gnd init --force` re-emits it (G-006-configurable still applies for downstream edits).

`init` is also the only safe place to demonstrate the ID grammar before any IDs exist: a repo that has not yet authored its first `FS-001-…` declaration still gets a literate `agents.md` from `init` that teaches the grammar by example.

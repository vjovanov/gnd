# FS-init: grund bootstraps a new grund-conformant repo

The `init` subcommand writes the minimum set of files a project needs to start using `grund` — an `AGENTS.md` entry point at the repo root and a `.agents/grund.toml` config — so that a fresh repo (or an existing repo adopting the scheme) becomes scannable in one command. The config lives under `.agents/` per [§FS-config.1](FS-config.md#1-file-location-and-discovery), so the repo root stays free of `grund`-specific files. In a repo that already has `AGENTS.md`, `init` preserves the existing file and appends or updates a versioned `grund` block by default. If the repo already has known companion agent entrypoints, `init` makes those grund-aware too; for example, an existing `CLAUDE.md` that is not a symlink to `AGENTS.md` gets the same managed block. Serves [§GOAL-agent-grounding.1](../goals.md#1-the-three-layers) (the managed `grund` block is the instruction layer — an agent reading its entry-point file at session start arrives already taught), [§GOAL-zero-config](../goals.md#goal-zero-config-works-on-any-conformant-tree) (the emitted defaults are the canonical grammar), and [§GOAL-friendliness-first](../goals.md#goal-friendliness-first-as-user--and-agent-friendly-as-possible) (no prompts, no surprises).

`init` is the only `grund` subcommand that creates adoption scaffolding in the working tree. It may also update the managed `grund` block inside existing agent entrypoint files; it must not rewrite unrelated user-authored content in those files unless the file is the canonical `AGENTS.md` and `--force` is passed.

For verbose implementer fixtures — exact stderr transcripts, final tree expectations, and common existing-file cases — see [§FS-init-fixtures](FS-init-fixtures.md#fs-init-fixtures-concrete-init-fixtures). Those fixtures are examples of this spec, not a separate feature.

## 1. Inputs

```
grund init [<path>] [--name <name>] [--docs] [--force] [--append]
```

- `<path>` — directory in which to scaffold. Defaults to `.` (the current directory). Applies to every form of `init` — including `--docs` — and prefixes every emitted path in §2.1. Must exist; `init` does not create the target directory itself (a missing target is a user error, not something to silently paper over).
- `--name <name>` — human-readable project name baked into the generated `AGENTS.md` heading and the `.agents/grund.toml` `project_name` key. Defaults to the basename of `<path>` resolved to an absolute path.
- `--docs` — also scaffold the canonical `docs/` tree (stub `grund.md`, `goals.md`, `roadmap.md`, `changelog.md`, `functional-spec/`, `architecture/`, `decisions/architectural/`, `decisions/functional/`) and an empty `e2e/` directory with a stub `README.md`. The `roadmap.md` and `changelog.md` stubs are scaffolded because the generated `AGENTS.md` block's `docs/` table links to them (§2.3). Off by default — most adopters already have a `docs/` of some shape and want only the entry point and config. Composes with `<path>`: every scaffolded file lands under `<path>/`.
- `--force` — overwrite files that already exist at the target paths. Off by default. Mutually exclusive with `--append`: passing both is a CLI-level error and exits 2 without touching the working tree (per §4).
- `--append` — explicitly request the default `AGENTS.md` behavior: keep existing content and append or update the managed `grund` block. This flag is accepted for scripts that want to state intent, but it is not required.

Per [§FS-non-goals.10](FS-non-goals.md#10-interactive-mode), `init` is non-interactive: it never prompts. Every choice is a flag.

### 1.1 Usage examples

```
grund init                       # cwd, no docs tree
grund init path/to/repo          # appends/updates AGENTS.md if it already exists
grund init --docs                # cwd, with docs/ + e2e/ scaffolds
grund init --docs path/to/repo   # explicit target, with docs/ + e2e/ scaffolds
grund init --append path/to/repo # same AGENTS.md behavior as the default
grund init --docs --name acme path/to/repo  # full form
```

Argument order is flexible: positional `<path>` may appear before or after the flags.

## 2. Outputs

### 2.1 Files written, updated, or left in place

In the default form (no `--docs`):

- `<path>/AGENTS.md` — written when absent; appended or updated when present; see §2.3.
- Known companion agent entrypoints — currently `<path>/AGENTS.override.md`, `<path>/CLAUDE.md`, `<path>/.claude/CLAUDE.md`, `<path>/GEMINI.md`, and `<path>/.github/copilot-instructions.md` — are appended or updated when they already exist and are not symlinks to `AGENTS.md`; see §2.3. Missing companion aliases are created only when their owning agent-specific workspace already exists: `.claude/` creates `CLAUDE.md` and `.claude/CLAUDE.md`, and `.gemini/` creates `GEMINI.md`. `AGENTS.override.md` is an override file, and `.github/` is generic GitHub metadata rather than a Copilot-specific signal, so `AGENTS.override.md` and `.github/copilot-instructions.md` are still updated only when they already exist.
- `<path>/.agents/grund.toml` — see §2.4. The `.agents/` directory is created if missing.

With `--docs`, additionally:

- `<path>/docs/grund.md`
- `<path>/docs/goals.md`
- `<path>/docs/roadmap.md`
- `<path>/docs/changelog.md`
- `<path>/docs/functional-spec/README.md`
- `<path>/docs/architecture/README.md`
- `<path>/docs/decisions/architectural/.gitkeep`
- `<path>/docs/decisions/functional/.gitkeep`
- `<path>/e2e/README.md`
- `<path>/e2e/cases/.gitkeep`

Each scaffolded markdown file is a minimal starter — enough structure to teach the layout, no real content:

- `grund.md` — the H1 plus a one-line note on how the project's reason for being is declared inline (`# GND-NNN-slug: …`), then the three H2 sections (`## 1. The problem`, `## 2. What this project does about it`, `## 3. Who it is for`), each with a one-line italic prompt to be replaced.
- `goals.md` — the H1 plus a one-line note on how goals are declared inline (`# GOAL-NNN-slug: …`).
- `roadmap.md`, `changelog.md` — the H1 plus a single `<!-- placeholder - replace with real content -->` line.
- `functional-spec/README.md`, `architecture/README.md` — the H1, the navigational note about how `FS-`/`AR-` IDs declare into the directory and the convention that the index lists every spec, and an empty `| ID | Subject |` table to fill in.
- `e2e/README.md` — the H1 (`# e2e`) plus a one-line note that every behaviour under `docs/functional-spec/` has at least one case.

The exact bytes for a given `grund` version are embedded in the binary; reference copies live under `templates/` in the `grund` source tree, and two `grund init --docs` runs at the same version with the same `--name` produce byte-identical scaffolds ([§FS-non-goals.13](FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)). `grund check` is clean against the freshly-scaffolded tree. The `.gitkeep` files exist solely so the empty directories survive a `git add`; their content is a single line: `# placeholder — replace this directory's contents with real declarations`.

### 2.2 Stdout / stderr

On success, stderr lists every path touched, one per line:

```
wrote AGENTS.md
wrote .agents/grund.toml
```

The prefix is `wrote ` for a newly written or overwritten file, including a workspace-triggered companion alias; `appended ` when the versioned `grund` block was added to an existing agent entrypoint; `updated ` when an existing managed block's bytes changed (an older block upgraded, or a same-version block re-rendered against a changed template or config — §2.3); and `exists ` when an existing file was left unchanged — including an agent entrypoint whose managed block already matches the current rendered block byte-for-byte, in which case `init` rewrites nothing, so re-running `grund init` on an up-to-date repo touches no files. A companion file that is a symlink to `AGENTS.md` is omitted from the output because the canonical file update already covers it.

After the file list, stderr prints a short `next:` block — a blank line, then numbered first steps, then `see AGENTS.md for the full workflow.` — so the user is not left at a bare list of paths wondering what to do. The steps are: run `grund check` (a fresh tree is clean); allocate an ID with `ID=$(grund id FS "…")` and write `docs/functional-spec/$ID.md` with the `# <ID>: …` H1; cite it as `§<ID>` from the docs and e2e tests that depend on it. When `--docs` was *not* passed, step 2 instead points at re-running with `--docs` (or creating `docs/` and `e2e/` by hand). This block is guidance, not a finding; it is part of the success output and does not change the exit code.

Paths are relative to `<path>`. Stdout is always empty (consistent with [§GOAL-friendliness-first.1](../goals.md#1-hard-requirements) — output that other tools might pipe stays clean).

### 2.3 Generated agent entrypoints

The emitted agent guidance is a canonical managed block: it teaches the session-start workflow and rules listed in §2.3.4, rendered against the target repo's effective configuration. The block stays concise under [§GOAL-token-economy](../goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file): it teaches only the rules an agent needs before work begins, leaving detail to cited specs and `grund show` output. The canonical text for a given block version `vN` is embedded in the `grund` binary; the reference copy lives at `templates/AGENTS.md` in the `grund` source tree, and the `vN` marker (§2.3) is what versions it under [§GOAL-no-silent-breakage](../goals.md#goal-no-silent-breakage-changes-ship-through-a-deprecation-path) — so changing the taught workflow is itself a block-version bump, carried by that mechanism, not a silent rewrite.

Several things in the block are *substituted in* rather than fixed for that `vN`, so the file describes the repo it is in: the project name from `--name` (interpolated into the scaffolding H1 emitted above the block for a fresh `AGENTS.md`), and the **effective ID grammar and artifact map** — taken from the `.agents/grund.toml` `init` leaves governing the target (an existing config in the target, or the defaults `init` is about to write, never an ancestor's). From that config the block fills in the ID shape (`<KIND>-<NNN>-<slug>`, `<KIND>-<slug>`, …, derived from `[id].format`), one worked example ID and citation, the `[id].section_separator`, the marker and `$$`-trigger from `[reference]`, the `KIND ∈ {…}` set from `[[kinds]]`, a raw-readable link list of each kind's configured declaration home and title, and a sentence on whether bare ID-shaped tokens count as citations (driven by `[reference].strict`). The contract this spec makes is the *determinism and versioning*, not a literal transcript: two `grund init` runs at the same `grund` version against trees with the same `--name` and the same effective config produce byte-identical managed blocks ([§FS-non-goals.13](FS-non-goals.md#13-anything-that-would-let-two-grund-installs-disagree)), and `grund check`'s `AGENTS.md` validation ([§FS-check.3.5](FS-check.md#35-invalid-agent-entrypoint-init-block)) checks the marker line and the version, not a byte-diff against the canonical text.

The managed block is an H2 section whose heading carries the schema version:

```markdown
## Grounding with grund (vN)
…
```

The integer after `v` is the agent guidance block schema version. The heading line is the block's begin marker; the block runs until the next H1 or H2 heading, or end of file. A fresh `AGENTS.md` consists of this block preceded by a one-line scaffolding H1 (`# {NAME} — agent instructions`); the H1 is *unmanaged*, so `init --force` rewrites the block but leaves the title alone. If `AGENTS.md` already exists and contains no managed block, `init` appends the current block after the existing content (no H1 is inserted — the host file owns its title). If a known companion entrypoint such as `AGENTS.override.md`, `CLAUDE.md`, `.claude/CLAUDE.md`, `GEMINI.md`, or `.github/copilot-instructions.md` exists and is not a symlink to `AGENTS.md`, the same append/update rules apply to that companion. If a missing neutral alias is created because its workspace exists (§2.1), `init` writes just the managed block, with no extra H1, so the agent-specific file remains a thin alias for the canonical workflow. If the file already contains any supported block version, including the current one, `init` re-renders the block from the current effective `.agents/grund.toml`, compares it to the bytes between the marker line and the next H1/H2 (or EOF), and — when they differ — replaces only those bytes, leaving all content before and after the block untouched (reported `updated `). This means same-version template/config changes propagate on the next `grund init` without requiring `--force`. When the re-render is byte-identical to what is on disk, `init` writes nothing and reports the file with `exists ` (§2.2) — re-running `grund init` on an already-current repo is a no-op on every file. If the file contains a newer block version than the running binary supports, `init` exits 2 and leaves the file unchanged.

`AGENTS.md` is the canonical entrypoint and is always created or maintained. Companion entrypoints are mostly discovery-based: `init` updates them if the repo already has them, and creates the neutral aliases for Claude or Gemini only when the matching agent-specific workspace directory already shows that tool is in use. The built-in companion set covers common root or repository instruction files used by agent-specific tooling: Codex override instructions (`AGENTS.override.md`), Claude Code (`CLAUDE.md`, `.claude/CLAUDE.md`), Gemini (`GEMINI.md`), and GitHub Copilot (`.github/copilot-instructions.md`). Since `.github/` is also used for Actions, issue templates, and other non-Copilot metadata, `init` does not create `.github/copilot-instructions.md` from directory existence alone. When one of those paths is a symlink to `AGENTS.md`, `init` does not touch it separately and `grund check` treats the canonical `AGENTS.md` block as sufficient.

#### 2.3.1 Position invariants

The managed block's **position within an existing agent entrypoint is preserved on update**. `init` does not move the block: a supported block sandwiched between user-authored sections is replaced in place with the current rendered block, with the surrounding sections — both before and after — left byte-identical. When the block is already byte-identical to the current render, nothing is rewritten at all (`exists `, §2.2) — the strongest form of "position preserved."

#### 2.3.2 Line endings

`init` does not normalize line endings. When updating or reading an existing agent entrypoint:

- The bytes outside the managed block (everything before the `## Grounding with grund (vN)` heading line, and everything after the next H1 or H2) are preserved byte-for-byte, including CRLF (`\r\n`) and lone-CR endings.
- The H2 marker regex tolerates an optional trailing `\r` before the line end, so a CRLF-encoded file is detected correctly.
- The freshly-written block uses LF endings (the bytes embedded in the binary). On a CRLF-encoded host file the result is mixed line endings inside the managed region and CRLF outside; this is intentional. Normalizing the rest of the file would violate the "leave content alone" guarantee.

#### 2.3.3 Citation form

The managed block teaches a single citation form — `§<ID>`, bare, with an optional `.<section>` path. `[fmt.cross_refs].enabled` per [§DF-md-link-emission.2.4](../decisions/functional/DF-md-link-emission.md#24-opt-in-never-default) governs whether `grund fmt` wraps on-disk citations in Markdown links; the managed block's guidance is the same in either mode — cite specs by ID; any Markdown wrap is generated by `grund fmt`, not hand-authored.

The generated file uses the canonical `grund` reference grammar even before any IDs exist in the repo — citations of `§GOAL-no-dangling-refs`, `§FS-check`, etc. inside the boilerplate point at the **grund project's own** documentation, not the new repo's. This is intentional: the boilerplate is a teaching surface, and the IDs anchor the teaching to a stable source. The generated `.agents/grund.toml` (§2.4) sets `[scan] include = ["docs", "e2e", "src"]` so the boilerplate's pedagogical citations in `AGENTS.md` are not themselves scanned (the file lives at the repo root, outside `include`) — the citations remain inert text in the host repo and never flow into its findings.

#### 2.3.4 Managed-block content points

The generated `AGENTS.md` block is not a literal transcript in this spec, but its canonical template must preserve the following separately citeable content points. Each point is phrased compactly in the template, because the entrypoint is read at session start and serves [§GOAL-token-economy](../goals.md#goal-token-economy-give-an-agent-the-right-amount-of-spec-not-the-whole-file).

##### 2.3.4.1 Entrypoint

The block identifies itself as the entrypoint for agents working in the project and instructs them to read it, then read the declared artifacts it points to, before making changes.

##### 2.3.4.2 Reference Scheme

The block teaches the configured reference scheme: IDs have the configured shape, citations use the configured marker and optional section path, `grund check` validates citations, `grund list` discovers IDs, `grund show` reads cited declarations or sections, and `grund refs` reports what leans on a declaration.

##### 2.3.4.3 Cheap Grounding

The block teaches the cheap grounding ladder: use `grund show <ID>` as the first read for a bare citation, `grund show <ID> --toc` when section navigation is needed, `grund show <ID>.<section>` for section citations, `grund show <ID> --full` only when a narrower read is insufficient, `grund list --kind FS,AR` for scoped discovery, and `grund refs <ID> --summary` before a full back-reference listing.

##### 2.3.4.4 Project Map

The block describes the configured declaration homes from the effective `.agents/grund.toml` as a raw-readable Markdown link list (`- [KIND](home): Title`), so the generated instructions name and link the host repo's actual artifact layout instead of hard-coding the default layout or relying on table rendering. The scan scope (`[scan].include` / `[scan].exclude`) is *not* surfaced here — it is configuration an agent never needs to read inline, since `grund show`, `grund list`, and `grund refs` apply it transparently.

##### 2.3.4.5 Declaration Forms

The block teaches that declarations are heading lines in markdown or supported language doc-comments, and that an inline source declaration can be represented by a one-line markdown stub in the configured kind home.

##### 2.3.4.6 Spec First

The rules tell agents to write or update the most-specific functional or architectural spec point before writing behavior or design code that implements it.

##### 2.3.4.7 Most-Specific Citations

The code back-reference guidance tells agents to cite the most-specific spec point the code or prose realizes: whole behavior on the function, class, or block doc-comment; narrower clauses or decisions inline where they are enforced.

##### 2.3.4.8 Refresh Before Editing

The rules tell agents to refresh the cited spec with `grund show` before editing code that already carries a `§<ID>` or `§<ID>.<section>` citation.

##### 2.3.4.9 Declaration Blast Radius

The rules tell agents to run `grund refs <ID> --summary` before changing, moving, or renaming a declaration, and to use the full `grund refs <ID>` output when exact citation sites are needed.

##### 2.3.4.10 Citation Direction

The rules teach the expected citation direction: specs cite goals, architecture cites specs, code cites the specs it implements, and executable tests or cases cite the behavior they verify.

##### 2.3.4.11 Decisions

The rules tell agents that decisions must be cited from the spec or architecture point they shaped, and that decision history is append-only: reversals are new decisions that supersede older ones rather than rewrites.

##### 2.3.4.12 Cross-Linking

The block tells agents to cross-link specs only via IDs — a single short sentence in the citations narrative, not phrased as a rule. The `[fmt.cross_refs].enabled` mode (§2.3.3) does not change this wording.

##### 2.3.4.13 Executable Proof

The rules tell agents that behavior is proven by executable tests or cases, and that disagreements between the spec and executable proof require fixing both in the same change.

##### 2.3.4.14 Final Check

The rules tell agents to run `grund check` before committing, because dangling references are stop-the-line bugs whose diagnostics name the file and line.

### 2.4 Generated `.agents/grund.toml`

`init` writes this file **only when it is absent**. An existing `.agents/grund.toml` is the repo's configuration — the one surface a project customizes ([§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable)) — and `init` never overwrites it, not even with `--force`: an existing config is reported as `exists` and left byte-for-byte unchanged (§3). `--force` resets the things `init` owns end to end — the managed `AGENTS.md` block and the `--docs` scaffold stubs — not the user's settings; a customized config that `init --force` clobbered would be a footgun. (When the file is absent and `init` does write it, every key written matches the built-in default for that key — the file is a teaching surface, not an override surface, so a new user can see the schema they will be editing. Top of file is `grund_config_version = 1` per [§FS-config.5](FS-config.md#5-schema-versioning); the schema written is exactly the one in [§FS-config.3](FS-config.md#3-schema), no extra keys, no missing keys.)

A single `project_name = "<name>"` key appears at the top above the section tables. This key is metadata only — it is not consumed by any other `grund` subcommand and exists so downstream tooling (IDE status bars, CI dashboards) can read it without re-deriving the name.

## 3. Refusing to clobber

Without `--force`, `init` never overwrites an existing file. Existing agent entrypoints are handled by the append/update rules in §2.3. Every other existing target file is left unchanged and reported with `exists `:

```
exists .agents/grund.toml
```

This makes repeated `grund init` runs idempotent and safe for existing repos. With `--force`, `AGENTS.md` and the `--docs` scaffold files are overwritten in place; their previous contents are not preserved (the user has git for that, per [§FS-non-goals.6](FS-non-goals.md#6-decision-database-audit-log-history-tracking) — `grund` does not maintain its own history). **`.agents/grund.toml` is the exception: `--force` never overwrites it.** Once the config exists it is the project's, not `init`'s — `init --force` still reports it as `exists ` and leaves it untouched (§2.4); the file is only ever written when it is absent. Companion agent entrypoints are likewise not full-file overwritten by `--force`; if present and not symlinked to `AGENTS.md`, only their managed block is appended or updated so unrelated agent-specific instructions remain intact.

The `--docs` mode applies the same rule across the docs tree: existing scaffold files are reported as `exists ` and left unchanged; missing scaffold files are written.

## 4. Exit codes

- `0` — every requested file was written, appended, updated, or already current.
- `2` — I/O error (target path does not exist, permission denied, disk full, etc.); a CLI-level error such as `--append` and `--force` together (§1); or an existing `AGENTS.md` contains a managed block whose schema version is newer than the running binary supports (§2.3), in which case the file is left unchanged.

Exit-code mapping is fixed per [§GOAL-friendliness-first.2](../goals.md#2-what-this-rules-out) and [§FS-non-goals.9](FS-non-goals.md#9-severity-exit-code-or-report-ordering-customization).

## 5. Agent setup instructions

`grund agent-setup-instructions` prints the AI-agent-facing guided setup instructions for adopting `grund` in an arbitrary repo. This is a read-only discovery command: stdout is Markdown, stderr is empty, exit `0`; passing any positional argument or flag is a CLI-level error (`error: agent-setup-instructions takes no arguments`, exit `2`). The command exists so a user can tell an agent only "set up grund" plus provide an installed `grund` binary, and the agent can still discover the recommended setup workflow without browsing the source repository.

The output reads like an agent skill rather than a human tutorial. It instructs the agent to inspect the target repo first, identify the existing specs, artifact types, roadmaps, changelogs, decisions, plans, tests, and agent instruction files, recommend suitable `grund init` and `.agents/grund.toml` choices with evidence, show pros and cons for every config option, ask the user to confirm or override the recommendations, write `.agents/grund.toml`, run `grund init`, then validate with `grund config validate` and `grund check`. The config must be written before `grund init` so the generated managed block reflects the selected ID grammar, marker, strict mode, kinds, and existing artifact layout. For an existing docs-heavy repo, the recommended `grund init` form omits `--docs`; existing specs are represented in `[[kinds]]` and `[scan]` settings rather than replaced by generic scaffold folders.

The core instructions must not fork from the distributable skill. The binary embeds `skills/grund-init/SKILL.md` and `grund agent-setup-instructions` prints that same Markdown source byte-for-byte. The source package therefore exposes the instructions in two ways with one body: agents that can read the repository may load `skills/grund-init/SKILL.md` as a skill; agents that only have the installed CLI may run `grund agent-setup-instructions`. A release that edits one surface without the other is invalid.

## 6. Why this exists

Without `init`, the on-ramp to `grund` is a copy-paste of someone else's `AGENTS.md`, with the resulting drift between projects. `init` collapses the on-ramp to one command and freezes the canonical entry point at the `grund` version that wrote it — when the canonical form evolves, `grund init --force` re-emits it ([§GOAL-configurable](../goals.md#goal-configurable-every-default-is-overridable) still applies for downstream edits).

`init` is also the only safe place to demonstrate the ID grammar before any IDs exist: a repo that has not yet authored its first `FS-001-…` declaration still gets a literate `AGENTS.md` from `init` that teaches the grammar by example.

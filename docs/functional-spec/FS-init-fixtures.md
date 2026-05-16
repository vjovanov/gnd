# FS-init-fixtures: concrete init fixtures

This file is the verbose fixture companion to [§FS-init](FS-init.md#fs-init-grund-bootstraps-a-new-grund-conformant-repo). It does not add a new command; it gives implementers concrete states and transcripts for the `init` behavior that [§FS-init](FS-init.md#fs-init-grund-bootstraps-a-new-grund-conformant-repo) defines in prose.

The fixtures use `{repo}` for an existing target directory and `{repo_copy}` for a mutable copy of that directory. Every success case has empty stdout. Every path printed by `init` is relative to the target directory. The stderr blocks below are exact apart from the placeholder path in the missing-target case.

## 1. Default form

Command:

```text
grund init {repo_copy}
```

Precondition: `{repo_copy}` exists and contains no `AGENTS.md` and no `.agents/grund.toml`.

Exit `0`, stdout empty, stderr:

```text
wrote AGENTS.md
wrote .agents/grund.toml

next:
  1. re-run with --docs to scaffold docs/ and e2e/ (or create those folders yourself) — until then `grund check` has nothing to scan
  2. run `grund check` — a scaffolded tree is clean
  3. allocate an ID:  ID=$(grund id FS "…")  then write  docs/functional-spec/$ID.md
see AGENTS.md for the full workflow.
```

Final tree:

```text
AGENTS.md
.agents/grund.toml
```

`AGENTS.md` contains exactly one H2-managed block headed `## Grounding with grund (v1)`. `.agents/grund.toml` is the default generated config from [§FS-init.2.4](FS-init.md#24-generated-agentsgrundtoml), including `grund_config_version = 1`, `project_name`, `[reference]`, `[id]`, every default `[[kinds]]`, `[scan]`, `[output]`, and `[fmt.cross_refs]`.

## 2. Docs form

Command:

```text
grund init {repo_copy} --docs
```

Precondition: `{repo_copy}` exists and contains no generated files.

Exit `0`, stdout empty, stderr:

```text
wrote AGENTS.md
wrote .agents/grund.toml
wrote docs/grund.md
wrote docs/goals.md
wrote docs/roadmap.md
wrote docs/changelog.md
wrote docs/functional-spec/README.md
wrote docs/architecture/README.md
wrote docs/decisions/architectural/.gitkeep
wrote docs/decisions/functional/.gitkeep
wrote e2e/README.md
wrote e2e/cases/.gitkeep

next:
  1. run `grund check` — a freshly scaffolded tree is clean
  2. allocate an ID:  ID=$(grund id FS "…")  then write  docs/functional-spec/$ID.md
     (H1: `# <ID>: <one-line statement of the behavior>`)
  3. cite it as §<ID> from the docs and e2e tests that depend on it, then `grund check` again
see AGENTS.md for the full workflow.
```

Final tree includes the default-form files plus exactly these docs/e2e scaffold paths:

```text
docs/grund.md
docs/goals.md
docs/roadmap.md
docs/changelog.md
docs/functional-spec/README.md
docs/architecture/README.md
docs/decisions/architectural/.gitkeep
docs/decisions/functional/.gitkeep
e2e/README.md
e2e/cases/.gitkeep
```

## 3. Existing files

When the managed block and config are already current, `init` is a no-op on disk:

```text
exists AGENTS.md
exists .agents/grund.toml

next:
  1. re-run with --docs to scaffold docs/ and e2e/ (or create those folders yourself) — until then `grund check` has nothing to scan
  2. run `grund check` — a scaffolded tree is clean
  3. allocate an ID:  ID=$(grund id FS "…")  then write  docs/functional-spec/$ID.md
see AGENTS.md for the full workflow.
```

When `AGENTS.md` exists without a managed block and `--force` is not passed, `init` appends the block and reports:

```text
appended AGENTS.md
wrote .agents/grund.toml
```

When `AGENTS.md` exists and `--force` is passed, `init` rewrites the canonical file and reports `wrote AGENTS.md`. When `.agents/grund.toml` already exists, `init --force` still preserves it and reports `exists .agents/grund.toml`; config is never clobbered once present.

## 4. Target and flag failures

A missing target directory is a CLI-level failure: exit `2`, stdout empty, stderr:

```text
error: target directory does not exist: <path>
```

Passing `--force` and `--append` together is a CLI-level failure: exit `2`, stdout empty, stderr:

```text
error: --force and --append are mutually exclusive
```

These failures leave the target tree unchanged.

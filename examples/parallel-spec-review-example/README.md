# Parallel Spec Review Example

This is the checked-in rendered example for the `parallel-spec-review`
template.

## Values Used

```yaml
spec: docs/functional-spec/FS-check.md
review_focus: Check consistency with CLI error handling and examples.
iterations: 2
```

All other inputs use the template defaults:

- reviewers: Claude, Codex, Gemini
- summary agent: Claude
- fix agent: Codex
- review output directory: `runtime/reviews`
- summary output directory: `runtime/summaries`
- fix output directory: `runtime/fixes`

## Validate

```bash
rhei validate examples/parallel-spec-review-example
```

## Dry Run

Use `--parallel 3` to exercise the reviewer fan-out:

```bash
rhei run examples/parallel-spec-review-example --dry-run --parallel 3
```

## Regenerate

```bash
rhei instantiate parallel-spec-review \
  --set spec=docs/functional-spec/FS-check.md \
  --set review_focus="Check consistency with CLI error handling and examples." \
  --set iterations=2 \
  --output examples/parallel-spec-review-example
```

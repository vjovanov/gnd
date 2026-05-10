# Scheme: `{kind}-{number}`

Pure-numeric IDs. Familiar to anyone who has read an RFC or a JEP.

```toml
[id]
format = "{kind}-{number}"
```

Example IDs:

```
RFC-001
FS-002
AS-014
```

## Pros

- **Shortest possible ID.** Citations stay tight in prose: `§RFC-001`, `§FS-002.1`.
- **Title-edit safe.** A spec's heading can be reworded indefinitely without disturbing any existing citation — the ID has no descriptive payload to drift.
- **Easy to allocate.** `gnd name FS "..."` just emits the next free number; no slug derivation, no collision check on the descriptive part.
- **Familiar.** Reviewers already trained on RFC-/JEP-/PEP-style identifiers feel at home.

## Cons

- **Opaque in prose.** A reader skimming code or a PR sees `§FS-042` and has no idea what the claim is about until they `gnd show`. This punishes drive-by review.
- **Memory load.** Maintainers learn the catalog by number; new contributors don't have that map.
- **Search friction.** Grepping `§FS-` finds every cite uniformly, with no descriptive hint to triage.

## Verify

```bash
gnd examples/scheme-numbered/repo
echo $?    # 0
```

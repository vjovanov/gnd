# Examples

Self-contained mini-repos demonstrating each ID scheme `gnd` supports.
Each subfolder is shaped like an `e2e/cases/<name>/` directory — `repo/`
holds the fixture, and `expected.exit`/`expected.stdout`/`expected.stderr`
record the contract — so each example doubles as a regression fixture.
`tests/examples.rs` runs `gnd <repo>` against every one of them on `cargo
test`, so the snippets below cannot drift from what the tool actually does.

## ID schemes

| Folder                                                       | `[id] format`             | Example IDs                |
|--------------------------------------------------------------|---------------------------|----------------------------|
| [`scheme-numbered-slug/`](scheme-numbered-slug/)             | `{kind}-{number}-{slug}`  | `FS-001-login`             |
| [`scheme-numbered/`](scheme-numbered/)                       | `{kind}-{number}`         | `RFC-001`, `FS-002`        |
| [`scheme-slug/`](scheme-slug/)                               | `{kind}-{slug}`           | `FS-login`, `AS-event-bus` |

Each subfolder's `README.md` lists the trade-offs for that scheme. The
top-level project [README](../README.md#supported-id-schemes) summarizes
when to reach for each.

## Run an example

From the repo root, with a built `gnd` binary on `$PATH` (or invoked
via `cargo run --`):

```bash
gnd examples/scheme-slug/repo
echo $?    # 0
```

A passing scheme prints `success` on stdout and exits 0.

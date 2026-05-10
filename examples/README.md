# Examples

Self-contained mini-repos demonstrating each ID scheme `gnd` supports.
Each subfolder is shaped like an `e2e/cases/<name>/` directory — `repo/`
holds the fixture, and `expected.exit`/`expected.stdout`/`expected.stderr`
record the contract — so the same example doubles as a regression
fixture if you want to wire it into the `tests/e2e.rs` discovery list.

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

A passing scheme prints nothing on stdout and exits 0.

## Other examples

`parallel-spec-review-example/` is unrelated to ID schemes — it is a
checked-in render of the Rhei `parallel-spec-review` template that
happens to live alongside this repo's docs.

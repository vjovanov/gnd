# grund-core

`grund-core` is the shared Rust engine behind [`grund`](https://crates.io/crates/grund).
It contains the scanner, resolver, checker, formatter, config loader, structured
`check` / `show` APIs, and compatibility command adapters used by the CLI while
the remaining subcommand APIs are split out. This package role follows
[§FS-distribution.1](../../docs/functional-spec/FS-distribution.md#1-targets).

Most users should install the CLI crate instead:

```bash
cargo install grund
```

Use `grund-core` directly when embedding the engine in Rust code. The public API
is still pre-1.0 and may change between minor releases.

Project home: <https://github.com/vjovanov/grund>

use std::process::ExitCode;

// The `grund` CLI binary is a thin shell over the `grund` library crate — all logic
// lives behind `main_entry` so the same engine can back the LSP and the language
// bindings (§AS-bindings.3, §AS-bindings.2).
fn main() -> ExitCode {
    grund::main_entry()
}

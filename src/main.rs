use std::process::ExitCode;

// The `grund` CLI binary is a thin shell over the `grund-core` crate so the
// same engine can back the LSP and language bindings (§AR-bindings.1).
fn main() -> ExitCode {
    grund_core::main_entry()
}

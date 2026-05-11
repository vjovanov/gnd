//! Instruction-counting benchmarks for the `gnd` CLI — see
//! `docs/architectural-spec/AS-benchmarks.md`.
//!
//! Each benchmark runs the freshly built `gnd` binary under Callgrind via
//! `iai-callgrind`, so the reported figure is the deterministic instruction
//! count for an actual CLI invocation against this repository's conformant
//! tree. Instruction counts (unlike wall-clock time) do not flake on a loaded
//! CI runner, which is what makes "track the number across commits and fail on
//! regression" implementable — the meter the G-fast-feedback budget asks for.
//!
//! The benched subcommands are the ones agents and CI invoke on every loop:
//!
//! - `check` — every save / commit / push; the headline operation.
//! - `list` / `show` / `refs` — the agent grounding workflow (discover IDs,
//!   read a declaration body, see a declaration's blast radius).
//! - `cover` — what the co-change recipe / CI consume.
//! - `fmt --check` — the pre-commit / CI normalization gate.
//!
//! Run with `cargo bench` (requires Valgrind and `iai-callgrind-runner` on
//! `PATH`; CI installs both — see `.github/workflows/ci.yml`).

#[cfg(feature = "bench")]
use iai_callgrind::{Command, binary_benchmark, binary_benchmark_group, main};

/// The freshly built `gnd` binary under test (Cargo exports this env var).
#[cfg(feature = "bench")]
const GND: &str = env!("CARGO_BIN_EXE_gnd");
/// This repository's root — the conformant tree every benchmark scans, matching
/// the `gnd .` self-host loop CI already runs.
#[cfg(feature = "bench")]
const REPO: &str = env!("CARGO_MANIFEST_DIR");

/// A representative declared ID with a substantial body — what `gnd show` reads
/// when an agent grounds itself before editing.
#[cfg(feature = "bench")]
const SHOW_ID: &str = "FS-check";
/// A heavily-cited goal — `gnd refs` over it walks the whole tree and returns
/// the blast radius an agent checks before changing a declaration.
#[cfg(feature = "bench")]
const REFS_ID: &str = "G-fast-feedback";

// `gnd check <repo>` — validate every citation in the tree.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn check() -> Command {
    Command::new(GND).args(["check", REPO]).build()
}

// `gnd list <repo>` — every declared ID.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn list() -> Command {
    Command::new(GND).args(["list", REPO]).build()
}

// `gnd show <ID> <repo>` — one declaration body.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn show() -> Command {
    Command::new(GND).args(["show", SHOW_ID, REPO]).build()
}

// `gnd refs <ID> <repo>` — every citation site of an ID.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn refs() -> Command {
    Command::new(GND).args(["refs", REFS_ID, REPO]).build()
}

// `gnd cover <repo>` — the citation graph grouped by scanned file.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn cover() -> Command {
    Command::new(GND).args(["cover", REPO]).build()
}

// `gnd fmt --check <repo>` — report (without writing) any non-canonical citation.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn fmt_check() -> Command {
    Command::new(GND).args(["fmt", "--check", REPO]).build()
}

#[cfg(feature = "bench")]
binary_benchmark_group!(
    name = commands;
    benchmarks = check, list, show, refs, cover, fmt_check
);

#[cfg(feature = "bench")]
main!(binary_benchmark_groups = commands);

#[cfg(not(feature = "bench"))]
fn main() {}

// §AR-benchmarks: instruction-counting benches cover the hot CLI commands.
//
// Each benchmark runs the freshly built `grund` binary under Callgrind via
// `iai-callgrind`, so the reported figure is the deterministic instruction
// count for an actual CLI invocation against this repository's conformant
// tree. Instruction counts (unlike wall-clock time) do not flake on a loaded
// CI runner, which is what makes "track the number across commits and fail on
// regression" implementable.
//
// The benched subcommands are the ones agents and CI invoke on every loop:
// `check`, `list`, the `show` ladder, `refs`, `cover`, and `fmt --check`.
//
// Each command exits 0 on this repository's conformant tree, so iai-callgrind
// is happy. On a broken tree, `check` / `fmt --check` exit non-zero and the
// bench fails; a baseline recorded against a broken tree is worthless.
//
// The self-repo command list also drives the release/benchmark PGO training run
// in `scripts/pgo-build.sh`; keep those hot commands in sync. Run with
// `cargo bench -p grund --features bench --bench instructions` (requires Valgrind and
// `iai-callgrind-runner` on `PATH`).

#[cfg(feature = "bench")]
use iai_callgrind::{Command, binary_benchmark, binary_benchmark_group, main};
#[cfg(feature = "bench")]
use std::{
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};

/// The freshly built `grund` binary under test (Cargo exports this env var).
#[cfg(feature = "bench")]
const GRUND: &str = env!("CARGO_BIN_EXE_grund");
/// This repository's root — the conformant tree every benchmark scans, matching
/// the `grund .` self-host loop CI already runs.
#[cfg(feature = "bench")]
const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
/// Generated large conformant fixture for the `grund check` 10k-file budget.
#[cfg(feature = "bench")]
const LARGE_REPO_REL: &str = "target/bench-fixtures/large-conformant-repo";
#[cfg(feature = "bench")]
const LARGE_REPO_FILE_COUNT: usize = 10_000;

/// A representative declared ID with a substantial body — enough to exercise
/// the brief preview, lead-default read, and full recursive body in the show
/// ladder when an agent grounds itself before editing.
#[cfg(feature = "bench")]
const SHOW_ID: &str = "FS-check";
/// A heavily-cited goal — `grund refs` over it walks the whole tree and returns
/// the blast radius an agent checks before changing a declaration.
#[cfg(feature = "bench")]
const REFS_ID: &str = "GOAL-fast-feedback";

#[cfg(feature = "bench")]
fn large_repo() -> PathBuf {
    Path::new(REPO).join(LARGE_REPO_REL)
}

#[cfg(feature = "bench")]
fn ensure_large_fixture() -> PathBuf {
    let root = large_repo();
    let script = Path::new(REPO).join("scripts/generate_large_benchmark_fixture.py");
    let status = ProcessCommand::new("python3")
        .arg(script)
        .arg("--root")
        .arg(&root)
        .arg("--files")
        .arg(LARGE_REPO_FILE_COUNT.to_string())
        .status()
        .expect("run large benchmark fixture generator");
    assert!(status.success(), "large benchmark fixture generator failed");
    root
}

// `grund check <repo>` — validate every citation in the tree.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn check() -> Command {
    Command::new(GRUND).args(["check", REPO]).build()
}

// `grund check <large-repo>` — the 10k-file budget input.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn check_large_10k() -> Command {
    let root = ensure_large_fixture();
    Command::new(GRUND).arg("check").arg(root).build()
}

// `grund list <repo>` — every declared ID.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn list() -> Command {
    Command::new(GRUND).args(["list", REPO]).build()
}

// `grund <ID> --brief <repo>` — title plus first paragraph.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn show_brief() -> Command {
    Command::new(GRUND)
        .args(["show", SHOW_ID, "--brief", REPO])
        .build()
}

// `grund <ID> <repo>` — the lead-default declaration read.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn show() -> Command {
    Command::new(GRUND).args(["show", SHOW_ID, REPO]).build()
}

// `grund <ID> --full <repo>` — one full declaration body.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn show_full() -> Command {
    Command::new(GRUND)
        .args(["show", SHOW_ID, "--full", REPO])
        .build()
}

// `grund refs <ID> <repo>` — every citation site of an ID.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn refs() -> Command {
    Command::new(GRUND).args(["refs", REFS_ID, REPO]).build()
}

// `grund cover <repo>` — the citation graph grouped by scanned file.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn cover() -> Command {
    Command::new(GRUND).args(["cover", REPO]).build()
}

// `grund fmt --check <repo>` — report (without writing) any non-canonical citation.
#[cfg(feature = "bench")]
#[binary_benchmark]
fn fmt_check() -> Command {
    Command::new(GRUND).args(["fmt", "--check", REPO]).build()
}

#[cfg(feature = "bench")]
binary_benchmark_group!(
    name = commands;
    benchmarks = check, check_large_10k, list, show_brief, show, show_full, refs, cover, fmt_check
);

#[cfg(feature = "bench")]
main!(binary_benchmark_groups = commands);

#[cfg(not(feature = "bench"))]
fn main() {}

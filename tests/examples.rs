//! Regression coverage for the runnable mini-repos under `examples/`.
//!
//! `README.md` and `examples/README.md` advertise each `examples/<scheme>/repo`
//! as a self-contained tree that `gnd <path>` validates, with golden
//! `expected.exit` / `expected.stdout` / `expected.stderr` files alongside it.
//! This test runs `gnd` against every such directory and asserts the golden
//! contract, so the advertised examples cannot silently rot. It uses the lighter
//! contract the examples carry (no `spec.refs`, no `command.args`): each example
//! is exercised by the bare `gnd <repo>` form documented in the READMEs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn examples_match_expected_reports() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let examples_dir = manifest_dir.join("examples");
    let cases = discover_examples(&examples_dir);
    assert!(
        !cases.is_empty(),
        "expected at least one runnable example under {}",
        examples_dir.display()
    );
    for case in cases {
        run_example(&manifest_dir, &case);
    }
}

fn discover_examples(examples_dir: &Path) -> Vec<PathBuf> {
    let mut cases = fs::read_dir(examples_dir)
        .unwrap_or_else(|err| panic!("read {}: {err}", examples_dir.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir() && path.join("expected.exit").is_file())
        .collect::<Vec<_>>();
    cases.sort();
    cases
}

fn run_example(manifest_dir: &Path, case: &Path) {
    let name = case
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("<invalid example name>");
    let repo = case.join("repo");
    assert!(
        repo.is_dir(),
        "{name}: example is missing its repo/ fixture directory"
    );
    let repo_arg = repo
        .strip_prefix(manifest_dir)
        .unwrap_or(&repo)
        .to_string_lossy()
        .into_owned();

    let output = Command::new(env!("CARGO_BIN_EXE_gnd"))
        .arg(&repo_arg)
        .current_dir(manifest_dir)
        .output()
        .unwrap_or_else(|err| panic!("{name}: run gnd: {err}"));

    let actual_exit = output.status.code().unwrap_or(-1);
    let actual_stdout = String::from_utf8(output.stdout)
        .unwrap_or_else(|err| panic!("{name}: stdout was not UTF-8: {err}"));
    let actual_stderr = String::from_utf8(output.stderr)
        .unwrap_or_else(|err| panic!("{name}: stderr was not UTF-8: {err}"));

    let expected_exit = read_to_string(case.join("expected.exit"))
        .trim()
        .parse::<i32>()
        .unwrap_or_else(|err| panic!("{name}: parse expected.exit: {err}"));
    assert_eq!(
        actual_exit, expected_exit,
        "{name}: exit code mismatch\nstdout:\n{actual_stdout}\nstderr:\n{actual_stderr}"
    );
    assert_eq!(
        actual_stdout,
        read_expected_output(case.join("expected.stdout")),
        "{name}: stdout mismatch"
    );
    assert_eq!(
        actual_stderr,
        read_expected_output(case.join("expected.stderr")),
        "{name}: stderr mismatch"
    );
}

fn read_to_string(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

/// `expected.stdout` / `expected.stderr` use the e2e convention that a lone
/// newline means "empty output" so an empty capture round-trips to a one-byte
/// file rather than vanishing from the diff.
fn read_expected_output(path: impl AsRef<Path>) -> String {
    let output = read_to_string(path);
    if output == "\n" {
        String::new()
    } else {
        output
    }
}

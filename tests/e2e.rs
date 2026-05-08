use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CANONICAL_KINDS: &[&str] = &["G", "FS", "AS", "DA", "DF", "E2E"];

#[test]
fn e2e_cases_match_expected_reports() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for case in discover_cases(&manifest_dir) {
        run_case(&manifest_dir, &case);
    }
}

#[test]
fn e2e_output_is_deterministic() {
    // G-005-friendliness-first.3 / FS-007-non-goals.13: same input → same output, byte-for-byte.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for case in discover_cases(&manifest_dir) {
        let name = case
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("<invalid case name>");
        if is_mutating_case(&case) {
            continue;
        }
        let args = command_args(&manifest_dir, &case, name);
        let first = run_gnd(&manifest_dir, &args, name);
        let second = run_gnd(&manifest_dir, &args, name);
        assert_eq!(
            first.status.code(),
            second.status.code(),
            "{name}: exit code differs between runs"
        );
        assert_eq!(
            first.stdout, second.stdout,
            "{name}: stdout differs between runs"
        );
        assert_eq!(
            first.stderr, second.stderr,
            "{name}: stderr differs between runs"
        );
    }
}

fn is_mutating_case(case: &Path) -> bool {
    let command_file = case.join("command.args");
    if !command_file.exists() {
        return false;
    }
    let command = read_to_string(command_file);
    command.contains("--write") || command.contains("{repo_copy}")
}

fn discover_cases(manifest_dir: &Path) -> Vec<PathBuf> {
    let cases_dir = manifest_dir.join("e2e/cases");
    let mut cases = fs::read_dir(&cases_dir)
        .unwrap_or_else(|err| panic!("read {}: {err}", cases_dir.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    cases.sort();
    assert!(
        !cases.is_empty(),
        "expected at least one e2e case under {}",
        cases_dir.display()
    );
    cases
}

fn run_gnd(manifest_dir: &Path, args: &[String], name: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_gnd"))
        .args(args)
        .current_dir(manifest_dir)
        .output()
        .unwrap_or_else(|err| panic!("{name}: run gnd: {err}"))
}

fn run_case(manifest_dir: &Path, case: &Path) {
    let name = case
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<invalid case name>");
    assert_spec_refs(case, name);

    let args = command_args(manifest_dir, case, name);
    let output = run_gnd(manifest_dir, &args, name);
    let actual_exit = output.status.code().unwrap_or(-1);
    let actual_stdout = String::from_utf8(output.stdout)
        .unwrap_or_else(|err| panic!("{name}: stdout was not UTF-8: {err}"));
    let actual_stderr = String::from_utf8(output.stderr)
        .unwrap_or_else(|err| panic!("{name}: stderr was not UTF-8: {err}"));

    if std::env::var_os("UPDATE_EXPECTED").is_some() {
        write_expected(&case.join("expected.exit"), &format!("{actual_exit}\n"));
        write_expected(&case.join("expected.stdout"), &actual_stdout);
        write_expected(&case.join("expected.stderr"), &actual_stderr);
        return;
    }

    let expected_exit = read_to_string(case.join("expected.exit"));
    let expected_exit = expected_exit
        .trim()
        .parse::<i32>()
        .unwrap_or_else(|err| panic!("{name}: parse expected.exit: {err}"));
    assert_eq!(
        actual_exit, expected_exit,
        "{name}: exit code mismatch\nstdout:\n{actual_stdout}\nstderr:\n{actual_stderr}"
    );

    let expected_stdout = read_expected_output(case.join("expected.stdout"));
    let expected_stderr = read_expected_output(case.join("expected.stderr"));
    assert_expected_errors_are_concise(case, name, &expected_stderr);
    assert_eq!(actual_stdout, expected_stdout, "{name}: stdout mismatch");
    assert_eq!(actual_stderr, expected_stderr, "{name}: stderr mismatch");
}

fn assert_expected_errors_are_concise(case: &Path, name: &str, stderr: &str) {
    if read_to_string(case.join("expected.exit")).trim() == "0" {
        return;
    }
    assert!(
        !stderr.contains("error(s)") && !stderr.contains("warning(s)"),
        "{name}: stderr should not include aggregate summaries"
    );
    for line in stderr.lines().filter(|line| !line.trim().is_empty()) {
        assert!(
            line.len() <= 180,
            "{name}: stderr line is too long for a concise diagnostic: {line}"
        );
    }
}

fn write_expected(path: &Path, content: &str) {
    // Preserve the "single newline = empty" convention for stdout/stderr so empty captures
    // round-trip to a one-byte file rather than vanishing from the diff.
    let is_exit = path.extension().and_then(|s| s.to_str()) == Some("exit");
    let body = if content.is_empty() && !is_exit {
        "\n".to_string()
    } else {
        content.to_string()
    };
    fs::write(path, body).unwrap_or_else(|err| panic!("write {}: {err}", path.display()));
}

fn command_args(manifest_dir: &Path, case: &Path, name: &str) -> Vec<String> {
    let repo = case.join("repo");
    let repo_arg = repo
        .strip_prefix(manifest_dir)
        .unwrap_or(&repo)
        .to_string_lossy()
        .into_owned();
    let repo_copy = manifest_dir.join("target/e2e-work").join(name).join("repo");
    let repo_copy_arg = repo_copy
        .strip_prefix(manifest_dir)
        .unwrap_or(&repo_copy)
        .to_string_lossy()
        .into_owned();
    let command_file = case.join("command.args");
    if !command_file.exists() {
        return vec![repo_arg];
    }

    let command = read_to_string(command_file);
    if command.contains("{repo_copy}") {
        if let Some(parent) = repo_copy.parent() {
            let _ = fs::remove_dir_all(parent);
            fs::create_dir_all(parent)
                .unwrap_or_else(|err| panic!("{name}: create {}: {err}", parent.display()));
        }
        copy_dir(&repo, &repo_copy);
    }

    command
        .split_whitespace()
        .map(|arg| {
            if arg == "{repo}" {
                repo_arg.clone()
            } else if arg == "{repo_copy}" {
                repo_copy_arg.clone()
            } else {
                arg.to_string()
            }
        })
        .collect()
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap_or_else(|err| panic!("create {}: {err}", to.display()));
    for entry in fs::read_dir(from).unwrap_or_else(|err| panic!("read {}: {err}", from.display())) {
        let entry = entry.unwrap();
        let source = entry.path();
        let target = to.join(entry.file_name());
        if source.is_dir() {
            copy_dir(&source, &target);
        } else {
            fs::copy(&source, &target).unwrap_or_else(|err| {
                panic!("copy {} to {}: {err}", source.display(), target.display())
            });
        }
    }
}

fn assert_spec_refs(case: &Path, name: &str) {
    let refs_path = case.join("spec.refs");
    let refs = read_to_string(&refs_path);
    let refs = refs
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    assert!(
        !refs.is_empty(),
        "{name}: expected at least one spec reference in {}",
        refs_path.display()
    );
    for reference in refs {
        assert!(
            has_canonical_kind_prefix(reference),
            "{name}: spec.refs entry {reference} does not start with a canonical kind ({})",
            CANONICAL_KINDS.join(", ")
        );
    }
}

fn has_canonical_kind_prefix(reference: &str) -> bool {
    CANONICAL_KINDS.iter().any(|k| {
        reference
            .strip_prefix(k)
            .is_some_and(|rest| rest.starts_with('-'))
    })
}

fn read_to_string(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn read_expected_output(path: impl AsRef<Path>) -> String {
    let output = read_to_string(path);
    if output == "\n" {
        String::new()
    } else {
        output
    }
}

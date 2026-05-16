use std::path::PathBuf;

#[path = "support/case_runner.rs"]
mod case_runner;

use case_runner::CaseKind::{E2e, Example};
use case_runner::{assert_case_is_deterministic, discover_e2e_cases, discover_examples, run_case};

#[test]
fn e2e_cases_match_expected_reports() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for case in discover_e2e_cases(&manifest_dir) {
        run_case(&manifest_dir, &case, E2e);
    }
}

#[test]
fn e2e_output_is_deterministic() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for case in discover_e2e_cases(&manifest_dir) {
        assert_case_is_deterministic(&manifest_dir, &case);
    }
}

#[test]
fn examples_are_e2e_cases() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for case in discover_examples(&manifest_dir) {
        run_case(&manifest_dir, &case, Example);
    }
}

#[test]
fn example_output_is_deterministic() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for case in discover_examples(&manifest_dir) {
        assert_case_is_deterministic(&manifest_dir, &case);
    }
}

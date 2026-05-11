// Content-and-contract tests for `grund init`. The e2e harness covers the CLI surface
// (exit code + stderr listing); these tests cover what the *bytes on disk* look like
// after init runs: every emitted file exists, the `grund.toml` location matches the spec,
// the config validates, and `grund check` is clean against the freshly-scaffolded tree.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workdir(suffix: &str) -> PathBuf {
    let dir = manifest_dir().join("target/init-tests").join(suffix);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create workdir");
    dir
}

fn run_grund<P: AsRef<Path>>(args: &[&str], cwd: P) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grund"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn grund")
}

#[test]
fn init_default_writes_canonical_pair_and_passes_check() {
    // FS-init.2.1 (default form) + FS-config.1 (.agents/grund.toml location).
    let target = workdir("init_default_writes_canonical_pair_and_passes_check");
    let output = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        target.join("AGENTS.md").is_file(),
        "AGENTS.md was not written"
    );
    assert!(
        target.join(".agents/grund.toml").is_file(),
        ".agents/grund.toml was not written; init must place grund.toml under .agents/"
    );
    assert!(
        !target.join("grund.toml").exists(),
        "init must NOT write grund.toml at the repo root — it lives under .agents/"
    );

    let validate = run_grund(
        &["config", "validate", target.to_str().unwrap()],
        manifest_dir(),
    );
    assert!(
        validate.status.success(),
        "init's grund.toml does not validate:\n{}",
        String::from_utf8_lossy(&validate.stderr)
    );
}

#[test]
fn init_docs_form_emits_full_scaffold_and_check_is_clean() {
    // FS-init.2.1 (--docs form). The scaffolded tree must satisfy `grund check` —
    // i.e. the canonical AGENTS.md + grund.toml + docs skeleton is internally consistent.
    let target = workdir("init_docs_form_emits_full_scaffold_and_check_is_clean");
    let output = run_grund(
        &[
            "init",
            target.to_str().unwrap(),
            "--docs",
            "--name",
            "DemoProject",
        ],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init --docs failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let expected = [
        "AGENTS.md",
        ".agents/grund.toml",
        "docs/grund.md",
        "docs/goals/goals.md",
        "docs/roadmap.md",
        "docs/changelog.md",
        "docs/functional-spec/README.md",
        "docs/architectural-spec/README.md",
        "docs/decisions/architectural/.gitkeep",
        "docs/decisions/functional/.gitkeep",
        "e2e/README.md",
        "e2e/cases/.gitkeep",
    ];
    for rel in expected {
        assert!(target.join(rel).exists(), "init --docs did not write {rel}");
    }

    let agents = fs::read_to_string(target.join("AGENTS.md")).expect("read AGENTS.md");
    assert!(
        agents.contains("DemoProject"),
        "AGENTS.md must interpolate the --name into the H1 / opening sentence"
    );

    let grund_toml =
        fs::read_to_string(target.join(".agents/grund.toml")).expect("read .agents/grund.toml");
    assert!(
        grund_toml.contains("project_name = \"DemoProject\""),
        ".agents/grund.toml must carry project_name from --name"
    );

    let check = run_grund(&["check", target.to_str().unwrap()], manifest_dir());
    assert!(
        check.status.success(),
        "freshly init'd tree should be grund-clean but produced:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}

#[test]
fn init_agents_guidance_uses_existing_configured_artifact_homes() {
    let target = workdir("init_agents_guidance_uses_existing_configured_artifact_homes");
    fs::create_dir_all(target.join(".agents")).expect("create .agents");
    fs::write(
        target.join(".agents/grund.toml"),
        r#"grund_config_version = 1

[scan]
include = ["specs", "records", "crates"]

[[kinds]]
prefix = "FS"
folder = "specs"
title = "Product spec"

[[kinds]]
prefix = "ADR"
folder = "records/adr"
title = "Architecture decision"
"#,
    )
    .expect("write custom grund.toml");

    let output = run_grund(
        &["init", target.to_str().unwrap(), "--name", "Configured"],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let agents = fs::read_to_string(target.join("AGENTS.md")).expect("read AGENTS.md");
    assert!(
        agents.contains("| `FS` | `specs` | Product spec |"),
        "AGENTS.md should describe configured spec homes:\n{agents}"
    );
    assert!(
        agents.contains("| `ADR` | `records/adr` | Architecture decision |"),
        "AGENTS.md should describe configured decision homes:\n{agents}"
    );
    assert!(
        agents.contains("`specs`, `records`, `crates`"),
        "AGENTS.md should describe configured scan scope:\n{agents}"
    );
    assert!(
        !agents.contains("docs/architectural-spec/") && !agents.contains("docs/decisions/"),
        "AGENTS.md must not introduce canonical docs folders when specs are configured elsewhere"
    );
}

#[test]
fn init_rerun_on_current_repo_writes_nothing_and_reports_exists() {
    // FS-init.2.2 / FS-init.2.3: re-running `grund init` on a repo whose managed
    // AGENTS.md block already matches the current render rewrites nothing — the
    // file's bytes are untouched and it is reported with `exists `, not `updated `.
    let target = workdir("init_rerun_on_current_repo_writes_nothing_and_reports_exists");
    let first = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(first.status.success());

    let agents_before = fs::read(target.join("AGENTS.md")).unwrap();
    let toml_before = fs::read(target.join(".agents/grund.toml")).unwrap();

    let second = run_grund(&["init", target.to_str().unwrap()], manifest_dir());
    assert!(second.status.success());
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(
        stderr.contains("exists AGENTS.md"),
        "second `grund init` should report `exists AGENTS.md`, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("updated AGENTS.md") && !stderr.contains("wrote AGENTS.md"),
        "second `grund init` must not rewrite an already-current AGENTS.md, got:\n{stderr}"
    );
    assert!(
        stderr.contains("exists .agents/grund.toml"),
        "second `grund init` should report `exists .agents/grund.toml`, got:\n{stderr}"
    );

    assert_eq!(
        fs::read(target.join("AGENTS.md")).unwrap(),
        agents_before,
        "AGENTS.md bytes changed on a no-op re-init"
    );
    assert_eq!(
        fs::read(target.join(".agents/grund.toml")).unwrap(),
        toml_before,
        ".agents/grund.toml bytes changed on a no-op re-init"
    );
}

#[test]
fn init_force_never_overwrites_an_existing_config() {
    // FS-init.2.4 / FS-init.3: `.agents/grund.toml` is the repo's config, not a
    // scaffold artifact — `grund init --force` regenerates AGENTS.md but leaves an
    // existing config byte-for-byte intact and reports it with `exists `, never
    // `wrote `. (Overwriting it with the canonical template was a footgun.)
    let target = workdir("init_force_never_overwrites_an_existing_config");
    fs::create_dir_all(target.join(".agents")).expect("create .agents");
    let custom_config = "grund_config_version = 1\n\
        project_name = \"Custom\"\n\n\
        [reference]\nstrict = true\n\n\
        [[kinds]]\nprefix = \"SPEC\"\nfolder = \"specs\"\ntitle = \"Spec\"\n";
    fs::write(target.join(".agents/grund.toml"), custom_config).expect("write custom config");

    let output = run_grund(
        &["init", target.to_str().unwrap(), "--force"],
        manifest_dir(),
    );
    assert!(
        output.status.success(),
        "init --force failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("exists .agents/grund.toml"),
        "`grund init --force` must report `exists .agents/grund.toml`, got:\n{stderr}"
    );
    assert!(
        !stderr.contains("wrote .agents/grund.toml"),
        "`grund init --force` must not overwrite an existing config, got:\n{stderr}"
    );
    assert_eq!(
        fs::read_to_string(target.join(".agents/grund.toml")).unwrap(),
        custom_config,
        "`grund init --force` left .agents/grund.toml byte-for-byte? it did not"
    );
}

#[test]
fn init_is_byte_deterministic() {
    // FS-non-goals.13: same input → byte-identical output.
    let a = workdir("init_is_byte_deterministic_a");
    let b = workdir("init_is_byte_deterministic_b");
    for target in [&a, &b] {
        let out = run_grund(
            &["init", target.to_str().unwrap(), "--name", "Same"],
            manifest_dir(),
        );
        assert!(out.status.success());
    }
    let agents_a = fs::read(a.join("AGENTS.md")).unwrap();
    let agents_b = fs::read(b.join("AGENTS.md")).unwrap();
    assert_eq!(agents_a, agents_b, "AGENTS.md must be byte-identical");
    let toml_a = fs::read(a.join(".agents/grund.toml")).unwrap();
    let toml_b = fs::read(b.join(".agents/grund.toml")).unwrap();
    assert_eq!(toml_a, toml_b, ".agents/grund.toml must be byte-identical");
}

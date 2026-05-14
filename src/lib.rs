use anyhow::{Context, Result, anyhow};
use ignore::WalkBuilder;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;
use unicode_normalization::UnicodeNormalization;

// §AR-core-module-layout.1: keep the crate root as the entrypoint while the
// implementation is split into category files. `include!` keeps this first
// refactor behavior-preserving; a later crate/API split can add true module
// boundaries under §AR-bindings.
include!("grammar.rs");
include!("model.rs");
include!("config.rs");
include!("scanner.rs");
include!("checker.rs");
include!("checker_cmd.rs");
include!("output.rs");
include!("show.rs");
include!("show_render.rs");
include!("fmt.rs");
include!("fmt_links.rs");
include!("id.rs");
include!("refs.rs");
include!("cover.rs");
include!("list.rs");
include!("completions.rs");
include!("init_templates.rs");
include!("init.rs");
include!("cli_help.rs");
include!("cli.rs");
include!("tests.rs");

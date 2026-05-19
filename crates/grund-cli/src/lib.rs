// §RM-core-cli-split: the `grund` frontend crate owns top-level CLI dispatch.
use std::process::ExitCode;

use grund_core::{
    AGENT_SETUP_INSTRUCTIONS, canonical_template_text, command_check, command_complete,
    command_completions, command_config, command_cover, command_fmt, command_id, command_init,
    command_list, command_refs, command_show, command_show_default,
};

const SUBCOMMANDS: &[&str] = &[
    "check",
    "show",
    "list",
    "refs",
    "cover",
    "fmt",
    "id",
    "init",
    "config",
    "agent-setup-instructions",
    "completions",
];

include!("cli_help.rs");
include!("cli.rs");

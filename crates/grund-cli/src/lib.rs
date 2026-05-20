// §RM-core-cli-split: the `grund` frontend crate owns top-level CLI dispatch.
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::ExitCode;

use grund_core::{
    AGENT_SETUP_INSTRUCTIONS, ApiScanError, Config, CoverCitation, CoverOpts, FmtOpts, IdOpts,
    IdProposal, IdProposalOutcome, InitAgentEntrypointSelection, InitNext, InitOpts, InitOutput,
    ListEntry, ListOpts, RefHit, RefsOpts, canonical_template_text, command_check,
    command_complete, command_completions, command_show, command_show_default, cover,
    effective_config, format_references, init, list, propose_id, refs, validate_config,
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

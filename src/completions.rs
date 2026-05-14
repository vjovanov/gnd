/// `grund complete <subcommand>` — the namespace for internal completion helpers
/// the generated shell scripts call (§FS-completions.2).
fn command_complete(args: &[String]) -> ExitCode {
    match args.first().map(|arg| arg.as_str()) {
        Some("ids") => command_complete_ids(&args[1..]),
        _ => {
            eprintln!("error: expected `complete ids`");
            ExitCode::from(2)
        }
    }
}

/// `grund complete ids [--prefix P] [--sections] [path]` — the dynamic helper a
/// shell completion calls on every tab press (§FS-completions.2): emit declared
/// IDs (or `ID.section` candidates) matching the prefix, one per line. Scan/config
/// failures exit `0` silently so a broken repo never smears diagnostics across the
/// prompt; output is deterministic (§FS-completions.3).
fn command_complete_ids(args: &[String]) -> ExitCode {
    let mut path = PathBuf::from(".");
    let mut path_provided = false;
    let mut prefix = String::new();
    let mut force_sections = false;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--prefix" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --prefix requires a value");
                    return ExitCode::from(2);
                }
                prefix = args[idx].clone();
            }
            other if other.starts_with("--prefix=") => {
                prefix = other.trim_start_matches("--prefix=").to_string();
            }
            "--sections" => force_sections = true,
            "--path" => {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("error: --path requires a value");
                    return ExitCode::from(2);
                }
                path = PathBuf::from(&args[idx]);
                path_provided = true;
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag `{other}`");
                return ExitCode::from(2);
            }
            other => {
                path = PathBuf::from(other);
                path_provided = true;
            }
        }
        idx += 1;
    }

    // Completion is called on every tab press. Config or scan failures must not
    // smear diagnostics across the prompt; explicit flag misuse above is still a
    // normal CLI error because it is a bug in the installed completion script.
    let config = match load_config(&path) {
        Ok(config) => config,
        Err(_) => return ExitCode::SUCCESS,
    };
    let (findings, _) = match scan_tree(&config, Some(&path), path_provided) {
        Ok(out) => out,
        Err(_) => return ExitCode::SUCCESS,
    };

    let complete_sections = force_sections || prefix.contains(&config.section_separator);
    let mut candidates = BTreeSet::new();
    for (id, decls) in &findings.declarations {
        let rendered = render_id(&config, id);
        if complete_sections {
            for decl in decls {
                for section in decl.sections.keys() {
                    candidates.insert(format!(
                        "{}{}{}",
                        rendered, config.section_separator, section
                    ));
                }
            }
        } else {
            candidates.insert(rendered);
        }
    }

    for candidate in candidates {
        if candidate.starts_with(&prefix) {
            println!("{candidate}");
        }
    }
    ExitCode::SUCCESS
}

/// `grund completions <bash|zsh|fish>` — print the completion script for one shell
/// to stdout, ready to `source` (§FS-completions.1, §FS-completions.4). The scripts
/// call back into `grund complete ids` for the dynamic ID list.
fn command_completions(args: &[String]) -> ExitCode {
    if args.is_empty() {
        eprintln!("error: completions requires <bash|zsh|fish>");
        return ExitCode::from(2);
    }
    if args.len() > 1 {
        eprintln!("error: completions takes exactly one shell argument");
        return ExitCode::from(2);
    }
    match args[0].as_str() {
        "bash" => {
            print_bash_completion();
            ExitCode::SUCCESS
        }
        "zsh" => {
            print_zsh_completion();
            ExitCode::SUCCESS
        }
        "fish" => {
            print_fish_completion();
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("error: unsupported shell `{other}`");
            eprintln!("known shells: bash, zsh, fish");
            ExitCode::from(2)
        }
    }
}

/// The bash completion script: subcommand + flag completion, with `grund show` /
/// `grund refs` ID arguments wired to `grund complete ids` (§FS-completions.1,
/// §FS-completions.2).
fn print_bash_completion() {
    print!(
        r#"# bash completion for grund
_grund_complete_ids() {{
    local cur="${{COMP_WORDS[COMP_CWORD]}}"
    mapfile -t COMPREPLY < <(grund complete ids --prefix "$cur" 2>/dev/null)
}}

_grund() {{
    local cur="${{COMP_WORDS[COMP_CWORD]}}"
    local sub="${{COMP_WORDS[1]}}"
    COMPREPLY=()

    if [[ $COMP_CWORD -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "check show list refs cover fmt id init config agent-setup-instructions completions" -- "$cur") )
        return 0
    fi

    case "$sub" in
        show|refs)
            _grund_complete_ids
            return 0
            ;;
    esac
}}

complete -F _grund grund
"#
    );
}

/// The zsh completion script — the zsh counterpart of `print_bash_completion`
/// (§FS-completions.1, §FS-completions.2).
fn print_zsh_completion() {
    println!(
        r#"#compdef grund

_grund_ids() {{
  local -a ids
  ids=("${{(@f)$(grund complete ids --prefix "$words[CURRENT]" 2>/dev/null)}}")
  _describe 'grund ids' ids
}}

_grund() {{
  local -a commands
  commands=(
    'check:validate every reference in a repo'
    'show:print one declaration body by ID'
    'list:list declared IDs'
    'refs:list citations of an ID'
    'cover:group citations by file'
    'fmt:normalize citation syntax'
    'id:emit the next conflict-free ID'
    'init:scaffold AGENTS.md and config'
    'config:inspect the effective config'
    'agent-setup-instructions:print the guided setup instructions for AI agents'
    'completions:print shell completion script'
  )

  if (( CURRENT == 2 )); then
    _describe 'grund command' commands
    return
  fi

  case "$words[2]" in
    show|refs) _grund_ids ;;
    *) _files ;;
  esac
}}

_grund "$@"
"#
    );
}

/// The fish completion script — `complete -c grund …` lines, ID arguments wired to
/// `grund complete ids` (§FS-completions.1, §FS-completions.2).
fn print_fish_completion() {
    println!(
        r#"# fish completion for grund
function __grund_complete_ids
    set -l token (commandline -ct)
    grund complete ids --prefix "$token" 2>/dev/null
end

complete -c grund -f -n "__fish_use_subcommand" -a "check show list refs cover fmt id init config agent-setup-instructions completions"
complete -c grund -f -n "__fish_seen_subcommand_from show refs" -a "(__grund_complete_ids)"
"#
    );
}

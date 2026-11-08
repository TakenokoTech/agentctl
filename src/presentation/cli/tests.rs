use clap::CommandFactory;

use super::*;

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("agentctl").chain(args.iter().copied()))
}

#[test]
fn cli_definition_is_valid() {
    Cli::command().debug_assert();
}

#[test]
fn raw_and_config_are_accepted_before_and_after_the_subcommand() {
    let cli = parse(&["--raw", "--config", "a.toml", "list"]).unwrap();
    assert!(cli.raw);
    assert_eq!(cli.config, PathBuf::from("a.toml"));

    let cli = parse(&["list", "--raw", "--config", "b.toml"]).unwrap();
    assert!(cli.raw);
    assert_eq!(cli.config, PathBuf::from("b.toml"));
}

#[test]
fn config_defaults_to_agentctl_toml_and_raw_to_off() {
    // AGENTCTL_CONFIG が環境に設定されていると既定値を検証できない
    if std::env::var_os("AGENTCTL_CONFIG").is_some() {
        return;
    }
    let cli = parse(&["list"]).unwrap();
    assert_eq!(cli.config, PathBuf::from("agentctl.toml"));
    assert!(!cli.raw);
}

#[test]
fn mcp_call_takes_tool_and_optional_args() {
    let cli = parse(&["mcp", "srv", "call", "echo", "--args", "-"]).unwrap();
    match cli.cmd {
        Cmd::Mcp {
            server,
            cmd: McpCmd::Call { tool, args },
        } => {
            assert_eq!((server.as_str(), tool.as_str()), ("srv", "echo"));
            assert_eq!(args.as_deref(), Some("-"));
        }
        _ => panic!("expected `mcp call`"),
    }

    let cli = parse(&["mcp", "srv", "call", "echo"]).unwrap();
    assert!(matches!(
        cli.cmd,
        Cmd::Mcp {
            cmd: McpCmd::Call { args: None, .. },
            ..
        }
    ));
}

#[test]
fn a2a_send_takes_text_and_flags() {
    let cli = parse(&[
        "a2a",
        "ag",
        "send",
        "hello",
        "--context-id",
        "c",
        "--task-id",
        "t",
        "--no-wait",
    ])
    .unwrap();
    match cli.cmd {
        Cmd::A2a {
            agent,
            cmd:
                A2aCmd::Send {
                    text,
                    context_id,
                    task_id,
                    no_wait,
                },
        } => {
            assert_eq!((agent.as_str(), text.as_str()), ("ag", "hello"));
            assert_eq!(context_id.as_deref(), Some("c"));
            assert_eq!(task_id.as_deref(), Some("t"));
            assert!(no_wait);
        }
        _ => panic!("expected `a2a send`"),
    }
}

#[test]
fn a2a_task_subcommands_take_an_id() {
    for (sub, id) in [("get", "1"), ("watch", "2"), ("cancel", "3")] {
        let cli = parse(&["a2a", "ag", "task", sub, id]).unwrap();
        assert!(
            matches!(
                cli.cmd,
                Cmd::A2a {
                    cmd: A2aCmd::Task { .. },
                    ..
                }
            ),
            "{sub}"
        );
    }
}

#[test]
fn missing_required_arguments_are_rejected() {
    assert!(parse(&[]).is_err());
    assert!(parse(&["mcp", "srv"]).is_err());
    assert!(parse(&["mcp", "srv", "call"]).is_err());
    assert!(parse(&["a2a", "ag", "send"]).is_err());
    assert!(parse(&["a2a", "ag", "task", "get"]).is_err());
}

// ---------- `-` で始まるテキスト ----------

fn send_text(args: &[&str]) -> Result<(String, bool), clap::Error> {
    let mut all = vec!["a2a", "ag", "send"];
    all.extend_from_slice(args);
    match parse(&all)?.cmd {
        Cmd::A2a {
            cmd: A2aCmd::Send { text, no_wait, .. },
            ..
        } => Ok((text, no_wait)),
        _ => panic!("expected `a2a send`"),
    }
}

#[test]
fn text_starting_with_a_hyphen_is_accepted_as_the_text() {
    assert_eq!(
        send_text(&["-1 is the answer"]).unwrap(),
        ("-1 is the answer".to_owned(), false)
    );
    assert_eq!(
        send_text(&["--not a flag"]).unwrap(),
        ("--not a flag".to_owned(), false)
    );
}

#[test]
fn a_double_dash_still_separates_the_text() {
    assert_eq!(
        send_text(&["--no-wait", "--", "--config=/x"]).unwrap(),
        ("--config=/x".to_owned(), true)
    );
}

#[test]
fn known_flags_are_still_flags_before_or_after_the_text() {
    assert_eq!(
        send_text(&["--no-wait", "hi"]).unwrap(),
        ("hi".to_owned(), true)
    );
    assert_eq!(
        send_text(&["hi", "--no-wait"]).unwrap(),
        ("hi".to_owned(), true)
    );
}

#[test]
fn a_flag_is_not_swallowed_as_the_text_when_the_text_is_missing() {
    assert!(send_text(&["--no-wait"]).is_err());
    assert!(send_text(&["--context-id", "c"]).is_err());
}

// ---------- --timeout ----------

#[test]
fn timeout_defaults_to_60_seconds() {
    // AGENTCTL_TIMEOUT が環境に設定されていると既定値を検証できない
    if std::env::var_os("AGENTCTL_TIMEOUT").is_some() {
        return;
    }
    assert_eq!(
        parse(&["list"]).unwrap().timeout(),
        Some(Duration::from_secs(60))
    );
}

#[test]
fn timeout_takes_seconds_before_or_after_the_subcommand() {
    assert_eq!(
        parse(&["--timeout", "5", "list"]).unwrap().timeout(),
        Some(Duration::from_secs(5))
    );
    assert_eq!(
        parse(&["list", "--timeout", "7"]).unwrap().timeout(),
        Some(Duration::from_secs(7))
    );
}

#[test]
fn a_timeout_of_zero_disables_the_limit() {
    assert_eq!(parse(&["--timeout", "0", "list"]).unwrap().timeout(), None);
}

#[test]
fn a_timeout_that_is_not_a_number_of_seconds_is_rejected() {
    assert!(parse(&["--timeout", "soon", "list"]).is_err());
    assert!(parse(&["--timeout", "-1", "list"]).is_err());
}

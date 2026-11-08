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

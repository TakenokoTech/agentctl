use super::*;
use crate::domain::config::Kind;
use clap::Parser;

#[test]
fn list_line_is_tab_separated_kind_name_description() {
    let e = Entry {
        kind: Kind::A2a,
        name: "test",
        description: "Echo agent",
    };
    assert_eq!(list_line(&e), "a2a\ttest\tEcho agent");
}

#[test]
fn list_line_keeps_the_trailing_tab_for_an_empty_description() {
    let e = Entry {
        kind: Kind::Mcp,
        name: "x",
        description: "",
    };
    assert_eq!(list_line(&e), "mcp\tx\t");
}

#[test]
fn usage_errors_exit_64_but_help_and_version_exit_0() {
    let code = |args: &[&str]| {
        let e = Cli::try_parse_from(std::iter::once("agentctl").chain(args.iter().copied()))
            .err()
            .unwrap();
        usage_exit_code(&e)
    };
    assert_eq!(code(&["nope"]), ExitCode::from(64));
    assert_eq!(code(&["mcp"]), ExitCode::from(64));
    assert_eq!(code(&["--help"]), ExitCode::SUCCESS);
    assert_eq!(code(&["--version"]), ExitCode::SUCCESS);
}

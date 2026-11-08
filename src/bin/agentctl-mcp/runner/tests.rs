use super::*;

#[test]
fn exit_0_is_a_success_with_stdout_without_the_trailing_newline() {
    assert_eq!(
        interpret(Some(0), "a\nb\n", ""),
        Outcome {
            text: "a\nb".into(),
            is_error: false
        }
    );
}

#[test]
fn only_one_trailing_newline_is_removed() {
    assert_eq!(interpret(Some(0), "a\n\n", "").text, "a\n");
}

#[test]
fn success_ignores_stderr() {
    assert_eq!(interpret(Some(0), "ok\n", "warning\n").text, "ok");
}

#[test]
fn exit_2_keeps_the_tool_output_as_an_error() {
    let o = interpret(Some(2), "boom\n", "");
    assert_eq!(
        o,
        Outcome {
            text: "boom".into(),
            is_error: true
        }
    );
}

#[test]
fn exit_1_uses_stderr() {
    let o = interpret(Some(1), "", "error: unknown MCP server 'x'\n");
    assert_eq!(
        o,
        Outcome {
            text: "error: unknown MCP server 'x'".into(),
            is_error: true
        }
    );
}

#[test]
fn a_usage_error_with_no_stdout_falls_back_to_stderr() {
    let o = interpret(Some(2), "", "error: unexpected argument\n");
    assert_eq!(o.text, "error: unexpected argument");
    assert!(o.is_error);
}

#[test]
fn stdout_wins_over_stderr_when_both_are_present() {
    assert_eq!(interpret(Some(2), "out\n", "err\n").text, "out");
}

#[test]
fn a_failure_with_no_output_is_described_by_its_status() {
    assert_eq!(
        interpret(Some(3), "", "").text,
        "error: agentctl exited with status 3"
    );
    assert_eq!(
        interpret(None, "", "").text,
        "error: agentctl was terminated by a signal"
    );
}

#[tokio::test]
async fn a_missing_executable_is_reported_with_how_to_fix_it() {
    let runner = Runner::new("/nonexistent/agentctl");
    let o = runner
        .run(&Invocation {
            args: vec!["list".into()],
            stdin: None,
        })
        .await;
    assert!(o.is_error);
    assert!(
        o.text
            .starts_with("error: failed to run `/nonexistent/agentctl`"),
        "{}",
        o.text
    );
    assert!(o.text.contains(BIN_ENV), "{}", o.text);
}

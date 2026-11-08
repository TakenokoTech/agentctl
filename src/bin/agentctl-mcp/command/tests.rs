use serde_json::json;

use super::*;

fn args(v: Value) -> JsonObject {
    v.as_object().cloned().expect("object")
}

fn inv(tool: &str, v: Value) -> Invocation {
    to_invocation(tool, args(v)).unwrap_or_else(|e| panic!("{tool}: {e}"))
}

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_owned()).collect()
}

// ---------- 変換 ----------

#[test]
fn list_maps_to_list() {
    assert_eq!(inv("list", json!({})).args, strings(&["list"]));
}

#[test]
fn mcp_tools_and_schema_map_to_their_subcommands() {
    assert_eq!(
        inv("mcp_tools", json!({"server": "s"})).args,
        strings(&["mcp", "s", "tools"])
    );
    assert_eq!(
        inv("mcp_schema", json!({"server": "s", "tool": "t"})).args,
        strings(&["mcp", "s", "schema", "t"])
    );
}

#[test]
fn mcp_call_without_args_passes_no_args_option() {
    let i = inv("mcp_call", json!({"server": "s", "tool": "t"}));
    assert_eq!(i.args, strings(&["mcp", "s", "call", "t"]));
    assert_eq!(i.stdin, None);
}

#[test]
fn mcp_call_sends_the_arguments_through_stdin() {
    let i = inv(
        "mcp_call",
        json!({"server": "s", "tool": "t", "args": {"message": "hi"}}),
    );
    assert_eq!(i.args, strings(&["mcp", "s", "call", "t", "--args", "-"]));
    assert_eq!(i.stdin.as_deref(), Some(r#"{"message":"hi"}"#));
}

#[test]
fn raw_is_a_leading_global_option() {
    let i = inv("mcp_call", json!({"server": "s", "tool": "t", "raw": true}));
    assert_eq!(i.args, strings(&["--raw", "mcp", "s", "call", "t"]));
    let i = inv("a2a_card", json!({"agent": "a", "raw": true}));
    assert_eq!(i.args, strings(&["--raw", "a2a", "a", "card"]));
}

#[test]
fn raw_false_adds_nothing() {
    let i = inv("a2a_card", json!({"agent": "a", "raw": false}));
    assert_eq!(i.args, strings(&["a2a", "a", "card"]));
}

#[test]
fn a2a_send_puts_the_text_after_a_double_dash() {
    let i = inv("a2a_send", json!({"agent": "a", "text": "hello"}));
    assert_eq!(i.args, strings(&["a2a", "a", "send", "--", "hello"]));
}

#[test]
fn a2a_send_text_starting_with_a_hyphen_is_not_an_option() {
    let i = inv("a2a_send", json!({"agent": "a", "text": "--config=/evil"}));
    assert_eq!(
        i.args,
        strings(&["a2a", "a", "send", "--", "--config=/evil"])
    );
}

#[test]
fn a2a_send_passes_option_values_in_the_equals_form() {
    let i = inv(
        "a2a_send",
        json!({"agent": "a", "text": "t", "context_id": "-c", "task_id": "x", "no_wait": true}),
    );
    assert_eq!(
        i.args,
        strings(&[
            "a2a",
            "a",
            "send",
            "--context-id=-c",
            "--task-id=x",
            "--no-wait",
            "--",
            "t",
        ])
    );
}

#[test]
fn a2a_task_get_and_cancel_map_to_task_subcommands() {
    assert_eq!(
        inv("a2a_task_get", json!({"agent": "a", "id": "1"})).args,
        strings(&["a2a", "a", "task", "get", "1"])
    );
    assert_eq!(
        inv("a2a_task_cancel", json!({"agent": "a", "id": "1"})).args,
        strings(&["a2a", "a", "task", "cancel", "1"])
    );
}

// ---------- 入力の検証 ----------

#[test]
fn names_that_look_like_options_are_rejected() {
    for (tool, v) in [
        ("mcp_tools", json!({"server": "--config=/evil"})),
        ("mcp_tools", json!({"server": "-h"})),
        ("mcp_schema", json!({"server": "s", "tool": "--raw"})),
        ("mcp_call", json!({"server": "s", "tool": "-x"})),
        ("a2a_card", json!({"agent": "--raw"})),
        ("a2a_send", json!({"agent": "-a", "text": "t"})),
        ("a2a_task_get", json!({"agent": "a", "id": "--help"})),
        ("a2a_task_cancel", json!({"agent": "a", "id": "-1"})),
    ] {
        let err = to_invocation(tool, args(v)).unwrap_err();
        assert!(
            err.contains("must not be empty or start with '-'"),
            "{tool}: {err}"
        );
    }
}

#[test]
fn empty_names_are_rejected() {
    assert!(to_invocation("mcp_tools", args(json!({"server": ""}))).is_err());
    assert!(to_invocation("a2a_task_get", args(json!({"agent": "a", "id": ""}))).is_err());
}

#[test]
fn list_takes_no_arguments() {
    assert!(to_invocation("list", args(json!({"x": 1}))).is_err());
}

#[test]
fn missing_required_arguments_are_rejected() {
    assert!(to_invocation("mcp_tools", JsonObject::new()).is_err());
    assert!(to_invocation("mcp_schema", args(json!({"server": "s"}))).is_err());
    assert!(to_invocation("a2a_send", args(json!({"agent": "a"}))).is_err());
    assert!(to_invocation("a2a_task_get", args(json!({"agent": "a"}))).is_err());
}

#[test]
fn unknown_arguments_are_rejected() {
    let err = to_invocation("mcp_tools", args(json!({"server": "s", "srever": "x"}))).unwrap_err();
    assert!(err.contains("srever"), "{err}");
}

#[test]
fn arguments_of_the_wrong_type_are_rejected() {
    assert!(to_invocation("mcp_tools", args(json!({"server": 1}))).is_err());
    assert!(
        to_invocation(
            "mcp_call",
            args(json!({"server": "s", "tool": "t", "args": "x"}))
        )
        .is_err()
    );
    assert!(to_invocation("a2a_card", args(json!({"agent": "a", "raw": "yes"}))).is_err());
}

#[test]
fn unknown_tools_are_rejected_including_watch() {
    assert_eq!(
        to_invocation("a2a_task_watch", JsonObject::new()).unwrap_err(),
        "unknown tool 'a2a_task_watch'"
    );
}

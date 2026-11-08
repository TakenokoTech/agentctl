use super::*;

fn output(structured: Option<Value>, contents: Vec<Content>) -> ToolOutput {
    ToolOutput {
        is_error: false,
        structured,
        contents,
        raw: json!({"raw": true}),
    }
}

#[test]
fn error_results_exit_2_and_others_exit_0() {
    let mut out = output(None, vec![]);
    assert_eq!(exit_code(&out), ExitCode::SUCCESS);
    out.is_error = true;
    assert_eq!(exit_code(&out), ExitCode::from(EXIT_TOOL_FAILED));
}

#[test]
fn tool_line_without_description_ends_with_a_tab() {
    let t = ToolInfo {
        name: "echo".into(),
        description: None,
        input_schema: Value::Null,
    };
    assert_eq!(tool_line(&t), "echo\t");
}

#[test]
fn schema_json_has_name_description_and_input_schema() {
    let t = ToolInfo {
        name: "echo".into(),
        description: Some("Echo\nmore".into()),
        input_schema: json!({"type": "object"}),
    };
    assert_eq!(
        schema_json(&t),
        json!({"name": "echo", "description": "Echo\nmore", "inputSchema": {"type": "object"}})
    );
}

#[test]
fn empty_output_prints_nothing() {
    assert!(render_output(&output(None, vec![]), false).is_empty());
}

#[test]
fn args_must_be_a_json_object() {
    assert!(parse_args(r#"{"a": 1}"#).is_ok());
    assert!(parse_args("[1]").is_err());
    assert!(parse_args("not json").is_err());
}

#[test]
fn tool_line_uses_only_the_first_line_of_the_description() {
    let t = ToolInfo {
        name: "echo".into(),
        description: Some("First\nSecond".into()),
        input_schema: Value::Null,
    };
    assert_eq!(tool_line(&t), "echo\tFirst");
}

#[test]
fn structured_content_wins_over_text() {
    let out = output(Some(json!({"a": 1})), vec![Content::Text("t".into())]);
    assert_eq!(render_output(&out, false), vec![r#"{"a":1}"#]);
}

#[test]
fn text_is_printed_as_is_and_other_content_as_json() {
    let out = output(
        None,
        vec![
            Content::Text("hello".into()),
            Content::Other(json!({"type": "image"})),
        ],
    );
    assert_eq!(
        render_output(&out, false),
        vec!["hello".to_owned(), r#"{"type":"image"}"#.to_owned()]
    );
}

#[test]
fn raw_prints_the_protocol_response() {
    let out = output(Some(json!({"a": 1})), vec![]);
    assert_eq!(render_output(&out, true), vec![r#"{"raw":true}"#]);
}

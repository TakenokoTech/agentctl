use std::sync::Arc;

use rmcp::model::ContentBlock;
use serde_json::json;

use super::*;

#[test]
fn tool_info_keeps_name_description_and_schema() {
    let schema = json!({"type": "object", "required": ["a"]});
    let tool = Tool::new(
        "echo",
        "Echo it",
        Arc::new(schema.as_object().unwrap().clone()),
    );
    let info = tool_info(&tool).unwrap();
    assert_eq!(info.name, "echo");
    assert_eq!(info.description.as_deref(), Some("Echo it"));
    assert_eq!(info.input_schema, schema);
}

#[test]
fn text_content_becomes_text() {
    let res = CallToolResult::success(vec![ContentBlock::text("hello")]);
    let out = to_output(&res).unwrap();
    assert!(!out.is_error);
    assert_eq!(out.contents, vec![Content::Text("hello".into())]);
    assert_eq!(out.structured, None);
}

#[test]
fn error_flag_is_carried_over() {
    let res = CallToolResult::error(vec![ContentBlock::text("boom")]);
    assert!(to_output(&res).unwrap().is_error);
}

#[test]
fn non_text_content_is_kept_as_json() {
    let res = CallToolResult::success(vec![ContentBlock::image("AAAA", "image/png")]);
    let out = to_output(&res).unwrap();
    match &out.contents[..] {
        [Content::Other(v)] => assert_eq!(v["type"], "image"),
        other => panic!("unexpected contents: {other:?}"),
    }
}

#[test]
fn structured_content_is_carried_over() {
    let mut res = CallToolResult::success(vec![ContentBlock::text("t")]);
    res.structured_content = Some(json!({"a": 1}));
    let out = to_output(&res).unwrap();
    assert_eq!(out.structured, Some(json!({"a": 1})));
}

#[test]
fn raw_is_the_serialized_protocol_result() {
    let res = CallToolResult::success(vec![ContentBlock::text("hello")]);
    let out = to_output(&res).unwrap();
    assert_eq!(out.raw["content"][0]["text"], "hello");
}

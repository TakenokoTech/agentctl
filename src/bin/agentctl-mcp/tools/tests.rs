use std::collections::HashSet;

use serde_json::json;

use super::*;
use crate::command::{JsonObject, to_invocation};

/// 必須の引数だけを埋めた入力。
fn required_only(t: &Tool) -> JsonObject {
    let mut input = JsonObject::new();
    for r in t.input_schema["required"].as_array().unwrap() {
        input.insert(r.as_str().unwrap().to_owned(), json!("x"));
    }
    input
}

#[test]
fn exposes_one_tool_per_subcommand_except_watch() {
    let names: Vec<_> = tools().iter().map(|t| t.name.to_string()).collect();
    assert_eq!(
        names,
        [
            "list",
            "mcp_tools",
            "mcp_schema",
            "mcp_call",
            "a2a_card",
            "a2a_send",
            "a2a_task_get",
            "a2a_task_cancel",
        ]
    );
}

#[test]
fn every_tool_has_a_description_and_a_closed_object_schema() {
    for t in tools() {
        assert!(
            t.description.as_deref().is_some_and(|d| !d.is_empty()),
            "{}",
            t.name
        );
        assert_eq!(t.input_schema["type"], "object", "{}", t.name);
        assert_eq!(t.input_schema["additionalProperties"], false, "{}", t.name);
    }
}

#[test]
fn required_properties_are_declared_in_the_schema() {
    for t in tools() {
        let props: HashSet<_> = t.input_schema["properties"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        for r in t.input_schema["required"].as_array().unwrap() {
            assert!(props.contains(r.as_str().unwrap()), "{}: {r}", t.name);
        }
    }
}

#[test]
fn every_tool_is_accepted_with_only_its_required_arguments() {
    // 定義と変換のずれ(定義にあるのに変換が拒否する、など)を防ぐ
    for t in tools() {
        assert!(
            to_invocation(&t.name, required_only(&t)).is_ok(),
            "{}",
            t.name
        );
    }
}

#[test]
fn every_declared_property_is_accepted_by_the_conversion() {
    for t in tools() {
        for (key, schema) in t.input_schema["properties"].as_object().unwrap() {
            let mut input = required_only(&t);
            let value = match schema["type"].as_str().unwrap() {
                "boolean" => json!(true),
                "object" => json!({"k": 1}),
                _ => json!("x"),
            };
            input.insert(key.clone(), value);
            assert!(to_invocation(&t.name, input).is_ok(), "{}.{key}", t.name);
        }
    }
}

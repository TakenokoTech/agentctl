use super::*;
use crate::domain::config::Kind;

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

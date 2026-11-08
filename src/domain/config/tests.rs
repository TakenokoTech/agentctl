use super::*;

#[test]
fn kind_names_are_the_subcommand_names() {
    assert_eq!(Kind::Mcp.as_str(), "mcp");
    assert_eq!(Kind::A2a.as_str(), "a2a");
}

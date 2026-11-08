use super::*;

#[test]
fn the_tail_keeps_only_the_last_lines() {
    let mut tail = Tail::default();
    for i in 0..30 {
        tail.push(&format!("line {i}"));
    }
    let text = tail.text();
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), TAIL_LINES);
    assert_eq!(lines.first(), Some(&"line 10"));
    assert_eq!(lines.last(), Some(&"line 29"));
}

#[test]
fn a_long_line_is_cut() {
    let mut tail = Tail::default();
    tail.push(&"x".repeat(MAX_LINE_CHARS + 100));
    assert_eq!(tail.text().chars().count(), MAX_LINE_CHARS);
}

#[test]
fn lines_are_cut_by_characters_not_bytes() {
    let mut tail = Tail::default();
    tail.push(&"あ".repeat(MAX_LINE_CHARS + 10));
    assert_eq!(tail.text().chars().count(), MAX_LINE_CHARS);
}

#[test]
fn an_empty_tail_is_empty() {
    let tail = Tail::default();
    assert!(tail.is_empty());
    assert_eq!(tail.text(), "");
}

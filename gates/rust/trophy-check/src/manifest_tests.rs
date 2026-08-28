use super::{matches_for, pattern_matches, Entry, TestCase};

fn entry(pattern: &str) -> Entry {
    Entry {
        id: "e".into(),
        story: "US-001".into(),
        layer: "unit".into(),
        pattern: pattern.into(),
        block_merge: true,
        desc: "d".into(),
    }
}

fn test(full: &str) -> TestCase {
    TestCase {
        full: full.into(),
        testcase: full.rsplit("::").next().unwrap_or(full).into(),
    }
}

#[test]
fn a_bare_substring_matches() {
    assert!(pattern_matches("records_id", "app::runs::records_id_once"));
}

#[test]
fn groups_are_anded() {
    let full = "app::step_run::records_id";
    assert!(pattern_matches("step_run & records_id", full));
    assert!(!pattern_matches("step_run & absent", full));
}

#[test]
fn alternatives_within_a_group_are_ored() {
    let full = "app::step_run::records_id";
    assert!(pattern_matches("step_run & (records_id|writes_id)", full));
    assert!(!pattern_matches("step_run & (nope|neither)", full));
}

#[test]
fn parens_and_whitespace_are_ignored() {
    assert!(pattern_matches("a & (b|c)", "xax bx"));
    assert!(pattern_matches("a&b|c", "xax bx"));
}

#[test]
fn an_empty_pattern_matches_nothing_rather_than_everything() {
    assert!(!pattern_matches("", "anything"));
    assert!(!pattern_matches("  &  ", "anything"));
}

#[test]
fn matches_for_returns_every_matching_test() {
    let tests = vec![
        test("app::a::records_id"),
        test("app::b::records_id"),
        test("app::c::unrelated"),
    ];
    assert_eq!(matches_for(&entry("records_id"), &tests).len(), 2);
}

#[test]
fn a_pattern_matching_no_test_returns_empty() {
    let tests = vec![test("app::a::unrelated")];
    assert!(matches_for(&entry("records_id"), &tests).is_empty());
}

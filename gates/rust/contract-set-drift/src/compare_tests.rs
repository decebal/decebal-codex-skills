use super::{compare, extract, Presence, Sets};
use std::collections::BTreeSet;

fn sets(pairs: &[(&str, &[&str])]) -> Sets {
    pairs
        .iter()
        .map(|(name, ops)| {
            (
                (*name).to_string(),
                ops.iter()
                    .map(|o| (*o).to_string())
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect()
}

#[test]
fn identical_sets_are_not_drift() {
    let a = sets(&[("home", &["read"])]);
    assert!(compare(&a, &a).is_empty());
}

#[test]
fn an_operation_only_the_right_side_demands_blames_the_left() {
    let drift = compare(&sets(&[("home", &[])]), &sets(&[("home", &["read"])]));
    assert_eq!(drift.len(), 1);
    assert!(drift[0].missing_from_left.contains("read"));
    assert!(drift[0].missing_from_right.is_empty());
}

#[test]
fn an_operation_only_the_left_side_declares_blames_the_right() {
    let drift = compare(&sets(&[("home", &["read"])]), &sets(&[("home", &[])]));
    assert!(drift[0].missing_from_right.contains("read"));
}

#[test]
fn an_entry_missing_from_one_side_is_drift_even_with_an_empty_set() {
    let drift = compare(&sets(&[("home", &[])]), &Sets::new());
    assert_eq!(drift.len(), 1);
    assert_eq!(drift[0].presence, Presence::LeftOnly);
}

#[test]
fn drift_is_reported_per_entry_not_collapsed() {
    let left = sets(&[("a", &["x"]), ("b", &["y"])]);
    let right = sets(&[("a", &[]), ("b", &[])]);
    assert_eq!(compare(&left, &right).len(), 2);
}

#[test]
fn blame_points_at_the_side_that_is_missing_the_declaration() {
    let drift = compare(&sets(&[("home", &[])]), &sets(&[("home", &["read"])]));
    assert!(drift[0].blames_left());
    assert!(!drift[0].blames_right());
}

#[test]
fn an_entry_present_on_one_side_only_blames_the_side_that_lacks_it() {
    let drift = compare(&sets(&[("home", &[])]), &Sets::new());
    assert!(drift[0].blames_right());
    assert!(!drift[0].blames_left());
}

#[test]
fn extracts_a_nested_map_and_its_per_entry_set() {
    let doc: serde_json::Value =
        serde_json::from_str(r#"{"contract":{"features":{"home":{"ops":["read"]}}}}"#).unwrap();
    let got = extract(&doc, "contract.features", "ops").unwrap();
    assert_eq!(got["home"], BTreeSet::from(["read".to_string()]));
}

#[test]
fn an_entry_without_the_set_key_reads_as_an_empty_set() {
    let doc: serde_json::Value = serde_json::from_str(r#"{"features":{"home":{}}}"#).unwrap();
    let got = extract(&doc, "features", "ops").unwrap();
    assert!(got["home"].is_empty());
}

#[test]
fn an_empty_set_key_takes_the_entry_itself_as_the_array() {
    let doc: serde_json::Value = serde_json::from_str(r#"{"features":{"home":["read"]}}"#).unwrap();
    let got = extract(&doc, "features", "").unwrap();
    assert_eq!(got["home"], BTreeSet::from(["read".to_string()]));
}

#[test]
fn a_missing_map_path_is_an_error_not_an_empty_contract() {
    let doc: serde_json::Value = serde_json::from_str(r#"{"other":{}}"#).unwrap();
    assert!(extract(&doc, "features", "ops").is_err());
}

#[test]
fn a_non_object_map_path_is_an_error() {
    let doc: serde_json::Value = serde_json::from_str(r#"{"features":[]}"#).unwrap();
    assert!(extract(&doc, "features", "ops").is_err());
}

#[test]
fn a_non_string_member_of_a_set_is_an_error() {
    let doc: serde_json::Value =
        serde_json::from_str(r#"{"features":{"home":{"ops":[1]}}}"#).unwrap();
    assert!(extract(&doc, "features", "ops").is_err());
}

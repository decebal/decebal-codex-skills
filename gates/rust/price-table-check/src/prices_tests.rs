use super::{disagreements, extract, uncovered, Price, Shape, Table};

fn shape(map: &str, scale: f64) -> Shape {
    Shape {
        map_path: map.into(),
        input_key: "input".into(),
        output_key: "output".into(),
        scale,
    }
}

fn table(pairs: &[(&str, f64, f64)]) -> Table {
    pairs
        .iter()
        .map(|(id, i, o)| {
            (
                (*id).to_string(),
                Price {
                    input: *i,
                    output: *o,
                },
            )
        })
        .collect()
}

#[test]
fn extracts_prices_from_a_nested_map() {
    let doc: serde_json::Value =
        serde_json::from_str(r#"{"models":{"a":{"input":3.0,"output":15.0}}}"#).unwrap();
    let got = extract(&doc, &shape("models", 1.0)).unwrap();
    assert_eq!(got["a"].input, 3.0);
    assert_eq!(got["a"].output, 15.0);
}

#[test]
fn scale_converts_per_token_into_per_million() {
    let doc: serde_json::Value =
        serde_json::from_str(r#"{"a":{"input":0.000003,"output":0.000015}}"#).unwrap();
    let got = extract(&doc, &shape("", 1_000_000.0)).unwrap();
    assert_eq!(got["a"].input, 3.0);
    assert_eq!(got["a"].output, 15.0);
}

#[test]
fn an_entry_missing_either_number_is_dropped_not_defaulted_to_zero() {
    let doc: serde_json::Value =
        serde_json::from_str(r#"{"a":{"input":1.0},"b":{"input":1.0,"output":2.0}}"#).unwrap();
    let got = extract(&doc, &shape("", 1.0)).unwrap();
    assert!(!got.contains_key("a"));
    assert!(got.contains_key("b"));
}

#[test]
fn a_missing_map_path_is_an_error_not_an_empty_table() {
    let doc: serde_json::Value = serde_json::from_str(r#"{"other":{}}"#).unwrap();
    assert!(extract(&doc, &shape("models", 1.0)).is_err());
}

#[test]
fn matching_prices_are_no_disagreement() {
    let t = table(&[("a", 3.0, 15.0)]);
    assert!(disagreements(&t, &t, 0.0, &[]).is_empty());
}

#[test]
fn a_differing_input_price_is_reported() {
    let ours = table(&[("a", 3.0, 15.0)]);
    let theirs = table(&[("a", 4.0, 15.0)]);
    let found = disagreements(&ours, &theirs, 0.0, &[]);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, "a");
}

#[test]
fn a_difference_inside_the_tolerance_passes() {
    let ours = table(&[("a", 3.0, 15.0)]);
    let theirs = table(&[("a", 3.004, 15.0)]);
    assert!(disagreements(&ours, &theirs, 0.01, &[]).is_empty());
}

#[test]
fn an_allowlisted_id_is_never_reported() {
    let ours = table(&[("a", 3.0, 15.0)]);
    let theirs = table(&[("a", 99.0, 99.0)]);
    assert!(disagreements(&ours, &theirs, 0.0, &["a".to_string()]).is_empty());
}

#[test]
fn a_model_the_reference_has_never_heard_of_is_not_a_disagreement() {
    let ours = table(&[("new", 3.0, 15.0)]);
    assert!(disagreements(&ours, &Table::new(), 0.0, &[]).is_empty());
}

#[test]
fn uncovered_names_what_the_reference_does_not_price() {
    let ours = table(&[("a", 1.0, 1.0), ("new", 1.0, 1.0)]);
    let theirs = table(&[("a", 1.0, 1.0)]);
    assert_eq!(uncovered(&ours, &theirs), vec!["new".to_string()]);
}

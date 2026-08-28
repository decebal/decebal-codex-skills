use super::{parse_failed, parse_list, union_expr};
use std::collections::BTreeSet;

const LISTING: &str = r#"{"rust-suites":{
  "app::bin":{"binary-id":"app::bin","testcases":{"a::one":{},"a::two":{}}},
  "app::lib":{"binary-id":"app::lib","testcases":{"b::three":{}}}
}}"#;

#[test]
fn parses_every_suite_and_prefixes_the_binary_id() {
    let mut tests = parse_list(LISTING).unwrap();
    tests.sort_by(|a, b| a.full.cmp(&b.full));
    assert_eq!(tests.len(), 3);
    assert_eq!(tests[0].full, "app::bin::a::one");
    assert_eq!(tests[0].testcase, "a::one");
}

#[test]
fn a_suite_without_testcases_is_skipped_not_an_error() {
    let json = r#"{"rust-suites":{"app::bin":{"binary-id":"app::bin"}}}"#;
    assert!(parse_list(json).unwrap().is_empty());
}

#[test]
fn a_listing_without_rust_suites_is_an_error() {
    assert!(parse_list("{}").is_err());
}

#[test]
fn malformed_json_is_an_error_not_an_empty_listing() {
    assert!(parse_list("not json").is_err());
}

#[test]
fn a_fail_line_yields_the_testcase_name() {
    let out = "    FAIL [   0.031s] app::bin a::one\n   PASS [ 0.001s] app::bin a::two\n";
    assert_eq!(parse_failed(out), BTreeSet::from(["a::one".to_string()]));
}

#[test]
fn every_failure_status_is_recognised_not_only_fail() {
    let out = "TIMEOUT [ 1s] app::bin a::slow\nSIGSEGV [ 1s] app::bin a::crash\n";
    assert_eq!(parse_failed(out).len(), 2);
}

#[test]
fn a_clean_run_reports_no_failures() {
    assert!(parse_failed("   PASS [ 0.001s] app::bin a::two\n").is_empty());
}

#[test]
fn the_run_filter_selects_exactly_the_named_tests() {
    let set = BTreeSet::from(["a::one".to_string(), "b::two".to_string()]);
    assert_eq!(union_expr(&set), "test(=a::one) | test(=b::two)");
}

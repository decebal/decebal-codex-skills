use super::{has_extension, is_comment, scan, Rule};
use regex::Regex;

fn rule(patterns: &[&str]) -> Rule {
    Rule {
        name: "test".into(),
        roots: vec!["src/".into()],
        files: Vec::new(),
        extensions: vec!["rs".into()],
        patterns: patterns.iter().map(|p| Regex::new(p).unwrap()).collect(),
        allow: Vec::new(),
        include_comments: false,
        skip_tests: false,
        why: "because".into(),
    }
}

#[test]
fn flags_a_banned_symbol_on_a_code_line() {
    let hits = scan("src/a.rs", "use keyring;\n", &rule(&[r"use\s+keyring"]));
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].line, 1);
}

#[test]
fn a_comment_naming_the_symbol_is_not_a_violation() {
    let text = "// never `use keyring` here\n/* use keyring */\n * use keyring\n";
    assert!(scan("src/a.rs", text, &rule(&[r"use\s+keyring"])).is_empty());
}

#[test]
fn include_comments_opts_back_in() {
    let mut r = rule(&[r"use\s+keyring"]);
    r.include_comments = true;
    assert_eq!(scan("src/a.rs", "// use keyring\n", &r).len(), 1);
}

#[test]
fn reports_every_matching_line_not_just_the_first() {
    let text = "use keyring;\nlet e = keyring::Entry::new();\n";
    let hits = scan(
        "src/a.rs",
        text,
        &rule(&[r"use\s+keyring", r"keyring::Entry"]),
    );
    assert_eq!(hits.len(), 2);
}

#[test]
fn roots_select_by_prefix() {
    let r = rule(&[""]);
    assert!(r.selects("src/a.rs"));
    assert!(!r.selects("other/a.rs"));
}

#[test]
fn an_allowlisted_path_is_never_selected() {
    let mut r = rule(&[""]);
    r.allow = vec!["src/migration.rs".into()];
    assert!(!r.selects("src/migration.rs"));
    assert!(r.selects("src/other.rs"));
}

#[test]
fn extensions_filter_out_other_languages() {
    let r = rule(&[""]);
    assert!(!r.selects("src/a.ts"));
}

#[test]
fn an_explicit_file_list_selects_only_those_files() {
    let mut r = rule(&[""]);
    r.roots = Vec::new();
    r.files = vec!["src/guarded.rs".into()];
    assert!(r.selects("src/guarded.rs"));
    assert!(!r.selects("src/other.rs"));
}

#[test]
fn no_roots_and_no_files_means_every_file_of_the_extension() {
    let mut r = rule(&[""]);
    r.roots = Vec::new();
    assert!(r.selects("anywhere/a.rs"));
}

#[test]
fn comment_openers_cover_the_languages_these_gates_read() {
    assert!(is_comment("  // rust"));
    assert!(is_comment("  # shell"));
    assert!(is_comment("  <!-- html -->"));
    assert!(!is_comment("let x = 1; // trailing"));
}

#[test]
fn tests_are_scanned_by_default_because_a_banned_api_is_banned_there_too() {
    let r = rule(&[""]);
    assert!(r.selects("src/thing_tests.rs"));
}

#[test]
fn skip_tests_drops_every_test_convention() {
    let mut r = rule(&[""]);
    r.skip_tests = true;
    assert!(!r.selects("src/thing_tests.rs"));
    assert!(!r.selects("src/thing/tests.rs"));
    assert!(!r.selects("src/tests/helper.rs"));
    assert!(r.selects("src/thing.rs"));
}

#[test]
fn skip_tests_does_not_drop_a_file_that_merely_contains_test() {
    let mut r = rule(&[""]);
    r.skip_tests = true;
    assert!(r.selects("src/attestations.rs"));
    assert!(r.selects("src/latest.rs"));
}

#[test]
fn extension_match_is_exact_not_a_suffix() {
    assert!(has_extension("a.rs", &["rs".into()]));
    assert!(!has_extension("a.rss", &["rs".into()]));
    assert!(!has_extension("norsextension", &["rs".into()]));
}

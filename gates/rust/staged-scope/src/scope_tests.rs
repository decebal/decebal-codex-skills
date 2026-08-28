//! Sibling test file (rules/testing-gates.md — no inline test modules).

use super::*;

fn rules() -> Rules {
    let cfg = Config::parse(
        r#"
[scope.backend]
paths = ["apps/api/", "crates/", "Cargo.toml", "Cargo.lock"]

[scope.frontend]
paths = ["apps/web/", "packages/", "package.json"]

[gates]
inert = ["docs/", "*.md", "LICENSE", ".gitignore"]
"#,
    )
    .expect("parses");
    Rules::from_config(&cfg)
}

fn files(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn a_directory_prefix_matches_everything_under_it() {
    assert!(matches("apps/api/", "apps/api/src/main.rs"));
    assert!(!matches("apps/api/", "apps/apiary/src/main.rs"));
}

#[test]
fn an_exact_pattern_matches_only_that_path() {
    assert!(matches("Cargo.toml", "Cargo.toml"));
    // The root manifest is a scope trigger; a nested one already matched its
    // own directory prefix, and treating it as the root's would widen the diff.
    assert!(!matches("Cargo.toml", "crates/x/Cargo.toml"));
}

#[test]
fn a_glob_matches_across_directory_separators() {
    assert!(matches("*.md", "docs/guides/x.md"));
    assert!(matches("apps/*/src/", "apps/web/src/"));
    assert!(!matches("*.md", "docs/guides/x.mdx"));
}

#[test]
fn a_star_must_not_let_the_prefix_and_suffix_overlap() {
    // "ab" starts with "a" and ends with "b", but there is nothing for `*` to
    // consume in "ab" if the pattern demands three characters.
    assert!(matches("a*b", "ab"));
    assert!(!matches("aa*bb", "ab"));
}

#[test]
fn one_file_can_belong_to_two_scopes() {
    // A shared manifest legitimately triggers both — this is why a scope is a
    // set, not a single verdict.
    let cfg = Config::parse(
        "[scope.a]\npaths = [\"package.json\"]\n[scope.b]\npaths = [\"package.json\"]\n",
    )
    .unwrap();
    assert_eq!(
        Rules::from_config(&cfg).classify(&files(&["package.json"])),
        vec!["a", "b"]
    );
}

#[test]
fn classifies_a_mixed_change_set() {
    assert_eq!(
        rules().classify(&files(&["apps/api/src/main.rs", "apps/web/src/App.svelte"])),
        vec!["backend", "frontend"]
    );
}

#[test]
fn an_inert_path_emits_nothing() {
    assert!(rules()
        .classify(&files(&["docs/guides/x.md", "LICENSE"]))
        .is_empty());
}

#[test]
fn an_unmatched_path_emits_unknown_and_is_named() {
    // The default-deny. A new top-level directory must never run zero gates
    // silently — this is the whole reason the catch-all exists.
    let r = rules();
    let changed = files(&["apps/mobile/src/main.kt"]);
    assert_eq!(r.classify(&changed), vec!["unknown"]);
    assert_eq!(r.unmatched(&changed), vec![&changed[0]]);
}

#[test]
fn a_config_with_no_scopes_reports_everything_as_unknown() {
    // Fails LOUD rather than passing by matching nothing.
    let r = Rules::from_config(&Config::default());
    assert_eq!(r.classify(&files(&["src/main.rs"])), vec!["unknown"]);
}

#[test]
fn a_scope_declared_with_no_paths_is_dropped_rather_than_matching_everything() {
    let cfg = Config::parse("[scope.empty]\npaths = []\n").unwrap();
    assert!(Rules::from_config(&cfg).scopes.is_empty());
}

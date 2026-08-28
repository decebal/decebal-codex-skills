//! Tests for file→crate assignment. Kept in a separate file (no inline
//! `#[cfg(test)] mod tests`) per `rules/testing-gates.md`.

use crate::manifest::CrateEditions;
use crate::plan::build;
use std::path::PathBuf;

fn editions(pairs: &[(&str, &str)]) -> CrateEditions {
    pairs
        .iter()
        .map(|(dir, edition)| (PathBuf::from(dir), (*edition).to_string()))
        .collect()
}

fn files(paths: &[&str]) -> Vec<PathBuf> {
    paths.iter().map(PathBuf::from).collect()
}

#[test]
fn each_file_takes_the_edition_of_its_own_crate() {
    let plan = build(
        &files(&["crates/old/src/lib.rs", "crates/new/src/lib.rs"]),
        &editions(&[("crates/old", "2021"), ("crates/new", "2024")]),
    );
    assert_eq!(plan.groups["2021"], files(&["crates/old/src/lib.rs"]));
    assert_eq!(plan.groups["2024"], files(&["crates/new/src/lib.rs"]));
    assert!(plan.unowned.is_empty());
}

#[test]
fn a_nested_crate_wins_over_the_workspace_above_it() {
    let plan = build(
        &files(&["outer/src/lib.rs", "outer/inner/src/lib.rs"]),
        &editions(&[("outer", "2021"), ("outer/inner", "2024")]),
    );
    assert_eq!(plan.groups["2021"], files(&["outer/src/lib.rs"]));
    assert_eq!(plan.groups["2024"], files(&["outer/inner/src/lib.rs"]));
}

#[test]
fn deeply_nested_sources_still_find_their_crate() {
    let plan = build(
        &files(&["apps/backend/src/a/b/c/d.rs"]),
        &editions(&[("apps/backend", "2021")]),
    );
    assert_eq!(plan.groups["2021"], files(&["apps/backend/src/a/b/c/d.rs"]));
}

#[test]
fn a_file_under_no_crate_is_reported_rather_than_silently_skipped() {
    let plan = build(
        &files(&["scripts/loose.rs", "crates/x/src/lib.rs"]),
        &editions(&[("crates/x", "2021")]),
    );
    assert_eq!(plan.unowned, files(&["scripts/loose.rs"]));
    assert_eq!(plan.groups["2021"], files(&["crates/x/src/lib.rs"]));
}

#[test]
fn groups_are_sorted_so_reports_are_stable() {
    let plan = build(
        &files(&["c/src/z.rs", "c/src/a.rs", "c/src/m.rs"]),
        &editions(&[("c", "2021")]),
    );
    assert_eq!(
        plan.groups["2021"],
        files(&["c/src/a.rs", "c/src/m.rs", "c/src/z.rs"])
    );
}

#[test]
fn a_crate_at_the_repo_root_owns_files_no_deeper_crate_claims() {
    let plan = build(
        &files(&["src/main.rs", "crates/x/src/lib.rs"]),
        &editions(&[("", "2021"), ("crates/x", "2024")]),
    );
    assert_eq!(plan.groups["2021"], files(&["src/main.rs"]));
    assert_eq!(plan.groups["2024"], files(&["crates/x/src/lib.rs"]));
}

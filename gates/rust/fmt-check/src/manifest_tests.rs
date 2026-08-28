//! Tests for manifest parsing and edition resolution. Kept in a separate file
//! (no inline `#[cfg(test)] mod tests`) per `rules/testing-gates.md`.

use crate::manifest::{parse, resolve, EditionSpec, Manifest, Parsed};
use std::path::PathBuf;

fn manifest(dir: &str, text: &str) -> Manifest {
    Manifest {
        dir: PathBuf::from(dir),
        parsed: parse(text),
    }
}

#[test]
fn reads_a_literal_edition() {
    let parsed = parse("[package]\nname = \"x\"\nedition = \"2024\"\n");
    assert!(parsed.has_package);
    assert_eq!(
        parsed.package_edition,
        Some(EditionSpec::Literal("2024".into()))
    );
}

#[test]
fn reads_both_inheritance_spellings() {
    let dotted = parse("[package]\nedition.workspace = true\n");
    assert_eq!(dotted.package_edition, Some(EditionSpec::Inherited));

    let inline = parse("[package]\nedition = { workspace = true }\n");
    assert_eq!(inline.package_edition, Some(EditionSpec::Inherited));
}

#[test]
fn a_workspace_table_marks_the_root_and_carries_its_edition() {
    let parsed =
        parse("[workspace]\nmembers = [\"api\"]\n\n[workspace.package]\nedition = \"2021\"\n");
    assert!(parsed.is_workspace_root);
    assert!(!parsed.has_package);
    assert_eq!(parsed.workspace_edition.as_deref(), Some("2021"));
}

#[test]
fn a_workspace_edition_is_not_mistaken_for_a_package_edition() {
    let parsed = parse("[workspace.package]\nedition = \"2021\"\n");
    assert_eq!(parsed.package_edition, None);
    assert!(!parsed.has_package);
}

#[test]
fn a_commented_out_edition_is_ignored() {
    let parsed = parse("[package]\n# edition = \"2015\"\nedition = \"2024\" # the real one\n");
    assert_eq!(
        parsed.package_edition,
        Some(EditionSpec::Literal("2024".into()))
    );
}

#[test]
fn a_hash_inside_a_string_does_not_truncate_the_line() {
    let parsed = parse("[package]\ndescription = \"tracks # of runs\"\nedition = \"2021\"\n");
    assert_eq!(
        parsed.package_edition,
        Some(EditionSpec::Literal("2021".into()))
    );
}

#[test]
fn a_bin_table_does_not_leak_into_the_package_table() {
    let parsed = parse("[package]\nedition = \"2021\"\n\n[[bin]]\nname = \"x\"\n");
    assert_eq!(
        parsed.package_edition,
        Some(EditionSpec::Literal("2021".into()))
    );
    assert!(parsed.has_package);
}

#[test]
fn inheritance_resolves_against_the_nearest_workspace_root() {
    let manifests = vec![
        manifest("", "[workspace]\n[workspace.package]\nedition = \"2015\"\n"),
        manifest(
            "services",
            "[workspace]\n[workspace.package]\nedition = \"2021\"\n",
        ),
        manifest("services/api", "[package]\nedition.workspace = true\n"),
    ];
    let editions = resolve(&manifests).expect("resolves");
    assert_eq!(
        editions
            .get(&PathBuf::from("services/api"))
            .map(String::as_str),
        Some("2021")
    );
}

#[test]
fn a_virtual_manifest_contributes_no_crate() {
    let manifests = vec![manifest("", "[workspace]\nmembers = [\"a\"]\n")];
    let editions = resolve(&manifests).expect("resolves");
    assert!(editions.is_empty());
}

#[test]
fn a_package_with_no_edition_is_an_error_not_a_default() {
    let manifests = vec![manifest("crates/x", "[package]\nname = \"x\"\n")];
    let errors = resolve(&manifests).expect_err("must not guess");
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("crates/x/Cargo.toml"), "{}", errors[0]);
    assert!(errors[0].contains("no `edition` key"), "{}", errors[0]);
}

#[test]
fn unresolvable_inheritance_is_an_error() {
    let manifests = vec![
        manifest("", "[workspace]\nmembers = [\"crates/x\"]\n"),
        manifest("crates/x", "[package]\nedition.workspace = true\n"),
    ];
    let errors = resolve(&manifests).expect_err("no workspace edition to inherit");
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("inherits `edition`"), "{}", errors[0]);
}

#[test]
fn a_crate_that_is_its_own_workspace_root_inherits_from_itself() {
    let manifests = vec![manifest(
        "tooling/standalone",
        "[workspace]\n[workspace.package]\nedition = \"2024\"\n\n[package]\nedition.workspace = true\n",
    )];
    let editions = resolve(&manifests).expect("resolves");
    assert_eq!(
        editions
            .get(&PathBuf::from("tooling/standalone"))
            .map(String::as_str),
        Some("2024")
    );
}

#[test]
fn an_empty_manifest_parses_to_nothing() {
    assert_eq!(parse(""), Parsed::default());
}

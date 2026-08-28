use super::{is_license_file, kind_of, license, name, Kind};

const CARGO: &str = "[package]\nname = \"tiny-http\"\nversion = \"0.12.0\"\nlicense = \"MIT\"\n";
const NPM: &str = "{\n  \"name\": \"left-pad\",\n  \"license\": \"WTFPL\"\n}\n";

#[test]
fn reads_a_cargo_name_and_license() {
    assert_eq!(name(CARGO, Kind::Cargo).as_deref(), Some("tiny-http"));
    assert_eq!(license(CARGO, Kind::Cargo).as_deref(), Some("MIT"));
}

#[test]
fn reads_an_npm_name_and_license() {
    assert_eq!(name(NPM, Kind::Npm).as_deref(), Some("left-pad"));
    assert_eq!(license(NPM, Kind::Npm).as_deref(), Some("WTFPL"));
}

#[test]
fn a_missing_license_field_is_none_not_an_empty_string() {
    let text = "[package]\nname = \"x\"\n";
    assert_eq!(license(text, Kind::Cargo), None);
}

#[test]
fn an_empty_license_string_counts_as_missing() {
    let text = "[package]\nname = \"x\"\nlicense = \"\"\n";
    assert_eq!(license(text, Kind::Cargo), None);
}

#[test]
fn license_file_stands_in_for_an_spdx_expression() {
    let text = "[package]\nname = \"x\"\nlicense-file = \"LICENSE-CUSTOM\"\n";
    assert_eq!(
        license(text, Kind::Cargo).as_deref(),
        Some("LICENSE-CUSTOM")
    );
}

#[test]
fn a_key_that_merely_starts_the_same_is_not_matched() {
    let text = "[package]\nname_of_thing = \"wrong\"\nname = \"right\"\n";
    assert_eq!(name(text, Kind::Cargo).as_deref(), Some("right"));
}

#[test]
fn a_trailing_comma_does_not_end_up_in_an_npm_value() {
    let text = "{\n  \"name\": \"a\",\n  \"license\": \"MIT\",\n}\n";
    assert_eq!(license(text, Kind::Npm).as_deref(), Some("MIT"));
}

#[test]
fn manifest_kind_is_decided_by_file_name() {
    assert_eq!(kind_of("Cargo.toml"), Some(Kind::Cargo));
    assert_eq!(kind_of("package.json"), Some(Kind::Npm));
    assert_eq!(kind_of("go.mod"), None);
}

#[test]
fn license_files_are_recognised_under_every_common_spelling() {
    assert!(is_license_file("LICENSE"));
    assert!(is_license_file("LICENSE-MIT"));
    assert!(is_license_file("licence.txt"));
    assert!(is_license_file("COPYING"));
    assert!(!is_license_file("README.md"));
}

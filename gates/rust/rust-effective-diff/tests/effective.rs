//! Behavioral tests for the effective-diff classifier. Non-effective changes
//! (comments, doc comments, whitespace, formatting) must return `false`; any
//! change the compiler or runtime cares about must return `true`.

use rust_effective_diff::is_effective;

const BASE: &str = r#"
//! Module doc citing ADR-016.
use std::fmt;

/// Doc for foo, see ADR-007.
pub fn foo(x: u32) -> u32 {
    // a line comment mentioning ADR-064
    let msg = "connect failure per ADR-016";
    x + 1 // trailing comment
}
"#;

#[test]
fn identical_is_not_effective() {
    assert!(!is_effective(BASE, BASE));
}

#[test]
fn line_comment_change_is_not_effective() {
    let changed = BASE.replace(
        "// a line comment mentioning ADR-064",
        "// a line comment mentioning ADR-065",
    );
    assert!(!is_effective(BASE, &changed));
}

#[test]
fn doc_comment_change_is_not_effective() {
    let changed = BASE
        .replace("Module doc citing ADR-016.", "Module doc citing ADR-068.")
        .replace("Doc for foo, see ADR-007.", "Doc for foo, see ADR-066.");
    assert!(!is_effective(BASE, &changed));
}

#[test]
fn whitespace_and_formatting_change_is_not_effective() {
    let changed = BASE.replace(
        "x + 1 // trailing comment",
        "x    +    1 // trailing comment",
    );
    assert!(!is_effective(BASE, &changed));
}

#[test]
fn adding_blank_lines_is_not_effective() {
    let changed = format!("{BASE}\n\n\n");
    assert!(!is_effective(BASE, &changed));
}

#[test]
fn string_literal_change_is_effective() {
    // The ADR reference lives inside a real string literal — changing it
    // changes program data and MUST be treated as effective.
    let changed = BASE.replace("connect failure per ADR-016", "connect failure per ADR-068");
    assert!(is_effective(BASE, &changed));
}

#[test]
fn code_change_is_effective() {
    let changed = BASE.replace("x + 1", "x + 2");
    assert!(is_effective(BASE, &changed));
}

#[test]
fn identifier_rename_is_effective() {
    let changed = BASE.replace("pub fn foo", "pub fn bar");
    assert!(is_effective(BASE, &changed));
}

#[test]
fn real_attribute_change_is_effective() {
    let a = "#[derive(Debug)]\nstruct S;\n";
    let b = "#[derive(Debug, Clone)]\nstruct S;\n";
    assert!(is_effective(a, b));
}

#[test]
fn cfg_attribute_change_is_effective() {
    let a = "#[cfg(target_os = \"macos\")]\nfn p() {}\n";
    let b = "#[cfg(target_os = \"windows\")]\nfn p() {}\n";
    assert!(is_effective(a, b));
}

#[test]
fn unparsable_input_is_effective() {
    // A partial/invalid edit that does not tokenize — conservative default.
    let a = "fn ok() {}\n";
    let b = "fn ok() { let s = \"unterminated;\n";
    assert!(is_effective(a, b));
}

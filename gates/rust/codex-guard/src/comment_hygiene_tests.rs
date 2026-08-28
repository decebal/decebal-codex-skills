//! Sibling test file (rules/testing-gates.md — no inline test modules).

use super::*;

fn slash() -> &'static [&'static str] {
    markers_for("a.ts").unwrap()
}

#[test]
fn picks_the_marker_set_by_extension() {
    assert_eq!(markers_for("a.rs"), markers_for("a.ts"));
    assert_eq!(markers_for("a.py"), Some(&["#"][..]));
    assert_eq!(markers_for("a.pug"), Some(&["//-", "//"][..]));
    // `#` opens a comment in shell but names a private field in TypeScript,
    // which is the whole reason the sets differ.
    assert!(!markers_for("a.ts").unwrap().contains(&"#"));
}

#[test]
fn a_file_type_with_no_marker_set_is_out_of_scope() {
    assert_eq!(markers_for("a.md"), None);
    assert_eq!(markers_for("LICENSE"), None);
}

#[test]
fn flags_a_line_leading_comment() {
    let found = comment_lines("// set up the listener\nconst a = 1", slash());
    assert_eq!(found, vec!["// set up the listener"]);
}

#[test]
fn a_marker_inside_a_line_is_not_a_comment() {
    let found = comment_lines("const u = \"https://x.dev\"", slash());
    assert!(found.is_empty());
}

#[test]
fn doc_comments_are_api_surface_and_are_left_alone() {
    let src = "/// doc\n//! module doc\n/** jsdoc */\n#!/usr/bin/env bash\n// narration";
    assert_eq!(comment_lines(src, slash()), vec!["// narration"]);
}

#[test]
fn indented_comments_still_count() {
    let found = comment_lines("    // indented", slash());
    assert_eq!(found.len(), 1);
}

#[test]
fn the_line_count_is_capped() {
    let src = "// x\n".repeat(60);
    assert_eq!(comment_lines(&src, slash()).len(), MAX_LINES);
}

#[test]
fn the_message_carries_the_count_the_path_and_the_lines() {
    let out = context_for("src/a.ts", &["// one".into(), "// two".into()]);
    assert!(out.contains("2 comment line(s) added to src/a.ts"));
    assert!(out.contains("// one\n// two"));
    assert!(
        out.contains("contract, risk, or non-obvious reason"),
        "{out}"
    );
}

#[test]
fn parses_codex_apply_patch_payload_by_file() {
    let patch = "*** Begin Patch\n*** Update File: src/a.ts\n@@\n+// explain the invariant\n+const x = 1;\n*** Add File: src/b.py\n+# explain the boundary\n+value = 2\n*** End Patch";
    let found = patch_comments(patch);
    assert_eq!(found.len(), 2);
    assert_eq!(
        found[0],
        ("src/a.ts".into(), vec!["// explain the invariant".into()])
    );
    assert_eq!(
        found[1],
        ("src/b.py".into(), vec!["# explain the boundary".into()])
    );
}

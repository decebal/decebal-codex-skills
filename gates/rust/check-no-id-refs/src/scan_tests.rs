//! Sibling test file (rules/testing-gates.md — no inline test modules).
//!
//! Fixtures assemble their ids at RUNTIME (`format!("{}{}", "t-", "d0d1d6")`)
//! rather than writing them as literals. A literal would put a matching line in
//! this file, and the gate reads its own repo — so the gate would flag its own
//! tests. The alternative was an allowlist entry, which is worse: an allowlist
//! is a place for the next exception to hide, and this costs one `format!`.

use super::*;

fn shapes() -> Vec<String> {
    DEFAULT_ID_SHAPES.iter().map(|s| s.to_string()).collect()
}
fn re() -> Regex {
    pattern(&shapes()).unwrap()
}
fn exts() -> Vec<String> {
    DEFAULT_EXTENSIONS.iter().map(|s| s.to_string()).collect()
}
/// An id of the default `t-` shape, never written as a literal here.
fn task_id() -> String {
    format!("{}{}", "t-", "d0d1d6")
}
/// An id of the default `bd-` shape.
fn bead_id() -> String {
    format!("{}{}", "bd-", "91f")
}

#[test]
fn flags_an_id_in_a_line_comment() {
    let src = format!("// Prevention ({}): re-check the branch.\n", task_id());
    let hits = scan("a.rs", &src, &re());
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].line, 1);
}

#[test]
fn flags_an_id_in_a_doc_continuation_line() {
    let src = format!(" * see {} for the rest\n", bead_id());
    assert_eq!(scan("a.rs", &src, &re()).len(), 1);
}

#[test]
fn flags_an_id_in_an_html_comment() {
    let src = format!("<!-- {} -->\n", task_id());
    assert_eq!(scan("a.svelte", &src, &re()).len(), 1);
}

#[test]
fn an_id_shaped_string_literal_is_not_a_comment() {
    // An app's own task-run ids and password fixtures are id-shaped and
    // legitimate. Anchoring on a comment opener is what keeps them out.
    let src = format!(
        "let id = \"{}-pw\";\nlet run = format!(\"t-{{}}\", n);\n",
        task_id()
    );
    assert!(scan("a.rs", &src, &re()).is_empty());
}

#[test]
fn a_decision_record_reference_is_never_matched() {
    // Different prefix on purpose: ADR-007 is the thing a comment SHOULD cite.
    assert!(scan("a.rs", "// see ADR-007 for why\n", &re()).is_empty());
}

#[test]
fn a_word_boundary_stops_a_partial_match() {
    assert!(scan("a.rs", "// the next-1a2b release\n", &re()).is_empty());
}

#[test]
fn reads_source_extensions_and_skips_vendored_trees() {
    assert!(has_scanned_extension("src/a.rs", &exts()));
    assert!(has_scanned_extension("web/a.svelte", &exts()));
    // Markdown is excluded on purpose — docs cite ids legitimately.
    assert!(!has_scanned_extension("docs/a.md", &exts()));
    assert!(!has_scanned_extension("node_modules/x/a.js", &exts()));
    assert!(!has_scanned_extension("target/debug/a.rs", &exts()));
    assert!(!has_scanned_extension("LICENSE", &exts()));
}

#[test]
fn a_custom_shape_is_honoured() {
    let re = pattern(&[r"[A-Z]{2,}-\d+".to_string()]).unwrap();
    assert_eq!(scan("a.rs", "// fixes PROJ-123\n", &re).len(), 1);
}

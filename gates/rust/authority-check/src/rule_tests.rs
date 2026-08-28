use super::{scan, undefined_markers, Kind, Rule};

fn rule() -> Rule {
    Rule {
        name: "tool-capability".into(),
        authority: "src/rust/capabilities.rs".into(),
        renderers: vec!["app/src/".into()],
        extensions: vec!["ts".into(), "svelte".into()],
        markers: vec!["TOOL_CAPABLE_PREFIXES".into(), "modelSupportsTools".into()],
        contract: Some("app/src/wire.ts".into()),
        retired: vec!["disabled_workflow_only".into()],
        why: "Render the verdict.".into(),
    }
}

#[test]
fn a_renderer_reconstructing_the_oracle_is_a_violation() {
    let hits = scan(
        "app/src/a.ts",
        "const x = modelSupportsTools(id)\n",
        &rule(),
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].kind, Kind::Rederived);
}

#[test]
fn the_authority_may_name_its_own_markers() {
    let text = "const TOOL_CAPABLE_PREFIXES = []\n";
    assert!(scan("src/rust/capabilities.rs", text, &rule()).is_empty());
}

#[test]
fn the_wire_contract_may_name_the_markers_too() {
    let text = "export const modelSupportsTools = 1\n";
    assert!(scan("app/src/wire.ts", text, &rule()).is_empty());
}

#[test]
fn a_retired_symbol_is_forbidden_even_in_the_authority() {
    let hits = scan(
        "src/rust/capabilities.rs",
        "let s = disabled_workflow_only;\n",
        &rule(),
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].kind, Kind::Retired);
}

#[test]
fn a_retired_symbol_is_forbidden_in_the_contract_too() {
    let hits = scan("app/src/wire.ts", "disabled_workflow_only\n", &rule());
    assert_eq!(hits.len(), 1);
}

#[test]
fn a_comment_explaining_the_ban_is_not_a_violation() {
    let text = "// modelSupportsTools was removed; read `tool_capable`\n";
    assert!(scan("app/src/a.ts", text, &rule()).is_empty());
}

#[test]
fn selects_only_renderer_trees_of_the_declared_extensions() {
    let r = rule();
    assert!(r.selects("app/src/a.ts"));
    assert!(r.selects("app/src/A.svelte"));
    assert!(!r.selects("app/src/a.rs"));
    assert!(!r.selects("other/a.ts"));
}

#[test]
fn test_files_are_exempt_so_a_removal_assertion_can_name_the_symbol() {
    let r = rule();
    assert!(r.is_exempt_test("app/src/__tests__/a.ts"));
    assert!(r.is_exempt_test("app/src/a.vitest.ts"));
    assert!(!r.is_exempt_test("app/src/attest.ts"));
}

#[test]
fn a_marker_the_authority_no_longer_defines_is_reported() {
    let missing = undefined_markers("const TOOL_CAPABLE_PREFIXES = []\n", &rule());
    assert_eq!(missing, vec!["modelSupportsTools".to_string()]);
}

#[test]
fn an_authority_defining_every_marker_reports_nothing_missing() {
    let text = "TOOL_CAPABLE_PREFIXES modelSupportsTools\n";
    assert!(undefined_markers(text, &rule()).is_empty());
}

#[test]
fn every_offending_line_is_reported_not_only_the_first() {
    let text = "modelSupportsTools(a)\nTOOL_CAPABLE_PREFIXES\n";
    assert_eq!(scan("app/src/a.ts", text, &rule()).len(), 2);
}

use super::{imports_in, is_test_file, Edge, Layer, Model};
use std::collections::BTreeMap;

fn layer(name: &str) -> Layer {
    Layer {
        name: name.into(),
        dir: format!("src/{name}/"),
    }
}

fn model() -> Model {
    Model {
        order: vec![
            layer("presentation"),
            layer("application"),
            layer("infrastructure"),
        ],
        pure: vec![layer("domain")],
        import_template: "use crate::{layer}".into(),
        ceilings: BTreeMap::new(),
        facades: BTreeMap::new(),
    }
}

fn has(edges: &[Edge], from: &str, to: &str) -> bool {
    edges.iter().any(|e| e.from.name == from && e.to.name == to)
}

#[test]
fn a_layer_may_not_import_one_before_it() {
    let edges = model().forbidden_edges();
    assert!(has(&edges, "application", "presentation"));
    assert!(has(&edges, "infrastructure", "application"));
    assert!(has(&edges, "infrastructure", "presentation"));
}

#[test]
fn importing_downward_is_not_an_edge() {
    let edges = model().forbidden_edges();
    assert!(!has(&edges, "presentation", "application"));
    assert!(!has(&edges, "application", "infrastructure"));
}

#[test]
fn a_pure_layer_may_import_no_other_layer() {
    let edges = model().forbidden_edges();
    assert!(has(&edges, "domain", "presentation"));
    assert!(has(&edges, "domain", "application"));
    assert!(has(&edges, "domain", "infrastructure"));
}

#[test]
fn a_layer_never_forbids_importing_itself() {
    assert!(!model()
        .forbidden_edges()
        .iter()
        .any(|e| e.from.name == e.to.name));
}

#[test]
fn a_file_is_owned_by_the_layer_with_the_longest_matching_dir() {
    let mut m = model();
    m.order[1].dir = "src/".into();
    let owner = m.owning_layer("src/domain/a.rs").unwrap();
    assert_eq!(owner.name, "domain");
}

#[test]
fn a_file_under_only_one_layer_is_owned_by_it() {
    let m = model();
    assert_eq!(
        m.owning_layer("src/application/a.rs").unwrap().name,
        "application"
    );
}

#[test]
fn a_file_under_no_declared_layer_is_owned_by_none() {
    let m = model();
    assert!(m.owning_layer("tooling/x.rs").is_none());
}

#[test]
fn owning_layer_reaches_pure_layers_too() {
    let m = model();
    assert_eq!(m.owning_layer("src/domain/a.rs").unwrap().name, "domain");
}

#[test]
fn the_import_needle_substitutes_the_target_layer() {
    let m = model();
    let edge = Edge {
        from: layer("application"),
        to: layer("presentation"),
    };
    assert_eq!(m.import_of(&edge), "use crate::presentation");
}

#[test]
fn a_ceiling_defaults_to_zero_and_is_keyed_by_edge() {
    let mut m = model();
    m.ceilings.insert("application_to_presentation".into(), 7);
    let edge = Edge {
        from: layer("application"),
        to: layer("presentation"),
    };
    assert_eq!(m.ceiling_for(&edge), 7);
    let other = Edge {
        from: layer("infrastructure"),
        to: layer("presentation"),
    };
    assert_eq!(m.ceiling_for(&other), 0);
}

#[test]
fn finds_an_import_and_reports_a_one_based_line() {
    let text = "mod a;\nuse crate::presentation::Foo;\n";
    let hits = imports_in("src/application/a.rs", text, "use crate::presentation");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].line, 2);
}

#[test]
fn a_commented_import_is_not_a_violation() {
    let text = "// use crate::presentation::Foo;\n/* use crate::presentation */\n";
    assert!(imports_in("src/application/a.rs", text, "use crate::presentation").is_empty());
}

#[test]
fn everything_from_the_first_cfg_test_down_is_test_code() {
    let text = "use crate::x;\n#[cfg(test)]\nmod t {\n use crate::presentation::Foo;\n}\n";
    assert!(imports_in("src/application/a.rs", text, "use crate::presentation").is_empty());
}

#[test]
fn code_above_a_cfg_test_block_is_still_read() {
    let text = "use crate::presentation::Foo;\n#[cfg(test)]\nmod t {}\n";
    assert_eq!(
        imports_in("src/application/a.rs", text, "use crate::presentation").len(),
        1
    );
}

#[test]
fn test_files_are_recognised_by_every_convention_the_rules_allow() {
    assert!(is_test_file("src/a/tests.rs"));
    assert!(is_test_file("src/a_tests.rs"));
    assert!(is_test_file("tests/integration.rs"));
    assert!(!is_test_file("src/attestations.rs"));
}

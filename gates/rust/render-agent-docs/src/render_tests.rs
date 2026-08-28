//! Sibling test file (rules/testing-gates.md — no inline test modules).

use super::*;

fn manifest() -> Manifest {
    let cfg = Config::parse(
        r#"
rules = ["git-discipline", "timeouts"]
rules_dir = "rules"
overlay = "docs/overlay.md"

[targets.agents]
path = "AGENTS.md"
title = "AGENTS.md — Acme"
"#,
    )
    .expect("parses");
    Manifest::from_config(&cfg)
}

fn rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "git-discipline".into(),
            text: "# Git discipline\n\nCheck branch liveness.".into(),
        },
        Rule {
            name: "timeouts".into(),
            text: "# Timeouts\n\nBound every gate.".into(),
        },
    ]
}

#[test]
fn reads_target_and_rule_source_from_manifest() {
    let m = manifest();
    assert_eq!(m.targets.len(), 1);
    assert_eq!(m.targets[0].path, "AGENTS.md");
    assert_eq!(m.rules_dir, "rules");
}

#[test]
fn embeds_rules_in_declared_order_without_markdown_imports() {
    let m = manifest();
    let out = render(&m.targets[0], None, &rules());
    let git = out.find("### git-discipline").expect("git rule");
    let timeouts = out.find("### timeouts").expect("timeouts rule");
    assert!(git < timeouts);
    assert!(out.contains("Check branch liveness."));
    assert!(out.contains("Bound every gate."));
    assert!(!out.lines().any(|line| line.starts_with('@')));
}

#[test]
fn carries_banner_and_overlay_after_rules() {
    let m = manifest();
    let out = render(&m.targets[0], Some("## Stack\n\nRust."), &rules());
    assert!(out.contains(BANNER));
    assert!(out.find("Bound every gate.") < out.find("## Stack"));
}

#[test]
fn target_without_title_falls_back_to_path() {
    let cfg = Config::parse("[targets.a]\npath = \"A.md\"\n").unwrap();
    let m = Manifest::from_config(&cfg);
    assert_eq!(m.targets[0].title, "A.md");
}

#[test]
fn no_rules_means_no_rules_section() {
    let cfg = Config::parse("[targets.a]\npath = \"A.md\"\n").unwrap();
    let m = Manifest::from_config(&cfg);
    assert!(!render(&m.targets[0], Some("body"), &[]).contains("## Rules"));
}

#[test]
fn strips_rule_h1_but_keeps_body() {
    assert_eq!(rule_body("# Rule title\n\nRule body."), "Rule body.");
    assert_eq!(rule_body("Rule body."), "Rule body.");
}

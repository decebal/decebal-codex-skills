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
    Manifest::from_config(&cfg).expect("valid manifest")
}

fn target_with(max_bytes: &str) -> Result<Manifest, String> {
    let cfg = Config::parse(&format!(
        "[targets.agents]\npath = \"AGENTS.md\"\nmax_bytes = \"{max_bytes}\"\n"
    ))
    .expect("parses");
    Manifest::from_config(&cfg)
}

fn rule(name: &str, text: &str) -> Rule {
    Rule {
        name: name.into(),
        text: text.into(),
    }
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
    let out = render(&m.targets[0], None, &rules(), true);
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
    let out = render(&m.targets[0], Some("## Stack\n\nRust."), &rules(), true);
    assert!(out.contains(BANNER));
    assert!(out.find("Bound every gate.") < out.find("## Stack"));
}

#[test]
fn target_without_title_falls_back_to_path() {
    let cfg = Config::parse("[targets.a]\npath = \"A.md\"\n").unwrap();
    let m = Manifest::from_config(&cfg).unwrap();
    assert_eq!(m.targets[0].title, "A.md");
}

#[test]
fn no_rules_means_no_rules_section() {
    let cfg = Config::parse("[targets.a]\npath = \"A.md\"\n").unwrap();
    let m = Manifest::from_config(&cfg).unwrap();
    assert!(!render(&m.targets[0], Some("body"), &[], true).contains("## Rules"));
}

#[test]
fn strips_rule_h1_but_keeps_body() {
    assert_eq!(rule_body("# Rule title\n\nRule body."), "Rule body.");
    assert_eq!(rule_body("Rule body."), "Rule body.");
}

#[test]
fn a_path_scoped_rule_is_named_not_inlined() {
    let m = manifest();
    let rules = vec![
        rule("git-discipline", "# Git\n\nCheck branch liveness."),
        rule(
            "testing-authoring",
            "---\npaths:\n  - \"**/tests/**\"\n---\n\n# Testing\n\nOne process per test.",
        ),
        rule(
            "dependency-hygiene",
            "---\npaths: [\"**/Cargo.toml\"]\n---\n# Deps\n\nPin it.",
        ),
        rule("timeouts", "# Timeouts\n\nBound every gate."),
    ];
    let out = render(&m.targets[0], Some("## Stack"), &rules, true);
    assert!(!out.contains("One process per test."), "{out}");
    assert!(!out.contains("### testing-authoring"), "{out}");
    assert!(!out.contains("Pin it."), "{out}");
    let line = "Path-scoped, loaded by `codex-guard rule-scope` when a call touches a matching file: testing-authoring, dependency-hygiene.";
    let listed = out.find(line).unwrap_or_else(|| panic!("{out}"));
    assert!(out.find("Bound every gate.").unwrap() < listed);
    assert!(listed < out.find("## Stack").unwrap());

    let inlined = render(&m.targets[0], None, &rules, false);
    assert!(
        inlined.contains("### testing-authoring\n\nOne process per test."),
        "{inlined}"
    );
    assert!(!inlined.contains("paths:"), "{inlined}");
    assert!(!inlined.contains("Path-scoped"), "{inlined}");
}

#[test]
fn only_a_directory_rule_scope_reads_may_defer_scoped_rules() {
    let home = Path::new("/home/u/.codex");
    assert!(rule_scope_reads(Path::new(".agents/rules"), Some(home)));
    assert!(rule_scope_reads(
        Path::new("/r/widgets/.claude/rules"),
        None
    ));
    assert!(rule_scope_reads(
        Path::new("/home/u/.codex/agent-rules"),
        Some(home)
    ));
    assert!(!rule_scope_reads(Path::new("rules"), Some(home)));
    assert!(!rule_scope_reads(
        Path::new("/r/widgets/docs/rules"),
        Some(home)
    ));
    assert!(!rule_scope_reads(
        Path::new("/home/u/.codex/rules"),
        Some(home)
    ));
}

#[test]
fn frontmatter_without_paths_is_stripped_and_the_rule_inlined() {
    let m = manifest();
    let rules = vec![rule(
        "comments",
        "---\ndescription: comment tests\n---\n# Comments\n\nSay what the code cannot.",
    )];
    let out = render(&m.targets[0], None, &rules, true);
    assert!(
        out.contains("### comments\n\nSay what the code cannot."),
        "{out}"
    );
    assert!(!out.contains("description:"), "{out}");
    assert!(!out.contains("Path-scoped"), "{out}");
}

#[test]
fn max_bytes_defaults_to_codex_project_doc_max_bytes() {
    assert_eq!(manifest().targets[0].max_bytes, DEFAULT_MAX_BYTES);
    assert_eq!(DEFAULT_MAX_BYTES, 32_768);
    assert_eq!(target_with("0").unwrap().targets[0].max_bytes, 0);
    assert_eq!(target_with(" 4096 ").unwrap().targets[0].max_bytes, 4096);
    let err = target_with("32k").unwrap_err();
    assert!(err.contains("targets.agents.max_bytes"), "{err}");
}

#[test]
fn a_target_over_its_cap_is_named_with_size_and_cap() {
    let mut t = manifest().targets[0].clone();
    t.max_bytes = 10;
    assert!(over_cap(&t, "0123456789").is_none());
    let why = over_cap(&t, "0123456789x").expect("over");
    assert!(why.contains("AGENTS.md (target agents)"), "{why}");
    assert!(why.contains("11 bytes"), "{why}");
    assert!(why.contains("max_bytes of 10"), "{why}");
    assert!(why.contains("project_doc_max_bytes"), "{why}");

    t.max_bytes = 0;
    assert!(over_cap(&t, &"x".repeat(1_000_000)).is_none());
}

use super::{expand_glob, workspace_globs, Package, Requirement};
use std::path::{Path, PathBuf};

fn tests_requirement() -> Requirement {
    Requirement {
        script: "test".into(),
        stems: vec![".test".into(), "_test".into(), ".vitest".into()],
        extensions: vec!["ts".into(), "tsx".into()],
        fix: "add a test script".into(),
    }
}

fn typecheck_requirement() -> Requirement {
    Requirement {
        script: "typecheck".into(),
        stems: Vec::new(),
        extensions: vec!["ts".into(), "svelte".into()],
        fix: "add a typecheck script".into(),
    }
}

fn package(files: &[&str], declared: &[&str]) -> Package {
    Package {
        name: "@scope/pkg".into(),
        dir: PathBuf::from("packages/pkg"),
        declared: declared.iter().map(|s| (*s).to_string()).collect(),
        files: files.iter().map(PathBuf::from).collect(),
    }
}

#[test]
fn a_stemmed_requirement_matches_only_suite_names() {
    let r = tests_requirement();
    assert!(r.covers("a.test.ts"));
    assert!(r.covers("a_test.ts"));
    assert!(r.covers("a.vitest.ts"));
    assert!(!r.covers("a.ts"));
}

#[test]
fn a_stemless_requirement_matches_any_file_of_its_extensions() {
    let r = typecheck_requirement();
    assert!(r.covers("a.ts"));
    assert!(r.covers("A.svelte"));
    assert!(!r.covers("a.js"));
}

#[test]
fn extension_gates_the_stem_match() {
    assert!(!tests_requirement().covers("a.test.py"));
}

#[test]
fn evidence_lists_the_files_the_script_would_have_covered() {
    let p = package(&["packages/pkg/a.test.ts", "packages/pkg/b.ts"], &[]);
    assert_eq!(p.evidence(&tests_requirement()).len(), 1);
    assert_eq!(p.evidence(&typecheck_requirement()).len(), 2);
}

#[test]
fn a_declared_script_is_recognised() {
    let p = package(&[], &["test", "build"]);
    assert!(p.declares("test"));
    assert!(!p.declares("typecheck"));
}

#[test]
fn a_package_with_evidence_and_no_script_is_the_violation() {
    let p = package(&["packages/pkg/a.test.ts"], &["build"]);
    let r = tests_requirement();
    assert!(!p.declares(&r.script));
    assert!(!p.evidence(&r).is_empty());
}

#[test]
fn a_package_with_no_evidence_is_not_a_violation_however_few_scripts_it_has() {
    let p = package(&["packages/pkg/README.md"], &[]);
    assert!(p.evidence(&tests_requirement()).is_empty());
}

#[test]
fn an_unsupported_glob_is_an_error_not_a_silent_skip() {
    let err = expand_glob(Path::new("."), "packages/*/src").unwrap_err();
    assert!(err.contains("unsupported"), "{err}");
}

#[test]
fn a_literal_workspace_path_that_does_not_exist_expands_to_nothing() {
    let dirs = expand_glob(Path::new("."), "definitely-not-here").unwrap();
    assert!(dirs.is_empty());
}

#[test]
fn a_root_manifest_without_workspaces_is_an_error() {
    let dir = std::env::temp_dir().join("test-script-check-no-workspaces");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("package.json"), "{\"name\":\"root\"}").unwrap();
    let err = workspace_globs(&dir).unwrap_err();
    assert!(err.contains("workspaces"), "{err}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_pnpm_style_nested_workspaces_object_is_read() {
    let dir = std::env::temp_dir().join("test-script-check-pnpm-style");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("package.json"),
        "{\"workspaces\":{\"packages\":[\"packages/*\"]}}",
    )
    .unwrap();
    assert_eq!(workspace_globs(&dir).unwrap(), vec!["packages/*"]);
    std::fs::remove_dir_all(&dir).ok();
}

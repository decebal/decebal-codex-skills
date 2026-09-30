//! What rule-scope delivers, when, and in what shape.

use super::{
    command_paths, compose, decide, forget, git_root, glob_regex, load_rules, match_key,
    parse_rule, patch_paths, paths_globs, rule_dirs, split_frontmatter, Rule, HEADER,
    MAX_CANDIDATES, MAX_CONTEXT,
};
use crate::hook;
use serde_json::{json, Value};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

struct Fixture {
    home: PathBuf,
    root: PathBuf,
}

/// A Codex home and a git repo, each holding rules.
fn fixture(tag: &str) -> Fixture {
    let base = std::env::temp_dir().join(format!("codex-guard-rules-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let home = base.join("codex-home");
    let root = base.join("widgets");
    std::fs::create_dir_all(home.join("agent-rules")).unwrap();
    std::fs::create_dir_all(root.join(".git")).unwrap();
    std::fs::create_dir_all(root.join(".claude").join("rules")).unwrap();
    std::fs::create_dir_all(root.join(".agents").join("rules")).unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();

    write(
        &home.join("agent-rules").join("dependency-hygiene.md"),
        "---\npaths:\n  - \"**/Cargo.toml\"\n  - '**/package.json'\n---\n\n# Dependencies\n\nOne crate at a time.\n",
    );
    write(
        &home.join("agent-rules").join("git-discipline.md"),
        "# Git discipline\n\nNo frontmatter: inlined into AGENTS.md instead.\n",
    );
    write(
        &root.join(".claude").join("rules").join("testing.md"),
        "---\ndescription: tests\npaths: [\"**/*_tests.rs\", \"**/tests/**\"]\n---\nRun the scoped suite.\n",
    );
    write(
        &root.join(".agents").join("rules").join("ui.md"),
        "---\npaths:\n  - src/**/*.{svelte,ts}\n---\nUse the shared Button.\n",
    );
    Fixture { home, root }
}

fn write(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap();
}

fn call(tool: &str, command: &str, cwd: &Path, session: &str) -> Value {
    json!({"session_id": session, "turn_id": "t1", "transcript_path": null,
           "cwd": cwd.to_str().unwrap(), "hook_event_name": "PreToolUse", "model": "gpt-5",
           "permission_mode": "default", "tool_name": tool,
           "tool_input": {"command": command}, "tool_use_id": "call_1"})
}

fn patch(path: &str) -> String {
    format!("*** Begin Patch\n*** Update File: {path}\n@@\n-a\n+b\n*** End Patch")
}

fn rule(name: &str, body: &str) -> Rule {
    Rule {
        path: PathBuf::from(format!("/rules/{name}.md")),
        stem: name.into(),
        globs: vec![],
        body: body.into(),
    }
}

#[test]
fn frontmatter_splits_from_the_body() {
    assert_eq!(
        split_frontmatter("---\npaths: [a]\n---\n# Body\n"),
        Some(("paths: [a]\n", "# Body\n"))
    );
    assert_eq!(
        split_frontmatter("\u{feff}---\r\nk: v\r\n---\r\nbody"),
        Some(("k: v\r\n", "body"))
    );
    assert_eq!(split_frontmatter("# No frontmatter\n---\n"), None);
    assert_eq!(split_frontmatter("---\nnever closed\n"), None);
}

#[test]
fn paths_read_from_block_lists_inline_lists_and_scalars() {
    assert_eq!(
        paths_globs(
            "paths:\n  - \"**/*.rs\"\n  - 'docs/**'\n  - bare/*.md\n\n  # note\nother: x\n"
        ),
        Some(vec!["**/*.rs".into(), "docs/**".into(), "bare/*.md".into()])
    );
    assert_eq!(
        paths_globs("paths: [\"**/*.{ts,tsx}\", 'a/*', b/**]\n"),
        Some(vec!["**/*.{ts,tsx}".into(), "a/*".into(), "b/**".into()])
    );
    assert_eq!(
        paths_globs("paths: [**/*.{rs,toml}]"),
        Some(vec!["**/*.{rs,toml}".into()])
    );
    assert_eq!(
        paths_globs("paths: \"src/**\""),
        Some(vec!["src/**".into()])
    );
    assert_eq!(paths_globs("description: no paths here\n"), None);
    assert_eq!(paths_globs("  paths: [nested]\n"), None);
}

#[test]
fn only_a_file_with_paths_frontmatter_is_a_scoped_rule() {
    let scoped = parse_rule(
        PathBuf::from("/r/testing.md"),
        "---\npaths:\n  - \"**/tests/**\"\n---\n\n# Testing\n\nBody.\n",
    )
    .expect("scoped");
    assert_eq!(scoped.stem, "testing");
    assert_eq!(scoped.body, "# Testing\n\nBody.");
    assert!(scoped.matches_any(&["crate/tests/a.rs".into()]));

    assert!(parse_rule(PathBuf::from("/r/plain.md"), "# Plain\n\nBody.").is_none());
    assert!(parse_rule(PathBuf::from("/r/meta.md"), "---\nname: x\n---\nBody.").is_none());
}

#[test]
fn globs_translate_to_anchored_regexes() {
    let cases: &[(&str, &str, bool)] = &[
        ("**/*.rs", "main.rs", true),
        ("**/*.rs", "a/b/main.rs", true),
        ("**/*.rs", "main.rsx", false),
        ("*.rs", "a/main.rs", false),
        ("Cargo.lock", "Cargo.lock", true),
        ("Cargo.lock", "sub/Cargo.lock", false),
        ("**/Cargo.lock", "sub/Cargo.lock", true),
        ("docs/**/*.html", "docs/a.html", true),
        ("docs/**/*.html", "docs/x/y/a.html", true),
        ("docs/**/*.html", "src/docs/a.html", false),
        (".husky/**", ".husky/pre-push", true),
        (".husky/**", ".husky", true),
        (".husky/**", "x/.husky/pre-push", false),
        ("**/tests/**", "tests/a.rs", true),
        ("**/tests/**", "crate/tests/deep/a.rs", true),
        ("**/tests/**", "crate/contests/a.rs", false),
        ("**/*test*.{rs,ts,tsx,js}", "src/a_tests.rs", true),
        ("**/*test*.{rs,ts,tsx,js}", "src/a_tests.py", false),
        (
            "packages/*/src/**/*.{svelte,ts}",
            "packages/ui/src/lib/a.svelte",
            true,
        ),
        (
            "packages/*/src/**/*.{svelte,ts}",
            "packages/ui/x/src/a.ts",
            false,
        ),
        ("Taskfile*.yml", "Taskfile.dev.yml", true),
        ("?.md", "a.md", true),
        ("?.md", "ab.md", false),
        ("?.md", "/.md", false),
        ("a.b", "axb", false),
        ("{a,b{c,d}}.md", "bd.md", true),
        ("{a,b{c,d}}.md", "b.md", false),
        ("./src/*.rs", "src/a.rs", true),
        ("a{b", "a{b", true),
    ];
    for (glob, path, expected) in cases {
        let re = glob_regex(glob).unwrap_or_else(|| panic!("{glob} did not compile"));
        assert_eq!(re.is_match(path), *expected, "{glob} vs {path}: {re}");
    }
    assert!(glob_regex("").is_none());
}

#[test]
fn a_patch_names_its_added_updated_and_moved_paths() {
    let text = "*** Begin Patch\n*** Add File: src/new.ts\n+x\n*** Update File: src/old.rs\n*** Move to: src/moved.rs\n@@\n-a\n+b\n*** Delete File: src/gone.rs\n*** End Patch";
    assert_eq!(
        patch_paths(text),
        vec!["src/new.ts", "src/old.rs", "src/moved.rs"]
    );
}

#[test]
fn a_command_names_its_path_like_words() {
    assert_eq!(
        command_paths("cargo test --manifest-path gates/rust/Cargo.toml -p codex-guard"),
        vec!["gates/rust/Cargo.toml"]
    );
    assert_eq!(
        command_paths("cat Cargo.toml src/main.rs"),
        vec!["Cargo.toml", "src/main.rs"]
    );
    assert_eq!(
        command_paths("cargo build --manifest-path=a/Cargo.toml"),
        vec!["a/Cargo.toml"]
    );
    assert!(command_paths("curl https://example.com/a/b").is_empty());
    assert!(command_paths("ls -la").is_empty());
    assert!(command_paths("cargo publish --dry-run 0.1.0").is_empty());
    let many = (0..100)
        .map(|i| format!("f{i}.rs"))
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(command_paths(&many).len(), MAX_CANDIDATES);
}

#[test]
fn keys_are_root_relative_inside_the_repo_and_as_written_outside() {
    let root = Path::new("/work/widgets");
    assert_eq!(
        match_key("src/a.rs", "/work/widgets/crate", Some(root)).as_deref(),
        Some("crate/src/a.rs")
    );
    assert_eq!(
        match_key("/work/widgets/Cargo.toml", "/elsewhere", Some(root)).as_deref(),
        Some("Cargo.toml")
    );
    assert_eq!(
        match_key("../other/a.rs", "/work/widgets", Some(root)).as_deref(),
        Some("../other/a.rs")
    );
    assert_eq!(
        match_key("src/a.rs", "/work/widgets", None).as_deref(),
        Some("src/a.rs")
    );
}

#[test]
fn the_git_root_is_the_nearest_ancestor_with_git() {
    let f = fixture("root");
    assert_eq!(git_root(&f.root.join("src")), Some(f.root.clone()));
    assert_eq!(git_root(Path::new("relative/dir")), None);
}

#[test]
fn rules_load_from_every_source_and_skip_unscoped_files() {
    let f = fixture("load");
    let rules = load_rules(&rule_dirs(Some(&f.home), Some(&f.root)));
    let stems: Vec<&str> = rules.iter().map(|r| r.stem.as_str()).collect();
    assert_eq!(stems, vec!["dependency-hygiene", "ui", "testing"]);
}

#[test]
fn a_matching_patch_delivers_the_rule_once_per_session() {
    let f = fixture("once");
    let first = call("apply_patch", &patch("src/a_tests.rs"), &f.root, "s1");
    let text = decide(&first, Some(&f.home), SystemTime::now()).expect("testing rule");
    assert!(text.starts_with(HEADER), "{text}");
    let testing =
        std::fs::canonicalize(f.root.join(".claude").join("rules").join("testing.md")).unwrap();
    assert!(
        text.contains(&format!("### testing ({})", testing.display())),
        "{text}"
    );
    assert!(text.contains("Run the scoped suite."), "{text}");
    assert!(!text.contains("description: tests"), "frontmatter stripped");
    assert!(!text.contains("Git discipline"), "unscoped rule never sent");

    assert!(decide(&first, Some(&f.home), SystemTime::now()).is_none());
    let other_session = call("apply_patch", &patch("src/a_tests.rs"), &f.root, "s2");
    assert!(decide(&other_session, Some(&f.home), SystemTime::now()).is_some());
}

#[test]
fn a_compaction_clears_the_record_so_rules_come_back() {
    let f = fixture("compact");
    let tests = call("apply_patch", &patch("src/a_tests.rs"), &f.root, "s1");
    assert!(decide(&tests, Some(&f.home), SystemTime::now()).is_some());
    assert!(decide(&tests, Some(&f.home), SystemTime::now()).is_none());

    let compact = json!({"session_id": "s1", "turn_id": "t2", "transcript_path": null,
        "cwd": f.root.to_str().unwrap(), "hook_event_name": "PostCompact", "model": "gpt-5",
        "trigger": "auto"});
    forget(&compact, Some(&f.home));
    assert!(decide(&tests, Some(&f.home), SystemTime::now()).is_some());
}

#[test]
fn a_rule_reached_through_two_directories_loads_once() {
    let f = fixture("dedup");
    let claude = f.root.join(".claude").join("rules").join("testing.md");
    symlink(
        &claude,
        f.root.join(".agents").join("rules").join("testing.md"),
    )
    .unwrap();
    std::fs::copy(&claude, f.home.join("agent-rules").join("testing.md")).unwrap();
    let rules = load_rules(&rule_dirs(Some(&f.home), Some(&f.root)));
    let testing = rules.iter().filter(|r| r.stem == "testing").count();
    assert_eq!(testing, 1);
}

#[test]
fn a_later_call_delivers_only_rules_not_yet_sent() {
    let f = fixture("later");
    let tests = call("apply_patch", &patch("src/a_tests.rs"), &f.root, "s1");
    assert!(decide(&tests, Some(&f.home), SystemTime::now()).is_some());

    let both = call(
        "Bash",
        "cat src/b_tests.rs src/view.ts Cargo.toml",
        &f.root.join("src").join(".."),
        "s1",
    );
    let text = decide(&both, Some(&f.home), SystemTime::now()).expect("two new rules");
    assert!(text.contains("### dependency-hygiene ("), "{text}");
    assert!(text.contains("### ui ("), "{text}");
    assert!(!text.contains("### testing ("), "{text}");
}

#[test]
fn nothing_matches_or_nothing_to_record_means_silence() {
    let f = fixture("silent");
    let readme = call("apply_patch", &patch("README.md"), &f.root, "s1");
    assert!(decide(&readme, Some(&f.home), SystemTime::now()).is_none());

    let no_paths = call("Bash", "git status", &f.root, "s1");
    assert!(decide(&no_paths, Some(&f.home), SystemTime::now()).is_none());

    let tests = call("apply_patch", &patch("src/a_tests.rs"), &f.root, "");
    assert!(decide(&tests, Some(&f.home), SystemTime::now()).is_none());
    let tests = call("apply_patch", &patch("src/a_tests.rs"), &f.root, "s1");
    assert!(decide(&tests, None, SystemTime::now()).is_none());

    let other = call("Read", "src/a_tests.rs", &f.root, "s1");
    assert!(decide(&other, Some(&f.home), SystemTime::now()).is_none());
}

#[test]
fn the_text_is_capped_and_overflow_becomes_read_pointers() {
    let big = "x".repeat(6_000);
    let rules: Vec<Rule> = ["a", "b", "c", "d"].map(|n| rule(n, &big)).into();
    let refs: Vec<&Rule> = rules.iter().collect();
    let text = compose(&refs);
    assert!(text.len() <= MAX_CONTEXT, "{} bytes", text.len());
    assert!(text.contains("### a (/rules/a.md)"));
    assert!(text.contains("### b (/rules/b.md)"));
    assert!(!text.contains("### c ("));
    assert!(text.contains("- read /rules/c.md"));
    assert!(text.contains("- read /rules/d.md"));
}

#[test]
fn a_rule_that_fits_exactly_is_inlined_without_a_pointer_reserve() {
    let header = format!("{HEADER}\n\n").len();
    let frame = "### a (/rules/a.md)\n\n\n\n".len();
    let body = "y".repeat(MAX_CONTEXT - header - frame);
    let only = rule("a", &body);
    let text = compose(&[&only]);
    assert!(text.contains(&body));
    assert!(!text.contains("- read"));
}

#[test]
fn the_output_is_additional_context_with_no_decision() {
    let out = hook::context_output("PreToolUse", "rule text");
    assert_eq!(
        out,
        json!({"hookSpecificOutput": {"hookEventName": "PreToolUse", "additionalContext": "rule text"}})
    );
    assert!(out["hookSpecificOutput"]
        .get("permissionDecision")
        .is_none());
}

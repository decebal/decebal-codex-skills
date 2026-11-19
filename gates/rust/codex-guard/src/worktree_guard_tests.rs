use super::{
    invalid_workspace, managed_target_is_longhand, primary_checkout, targets_longhand, worktree_add,
};
use std::fs;

#[test]
fn identifies_direct_add_but_not_inspection_or_removal() {
    assert!(worktree_add(
        "git worktree add --detach /tmp/example origin/main"
    ));
    assert!(worktree_add(
        "git -C /tmp/repo worktree add -b topic /tmp/example"
    ));
    assert!(!worktree_add("git worktree list"));
    assert!(!worktree_add("git worktree remove /tmp/example"));
    assert!(!worktree_add("echo git worktree add is unsafe"));
}

#[test]
fn managed_worktree_cannot_pass_without_location_and_seed() {
    let fixture = std::env::temp_dir().join(format!("guard-test-{}", std::process::id()));
    let primary = fixture.join("longhand");
    let external = fixture.join("other");
    let outside = fixture.join("unrelated");
    fs::create_dir_all(primary.join(".git/worktrees/test")).unwrap();
    fs::create_dir_all(primary.join("apps/desktop")).unwrap();
    fs::write(primary.join("agent-docs.toml"), "").unwrap();
    fs::write(primary.join("apps/desktop/tauri.conf.json"), "").unwrap();
    fs::create_dir_all(&external).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(
        external.join(".git"),
        format!("gitdir: {}/.git/worktrees/test\n", primary.display()),
    )
    .unwrap();
    let found = primary_checkout(&external).unwrap();
    assert_eq!(found, primary.canonicalize().unwrap());
    assert!(invalid_workspace(&external, &found));
    let command = format!(
        "git -C {} worktree add --detach /tmp/unsafe",
        primary.display()
    );
    assert!(targets_longhand(&command, &outside));
    assert!(targets_longhand(&format!("bash -c '{command}'"), &outside));
    assert!(targets_longhand(
        &format!(
            "bash -c 'cd {}\ngit worktree add --detach /tmp/unsafe'",
            primary.display()
        ),
        &outside
    ));
    let explicit_git_dir = format!(
        "git --git-dir={} worktree add --detach /tmp/unsafe",
        primary.join(".git").display()
    );
    assert!(targets_longhand(&explicit_git_dir, &outside));
    let env_git_dir = format!(
        "GIT_DIR={} git worktree add --detach /tmp/unsafe",
        primary.join(".git").display()
    );
    assert!(targets_longhand(&env_git_dir, &outside));
    let managed = serde_json::json!({"tool_input": {"sourceRoot": primary}});
    assert!(managed_target_is_longhand(&managed, &outside));
    let unrelated = serde_json::json!({"tool_input": {"sourceRoot": outside}});
    assert!(!managed_target_is_longhand(&unrelated, &outside));
    fs::remove_dir_all(fixture).unwrap();
}

#[test]
fn shell_wrapped_worktree_creation_is_detected() {
    assert!(worktree_add(
        "bash -c 'git worktree add --detach /tmp/unsafe origin/main'"
    ));
    assert!(worktree_add(
        "zsh -lc 'git worktree add --detach /tmp/unsafe origin/main'"
    ));
}

#[test]
fn seeded_colocated_worktree_passes_and_empty_artifacts_fail() {
    let fixture = std::env::current_dir()
        .unwrap()
        .join(format!("guard-fixture-{}", std::process::id()));
    let primary = fixture.join("longhand");
    let linked = primary.join(".worktrees/agent");
    fs::create_dir_all(primary.join(".git/worktrees/agent")).unwrap();
    fs::create_dir_all(primary.join("apps/desktop")).unwrap();
    fs::write(primary.join("agent-docs.toml"), "").unwrap();
    fs::write(primary.join("apps/desktop/tauri.conf.json"), "").unwrap();
    fs::create_dir_all(&linked).unwrap();
    fs::write(
        linked.join(".git"),
        format!("gitdir: {}/.git/worktrees/agent\n", primary.display()),
    )
    .unwrap();
    for rel in [
        "node_modules/.bin",
        "control-plane/admin/node_modules/.bin",
        "apps/app/dist",
        "apps/desktop/binaries",
        "apps/renderer/dist",
    ] {
        fs::create_dir_all(linked.join(rel)).unwrap();
    }
    for rel in [
        "node_modules/.bin/biome",
        "control-plane/admin/node_modules/.bin/biome",
        "apps/app/dist/index.html",
        "apps/desktop/binaries/seed",
        "apps/renderer/dist/seed",
    ] {
        fs::write(linked.join(rel), "seed").unwrap();
    }
    let actual = primary_checkout(&linked).unwrap();
    assert!(!invalid_workspace(&linked, &actual));
    fs::remove_file(linked.join("apps/app/dist/index.html")).unwrap();
    assert!(invalid_workspace(&linked, &actual));
    fs::remove_dir_all(fixture).unwrap();
}

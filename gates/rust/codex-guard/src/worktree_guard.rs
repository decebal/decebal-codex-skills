use serde_json::Value;
use std::path::{Path, PathBuf};

use crate::{hook, pr_guard};

fn primary_checkout(cwd: &Path) -> Option<PathBuf> {
    let root = cwd.ancestors().find(|dir| dir.join(".git").exists())?;
    let git = root.join(".git");
    let common = if git.is_dir() {
        git
    } else {
        let content = std::fs::read_to_string(git).ok()?;
        let path = content.trim().strip_prefix("gitdir: ")?;
        let path = hook::resolve_path(root.to_str()?, path);
        path.parent()?.parent()?.to_path_buf()
    };
    let primary = common.parent()?.canonicalize().ok()?;
    if primary.join("agent-docs.toml").is_file()
        && primary.join("apps/desktop/tauri.conf.json").is_file()
    {
        Some(primary)
    } else {
        None
    }
}

fn worktree_add(command: &str) -> bool {
    worktree_add_at_depth(command, 0)
}

fn worktree_add_at_depth(command: &str, depth: u8) -> bool {
    if depth > 4 {
        return false;
    }
    pr_guard::segments(command).iter().any(|words| {
        let (_, argv) = pr_guard::strip_wrappers(words);
        let executable = argv.first().map(String::as_str).unwrap_or("");
        let is_git = Path::new(executable)
            .file_name()
            .is_some_and(|name| name == "git");
        let is_shell = ["bash", "sh", "zsh"].iter().any(|shell| {
            Path::new(executable)
                .file_name()
                .is_some_and(|name| name == *shell)
        });
        (is_git && argv.windows(2).any(|pair| pair == ["worktree", "add"]))
            || (is_shell
                && argv.windows(2).any(|pair| {
                    shell_command_flag(&pair[0]) && worktree_add_at_depth(&pair[1], depth + 1)
                }))
    })
}

fn shell_command_flag(word: &str) -> bool {
    word.strip_prefix('-')
        .is_some_and(|flags| !flags.starts_with('-') && flags.contains('c'))
}

fn targets_longhand(command: &str, cwd: &Path) -> bool {
    targets_longhand_at_depth(command, cwd, 0)
}

fn targets_longhand_at_depth(command: &str, cwd: &Path, depth: u8) -> bool {
    if primary_checkout(cwd).is_some() {
        return true;
    }
    if depth > 4 {
        return false;
    }
    pr_guard::segments(command).iter().any(|words| {
        (words.first().is_some_and(|word| word == "cd")
            && words.get(1).is_some_and(|path| {
                primary_checkout(&hook::resolve_path(cwd.to_str().unwrap_or(""), path)).is_some()
            }))
            || words.windows(2).any(|pair| {
                pair[0] == "-C"
                    && primary_checkout(&hook::resolve_path(cwd.to_str().unwrap_or(""), &pair[1]))
                        .is_some()
                    || pair[0] == "--git-dir" && git_dir_is_longhand(cwd, &pair[1])
            })
            || words.iter().any(|word| {
                word.strip_prefix("-C")
                    .filter(|path| !path.is_empty())
                    .is_some_and(|path| {
                        primary_checkout(&hook::resolve_path(cwd.to_str().unwrap_or(""), path))
                            .is_some()
                    })
                    || word
                        .strip_prefix("--git-dir=")
                        .or_else(|| word.strip_prefix("GIT_DIR="))
                        .is_some_and(|path| git_dir_is_longhand(cwd, path))
            })
            || words.iter().any(|word| {
                ["bash", "sh", "zsh"].iter().any(|shell| {
                    Path::new(word)
                        .file_name()
                        .is_some_and(|name| name == *shell)
                })
            }) && words.windows(2).any(|pair| {
                shell_command_flag(&pair[0]) && targets_longhand_at_depth(&pair[1], cwd, depth + 1)
            })
    })
}

fn git_dir_is_longhand(cwd: &Path, path: &str) -> bool {
    let gitdir = hook::resolve_path(cwd.to_str().unwrap_or(""), path);
    gitdir.parent().and_then(primary_checkout).is_some()
}

fn invalid_workspace(cwd: &Path, primary: &Path) -> bool {
    let Some(root) = cwd.ancestors().find(|dir| dir.join(".git").exists()) else {
        return true;
    };
    let Ok(root) = root.canonicalize() else {
        return true;
    };
    if root == primary {
        return false;
    }
    let Some(parent) = primary.parent() else {
        return true;
    };
    let inside = root.starts_with(primary) || root.parent().and_then(Path::parent) == Some(parent);
    let temporary = [
        "/tmp",
        "/private/tmp",
        "/var/folders",
        "/private/var/folders",
    ]
    .iter()
    .any(|path| root.starts_with(path))
        || std::env::var_os("TMPDIR")
            .and_then(|path| PathBuf::from(path).canonicalize().ok())
            .is_some_and(|path| root.starts_with(path));
    let nonempty = |path: PathBuf| {
        std::fs::read_dir(path)
            .ok()
            .and_then(|mut entries| entries.next())
            .is_some()
    };
    let ready = root.join("node_modules/.bin/biome").exists()
        && root
            .join("control-plane/admin/node_modules/.bin/biome")
            .exists()
        && root.join("apps/app/dist/index.html").is_file()
        && nonempty(root.join("apps/desktop/binaries"))
        && nonempty(root.join("apps/renderer/dist"))
        && (!primary.join(".env").is_file() || root.join(".env").is_file());
    !inside || temporary || !ready
}

fn managed_target_is_longhand(payload: &Value, cwd: &Path) -> bool {
    primary_checkout(cwd).is_some()
        || primary_checkout(Path::new(hook::str_at(
            payload,
            &["tool_input", "sourceRoot"],
        )))
        .is_some()
}

pub fn run(payload: &Value) -> ! {
    let cwd = Path::new(hook::str_at(payload, &["cwd"]));
    let event = hook::str_at(payload, &["hook_event_name"]);
    let reason = "Longhand requires a seeded project-local worktree. Use tooling/scripts/agent-worktree.sh create from the primary checkout.";
    if event == "SessionStart"
        && primary_checkout(cwd).is_some_and(|primary| invalid_workspace(cwd, &primary))
    {
        println!(
            "{}",
            serde_json::json!({"continue": false, "stopReason": reason})
        );
        std::process::exit(0);
    }
    if event == "PreToolUse" {
        let tool = hook::str_at(payload, &["tool_name"]);
        let command = hook::str_at(payload, &["tool_input", "command"]);
        let workdir = hook::str_at(payload, &["tool_input", "workdir"]);
        let effective_cwd = if workdir.is_empty() {
            cwd.to_path_buf()
        } else {
            hook::resolve_path(cwd.to_str().unwrap_or(""), workdir)
        };
        if (tool == "Bash" && worktree_add(command) && targets_longhand(command, &effective_cwd))
            || (tool.ends_with("__create_worktree") && managed_target_is_longhand(payload, cwd))
        {
            hook::deny(reason);
        }
    }
    hook::allow()
}

#[cfg(test)]
#[path = "worktree_guard_tests.rs"]
mod tests;

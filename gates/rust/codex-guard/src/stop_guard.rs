use serde_json::{json, Value};
use std::process::Command;
use std::process::ExitCode;
use std::time::{Duration, Instant};

const CONTINUE: &str = "Check for unfinished delivery. Continue work within the user's authorization: resolve fixable failures, preserve evidence, and get the change into its appropriate PR. Do not bypass hooks, invent evidence, or commit or rewrite another session's work. If a credential, user decision, permission, or external action is genuinely needed, finish independent work and explain the exact blocker. Respect explicit requests to stop and do not commit or create branches without required authorization.";

pub fn run(payload: &Value) -> ExitCode {
    let mut result = decision(payload);
    if result == json!({})
        && payload.get("stop_hook_active").and_then(Value::as_bool) == Some(false)
        && branch_has_unrepresented_changes(payload)
    {
        result = json!({"decision": "block", "reason": CONTINUE});
    }
    println!("{result}");
    ExitCode::SUCCESS
}

fn branch_has_unrepresented_changes(payload: &Value) -> bool {
    let cwd = crate::hook::str_at(payload, &["cwd"]);
    if cwd.is_empty() {
        return false;
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    let run = |program: &str, args: &[&str]| {
        crate::pr_guard::run_bounded(Command::new(program).current_dir(cwd).args(args), deadline)
    };
    let Some(branch) = run("git", &["branch", "--show-current"]) else {
        return false;
    };
    let branch = branch.trim();
    if branch.is_empty() || matches!(branch, "main" | "master") {
        return false;
    }
    let Some(prs) = run(
        "gh",
        &[
            "pr", "list", "--head", branch, "--state", "all", "--json", "state",
        ],
    ) else {
        return false;
    };
    let Ok(prs) = serde_json::from_str::<Vec<Value>>(&prs) else {
        return false;
    };
    if prs
        .iter()
        .any(|pr| pr.get("state").and_then(Value::as_str) == Some("MERGED"))
    {
        return false;
    }
    let dirty = run("git", &["status", "--porcelain"]).is_some_and(|s| !s.trim().is_empty());
    let differs = run("git", &["diff", "--name-only", "origin/main...HEAD"])
        .is_some_and(|s| !s.trim().is_empty());
    let unpushed = run("git", &["rev-list", "--count", "@{upstream}..HEAD"])
        .and_then(|s| s.trim().parse::<u64>().ok())
        .is_some_and(|count| count > 0);
    needs_attention(&prs, dirty, differs, unpushed)
}

fn needs_attention(prs: &[Value], dirty: bool, differs: bool, unpushed: bool) -> bool {
    if prs
        .iter()
        .any(|pr| pr.get("state").and_then(Value::as_str) == Some("MERGED"))
    {
        return false;
    }
    if prs
        .iter()
        .any(|pr| pr.get("state").and_then(Value::as_str) == Some("OPEN"))
    {
        return dirty || unpushed;
    }
    dirty || differs
}

pub(crate) fn decision(payload: &Value) -> Value {
    if payload.get("stop_hook_active").and_then(Value::as_bool) != Some(false) {
        return json!({});
    }
    let Some(message) = payload
        .get("last_assistant_message")
        .and_then(Value::as_str)
    else {
        return json!({});
    };
    let text = message.to_lowercase();
    let incomplete = [
        "incomplete",
        "unfinished",
        "not finished",
        "not complete",
        "not completed",
        "still need",
        "still needs",
        "remaining",
        "not yet",
        "no pr",
        "no pull request",
        "no draft pr",
        "no readyplaydays pr",
        "not opened",
        "not pushed",
        "not been pushed",
        "hasn't been pushed",
        "has not been pushed",
        "failed",
        "failing",
    ]
    .iter()
    .any(|phrase| text.contains(phrase));
    let delivery = [
        "review",
        "evidence",
        "gate",
        "hook",
        " pr",
        "pr ",
        "pull request",
        "push",
        "pushed",
        "verification",
        "verify",
        "test",
    ]
    .iter()
    .any(|phrase| text.contains(phrase));
    if incomplete && delivery {
        json!({"decision": "block", "reason": CONTINUE})
    } else {
        json!({})
    }
}

#[cfg(test)]
#[path = "stop_guard_tests.rs"]
mod tests;

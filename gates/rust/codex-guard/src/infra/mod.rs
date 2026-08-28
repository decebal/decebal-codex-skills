//! PreToolUse — deny by BLAST RADIUS, not by how simple the command
//! looks.
//!
//! Two layers. The literal one always runs: classify the typed command, then the
//! prod-token and breadth rules. The deep one follows wrapper chains, and runs
//! LAST so a bug in it can only fail open — never weaken the literal floor.
//!
//! Decision posture is "silent unless a risk signal": a clean pattern hit denies
//! risky operations. Codex PreToolUse hooks cannot request an approval prompt,
//! so `Ask` classifications become denials with a confirmation-oriented reason.
//! Everything else is silently allowed.

pub mod classify;
pub mod resolve;

use crate::hook;
use classify::{classify, Verdict};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// What the guard decided. Separate from emitting it, so the whole decision
/// matrix is testable in-process rather than by spawning the binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny(String),
    Ask(String),
}

/// Local git subcommands never touch infra, and their `-m`/`-F` message text is
/// DATA — a commit message reading "block gcloud run deploy" must not trip the
/// guard.
///
/// Matched only when no separator survives quote-stripping: a bare prefix match
/// would wave through everything after `git commit -m x ;`. `git push` stays out
/// of the list, because a force-push to a protected branch is caught above.
fn is_safe_git(scan: &str) -> bool {
    const SEPARATORS: [&str; 8] = ["&&", "||", ";", "|", "$(", "`", "&", "\n"];
    if SEPARATORS.iter().any(|s| scan.contains(s)) {
        return false;
    }
    const SAFE: [&str; 16] = [
        "git commit",
        "git add",
        "git status",
        "git diff",
        "git log",
        "git show",
        "git stash",
        "git fetch",
        "git branch",
        "git checkout",
        "git switch",
        "git restore",
        "git tag",
        "git cherry-pick",
        "git revert",
        "git blame",
    ];
    SAFE.iter().any(|p| scan.starts_with(p))
}

/// The skeleton a pattern is matched against.
///
/// Quoted data is stripped, so a dangerous phrase inside a string is not read as
/// an executed command. Shell `-c` and `eval` forms keep their quoted body —
/// there it IS executed.
fn skeleton(lc: &str) -> String {
    if lc.contains("bash -c ")
        || lc.contains("sh -c ")
        || lc.contains("zsh -c ")
        || lc.contains("eval ")
    {
        return lc.to_string();
    }
    let no_double = classify::cached("\"[^\"]*\"").replace_all(lc, "");
    classify::cached("'[^']*'")
        .replace_all(&no_double, "")
        .into_owned()
}

struct ProdTokens {
    prod: Option<String>,
    nonprod: Option<String>,
}

/// A repo shipping its own token file wins; the personal copy covers every
/// project that does not.
fn load_prod_tokens(cwd: &str) -> ProdTokens {
    let mut bases: Vec<PathBuf> = Vec::new();
    if let Some(root) = project_root(cwd) {
        bases.push(root);
    }
    if !cwd.is_empty() {
        bases.push(PathBuf::from(cwd));
    }
    if let Ok(h) = std::env::var("HOME") {
        bases.push(PathBuf::from(h));
    }

    for base in bases {
        let file = base.join(".codex/prod-guard-tokens.txt");
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let mut prod = Vec::new();
        let mut nonprod = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(t) = line.strip_prefix("prod:") {
                if !t.is_empty() {
                    prod.push(regex::escape(t));
                }
            } else if let Some(t) = line.strip_prefix("nonprod:") {
                if !t.is_empty() {
                    nonprod.push(regex::escape(t));
                }
            }
        }
        return ProdTokens {
            prod: (!prod.is_empty()).then(|| prod.join("|")),
            nonprod: (!nonprod.is_empty()).then(|| nonprod.join("|")),
        };
    }
    ProdTokens {
        prod: None,
        nonprod: None,
    }
}

fn project_root(cwd: &str) -> Option<PathBuf> {
    let cwd = Path::new(cwd);
    cwd.ancestors()
        .find(|candidate| candidate.join(".git").exists())
        .map(Path::to_path_buf)
}

fn matches_group(scan: &str, group: &Option<String>) -> bool {
    match group {
        Some(g) => classify::cached(&format!("({g})")).is_match(scan),
        None => false,
    }
}

pub fn run(payload: &Value) -> ! {
    if std::env::var("INFRA_GUARD_OFF").as_deref() == Ok("1") {
        hook::allow();
    }
    match decide(payload) {
        Decision::Allow => hook::allow(),
        Decision::Ask(r) => hook::deny(&format!(
            "Confirmation required: {r}. Codex PreToolUse hooks cannot prompt; review scope, then use an execution-policy prompt or temporarily disable this hook."
        )),
        Decision::Deny(r) => hook::deny(&r),
    }
}

pub fn decide(payload: &Value) -> Decision {
    let cmd = hook::str_at(payload, &["tool_input", "command"]);
    if cmd.is_empty() {
        return Decision::Allow;
    }
    let cwd = hook::str_at(payload, &["cwd"]);
    let lc = cmd.to_lowercase();
    let scan = skeleton(&lc);

    if is_safe_git(&scan) {
        return Decision::Allow;
    }

    let literal = classify(&scan);
    if let Some(Verdict::Deny(reason)) = &literal {
        return Decision::Deny(format!("{reason} (infra-guard)"));
    }

    let tokens = load_prod_tokens(cwd);
    let prod_hit = matches_group(&scan, &tokens.prod);
    let nonprod_hit = matches_group(&scan, &tokens.nonprod);
    let mutating = classify::is_mutating(&scan);

    if mutating && prod_hit && nonprod_hit {
        return Decision::Deny("Command targets PROD and non-prod together — never batch prod with anything; split per-env. (infra-guard)".into());
    }

    let mut ask_reason: Option<String> = match &literal {
        Some(Verdict::Ask(r)) => Some(r.clone()),
        _ => None,
    };
    if mutating && ask_reason.is_none() {
        if prod_hit {
            ask_reason = Some(
                "mutating command touches PROD — state blast radius + rollback, then confirm"
                    .into(),
            );
        } else if classify::is_broad(&scan) {
            ask_reason = Some("mutation with --all / -A — wide blast radius; confirm scope".into());
        }
    }
    if let Some(r) = ask_reason {
        return Decision::Ask(format!("{r} (infra-guard)"));
    }

    deep_scan(cmd, cwd, tokens.prod.as_deref())
}

/// The deep layer. Cheap pre-filter first: a command naming no wrapper form
/// cannot reach one.
fn deep_scan(cmd: &str, cwd: &str, prod_re: Option<&str>) -> Decision {
    let limits = resolve::Limits::from_env();
    if limits.max_depth == 0 {
        return Decision::Allow;
    }
    let looks_wrapped = [
        "bash ", "sh ", "make ", "source ", ". ", "bun ", "npm ", "pnpm ", ".sh",
    ]
    .iter()
    .any(|m| cmd.contains(m));
    if !looks_wrapped {
        return Decision::Allow;
    }

    let root = project_root(cwd).unwrap_or_else(|| PathBuf::from(cwd));
    let scan = resolve::scan_tree(cmd, &root, Path::new(cwd), &limits, prod_re);
    let hash = resolve::tree_hash(&scan.units).unwrap_or_default();
    if resolve::is_kosher(&hash) {
        return Decision::Allow;
    }

    match scan.verdict {
        Some(Verdict::Deny(_)) => Decision::Deny(format!(
            "Wrapper chain runs a blocked command — {}. Review the chain; propose it instead. (infra-guard deep)",
            scan.reason
        )),
        Some(Verdict::Ask(_)) => Decision::Ask(format!(
            "Wrapper chain runs — {}. Confirm. (infra-guard deep)",
            scan.reason
        )),
        None if scan.risk => Decision::Ask(format!(
            "Wrapper chain references an infra tool / prod token with no clean match (dynamic command?). Review it; trust with `codex-guard trust '{cmd}'` if safe. (infra-guard deep)"
        )),
        None => Decision::Allow,
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;

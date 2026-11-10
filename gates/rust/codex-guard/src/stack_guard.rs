//! PreToolUse, Bash — stacked pull requests land bottom-up, one commit per
//! author per layer, and never carry a parent's stale commits.
//!
//! A PR is stacked when its base is another PR's head branch rather than the
//! repo's default branch. The guard reads the live PR graph with `gh` and the
//! local branch with `git`, and denies:
//!
//! - `gh pr create --base <non-default>` unless it is `--draft`, the base has an
//!   open PR, and the new head contains that PR's head commit;
//! - `gh pr ready` on a PR whose base is not the default branch;
//! - `gh pr merge` on a PR whose base is not the default branch, and, when
//!   `git config stack-guard.gate-context` names a commit status, on a head
//!   without that status at `success` or without `--match-head-commit <sha>`;
//! - `git push` of a branch that holds commits already on the default branch
//!   under other SHAs, still contains a merged parent's head, or no longer
//!   contains its open parent's head;
//! - `git commit` (not `--amend`) that would give its author a second commit on
//!   a stacked PR.
//!
//! Fails open: no GitHub remote, no `gh`, a missing object, or a lookup past the
//! deadline all allow. A guard that blocked every push when `gh` was slow would
//! be routed around, and then it guards nothing.

use crate::hook;
use crate::pr_guard::{
    github_repo, normalize_repo, run_bounded, segments, strip_heredoc_bodies, strip_wrappers,
};
use serde_json::Value;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// One budget for every child the guard runs, shared across all lookups.
const DEADLINE: Duration = Duration::from_secs(8);

/// `gh` flags that take a value, so the value is not read as a positional.
const GH_VALUE_FLAGS: &[&str] = &[
    "-R",
    "--repo",
    "-B",
    "--base",
    "-H",
    "--head",
    "-t",
    "--title",
    "-b",
    "--body",
    "-F",
    "--body-file",
    "-a",
    "--assignee",
    "-l",
    "--label",
    "-m",
    "--milestone",
    "-p",
    "--project",
    "-r",
    "--reviewer",
    "-T",
    "--template",
    "-A",
    "--author-email",
    "--match-head-commit",
    "--subject",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Selector {
    Number(u64),
    Branch(String),
    Current,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Trigger {
    CreatePr {
        repo: Option<String>,
        base: Option<String>,
        head: Option<String>,
        draft: bool,
        dry_run: bool,
    },
    Ready {
        repo: Option<String>,
        pr: Selector,
    },
    Merge {
        repo: Option<String>,
        pr: Selector,
        match_head: Option<String>,
    },
    /// `src` is the local ref pushed, `branch` the remote branch it lands on.
    Push {
        dir: Option<String>,
        src: String,
        branch: Option<String>,
    },
    Commit {
        dir: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pr {
    pub number: u64,
    pub base: String,
    pub head: String,
    /// `OPEN`, `MERGED` or `CLOSED`, as `gh` reports it.
    pub state: String,
    pub head_oid: String,
}

impl Pr {
    fn is_open(&self) -> bool {
        self.state == "OPEN"
    }
    fn short(&self) -> &str {
        self.head_oid.get(..9).unwrap_or(&self.head_oid)
    }
}

/// The facts the guard reads from outside the process.
pub(crate) trait Lookup {
    /// `owner/repo` of the `origin` remote of the repo at `dir`, lowercased.
    fn origin_repo(&self, dir: &str, deadline: Instant) -> Option<String>;
    /// The branch checked out at `dir`; `None` when detached or unreadable.
    fn current_branch(&self, dir: &str, deadline: Instant) -> Option<String>;
    /// The default branch of `repo`, read through the repo at `dir`.
    fn default_branch(&self, dir: &str, repo: &str, deadline: Instant) -> Option<String>;
    /// The open PR with this head, else the newest one.
    fn pr_by_head(&self, repo: &str, head: &str, deadline: Instant) -> Option<Pr>;
    fn pr_by_number(&self, repo: &str, number: u64, deadline: Instant) -> Option<Pr>;
    /// `None` when either commit is not in the local object store.
    fn is_ancestor(
        &self,
        dir: &str,
        ancestor: &str,
        descendant: &str,
        deadline: Instant,
    ) -> Option<bool>;
    /// How many commits on `tip` that are not on `upstream` already have an
    /// equivalent patch there (`git cherry`'s `-` lines).
    fn already_upstream(
        &self,
        dir: &str,
        upstream: &str,
        tip: &str,
        deadline: Instant,
    ) -> Option<usize>;
    /// Commits in `since..HEAD` by the configured `user.email`.
    fn own_commits_since(&self, dir: &str, since: &str, deadline: Instant) -> Option<u64>;
    /// `git config stack-guard.gate-context` in the repo at `dir`.
    fn gate_context(&self, dir: &str, deadline: Instant) -> Option<String>;
    /// The newest state of `context` on `sha`: `Some(None)` when the commit has
    /// no such status, `None` when the lookup failed.
    fn status(
        &self,
        repo: &str,
        sha: &str,
        context: &str,
        deadline: Instant,
    ) -> Option<Option<String>>;
}

pub fn run(payload: &Value) -> ! {
    match decide(payload, &Live, Instant::now() + DEADLINE) {
        Some(reason) => hook::deny(&reason),
        None => hook::allow(),
    }
}

/// The deny reason, or `None` to allow.
pub(crate) fn decide(payload: &Value, lookup: &dyn Lookup, deadline: Instant) -> Option<String> {
    if hook::str_at(payload, &["tool_name"]) != "Bash" {
        return None;
    }
    let cwd = hook::str_at(payload, &["cwd"]);
    classify_all(hook::str_at(payload, &["tool_input", "command"]))
        .iter()
        .find_map(|trigger| check(trigger, cwd, lookup, deadline))
}

fn check(trigger: &Trigger, cwd: &str, lookup: &dyn Lookup, deadline: Instant) -> Option<String> {
    match trigger {
        Trigger::CreatePr {
            repo,
            base,
            head,
            draft,
            dry_run,
        } => {
            if *dry_run {
                return None;
            }
            let base = base.as_deref()?;
            let (repo, local) = resolve_repo(repo.as_deref(), cwd, lookup, deadline)?;
            let default = lookup.default_branch(cwd, &repo, deadline)?;
            if base == default {
                return None;
            }
            let Some(parent) = lookup.pr_by_head(&repo, base, deadline).filter(Pr::is_open) else {
                return Some(format!(
                    "Base {base} has no open PR in {repo}, so this PR would not be a stack layer. Target {default}, or open the parent PR first."
                ));
            };
            if !draft {
                return Some(format!(
                    "A stacked PR (base {base}, #{}) opens as a draft: add --draft. It is marked ready only after #{} lands and this branch is restacked onto {default}.",
                    parent.number, parent.number
                ));
            }
            let tip = head.as_deref().unwrap_or("HEAD");
            if local && lookup.is_ancestor(cwd, &parent.head_oid, tip, deadline) == Some(false) {
                return Some(format!(
                    "{tip} does not contain #{}'s head {}: the parent moved since this branch was cut. Restack first: `git fetch origin {base}` then `git rebase --onto origin/{base} <old-parent-tip> {tip}`.",
                    parent.number,
                    parent.short()
                ));
            }
            None
        }
        Trigger::Ready { repo, pr } => {
            let (repo, _) = resolve_repo(repo.as_deref(), cwd, lookup, deadline)?;
            let pr = resolve_pr(pr, &repo, cwd, lookup, deadline)?;
            let default = lookup.default_branch(cwd, &repo, deadline)?;
            (pr.base != default).then(|| {
                format!(
                    "#{} is based on {}, not {default}. A stacked PR stays draft until its parent lands and it is restacked onto {default} (`gh pr edit {} --base {default}`).",
                    pr.number, pr.base, pr.number
                )
            })
        }
        Trigger::Merge {
            repo,
            pr,
            match_head,
        } => {
            let (repo, local) = resolve_repo(repo.as_deref(), cwd, lookup, deadline)?;
            let pr = resolve_pr(pr, &repo, cwd, lookup, deadline)?;
            let default = lookup.default_branch(cwd, &repo, deadline)?;
            if pr.base != default {
                let parent = lookup
                    .pr_by_head(&repo, &pr.base, deadline)
                    .map(|p| format!(" (#{})", p.number))
                    .unwrap_or_default();
                return Some(format!(
                    "#{} is stacked on {}{parent}. Stacks land bottom-up: merge the parent, restack #{} onto {default}, gate it, then merge it.",
                    pr.number, pr.base, pr.number
                ));
            }
            if !local {
                return None;
            }
            let context = lookup.gate_context(cwd, deadline)?;
            if match_head.as_deref() != Some(pr.head_oid.as_str()) {
                return Some(format!(
                    "Merge exactly the head that was gated: add `--match-head-commit {}` (#{}'s head).",
                    pr.head_oid, pr.number
                ));
            }
            match lookup.status(&repo, &pr.head_oid, &context, deadline)? {
                Some(state) if state == "success" => None,
                state => Some(format!(
                    "#{} head {} has {context} = {}. Run the gate on that exact head and merge once it is success.",
                    pr.number,
                    pr.short(),
                    state.as_deref().unwrap_or("absent")
                )),
            }
        }
        Trigger::Push { dir, src, branch } => {
            let dir = dir.as_deref().unwrap_or(cwd);
            let repo = lookup.origin_repo(dir, deadline)?;
            let branch = match branch {
                Some(branch) => branch.clone(),
                None => lookup.current_branch(dir, deadline)?,
            };
            let default = lookup.default_branch(dir, &repo, deadline)?;
            if branch == default {
                return None;
            }
            let upstream = format!("origin/{default}");
            if let Some(stale) = lookup
                .already_upstream(dir, &upstream, src, deadline)
                .filter(|n| *n > 0)
            {
                return Some(format!(
                    "{stale} commit(s) on {branch} are already on {default} under other SHAs (a parent PR landed). Restack first: `git fetch origin {default}` then `git rebase {upstream}`, which drops them."
                ));
            }
            let pr = lookup
                .pr_by_head(&repo, &branch, deadline)
                .filter(Pr::is_open)?;
            if pr.base == default {
                return None;
            }
            let parent = lookup.pr_by_head(&repo, &pr.base, deadline)?;
            let contains = lookup.is_ancestor(dir, &parent.head_oid, src, deadline)?;
            match (parent.state.as_str(), contains) {
                ("MERGED", true) => Some(format!(
                    "Parent #{} merged into {default} under new SHAs, and {branch} still carries its old head {}. Restack first: `git fetch origin {default}`, `git rebase --onto {upstream} {} {src}`, push, then `gh pr edit {} --base {default}`.",
                    parent.number,
                    parent.short(),
                    parent.head_oid,
                    pr.number
                )),
                ("OPEN", false) => Some(format!(
                    "{branch} (#{}) no longer contains its parent #{}'s head {}: the parent moved. Restack first: `git fetch origin {}` then `git rebase --onto origin/{} <old-parent-tip> {src}`.",
                    pr.number,
                    parent.number,
                    parent.short(),
                    pr.base,
                    pr.base
                )),
                _ => None,
            }
        }
        Trigger::Commit { dir } => {
            let dir = dir.as_deref().unwrap_or(cwd);
            let repo = lookup.origin_repo(dir, deadline)?;
            let branch = lookup.current_branch(dir, deadline)?;
            let default = lookup.default_branch(dir, &repo, deadline)?;
            if branch == default
                || lookup.own_commits_since(dir, &format!("origin/{default}"), deadline)? == 0
            {
                return None;
            }
            let pr = lookup
                .pr_by_head(&repo, &branch, deadline)
                .filter(Pr::is_open)?;
            if pr.base == default {
                return None;
            }
            let parent = lookup.pr_by_head(&repo, &pr.base, deadline)?;
            let own = lookup.own_commits_since(dir, &parent.head_oid, deadline)?;
            (own > 0).then(|| {
                format!(
                    "You already have a commit on stacked #{} ({branch}); a layer holds one commit per author. Fold this change in with `git commit --amend`.",
                    pr.number
                )
            })
        }
    }
}

/// The repo a command acts on, and whether it is the repo checked out at `cwd`
/// (only then do local `git` facts describe it).
fn resolve_repo(
    named: Option<&str>,
    cwd: &str,
    lookup: &dyn Lookup,
    deadline: Instant,
) -> Option<(String, bool)> {
    let origin = lookup.origin_repo(cwd, deadline);
    match named {
        Some(named) => {
            let named = normalize_repo(named)?;
            let local = origin.as_deref() == Some(named.as_str());
            Some((named, local))
        }
        None => origin.map(|repo| (repo, true)),
    }
}

fn resolve_pr(
    selector: &Selector,
    repo: &str,
    cwd: &str,
    lookup: &dyn Lookup,
    deadline: Instant,
) -> Option<Pr> {
    match selector {
        Selector::Number(number) => lookup.pr_by_number(repo, *number, deadline),
        Selector::Branch(branch) => lookup.pr_by_head(repo, branch, deadline),
        Selector::Current => {
            let branch = lookup.current_branch(cwd, deadline)?;
            lookup.pr_by_head(repo, &branch, deadline)
        }
    }
}

/// Every trigger in `command`. Heredoc bodies are data, so they are removed
/// before the command is split.
pub(crate) fn classify_all(command: &str) -> Vec<Trigger> {
    segments(&strip_heredoc_bodies(command))
        .iter()
        .filter_map(|segment| {
            let (env, argv) = strip_wrappers(segment);
            match argv.first().map(String::as_str) {
                Some("gh") => classify_gh(&argv[1..], &env),
                Some("git") => classify_git(&argv[1..]),
                _ => None,
            }
        })
        .collect()
}

fn classify_gh(args: &[String], env: &[(String, String)]) -> Option<Trigger> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        return None;
    }
    let mut repo = env
        .iter()
        .find(|(name, _)| name == "GH_REPO")
        .map(|(_, value)| value.clone());
    let mut flags: Vec<(String, Option<String>)> = Vec::new();
    let mut words: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        if let Some((flag, value)) = arg.split_once('=').filter(|_| arg.starts_with("--")) {
            flags.push((flag.to_string(), Some(value.to_string())));
        } else if GH_VALUE_FLAGS.contains(&arg) {
            flags.push((arg.to_string(), args.get(i + 1).cloned()));
            i += 1;
        } else if arg.starts_with('-') && arg.len() > 2 && !arg.starts_with("--") {
            let (flag, value) = arg.split_at(2);
            if GH_VALUE_FLAGS.contains(&flag) {
                flags.push((flag.to_string(), Some(value.to_string())));
            } else {
                flags.push((arg.to_string(), None));
            }
        } else if arg.starts_with('-') {
            flags.push((arg.to_string(), None));
        } else {
            words.push(arg.to_string());
        }
        i += 1;
    }
    let value = |names: &[&str]| {
        flags
            .iter()
            .rev()
            .find(|(flag, _)| names.contains(&flag.as_str()))
            .and_then(|(_, value)| value.clone())
    };
    let has = |names: &[&str]| flags.iter().any(|(flag, _)| names.contains(&flag.as_str()));
    if let Some(named) = value(&["-R", "--repo"]) {
        repo = Some(named);
    }
    if words.first().map(String::as_str) != Some("pr") {
        return None;
    }
    let selector = || match words.get(2) {
        None => Selector::Current,
        Some(word) => word
            .parse()
            .ok()
            .or_else(|| pull_number(word))
            .map_or_else(|| Selector::Branch(word.clone()), Selector::Number),
    };
    match words.get(1).map(String::as_str) {
        Some("create" | "new") => Some(Trigger::CreatePr {
            repo,
            base: value(&["-B", "--base"]),
            head: value(&["-H", "--head"]),
            draft: has(&["-d", "--draft"]),
            dry_run: has(&["--dry-run", "-w", "--web"]),
        }),
        Some("ready") if !has(&["--undo"]) => Some(Trigger::Ready {
            repo,
            pr: selector(),
        }),
        Some("merge") if !has(&["--disable-auto"]) => Some(Trigger::Merge {
            repo,
            pr: selector(),
            match_head: value(&["--match-head-commit"]),
        }),
        _ => None,
    }
}

/// `n` from a `…/pull/<n>` URL.
fn pull_number(word: &str) -> Option<u64> {
    word.trim_end_matches('/')
        .rsplit_once("/pull/")
        .and_then(|(_, n)| n.parse().ok())
}

fn classify_git(args: &[String]) -> Option<Trigger> {
    let mut dir = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-C" => {
                dir = args.get(i + 1).cloned();
                i += 2;
            }
            "-c" => i += 2,
            arg if arg.starts_with('-') => i += 1,
            _ => break,
        }
    }
    let rest = &args[(i + 1).min(args.len())..];
    match args.get(i)?.as_str() {
        "commit" => {
            let amend = rest
                .iter()
                .any(|a| a == "--amend" || a == "--fixup" || a.starts_with("--fixup="));
            let dry = rest.iter().any(|a| a == "--dry-run" || a == "-h");
            (!amend && !dry).then_some(Trigger::Commit { dir })
        }
        "push" => classify_push(rest, dir),
        _ => None,
    }
}

fn classify_push(args: &[String], dir: Option<String>) -> Option<Trigger> {
    const VALUE_FLAGS: &[&str] = &["-o", "--push-option", "--repo", "--receive-pack", "--exec"];
    const SKIP: &[&str] = &[
        "-d",
        "--delete",
        "--tags",
        "--all",
        "--mirror",
        "--dry-run",
        "-n",
        "-h",
        "--prune",
    ];
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        if SKIP.contains(&arg) {
            return None;
        }
        if VALUE_FLAGS.contains(&arg) {
            i += 2;
            continue;
        }
        if !arg.starts_with('-') {
            positional.push(arg);
        }
        i += 1;
    }
    let refspecs = positional.get(1..).unwrap_or_default();
    match refspecs {
        [] => Some(Trigger::Push {
            dir,
            src: "HEAD".into(),
            branch: None,
        }),
        [refspec] => {
            let refspec = refspec.trim_start_matches('+');
            let (src, dst) = refspec.split_once(':').unwrap_or((refspec, refspec));
            if src.is_empty() || dst.is_empty() {
                return None;
            }
            let dst = dst.strip_prefix("refs/heads/").unwrap_or(dst);
            let branch = (dst != "HEAD").then(|| dst.to_string());
            Some(Trigger::Push {
                dir,
                src: src.to_string(),
                branch,
            })
        }
        _ => None,
    }
}

/// The real lookups: `git` and `gh`, each run as a child this process owns.
struct Live;

fn here(dir: &str) -> &str {
    if dir.is_empty() {
        "."
    } else {
        dir
    }
}

fn git(dir: &str, args: &[&str], deadline: Instant) -> Option<String> {
    run_bounded(
        Command::new("git").arg("-C").arg(here(dir)).args(args),
        deadline,
    )
    .map(|out| out.trim().to_string())
}

fn parse_pr(pr: &Value) -> Option<Pr> {
    let text = |key: &str| {
        pr.get(key)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    Some(Pr {
        number: pr.get("number").and_then(Value::as_u64)?,
        base: text("baseRefName"),
        head: text("headRefName"),
        state: text("state"),
        head_oid: text("headRefOid"),
    })
}

const PR_FIELDS: &str = "number,baseRefName,headRefName,state,headRefOid";

impl Lookup for Live {
    fn origin_repo(&self, dir: &str, deadline: Instant) -> Option<String> {
        github_repo(&git(dir, &["remote", "get-url", "origin"], deadline)?)
    }

    fn current_branch(&self, dir: &str, deadline: Instant) -> Option<String> {
        git(
            dir,
            &["symbolic-ref", "--quiet", "--short", "HEAD"],
            deadline,
        )
        .filter(|b| !b.is_empty())
    }

    fn default_branch(&self, dir: &str, repo: &str, deadline: Instant) -> Option<String> {
        if self.origin_repo(dir, deadline).as_deref() == Some(repo) {
            if let Some(head) = git(
                dir,
                &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
                deadline,
            ) {
                if let Some(name) = head.strip_prefix("origin/") {
                    return Some(name.to_string());
                }
            }
        }
        run_bounded(
            Command::new("gh").args([
                "repo",
                "view",
                repo,
                "--json",
                "defaultBranchRef",
                "-q",
                ".defaultBranchRef.name",
            ]),
            deadline,
        )
        .map(|out| out.trim().to_string())
        .filter(|name| !name.is_empty())
    }

    fn pr_by_head(&self, repo: &str, head: &str, deadline: Instant) -> Option<Pr> {
        let out = run_bounded(
            Command::new("gh").args([
                "pr", "list", "--repo", repo, "--head", head, "--state", "all", "--limit", "20",
                "--json", PR_FIELDS,
            ]),
            deadline,
        )?;
        let listed: Vec<Value> = serde_json::from_str(&out).ok()?;
        let prs: Vec<Pr> = listed.iter().filter_map(parse_pr).collect();
        prs.iter()
            .find(|pr| pr.is_open())
            .or_else(|| prs.first())
            .cloned()
    }

    fn pr_by_number(&self, repo: &str, number: u64, deadline: Instant) -> Option<Pr> {
        let out = run_bounded(
            Command::new("gh").args([
                "pr",
                "view",
                &number.to_string(),
                "--repo",
                repo,
                "--json",
                PR_FIELDS,
            ]),
            deadline,
        )?;
        parse_pr(&serde_json::from_str(&out).ok()?)
    }

    fn is_ancestor(
        &self,
        dir: &str,
        ancestor: &str,
        descendant: &str,
        deadline: Instant,
    ) -> Option<bool> {
        let mut command = Command::new("git");
        command.args([
            "-C",
            here(dir),
            "merge-base",
            "--is-ancestor",
            ancestor,
            descendant,
        ]);
        match exit_code(&mut command, deadline)? {
            0 => Some(true),
            1 => Some(false),
            _ => None,
        }
    }

    fn already_upstream(
        &self,
        dir: &str,
        upstream: &str,
        tip: &str,
        deadline: Instant,
    ) -> Option<usize> {
        let out = git(dir, &["cherry", upstream, tip], deadline)?;
        Some(out.lines().filter(|line| line.starts_with('-')).count())
    }

    fn own_commits_since(&self, dir: &str, since: &str, deadline: Instant) -> Option<u64> {
        let email = git(dir, &["config", "user.email"], deadline)?;
        let author = format!("--author=<{email}>");
        let range = format!("{since}..HEAD");
        git(dir, &["rev-list", "--count", &author, &range], deadline)?
            .parse()
            .ok()
    }

    fn gate_context(&self, dir: &str, deadline: Instant) -> Option<String> {
        git(
            dir,
            &["config", "--get", "stack-guard.gate-context"],
            deadline,
        )
        .filter(|c| !c.is_empty())
    }

    fn status(
        &self,
        repo: &str,
        sha: &str,
        context: &str,
        deadline: Instant,
    ) -> Option<Option<String>> {
        let path = format!("repos/{repo}/commits/{sha}/statuses?per_page=100");
        let out = run_bounded(Command::new("gh").args(["api", &path]), deadline)?;
        let statuses: Vec<Value> = serde_json::from_str(&out).ok()?;
        Some(
            statuses
                .iter()
                .find(|s| s.get("context").and_then(Value::as_str) == Some(context))
                .and_then(|s| s.get("state").and_then(Value::as_str))
                .map(str::to_string),
        )
    }
}

/// The exit code of a run, or `None` at the deadline or on a signal. The child
/// is killed THEN reaped on the deadline, so none outlives the hook.
fn exit_code(command: &mut Command, deadline: Instant) -> Option<i32> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.code(),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(15)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

#[cfg(test)]
#[path = "stack_guard_tests.rs"]
mod stack_guard_tests;

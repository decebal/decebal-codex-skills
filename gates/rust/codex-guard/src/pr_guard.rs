//! PreToolUse, Bash — one session keeps at most one open PR per GitHub repo.
//!
//! While a PR this session created is still open in a repo, `gh pr create` /
//! `gh pr new` and branch creation in that repo are denied: the change belongs on
//! the open PR's branch. A second PR is the user's call, made in their own words
//! and run by them with `!` in the Codex prompt.
//!
//! The session's PRs come from a ledger this guard writes. Every `gh pr create`
//! it allows appends `{"repo","head","at"}` to
//! `$CODEX_HOME/pr-guard/<session_id>.jsonl`; the session's open PRs in a repo are
//! the user's open PRs there whose head branch the ledger holds for that repo.
//!
//! A create is recorded before it runs, so a create that fails still leaves a
//! head behind. A head that already had an open PR at that moment is recorded
//! with that PR's number, which is then never counted as the session's.
//!
//! The rollout transcript is read only when the session has no ledger, which
//! means it began before the hook was installed. In code mode a long command's
//! output can arrive on a LATER tool call than the one that started it, so
//! joining a creation to its output by `call_id` misses PRs; the ledger does not
//! depend on reading any output.
//!
//! Fails open: no GitHub remote, no `gh`, an unreadable ledger or transcript, or
//! a lookup past the deadline all allow. A guard that blocked PRs whenever `gh`
//! was slow would be routed around, and then it guards nothing.

use crate::hook;
use crate::session_state;
use regex::Regex;
use serde_json::{json, Value};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime};

/// One budget for every child the guard runs, shared across all lookups.
const DEADLINE: Duration = Duration::from_secs(8);

/// Transcript bytes read at most; a rollout past this is scanned only in part.
pub(crate) const MAX_TRANSCRIPT_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Trigger {
    /// `gh pr create` / `gh pr new`, with the repo named by `-R`/`--repo`/`GH_REPO`
    /// and the branch named by `-H`/`--head`. `dry_run` covers `--dry-run` and
    /// `--web`, which create nothing from the command itself.
    CreatePr {
        repo: Option<String>,
        head: Option<String>,
        dry_run: bool,
    },
    /// A new local branch, in the repo at `dir` (`git -C`) or the session's cwd.
    /// `force` is `-C`/`-B`/`--force-create`/`branch -f`, which only resets a
    /// branch that already exists.
    CreateBranch {
        name: String,
        dir: Option<String>,
        force: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpenPr {
    pub number: u64,
    pub title: String,
    pub head: String,
}

/// The facts the guard reads from outside the process.
pub(crate) trait Lookup {
    /// `owner/repo` of the `origin` remote of the repo at `dir`, lowercased.
    fn origin_repo(&self, dir: &str, deadline: Instant) -> Option<String>;
    /// The branch checked out at `dir`; `None` when detached or unreadable.
    fn current_branch(&self, dir: &str, deadline: Instant) -> Option<String>;
    /// The user's open PRs in `repo`; empty when they cannot be read.
    fn open_prs(&self, repo: &str, deadline: Instant) -> Vec<OpenPr>;
    /// Whether `refs/heads/<name>` exists in the repo at `dir`.
    fn branch_exists(&self, dir: &str, name: &str, deadline: Instant) -> bool;
}

/// Open-PR lists, fetched at most once per repo per hook run.
struct OpenPrs<'a> {
    lookup: &'a dyn Lookup,
    deadline: Instant,
    by_repo: HashMap<String, Vec<OpenPr>>,
}

impl OpenPrs<'_> {
    fn get(&mut self, repo: &str) -> &[OpenPr] {
        let (lookup, deadline) = (self.lookup, self.deadline);
        self.by_repo
            .entry(repo.to_string())
            .or_insert_with(|| lookup.open_prs(repo, deadline))
    }
}

pub fn run(payload: &Value) -> ! {
    let ledger = ledger_path(
        session_state::codex_home().as_deref(),
        hook::str_at(payload, &["session_id"]),
    );
    let verdict = decide(
        payload,
        &Live,
        ledger.as_deref(),
        Instant::now() + DEADLINE,
        SystemTime::now(),
    );
    match verdict {
        Some(reason) => hook::deny(&reason),
        None => hook::allow(),
    }
}

/// `<codex_home>/pr-guard/<session>.jsonl`, or `None` without a home or an id.
pub(crate) fn ledger_path(codex_home: Option<&Path>, session_id: &str) -> Option<PathBuf> {
    let name = session_state::file_name(session_id)?;
    Some(codex_home?.join("pr-guard").join(format!("{name}.jsonl")))
}

/// The deny reason, or `None` to allow. An allowed PR creation is recorded in
/// `ledger` before returning.
pub(crate) fn decide(
    payload: &Value,
    lookup: &dyn Lookup,
    ledger: Option<&Path>,
    deadline: Instant,
    now: SystemTime,
) -> Option<String> {
    if hook::str_at(payload, &["tool_name"]) != "Bash" {
        return None;
    }
    let triggers = classify_all(hook::str_at(payload, &["tool_input", "command"]));
    if triggers.is_empty() {
        return None;
    }
    let cwd = hook::str_at(payload, &["cwd"]);
    let entries: Vec<LedgerEntry> = ledger
        .and_then(|path| File::open(path).ok())
        .map(|file| read_ledger(BufReader::new(file)))
        .unwrap_or_default();
    let has_ledger = ledger.is_some_and(Path::exists);
    let transcript = hook::str_at(payload, &["transcript_path"]);
    let mut transcript_created: Option<Vec<(String, u64)>> = None;
    let mut open = OpenPrs {
        lookup,
        deadline,
        by_repo: HashMap::new(),
    };
    let mut to_record: Vec<(String, String)> = Vec::new();
    let mut new_branch: Option<String> = None;

    for trigger in &triggers {
        let repo = match trigger {
            Trigger::CreatePr {
                repo: Some(named), ..
            } => normalize_repo(named),
            Trigger::CreatePr { repo: None, .. } => lookup.origin_repo(cwd, deadline),
            Trigger::CreateBranch { dir, .. } => {
                lookup.origin_repo(dir.as_deref().unwrap_or(cwd), deadline)
            }
        };
        let Some(repo) = repo else {
            continue;
        };
        if let Trigger::CreateBranch {
            name,
            dir,
            force: true,
        } = trigger
        {
            if lookup.branch_exists(dir.as_deref().unwrap_or(cwd), name, deadline) {
                continue;
            }
        }
        let head = match trigger {
            Trigger::CreatePr {
                head,
                dry_run: false,
                ..
            } => head
                .clone()
                .or_else(|| new_branch.clone())
                .or_else(|| lookup.current_branch(cwd, deadline))
                .filter(|h| !h.is_empty()),
            _ => None,
        };

        let recorded: Vec<&LedgerEntry> = entries.iter().filter(|e| e.repo == repo).collect();
        if !recorded.is_empty() {
            let listed = open.get(&repo);
            let newest = recorded.iter().rev().find_map(|entry| {
                listed
                    .iter()
                    .find(|pr| pr.head == entry.head && Some(pr.number) != entry.preexisting)
            });
            if let Some(pr) = newest {
                return Some(reason(&repo, pr, trigger));
            }
        }
        if !has_ledger {
            let created = transcript_created.get_or_insert_with(|| {
                File::open(transcript)
                    .ok()
                    .map(|file| {
                        transcript_prs(BufReader::new(file.take(MAX_TRANSCRIPT_BYTES)), deadline)
                    })
                    .unwrap_or_default()
            });
            let numbers: Vec<u64> = created
                .iter()
                .filter(|(pr_repo, _)| *pr_repo == repo)
                .map(|(_, number)| *number)
                .collect();
            if !numbers.is_empty() {
                let listed = open.get(&repo);
                let newest = numbers
                    .iter()
                    .rev()
                    .find_map(|number| listed.iter().find(|pr| pr.number == *number));
                if let Some(pr) = newest {
                    return Some(reason(&repo, pr, trigger));
                }
            }
        }

        match trigger {
            Trigger::CreateBranch { name, .. } => new_branch = Some(name.clone()),
            Trigger::CreatePr { .. } => {
                if let Some(head) = head {
                    to_record.push((repo, head));
                }
            }
        }
    }

    if let Some(path) = ledger {
        for (repo, head) in to_record {
            let preexisting = open
                .get(&repo)
                .iter()
                .find(|pr| pr.head == head)
                .map(|pr| pr.number);
            let _ = append_ledger(path, &repo, &head, preexisting, now);
        }
    }
    None
}

/// `owner/repo` from a `-R`/`--repo`/`GH_REPO` value, lowercased. Accepts the
/// `[HOST/]OWNER/REPO` and URL forms `gh` accepts.
pub(crate) fn normalize_repo(value: &str) -> Option<String> {
    let value = value.trim();
    let value = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .unwrap_or(value);
    let value = value.strip_prefix("github.com/").unwrap_or(value);
    let value = value.trim_end_matches('/');
    let value = value.strip_suffix(".git").unwrap_or(value);
    let (owner, repo) = value.split_once('/')?;
    (!owner.is_empty() && !repo.is_empty() && !repo.contains('/'))
        .then(|| format!("{owner}/{repo}").to_lowercase())
}

pub(crate) fn reason(repo: &str, open: &OpenPr, trigger: &Trigger) -> String {
    let OpenPr {
        number,
        title,
        head,
    } = open;
    let facts = format!(
        "This session already has an open PR in {repo}: #{number} \"{title}\" (branch {head})."
    );
    match trigger {
        Trigger::CreatePr { .. } => format!(
            "{facts} Commit this change on {head} instead: a fix that PR needs to go green, a follow-up on the same surface, and work found along the way all belong on it. A second open PR from one session needs the user's own words; if they ask for one, they run it themselves by prefixing the command with `!` in the Codex prompt (`!gh pr create ...`)."
        ),
        Trigger::CreateBranch { name, .. } => format!(
            "{facts} Commit on {head} instead of creating branch {name}; start a new branch once #{number} merges, or when the user asks for one in their own words (they run `!git switch -c {name}` in the Codex prompt themselves)."
        ),
    }
}

/// Every PR or branch creation in `command`, in order. Heredoc bodies are data,
/// so they are removed before the command is split.
pub(crate) fn classify_all(command: &str) -> Vec<Trigger> {
    segments(&strip_heredoc_bodies(command))
        .iter()
        .filter_map(|segment| classify_segment(segment))
        .collect()
}

/// `command` with every heredoc body removed, keeping the line that opens it.
///
/// Quoting on the opening line is not tracked, so a `<<WORD` inside a quoted
/// string also starts a body. That hides lines from the guard, which errs
/// toward allowing.
pub(crate) fn strip_heredoc_bodies(command: &str) -> Cow<'_, str> {
    if !command.contains("<<") {
        return Cow::Borrowed(command);
    }
    let mut out = String::with_capacity(command.len());
    let mut pending: Vec<(String, bool)> = Vec::new();
    for line in command.split_inclusive('\n') {
        if let Some((delimiter, strip_tabs)) = pending.first() {
            let body = line.trim_end_matches(['\n', '\r']);
            let body = if *strip_tabs {
                body.trim_start_matches('\t')
            } else {
                body
            };
            if body == delimiter {
                pending.remove(0);
            }
            continue;
        }
        out.push_str(line);
        pending.extend(heredoc_delimiters(line));
    }
    Cow::Owned(out)
}

/// `(delimiter, strip_leading_tabs)` for each `<<WORD` / `<<-WORD` on `line`.
/// `<<<` is a here-string, which has no body.
fn heredoc_delimiters(line: &str) -> Vec<(String, bool)> {
    let chars: Vec<char> = line.chars().collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i + 1 < chars.len() {
        let opens = chars[i] == '<'
            && chars[i + 1] == '<'
            && (i == 0 || chars[i - 1] != '<')
            && chars.get(i + 2) != Some(&'<');
        if !opens {
            i += 1;
            continue;
        }
        let mut j = i + 2;
        let strip_tabs = chars.get(j) == Some(&'-');
        if strip_tabs {
            j += 1;
        }
        while chars.get(j).is_some_and(|c| *c == ' ' || *c == '\t') {
            j += 1;
        }
        let quote = chars.get(j).copied().filter(|c| *c == '\'' || *c == '"');
        if quote.is_some() {
            j += 1;
        }
        let start = j;
        while chars
            .get(j)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        {
            j += 1;
        }
        if j > start {
            found.push((chars[start..j].iter().collect(), strip_tabs));
        }
        i = j.max(i + 2);
    }
    found
}

fn classify_segment(tokens: &[String]) -> Option<Trigger> {
    let (env, argv) = strip_wrappers(tokens);
    match argv.first().map(String::as_str) {
        Some("gh") => classify_gh(&argv[1..], &env),
        Some("git") => classify_git(&argv[1..]),
        _ => None,
    }
}

/// Leading `VAR=value` assignments and the `env`, `command`, `rtk` and
/// `rtk proxy` wrappers come off; the assignments are kept for `GH_REPO`.
pub(crate) fn strip_wrappers(tokens: &[String]) -> (Vec<(String, String)>, Vec<String>) {
    let mut env = Vec::new();
    let mut rest = tokens;
    loop {
        match rest.first().map(String::as_str) {
            Some(token) if assignment(token).is_some() => {
                if let Some(pair) = assignment(token) {
                    env.push(pair);
                }
                rest = &rest[1..];
            }
            Some("env") => {
                rest = &rest[1..];
                while rest.first().is_some_and(|t| t.starts_with('-')) {
                    rest = &rest[1..];
                }
            }
            Some("command") => rest = &rest[1..],
            Some("rtk") => {
                rest = &rest[1..];
                if rest.first().map(String::as_str) == Some("proxy") {
                    rest = &rest[1..];
                }
            }
            _ => break,
        }
    }
    (env, rest.to_vec())
}

fn assignment(token: &str) -> Option<(String, String)> {
    let (name, value) = token.split_once('=')?;
    let valid = !name.is_empty()
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !name.starts_with(|c: char| c.is_ascii_digit());
    valid.then(|| (name.to_string(), value.to_string()))
}

fn classify_gh(args: &[String], env: &[(String, String)]) -> Option<Trigger> {
    let mut repo = env
        .iter()
        .find(|(name, _)| name == "GH_REPO")
        .map(|(_, value)| value.clone());
    let mut head = None;
    let mut words = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        if arg == "-R" || arg == "--repo" {
            repo = args.get(i + 1).cloned();
            i += 2;
            continue;
        }
        if arg == "-H" || arg == "--head" {
            head = args.get(i + 1).cloned();
            i += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--repo=") {
            repo = Some(value.to_string());
        } else if let Some(value) = arg.strip_prefix("--head=") {
            head = Some(value.to_string());
        } else if let Some(value) = arg.strip_prefix("-R").filter(|v| !v.is_empty()) {
            repo = Some(value.to_string());
        } else if let Some(value) = arg.strip_prefix("-H").filter(|v| !v.is_empty()) {
            head = Some(value.to_string());
        } else if !arg.starts_with('-') {
            words.push(arg);
        }
        i += 1;
    }
    let help = args.iter().any(|a| a == "-h" || a == "--help");
    let dry_run = args
        .iter()
        .any(|a| a == "--dry-run" || a == "--web" || a == "-w");
    let creates = words.first() == Some(&"pr") && matches!(words.get(1), Some(&"create" | &"new"));
    // `--head owner:branch` names a fork; `headRefName` carries the branch alone.
    let head = head.map(|h| h.rsplit(':').next().unwrap_or_default().to_string());
    (creates && !help).then_some(Trigger::CreatePr {
        repo,
        head,
        dry_run,
    })
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
    let subcommand = args.get(i)?.as_str();
    let rest = &args[i + 1..];
    let (name, force) = match subcommand {
        "switch" => create_or_reset(rest, &["-c", "--create"], &["-C", "--force-create"]),
        "checkout" => create_or_reset(rest, &["-b"], &["-B"]),
        "worktree" if rest.first().map(String::as_str) == Some("add") => {
            create_or_reset(&rest[1..], &["-b"], &["-B"])
        }
        "branch" => new_branch_name(rest).map(|name| {
            let force = rest.iter().any(|a| a == "-f" || a == "--force");
            (name, force)
        }),
        _ => None,
    }?;
    Some(Trigger::CreateBranch { name, dir, force })
}

/// The branch a create flag names, and whether the flag was the resetting form.
fn create_or_reset(args: &[String], create: &[&str], reset: &[&str]) -> Option<(String, bool)> {
    value_after(args, create)
        .map(|name| (name, false))
        .or_else(|| value_after(args, reset).map(|name| (name, true)))
}

fn value_after(args: &[String], flags: &[&str]) -> Option<String> {
    args.iter().enumerate().find_map(|(i, arg)| {
        if flags.contains(&arg.as_str()) {
            return args.get(i + 1).cloned();
        }
        flags.iter().find_map(|flag| {
            let long = flag.starts_with("--");
            let value = arg.strip_prefix(&format!("{flag}="))?;
            long.then(|| value.to_string())
        })
    })
}

/// `git branch <name> [<start>]` creates; listing, deleting, renaming and
/// upstream edits do not.
fn new_branch_name(args: &[String]) -> Option<String> {
    const NOT_CREATING: &[&str] = &[
        "-d",
        "-D",
        "--delete",
        "-m",
        "-M",
        "--move",
        "-l",
        "--list",
        "-a",
        "--all",
        "-r",
        "--remotes",
        "-v",
        "-vv",
        "--verbose",
        "--show-current",
        "--contains",
        "--no-contains",
        "--merged",
        "--no-merged",
        "--points-at",
        "-u",
        "--set-upstream-to",
        "--unset-upstream",
        "--edit-description",
    ];
    let listing = args.iter().any(|arg| {
        let flag = arg.split_once('=').map_or(arg.as_str(), |(flag, _)| flag);
        let combined = flag.len() > 2 && flag.starts_with('-') && !flag.starts_with("--");
        let short_listing = combined
            && flag[1..]
                .chars()
                .any(|c| NOT_CREATING.contains(&format!("-{c}").as_str()));
        NOT_CREATING.contains(&flag)
            || short_listing
            || flag.starts_with("--format")
            || flag.starts_with("--sort")
    });
    if listing {
        return None;
    }
    args.iter().find(|arg| !arg.starts_with('-')).cloned()
}

/// Shell words, split into segments at unquoted `;`, `&&`, `||`, `|` and newlines.
pub(crate) fn segments(command: &str) -> Vec<Vec<String>> {
    let mut segments = Vec::new();
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote: Option<char> = None;
    let mut chars = command.chars().peekable();

    let end_word = |word: &mut String, words: &mut Vec<String>| {
        if !word.is_empty() {
            words.push(std::mem::take(word));
        }
    };

    while let Some(ch) = chars.next() {
        match quote {
            Some(q) if ch == q => quote = None,
            Some('"') if ch == '\\' => {
                if let Some(next) = chars.next() {
                    word.push(next);
                }
            }
            Some(_) => word.push(ch),
            None => match ch {
                '\'' | '"' => quote = Some(ch),
                '\\' => {
                    if let Some(next) = chars.next() {
                        word.push(next);
                    }
                }
                ' ' | '\t' => end_word(&mut word, &mut words),
                ';' | '\n' | '|' | '&' => {
                    if matches!(ch, '|' | '&') && chars.peek() == Some(&ch) {
                        chars.next();
                    }
                    end_word(&mut word, &mut words);
                    if !words.is_empty() {
                        segments.push(std::mem::take(&mut words));
                    }
                }
                _ => word.push(ch),
            },
        }
    }
    end_word(&mut word, &mut words);
    if !words.is_empty() {
        segments.push(words);
    }
    segments
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LedgerEntry {
    pub repo: String,
    pub head: String,
    /// The PR already open on `head` when the create was recorded; not the session's.
    pub preexisting: Option<u64>,
}

/// Every well-formed ledger line, in the order appended. A torn or foreign line
/// is skipped rather than ending the read.
pub(crate) fn read_ledger(reader: impl BufRead) -> Vec<LedgerEntry> {
    reader
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| {
            let entry: Value = serde_json::from_str(&line).ok()?;
            let text = |key: &str| {
                entry
                    .get(key)
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
            };
            Some(LedgerEntry {
                repo: text("repo")?.to_lowercase(),
                head: text("head")?,
                preexisting: entry.get("preexisting").and_then(Value::as_u64),
            })
        })
        .collect()
}

pub(crate) fn append_ledger(
    path: &Path,
    repo: &str,
    head: &str,
    preexisting: Option<u64>,
    now: SystemTime,
) -> std::io::Result<()> {
    session_state::prepare(path, now);
    let at = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let mut line = json!({"repo": repo, "head": head, "at": at});
    if let Some(number) = preexisting {
        line["preexisting"] = json!(number);
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")
}

/// `(repo, number)` for every PR whose creation and URL-bearing output share a
/// `call_id` in a Codex rollout, in creation order, repo lowercased. A call
/// counts only when a command it runs classifies as a PR creation; an output
/// reporting that the PR already exists names someone's earlier PR.
///
/// Stops at `deadline`; the caller bounds the bytes with `Read::take`.
pub(crate) fn transcript_prs(mut reader: impl BufRead, deadline: Instant) -> Vec<(String, u64)> {
    let mut created: Vec<(String, u64)> = Vec::new();
    let mut seen: HashSet<(String, u64)> = HashSet::new();
    let mut pending: HashSet<String> = HashSet::new();
    let mut buf = Vec::new();

    loop {
        if Instant::now() >= deadline {
            break;
        }
        buf.clear();
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let line = String::from_utf8_lossy(&buf);
        let relevant = line.contains("gh pr create")
            || line.contains("gh pr new")
            || (!pending.is_empty() && line.contains("/pull/"));
        if !relevant {
            continue;
        }
        let Ok(entry) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if entry.get("type").and_then(Value::as_str) != Some("response_item") {
            continue;
        }
        let Some(item) = entry.get("payload") else {
            continue;
        };
        let call_id = item.get("call_id").and_then(Value::as_str).unwrap_or("");
        let text_of = |key: &str| item.get(key).and_then(Value::as_str).unwrap_or("");
        match item.get("type").and_then(Value::as_str).unwrap_or("") {
            "function_call" | "custom_tool_call" => {
                let creates = call_commands(text_of("arguments"), text_of("input"))
                    .iter()
                    .flat_map(|command| classify_all(command))
                    .any(|t| matches!(t, Trigger::CreatePr { dry_run: false, .. }));
                if creates && !call_id.is_empty() {
                    pending.insert(call_id.to_string());
                }
            }
            "function_call_output" | "custom_tool_call_output" => {
                if !pending.remove(call_id) {
                    continue;
                }
                let text = output_text(item.get("output"));
                if text.contains("already exists") {
                    continue;
                }
                for pr in pull_urls(&text) {
                    if seen.insert(pr.clone()) {
                        created.push(pr);
                    }
                }
            }
            _ => {}
        }
    }
    created
}

/// The shell commands a recorded tool call runs.
///
/// A `function_call` carries JSON arguments whose `cmd` or `command` is a string
/// or an argv; a `bash -c <script>` argv yields the script. A code-mode
/// `custom_tool_call` carries JavaScript, whose `cmd:` string literals are the
/// commands.
pub(crate) fn call_commands(arguments: &str, input: &str) -> Vec<String> {
    let mut commands = Vec::new();
    if let Ok(args) = serde_json::from_str::<Value>(arguments) {
        for key in ["cmd", "command"] {
            match args.get(key) {
                Some(Value::String(command)) => commands.push(command.clone()),
                Some(Value::Array(argv)) => {
                    let argv: Vec<&str> = argv.iter().filter_map(Value::as_str).collect();
                    let shell = argv.first().is_some_and(|p| {
                        matches!(p.rsplit('/').next(), Some("bash" | "sh" | "zsh"))
                    });
                    let script = argv
                        .get(1)
                        .is_some_and(|f| f.starts_with('-') && f.contains('c'));
                    match (shell && script, argv.get(2)) {
                        (true, Some(script)) => commands.push((*script).to_string()),
                        _ => commands.push(argv.join(" ")),
                    }
                }
                _ => {}
            }
        }
    }
    static CMD: OnceLock<Regex> = OnceLock::new();
    let cmd = CMD.get_or_init(|| {
        Regex::new(
            r#""?\bcmd"?\s*:\s*(?:"((?:[^"\\]|\\.)*)"|'((?:[^'\\]|\\.)*)'|`((?:[^`\\]|\\.)*)`)"#,
        )
        .expect("valid regex")
    });
    for caps in cmd.captures_iter(input) {
        if let Some(literal) = caps.get(1).or_else(|| caps.get(2)).or_else(|| caps.get(3)) {
            commands.push(unescape_js(literal.as_str()));
        }
    }
    commands
}

/// The value of a JavaScript string literal body: `\n`, `\t`, `\r` and a
/// backslash before any other character.
fn unescape_js(literal: &str) -> String {
    let mut out = String::with_capacity(literal.len());
    let mut chars = literal.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

fn output_text(output: Option<&Value>) -> String {
    match output {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(blocks)) => blocks
            .iter()
            .filter_map(|b| b.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// Every `github.com/<owner>/<repo>/pull/<n>` in `text`, repo lowercased.
pub(crate) fn pull_urls(text: &str) -> Vec<(String, u64)> {
    static PULL: OnceLock<Regex> = OnceLock::new();
    let pull = PULL.get_or_init(|| {
        Regex::new(r"github\.com/([A-Za-z0-9._-]+)/([A-Za-z0-9._-]+)/pull/(\d+)")
            .expect("valid regex")
    });
    pull.captures_iter(text)
        .filter_map(|c| {
            let number = c[3].parse().ok()?;
            Some((format!("{}/{}", &c[1], &c[2]).to_lowercase(), number))
        })
        .collect()
}

/// `owner/repo` from an https or ssh GitHub remote URL, lowercased.
pub(crate) fn github_repo(url: &str) -> Option<String> {
    let url = url.trim();
    let path = url
        .strip_prefix("git@github.com:")
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))
        .or_else(|| url.strip_prefix("https://github.com/"))
        .or_else(|| url.strip_prefix("http://github.com/"))?;
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let (owner, repo) = path.split_once('/')?;
    (!owner.is_empty() && !repo.is_empty() && !repo.contains('/'))
        .then(|| format!("{owner}/{repo}").to_lowercase())
}

/// The real lookups: `git` and `gh`, each run as a child this process owns.
struct Live;

fn dir_or_here(dir: &str) -> &str {
    if dir.is_empty() {
        "."
    } else {
        dir
    }
}

impl Lookup for Live {
    fn origin_repo(&self, dir: &str, deadline: Instant) -> Option<String> {
        let out = run_bounded(
            Command::new("git").args(["-C", dir_or_here(dir), "remote", "get-url", "origin"]),
            deadline,
        )?;
        github_repo(&out)
    }

    fn current_branch(&self, dir: &str, deadline: Instant) -> Option<String> {
        let out = run_bounded(
            Command::new("git").args(["-C", dir_or_here(dir), "branch", "--show-current"]),
            deadline,
        )?;
        let branch = out.trim();
        (!branch.is_empty()).then(|| branch.to_string())
    }

    fn branch_exists(&self, dir: &str, name: &str, deadline: Instant) -> bool {
        let reference = format!("refs/heads/{name}");
        run_bounded(
            Command::new("git").args([
                "-C",
                dir_or_here(dir),
                "rev-parse",
                "--verify",
                "--quiet",
                &reference,
            ]),
            deadline,
        )
        .is_some()
    }

    fn open_prs(&self, repo: &str, deadline: Instant) -> Vec<OpenPr> {
        let out = run_bounded(
            Command::new("gh").args([
                "pr",
                "list",
                "--repo",
                repo,
                "--state",
                "open",
                "--author",
                "@me",
                "--limit",
                "200",
                "--json",
                "number,headRefName,title",
            ]),
            deadline,
        );
        let listed: Vec<Value> = out
            .and_then(|out| serde_json::from_str(&out).ok())
            .unwrap_or_default();
        listed
            .iter()
            .filter_map(|pr| {
                let text = |key: &str| {
                    pr.get(key)
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string()
                };
                Some(OpenPr {
                    number: pr.get("number").and_then(Value::as_u64)?,
                    title: text("title"),
                    head: text("headRefName"),
                })
            })
            .collect()
    }
}

/// Stdout of a successful run, or `None` on failure or at the deadline. The
/// child is killed THEN reaped on the deadline, so none outlives the hook.
///
/// Stdout drains on a reader thread while this thread polls the child, so an
/// output larger than the pipe buffer cannot stall the child until the deadline.
/// After a kill the reader is not joined: a grandchild can still hold the pipe,
/// and the hook process exits right after.
pub(crate) fn run_bounded(command: &mut Command, deadline: Instant) -> Option<String> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut out = String::new();
        stdout.read_to_string(&mut out).ok().map(|_| out)
    });
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return reader.join().ok().flatten(),
            Ok(Some(_)) => return None,
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
#[path = "pr_guard_tests.rs"]
mod pr_guard_tests;

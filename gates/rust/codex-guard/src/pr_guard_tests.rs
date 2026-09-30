//! What the one-open-PR guard must deny, and everything it must leave alone.

use super::{
    append_ledger, call_commands, classify_all, decide, github_repo, ledger_path, normalize_repo,
    pull_urls, read_ledger, reason, run_bounded, segments, strip_heredoc_bodies, strip_wrappers,
    transcript_prs, LedgerEntry, Lookup, OpenPr, Trigger,
};
use crate::session_state::{set_mtime, MAX_AGE};
use serde_json::{json, Value};
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime};

fn classify(command: &str) -> Option<Trigger> {
    classify_all(command).into_iter().next()
}

struct Fake {
    origin: Option<&'static str>,
    branch: Option<&'static str>,
    open: Vec<(&'static str, u64, &'static str)>,
    existing: Vec<&'static str>,
}

impl Fake {
    fn new(open: Vec<(&'static str, u64, &'static str)>) -> Fake {
        Fake {
            origin: Some("acme/widgets"),
            branch: Some("feat/x"),
            open,
            existing: vec!["main", "feat/x"],
        }
    }
}

impl Lookup for Fake {
    fn origin_repo(&self, _dir: &str, _deadline: Instant) -> Option<String> {
        self.origin.map(str::to_string)
    }
    fn current_branch(&self, _dir: &str, _deadline: Instant) -> Option<String> {
        self.branch.map(str::to_string)
    }
    fn open_prs(&self, repo: &str, _deadline: Instant) -> Vec<OpenPr> {
        self.open
            .iter()
            .filter(|(r, _, _)| *r == repo)
            .map(|(_, n, head)| OpenPr {
                number: *n,
                title: "the open one".into(),
                head: (*head).into(),
            })
            .collect()
    }
    fn branch_exists(&self, _dir: &str, name: &str, _deadline: Instant) -> bool {
        self.existing.contains(&name)
    }
}

fn entry(repo: &str, head: &str) -> LedgerEntry {
    LedgerEntry {
        repo: repo.into(),
        head: head.into(),
        preexisting: None,
    }
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("codex-guard-pr-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn ledger_in(dir: &Path) -> PathBuf {
    dir.join("pr-guard").join("s1.jsonl")
}

fn record(ledger: &Path, repo: &str, head: &str) {
    append_ledger(ledger, repo, head, None, SystemTime::now()).unwrap();
}

fn entries(ledger: &Path) -> Vec<LedgerEntry> {
    File::open(ledger)
        .map(|f| read_ledger(BufReader::new(f)))
        .unwrap_or_default()
}

fn payload(command: &str) -> Value {
    json!({"session_id": "s1", "turn_id": "t1", "transcript_path": null, "cwd": "/work/widgets",
           "hook_event_name": "PreToolUse", "model": "gpt-5", "permission_mode": "default",
           "tool_name": "Bash", "tool_input": {"command": command}, "tool_use_id": "call_9"})
}

fn with_transcript(command: &str, transcript: &Path) -> Value {
    let mut p = payload(command);
    p["transcript_path"] = json!(transcript.to_str().unwrap());
    p
}

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

fn decide_with(payload: &Value, fake: &Fake, ledger: &Path) -> Option<String> {
    decide(payload, fake, Some(ledger), deadline(), SystemTime::now())
}

/// One rollout line, as Codex writes it.
fn rollout(item: Value) -> String {
    let line =
        json!({"timestamp": "2026-09-30T10:00:00.000Z", "type": "response_item", "payload": item});
    format!("{line}\n")
}

/// Code mode: a `custom_tool_call` running `gh pr create`, and its output.
fn code_mode_create(call_id: &str, output: &str) -> String {
    let input = "text(await tools.exec_command({cmd:\"gh pr create --repo acme/widgets --title t --body-file b.md\", workdir:\"/work/widgets\"}))";
    [
        rollout(
            json!({"type": "custom_tool_call", "name": "exec", "input": input, "call_id": call_id, "status": "completed"}),
        ),
        rollout(json!({"type": "custom_tool_call_output", "call_id": call_id, "output": output})),
    ]
    .concat()
}

#[test]
fn a_second_pr_while_the_first_is_open_is_denied() {
    let dir = scratch("second");
    let ledger = ledger_in(&dir);
    record(&ledger, "acme/widgets", "feat/x");
    let fake = Fake::new(vec![("acme/widgets", 12, "feat/x")]);
    let why = decide_with(&payload("gh pr create --title Second"), &fake, &ledger).expect("denied");
    assert!(why.contains("acme/widgets"), "{why}");
    assert!(why.contains("#12"), "{why}");
    assert!(why.contains("the open one"), "{why}");
    assert!(why.contains("feat/x"), "{why}");
    assert!(why.contains("!gh pr create"), "{why}");
}

#[test]
fn an_allowed_creation_is_recorded_with_the_cwd_branch() {
    let dir = scratch("record-cwd");
    let ledger = ledger_in(&dir);
    let fake = Fake::new(vec![]);
    assert!(decide_with(&payload("gh pr create --fill"), &fake, &ledger).is_none());
    assert_eq!(entries(&ledger), vec![entry("acme/widgets", "feat/x")]);
    let line = std::fs::read_to_string(&ledger).unwrap();
    let parsed: Value = serde_json::from_str(line.trim()).unwrap();
    assert!(parsed["at"].as_u64().unwrap() > 0, "{line}");
}

#[test]
fn the_head_flag_wins_over_the_cwd_branch() {
    let dir = scratch("record-head");
    let ledger = ledger_in(&dir);
    let fake = Fake::new(vec![]);
    for command in [
        "gh pr create --head feat/y --fill",
        "gh pr create -H feat/y",
        "gh pr create --head=feat/y",
        "gh pr create --head acme:feat/y",
    ] {
        let _ = std::fs::remove_file(&ledger);
        assert!(decide_with(&payload(command), &fake, &ledger).is_none());
        assert_eq!(entries(&ledger)[0].head, "feat/y", "{command}");
    }
}

#[test]
fn an_unknown_head_or_repo_records_nothing_and_allows() {
    let dir = scratch("unknown");
    let ledger = ledger_in(&dir);
    let detached = Fake {
        branch: None,
        ..Fake::new(vec![])
    };
    assert!(decide_with(&payload("gh pr create"), &detached, &ledger).is_none());
    assert!(entries(&ledger).is_empty());

    let no_remote = Fake {
        origin: None,
        ..Fake::new(vec![])
    };
    assert!(decide_with(&payload("gh pr create"), &no_remote, &ledger).is_none());
    assert!(entries(&ledger).is_empty());
}

#[test]
fn a_second_pr_after_the_first_merged_is_allowed_and_recorded() {
    let dir = scratch("merged");
    let ledger = ledger_in(&dir);
    record(&ledger, "acme/widgets", "feat/old");
    let fake = Fake::new(vec![]);
    assert!(decide_with(&payload("gh pr create --title Next"), &fake, &ledger).is_none());
    assert_eq!(entries(&ledger).len(), 2);
}

#[test]
fn an_open_pr_in_another_repo_does_not_block_this_one() {
    let dir = scratch("other-repo");
    let ledger = ledger_in(&dir);
    record(&ledger, "acme/gadgets", "feat/x");
    let fake = Fake::new(vec![("acme/gadgets", 3, "feat/x")]);
    assert!(decide_with(&payload("gh pr create"), &fake, &ledger).is_none());
}

#[test]
fn a_pr_the_session_did_not_create_does_not_count() {
    let dir = scratch("foreign");
    let ledger = ledger_in(&dir);
    record(&ledger, "acme/widgets", "feat/mine");
    let fake = Fake::new(vec![("acme/widgets", 99, "someone-else")]);
    assert!(decide_with(&payload("gh pr create"), &fake, &ledger).is_none());
}

#[test]
fn the_repo_named_on_the_command_is_the_one_checked() {
    let dir = scratch("repo-flag");
    let ledger = ledger_in(&dir);
    record(&ledger, "acme/widgets", "feat/x");
    let fake = Fake {
        origin: Some("acme/unrelated"),
        ..Fake::new(vec![("acme/widgets", 12, "feat/x")])
    };
    for command in [
        "gh pr create -R Acme/Widgets",
        "gh pr create --repo=acme/widgets",
        "GH_REPO=acme/widgets gh pr create",
    ] {
        assert!(
            decide_with(&payload(command), &fake, &ledger).is_some(),
            "{command}"
        );
    }
    assert!(decide_with(&payload("gh pr create"), &fake, &ledger).is_none());
}

#[test]
fn every_branch_creation_form_is_denied_while_a_pr_is_open() {
    let dir = scratch("branches");
    let ledger = ledger_in(&dir);
    record(&ledger, "acme/widgets", "feat/x");
    let fake = Fake::new(vec![("acme/widgets", 12, "feat/x")]);
    for command in [
        "git switch -c feat/next",
        "git switch --create feat/next",
        "git -C /work/widgets switch -C feat/next",
        "git checkout -b feat/next",
        "git checkout -B feat/next origin/main",
        "git worktree add -b feat/next ../wt origin/main",
        "git worktree add ../wt -b feat/next",
        "git branch feat/next origin/main",
        "git -c core.x=1 branch feat/next",
    ] {
        let why = decide_with(&payload(command), &fake, &ledger);
        let why = why.unwrap_or_else(|| panic!("{command} should be denied"));
        assert!(why.contains("feat/next"), "{command}: {why}");
        assert!(why.contains("feat/x"), "{command}: {why}");
    }
    assert_eq!(entries(&ledger).len(), 1, "a branch is never recorded");
}

#[test]
fn commands_that_create_nothing_are_allowed_even_with_an_open_pr() {
    let dir = scratch("harmless");
    let ledger = ledger_in(&dir);
    record(&ledger, "acme/widgets", "feat/x");
    let fake = Fake::new(vec![("acme/widgets", 12, "feat/x")]);
    for command in [
        "gh pr create --help",
        "gh pr list --search \"gh pr create\"",
        "gh pr view 12",
        "git commit -m \"gh pr create next time\"",
        "grep -rn \"git switch -c\" docs",
        "echo gh pr create",
        "git branch",
        "git branch -d old",
        "git branch -D old",
        "git branch -m old new",
        "git branch --show-current",
        "git branch -vv",
        "git branch --merged main",
        "git branch -u origin/main",
        "git switch main",
        "git checkout main",
        "git checkout -- file.txt",
        "git worktree add --detach ../wt origin/main",
        "git worktree remove ../wt",
    ] {
        assert!(
            decide_with(&payload(command), &fake, &ledger).is_none(),
            "{command} should be allowed"
        );
    }
}

#[test]
fn the_newest_open_pr_is_named() {
    let dir = scratch("newest");
    let ledger = ledger_in(&dir);
    record(&ledger, "acme/widgets", "feat/old");
    record(&ledger, "acme/widgets", "feat/new");
    let fake = Fake::new(vec![
        ("acme/widgets", 10, "feat/old"),
        ("acme/widgets", 11, "feat/new"),
    ]);
    let why = decide_with(&payload("gh pr create"), &fake, &ledger).unwrap();
    assert!(why.contains("#11"), "{why}");
}

#[test]
fn missing_or_unreadable_inputs_allow() {
    let fake = Fake::new(vec![("acme/widgets", 12, "feat/x")]);
    assert!(decide(
        &payload("git switch -c y"),
        &fake,
        None,
        deadline(),
        SystemTime::now()
    )
    .is_none());

    let dir = scratch("unreadable");
    let garbled = ledger_in(&dir);
    std::fs::create_dir_all(garbled.parent().unwrap()).unwrap();
    std::fs::write(&garbled, "{not json\n\u{0}\u{1}\n").unwrap();
    assert!(decide_with(&payload("git switch -c y"), &fake, &garbled).is_none());

    let missing = PathBuf::from("/nonexistent/codex-guard/rollout.jsonl");
    assert!(decide_with(
        &with_transcript("git switch -c y", &missing),
        &fake,
        &garbled
    )
    .is_none());

    let other_tool = json!({"tool_name": "apply_patch", "tool_input": {"command": "gh pr create"}});
    assert!(decide_with(&other_tool, &fake, &garbled).is_none());
}

#[test]
fn a_transcript_created_pr_counts_for_a_session_older_than_the_ledger() {
    let dir = scratch("transcript");
    let path = dir.join("rollout.jsonl");
    std::fs::write(
        &path,
        code_mode_create(
            "call_1",
            "ok created #12 https://github.com/acme/widgets/pull/12",
        ),
    )
    .unwrap();
    let fake = Fake::new(vec![("acme/widgets", 12, "feat/x")]);
    let why = decide_with(
        &with_transcript("git checkout -b feat/y", &path),
        &fake,
        &ledger_in(&dir),
    )
    .expect("denied from the transcript");
    assert!(why.contains("#12"), "{why}");
}

#[test]
fn transcript_reads_function_calls_with_block_outputs() {
    let args = json!({"command": ["bash", "-lc", "gh pr new --fill"], "workdir": "/work/widgets"});
    let body = [
        rollout(json!({"type": "function_call", "name": "shell", "arguments": args.to_string(), "call_id": "fc_1"})),
        rollout(json!({"type": "function_call_output", "call_id": "fc_1",
            "output": [{"type": "input_text", "text": "https://github.com/Acme/Widgets/pull/5\n"}]})),
    ]
    .concat();
    assert_eq!(
        transcript_prs(Cursor::new(body), deadline()),
        vec![("acme/widgets".to_string(), 5)]
    );
}

#[test]
fn an_output_on_a_later_call_is_not_claimed_by_the_transcript_source() {
    // The create's own call returns a running process; the URL arrives on the
    // `write_stdin` call that polled it, under a different call_id.
    let body = [
        code_mode_create("call_1", "Process running with session ID 7"),
        rollout(
            json!({"type": "function_call", "name": "write_stdin", "arguments": "{\"session_id\":7}", "call_id": "call_2"}),
        ),
        rollout(json!({"type": "function_call_output", "call_id": "call_2",
            "output": "ok created #13 https://github.com/acme/widgets/pull/13"})),
    ]
    .concat();
    assert!(transcript_prs(Cursor::new(body.clone()), deadline()).is_empty());

    let dir = scratch("later-call");
    let path = dir.join("rollout.jsonl");
    std::fs::write(&path, body).unwrap();
    let ledger = ledger_in(&dir);
    let before = Fake::new(vec![]);
    assert!(decide_with(&with_transcript("gh pr create", &path), &before, &ledger).is_none());
    let after = Fake::new(vec![("acme/widgets", 13, "feat/x")]);
    assert!(
        decide_with(&with_transcript("gh pr create", &path), &after, &ledger).is_some(),
        "the ledger written by the first call is what catches the second"
    );
}

#[test]
fn a_pull_url_in_an_unrelated_output_does_not_count() {
    let body = [
        rollout(
            json!({"type": "function_call", "name": "shell", "arguments": "{\"command\":\"gh pr view 5\"}", "call_id": "v1"}),
        ),
        rollout(json!({"type": "function_call_output", "call_id": "v1",
            "output": "https://github.com/acme/widgets/pull/5"})),
    ]
    .concat();
    assert!(transcript_prs(Cursor::new(body), deadline()).is_empty());
}

#[test]
fn the_transcript_scan_stops_at_the_deadline() {
    let body = code_mode_create(
        "call_1",
        "ok created #12 https://github.com/acme/widgets/pull/12",
    );
    let past = Instant::now() - Duration::from_millis(1);
    assert!(transcript_prs(Cursor::new(body), past).is_empty());
}

#[test]
fn the_ledger_path_carries_a_sanitised_session_id() {
    let home = Path::new("/home/u/.codex");
    assert_eq!(
        ledger_path(Some(home), "../../x y"),
        Some(PathBuf::from("/home/u/.codex/pr-guard/xy.jsonl"))
    );
    assert_eq!(ledger_path(Some(home), "/../"), None);
    assert_eq!(ledger_path(None, "s1"), None);
}

#[test]
fn a_torn_ledger_line_is_skipped_and_repos_are_lowercased() {
    let body = "{\"repo\":\"Acme/Widgets\",\"head\":\"feat/x\",\"at\":1}\n{\"repo\":\"acme/wid\n{\"repo\":\"acme/widgets\",\"head\":\"\"}\n{\"repo\":\"acme/widgets\",\"head\":\"feat/y\",\"at\":2}\n";
    assert_eq!(
        read_ledger(Cursor::new(body)),
        vec![
            entry("acme/widgets", "feat/x"),
            entry("acme/widgets", "feat/y")
        ]
    );
}

#[test]
fn a_new_ledger_prunes_stale_ones_and_an_existing_one_does_not() {
    let dir = scratch("prune");
    let old = dir.join("pr-guard").join("old-session.jsonl");
    std::fs::create_dir_all(old.parent().unwrap()).unwrap();
    std::fs::write(&old, "").unwrap();
    set_mtime(&old, SystemTime::now() - MAX_AGE - Duration::from_secs(60));

    let ledger = ledger_in(&dir);
    std::fs::write(&ledger, "").unwrap();
    record(&ledger, "acme/widgets", "feat/x");
    assert!(
        old.exists(),
        "appending to an existing ledger must not prune"
    );

    record(
        &dir.join("pr-guard").join("s2.jsonl"),
        "acme/widgets",
        "feat/z",
    );
    assert!(!old.exists());
    assert!(ledger.exists());
}

#[test]
fn a_help_request_is_not_a_creation() {
    assert_eq!(classify("gh pr create --help"), None);
    assert_eq!(classify("gh pr new -h"), None);
}

#[test]
fn wrappers_come_off_before_classifying() {
    for command in [
        "rtk gh pr create",
        "rtk proxy gh pr create",
        "env GH_TOKEN=x gh pr create",
        "env -i gh pr create",
        "command gh pr create",
        "A=1 B=2 gh pr create",
    ] {
        assert!(
            matches!(classify(command), Some(Trigger::CreatePr { .. })),
            "{command}"
        );
    }
    let words: Vec<String> = ["GH_REPO=acme/widgets", "rtk", "proxy", "gh", "pr"]
        .map(String::from)
        .to_vec();
    let (env, argv) = strip_wrappers(&words);
    assert_eq!(env, vec![("GH_REPO".into(), "acme/widgets".into())]);
    assert_eq!(argv, vec!["gh".to_string(), "pr".into()]);
}

#[test]
fn a_chained_command_is_checked_segment_by_segment() {
    assert!(matches!(
        classify("git add a && git commit -m x && gh pr create --fill"),
        Some(Trigger::CreatePr { .. })
    ));
    assert!(matches!(
        classify("git fetch; git switch -c feat/y origin/main"),
        Some(Trigger::CreateBranch { .. })
    ));
    assert_eq!(
        segments("a 'b c' | d \"e\\\"f\" || g\nh"),
        vec![
            vec!["a".to_string(), "b c".into()],
            vec!["d".into(), "e\"f".into()],
            vec!["g".into()],
            vec!["h".into()],
        ]
    );
}

#[test]
fn a_quoted_mention_is_not_a_command() {
    assert_eq!(classify("git commit -m 'run gh pr create later'"), None);
    assert_eq!(classify("echo \"git switch -c x\""), None);
}

#[test]
fn remote_urls_resolve_to_owner_and_repo() {
    for url in [
        "git@github.com:Acme/Widgets.git",
        "https://github.com/acme/widgets",
        "https://github.com/acme/widgets.git\n",
        "ssh://git@github.com/acme/widgets.git",
    ] {
        assert_eq!(github_repo(url).as_deref(), Some("acme/widgets"), "{url}");
    }
    assert_eq!(github_repo("https://gitlab.com/acme/widgets"), None);
    assert_eq!(
        pull_urls("ok created #3 https://github.com/A/W/pull/3"),
        vec![("a/w".into(), 3)]
    );
}

#[test]
fn the_reason_carries_the_way_out() {
    let open = OpenPr {
        number: 12,
        title: "fix it".into(),
        head: "feat/x".into(),
    };
    let pr = reason(
        "acme/widgets",
        &open,
        &Trigger::CreatePr {
            repo: None,
            head: None,
            dry_run: false,
        },
    );
    assert!(pr.contains("the user's own words"), "{pr}");
    assert!(pr.contains("prefixing the command with `!`"), "{pr}");
    let branch = reason(
        "acme/widgets",
        &open,
        &Trigger::CreateBranch {
            name: "feat/y".into(),
            dir: None,
            force: false,
        },
    );
    assert!(branch.contains("once #12 merges"), "{branch}");
    assert!(branch.contains("!git switch -c feat/y"), "{branch}");
}

#[test]
fn the_deny_text_never_carries_the_cwd_or_transcript_path() {
    let dir = scratch("no-leak");
    let ledger = ledger_in(&dir);
    record(&ledger, "acme/widgets", "feat/x");
    let transcript = dir.join("rollout.jsonl");
    std::fs::write(&transcript, "").unwrap();
    let mut p = with_transcript("git switch -c feat/y", &transcript);
    p["cwd"] = json!(dir.to_str().unwrap());
    let fake = Fake::new(vec![("acme/widgets", 12, "feat/x")]);
    let why = decide_with(&p, &fake, &ledger).expect("denied");
    assert!(!why.contains(dir.to_str().unwrap()), "{why}");
}

#[test]
fn resetting_a_branch_that_exists_is_not_a_creation() {
    let dir = scratch("reset");
    let ledger = ledger_in(&dir);
    record(&ledger, "acme/widgets", "feat/x");
    let fake = Fake::new(vec![("acme/widgets", 12, "feat/x")]);
    for command in [
        "git branch -f main origin/main",
        "git branch --force main origin/main",
        "git switch -C main origin/main",
        "git checkout -B feat/x origin/feat/x",
        "git branch -dr origin/old",
    ] {
        assert!(
            decide_with(&payload(command), &fake, &ledger).is_none(),
            "{command} should be allowed"
        );
    }
    for command in ["git branch -f feat/new", "git switch -C feat/new"] {
        assert!(
            decide_with(&payload(command), &fake, &ledger).is_some(),
            "{command} creates a branch and should be denied"
        );
    }
}

#[test]
fn a_heredoc_body_is_data() {
    let dir = scratch("heredoc");
    let ledger = ledger_in(&dir);
    let body = "cat > /tmp/pr.md <<'EOF'\ngit switch -c feat/y\ngh pr create --fill\nEOF";
    assert!(classify_all(body).is_empty(), "{:?}", classify_all(body));
    assert!(decide_with(&payload(body), &Fake::new(vec![]), &ledger).is_none());
    assert!(entries(&ledger).is_empty(), "nothing is recorded");

    record(&ledger, "acme/widgets", "feat/x");
    let open = Fake::new(vec![("acme/widgets", 12, "feat/x")]);
    assert!(decide_with(&payload(body), &open, &ledger).is_none());
    let after = "cat <<-EOF\n\tgh pr create\n\tEOF\ngit switch -c feat/y";
    assert!(decide_with(&payload(after), &open, &ledger).is_some());
    assert_eq!(
        strip_heredoc_bodies("a <<<word\nb <<X\nbody\nX\nc"),
        "a <<<word\nb <<X\nc"
    );
}

#[test]
fn a_dry_run_or_web_create_is_not_recorded() {
    let dir = scratch("dry-run");
    let ledger = ledger_in(&dir);
    let fake = Fake::new(vec![]);
    for command in [
        "gh pr create --dry-run",
        "gh pr create --web",
        "gh pr create -w",
    ] {
        assert!(decide_with(&payload(command), &fake, &ledger).is_none());
    }
    assert!(entries(&ledger).is_empty());
}

#[test]
fn a_pr_already_open_on_the_head_is_not_the_sessions() {
    let dir = scratch("preexisting");
    let ledger = ledger_in(&dir);
    let fake = Fake::new(vec![("acme/widgets", 7, "feat/x")]);
    assert!(decide_with(&payload("gh pr create --fill"), &fake, &ledger).is_none());
    assert_eq!(entries(&ledger)[0].preexisting, Some(7));
    assert!(
        decide_with(&payload("gh pr create --fill"), &fake, &ledger).is_none(),
        "#7 was open before this session's create"
    );
}

#[test]
fn a_pr_after_a_new_branch_in_one_command_records_that_branch() {
    let dir = scratch("branch-then-pr");
    let ledger = ledger_in(&dir);
    let fake = Fake::new(vec![]);
    let command = "git switch -c feat/new\ngh pr create --fill";
    assert!(decide_with(&payload(command), &fake, &ledger).is_none());
    assert_eq!(entries(&ledger), vec![entry("acme/widgets", "feat/new")]);
}

#[test]
fn repo_and_head_flags_are_normalised() {
    let dir = scratch("normalise");
    let ledger = ledger_in(&dir);
    record(&ledger, "acme/widgets", "feat/x");
    let fake = Fake {
        origin: Some("acme/unrelated"),
        ..Fake::new(vec![("acme/widgets", 12, "feat/x")])
    };
    for command in [
        "gh pr create -R github.com/Acme/Widgets",
        "gh pr create -R https://github.com/acme/widgets.git",
        "gh pr create -Racme/widgets",
    ] {
        assert!(
            decide_with(&payload(command), &fake, &ledger).is_some(),
            "{command}"
        );
    }
    assert_eq!(normalize_repo("acme"), None);
    assert!(matches!(
        classify("gh pr create -Hfeat/y"),
        Some(Trigger::CreatePr { head: Some(h), .. }) if h == "feat/y"
    ));
}

#[test]
fn the_transcript_counts_only_a_command_that_creates() {
    let search = json!({"cmd": "grep -rn \"gh pr create\" docs"});
    let exists = "a pull request for branch \"feat/x\" into branch \"main\" already exists:\nhttps://github.com/acme/widgets/pull/4";
    let body = [
        rollout(json!({"type": "function_call", "name": "exec_command", "arguments": search.to_string(), "call_id": "g1"})),
        rollout(json!({"type": "function_call_output", "call_id": "g1",
            "output": "docs/x.md: gh pr create https://github.com/acme/widgets/pull/9"})),
        code_mode_create("c1", exists),
    ]
    .concat();
    assert!(transcript_prs(Cursor::new(body), deadline()).is_empty());
}

#[test]
fn call_commands_reads_each_recorded_shape() {
    assert_eq!(
        call_commands(r#"{"cmd":"gh pr create --fill"}"#, ""),
        vec!["gh pr create --fill".to_string()]
    );
    assert_eq!(
        call_commands(r#"{"command":["/bin/bash","-lc","gh pr new"]}"#, ""),
        vec!["gh pr new".to_string()]
    );
    assert_eq!(
        call_commands(
            "",
            r#"text(await tools.exec_command({cmd:"gh pr create --title \"a b\"", workdir:"/w"}))"#
        ),
        vec!["gh pr create --title \"a b\"".to_string()]
    );
}

#[test]
fn a_hung_child_is_killed_at_the_deadline() {
    // test-hang-allow: a real `sleep` child, bounded by the 300 ms deadline under test.
    let started = Instant::now();
    let out = run_bounded(
        Command::new("sleep").arg("30"),
        Instant::now() + Duration::from_millis(300),
    );
    assert!(out.is_none());
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "took {:?}",
        started.elapsed()
    );
}

#[test]
fn a_finished_child_returns_its_stdout() {
    let out = run_bounded(
        Command::new("echo").arg("hello"),
        Instant::now() + Duration::from_secs(5),
    );
    assert_eq!(out.as_deref(), Some("hello\n"));
}

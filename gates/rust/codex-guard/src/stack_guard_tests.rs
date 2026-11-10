//! What the stack guard must deny, and everything it must leave alone.

use super::{classify_all, decide, Lookup, Pr, Selector, Trigger};
use serde_json::json;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const REPO: &str = "acme/app";

fn pr(number: u64, base: &str, head: &str, state: &str, oid: &str) -> Pr {
    Pr {
        number,
        base: base.into(),
        head: head.into(),
        state: state.into(),
        head_oid: oid.into(),
    }
}

#[derive(Default)]
struct Fake {
    branch: Option<&'static str>,
    prs: Vec<Pr>,
    /// `(ancestor, descendant)` pairs that hold; a pair naming `missing` is unknown.
    ancestry: Vec<(&'static str, &'static str)>,
    missing: Option<&'static str>,
    upstream_dupes: usize,
    own: HashMap<&'static str, u64>,
    /// Commits per author on any `since..tip` range.
    authors: Vec<(&'static str, usize)>,
    gate: Option<&'static str>,
    status: Option<&'static str>,
}

impl Lookup for Fake {
    fn origin_repo(&self, _dir: &str, _deadline: Instant) -> Option<String> {
        Some(REPO.into())
    }
    fn current_branch(&self, _dir: &str, _deadline: Instant) -> Option<String> {
        self.branch.map(str::to_string)
    }
    fn default_branch(&self, _dir: &str, _repo: &str, _deadline: Instant) -> Option<String> {
        Some("main".into())
    }
    fn pr_by_head(&self, _repo: &str, head: &str, _deadline: Instant) -> Option<Pr> {
        let matching: Vec<&Pr> = self.prs.iter().filter(|p| p.head == head).collect();
        matching
            .iter()
            .find(|p| p.state == "OPEN")
            .or_else(|| matching.first())
            .map(|p| (*p).clone())
    }
    fn pr_by_number(&self, _repo: &str, number: u64, _deadline: Instant) -> Option<Pr> {
        self.prs.iter().find(|p| p.number == number).cloned()
    }
    fn is_ancestor(&self, _dir: &str, a: &str, d: &str, _deadline: Instant) -> Option<bool> {
        if self.missing.is_some_and(|m| m == a || m == d) {
            return None;
        }
        Some(self.ancestry.contains(&(a, d)))
    }
    fn already_upstream(&self, _dir: &str, _up: &str, _tip: &str, _d: Instant) -> Option<usize> {
        Some(self.upstream_dupes)
    }
    fn own_commits_since(&self, _dir: &str, since: &str, _deadline: Instant) -> Option<u64> {
        Some(self.own.get(since).copied().unwrap_or(0))
    }
    fn author_counts(
        &self,
        _dir: &str,
        _since: &str,
        _tip: &str,
        _d: Instant,
    ) -> Option<Vec<(String, usize)>> {
        Some(
            self.authors
                .iter()
                .map(|(a, n)| ((*a).into(), *n))
                .collect(),
        )
    }
    fn has_open_child(&self, _repo: &str, head: &str, _d: Instant) -> Option<bool> {
        Some(self.prs.iter().any(|p| p.base == head && p.state == "OPEN"))
    }
    fn gate_context(&self, _dir: &str, _deadline: Instant) -> Option<String> {
        self.gate.map(str::to_string)
    }
    fn status(&self, _r: &str, _sha: &str, _c: &str, _d: Instant) -> Option<Option<String>> {
        Some(self.status.map(str::to_string))
    }
}

fn run(command: &str, fake: &Fake) -> Option<String> {
    let payload = json!({"tool_name": "Bash", "tool_input": {"command": command}, "cwd": "/work"});
    decide(&payload, fake, Instant::now() + Duration::from_secs(5))
}

/// #10 `feat/a` on main, #11 `feat/b` stacked on it.
fn two_layer_stack() -> Vec<Pr> {
    vec![
        pr(10, "main", "feat/a", "OPEN", "aaaaaaaaaa"),
        pr(11, "feat/a", "feat/b", "OPEN", "bbbbbbbbbb"),
    ]
}

#[test]
fn classifies_the_stack_commands_and_nothing_else() {
    assert_eq!(
        classify_all("gh pr create -B feat/a --draft --title 'x -B y' --fill"),
        vec![Trigger::CreatePr {
            repo: None,
            base: Some("feat/a".into()),
            head: None,
            draft: true,
            dry_run: false,
        }]
    );
    assert_eq!(
        classify_all("rtk proxy gh pr merge https://github.com/acme/app/pull/42 --rebase"),
        vec![Trigger::Merge {
            repo: None,
            pr: Selector::Number(42),
            match_head: None,
        }]
    );
    assert_eq!(
        classify_all("git -C ../wt push -u origin HEAD:feat/b"),
        vec![Trigger::Push {
            dir: Some("../wt".into()),
            src: "HEAD".into(),
            branch: Some("feat/b".into()),
        }]
    );
    assert!(classify_all("git commit --amend --no-edit").is_empty());
    assert!(classify_all("git push origin --delete feat/b").is_empty());
    assert!(classify_all("gh pr ready 11 --undo").is_empty());
    assert!(classify_all("gh pr list --base feat/a").is_empty());
    assert!(classify_all("echo gh pr merge 1").is_empty());
}

#[test]
fn create_onto_default_branch_is_untouched() {
    let fake = Fake::default();
    assert_eq!(run("gh pr create --base main --title t", &fake), None);
    assert_eq!(run("gh pr create --title t", &fake), None);
}

#[test]
fn stacked_create_must_be_draft() {
    let fake = Fake {
        prs: two_layer_stack(),
        ancestry: vec![("aaaaaaaaaa", "HEAD")],
        ..Fake::default()
    };
    let reason = run("gh pr create --base feat/a --title t", &fake).unwrap();
    assert!(reason.contains("--draft"), "{reason}");
    assert_eq!(
        run("gh pr create --base feat/a --draft --title t", &fake),
        None
    );
}

#[test]
fn stacked_create_needs_an_open_parent() {
    let fake = Fake::default();
    let reason = run("gh pr create --base feat/nobody --draft", &fake).unwrap();
    assert!(reason.contains("no open PR"), "{reason}");
}

#[test]
fn stacked_create_must_contain_the_parent_head() {
    let fake = Fake {
        prs: two_layer_stack(),
        ..Fake::default()
    };
    let reason = run("gh pr create --base feat/a --draft", &fake).unwrap();
    assert!(reason.contains("parent moved"), "{reason}");
}

#[test]
fn ready_is_denied_until_the_base_is_main() {
    let fake = Fake {
        branch: Some("feat/b"),
        prs: two_layer_stack(),
        ..Fake::default()
    };
    assert!(run("gh pr ready 11", &fake)
        .unwrap()
        .contains("based on feat/a"));
    assert!(run("gh pr ready", &fake).unwrap().contains("#11"));
    assert_eq!(run("gh pr ready 10", &fake), None);
}

#[test]
fn merge_lands_bottom_up() {
    let fake = Fake {
        prs: two_layer_stack(),
        ..Fake::default()
    };
    let reason = run("gh pr merge 11 --rebase", &fake).unwrap();
    assert!(reason.contains("stacked on feat/a (#10)"), "{reason}");
    assert_eq!(run("gh pr merge 10 --rebase", &fake), None);
}

#[test]
fn a_stack_root_with_two_commits_by_one_author_is_not_ready_or_mergeable() {
    let mut fake = Fake {
        prs: two_layer_stack(),
        authors: vec![("amelian", 3), ("decebal", 1)],
        ..Fake::default()
    };
    let reason = run("gh pr ready 10", &fake).unwrap();
    assert!(reason.contains("has 3 commits by amelian;"), "{reason}");
    assert!(!reason.contains("decebal"), "{reason}");
    assert!(run("gh pr merge 10 --rebase", &fake)
        .unwrap()
        .contains("before it is merged"));

    fake.authors = vec![("amelian", 1), ("decebal", 1)];
    assert_eq!(run("gh pr ready 10", &fake), None);
}

#[test]
fn a_crowded_pr_outside_any_stack_is_left_to_other_checks() {
    let fake = Fake {
        prs: vec![pr(12, "main", "fix/solo", "OPEN", "cccccccccc")],
        authors: vec![("amelian", 5)],
        ..Fake::default()
    };
    assert_eq!(run("gh pr ready 12", &fake), None);
    assert_eq!(run("gh pr merge 12 --rebase", &fake), None);
}

#[test]
fn merge_with_a_gate_needs_the_gated_head_at_success() {
    let mut fake = Fake {
        prs: two_layer_stack(),
        gate: Some("longhand/gates"),
        ..Fake::default()
    };
    let reason = run("gh pr merge 10 --rebase", &fake).unwrap();
    assert!(
        reason.contains("--match-head-commit aaaaaaaaaa"),
        "{reason}"
    );

    let merge = "gh pr merge 10 --rebase --match-head-commit aaaaaaaaaa";
    assert!(run(merge, &fake).unwrap().contains("= absent"));
    fake.status = Some("failure");
    assert!(run(merge, &fake).unwrap().contains("= failure"));
    fake.status = Some("success");
    assert_eq!(run(merge, &fake), None);
}

#[test]
fn push_with_commits_already_on_main_is_denied() {
    let fake = Fake {
        branch: Some("feat/b"),
        upstream_dupes: 2,
        ..Fake::default()
    };
    let reason = run("git push", &fake).unwrap();
    assert!(reason.contains("2 commit(s)"), "{reason}");
    assert!(reason.contains("git rebase origin/main"), "{reason}");
}

#[test]
fn push_still_carrying_a_merged_parent_is_denied() {
    let fake = Fake {
        branch: Some("feat/b"),
        prs: vec![
            pr(10, "main", "feat/a", "MERGED", "aaaaaaaaaa"),
            pr(11, "feat/a", "feat/b", "OPEN", "bbbbbbbbbb"),
        ],
        ancestry: vec![("aaaaaaaaaa", "HEAD")],
        ..Fake::default()
    };
    let reason = run("git push", &fake).unwrap();
    assert!(
        reason.contains("git rebase --onto origin/main aaaaaaaaaa HEAD"),
        "{reason}"
    );
    assert!(reason.contains("gh pr edit 11 --base main"), "{reason}");
}

#[test]
fn restacked_push_after_parent_merged_is_allowed() {
    let fake = Fake {
        branch: Some("feat/b"),
        prs: vec![
            pr(10, "main", "feat/a", "MERGED", "aaaaaaaaaa"),
            pr(11, "feat/a", "feat/b", "OPEN", "bbbbbbbbbb"),
        ],
        ..Fake::default()
    };
    assert_eq!(run("git push --force-with-lease", &fake), None);
}

#[test]
fn push_after_parent_moved_is_denied() {
    let fake = Fake {
        branch: Some("feat/b"),
        prs: two_layer_stack(),
        ..Fake::default()
    };
    let reason = run("git push origin feat/b", &fake).unwrap();
    assert!(reason.contains("the parent moved"), "{reason}");
}

#[test]
fn push_of_a_current_stack_layer_is_allowed() {
    let fake = Fake {
        branch: Some("feat/b"),
        prs: two_layer_stack(),
        ancestry: vec![("aaaaaaaaaa", "HEAD")],
        ..Fake::default()
    };
    assert_eq!(run("git push", &fake), None);
}

#[test]
fn unknown_ancestry_fails_open() {
    let fake = Fake {
        branch: Some("feat/b"),
        prs: two_layer_stack(),
        missing: Some("aaaaaaaaaa"),
        ..Fake::default()
    };
    assert_eq!(run("git push", &fake), None);
    assert_eq!(run("gh pr create --base feat/a --draft", &fake), None);
}

#[test]
fn second_own_commit_on_a_stack_layer_is_denied() {
    let mut fake = Fake {
        branch: Some("feat/b"),
        prs: two_layer_stack(),
        own: HashMap::from([("origin/main", 3), ("aaaaaaaaaa", 1)]),
        ..Fake::default()
    };
    let reason = run("git commit -m 'more'", &fake).unwrap();
    assert!(reason.contains("--amend"), "{reason}");
    assert_eq!(run("git commit --amend --no-edit", &fake), None);

    fake.own = HashMap::from([("origin/main", 3), ("aaaaaaaaaa", 0)]);
    assert_eq!(run("git commit -m 'first on this layer'", &fake), None);
}

#[test]
fn commits_on_unstacked_branches_are_untouched() {
    let fake = Fake {
        branch: Some("feat/a"),
        prs: two_layer_stack(),
        own: HashMap::from([("origin/main", 4)]),
        ..Fake::default()
    };
    assert_eq!(run("git commit -m 'x'", &fake), None);
}

#[test]
fn other_tools_are_ignored() {
    let payload = json!({"tool_name": "Write", "tool_input": {"command": "gh pr merge 11"}});
    assert_eq!(
        decide(
            &payload,
            &Fake::default(),
            Instant::now() + Duration::from_secs(1)
        ),
        None
    );
}

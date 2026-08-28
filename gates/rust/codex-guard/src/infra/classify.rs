//! The deny/ask tiers. THE single source of the patterns — the hook and the
//! trust helper both read them from here, so a rule cannot exist in one and not
//! the other.
//!
//! Order is the policy: the first match wins, and every deny tier precedes every
//! ask tier. Written as straight-line early returns rather than a table, because
//! two of the rules need a second condition and a table would have to smuggle
//! them in past its own shape.

use regex::Regex;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Deny(String),
    Ask(String),
}

/// An infra tool naming itself. A risk signal, never a verdict on its own.
pub const INFRA_TOOL_RE: &str = r"gcloud|gsutil|terraform|kubectl|helm|(^| )aws |(^| )az ";

/// Words that mean a command CHANGES something. Gates the risk signal, so a
/// read-only mention of an infra tool never prompts.
pub const MUTATE_RE: &str = r"(update|delete|deploy|destroy|create| apply|publish|unpublish|deprecate|set-iam|add-iam|remove-iam|--force|prune|rmi| push| cp | rm | mv )";

/// Read-only forms, which cancel the risk signal even when a mutating word
/// appears elsewhere on the line.
pub const READONLY_RE: &str = r"(list|describe|get-value|get-iam-policy|auth login|auth list|logging read| plan| validate| fmt| output|--help|--version|--dry-run)";

/// The hook's own mutation test, which is NOT [`MUTATE_RE`]: this one gates the
/// prod-token and breadth rules, where ` push`/` cp `/` mv ` are too broad and a
/// hard reset is worth catching.
pub const HOOK_MUTATE_RE: &str = r"(update|delete|deploy|destroy|create| apply|publish|unpublish|deprecate|set-iam|add-iam|remove-iam|--force|prune|rmi|reset +--hard|rm +-[a-z]*r)";

/// Compile once per pattern, for the life of the process.
pub fn cached(pattern: &str) -> Regex {
    static CACHE: OnceLock<Mutex<HashMap<String, Regex>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = cache.lock().expect("regex cache");
    map.entry(pattern.to_string())
        .or_insert_with(|| Regex::new(pattern).expect("static pattern compiles"))
        .clone()
}

/// A surface prefix or a global flag sits between the tool and its command
/// group, so `gcloud alpha run deploy` and `gcloud --project X run deploy` reach
/// none of the `gcloud +<group>` rules unless they are folded out first.
pub fn fold_tool_prefixes(text: &str) -> String {
    let mut t = cached(r"(gcloud|gsutil) +(alpha|beta) +")
        .replace_all(text, "$1 ")
        .into_owned();
    let steps: [(&str, &str); 3] = [
        (r"(gcloud|gsutil) +--[a-z][a-z0-9-]*=[^ ]* +", "$1 "),
        (
            r"(gcloud|gsutil) +--(project|account|billing-project|configuration|impersonate-service-account|verbosity|format) +[^ -][^ ]* +",
            "$1 ",
        ),
        (
            r"(gcloud|gsutil) +--[a-z][a-z0-9-]* +(alpha|beta|run|storage|artifacts|iam|container|compute|functions|sql|projects) +",
            "$1 $2 ",
        ),
    ];
    for _ in 0..5 {
        let mut next = t.clone();
        for (pat, rep) in &steps {
            next = cached(pat).replace_all(&next, *rep).into_owned();
        }
        if next == t {
            break;
        }
        t = next;
    }
    t
}

/// Classify one lowercased command. `None` means no rule fired.
pub fn classify(text: &str) -> Option<Verdict> {
    let t = fold_tool_prefixes(text);
    let m = |p: &str| cached(p).is_match(&t);
    let deny = |r: &str| Some(Verdict::Deny(r.to_string()));
    let ask = |r: &str| Some(Verdict::Ask(r.to_string()));

    // --- deny ---------------------------------------------------------------
    if m(r"gcloud +run +(services|jobs) +(update|delete|replace)") {
        return deny("Cloud Run live-service mutation — never run direct; propose it for manual/CI execution");
    }
    if m(r"gcloud +run +deploy") {
        return deny(
            "gcloud run deploy mutates a live service — propose it (deploys are CI-only here)",
        );
    }
    if m(r"(add-iam-policy-binding|remove-iam-policy-binding|set-iam-policy)") {
        return deny("IAM change on a live resource — propose it for manual/CI run");
    }
    if m(r"terraform +(-chdir=[^ ]+ +)?(apply|destroy)") {
        return deny(
            "terraform apply/destroy mutates real infra — propose it (terraform plan is fine)",
        );
    }
    if m(r"terraform +(-chdir=[^ ]+ +)?state +(rm|mv|push)") {
        return deny("terraform state surgery can corrupt shared state — propose it");
    }
    if m(r"gcloud +storage +rm") {
        return deny("deleting GCS objects (possible prod CDN assets) — propose it");
    }
    if m(r"gsutil +rm") {
        return deny("gsutil rm deletes storage objects — propose it");
    }
    if m(r"gcloud +artifacts +.*delete") {
        return deny("deleting an artifact/version from GAR — propose it");
    }
    if m(r"helm +uninstall") {
        return deny("helm uninstall tears down a release — propose it");
    }
    if m(r"kubectl +delete +(namespace|ns|pvc|persistentvolumeclaim)") {
        return deny(
            "deleting a namespace or PVC destroys live workloads and their data — propose it",
        );
    }
    if m(r"(npm|bun|yarn|pnpm) +publish") && !m(r"--dry-run") {
        return deny("direct package publish — use a make target / CI, or propose it");
    }
    if m(r"(npm|bun|yarn|pnpm) +(unpublish|deprecate)") {
        return deny(
            "npm unpublish/deprecate breaks downstream consumers irreversibly — propose it",
        );
    }
    // Three conditions, because a force-push is only catastrophic on a shared
    // branch: a forced push to your own feature branch is routine.
    if m(r"git +push")
        && m(r"(\-\-force|\-\-force-with-lease| -f( |$))")
        && m(r"(develop|main|master|rc/)")
    {
        return deny("force-push to a protected branch rewrites shared history");
    }

    // --- ask ----------------------------------------------------------------
    if m(r"(make +[^&|]*publish-prod|publish:prod|publish-prod\.sh)") {
        return ask("local prod publish (CI-only here, no laptop fallback) — confirm you mean it");
    }
    if m(r"kubectl +(delete|drain|cordon|taint|scale)") {
        return ask("kubectl mutates live cluster state — confirm the target and context");
    }
    if m(r"kubectl +(apply|replace|patch|rollout +undo)") {
        return ask(
            "kubectl writes to the cluster the current context points at — confirm the context",
        );
    }
    if m(r"helm +(upgrade|rollback)") {
        return ask("helm upgrade/rollback replaces a live release — confirm");
    }
    if m(r"(^| )pkill( |$)") {
        return ask("pkill matches by name — easy to hit the wrong process; confirm the match");
    }
    if m(r"(^| )killall( |$)") {
        return ask("killall by name — confirm the target");
    }
    if m(r"kill +-(9|kill|s +9|s +kill)") {
        return ask("kill -9/-KILL is ungraceful — confirm the PID/target");
    }
    if m(r"docker +system +prune") {
        return ask("docker system prune wipes images/cache/volumes — confirm");
    }
    if m(r"(docker|podman) +(rmi|image +rm|volume +rm)") {
        return ask("removing container images/volumes — confirm");
    }
    if m(r"rm +-[a-z]+ +/( |$)") {
        return ask("rm -rf targeting / — catastrophic; confirm");
    }
    if m(r"rm +-[a-z]+ +/\*") {
        return ask("rm -rf /* wipes the filesystem root; confirm");
    }
    if m(r"rm +-[a-z]+ +~( |$)") {
        return ask("rm -rf on bare home (~); confirm");
    }
    if m(r"rm +-[a-z]+ +\$home( |$)") {
        return ask("rm -rf on $HOME; confirm");
    }
    // A `..` target leaves the tree the command was issued from — build output
    // never lives there, so the odds it is a mistake are high.
    if m(r"rm +-[a-z]*r[a-z]* +[^ ]*\.\.") {
        return ask("rm -r reaches outside the current tree (..); confirm the target");
    }
    if m(r"(^| )sudo ") {
        return ask("sudo — elevated, broad blast radius on a managed machine; confirm");
    }
    if m(r"launchctl +(bootout|unload|disable|kill)") {
        return ask("launchctl teardown can stop system services; confirm");
    }
    if m(r"gcloud +config +set +project") {
        return ask("switching gcloud project silently re-aims every later command; confirm");
    }
    None
}

/// Does this text mutate anything, by the HOOK's definition?
pub fn is_mutating(text: &str) -> bool {
    cached(HOOK_MUTATE_RE).is_match(text)
}

/// A mutation with `--all` / `-a`: wide blast radius, worth a prompt.
pub fn is_broad(text: &str) -> bool {
    cached(r"(\-\-all( |$)| -a( |$))").is_match(text)
}

#[cfg(test)]
#[path = "classify_tests.rs"]
mod classify_tests;

//! The infra-guard decision matrix, ported case-for-case from the shell version
//! it replaces, run against a throwaway repo on disk (sibling file per
//! rules/testing-gates.md).
//!
//! These build a real tree because the deep layer's whole job is resolving real
//! files: a mocked filesystem would test the mock's idea of `make -C`, which is
//! the part most likely to be wrong.

use super::*;
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A lab repo, unique per test so cases never share state.
struct Lab {
    root: PathBuf,
}

impl Lab {
    fn new() -> Lab {
        static N: AtomicUsize = AtomicUsize::new(0);
        let id = N.fetch_add(1, Ordering::SeqCst);
        let root =
            std::env::temp_dir().join(format!("codex-guard-lab-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for d in ["scripts", "sub", "apps", ".codex"] {
            std::fs::create_dir_all(root.join(d)).expect("lab dirs");
        }
        let lab = Lab { root };

        lab.write(
            ".codex/prod-guard-tokens.txt",
            "prod:my-company-prod\nnonprod:my-company-staging\n",
        );
        lab.write(
            "Makefile",
            "deploy-cdn:\n\tbash scripts/deploy.sh\n\nlogs:\n\tbash scripts/logs.sh\n\nbuild:\n\techo building the bundle\n\ndeploy-inline:\n\tgcloud run deploy web-cdn --source .\n",
        );
        lab.write(
            "sub/Makefile",
            "deploy-sub:\n\tgcloud run deploy web-cdn --source .\n\nbuild:\n\techo building the sub bundle\n",
        );
        lab.write(
            "apps/package.json",
            "{\"scripts\": {\"release\": \"npm publish --access public\"}}",
        );
        lab.write(
            "scripts/deploy.sh",
            "#!/usr/bin/env bash\ngcloud run services update web-cdn --region europe-west2\n",
        );
        // The comment is load-bearing: it proves comment lines are stripped
        // before classification, so a command named only in prose cannot decide
        // anything.
        lab.write(
            "scripts/logs.sh",
            "#!/usr/bin/env bash\n# Reads logs. gcloud run deploy appears in this comment only.\ngcloud logging read 'resource.type=\"cloud_run_revision\"' --project my-company-prod --limit 10\n",
        );
        lab.write(
            "scripts/nested.sh",
            "#!/usr/bin/env bash\ncd sub && bash inner.sh\n",
        );
        lab.write(
            "sub/inner.sh",
            "#!/usr/bin/env bash\ngcloud run deploy web-cdn --source .\n",
        );
        lab
    }

    fn write(&self, rel: &str, body: &str) {
        let p = self.root.join(rel);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).expect("parent");
        }
        std::fs::write(p, body).expect("write");
    }

    fn decide(&self, cmd: &str) -> Decision {
        self.decide_at_depth(cmd, None)
    }

    fn decide_at_depth(&self, cmd: &str, depth: Option<&str>) -> Decision {
        // The guard reads its depth from the environment, which is
        // process-global, so cases needing a non-default depth are
        // serialized behind one lock rather than run in parallel.
        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::remove_var("INFRA_GUARD_OFF");
        std::env::set_var("INFRA_GUARD_KOSHER", self.root.join("kosher.txt"));
        match depth {
            Some(d) => std::env::set_var("INFRA_GUARD_DEPTH", d),
            None => std::env::remove_var("INFRA_GUARD_DEPTH"),
        }
        let payload = json!({"tool_input": {"command": cmd}, "cwd": self.root.to_string_lossy()});
        decide(&payload)
    }
}

impl Drop for Lab {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn assert_deny(d: Decision, case: &str) {
    assert!(
        matches!(d, Decision::Deny(_)),
        "{case}: expected deny, got {d:?}"
    );
}
fn assert_ask(d: Decision, case: &str) {
    assert!(
        matches!(d, Decision::Ask(_)),
        "{case}: expected ask, got {d:?}"
    );
}
fn assert_silent(d: Decision, case: &str) {
    assert_eq!(d, Decision::Allow, "{case}");
}

#[test]
fn literal_layer() {
    let lab = Lab::new();
    assert_deny(lab.decide("gcloud run deploy web-cdn"), "gcloud run deploy");
    assert_deny(
        lab.decide("gcloud run jobs create sync --project my-company-prod --set-env-vars T=my-company-staging"),
        "mixed prod + non-prod",
    );
    assert_silent(
        lab.decide("git commit -m 'block gcloud run deploy'"),
        "dangerous phrase inside a string",
    );
}

#[test]
fn deep_layer_follows_wrappers() {
    let lab = Lab::new();
    assert_deny(
        lab.decide("make deploy-cdn"),
        "make target wrapping a deploy",
    );
    assert_deny(
        lab.decide("bash scripts/nested.sh"),
        "cd-prefixed nested script",
    );
    assert_silent(
        lab.decide("make logs"),
        "read-only gcloud with a prod token",
    );
    assert_silent(lab.decide("make build"), "ordinary build target");
}

#[test]
fn the_depth_dial() {
    let lab = Lab::new();
    assert_silent(
        lab.decide_at_depth("make deploy-cdn", Some("0")),
        "depth 0 disables the deep layer",
    );
    assert_deny(
        lab.decide_at_depth("gcloud run deploy web-cdn", Some("0")),
        "depth 0 keeps the literal layer",
    );
    assert_deny(
        lab.decide_at_depth("make deploy-inline", Some("1")),
        "depth 1 catches an inlined recipe",
    );
    assert_deny(
        lab.decide_at_depth("make deploy-cdn", Some("2")),
        "depth 2 reaches the recipe's script",
    );
    assert_deny(
        lab.decide_at_depth("make deploy-cdn", Some("abc")),
        "non-numeric depth falls back to the default",
    );
}

#[test]
fn resolution_forms_a_cd_averse_convention_pushes_you_toward() {
    let lab = Lab::new();
    assert_deny(
        lab.decide("make -C sub deploy-sub"),
        "make -C resolves the subdir Makefile",
    );
    assert_deny(
        lab.decide("make -C . deploy-inline"),
        "make -C . resolves the root Makefile",
    );
    assert_silent(
        lab.decide("make -C sub build"),
        "make -C stays quiet on a build target",
    );
    assert_deny(
        lab.decide("bun --cwd apps run release"),
        "bun --cwd resolves the package script",
    );
    assert_deny(lab.decide("./scripts/deploy.sh"), "./script.sh is scanned");
    assert_deny(
        lab.decide("scripts/deploy.sh"),
        "relative path with no dot is scanned",
    );
}

#[test]
fn gcloud_surface_prefixes_and_global_flags() {
    let lab = Lab::new();
    assert_deny(lab.decide("gcloud alpha run deploy web-cdn"), "alpha");
    assert_deny(
        lab.decide("gcloud beta run deploy web-cdn --source ."),
        "beta",
    );
    assert_deny(
        lab.decide("gcloud --project foo run deploy web-cdn"),
        "--project X",
    );
    assert_deny(
        lab.decide("gcloud --project=foo run deploy web-cdn"),
        "--project=X",
    );
}

#[test]
fn kubernetes_and_terraform() {
    let lab = Lab::new();
    assert_deny(
        lab.decide("kubectl delete namespace staging"),
        "delete namespace",
    );
    assert_deny(lab.decide("helm uninstall web -n prod"), "helm uninstall");
    assert_ask(
        lab.decide("kubectl delete deployment api -n default"),
        "delete deployment",
    );
    assert_ask(
        lab.decide("kubectl apply -f manifest.yaml"),
        "kubectl apply",
    );
    assert_silent(lab.decide("kubectl get pods -n default"), "kubectl get");
    assert_deny(lab.decide("terraform apply"), "terraform apply");
    assert_deny(
        lab.decide("terraform -chdir=infra/dev apply -auto-approve"),
        "-chdir apply",
    );
    assert_deny(
        lab.decide("terraform -chdir=infra/dev state rm aws_s3_bucket.x"),
        "state rm",
    );
    assert_silent(lab.decide("terraform -chdir=infra/dev plan"), "-chdir plan");
}

#[test]
fn destructive_local_commands_and_the_git_early_exit() {
    let lab = Lab::new();
    assert_deny(lab.decide("npm publish"), "npm publish");
    assert_ask(lab.decide("rm -rf ../some-repo"), "rm -r outside the tree");
    assert_deny(
        lab.decide("git commit -m x ; gcloud run deploy cdn"),
        "a git prefix does not cover a separator",
    );
    assert_silent(
        lab.decide("git commit -m 'ship it'"),
        "plain git commit exits early",
    );
}

#[test]
fn a_vetted_chain_stops_prompting_until_it_changes() {
    let lab = Lab::new();
    let cmd = "make deploy-cdn";
    assert_deny(lab.decide(cmd), "unvetted");

    let scan = resolve::scan_tree(cmd, &lab.root, &lab.root, &resolve::Limits::default(), None);
    let hash = resolve::tree_hash(&scan.units).expect("units");
    lab.write("kosher.txt", &format!("{hash}  {cmd}\n"));
    assert_silent(lab.decide(cmd), "vetted");

    // Editing the recipe busts it — trust never outlives what it was granted for.
    lab.write(
        "Makefile",
        "deploy-cdn:\n\tbash scripts/deploy.sh --now\n\nbuild:\n\techo building\n",
    );
    assert_deny(lab.decide(cmd), "recipe edited");
}

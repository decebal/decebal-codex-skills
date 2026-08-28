//! Sibling test file (rules/testing-gates.md — no inline test modules).

use super::*;

fn verdict(cmd: &str) -> Option<Verdict> {
    classify(&cmd.to_lowercase())
}

fn is_deny(cmd: &str) -> bool {
    matches!(verdict(cmd), Some(Verdict::Deny(_)))
}

fn is_ask(cmd: &str) -> bool {
    matches!(verdict(cmd), Some(Verdict::Ask(_)))
}

#[test]
fn denies_live_service_mutation() {
    assert!(is_deny("gcloud run deploy web-cdn"));
    assert!(is_deny(
        "gcloud run services update web-cdn --region europe-west2"
    ));
    assert!(is_deny("gcloud run jobs delete sync"));
}

#[test]
fn folds_surface_prefixes_and_global_flags() {
    assert!(is_deny("gcloud alpha run deploy web-cdn"));
    assert!(is_deny("gcloud beta run deploy web-cdn --source ."));
    assert!(is_deny("gcloud --project foo run deploy web-cdn"));
    assert!(is_deny("gcloud --project=foo run deploy web-cdn"));
}

#[test]
fn denies_terraform_mutation_bare_and_chdir() {
    assert!(is_deny("terraform apply"));
    assert!(is_deny("terraform -chdir=infra/dev apply -auto-approve"));
    assert!(is_deny(
        "terraform -chdir=infra/dev state rm aws_s3_bucket.x"
    ));
}

#[test]
fn terraform_plan_is_read_only() {
    assert_eq!(verdict("terraform -chdir=infra/dev plan"), None);
}

#[test]
fn denies_iam_changes() {
    assert!(is_deny(
        "gcloud projects add-iam-policy-binding p --member=x --role=y"
    ));
}

#[test]
fn denies_storage_and_artifact_deletion() {
    assert!(is_deny("gcloud storage rm gs://bucket/x"));
    assert!(is_deny("gsutil rm gs://bucket/x"));
    assert!(is_deny("gcloud artifacts versions delete v1"));
}

#[test]
fn kubernetes_splits_between_deny_and_ask_by_blast_radius() {
    assert!(is_deny("kubectl delete namespace staging"));
    assert!(is_deny("kubectl delete pvc data"));
    assert!(is_deny("helm uninstall web -n prod"));
    assert!(is_ask("kubectl delete deployment api -n default"));
    assert!(is_ask("kubectl apply -f manifest.yaml"));
    assert!(is_ask("helm upgrade web ./chart"));
    assert_eq!(verdict("kubectl get pods -n default"), None);
}

#[test]
fn denies_a_direct_publish_but_not_a_dry_run() {
    assert!(is_deny("npm publish"));
    assert!(is_deny("pnpm publish --access public"));
    assert_eq!(verdict("npm publish --dry-run"), None);
}

#[test]
fn a_force_push_needs_all_three_conditions() {
    assert!(is_deny("git push --force origin main"));
    // Forced, but to a branch nobody else builds on — routine.
    assert_eq!(verdict("git push --force origin feat/mine"), None);
    assert_eq!(verdict("git push origin main"), None);
}

#[test]
fn asks_before_destructive_local_commands() {
    assert!(is_ask("rm -rf ../some-repo"));
    assert!(is_ask("rm -rf /"));
    assert!(is_ask("rm -rf ~"));
    assert!(is_ask("sudo systemctl restart nginx"));
    assert!(is_ask("pkill node"));
    assert!(is_ask("docker system prune"));
}

#[test]
fn an_ordinary_command_matches_nothing() {
    assert_eq!(verdict("ls -la"), None);
    assert_eq!(verdict("cargo test"), None);
    assert_eq!(verdict("git commit -m 'ship it'"), None);
}

#[test]
fn deny_beats_ask_when_both_could_match() {
    // `kubectl delete namespace` matches the ask-tier `kubectl delete` too;
    // ordering is what makes the destructive reading win.
    assert!(is_deny("kubectl delete namespace staging"));
}

#[test]
fn the_hook_mutation_test_is_not_the_scan_one() {
    // ` push` counts as a mutation for the risk signal but must NOT arm the
    // prod-token rule, or every `git push` in a repo whose name contains a prod
    // token would prompt.
    assert!(!is_mutating("git push origin main"));
    assert!(is_mutating("gcloud run deploy web"));
    assert!(is_mutating("rm -rf build"));
}

#[test]
fn breadth_is_detected_on_a_flag_boundary() {
    assert!(is_broad("gcloud run services delete --all"));
    assert!(is_broad("kubectl delete pods -a"));
    assert!(!is_broad("gcloud run deploy --allow-unauthenticated"));
}

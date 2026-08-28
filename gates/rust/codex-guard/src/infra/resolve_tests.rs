//! Sibling test file (rules/testing-gates.md — no inline test modules).
//!
//! The resolver's end-to-end behaviour is covered by the decision matrix in
//! `mod_tests.rs`, which runs it against a real tree. These cover the two pieces
//! that are wrong in ways a matrix would not localise: which references a text
//! block yields, and what the trust hash covers.

use super::*;

fn refs(text: &str) -> Vec<Ref> {
    extract_refs(text).into_iter().collect()
}

#[test]
fn finds_a_script_behind_an_interpreter() {
    assert!(refs("bash scripts/deploy.sh").contains(&Ref::Script("scripts/deploy.sh".into())));
    assert!(refs("sh ./x.sh").contains(&Ref::Script("./x.sh".into())));
}

#[test]
fn finds_a_script_executed_directly() {
    // No interpreter to key on, which is the case a naive resolver misses.
    assert!(refs("./scripts/deploy.sh").contains(&Ref::Script("./scripts/deploy.sh".into())));
    assert!(refs("scripts/deploy.sh").contains(&Ref::Script("scripts/deploy.sh".into())));
}

#[test]
fn folds_a_cd_prefix_into_the_path() {
    assert!(refs("cd sub && bash inner.sh").contains(&Ref::Script("sub/inner.sh".into())));
}

#[test]
fn reads_the_make_dir_flag_as_a_dir_not_a_target() {
    let found = refs("make -C sub deploy-sub");
    assert!(
        found.contains(&Ref::Make("sub".into(), "deploy-sub".into())),
        "{found:?}"
    );
    assert!(
        !found.contains(&Ref::Make(String::new(), "-C".into())),
        "{found:?}"
    );
}

#[test]
fn finds_a_bare_make_target_and_the_recursive_form() {
    assert!(refs("make build").contains(&Ref::Make(String::new(), "build".into())));
    assert!(refs("$(MAKE) deploy").contains(&Ref::Make(String::new(), "deploy".into())));
}

#[test]
fn finds_node_scripts_in_every_cwd_form() {
    assert!(refs("bun run release").contains(&Ref::Node(String::new(), "release".into())));
    assert!(
        refs("bun --cwd apps run release").contains(&Ref::Node("apps".into(), "release".into()))
    );
    assert!(
        refs("npm --prefix apps run release").contains(&Ref::Node("apps".into(), "release".into()))
    );
    assert!(
        refs("cd apps && pnpm run release").contains(&Ref::Node("apps".into(), "release".into()))
    );
}

#[test]
fn a_command_reaching_nothing_yields_no_refs() {
    assert!(refs("ls -la").is_empty());
    assert!(refs("gcloud run deploy web").is_empty());
}

#[test]
fn the_risk_signal_ignores_read_only_use() {
    assert!(risky_lines("gcloud run deploy web-cdn", None));
    assert!(!risky_lines(
        "gcloud logging read --project p --limit 10",
        None
    ));
    assert!(!risky_lines("echo building the bundle", None));
}

#[test]
fn a_prod_token_arms_the_risk_signal() {
    assert!(!risky_lines("curl https://acme-prod/health --fail", None));
    assert!(risky_lines(
        "curl -X delete https://acme-prod/x",
        Some("acme-prod")
    ));
}

#[test]
fn the_tree_hash_is_order_independent_and_content_sensitive() {
    let a = vec!["h1  x".to_string(), "h2  y".to_string()];
    let b = vec!["h2  y".to_string(), "h1  x".to_string()];
    assert_eq!(tree_hash(&a), tree_hash(&b));

    let c = vec!["h1  x".to_string(), "h3  y".to_string()];
    assert_ne!(tree_hash(&a), tree_hash(&c));
}

#[test]
fn an_empty_chain_has_no_hash_so_nothing_can_be_trusted_by_accident() {
    assert_eq!(tree_hash(&[]), None);
}

#[test]
fn the_digest_matches_what_shasum_produced() {
    // Byte-identical to `printf '%s' '' | shasum -a 256`, so a kosher file
    // written by the shell version keeps working after the port.
    assert_eq!(
        sha256_hex(""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

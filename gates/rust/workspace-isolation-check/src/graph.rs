//! Reading a build graph's members, and testing them against forbidden path
//! segments. Text in, findings out — no process spawn.

/// Workspace member manifest paths, from `cargo metadata --no-deps`.
///
/// `--no-deps` is what makes this answer the right question: it lists workspace
/// MEMBERS rather than every resolved registry crate, so a hit is something this
/// repo chose to build, not something a dependency happens to pull.
pub fn cargo_members(json: &str) -> Result<Vec<String>, String> {
    let doc: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("cargo metadata is not JSON: {e}"))?;
    let packages = doc
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "cargo metadata has no `packages` array".to_string())?;
    Ok(packages
        .iter()
        .filter_map(|p| p.get("manifest_path")?.as_str().map(str::to_string))
        .collect())
}

/// Members whose manifest path contains a forbidden segment.
///
/// Segments are matched with their slashes (`/control-plane/`), so a sibling
/// worktree directory named `feat-saas-control-plane` is not a hit. That false
/// positive is not hypothetical: agent worktrees are named after the branch.
pub fn leaks<'a>(members: &'a [String], forbidden: &[String]) -> Vec<&'a String> {
    members
        .iter()
        .filter(|m| forbidden.iter().any(|f| m.contains(f.as_str())))
        .collect()
}

/// Lines of a lockfile naming a forbidden package.
pub fn lock_hits(lockfile: &str, forbidden_names: &[String]) -> Vec<(usize, String)> {
    lockfile
        .lines()
        .enumerate()
        .filter(|(_, line)| forbidden_names.iter().any(|n| line.contains(n.as_str())))
        .map(|(idx, line)| (idx + 1, line.trim().to_string()))
        .collect()
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod graph_tests;

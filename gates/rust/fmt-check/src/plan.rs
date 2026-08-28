//! Assigns source files to the crate that owns them, then groups by edition.

use crate::manifest::CrateEditions;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// What the checker will run: one rustfmt edition per group of files.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// edition → files, sorted.
    pub groups: BTreeMap<String, Vec<PathBuf>>,
    /// Files under no package directory. Non-empty means the plan is incomplete.
    pub unowned: Vec<PathBuf>,
}

/// Groups `files` by the edition of their **nearest enclosing** crate
/// directory. A nested crate wins over the workspace above it, which is what
/// makes `services/api` a 2021 crate rather than whatever the repo root
/// happens to say.
pub fn build(files: &[PathBuf], editions: &CrateEditions) -> Plan {
    let mut plan = Plan::default();
    for file in files {
        match owner(file, editions) {
            Some(edition) => plan
                .groups
                .entry(edition.to_string())
                .or_default()
                .push(file.clone()),
            None => plan.unowned.push(file.clone()),
        }
    }
    for group in plan.groups.values_mut() {
        group.sort();
    }
    plan.unowned.sort();
    plan
}

fn owner<'a>(file: &Path, editions: &'a CrateEditions) -> Option<&'a str> {
    let mut candidate = file.parent();
    while let Some(dir) = candidate {
        if let Some(edition) = editions.get(dir) {
            return Some(edition.as_str());
        }
        candidate = dir.parent();
    }
    None
}

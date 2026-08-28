//! Workspace scan: which packages own files a script would cover, and which
//! declare that script.

use std::fs;
use std::path::{Path, PathBuf};

/// Directories that never hold a package's own sources.
const SKIPPED_DIRS: &[&str] = &[
    "node_modules",
    "dist",
    "build",
    "out",
    "coverage",
    "target",
    ".svelte-kit",
    ".next",
    ".turbo",
    ".vercel",
    ".git",
];

/// One `[test-scripts.script.<name>]` table, resolved.
pub struct Requirement {
    /// The script name a package must declare — `test`, `typecheck`, `lint`.
    pub script: String,
    /// Stem endings that make a file evidence. Empty means "any file of the
    /// declared extensions", which is how a typecheck requirement is expressed.
    pub stems: Vec<String>,
    pub extensions: Vec<String>,
    pub fix: String,
}

impl Requirement {
    /// Would this script have covered that file?
    pub fn covers(&self, file_name: &str) -> bool {
        let Some((stem, ext)) = file_name.rsplit_once('.') else {
            return false;
        };
        if !self.extensions.iter().any(|e| e == ext) {
            return false;
        }
        self.stems.is_empty() || self.stems.iter().any(|m| stem.ends_with(m.as_str()))
    }
}

pub struct Package {
    pub name: String,
    /// Relative to the repo root, for messages a human can paste.
    pub dir: PathBuf,
    pub declared: Vec<String>,
    /// Every file under the package, relative to the root.
    pub files: Vec<PathBuf>,
}

impl Package {
    pub fn declares(&self, script: &str) -> bool {
        self.declared.iter().any(|s| s == script)
    }

    /// Files the requirement would have covered.
    pub fn evidence(&self, requirement: &Requirement) -> Vec<&PathBuf> {
        self.files
            .iter()
            .filter(|f| {
                f.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| requirement.covers(n))
            })
            .collect()
    }
}

/// The `workspaces` globs the root manifest declares. Read rather than
/// hard-coded, so a newly added workspace root is covered the day it appears.
pub fn workspace_globs(root: &Path) -> Result<Vec<String>, String> {
    let manifest = root.join("package.json");
    let text = fs::read_to_string(&manifest)
        .map_err(|e| format!("cannot read {}: {e}", manifest.display()))?;
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("cannot parse {}: {e}", manifest.display()))?;
    // Bun and npm take an array; pnpm and yarn nest it under `packages`.
    let entries = json
        .get("workspaces")
        .and_then(|w| w.as_array().or_else(|| w.get("packages")?.as_array()))
        .ok_or_else(|| format!("{} declares no `workspaces`", manifest.display()))?;
    entries
        .iter()
        .map(|v| {
            v.as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("{}: a workspace entry is not a string", manifest.display()))
        })
        .collect()
}

/// Expand one workspace glob into package directories.
///
/// Only a trailing `/*` and literal paths are supported. Anything else is an
/// ERROR rather than a silent skip: a pattern quietly ignored here would reopen
/// exactly the hole the gate exists to close.
pub fn expand_glob(root: &Path, glob: &str) -> Result<Vec<PathBuf>, String> {
    if let Some(prefix) = glob.strip_suffix("/*") {
        if prefix.contains('*') {
            return Err(format!("unsupported workspace glob `{glob}`"));
        }
        let base = root.join(prefix);
        let Ok(entries) = fs::read_dir(&base) else {
            return Ok(Vec::new());
        };
        let mut dirs: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();
        return Ok(dirs);
    }
    if glob.contains('*') {
        return Err(format!("unsupported workspace glob `{glob}`"));
    }
    let dir = root.join(glob);
    Ok(if dir.is_dir() { vec![dir] } else { Vec::new() })
}

/// Read one package directory. `None` when it holds no manifest.
pub fn inspect(root: &Path, dir: &Path) -> Option<Package> {
    let manifest = dir.join("package.json");
    let text = fs::read_to_string(&manifest).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    let rel = dir.strip_prefix(root).unwrap_or(dir).to_path_buf();

    let declared = json
        .get("scripts")
        .and_then(|s| s.as_object())
        .map(|scripts| {
            scripts
                .iter()
                .filter(|(_, body)| body.as_str().is_some_and(|b| !b.trim().is_empty()))
                .map(|(name, _)| name.clone())
                .collect()
        })
        .unwrap_or_default();

    let mut files = Vec::new();
    collect_files(dir, root, &mut files);
    files.sort();

    Some(Package {
        name: json
            .get("name")
            .and_then(|n| n.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| rel.to_string_lossy().into_owned()),
        dir: rel,
        declared,
        files,
    })
}

fn collect_files(dir: &Path, root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if path.is_dir() {
            if !SKIPPED_DIRS.contains(&file_name.as_ref()) {
                collect_files(&path, root, out);
            }
            continue;
        }
        out.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
    }
}

pub fn scan(root: &Path) -> Result<Vec<Package>, String> {
    let mut packages = Vec::new();
    for glob in workspace_globs(root)? {
        for dir in expand_glob(root, &glob)? {
            if let Some(package) = inspect(root, &dir) {
                packages.push(package);
            }
        }
    }
    Ok(packages)
}

#[cfg(test)]
#[path = "scan_tests.rs"]
mod scan_tests;

//! Cargo manifest discovery and edition resolution.
//!
//! Only the keys that decide a file's rustfmt edition are read: `[package]
//! edition`, its `workspace = true` inheritance forms, and `[workspace.package]
//! edition`. Everything else in a manifest is ignored, which is why this parses
//! TOML by hand rather than adding a dependency.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Directories never descended into: build output, JS packages, git internals,
/// and cargo's vendor trees. Anything else — including `third_party/`, which is
/// a real workspace member — is walked.
const SKIP_DIRS: &[&str] = &["target", "node_modules", ".git", "vendor"];

/// How a `[package]` names its edition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditionSpec {
    /// `edition = "2021"`
    Literal(String),
    /// `edition.workspace = true` or `edition = { workspace = true }`
    Inherited,
}

/// The parts of a `Cargo.toml` that decide editions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Parsed {
    /// Present iff the manifest has a `[package]` table.
    pub package_edition: Option<EditionSpec>,
    pub has_package: bool,
    /// `[workspace.package] edition`, the value an heir inherits.
    pub workspace_edition: Option<String>,
    /// Any `[workspace…]` table makes this directory a workspace root.
    pub is_workspace_root: bool,
}

/// A manifest on disk, keyed by its directory relative to the repo root.
#[derive(Debug, Clone)]
pub struct Manifest {
    pub dir: PathBuf,
    pub parsed: Parsed,
}

/// Walks `root` and returns every manifest outside [`SKIP_DIRS`], sorted by
/// directory so the caller's output is stable.
pub fn discover(root: &Path) -> io::Result<Vec<Manifest>> {
    let mut found = Vec::new();
    walk(root, root, &mut found)?;
    found.sort_by(|a, b| a.dir.cmp(&b.dir));
    Ok(found)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<Manifest>) -> io::Result<()> {
    let manifest = dir.join("Cargo.toml");
    if manifest.is_file() {
        let text = fs::read_to_string(&manifest)?;
        let rel = dir.strip_prefix(root).unwrap_or(dir).to_path_buf();
        out.push(Manifest {
            dir: rel,
            parsed: parse(&text),
        });
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if SKIP_DIRS.contains(&name.as_ref()) || name.starts_with('.') {
            continue;
        }
        walk(root, &entry.path(), out)?;
    }
    Ok(())
}

/// Reads the edition-bearing keys out of a manifest's text.
pub fn parse(text: &str) -> Parsed {
    let mut parsed = Parsed::default();
    let mut table = String::new();
    for raw in text.lines() {
        let line = strip_comment(raw).trim();
        if let Some(header) = table_header(line) {
            table = header;
            if table == "package" {
                parsed.has_package = true;
            }
            if table == "workspace" || table.starts_with("workspace.") {
                parsed.is_workspace_root = true;
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match (table.as_str(), key) {
            ("package", "edition") => {
                parsed.package_edition = Some(edition_value(value));
            }
            ("package", "edition.workspace") => {
                parsed.package_edition = Some(EditionSpec::Inherited);
            }
            ("workspace.package", "edition") => {
                parsed.workspace_edition = unquote(value);
            }
            _ => {}
        }
    }
    parsed
}

/// `[foo]` / `[[foo]]` → `foo`. Anything else → `None`.
fn table_header(line: &str) -> Option<String> {
    let inner = line.strip_prefix('[')?.strip_suffix(']')?;
    let inner = inner.strip_prefix('[').unwrap_or(inner);
    let inner = inner.strip_suffix(']').unwrap_or(inner);
    Some(inner.trim().replace('"', ""))
}

fn edition_value(value: &str) -> EditionSpec {
    if value.starts_with('{') && value.contains("workspace") && value.contains("true") {
        return EditionSpec::Inherited;
    }
    match unquote(value) {
        Some(literal) => EditionSpec::Literal(literal),
        None => EditionSpec::Inherited,
    }
}

fn unquote(value: &str) -> Option<String> {
    let value = value.trim();
    let inner = value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))?;
    Some(inner.to_string())
}

/// Drops a trailing `#` comment, honouring double-quoted strings so a `#`
/// inside a description does not truncate the line.
fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut in_string = false;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'"' => in_string = !in_string,
            b'#' if !in_string => return &line[..i],
            _ => {}
        }
    }
    line
}

/// A package directory and the edition rustfmt must use for its files.
pub type CrateEditions = BTreeMap<PathBuf, String>;

/// Resolves every package's edition, inheriting from the nearest ancestor
/// workspace root that declares `[workspace.package] edition`.
///
/// An unresolvable edition is an error rather than a default: guessing would
/// silently check code under rules that are not its own.
pub fn resolve(manifests: &[Manifest]) -> Result<CrateEditions, Vec<String>> {
    let workspaces: BTreeMap<&Path, &str> = manifests
        .iter()
        .filter(|m| m.parsed.is_workspace_root)
        .filter_map(|m| {
            m.parsed
                .workspace_edition
                .as_deref()
                .map(|e| (m.dir.as_path(), e))
        })
        .collect();

    let mut editions = CrateEditions::new();
    let mut errors = Vec::new();
    for manifest in manifests.iter().filter(|m| m.parsed.has_package) {
        let display = manifest.dir.join("Cargo.toml").display().to_string();
        match &manifest.parsed.package_edition {
            Some(EditionSpec::Literal(edition)) => {
                editions.insert(manifest.dir.clone(), edition.clone());
            }
            Some(EditionSpec::Inherited) => match nearest_workspace(&manifest.dir, &workspaces) {
                Some(edition) => {
                    editions.insert(manifest.dir.clone(), edition.to_string());
                }
                None => errors.push(format!(
                    "{display}: inherits `edition` from the workspace, but no ancestor \
                     Cargo.toml declares `[workspace.package] edition`"
                )),
            },
            None => errors.push(format!(
                "{display}: no `edition` key. Add one — fmt-check will not guess an \
                 edition, because the wrong one checks the crate under rules that are \
                 not its own"
            )),
        }
    }
    if errors.is_empty() {
        Ok(editions)
    } else {
        Err(errors)
    }
}

fn nearest_workspace<'a>(dir: &Path, workspaces: &BTreeMap<&Path, &'a str>) -> Option<&'a str> {
    let mut candidate: Option<&Path> = Some(dir);
    while let Some(current) = candidate {
        if let Some(edition) = workspaces.get(current) {
            return Some(edition);
        }
        candidate = current.parent();
    }
    None
}

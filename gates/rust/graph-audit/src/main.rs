//! graph-audit — cross-check docs and tasks against a code-graph oracle.
//!
//! The graph (`graphify-out/graph.json`, produced by the `graphify` skill) is
//! ground truth for "what code exists". Docs and tasks are *claims*. This tool
//! resolves every code reference in a claim against the oracle and flags the
//! mismatches:
//! * docs  — stale code-path references in LIVING docs (rules, guides, AGENTS.md)
//! * tasks — open tasks whose create/extract target already exists
//!   (DONE-CANDIDATE), or whose split target grew / relocated (WRONG-SCOPE)
//!
//! Living docs describe current state, so a stale reference in one is a bug.
//! Historical docs (decision records, audits, proposals) describe past state, so
//! the same reference is expected and reported as INFO only. That split is the
//! whole reason the tool is usable: without it, every ADR ever written drowns
//! the report.
//!
//! Run from the repo root:
//!   graph-audit [docs|tasks|all] [--graph P] [--out P] [--living A,B] [--historical A,B]
//!
//! Defaults: `--graph graphify-out/graph.json`, `--out graphify-out/AUDIT.md`,
//! `--living AGENTS.md,rules,docs/guides`,
//! `--historical docs/decisions,docs/audits,docs/proposals`.
//!
//! The task audit shells out to `cn` (chronis). With no `cn` on PATH it reads
//! an empty list and reports nothing — it never fails the run.

use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Existence/structure index built from graph.json + the working tree.
struct Oracle {
    /// Every `source_file` path the graph knows (repo-relative).
    src_files: HashSet<String>,
    /// basename -> all full paths carrying it (for rename suggestions).
    by_basename: HashMap<String, Vec<String>>,
}

impl Oracle {
    fn load(graph_path: &str) -> Oracle {
        let raw =
            fs::read_to_string(graph_path).unwrap_or_else(|e| panic!("read {graph_path}: {e}"));
        let v: serde_json::Value = serde_json::from_str(&raw).expect("parse graph.json");
        let mut src_files = HashSet::new();
        let mut by_basename: HashMap<String, Vec<String>> = HashMap::new();
        if let Some(nodes) = v.get("nodes").and_then(|n| n.as_array()) {
            for n in nodes {
                let sf = n
                    .get("source_file")
                    .or_else(|| n.get("source_location"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("");
                let sf = sf.split(':').next().unwrap_or("").trim();
                if sf.is_empty() {
                    continue;
                }
                let norm = norm(sf);
                if src_files.insert(norm.clone()) {
                    if let Some(bn) = norm.rsplit('/').next() {
                        by_basename
                            .entry(bn.to_string())
                            .or_default()
                            .push(norm.clone());
                    }
                }
            }
        }
        Oracle {
            src_files,
            by_basename,
        }
    }

    /// Resolve a referenced path against graph + filesystem.
    fn resolve(&self, raw_path: &str) -> Resolution {
        let p = norm(raw_path);
        // Exact (graph or working tree).
        if self.src_files.contains(&p) || Path::new(&p).exists() {
            return Resolution::Exists;
        }
        if p.contains('/') {
            // Suffix match on a full trailing path segment — this is the fix for the
            // basename-collision noise mode (`mod.rs` matching every mod.rs in the repo).
            let needle = format!("/{p}");
            if let Some(hit) = self.src_files.iter().find(|s| s.ends_with(&needle)) {
                return Resolution::Exists_(hit.clone());
            }
            // A split target may now be a directory module rather than a file.
            if let Some(stem) = p.strip_suffix(".rs") {
                let dirneedle = format!("/{stem}/");
                if self.src_files.iter().any(|s| s.contains(&dirneedle)) || Path::new(stem).is_dir()
                {
                    return Resolution::SplitDir(stem.to_string());
                }
            }
        } else {
            // Bare filename (no directory): docs legitimately cite files by short name.
            // A unique basename match means the file EXISTS — not a stale reference.
            // Only an ambiguous bare name is uncertain (treat as placeholder-ish).
            if let Some(paths) = self.by_basename.get(&p) {
                return if paths.len() == 1 {
                    Resolution::Exists_(paths[0].clone())
                } else {
                    Resolution::Ambiguous
                };
            }
            return Resolution::Missing;
        }
        // Cited WITH a directory that doesn't exist, but the basename lives elsewhere:
        // a genuine relocation worth flagging (wrong path, not just short-form).
        if let Some(bn) = p.rsplit('/').next() {
            if let Some(paths) = self.by_basename.get(bn) {
                if paths.len() == 1 {
                    return Resolution::Rename(paths[0].clone());
                }
            }
        }
        Resolution::Missing
    }
}

#[allow(non_camel_case_types)]
enum Resolution {
    Exists,
    Exists_(String),  // exists, matched via suffix to this full path
    SplitDir(String), // file became a directory module
    Rename(String),   // not at cited path; unique basename match elsewhere
    Ambiguous,        // bare filename, many matches — ignore as placeholder
    Missing,
}

fn norm(p: &str) -> String {
    p.trim()
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

fn line_count(p: &str) -> Option<usize> {
    fs::read_to_string(p).ok().map(|s| s.lines().count())
}

// --- placeholder / glob ignore-list (illustrative tokens, not real refs) ---
fn is_placeholder(p: &str) -> bool {
    const STEMS: &[&str] = &[
        "foo", "bar", "baz", "qux", "example", "mymodule", "name", "thing",
    ];
    if p.contains('*') || p.starts_with('.') {
        return true; // glob or extension-pattern like `.svelte.ts`
    }
    // Any path segment that is an illustrative stem (e.g. `foo/tests.rs`, `bar.rs`).
    p.split('/').any(|seg| {
        let base = seg.split('.').next().unwrap_or(seg);
        STEMS.contains(&base)
            || (base.ends_with("_tests") && STEMS.contains(&base.trim_end_matches("_tests")))
    })
}

fn path_ref_re() -> Regex {
    // backticked token ending in a real source extension
    Regex::new(r"`([\w./@-]+\.(?:rs|ts|tsx|svelte|js))`").unwrap()
}

// ===================== DOCS AUDIT =====================

fn audit_docs(oracle: &Oracle, living_roots: &[String], historical_roots: &[String]) -> String {
    let living: Vec<String> = collect_docs(living_roots);
    let historical: Vec<String> = collect_docs(historical_roots);
    let re = path_ref_re();

    let mut out = String::from("# Docs accuracy audit\n\n");
    out.push_str("Living docs describe *current* state — stale refs are bugs.\n");
    out.push_str("Historical docs (ADRs/audits/proposals) describe *past* state — stale refs are expected and only INFO.\n\n");

    out.push_str("## Living docs (actionable)\n\n");
    let mut rows = scan_docs(&living, &re, oracle);
    rows.sort_by_key(|r| std::cmp::Reverse(r.1.len()));
    for (doc, missing, renames, total) in &rows {
        if missing.is_empty() && renames.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "### {doc}  ({}/{} refs unresolved)\n",
            missing.len() + renames.len(),
            total
        ));
        for m in missing {
            out.push_str(&format!("- ❌ MISSING `{m}`\n"));
        }
        for (from, to) in renames {
            out.push_str(&format!("- ↪ RENAME `{from}` → likely `{to}`\n"));
        }
        out.push('\n');
    }

    out.push_str("## Historical docs (INFO only — verify before editing)\n\n");
    let mut hrows = scan_docs(&historical, &re, oracle);
    hrows.sort_by_key(|r| std::cmp::Reverse(r.1.len()));
    for (doc, missing, _ren, total) in hrows.iter().take(20) {
        if missing.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "- {doc}: {}/{} unresolved → {:?}\n",
            missing.len(),
            total,
            missing.iter().take(5).collect::<Vec<_>>()
        ));
    }
    out
}

type DocRow = (String, Vec<String>, Vec<(String, String)>, usize);

fn scan_docs(docs: &[String], re: &Regex, oracle: &Oracle) -> Vec<DocRow> {
    let mut rows = Vec::new();
    for doc in docs {
        let txt = match fs::read_to_string(doc) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let mut refs: HashSet<String> = HashSet::new();
        for cap in re.captures_iter(&txt) {
            refs.insert(cap[1].to_string());
        }
        let mut missing = Vec::new();
        let mut renames = Vec::new();
        let total = refs.len();
        for r in refs {
            if is_placeholder(&r) {
                continue;
            }
            match oracle.resolve(&r) {
                Resolution::Missing => missing.push(r),
                Resolution::Rename(to) => renames.push((r, to)),
                _ => {}
            }
        }
        rows.push((doc.clone(), missing, renames, total));
    }
    rows
}

fn collect_docs(roots: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for r in roots {
        let p = Path::new(r);
        if p.is_file() {
            out.push(r.clone());
        } else if p.is_dir() {
            walk_md(p, &mut out);
        }
    }
    out.sort();
    out
}

fn walk_md(dir: &Path, out: &mut Vec<String>) {
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let path = e.path();
            if path.is_dir() {
                walk_md(&path, out);
            } else if path.extension().map(|x| x == "md").unwrap_or(false) {
                if let Some(s) = path.to_str() {
                    out.push(s.to_string());
                }
            }
        }
    }
}

// ===================== BEADS AUDIT =====================

fn audit_tasks(oracle: &Oracle) -> String {
    let list = cn(&["list", "--toon"]);
    let verb = Regex::new(r"(?i)\b(extract|split|create|add|build|implement|introduce)\b").unwrap();
    let pathtok = Regex::new(r"[\w./-]+\.(?:rs|ts|tsx|svelte)").unwrap();

    let create_re = Regex::new(r"(?i)\b(extract|create|add|introduce)\b").unwrap();
    let target_re = Regex::new(r"<\s*(\d{3,5})").unwrap();

    let mut done = Vec::new(); // target file already exists
    let mut grew = Vec::new(); // split goal unreachable by extraction (grew)
    let mut review = Vec::new(); // source god-module already restructured
    let mut open = Vec::new(); // valid, simply not done yet
    let mut scanned = 0usize;

    for line in list.lines() {
        let cols: Vec<&str> = line.split('|').collect();
        if cols.len() < 5 {
            continue;
        }
        let (id, _ty, title, _pri, status) = (cols[0], cols[1], cols[2], cols[3], cols[4]);
        if matches!(status, "done" | "archived" | "status") {
            continue;
        }
        if !verb.is_match(title) {
            continue;
        }
        // Decision is driven ONLY by the full-path target named in the TITLE — the file
        // the bead acts on. Description paths are too noisy (sibling subtasks, bare
        // filenames) to drive a verdict; they only ever appeared as false vetoes.
        let title_targets: Vec<String> = pathtok
            .find_iter(title)
            .map(|m| m.as_str().to_string())
            .filter(|t| t.contains('/') && !is_placeholder(t))
            .collect();
        let primary = match title_targets.first() {
            Some(p) => p.clone(),
            None => continue,
        };
        scanned += 1;

        let desc = cn(&["show", id, "--toon"]);
        let is_split = title.to_lowercase().contains("split");
        let is_create = create_re.is_match(title);
        let head = format!("**[{id}]** {}", title.trim());
        let res = oracle.resolve(&primary);
        let resolved_path = match &res {
            Resolution::Exists => Some(primary.clone()),
            Resolution::Exists_(h) => Some(h.clone()),
            _ => None,
        };

        // Split parent with a numeric "< N" goal: did the file actually shrink to target?
        if is_split {
            if let Some(c) = target_re.captures(&desc) {
                let target: usize = c[1].parse().unwrap_or(0);
                if let Some(rp) = &resolved_path {
                    if let Some(lc) = line_count(rp) {
                        if lc > target {
                            grew.push(format!(
                                "- {head}\n  - ⚠️ `{rp}` = {lc} lines, goal <{target} (GREW). Extraction won't reach it — see rules/layer-boundaries.md on opening a god module. Re-scope or close.\n"
                            ));
                        } else {
                            done.push(format!(
                                "- {head}\n  - ✅ `{rp}` = {lc} lines ≤ goal <{target}.\n"
                            ));
                        }
                        continue;
                    }
                }
            }
        }

        // Extract/create child: the named file IS the new artifact.
        if is_create {
            match &res {
                Resolution::Exists | Resolution::Exists_(_) => {
                    let hit = resolved_path.unwrap_or(primary.clone());
                    done.push(format!(
                        "- {head}\n  - ✅ target `{hit}` already exists.\n"
                    ));
                }
                Resolution::SplitDir(d) => review.push(format!(
                    "- {head}\n  - 🔎 target file absent but source already split into `{d}/` — verify whether done under a different filename.\n"
                )),
                Resolution::Rename(to) => review.push(format!(
                    "- {head}\n  - 🔎 cited path absent; closest is `{to}` — code relocated, re-check premise.\n"
                )),
                Resolution::Missing => {
                    // New file not yet created. Valid open work UNLESS the source parent
                    // (dir the file would live in) no longer exists.
                    let parent = primary.rsplit_once('/').map(|(d, _)| d.to_string());
                    let src_gone = parent
                        .as_ref()
                        .map(|d| !Path::new(d).exists() && !oracle.src_files.iter().any(|s| s.contains(&format!("/{d}/")) || s.starts_with(&format!("{d}/"))))
                        .unwrap_or(false);
                    if src_gone {
                        review.push(format!(
                            "- {head}\n  - 🔎 destination dir `{}` not found — premise may be stale.\n",
                            parent.unwrap_or_default()
                        ));
                    } else {
                        open.push(format!("- {head} — target `{primary}` not yet created (valid open).\n"));
                    }
                }
                Resolution::Ambiguous => {}
            }
        }
    }

    let mut out = String::from("# Task audit\n\n");
    out.push_str(&format!(
        "Scanned {scanned} open/in-progress tasks carrying a create/split verb + full-path target.\n\n"
    ));
    out.push_str(&format!(
        "## ✅ DONE-CANDIDATES ({}) — target already exists; close them\n\n",
        done.len()
    ));
    out.extend(done);
    out.push_str(&format!(
        "\n## ⚠️ WRONG-SCOPE ({}) — split goal unreachable by extraction\n\n",
        grew.len()
    ));
    out.extend(grew);
    out.push_str(&format!(
        "\n## 🔎 REVIEW ({}) — source restructured / code relocated\n\n",
        review.len()
    ));
    out.extend(review);
    out.push_str(&format!(
        "\n## valid open ({}) — premise holds, just not done\n\n",
        open.len()
    ));
    out.extend(open);
    out
}

fn cn(args: &[&str]) -> String {
    Command::new("cn")
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

// ===================== MAIN =====================

/// `--name value` from the argument list, if present.
fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// A comma-separated `--name` list, or the given defaults.
fn roots(args: &[String], name: &str, defaults: &[&str]) -> Vec<String> {
    match flag(args, name) {
        Some(v) => v
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
        None => defaults.iter().map(|s| s.to_string()).collect(),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = args
        .first()
        .filter(|a| !a.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| "all".to_string());
    let graph = flag(&args, "--graph").unwrap_or_else(|| "graphify-out/graph.json".to_string());
    let out_path = flag(&args, "--out").unwrap_or_else(|| "graphify-out/AUDIT.md".to_string());
    let living = roots(&args, "--living", &["AGENTS.md", "rules", "docs/guides"]);
    let historical = roots(
        &args,
        "--historical",
        &["docs/decisions", "docs/audits", "docs/proposals"],
    );

    let oracle = Oracle::load(&graph);
    eprintln!(
        "[graph-audit] oracle: {} graph files indexed",
        oracle.src_files.len()
    );

    let mut report = String::new();
    match mode.as_str() {
        "docs" => report.push_str(&audit_docs(&oracle, &living, &historical)),
        "tasks" | "beads" => report.push_str(&audit_tasks(&oracle)),
        _ => {
            report.push_str(&audit_docs(&oracle, &living, &historical));
            report.push_str("\n\n---\n\n");
            report.push_str(&audit_tasks(&oracle));
        }
    }
    if let Some(dir) = Path::new(&out_path).parent() {
        let _ = fs::create_dir_all(dir);
    }
    fs::write(&out_path, &report).unwrap_or_else(|e| panic!("write {out_path}: {e}"));
    println!("{report}");
    eprintln!("[graph-audit] written to {out_path}");
}

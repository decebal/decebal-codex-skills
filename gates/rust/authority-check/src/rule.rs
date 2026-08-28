//! The rule model and the per-file scan. Content in, hits out — no filesystem.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub path: String,
    pub line: usize,
    pub marker: String,
    pub kind: Kind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A renderer reconstructing the authority's logic.
    Rederived,
    /// A symbol retired outright — forbidden even in the authority.
    Retired,
}

pub struct Rule {
    pub name: String,
    /// The file that DEFINES the verdict. Exempt from `markers`.
    pub authority: String,
    /// Trees that may only render the verdict.
    pub renderers: Vec<String>,
    pub extensions: Vec<String>,
    /// Symbols only the authority (and the wire contract) may name.
    pub markers: Vec<String>,
    /// The single file allowed to name the verdict's wire shape — it IS the
    /// contract, not a second implementation of it.
    pub contract: Option<String>,
    /// Symbols that no longer exist anywhere. Not exempt for the authority.
    pub retired: Vec<String>,
    pub why: String,
}

impl Rule {
    pub fn selects(&self, path: &str) -> bool {
        if !has_extension(path, &self.extensions) {
            return false;
        }
        self.renderers.iter().any(|r| path.starts_with(r.as_str()))
    }

    /// A test may legitimately name a retired symbol while asserting it is gone,
    /// and may name a marker while asserting the renderer does not compute it.
    pub fn is_exempt_test(&self, path: &str) -> bool {
        let name = path.rsplit('/').next().unwrap_or(path);
        path.contains("/__tests__/")
            || path.contains("/__vtests__/")
            || name.contains(".test.")
            || name.contains(".spec.")
            || name.contains(".vitest.")
    }
}

/// Which markers apply to this path.
///
/// The authority is exempt from `markers` — defining them is its job — but never
/// from `retired`, because a symbol that no longer exists must not survive in the
/// one file everything else defers to.
pub fn scan(path: &str, content: &str, rule: &Rule) -> Vec<Hit> {
    let is_authority = path == rule.authority;
    let is_contract = rule.contract.as_deref() == Some(path);

    let mut hits = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        if is_comment(line) {
            continue;
        }
        if !is_authority && !is_contract {
            for marker in &rule.markers {
                if line.contains(marker.as_str()) {
                    hits.push(Hit {
                        path: path.to_string(),
                        line: idx + 1,
                        marker: marker.clone(),
                        kind: Kind::Rederived,
                    });
                }
            }
        }
        for marker in &rule.retired {
            if line.contains(marker.as_str()) {
                hits.push(Hit {
                    path: path.to_string(),
                    line: idx + 1,
                    marker: marker.clone(),
                    kind: Kind::Retired,
                });
            }
        }
    }
    hits
}

/// Markers the authority does NOT define. The premise of the whole rule.
pub fn undefined_markers(authority_text: &str, rule: &Rule) -> Vec<String> {
    rule.markers
        .iter()
        .filter(|m| !authority_text.contains(m.as_str()))
        .cloned()
        .collect()
}

fn is_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with("/*") || t.starts_with('*') || t.starts_with('#')
}

pub fn has_extension(path: &str, extensions: &[String]) -> bool {
    if extensions.is_empty() {
        return true;
    }
    match path.rsplit_once('.') {
        Some((_, ext)) => extensions.iter().any(|e| e == ext),
        None => false,
    }
}

#[cfg(test)]
#[path = "rule_tests.rs"]
mod rule_tests;

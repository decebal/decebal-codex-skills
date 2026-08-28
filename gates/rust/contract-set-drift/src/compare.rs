//! Extracting the sets from a document, and the comparison itself.

use std::collections::{BTreeMap, BTreeSet};

/// Entry name -> the set it declares.
pub type Sets = BTreeMap<String, BTreeSet<String>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    Both,
    LeftOnly,
    RightOnly,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Drift {
    pub entry: String,
    pub presence: Presence,
    /// The right side declares these; the left does not.
    pub missing_from_left: BTreeSet<String>,
    /// The left side declares these; the right does not.
    pub missing_from_right: BTreeSet<String>,
}

impl Drift {
    /// Does this row say the LEFT side is the lagging half?
    pub fn blames_left(&self) -> bool {
        !self.missing_from_left.is_empty() || self.presence == Presence::RightOnly
    }

    /// Does this row say the RIGHT side is the lagging half?
    pub fn blames_right(&self) -> bool {
        !self.missing_from_right.is_empty() || self.presence == Presence::LeftOnly
    }
}

/// Both directions, and they mean different things: something the right side
/// requires but the left never declares says the LEFT is behind; the reverse
/// says the right is. One real outage had one of each, in two different
/// features, at the same time.
///
/// An entry missing from one side entirely is drift too, even when its set is
/// empty — a feature that needs no operations would otherwise vanish silently,
/// because set equality alone calls two empty sets a match.
pub fn compare(left: &Sets, right: &Sets) -> Vec<Drift> {
    let mut entries: BTreeSet<&String> = left.keys().collect();
    entries.extend(right.keys());

    entries
        .into_iter()
        .filter_map(|entry| {
            let presence = match (left.get(entry), right.get(entry)) {
                (Some(_), Some(_)) => Presence::Both,
                (Some(_), None) => Presence::LeftOnly,
                (None, Some(_)) => Presence::RightOnly,
                (None, None) => return None,
            };
            let empty = BTreeSet::new();
            let l = left.get(entry).unwrap_or(&empty);
            let r = right.get(entry).unwrap_or(&empty);
            if presence == Presence::Both && l == r {
                return None;
            }
            Some(Drift {
                entry: entry.clone(),
                presence,
                missing_from_left: r.difference(l).cloned().collect(),
                missing_from_right: l.difference(r).cloned().collect(),
            })
        })
        .collect()
}

/// Walk a dotted path into a document. An empty path is the document itself.
pub fn at<'a>(doc: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut node = doc;
    for segment in path.split('.').filter(|s| !s.is_empty()) {
        node = node.get(segment)?;
    }
    Some(node)
}

/// Read `<map_path>` as an object of entries, taking each entry's set from
/// `<set_key>` (or the entry itself, when `set_key` is empty).
pub fn extract(doc: &serde_json::Value, map_path: &str, set_key: &str) -> Result<Sets, String> {
    let node =
        at(doc, map_path).ok_or_else(|| format!("no value at `{map_path}` in the document"))?;
    let object = node
        .as_object()
        .ok_or_else(|| format!("`{map_path}` is not an object"))?;

    let mut sets = Sets::new();
    for (name, entry) in object {
        let target = if set_key.is_empty() {
            entry
        } else {
            match at(entry, set_key) {
                Some(v) => v,
                // A declared entry with no set is an EMPTY set, not an absent
                // entry: its presence is half of what this gate compares.
                None => {
                    sets.insert(name.clone(), BTreeSet::new());
                    continue;
                }
            }
        };
        let values = target
            .as_array()
            .ok_or_else(|| format!("`{map_path}.{name}.{set_key}` is not an array"))?;
        let mut set = BTreeSet::new();
        for v in values {
            let s = v
                .as_str()
                .ok_or_else(|| format!("`{map_path}.{name}` holds a non-string"))?;
            set.insert(s.to_string());
        }
        sets.insert(name.clone(), set);
    }
    Ok(sets)
}

#[cfg(test)]
#[path = "compare_tests.rs"]
mod compare_tests;

//! The direction model, and the scan over one file's content.
//!
//! Everything here takes strings, so the whole gate is testable without a
//! filesystem or a repo.

use std::collections::BTreeMap;

/// One declared layer: its name, and the path prefix its files live under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    pub name: String,
    pub dir: String,
}

/// The whole model.
pub struct Model {
    /// Dependency direction. A layer may import only layers AFTER it.
    pub order: Vec<Layer>,
    /// Layers that may import no other declared layer at all, in either
    /// direction — the domain, in the canonical four.
    pub pure: Vec<Layer>,
    /// `use crate::{layer}` — `{layer}` is replaced by the layer NAME.
    pub import_template: String,
    /// `from_to_to` -> the highest count that still passes.
    pub ceilings: BTreeMap<String, usize>,
    /// `from_to_to` -> the one file allowed to cross that edge.
    pub facades: BTreeMap<String, String>,
}

/// A directed pair that must not exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub from: Layer,
    pub to: Layer,
}

impl Edge {
    /// The config key for this edge's ceiling or facade.
    pub fn key(&self) -> String {
        format!("{}_to_{}", self.from.name, self.to.name)
    }

    pub fn label(&self) -> String {
        format!("{} -> {}", self.from.name, self.to.name)
    }
}

impl Model {
    /// Every declared layer, ordered ones first.
    pub fn layers(&self) -> impl Iterator<Item = &Layer> {
        self.order.iter().chain(self.pure.iter())
    }

    /// The layer a file belongs to: the one whose `dir` is the LONGEST matching
    /// prefix.
    ///
    /// Layer directories legitimately nest — a repo may put `application` at
    /// `src/` and `domain` at `src/domain/`. Matching on "any prefix" then
    /// attributes every domain file to BOTH layers, which double-counts each
    /// violation and, worse, reads domain files as application files when
    /// checking the edges application owns. Longest-prefix makes the
    /// attribution single-valued, which is what a count can be trusted from —
    /// and these counts are compared against ceilings.
    pub fn owning_layer(&self, path: &str) -> Option<&Layer> {
        self.layers()
            .filter(|layer| path.starts_with(layer.dir.as_str()))
            .max_by_key(|layer| layer.dir.len())
    }

    /// Every pair the direction forbids.
    ///
    /// Two sources, and they are different rules: an ORDERED layer may not
    /// import anything before it (that is the arrow pointing backwards), while a
    /// PURE layer may not import any other declared layer at all, nor be the
    /// target of one from below — it sits outside the ordering rather than at
    /// one end of it.
    pub fn forbidden_edges(&self) -> Vec<Edge> {
        let mut edges = Vec::new();
        for (i, from) in self.order.iter().enumerate() {
            for to in self.order.iter().take(i) {
                edges.push(Edge {
                    from: from.clone(),
                    to: to.clone(),
                });
            }
        }
        for from in &self.pure {
            for to in self.order.iter().chain(self.pure.iter()) {
                if to.name != from.name {
                    edges.push(Edge {
                        from: from.clone(),
                        to: to.clone(),
                    });
                }
            }
        }
        edges
    }

    /// The import statement that crosses `edge`.
    pub fn import_of(&self, edge: &Edge) -> String {
        self.import_template.replace("{layer}", &edge.to.name)
    }

    pub fn ceiling_for(&self, edge: &Edge) -> usize {
        self.ceilings.get(&edge.key()).copied().unwrap_or(0)
    }

    pub fn facade_for(&self, edge: &Edge) -> Option<&str> {
        self.facades.get(&edge.key()).map(String::as_str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub path: String,
    pub line: usize,
    pub text: String,
}

/// Lines of `content` that import `needle`, ignoring comments and test code.
///
/// Test code is cut at the first `#[cfg(test)]`: by this repo's own convention
/// a test module sits at the bottom of the file, so everything from that
/// attribute on is test code. A test that names a forbidden import while
/// asserting it is absent must not fail the gate that asserts the same thing.
pub fn imports_in(path: &str, content: &str, needle: &str) -> Vec<Hit> {
    let cut = content
        .lines()
        .position(|l| l.trim_start().starts_with("#[cfg(test)]"))
        .unwrap_or(usize::MAX);

    content
        .lines()
        .enumerate()
        .take_while(|(idx, _)| *idx < cut)
        .filter(|(_, line)| !is_comment(line))
        .filter(|(_, line)| line.contains(needle))
        .map(|(idx, line)| Hit {
            path: path.to_string(),
            line: idx + 1,
            text: line.trim().to_string(),
        })
        .collect()
}

fn is_comment(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with("/*") || t.starts_with('*')
}

/// Is this path a test file, whichever convention the repo uses?
pub fn is_test_file(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    path.split('/').any(|segment| segment == "tests")
        || name == "tests.rs"
        || name.ends_with("_tests.rs")
        || name.ends_with("_test.rs")
}

#[cfg(test)]
#[path = "layers_tests.rs"]
mod layers_tests;

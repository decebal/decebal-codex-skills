//! Reading a price table out of a document, and comparing two of them.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Price {
    pub input: f64,
    pub output: f64,
}

pub type Table = BTreeMap<String, Price>;

/// Where the two numbers sit inside one entry, and what scales them into a
/// common unit. References usually quote per-token; a shipped table usually
/// quotes per-million, and comparing the two raw is off by six orders of
/// magnitude in a way that looks like every price being wrong.
pub struct Shape {
    pub map_path: String,
    pub input_key: String,
    pub output_key: String,
    pub scale: f64,
}

#[derive(Debug, PartialEq)]
pub struct Disagreement {
    pub id: String,
    pub ours: Price,
    pub theirs: Price,
}

/// Round to the cent-per-million the tables are actually authored in, so a
/// float representation artefact is never reported as a price change.
pub fn round(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

pub fn at<'a>(doc: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut node = doc;
    for segment in path.split('.').filter(|s| !s.is_empty()) {
        node = node.get(segment)?;
    }
    Some(node)
}

/// Every entry carrying BOTH numbers.
///
/// An entry with neither is dropped rather than defaulted: a reference of a few
/// thousand models holds embedding, moderation and audio entries shaped
/// differently, and inventing a zero for them would report every one as a
/// mismatch.
pub fn extract(doc: &serde_json::Value, shape: &Shape) -> Result<Table, String> {
    let node =
        at(doc, &shape.map_path).ok_or_else(|| format!("no value at `{}`", shape.map_path))?;
    let object = node
        .as_object()
        .ok_or_else(|| format!("`{}` is not an object", shape.map_path))?;

    let mut table = Table::new();
    for (id, entry) in object {
        let (Some(input), Some(output)) = (
            entry
                .get(&shape.input_key)
                .and_then(serde_json::Value::as_f64),
            entry
                .get(&shape.output_key)
                .and_then(serde_json::Value::as_f64),
        ) else {
            continue;
        };
        table.insert(
            id.clone(),
            Price {
                input: round(input * shape.scale),
                output: round(output * shape.scale),
            },
        );
    }
    Ok(table)
}

/// Ids present in both tables whose prices differ by more than `tolerance`.
///
/// Only the intersection is compared. A model the reference has never heard of
/// is not evidence our price is wrong — it is evidence the reference does not
/// cover it, and failing on that would make the gate unusable the week a vendor
/// ships anything.
pub fn disagreements(
    ours: &Table,
    theirs: &Table,
    tolerance: f64,
    allow: &[String],
) -> Vec<Disagreement> {
    ours.iter()
        .filter(|(id, _)| !allow.iter().any(|a| a == *id))
        .filter_map(|(id, our_price)| {
            let their_price = theirs.get(id)?;
            let differs = (our_price.input - their_price.input).abs() > tolerance
                || (our_price.output - their_price.output).abs() > tolerance;
            differs.then(|| Disagreement {
                id: id.clone(),
                ours: *our_price,
                theirs: *their_price,
            })
        })
        .collect()
}

/// Ids we price that the reference does not cover — reported, never failed.
pub fn uncovered(ours: &Table, theirs: &Table) -> Vec<String> {
    ours.keys()
        .filter(|id| !theirs.contains_key(*id))
        .cloned()
        .collect()
}

#[cfg(test)]
#[path = "prices_tests.rs"]
mod prices_tests;

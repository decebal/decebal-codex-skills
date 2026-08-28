//! Decide whether the difference between two versions of a Rust source file is
//! "code-effective" — i.e. whether it changes anything the compiler or the
//! program's behavior cares about, as opposed to only comments, doc comments,
//! whitespace, or formatting.
//!
//! Strategy: tokenize both versions with `proc-macro2`. The lexer already drops
//! ordinary `//` and `/* */` comments (they are not tokens) and normalizes all
//! whitespace/formatting, so two files that differ only in those produce the
//! same token stream. Doc comments (`///`, `//!`) survive tokenization as
//! `#[doc = "..."]` / `#![doc = "..."]` attributes, so we additionally strip
//! those before comparing — a doc-comment edit cannot break `cargo
//! check`/`clippy`/tests (the desktop crate has no doctests; see
//! rules/testing-gates.md), so it is safe to treat it as non-effective.
//!
//! Safety bias: anything that is NOT provably comment/whitespace/doc-only is
//! reported as effective. String and char literals are preserved as tokens, so
//! any change to their contents counts as effective. If either version fails to
//! tokenize (a partial or invalid edit), the change is reported as effective.

use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use std::str::FromStr;

/// Returns `true` if the change from `old` to `new` affects compilation or
/// behavior; `false` only when the two differ purely in comments, doc comments,
/// whitespace, or formatting.
pub fn is_effective(old: &str, new: &str) -> bool {
    match (canonical(old), canonical(new)) {
        (Some(a), Some(b)) => a != b,
        // A version that does not tokenize (partial/invalid edit) — be
        // conservative and treat the change as effective.
        _ => true,
    }
}

/// Canonical, comment-and-doc-free rendering of a source file, or `None` if it
/// does not tokenize as Rust.
fn canonical(src: &str) -> Option<String> {
    let ts = TokenStream::from_str(src).ok()?;
    Some(strip_doc_attrs(ts).to_string())
}

/// Recursively remove `#[doc = ...]` (outer) and `#![doc = ...]` (inner)
/// attributes — the token form proc-macro2 produces for `///` and `//!` doc
/// comments (and hand-written `#[doc]`). Every other attribute (`#[cfg]`,
/// `#[derive]`, …) is preserved, because those DO affect compilation.
fn strip_doc_attrs(ts: TokenStream) -> TokenStream {
    let toks: Vec<TokenTree> = ts.into_iter().collect();
    let mut out: Vec<TokenTree> = Vec::with_capacity(toks.len());
    let mut i = 0;
    while i < toks.len() {
        if is_punct(&toks[i], '#') {
            // Inner attribute: `#` `!` `[ doc … ]`
            if i + 2 < toks.len() && is_punct(&toks[i + 1], '!') && is_doc_bracket(&toks[i + 2]) {
                i += 3;
                continue;
            }
            // Outer attribute: `#` `[ doc … ]`
            if i + 1 < toks.len() && is_doc_bracket(&toks[i + 1]) {
                i += 2;
                continue;
            }
        }
        match &toks[i] {
            TokenTree::Group(g) => {
                out.push(TokenTree::Group(Group::new(
                    g.delimiter(),
                    strip_doc_attrs(g.stream()),
                )));
            }
            other => out.push(other.clone()),
        }
        i += 1;
    }
    out.into_iter().collect()
}

fn is_punct(tt: &TokenTree, ch: char) -> bool {
    matches!(tt, TokenTree::Punct(p) if p.as_char() == ch)
}

/// A `[ doc … ]` bracket group — the body of a doc attribute.
fn is_doc_bracket(tt: &TokenTree) -> bool {
    if let TokenTree::Group(g) = tt {
        if g.delimiter() == Delimiter::Bracket {
            if let Some(TokenTree::Ident(id)) = g.stream().into_iter().next() {
                return id == "doc";
            }
        }
    }
    false
}

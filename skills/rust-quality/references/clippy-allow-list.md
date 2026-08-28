---
title: Clippy Pedantic Allow-List
impact: HIGH
impactDescription: turns clippy::pedantic from unusable to a real gate
tags: clippy, pedantic, lints, allow-list, workspace
---

# Clippy Pedantic Allow-List

`clippy::pedantic` catches genuine smells, but a handful of its lints fire so
often on idiomatic, correct code that leaving them at `warn` buries the
signal. This is the battle-tested set to `allow` at the workspace root so the
rest of `pedantic` stays useful as a gate.

Every name below is a real `clippy::pedantic` lint. Paste them under the
`[workspace.lints.clippy]` group entries (which carry `priority = -1`) exactly
as shown in the main skill.

| Lint | Fires on | Why allow it |
|---|---|---|
| `module_name_repetitions` | `mod http { struct HttpClient; }` | Re-exported paths (`http::HttpClient`) read fine; the "repetition" is intentional API naming. |
| `must_use_candidate` | any pub fn returning a value | `#[must_use]` matters for a few builder/guard types, not every getter; annotate those deliberately. |
| `missing_errors_doc` | pub fn returning `Result` | Demands an `# Errors` doc section on every fallible fn — enormous churn for internal crates. |
| `missing_panics_doc` | pub fn that can panic | Same churn as above for a `# Panics` section; enable only on a polished public API. |
| `cast_precision_loss` | `x as f64` from a wide int | Almost every int→float cast trips it; the loss is understood and intended at the call site. |
| `cast_possible_truncation` | `x as u32` from a wider int | Fires on every narrowing cast; use `try_into()` where it actually matters, not everywhere. |
| `cast_sign_loss` | `x as u64` from a signed int | Ubiquitous in index/length math where the value is known non-negative. |
| `cast_possible_wrap` | `x as i64` from an unsigned int | Same story as sign-loss; the ranges are known-safe in practice. |
| `similar_names` | `req`/`res`, `lat`/`lon` | Short, conventional pairs are clearer than artificially divergent names. |
| `too_many_lines` | fn over ~100 lines | A length heuristic, not a complexity one; splitting to satisfy it can hurt readability. See [../../../rules/layer-boundaries.md](../../../rules/layer-boundaries.md) for a real shape-based split instead. |
| `doc_markdown` | `GraphQL`, `OAuth` in doc text | Flags proper nouns as needing backticks; overwhelmingly false positives. |
| `struct_excessive_bools` | config struct with 3+ bools | Feature-flag/config structs legitimately hold many bools. |
| `fn_params_excessive_bools` | fn with 3+ bool params | Same as above for constructors and option-bag fns; a param struct is a judgement call, not a blanket rule. |
| `match_same_arms` | two arms with identical bodies | Merging arms often obscures that they model distinct cases; keeping them apart aids future edits. |
| `redundant_closure_for_method_calls` | `.map(\|x\| x.len())` | `.map(str::len)` is not always clearer and breaks when the signature shifts; stylistic. |
| `unused_self` | method not touching `self` | Trait-impl and future-proofing methods legitimately ignore `self`. |
| `return_self_not_must_use` | builder method returning `Self` | Would demand `#[must_use]` on every chained builder method — pure noise. |
| `items_after_statements` | `fn`/`const` declared mid-block | A local helper defined next to its single use is a readability win, not a smell. |
| `single_match_else` | `match` with one arm + `_` | `match` is often clearer than the `if let ... else` it wants, especially with guards. |
| `map_unwrap_or` | `.map(f).unwrap_or(d)` | The `.map_or(d, f)` rewrite reorders args and reads worse to many; stylistic. |
| `needless_pass_by_value` | fn taking `String`/`Vec` by value | Frequently the value IS consumed or moved into a struct; the lint can't see intent. |
| `wildcard_imports` | `use foo::*;` | Prelude modules and generated code rely on globs; enforce selectively, not workspace-wide. |
| `if_not_else` | `if !cond { a } else { b }` | The negated-first form is often the more natural reading for the domain. |
| `unreadable_literal` | `1000000` | Digit-grouping is worth doing but not worth a warning on every constant; fix on touch. |

## Adjusting the list

- **Promote back to `warn` on a public library crate.** `missing_errors_doc`,
  `missing_panics_doc`, `must_use_candidate`, and `doc_markdown` earn their
  keep on a polished, published API. Override them in that crate's own
  `[lints.clippy]` table rather than removing them from the workspace default.
- **Never allow a lint you have not seen fire.** Add to this list only when a
  real, correct construct in your code trips it — a speculative allow hides a
  future genuine warning.
- **Keep the allow-list additive to the group, not a replacement.** The value
  is `all` + `pedantic` at `warn` MINUS this short, justified subtraction.

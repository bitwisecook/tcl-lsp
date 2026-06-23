//! `textDocument/implementation` — `TclOO` subclass / method-override
//! fan-out.
//!
//! Where plain go-to-definition jumps to the
//! *defining* site, go-to-implementation answers "who realises this":
//!
//! - **Cursor on a class name** → the classes that list it among their
//!   `superclasses` / `mixins` (its direct subclasses).
//! - **Cursor on a method name, outside any class** → every class that
//!   defines a method of that name (all implementations).
//! - **Cursor on a method name, inside a class body** → the enclosing
//!   class's own definition plus the overrides in its descendants
//!   (ancestor definitions are intentionally omitted — they are
//!   reachable through go-to-definition).
//!
//! The Rust analyser models class / method bodies as `ClassDef`
//! `body_span`s rather than distinct scope kinds (there is no
//! `ScopeKind::Method`), so the enclosing class is the innermost class
//! whose `body_span` contains the cursor — the same test
//! `definition.rs::lookup_class_member` uses.

use std::collections::BTreeSet;

use tcl_compiler::analyser::{AnalysisResult, ClassDef};
use tcl_lexer::{LineIndex, Span};

use crate::definition::{LspRange, byte_offset_at, span_to_range};
use crate::hover::find_word_span_at_position;

/// Compute "go-to-implementation" locations for the symbol at the
/// cursor.  Returns an empty vector when the cursor is not on a class
/// or method name, or when nothing realises it.
#[must_use]
pub fn implementation(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<LspRange> {
    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);

    // Collected target spans, de-duplicated and ordered by start
    // offset so the response is deterministic.
    let mut spans: BTreeSet<(u32, u32)> = BTreeSet::new();

    // Case 1: the word names a class
    let target_qnames: Vec<&str> = analysis
        .all_classes
        .values()
        .filter(|cd| class_matches(cd, &word))
        .map(|cd| cd.qualified_name.as_str())
        .collect();
    if !target_qnames.is_empty() {
        for cd in analysis.all_classes.values() {
            if parents_of(cd).any(|p| qname_matches_any(p, &target_qnames)) {
                spans.insert((cd.name_span.start(), cd.name_span.end()));
            }
        }
        return finish(source, &line_index, spans);
    }

    // Case 2/3: the word names a method
    let cursor = byte_offset_at(source, line, character);
    let enclosing = enclosing_class(analysis, cursor);

    match enclosing {
        // Outside any class body: every class that defines the method
        // is an implementation.
        None => {
            for cd in analysis.all_classes.values() {
                if let Some(span) = method_span(cd, &word) {
                    spans.insert((span.start(), span.end()));
                }
            }
        }
        // Inside a class body: the enclosing class's own definition plus
        // the overrides defined in its descendants.  Ancestor
        // definitions are skipped (go-to-definition reaches those).
        Some(enclosing_qname) => {
            for cd in analysis.all_classes.values() {
                let Some(span) = method_span(cd, &word) else {
                    continue;
                };
                let is_self = qname_eq(&cd.qualified_name, enclosing_qname);
                let is_descendant =
                    !is_self && is_descendant_of(analysis, &cd.qualified_name, enclosing_qname);
                if is_self || is_descendant {
                    spans.insert((span.start(), span.end()));
                }
            }
        }
    }

    finish(source, &line_index, spans)
}

/// Materialise the collected `(start, end)` byte spans into
/// `LspRange`s, in start-offset order.
fn finish(source: &str, line_index: &LineIndex, spans: BTreeSet<(u32, u32)>) -> Vec<LspRange> {
    spans
        .into_iter()
        .map(|(s, e)| span_to_range(source, line_index, Span::new(s, e)))
        .collect()
}

/// `word` names `cd` if it matches the bare name or either spelling
/// of the qualified name (with / without the leading `::`).
fn class_matches(cd: &ClassDef, word: &str) -> bool {
    cd.name == word
        || cd.qualified_name == word
        || cd.qualified_name == format!("::{word}")
        || strip_colons(&cd.qualified_name) == strip_colons(word)
}

/// Iterate a class's direct parents — superclasses then mixins.
fn parents_of(cd: &ClassDef) -> impl Iterator<Item = &str> {
    cd.superclasses
        .iter()
        .chain(cd.mixins.iter())
        .map(String::as_str)
}

/// True when `parent` equals any of `targets`, comparing on the
/// `::`-stripped tail so bare and qualified spellings unify.
fn qname_matches_any(parent: &str, targets: &[&str]) -> bool {
    let p = strip_colons(parent);
    targets.iter().any(|t| strip_colons(t) == p)
}

fn qname_eq(a: &str, b: &str) -> bool {
    strip_colons(a) == strip_colons(b)
}

fn strip_colons(name: &str) -> &str {
    name.trim_start_matches(':')
}

/// The method `name_span` for `word` on `cd`, checking instance then
/// class methods.
fn method_span(cd: &ClassDef, word: &str) -> Option<Span> {
    cd.methods
        .get(word)
        .or_else(|| cd.class_methods.get(word))
        .map(|m| m.name_span)
}

/// The qualified name of the innermost class whose `body_span`
/// contains the cursor, if any.
fn enclosing_class(analysis: &AnalysisResult, cursor: u32) -> Option<&str> {
    let mut best: Option<(&str, u32)> = None;
    for cd in analysis.all_classes.values() {
        let body = cd.body_span;
        if body.start() < cursor && cursor < body.end() {
            // Prefer the narrowest (innermost) containing body.
            let width = body.end() - body.start();
            if best.is_none_or(|(_, w)| width < w) {
                best = Some((cd.qualified_name.as_str(), width));
            }
        }
    }
    best.map(|(q, _)| q)
}

/// True when `candidate` transitively lists `ancestor` among its
/// superclasses / mixins (i.e. `candidate` is a descendant of
/// `ancestor`).  BFS over the parent edges with a visited guard.
fn is_descendant_of(analysis: &AnalysisResult, candidate: &str, ancestor: &str) -> bool {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut work: Vec<&str> = vec![candidate];
    while let Some(cur) = work.pop() {
        let key = strip_colons(cur).to_owned();
        if !seen.insert(key) {
            continue;
        }
        // Resolve the class def for `cur` (match either spelling).
        let Some(cd) = analysis
            .all_classes
            .values()
            .find(|c| qname_eq(&c.qualified_name, cur))
        else {
            continue;
        };
        for parent in parents_of(cd) {
            if qname_eq(parent, ancestor) {
                return true;
            }
            work.push(parent);
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    /// Locate the (line, character) of the first occurrence of
    /// `needle` after byte `from`, for cursor placement in tests.
    fn pos_of(src: &str, needle: &str, occurrence: usize) -> (u32, u32) {
        let mut start = 0;
        for _ in 0..occurrence {
            let idx = src[start..].find(needle).expect("needle not found") + start;
            start = idx + 1;
        }
        let idx = start - 1;
        let prefix = &src[..idx];
        let line = u32::try_from(prefix.matches('\n').count()).unwrap();
        let col = u32::try_from(idx - prefix.rfind('\n').map_or(0, |n| n + 1)).unwrap();
        (line, col)
    }

    #[test]
    fn class_name_returns_direct_subclasses() {
        let src = "oo::class create Base {}\n\
                   oo::class create Derived {\n\
                   superclass Base\n\
                   }\n\
                   oo::class create Other {}\n";
        let analysis = analyse(src);
        // Cursor on `Base` in its own definition (line 0).
        let (l, c) = pos_of(src, "Base", 1);
        let locs = implementation(src, l, c, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        // The single subclass `Derived`'s name span is on line 1.
        assert_eq!(locs[0].start_line, 1);
    }

    #[test]
    fn method_outside_class_returns_all_definers() {
        let src = "oo::class create A {\n\
                   method run {} {}\n\
                   }\n\
                   oo::class create B {\n\
                   method run {} {}\n\
                   }\n\
                   set x [a run]\n";
        let analysis = analyse(src);
        // Cursor on `run` in the top-level call (last occurrence).
        let (l, c) = pos_of(src, "run", 3);
        let locs = implementation(src, l, c, &analysis);
        // Both A::run and B::run are implementations.
        assert_eq!(locs.len(), 2, "{locs:?}");
    }

    #[test]
    fn method_inside_class_returns_self_and_descendant_overrides() {
        let src = "oo::class create Base {\n\
                   method greet {} { return hi }\n\
                   }\n\
                   oo::class create Sub {\n\
                   superclass Base\n\
                   method greet {} { return yo }\n\
                   }\n";
        let analysis = analyse(src);
        // Cursor on `greet` inside Base's method definition (1st occ).
        let (l, c) = pos_of(src, "greet", 1);
        let locs = implementation(src, l, c, &analysis);
        // Base's own def + Sub's override.
        assert_eq!(locs.len(), 2, "{locs:?}");
    }

    #[test]
    fn unknown_word_returns_empty() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        let (l, c) = pos_of(src, "hello", 1);
        assert!(implementation(src, l, c, &analysis).is_empty());
    }
}

// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

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

    let cursor = byte_offset_at(&line_index, source, line, character);

    // Case 1: the word names a class — resolve the single class the cursor
    // denotes namespace-aware (declaration under the cursor, else the
    // caller-namespace candidate order), not every same-named class across
    // namespaces, then list its direct subclasses as the implementations.
    //
    // The subclass edges come from the shared class-hierarchy index rather
    // than a local `superclasses`/`mixins` scan: those fields hold the names
    // *as written* (a bare `superclass Shape` inside `::A` stays `"Shape"`),
    // so a leading-`::`-only tail comparison never matches the resolved
    // `::A::Shape` target and every namespaced subclass is missed.  The index
    // already resolves each written super/mixin owner-aware (the same
    // `normalise` the MRO builder uses) and unions super + mixin edges, so
    // it is the single source of truth `type_hierarchy::subtypes` shares.
    if let Some((target_qname, _target)) = crate::definition::resolve_class_target_at(
        analysis,
        crate::definition::CallResolution::document_only(),
        cursor,
        &word,
    ) {
        if let Some(subs) = analysis.class_hierarchy().subclasses.get(target_qname) {
            for sub_qname in subs {
                if let Some(cd) = analysis.all_classes.get(sub_qname) {
                    spans.insert((cd.name_span.start(), cd.name_span.end()));
                }
            }
        }
        return finish(source, &line_index, spans);
    }

    // Case 2/3: the word names a method
    let enclosing = crate::definition::enclosing_class_at(analysis, cursor);

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

/// Iterate a class's direct parents — superclasses then mixins.
fn parents_of(cd: &ClassDef) -> impl Iterator<Item = &str> {
    cd.superclasses
        .iter()
        .chain(cd.mixins.iter())
        .map(String::as_str)
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

    /// Two namespaces each define a class named `Shape`, each with its own
    /// subclass.  With the cursor on `::A::Shape`, only `::A`'s subclass is an
    /// implementation — the namespace-blind scan used to pool both namespaces'
    /// subclasses together.
    fn two_namespace_shapes() -> &'static str {
        "namespace eval A {\n\
             oo::class create Shape {}\n\
             oo::class create Circle {\n\
                 superclass Shape\n\
             }\n\
         }\n\
         namespace eval B {\n\
             oo::class create Shape {}\n\
             oo::class create Square {\n\
                 superclass Shape\n\
             }\n\
         }\n"
    }

    #[test]
    fn class_in_namespace_returns_only_that_namespaces_subclass() {
        let src = two_namespace_shapes();
        let analysis = analyse(src);
        // Cursor on the first `Shape` — its declaration inside `::A`.
        let (l, c) = pos_of(src, "Shape", 1);
        let locs = implementation(src, l, c, &analysis);
        // Exactly `::A::Circle` (line 2), not `::B::Square` (line 8).
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 2, "{locs:?}");
    }

    #[test]
    fn other_namespace_class_returns_its_own_subclass() {
        let src = two_namespace_shapes();
        let analysis = analyse(src);
        // `Shape` occurrences: (1) `::A`'s decl, (2) `::A::Circle`'s
        // `superclass Shape`, (3) `::B`'s decl.  Occurrence 3 is `::B::Shape`.
        let (l, c) = pos_of(src, "Shape", 3);
        let locs = implementation(src, l, c, &analysis);
        // Exactly `::B::Square` (line 8), not `::A::Circle` (line 2).
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 8, "{locs:?}");
    }

    #[test]
    fn namespaced_class_with_no_subclasses_returns_empty() {
        // A class that no one subclasses yields no implementations — the
        // resolver must not fall back to a same-tail class elsewhere.
        let src = "namespace eval A {\n\
                       oo::class create Widget {}\n\
                   }\n\
                   namespace eval B {\n\
                       oo::class create Widget {}\n\
                       oo::class create Button {\n\
                           superclass Widget\n\
                       }\n\
                   }\n";
        let analysis = analyse(src);
        // Cursor on `::A::Widget`, which nothing subclasses.
        let (l, c) = pos_of(src, "Widget", 1);
        let locs = implementation(src, l, c, &analysis);
        assert!(locs.is_empty(), "{locs:?}");
    }

    #[test]
    fn namespaced_subclass_via_mixin_is_an_implementation() {
        // The subclass map unions superclass and mixin edges, so a mixin of a
        // namespaced class counts as an implementation the same way a
        // subclass does.
        let src = "namespace eval A {\n\
                       oo::class create Trait {}\n\
                       oo::class create User {\n\
                           mixin Trait\n\
                       }\n\
                   }\n";
        let analysis = analyse(src);
        let (l, c) = pos_of(src, "Trait", 1);
        let locs = implementation(src, l, c, &analysis);
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].start_line, 2, "{locs:?}");
    }
}

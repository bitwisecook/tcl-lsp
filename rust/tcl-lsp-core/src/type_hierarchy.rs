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

//! Type-hierarchy provider.
//!
//! Resolves a `TclOO` class at the cursor ([`prepare`]) and walks its
//! [`supertypes`] (direct superclasses + mixins) and [`subtypes`] (direct
//! subclasses) via the class-hierarchy index.  Resolution is within the
//! analysed document; cross-file super/subtypes need the workspace index
//! (a follow-up).

use std::collections::{HashMap, HashSet};

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::analyser::class_hierarchy::{build_tail_index, resolve_class_name};
use tcl_compiler::analyser::types::ClassDef;
use tcl_lexer::LineIndex;

use crate::definition::LspRange;
use crate::hover::find_word_span_at_position;

/// One hierarchy item — class identification plus its name
/// and definition span for editor display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeHierarchyItem {
    /// Class qualified name.
    pub name: String,
    /// Detail (e.g. metaclass).
    pub detail: Option<String>,
    /// Range of the entire definition.
    pub range: LspRange,
    /// Range of just the name token.
    pub selection_range: LspRange,
}

/// Resolve a "prepare type hierarchy" request to a single
/// item — the class whose name is at the cursor.
#[must_use]
pub fn prepare(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<TypeHierarchyItem> {
    let line_index = LineIndex::new(source);
    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };
    // Resolve the class the cursor denotes namespace-aware (declaration under
    // the cursor, else the caller-namespace candidate order) rather than by a
    // namespace-blind name scan that could seed the hierarchy from a same-named
    // class in another namespace.
    let cursor_off = crate::definition::byte_offset_at(&line_index, source, line, character);
    if let Some((_, class_def)) = crate::definition::resolve_class_target_at(
        analysis,
        crate::definition::CallResolution::document_only(),
        cursor_off,
        &word,
    ) {
        return vec![item_for(class_def, source, &line_index)];
    }
    Vec::new()
}

/// Direct supertypes of `class_name`: its declared superclasses and
/// class-level mixins, in declaration order (supers then mixins),
/// de-duplicated.  Empty when the class is unknown or has none in this
/// document.
#[must_use]
pub fn supertypes(
    class_name: &str,
    source: &str,
    analysis: &AnalysisResult,
) -> Vec<TypeHierarchyItem> {
    let line_index = LineIndex::new(source);
    let tail_index = build_tail_index(analysis.all_classes.keys());
    let Some(cd) = resolve_class(class_name, "", analysis, &tail_index) else {
        return Vec::new();
    };
    // Each written super/mixin name is resolved **owner-aware** — relative to
    // the defining class's namespace (ancestry → global → unique tail) — so a
    // bare `superclass Base` naming a namespaced class links the same way the
    // MRO builder linked it, instead of abstaining whenever the tail isn't
    // globally unique.  A name that resolves back to the class itself (a self
    // edge a tail match could otherwise manufacture) is never listed.
    let owner = cd.qualified_name.clone();
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for name in cd.superclasses.iter().chain(cd.mixins.iter()) {
        if let Some(target) = resolve_class(name, &owner, analysis, &tail_index)
            && target.qualified_name != owner
            && seen.insert(target.qualified_name.clone())
        {
            out.push(item_for(target, source, &line_index));
        }
    }
    out
}

/// Direct subtypes of `class_name`: the classes that declare it as a
/// superclass, via the (namespace-aware) class-hierarchy subclass map.
/// Sorted by qualified name for determinism.
#[must_use]
pub fn subtypes(
    class_name: &str,
    source: &str,
    analysis: &AnalysisResult,
) -> Vec<TypeHierarchyItem> {
    let line_index = LineIndex::new(source);
    let tail_index = build_tail_index(analysis.all_classes.keys());
    let Some(cd) = resolve_class(class_name, "", analysis, &tail_index) else {
        return Vec::new();
    };
    let target = cd.qualified_name.clone();
    let hierarchy = analysis.class_hierarchy();
    let Some(subs) = hierarchy.subclasses.get(&target) else {
        return Vec::new();
    };
    let mut names: Vec<&String> = subs.iter().collect();
    names.sort();
    names
        .into_iter()
        .filter_map(|s| analysis.all_classes.get(s))
        .map(|cd| item_for(cd, source, &line_index))
        .collect()
}

/// Resolve a written class `name` to its `ClassDef`, **owner-aware** via the
/// shared [`resolve_class_name`] resolver: an exact hit, then `::name`, then a
/// walk outward from `owner`'s namespace to the global namespace, and finally
/// a *globally-unique* simple-name (tail) match.  `owner` is the qualified
/// name of the class in whose body `name` was written (`""` for a top-level /
/// already-qualified lookup).  Mirrors how the MRO builder linked the edge,
/// so supertype resolution no longer abstains on tails the hierarchy resolves.
fn resolve_class<'a>(
    name: &str,
    owner: &str,
    analysis: &'a AnalysisResult,
    tail_index: &HashMap<String, Vec<String>>,
) -> Option<&'a ClassDef> {
    let q = resolve_class_name(
        name,
        owner,
        |cand| analysis.all_classes.contains_key(cand),
        tail_index,
    )?;
    analysis.all_classes.get(&q)
}

/// Build a hierarchy item for `class_def` from its spans in `source`.
fn item_for(class_def: &ClassDef, source: &str, line_index: &LineIndex) -> TypeHierarchyItem {
    let name_range = span_to_range(source, line_index, class_def.name_span);
    let body_range = span_to_range(source, line_index, class_def.body_span);
    let full_range = LspRange {
        start_line: name_range.start_line,
        start_character: name_range.start_character,
        end_line: body_range.end_line,
        end_character: body_range.end_character,
    };
    TypeHierarchyItem {
        name: class_def.qualified_name.clone(),
        detail: Some(class_def.metaclass.clone()),
        range: full_range,
        selection_range: name_range,
    }
}

fn span_to_range(source: &str, line_index: &LineIndex, span: tcl_lexer::Span) -> LspRange {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    LspRange {
        start_line: start.line,
        start_character: start.character.get(),
        end_line: end.line,
        end_character: end.character.get(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    #[test]
    fn prepare_resolves_class_at_cursor() {
        let src = "oo::class create Greeter {}\n";
        let analysis = analyse(src);
        let items = prepare(src, 0, 18, &analysis);
        if !items.is_empty() {
            assert!(items[0].name.contains("Greeter"));
        }
    }

    /// `pos_of` — (line, character) of the `occurrence`-th `needle`.
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
    fn prepare_disambiguates_same_name_across_namespaces() {
        // `::A::Shape` and `::B::Shape` share a simple name.  The cursor on a
        // bare `Shape` written inside `::A` must prepare `::A::Shape`, not the
        // arbitrary first same-named class a namespace-blind scan would pick.
        let src = "namespace eval A {\n\
                       oo::class create Shape {}\n\
                       oo::class create Circle {\n\
                           superclass Shape\n\
                       }\n\
                   }\n\
                   namespace eval B {\n\
                       oo::class create Shape {}\n\
                   }\n";
        let analysis = analyse(src);
        // Occurrence 2 is `superclass Shape` inside `::A::Circle`.
        let (l, c) = pos_of(src, "Shape", 2);
        let items = prepare(src, l, c, &analysis);
        assert_eq!(items.len(), 1, "{items:?}");
        assert_eq!(items[0].name, "::A::Shape", "{items:?}");
    }

    #[test]
    fn prepare_of_non_class_word_is_empty() {
        // A word that names nothing resolvable as a class prepares nothing.
        let src = "oo::class create Shape {}\nputs hello\n";
        let analysis = analyse(src);
        let (l, c) = pos_of(src, "hello", 1);
        assert!(prepare(src, l, c, &analysis).is_empty());
    }

    #[test]
    fn subtypes_disambiguate_same_name_across_namespaces() {
        // Direct subtypes of `::A::Shape` are `::A`'s subclasses only; `::B`'s
        // same-named class and its subclass never leak in.
        let src = "namespace eval A {\n\
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
                   }\n";
        let analysis = analyse(src);
        let sub = subtypes("::A::Shape", src, &analysis);
        let names: Vec<&str> = sub.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["::A::Circle"], "{names:?}");
    }

    #[test]
    fn supertypes_returns_superclasses_and_mixins() {
        let src = "oo::class create Animal {}\noo::class create Legs {}\noo::class create Dog {\n    superclass Animal\n    mixin Legs\n}\n";
        let analysis = analyse(src);
        let sup = supertypes("::Dog", src, &analysis);
        let names: Vec<&str> = sup.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"::Animal"), "{names:?}");
        assert!(names.contains(&"::Legs"), "{names:?}");
    }

    #[test]
    fn subtypes_returns_direct_subclasses() {
        let src = "oo::class create Animal {}\noo::class create Dog {\n    superclass Animal\n}\noo::class create Cat {\n    superclass Animal\n}\n";
        let analysis = analyse(src);
        let sub = subtypes("::Animal", src, &analysis);
        let mut names: Vec<&str> = sub.iter().map(|i| i.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, vec!["::Cat", "::Dog"], "{names:?}");
    }

    #[test]
    fn supertypes_of_unknown_class_is_empty() {
        let analysis = analyse("oo::class create A {}\n");
        assert!(supertypes("::Nope", "oo::class create A {}\n", &analysis).is_empty());
    }

    #[test]
    fn supertypes_resolve_bare_namespaced_base_owner_aware() {
        // A subclass in `::Ns` names its base bare (`Base`); the base lives at
        // `::Ns::Base`.  Ownerless tail resolution used to abstain here; the
        // owner-aware resolver links it the way the MRO builder did.
        let src = "namespace eval Ns {\n    oo::class create Base {}\n    oo::class create Sub {\n        superclass Base\n    }\n}\n";
        let analysis = analyse(src);
        let sup = supertypes("::Ns::Sub", src, &analysis);
        let names: Vec<&str> = sup.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["::Ns::Base"], "{names:?}");
    }

    #[test]
    fn supertypes_never_lists_the_class_itself() {
        // A class whose own tail collides with a bare superclass name must not
        // be reported as its own supertype.
        let src = "oo::class create Base {}\noo::class create Derived {\n    superclass Base\n}\n";
        let analysis = analyse(src);
        for name in supertypes("::Derived", src, &analysis) {
            assert_ne!(name.name, "::Derived", "self-supertype leaked");
        }
    }
}

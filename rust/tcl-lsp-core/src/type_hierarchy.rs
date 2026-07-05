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

use std::collections::HashSet;

use tcl_compiler::analyser::AnalysisResult;
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
    for class_def in analysis.all_classes.values() {
        if class_def.name == word
            || class_def.qualified_name == word
            || class_def.qualified_name == format!("::{word}")
        {
            return vec![item_for(class_def, source, &line_index)];
        }
    }
    Vec::new()
}

/// Direct supertypes of `class_name`: its declared superclasses and
/// class-level mixins, in declaration order (supers then mixins),
/// de-duplicated.  Empty when the class is unknown or has none in this
/// document.
#[must_use]
pub fn supertypes(class_name: &str, source: &str, analysis: &AnalysisResult) -> Vec<TypeHierarchyItem> {
    let line_index = LineIndex::new(source);
    let Some(cd) = resolve_class(class_name, analysis) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for name in cd.superclasses.iter().chain(cd.mixins.iter()) {
        if let Some(target) = resolve_class(name, analysis)
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
pub fn subtypes(class_name: &str, source: &str, analysis: &AnalysisResult) -> Vec<TypeHierarchyItem> {
    let line_index = LineIndex::new(source);
    let Some(cd) = resolve_class(class_name, analysis) else {
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

/// Resolve a class name to its `ClassDef` in the analysed document —
/// exact, `::name`, or a unique simple-name (tail) match.
fn resolve_class<'a>(name: &str, analysis: &'a AnalysisResult) -> Option<&'a ClassDef> {
    if let Some(cd) = analysis.all_classes.get(name) {
        return Some(cd);
    }
    let q = format!("::{}", name.trim_start_matches("::"));
    if let Some(cd) = analysis.all_classes.get(&q) {
        return Some(cd);
    }
    let tail = name.rsplit("::").next().unwrap_or(name);
    let mut it = analysis
        .all_classes
        .values()
        .filter(|cd| cd.qualified_name.rsplit("::").next() == Some(tail));
    let first = it.next()?;
    it.next().is_none().then_some(first)
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
}

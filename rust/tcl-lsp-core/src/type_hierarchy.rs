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
//! current document or an independently retained workspace inventory. Original
//! declarations use exact source and namespace geometry; logical-only profiles
//! retain the reporting hierarchy view.

use std::collections::{HashMap, HashSet};

use crate::original_declaration::OriginalDeclarationIdentity;
use std::ops::ControlFlow;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::analyser::class_hierarchy::build_tail_index;
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
    /// Retained readonly source identity; display names cannot replace it.
    pub original_declaration: Option<OriginalDeclarationIdentity>,
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
    prepare_for_document("", source, line, character, analysis)
}

/// Prepare a hierarchy identity for an independently selected document owner.
#[must_use]
pub fn prepare_for_document(
    uri: &str,
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<TypeHierarchyItem> {
    let line_index = LineIndex::new(source);
    let cursor = crate::definition::byte_offset_at(&line_index, source, line, character);
    match crate::original_oo::class_at_cursor(analysis, source, cursor) {
        ControlFlow::Break(selected) => {
            return selected
                .and_then(|record| original_item_for(uri, source, analysis, record))
                .into_iter()
                .collect();
        }
        ControlFlow::Continue(()) if !analysis.allows_lexical_declaration_advice() => {
            return Vec::new();
        }
        ControlFlow::Continue(()) => {}
    }
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
        source,
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
    if !analysis.allows_lexical_declaration_advice() {
        return original_for_literal_name(class_name, source, analysis).map_or_else(
            Vec::new,
            |identity| {
                related_from_inventory(
                    &identity,
                    &[OriginalHierarchyDocument {
                        uri: "",
                        source,
                        analysis,
                    }],
                    false,
                )
            },
        );
    }
    let line_index = LineIndex::new(source);
    let tail_index = build_tail_index(analysis.all_classes.keys());
    let Some(cd) = resolve_class(class_name, "", analysis, &tail_index) else {
        return Vec::new();
    };
    // Logical-only profiles retain their source relation lookup geometry.
    // Same-tail names in other namespaces cannot replace that lookup.
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
    if !analysis.allows_lexical_declaration_advice() {
        return original_for_literal_name(class_name, source, analysis).map_or_else(
            Vec::new,
            |identity| {
                related_from_inventory(
                    &identity,
                    &[OriginalHierarchyDocument {
                        uri: "",
                        source,
                        analysis,
                    }],
                    true,
                )
            },
        );
    }
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

/// Resolve logical-only reporting names using retained relation coordinates.
fn resolve_class<'a>(
    name: &str,
    owner: &str,
    analysis: &'a AnalysisResult,
    tail_index: &HashMap<String, Vec<String>>,
) -> Option<&'a ClassDef> {
    let _ = tail_index;
    let q = if owner.is_empty() {
        tcl_compiler::analyser::class_hierarchy::resolve_written_class_name(
            name,
            &analysis.all_classes,
        )?
    } else {
        let lookup = analysis
            .all_classes
            .get(owner)?
            .relation_lookups
            .get(name)?
            .as_ref()?;
        tcl_compiler::analyser::class_hierarchy::resolve_class_lookup(
            lookup,
            &analysis.all_classes,
        )?
    };
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
        original_declaration: None,
    }
}

/// One current source owner in a readonly hierarchy inventory. Workspace
/// membership supplies source advice, independently of script loading order.
pub type OriginalHierarchyDocument<'a> =
    crate::original_declaration::OriginalDeclarationDocument<'a>;

fn original_item_for(
    uri: &str,
    source: &str,
    analysis: &AnalysisResult,
    record: &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<ClassDef>,
) -> Option<TypeHierarchyItem> {
    let identity = OriginalDeclarationIdentity::for_class(uri, source, analysis, record)?;
    let index = LineIndex::new(source);
    let mut item = item_for(record.metadata(), source, &index);
    // Reporting remains optional and follows selection. Opaque native names
    // use the shared source renderer rather than colliding replacement text.
    if std::str::from_utf8(record.name_input().bytes()).is_err() {
        item.name = identity.label();
    }
    item.selection_range = span_to_range(source, &index, identity.span());
    item.original_declaration = Some(identity);
    Some(item)
}

fn original_for_literal_name(
    name: &str,
    source: &str,
    analysis: &AnalysisResult,
) -> Option<OriginalDeclarationIdentity> {
    let mut classes = analysis.original_class_declarations().filter(|record| {
        crate::original_declaration::literal_name_matches_publication(name, record.name())
            == Some(true)
    });
    let record = classes.next()?;
    if classes.next().is_some()
        || analysis.original_procedure_declarations().any(|record| {
            crate::original_declaration::literal_name_matches_publication(name, record.name())
                == Some(true)
        })
    {
        return None;
    }
    OriginalDeclarationIdentity::for_class("", source, analysis, record)
}

/// Walk direct superclass/mixin declarations through the shared original
/// relation kernel. Exact source owners survive equal-byte documents and
/// display collisions. Missing, duplicate or earlier non-class publications
/// cannot select a class. This view supplies no runtime MRO or installation.
#[must_use]
pub fn related_from_inventory(
    identity: &OriginalDeclarationIdentity,
    documents: &[OriginalHierarchyDocument<'_>],
    subtypes: bool,
) -> Vec<TypeHierarchyItem> {
    use tcl_compiler::analyser::class_hierarchy::original_metadata::original_direct_metadata_relations_for_records;
    use tcl_compiler::analyser::types::OriginalSourceClassRelationKind;
    if identity.role() != crate::original_declaration::OriginalDeclarationRole::Class {
        return Vec::new();
    }
    let Some(inventory) =
        crate::original_declaration::OriginalClassInventory::from_documents(documents)
    else {
        return Vec::new();
    };
    let records = inventory.records();
    let occupied = inventory.occupied_non_classes();
    let roots = records
        .iter()
        .enumerate()
        .filter(|(index, record)| {
            inventory.owner(*index).is_some_and(|owner| {
                identity.is_current(owner.uri, owner.source, owner.analysis)
                    && identity.class_metadata(owner.analysis) == Some(**record)
            })
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let [root] = roots.as_slice() else {
        return Vec::new();
    };
    let direct = |index| -> Option<Vec<usize>> {
        let mut selected = original_direct_metadata_relations_for_records(
            records,
            index,
            OriginalSourceClassRelationKind::Superclass,
            occupied,
        )?;
        selected.extend(original_direct_metadata_relations_for_records(
            records,
            index,
            OriginalSourceClassRelationKind::Mixin,
            occupied,
        )?);
        selected.retain(|target| *target != index);
        let mut seen = HashSet::new();
        selected.retain(|target| seen.insert(*target));
        Some(selected)
    };
    let selected = if subtypes {
        (0..records.len())
            .filter(|index| *index != *root)
            .filter(|index| direct(*index).is_some_and(|targets| targets.contains(root)))
            .collect::<Vec<_>>()
    } else {
        direct(*root).unwrap_or_default()
    };
    selected
        .into_iter()
        .filter_map(|index| {
            let owner = inventory.owner(index)?;
            original_item_for(owner.uri, owner.source, owner.analysis, records[index])
        })
        .collect()
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
        assert_eq!(items.len(), 1);
        assert!(items[0].name.contains("Greeter"));
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
        // `::Ns::Base`.  Ownerless tail resolution abstains here; the
        // owner-aware resolver links it the way the MRO builder does.
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

#[cfg(test)]
mod original_hierarchy_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn cleared(source: &str) -> AnalysisResult {
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_classes.clear();
        analysis.superseded_classes.clear();
        analysis.all_procs.clear();
        analysis
    }

    #[test]
    fn original_hierarchy_uses_exact_opaque_relations_and_current_source_identity() {
        // Contract: naming.consumer.original-type-hierarchy
        // (docs/design/analysis/name-resolution-proofs/original-type-hierarchy.md).
        let source = "oo::class create B\\uD800 {}\noo::class create B\\uD801 {}\noo::class create C {superclass B\\uD800; mixin B\\uD801}\n";
        let analysis = cleared(source);
        let items = prepare_for_document("file:///classes.tcl", source, 2, 17, &analysis);
        assert_eq!(items.len(), 1);
        let identity = items[0].original_declaration.as_ref().unwrap();
        let documents = [OriginalHierarchyDocument {
            uri: "file:///classes.tcl",
            source,
            analysis: &analysis,
        }];
        let parents = related_from_inventory(identity, &documents, false);
        assert_eq!(parents.len(), 2);
        assert_ne!(parents[0].name, parents[1].name);
        assert_eq!(
            parents
                .iter()
                .map(|item| item
                    .original_declaration
                    .as_ref()
                    .unwrap()
                    .input()
                    .unwrap()
                    .bytes())
                .collect::<Vec<_>>(),
            vec![b"B\xed\xa0\x80".as_slice(), b"B\xed\xa0\x81".as_slice()]
        );
        let changed = format!("# changed\n{source}");
        assert!(
            related_from_inventory(
                identity,
                &[OriginalHierarchyDocument {
                    uri: "file:///classes.tcl",
                    source: &changed,
                    analysis: &analysis
                }],
                false
            )
            .is_empty()
        );
        let mut without_config = analysis.clone();
        without_config.body_lexer_config = None;
        assert!(
            prepare_for_document("file:///classes.tcl", source, 2, 17, &without_config).is_empty()
        );
    }

    #[test]
    fn original_hierarchy_preserves_document_owners_and_refuses_duplicate_providers() {
        // Implementation contract: naming.consumer.original-type-hierarchy
        // docs/design/analysis/name-resolution-proofs/original-type-hierarchy.md
        let base_source = "oo::class create Base {}\n";
        let child_source = "oo::class create Child {superclass Base}\n";
        let base = cleared(base_source);
        let child = cleared(child_source);
        let item = prepare_for_document("file:///child.tcl", child_source, 0, 18, &child).remove(0);
        let identity = item.original_declaration.as_ref().unwrap();
        let documents = [
            OriginalHierarchyDocument {
                uri: "file:///base.tcl",
                source: base_source,
                analysis: &base,
            },
            OriginalHierarchyDocument {
                uri: "file:///child.tcl",
                source: child_source,
                analysis: &child,
            },
        ];
        let parents = related_from_inventory(identity, &documents, false);
        assert_eq!(parents.len(), 1);
        let base_identity = parents[0].original_declaration.as_ref().unwrap();
        assert_eq!(base_identity.uri(), "file:///base.tcl");
        assert_eq!(
            related_from_inventory(base_identity, &documents, true)[0]
                .original_declaration
                .as_ref()
                .unwrap()
                .uri(),
            "file:///child.tcl"
        );
        let duplicates = [
            OriginalHierarchyDocument {
                uri: "file:///base.tcl",
                source: base_source,
                analysis: &base,
            },
            OriginalHierarchyDocument {
                uri: "file:///copy.tcl",
                source: base_source,
                analysis: &base,
            },
            OriginalHierarchyDocument {
                uri: "file:///child.tcl",
                source: child_source,
                analysis: &child,
            },
        ];
        assert!(related_from_inventory(identity, &duplicates, false).is_empty());
    }

    #[test]
    fn original_hierarchy_earlier_nonclass_headers_block_later_class_candidates() {
        // Implementation contract: naming.consumer.original-type-hierarchy
        // docs/design/analysis/name-resolution-proofs/original-type-hierarchy.md
        let source = "oo::class create Base {}\nnamespace eval N {proc Base {} {}; oo::class create Child {superclass Base}}\n";
        let analysis = cleared(source);
        let record = analysis
            .original_class_declarations()
            .find(|record| record.name_input().bytes() == b"Child")
            .unwrap();
        let identity = OriginalDeclarationIdentity::for_class(
            "file:///classes.tcl",
            source,
            &analysis,
            record,
        )
        .unwrap();
        assert!(
            related_from_inventory(
                &identity,
                &[OriginalHierarchyDocument {
                    uri: "file:///classes.tcl",
                    source,
                    analysis: &analysis
                }],
                false
            )
            .is_empty()
        );
        let offset = u32::try_from(source.rfind("Base").unwrap()).unwrap();
        assert!(matches!(
            crate::original_oo::class_at_cursor(&analysis, source, offset),
            ControlFlow::Break(None)
        ));
    }
}

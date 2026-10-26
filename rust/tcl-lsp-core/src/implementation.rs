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
//! - **Cursor on a retained method declaration or selector** → its own
//!   implementation and descendant overrides selected by exact relation inputs.
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

use tcl_compiler::analyser::AnalysisResult;
#[cfg(test)]
use tcl_compiler::analyser::ClassDef;
use tcl_lexer::{LineIndex, Span};

use crate::definition::{LspRange, byte_offset_at, span_to_range};

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
    if crate::original_context::CurrentSourcePosition::capture(source, analysis, line, character)
        .is_none()
    {
        return Vec::new();
    }
    if !analysis.allows_lexical_declaration_advice() {
        return original_implementation(source, line, character, analysis);
    }
    logical_implementation_advice("", source, line, character, analysis)
        .map_or_else(Vec::new, |advice| advice.possible_source_ranges().to_vec())
}

/// Current possible-positive implementation result with retained applicability.
/// It supplies no complete inheritance graph, dispatch or lookup absence.
#[derive(Debug, Clone)]
pub struct LogicalImplementationAdvice {
    uri: String,
    image: tcl_lexer::SourceImage,
    input: tcl_compiler::analyser::ResolvedAnalysisInput,
    root: LogicalImplementationRoot,
    targets: Vec<LogicalImplementationRoot>,
    ranges: Vec<LspRange>,
    incomplete: bool,
}
#[derive(Debug, Clone)]
enum LogicalImplementationRoot {
    Class(Box<crate::type_hierarchy::LogicalHierarchyClass>),
    Member(Box<tcl_compiler::analyser::types::LogicalSourceMemberDeclaration>),
}
impl LogicalImplementationAdvice {
    /// Revalidate the independent current document and selected source root.
    #[must_use]
    pub fn is_current(&self, uri: &str, source: &str, analysis: &AnalysisResult) -> bool {
        if uri != self.uri
            || crate::original_context::CurrentSourceContext::capture(source, analysis).is_none()
            || self.image != tcl_lexer::SourceImage::document(source)
            || analysis.resolved_input.as_ref() != Some(&self.input)
        {
            return false;
        }
        std::iter::once(&self.root)
            .chain(&self.targets)
            .all(|target| match target {
                LogicalImplementationRoot::Class(identity) => {
                    identity.is_current(uri, source, analysis)
                }
                LogicalImplementationRoot::Member(member) => {
                    analysis.logical_source_member_class(member).is_some()
                }
            })
    }
    /// Borrow possible positive ranges under the host's final current-source
    /// lease. An empty projection establishes no absence or graph completeness.
    #[must_use]
    pub fn possible_source_ranges(&self) -> &[LspRange] {
        &self.ranges
    }
    /// Earlier mutation or unavailable relation coverage remains an obligation.
    #[must_use]
    pub const fn has_unresolved_applicability(&self) -> bool {
        self.incomplete
    }
}

/// Prepare typed conditional implementation advice for final readonly publication.
#[must_use]
pub fn logical_implementation_advice(
    uri: &str,
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<LogicalImplementationAdvice> {
    let position =
        crate::original_context::CurrentSourcePosition::capture(source, analysis, line, character)?;
    analysis
        .allows_retained_logical_declaration_advice()
        .then_some(())?;
    let image = tcl_lexer::SourceImage::document(source);
    let index = LineIndex::new_lsp(source);
    let mut spans = BTreeSet::new();
    if let Some(selection) =
        crate::type_hierarchy::logical_class_at(uri, source, analysis, position.offset())
    {
        let target =
            crate::type_hierarchy::LogicalHierarchyClass::class(&selection, source, analysis)?;
        let classes: crate::type_hierarchy::LogicalImplementationClasses<'_> =
            crate::type_hierarchy::logical_implementation_classes(source, analysis, target, false)?;
        let mut targets = Vec::new();
        for class in classes.classes {
            targets.push(LogicalImplementationRoot::Class(Box::new(
                crate::type_hierarchy::LogicalHierarchyClass::capture(
                    uri, source, analysis, class,
                )?,
            )));
            spans.insert((class.name_span.start(), class.name_span.end()));
        }
        let incomplete = classes.incomplete || selection.has_unresolved_applicability();
        return Some(LogicalImplementationAdvice {
            uri: uri.to_owned(),
            image,
            input: analysis.resolved_input.as_ref()?.clone(),
            root: LogicalImplementationRoot::Class(Box::new(selection)),
            targets,
            ranges: finish(source, &index, spans),
            incomplete,
        });
    }
    let member = analysis.logical_source_member_at(
        &image,
        position.context().config(),
        position.offset(),
    )?;
    let root = analysis.logical_source_member_class(&member)?;
    let name = member
        .original_name_word()
        .and_then(tcl_syntax::word_rules::original_static_word_unicode_value)?;
    let (mut descendants, incomplete) = if member.side()
        == tcl_compiler::analyser::MemberSide::Instance
    {
        let classes: crate::type_hierarchy::LogicalImplementationClasses<'_> =
            crate::type_hierarchy::logical_implementation_classes(source, analysis, root, true)?;
        (classes.classes, classes.incomplete)
    } else {
        (Vec::new(), false)
    };
    let mut targets = Vec::new();
    descendants.push(root);
    for class in descendants {
        let owner = analysis.logical_source_class_declaration(class)?;
        let methods = match member.side() {
            tcl_compiler::analyser::MemberSide::Instance => &class.methods,
            tcl_compiler::analyser::MemberSide::ClassObject => &class.class_methods,
        };
        for method in methods.values() {
            let candidate = owner.member(member.side(), method)?;
            if candidate
                .original_name_word()
                .and_then(tcl_syntax::word_rules::original_static_word_unicode_value)
                .as_ref()
                == Some(&name)
            {
                targets.push(LogicalImplementationRoot::Member(Box::new(candidate)));
                spans.insert((method.name_span.start(), method.name_span.end()));
            }
        }
    }
    Some(LogicalImplementationAdvice {
        uri: uri.to_owned(),
        image,
        input: analysis.resolved_input.as_ref()?.clone(),
        root: LogicalImplementationRoot::Member(Box::new(member)),
        targets,
        ranges: finish(source, &index, spans),
        incomplete,
    })
}

// Native queries use the same declaration/selector owners as navigation.
// A bare matching argument or reporting MRO cannot identify an implementation.
fn original_implementation(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<LspRange> {
    use std::ops::ControlFlow;
    let index = LineIndex::new(source);
    let cursor = byte_offset_at(&index, source, line, character);
    let Some(config) = analysis.body_lexer_config else {
        return Vec::new();
    };
    let image = tcl_lexer::SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config)
        || analysis
            .original_variable_root_in_source(&image, config, cursor)
            .is_some()
    {
        return Vec::new();
    }
    match crate::original_oo::class_at_cursor(analysis, source, cursor) {
        ControlFlow::Break(Some(class)) => {
            let Some(identity) =
                crate::original_declaration::OriginalDeclarationIdentity::for_class(
                    "", source, analysis, class,
                )
            else {
                return Vec::new();
            };
            return crate::type_hierarchy::related_from_inventory(
                &identity,
                &[crate::type_hierarchy::OriginalHierarchyDocument {
                    uri: "",
                    source,
                    analysis,
                }],
                true,
            )
            .into_iter()
            .map(|item| item.selection_range)
            .collect();
        }
        ControlFlow::Break(None) => return Vec::new(),
        ControlFlow::Continue(()) => {}
    }
    let ControlFlow::Break(Some(method)) =
        crate::method_symbol::local_candidate(source, analysis, line, character)
    else {
        return Vec::new();
    };
    method_implementations_in_inventory(
        &method,
        &[crate::original_declaration::OriginalDeclarationDocument {
            uri: "",
            source,
            analysis,
        }],
    )
    .into_iter()
    .map(|target| target.range)
    .collect()
}

/// One readonly implementation target with independent document ownership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalImplementationTarget {
    /// Full current declaration identity; its label is not a lookup operand.
    pub identity: crate::original_declaration::OriginalDeclarationIdentity,
    /// Canonical method's source range in that identity's document.
    pub range: LspRange,
}

/// Select own declarations and instance descendant overrides from complete
/// current source inventories. The shared original relation kernel determines
/// ancestry; class-object methods use only their declaring class's own table.
/// Source relationships do not establish a live native MRO or dispatch entry.
#[must_use]
pub fn method_implementations_in_inventory(
    method: &crate::method_symbol::OriginalMethodCandidate,
    documents: &[crate::original_declaration::OriginalDeclarationDocument<'_>],
) -> Vec<OriginalImplementationTarget> {
    use crate::original_declaration::OriginalDeclarationIdentity;
    let Some(inventory) =
        crate::original_declaration::OriginalClassInventory::from_documents(documents)
    else {
        return Vec::new();
    };
    let records = inventory.records();
    let occupied = inventory.occupied_non_classes();
    let selected = records
        .iter()
        .enumerate()
        .filter(|(index, class)| {
            inventory.owner(*index).is_some_and(|owner| {
                owner.uri == method.uri()
                    && method.declaring_class_in(owner.source, owner.analysis) == Some(**class)
            })
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let [declaring_class] = selected.as_slice() else {
        return Vec::new();
    };
    let mut targets = Vec::new();
    for (root, class) in records.iter().enumerate() {
        let order =
            if method.method().side() == tcl_compiler::analyser::types::MemberSide::ClassObject {
                if root != *declaring_class {
                    continue;
                }
                vec![root]
            } else {
                let Some(order) = tcl_compiler::analyser::class_hierarchy::original_metadata::
                original_instance_metadata_order_for_inventory(records, root, occupied)
            else { continue; };
                order
            };
        if !order.contains(declaring_class) {
            continue;
        }
        let Some(own_methods) = class
            .metadata()
            .original_members
            .methods(method.method().side())
        else {
            continue;
        };
        let Some(owner) = inventory.owner(root) else {
            return Vec::new();
        };
        let index = LineIndex::new(owner.source);
        for own in own_methods {
            if own.original_name_input().policy() != method.method().original_name_input().policy()
                || own.original_name_input().bytes()
                    != method.method().original_name_input().bytes()
            {
                continue;
            }
            let Some(identity) = OriginalDeclarationIdentity::for_method(
                owner.uri,
                owner.source,
                owner.analysis,
                class,
                &own,
            ) else {
                continue;
            };
            if targets
                .iter()
                .any(|target: &OriginalImplementationTarget| target.identity == identity)
            {
                continue;
            }
            targets.push(OriginalImplementationTarget {
                range: span_to_range(
                    owner.source,
                    &index,
                    own.declaration().original_word().span(),
                ),
                identity,
            });
        }
    }
    targets
}

/// Materialise the collected `(start, end)` byte spans into
/// `LspRange`s, in start-offset order.
fn finish(source: &str, line_index: &LineIndex, spans: BTreeSet<(u32, u32)>) -> Vec<LspRange> {
    spans
        .into_iter()
        .map(|(s, e)| span_to_range(source, line_index, Span::new(s, e)))
        .collect()
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
    fn unknown_command_argument_cannot_supply_method_implementation_identity() {
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
        // The unresolved lowercase command is not an original OO receiver;
        // its same-spelled argument cannot identify either source method.
        assert!(locs.is_empty(), "{locs:?}");
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
    /// implementation — a namespace-blind scan would pool both namespaces'
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

    #[test]
    fn logical_method_implementations_use_resolved_bare_relations_in_nested_namespace() {
        for relation in ["superclass", "mixin"] {
            let source = format!(
                "namespace eval Outer {{
namespace eval Inner {{
\
                 oo::class create Base {{method run {{}} {{}}}}
\
                 oo::class create Child {{{relation} Base; method run {{}} {{}}}}
\
                 oo::class create Grandchild {{superclass Child; method run {{}} {{}}}}
\
                 }}
}}
"
            );
            let analysis = Analyser::new().analyse(&source, "tcl");
            assert!(analysis.allows_lexical_declaration_advice());
            let (line, character) = pos_of(&source, "run", 1);
            let locations = implementation(&source, line, character, &analysis);
            assert_eq!(
                locations
                    .iter()
                    .map(|location| location.start_line)
                    .collect::<Vec<_>>(),
                vec![2, 3, 4],
                "{relation}: {locations:?}"
            );
        }
    }

    #[test]
    fn logical_method_implementations_do_not_join_equal_tail_class_owners() {
        let source = "namespace eval A {
\
            oo::class create Base {method run {} {}}
\
            }
\
            namespace eval B {
\
            oo::class create Base {method run {} {}}
\
            oo::class create Child {superclass Base; method run {} {}}
\
            }
";
        let analysis = Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_lexical_declaration_advice());
        for (occurrence, expected) in [(1, vec![1]), (2, vec![4, 5])] {
            let (line, character) = pos_of(source, "run", occurrence);
            let locations = implementation(source, line, character, &analysis);
            assert_eq!(
                locations
                    .iter()
                    .map(|location| location.start_line)
                    .collect::<Vec<_>>(),
                expected,
                "{locations:?}"
            );
        }
    }

    #[test]
    fn constructed_implementation_ancestry_keeps_root_marker_distinct_from_colon_segment() {
        // Explicit metadata graph only, under the existing lexical caller gate.
        let mut analysis = Analyser::new().analyse("", "tcl");
        assert!(analysis.allows_lexical_declaration_advice());
        for (key, parents) in [
            ("::茶", vec![]),
            (":::茶", vec![]),
            ("::leaf", vec![":::茶"]),
        ] {
            analysis.all_classes.insert(
                key.to_owned(),
                ClassDef {
                    qualified_name: key.to_owned(),
                    superclasses: parents.into_iter().map(str::to_owned).collect(),
                    ..ClassDef::default()
                },
            );
        }
        assert_eq!(tcl_syntax::naming::unroot_rooted_key("::茶"), Some("茶"));
        assert_ne!(
            tcl_syntax::naming::unroot_rooted_key(":::茶"),
            tcl_syntax::naming::unroot_rooted_key("::茶")
        );
        let hierarchy = analysis.class_hierarchy();
        assert!(hierarchy.transitive_subtypes[":::茶"].contains("::leaf"));
        assert!(!hierarchy.transitive_subtypes["::茶"].contains("::leaf"));
        analysis
            .all_classes
            .get_mut(":::茶")
            .unwrap()
            .superclasses
            .push("::leaf".to_owned());
        let hierarchy = tcl_compiler::analyser::build_class_hierarchy(analysis.all_classes.clone());
        assert!(
            hierarchy
                .transitive_subtypes
                .get("::missing")
                .is_none_or(|descendants| !descendants.contains("::leaf"))
        );
    }
}

#[cfg(test)]
mod original_implementation_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;
    #[test]
    fn original_implementation_uses_opaque_method_identity_and_exact_descendant_relations() {
        let source = "oo::class create B {method m\\uD800 {} {}; method m\\uD801 {} {}}\noo::class create C {superclass B; method m\\uD800 {} {}}\noo::class create Other {method m\\uD800 {} {}}\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_classes.clear();
        analysis.superseded_classes.clear();
        analysis.all_procs.clear();
        let cursor = u32::try_from(source.find("m\\uD800").unwrap()).unwrap();
        let ranges = implementation(source, 0, cursor, &analysis);
        assert_eq!(ranges.len(), 2, "{ranges:?}");
        assert_eq!(
            ranges
                .iter()
                .map(|range| range.start_line)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        let other = u32::try_from(source.find("m\\uD801").unwrap()).unwrap();
        assert_eq!(implementation(source, 0, other, &analysis).len(), 1);
        assert!(implementation(&format!("# changed\n{source}"), 1, cursor, &analysis).is_empty());
    }
    #[test]
    fn original_method_implementations_keep_cross_document_owners_and_block_ambiguous_parents() {
        let base = r"oo::class create B\uD800 {method m\uD800 {} {}}";
        let child = r"oo::class create C {superclass B\uD800; method m\uD800 {} {}}";
        let unrelated = r"oo::class create Other {method m\uD800 {} {}}";
        let mut base_analysis = Analyser::new().analyse(base, "tcl8.6");
        let mut child_analysis = Analyser::new().analyse(child, "tcl8.6");
        let other_analysis = Analyser::new().analyse(unrelated, "tcl8.6");
        base_analysis.all_classes.clear();
        child_analysis.all_classes.clear();
        let cursor = u32::try_from(base.find(r"m\uD800").unwrap()).unwrap();
        let std::ops::ControlFlow::Break(Some(method)) =
            crate::method_symbol::local_candidate(base, &base_analysis, 0, cursor)
        else {
            panic!("original method declaration must be selectable");
        };
        let documents = [
            crate::original_declaration::OriginalDeclarationDocument {
                uri: "",
                source: base,
                analysis: &base_analysis,
            },
            crate::original_declaration::OriginalDeclarationDocument {
                uri: "file:///child.tcl",
                source: child,
                analysis: &child_analysis,
            },
            crate::original_declaration::OriginalDeclarationDocument {
                uri: "file:///other.tcl",
                source: unrelated,
                analysis: &other_analysis,
            },
        ];
        let targets = method_implementations_in_inventory(&method, &documents);
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].identity.uri(), "");
        assert_eq!(targets[1].identity.uri(), "file:///child.tcl");
        let duplicated = [
            crate::original_declaration::OriginalDeclarationDocument {
                uri: "",
                source: base,
                analysis: &base_analysis,
            },
            crate::original_declaration::OriginalDeclarationDocument {
                uri: "file:///equal-source.tcl",
                source: base,
                analysis: &base_analysis,
            },
            crate::original_declaration::OriginalDeclarationDocument {
                uri: "file:///child.tcl",
                source: child,
                analysis: &child_analysis,
            },
        ];
        let targets = method_implementations_in_inventory(&method, &duplicated);
        assert!(
            targets
                .iter()
                .all(|target| target.identity.uri() != "file:///child.tcl")
        );
        let stale = [crate::original_declaration::OriginalDeclarationDocument {
            uri: "",
            source: "# unrelated source",
            analysis: &base_analysis,
        }];
        assert!(method_implementations_in_inventory(&method, &stale).is_empty());
    }
    #[test]
    fn original_class_object_implementation_does_not_borrow_instance_superclass_order() {
        let source = "oo::class create B {self method shared {} {}}\noo::class create C {superclass B; self method shared {} {}}\n";
        let mut analysis = Analyser::new().analyse(source, "tcl9.0");
        analysis.all_classes.clear();
        analysis.superseded_classes.clear();
        assert_eq!(
            analysis
                .original_class_declarations()
                .flat_map(|class| class
                    .metadata()
                    .original_members
                    .methods(tcl_compiler::analyser::MemberSide::ClassObject)
                    .unwrap())
                .count(),
            2
        );
        let cursor = u32::try_from(source.find("shared").unwrap()).unwrap();
        let ranges = implementation(source, 0, cursor, &analysis);
        assert_eq!(ranges.len(), 1, "{ranges:?}");
        assert_eq!(ranges[0].start_line, 0);
    }
}

#[cfg(test)]
mod current_source_implementation_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn logical_method_implementation_requires_current_source_and_complete_input() {
        let source = "oo::class create C {method run {} {}}\nrun";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_lexical_declaration_advice());
        assert_eq!(implementation(source, 1, 1, &analysis).len(), 1);
        assert!(implementation(&format!("{source}\n# changed"), 1, 1, &analysis).is_empty());
        let config = analysis.body_lexer_config.unwrap();
        analysis.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
        assert!(implementation(source, 1, 1, &analysis).is_empty());
        analysis.body_lexer_config = Some(config);
        let input = analysis.resolved_input.take().unwrap();
        analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::resolve_environment("tcl8.4").default_context_registry(),
            config,
        ));
        assert!(implementation(source, 1, 1, &analysis).is_empty());
        analysis.resolved_input = None;
        assert!(implementation(source, 1, 1, &analysis).is_empty());
    }
}

#[cfg(test)]
mod logical_source_selection_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn at(source: &str, offset: usize, analysis: &AnalysisResult) -> Vec<LspRange> {
        let position =
            LineIndex::new_lsp(source).position_at_utf16(u32::try_from(offset).unwrap(), source);
        implementation(source, position.line, position.character.get(), analysis)
    }

    #[test]
    fn logical_implementation_uses_real_member_operand_and_rejects_equal_spelled_data() {
        let source = "oo::class create Base {method run {} {}}\noo::class create Child {superclass Base; method run {} {}}\nputs run\nrun\n";
        let analysis = Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_retained_logical_declaration_advice());
        let locations = at(source, source.find("run").unwrap(), &analysis);
        assert_eq!(
            locations
                .iter()
                .map(|range| range.start_line)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert!(at(source, source.find("puts run").unwrap() + 5, &analysis).is_empty());
        assert!(at(source, source.rfind("run").unwrap(), &analysis).is_empty());
    }

    #[test]
    fn logical_implementation_keeps_unicode_self_selector_and_exact_source_configuration() {
        let source = "oo::class create C {method 茶 {} {}; method again {} {my 茶}}";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        assert_eq!(at(source, source.rfind('茶').unwrap(), &analysis).len(), 1);
        assert!(
            at(
                &format!("{source}\n# changed"),
                source.rfind('茶').unwrap(),
                &analysis
            )
            .is_empty()
        );
        let config = analysis.body_lexer_config.unwrap();
        analysis.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
        assert!(at(source, source.rfind('茶').unwrap(), &analysis).is_empty());
    }
    #[test]
    fn logical_implementation_lifts_lone_cr_source_positions() {
        let source = "set z 0;\roo::class create C {method run {} {}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let locations = at(source, source.find("run").unwrap(), &analysis);
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].start_line, 1);
        assert_eq!(locations[0].end_line, 1);
    }
}

#[cfg(test)]
mod conditional_source_implementation_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn conditional_edges_keep_root_and_genuine_descendant_without_inventing_unknown_parent() {
        let source = "oo::class create Base {method run {} {}}\nmystery\noo::class create Child {superclass Base; method run {} {}}\noo::class create Other {superclass $missing; method run {} {}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let root = &analysis.all_classes["::Base"];
        let advice =
            crate::type_hierarchy::logical_implementation_classes(source, &analysis, root, true)
                .unwrap();
        assert!(advice.incomplete);
        assert_eq!(advice.classes.len(), 1);
        assert!(std::ptr::eq(
            advice.classes[0],
            &raw const analysis.all_classes["::Child"]
        ));
        let at = LineIndex::new_lsp(source)
            .position_at_utf16(u32::try_from(source.find("run").unwrap()).unwrap(), source);
        assert_eq!(
            implementation(source, at.line, at.character.get(), &analysis)
                .iter()
                .map(|range| range.start_line)
                .collect::<Vec<_>>(),
            vec![0, 2]
        );
        assert!(
            analysis
                .logical_source_direct_classes(&analysis.all_classes["::Child"])
                .is_none()
        );
        assert!(
            analysis
                .logical_source_class_relation_candidates(&analysis.all_classes["::Other"])
                .is_none()
        );
    }

    #[test]
    fn unrelated_unrepresented_relation_does_not_erase_root_own_member() {
        let source = "oo::class create Base {method run {} {}}\noo::class create Other {superclass Missing; mystery; method run {} {}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let at = LineIndex::new_lsp(source)
            .position_at_utf16(u32::try_from(source.find("run").unwrap()).unwrap(), source);
        let selected = implementation(source, at.line, at.character.get(), &analysis);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].start_line, 0);
        assert!(
            analysis
                .logical_source_class_relation_candidates(&analysis.all_classes["::Other"])
                .is_none()
        );
    }
}

#[cfg(test)]
mod retained_implementation_applicability_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn implementation_reply_keeps_debt_and_revalidates_every_positive_target() {
        let source = "oo::class create Base {method run {} {}}\nmystery\noo::class create Child {superclass Base; method run {} {}}\noo::class create Other {superclass $missing}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let at = LineIndex::new_lsp(source)
            .position_at_utf16(u32::try_from(source.find("run").unwrap()).unwrap(), source);
        let advice = logical_implementation_advice(
            "file:///classes.tcl",
            source,
            at.line,
            at.character.get(),
            &analysis,
        )
        .unwrap();
        assert!(advice.has_unresolved_applicability());
        assert_eq!(
            advice
                .possible_source_ranges()
                .iter()
                .map(|range| range.start_line)
                .collect::<Vec<_>>(),
            vec![0, 2]
        );
        assert!(advice.is_current("file:///classes.tcl", source, &analysis));
        let mut changed = analysis.clone();
        changed
            .all_classes
            .get_mut("::Child")
            .unwrap()
            .methods
            .get_mut("run")
            .unwrap()
            .visibility = "private".into();
        assert!(!advice.is_current("file:///classes.tcl", source, &changed));
        assert!(!advice.is_current("file:///other.tcl", source, &analysis));
        assert!(!advice.is_current(
            "file:///classes.tcl",
            &format!("{source}\n# changed"),
            &analysis
        ));
    }
}

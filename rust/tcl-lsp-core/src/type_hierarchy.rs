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
//! retain conditional original source declaration and relation candidates.

use std::collections::HashSet;

use crate::original_declaration::OriginalDeclarationIdentity;
use std::ops::ControlFlow;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::analyser::types::ClassDef;
use tcl_lexer::LineIndex;

use crate::definition::LspRange;

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
    /// Current Logical class owner; reporting labels cannot replace it.
    pub logical_declaration: Option<LogicalHierarchyClass>,
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
    if analysis.allows_lexical_declaration_advice() {
        return prepare_logical_advice_for_document(uri, source, line, character, analysis)
            .map_or_else(Vec::new, LogicalHierarchyAdvice::into_items);
    }
    let Some(position) =
        crate::original_context::CurrentSourcePosition::capture(source, analysis, line, character)
    else {
        return Vec::new();
    };
    match crate::original_oo::class_at_cursor(analysis, source, position.offset()) {
        ControlFlow::Break(selected) => selected
            .and_then(|record| original_item_for(uri, source, analysis, record))
            .into_iter()
            .collect(),
        ControlFlow::Continue(()) => Vec::new(),
    }
}

/// Typed possible-positive preparation for actual readonly publication.
#[must_use]
pub fn prepare_logical_advice_for_document(
    uri: &str,
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<LogicalHierarchyAdvice> {
    let position =
        crate::original_context::CurrentSourcePosition::capture(source, analysis, line, character)?;
    analysis
        .allows_retained_logical_declaration_advice()
        .then_some(())?;
    let selection = logical_class_at(uri, source, analysis, position.offset())?;
    let class = selection.class(source, analysis)?;
    let mut item = logical_item_for_document(uri, class, source, analysis)?;
    let incomplete = selection.has_unresolved_applicability();
    item.logical_declaration = Some(selection);
    Some(LogicalHierarchyAdvice {
        items: vec![item],
        incomplete,
    })
}

/// Direct supertypes of `class_name`: its declared superclasses and
/// class-level mixins, in declaration order (supers then mixins),
/// de-duplicated.  Empty when the class is unknown or has none in this
/// document. This presentation projection certifies no source-graph absence.
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
    logical_literal_hierarchy_advice(class_name, source, analysis, false)
        .map_or_else(Vec::new, LogicalHierarchyAdvice::into_items)
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
    logical_literal_hierarchy_advice(class_name, source, analysis, true)
        .map_or_else(Vec::new, LogicalHierarchyAdvice::into_items)
}

/// Typed literal-name source advice for compatibility presentation callers.
/// Actual Server publication retains this result through its current-source lease.
#[must_use]
pub fn logical_literal_hierarchy_advice(
    class_name: &str,
    source: &str,
    analysis: &AnalysisResult,
    subtypes: bool,
) -> Option<LogicalHierarchyAdvice> {
    let class = logical_class_for_literal_name(class_name, source, analysis)?;
    let identity = LogicalHierarchyClass::capture("", source, analysis, class)?;
    let documents = [OriginalHierarchyDocument {
        uri: "",
        source,
        analysis,
    }];
    if subtypes {
        let mut advice = related_logical_from_inventory(&identity, &documents, true)?;
        advice
            .items
            .sort_by(|left, right| left.name.cmp(&right.name));
        return Some(advice);
    }
    let inventory = LogicalHierarchyInventory::capture(&documents)?;
    let root = inventory
        .classes
        .iter()
        .position(|(candidate, _)| std::ptr::eq(*candidate, class))?;
    let (parents, incomplete) = inventory.direct(root)?;
    let items = parents
        .into_iter()
        .map(|parent| {
            let mut item =
                logical_item_for_document("", inventory.classes[parent].0, source, analysis)?;
            item.logical_declaration.as_mut()?.inherited_applicability |= incomplete;
            Some(item)
        })
        .collect::<Option<Vec<_>>>()?;
    Some(LogicalHierarchyAdvice { items, incomplete })
}

/// Select only a genuine declaration, constructor head or original relation
/// operand. Conditional candidates preserve earlier mutation debt; they do not
/// supply the stronger selected-provider relation table.
pub(crate) fn logical_class_at(
    uri: &str,
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Option<LogicalHierarchyClass> {
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let image = tcl_lexer::SourceImage::document(source);
    let mut selected = Vec::new();
    let mut relation_operand = false;
    for class in analysis.all_classes.values() {
        let Some(statements) = analysis.logical_source_class_relation_candidates(class) else {
            continue;
        };
        for statement in statements {
            for operand in statement.operands() {
                if !operand
                    .operand()
                    .span()
                    .as_range()
                    .contains(&(cursor as usize))
                {
                    continue;
                }
                relation_operand = true;
                if let Some(
                    tcl_compiler::command_binding::LogicalSourceClassRelationTarget::Source(target),
                ) = operand.target(analysis, class)
                {
                    let mut identity =
                        LogicalHierarchyClass::capture(uri, source, analysis, target)?;
                    identity.selection = Some(LogicalHierarchyOperandSelection {
                        receiver: analysis.logical_source_class_declaration(class)?.clone(),
                        operand: operand.clone(),
                    });
                    if !selected.contains(&identity) {
                        selected.push(identity);
                    }
                }
            }
        }
    }
    if relation_operand {
        return match selected.as_slice() {
            [selection] => Some(selection.clone()),
            _ => None,
        };
    }
    // The older selector also includes relations. Only a genuine Name word or
    // constructor head may enter its separate declaration/constructor route.
    let contains =
        |word: &tcl_lexer::NativeWord| word.span().as_range().contains(&(cursor as usize));
    let declaration_name = analysis.all_classes.values().any(|class| {
        analysis
            .logical_source_class_declaration(class)
            .and_then(
                tcl_compiler::analyser::types::LogicalSourceClassDeclaration::original_name_word,
            )
            .is_some_and(contains)
    });
    let constructor_head = analysis.command_invocations.iter().any(|invocation| {
        tcl_compiler::registry_invocation::source_structure::source_constructor_call_at(
            source,
            analysis,
            invocation.range.start(),
        )
        .is_some_and(|call| call.original_words().first().is_some_and(contains))
    });
    if !declaration_name && !constructor_head {
        return None;
    }
    let class = analysis.logical_source_class_at(&image, current.config(), cursor)?;
    LogicalHierarchyClass::capture(uri, source, analysis, class)
}

fn logical_class_for_literal_name<'a>(
    name: &str,
    source: &str,
    analysis: &'a AnalysisResult,
) -> Option<&'a ClassDef> {
    crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let query = tcl_syntax::naming::authored_source_command_slot(
        &tcl_core_types::ByteNamespacePath::root(),
        name.as_bytes(),
    )?;
    let mut selected = analysis.all_classes.values().filter(|class| {
        analysis
            .logical_source_class_declaration(class)
            .and_then(|owner| owner.source_candidate_coordinate())
            == Some(&query)
    });
    let class = selected.next()?;
    selected.next().is_none().then_some(class)
}

fn logical_item_for_document(
    uri: &str,
    class: &ClassDef,
    source: &str,
    analysis: &AnalysisResult,
) -> Option<TypeHierarchyItem> {
    let identity = LogicalHierarchyClass::capture(uri, source, analysis, class)?;
    let owner = analysis.logical_source_class_declaration(class)?;
    let name =
        tcl_syntax::naming::authored_source_command_spelling(owner.source_candidate_coordinate()?)?;
    let mut item = item_for(class, source, &LineIndex::new_lsp(source));
    item.name = name;
    item.logical_declaration = Some(identity);
    Some(item)
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
        logical_declaration: None,
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
    let index = LineIndex::new_lsp(source);
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

/// Sealed current Logical class selection with an independent document owner.
/// Display names and client ranges cannot issue or redirect this source receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalHierarchyClass {
    uri: String,
    image: tcl_lexer::SourceImage,
    input: tcl_compiler::analyser::ResolvedAnalysisInput,
    declaration: tcl_compiler::analyser::types::LogicalSourceClassDeclaration,
    selection: Option<LogicalHierarchyOperandSelection>,
    inherited_applicability: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct LogicalHierarchyOperandSelection {
    receiver: tcl_compiler::analyser::types::LogicalSourceClassDeclaration,
    operand: tcl_compiler::command_binding::LogicalSourceClassRelationCandidates,
}
impl LogicalHierarchyClass {
    pub(crate) fn capture(
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        class: &ClassDef,
    ) -> Option<Self> {
        crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        let declaration = analysis.logical_source_class_declaration(class)?;
        declaration.original_name_word()?;
        declaration.source_candidate_coordinate()?;
        Some(Self {
            uri: uri.to_owned(),
            image: tcl_lexer::SourceImage::document(source),
            input: analysis.resolved_input.as_ref()?.clone(),
            declaration: declaration.clone(),
            selection: None,
            inherited_applicability: false,
        })
    }
    /// Owning document, independently of equal source bytes or display labels.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }
    /// Revalidate whole source/input and the actual retained class declaration.
    #[must_use]
    pub fn is_current(&self, uri: &str, source: &str, analysis: &AnalysisResult) -> bool {
        uri == self.uri && self.class(source, analysis).is_some()
    }
    /// This source selection retains lookup or earlier publication applicability.
    #[must_use]
    pub fn has_unresolved_applicability(&self) -> bool {
        self.inherited_applicability
            || self
                .selection
                .as_ref()
                .is_some_and(|selection| selection.operand.has_earlier_unknown_mutations())
    }
    pub(crate) fn class<'a>(
        &self,
        source: &str,
        analysis: &'a AnalysisResult,
    ) -> Option<&'a ClassDef> {
        crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        if self.image != tcl_lexer::SourceImage::document(source)
            || analysis.resolved_input.as_ref() != Some(&self.input)
        {
            return None;
        }
        let mut classes = analysis.all_classes.values().filter(|class| {
            analysis.logical_source_class_declaration(class) == Some(&self.declaration)
        });
        let class = classes.next()?;
        classes.next().is_none().then_some(())?;
        if let Some(selection) = &self.selection {
            let mut receivers = analysis.all_classes.values().filter(|receiver| {
                analysis.logical_source_class_declaration(receiver) == Some(&selection.receiver)
            });
            let receiver = receivers.next()?;
            receivers.next().is_none().then_some(())?;
            let tcl_compiler::command_binding::LogicalSourceClassRelationTarget::Source(target) =
                selection.operand.target(analysis, receiver)?
            else {
                return None;
            };
            if !std::ptr::eq(target, class) {
                return None;
            }
        }
        Some(class)
    }
}

/// Partial positive hierarchy advice with source applicability kept separate.
/// No empty response certifies missing classes, complete lookup or execution.
#[derive(Debug, Clone)]
pub struct LogicalHierarchyAdvice {
    items: Vec<TypeHierarchyItem>,
    incomplete: bool,
}
impl LogicalHierarchyAdvice {
    /// Compatibility presentation projection of possible positives only.
    /// An empty vector establishes no absence. Server callers instead retain
    /// this typed reply until their final readonly source publication lease.
    #[must_use]
    pub fn into_items(self) -> Vec<TypeHierarchyItem> {
        self.items
    }
    /// Borrow possible positive items for publication under the caller's final
    /// current-source lease. This projection certifies no completeness or absence.
    #[must_use]
    pub fn possible_source_items(&self) -> &[TypeHierarchyItem] {
        &self.items
    }
    /// Retained unavailable relation/header coverage adds an obligation.
    /// False covers only those retained debts, never a complete class census.
    #[must_use]
    pub const fn has_unresolved_applicability(&self) -> bool {
        self.incomplete
    }
}

struct LogicalHierarchyInventory<'a> {
    classes: Vec<(&'a ClassDef, OriginalHierarchyDocument<'a>)>,
    occupied: Vec<(&'a str, tcl_core_types::ByteCommandSlot)>,
    unrepresented: bool,
}
impl<'a> LogicalHierarchyInventory<'a> {
    fn capture(documents: &[OriginalHierarchyDocument<'a>]) -> Option<Self> {
        let mut inventory = Self {
            classes: Vec::new(),
            occupied: Vec::new(),
            unrepresented: false,
        };
        let mut uris = HashSet::new();
        for &document in documents {
            crate::original_context::CurrentSourceContext::capture(
                document.source,
                document.analysis,
            )?;
            if !uris.insert(document.uri) {
                return None;
            }
            if !document
                .analysis
                .allows_retained_logical_declaration_advice()
            {
                inventory.unrepresented = true;
                continue;
            }
            for class in document
                .analysis
                .checked_logical_source_class_declarations()?
            {
                LogicalHierarchyClass::capture(
                    document.uri,
                    document.source,
                    document.analysis,
                    class,
                )?;
                inventory.classes.push((class, document));
            }
            let image = tcl_lexer::SourceImage::document(document.source);
            let config = document.analysis.body_lexer_config?;
            for declaration in document
                .analysis
                .checked_logical_source_procedure_declarations(&image, config)?
            {
                inventory
                    .occupied
                    .push((document.uri, declaration.source_candidate_coordinate()?));
            }
        }
        Some(inventory)
    }

    fn direct(&self, index: usize) -> Option<(Vec<usize>, bool)> {
        use tcl_compiler::command_binding::LogicalSourceClassRelationTarget;
        let &(class, document) = self.classes.get(index)?;
        let statements = document
            .analysis
            .logical_source_class_relation_candidates(class)?;
        let mut supers = Vec::new();
        let mut mixins = Vec::new();
        let mut incomplete = false;
        for statement in statements {
            let current = match statement.relation() {
                tcl_registry::definer::RelationSlot::Superclass => &mut supers,
                tcl_registry::definer::RelationSlot::Mixin => &mut mixins,
                tcl_registry::definer::RelationSlot::Filter => continue,
            };
            let mut selected = Vec::new();
            for operand in statement.operands() {
                incomplete |= operand.has_earlier_unknown_mutations();
                match operand.target(document.analysis, class)? {
                    LogicalSourceClassRelationTarget::Registry => {}
                    LogicalSourceClassRelationTarget::Source(target) => {
                        let target = self.classes.iter().position(|(candidate, owner)| {
                            owner.uri == document.uri && std::ptr::eq(*candidate, target)
                        })?;
                        selected.push(target);
                    }
                    LogicalSourceClassRelationTarget::External(coordinates) => {
                        let mut found = false;
                        for coordinate in coordinates {
                            let candidates = self
                                .classes
                                .iter()
                                .enumerate()
                                .filter(|(_, (candidate, owner))| {
                                    owner.uri != document.uri
                                        && owner
                                            .analysis
                                            .logical_source_class_declaration(candidate)
                                            .and_then(|declaration| {
                                                declaration.source_candidate_coordinate()
                                            })
                                            == Some(coordinate)
                                })
                                .map(|(candidate, _)| candidate)
                                .collect::<Vec<_>>();
                            let occupied = self
                                .occupied
                                .iter()
                                .any(|(uri, slot)| *uri != document.uri && slot == coordinate);
                            if !candidates.is_empty() || occupied {
                                incomplete |= occupied || candidates.len() != 1;
                                selected.extend(candidates);
                                found = true;
                                break;
                            }
                        }
                        incomplete |= !found;
                    }
                }
            }
            statement.apply_values(current, &selected);
        }
        supers.extend(mixins);
        supers.retain(|target| *target != index);
        let mut seen = HashSet::new();
        supers.retain(|target| seen.insert(*target));
        Some((supers, incomplete))
    }
}

pub(crate) struct LogicalImplementationClasses<'a> {
    pub(crate) classes: Vec<&'a ClassDef>,
    pub(crate) incomplete: bool,
}

/// Conditional local implementation candidates and their retained applicability
/// debt. Unavailable edges cannot erase a genuine root's own member, and cannot
/// nominate a descendant without an actual original relation operand.
pub(crate) fn logical_implementation_classes<'a>(
    source: &'a str,
    analysis: &'a AnalysisResult,
    root: &'a ClassDef,
    descendants: bool,
) -> Option<LogicalImplementationClasses<'a>> {
    LogicalHierarchyClass::capture("", source, analysis, root)?;
    let inventory = LogicalHierarchyInventory::capture(&[OriginalHierarchyDocument {
        uri: "",
        source,
        analysis,
    }])?;
    let root_index = inventory
        .classes
        .iter()
        .position(|(class, _)| std::ptr::eq(*class, root))?;
    let mut incomplete = inventory.unrepresented;
    let edges = inventory
        .classes
        .iter()
        .enumerate()
        .map(|(index, _)| {
            if let Some((parents, debt)) = inventory.direct(index) {
                incomplete |= debt;
                Some(parents)
            } else {
                incomplete = true;
                None
            }
        })
        .collect::<Vec<_>>();
    let mut selected = Vec::new();
    let mut pending = vec![root_index];
    while let Some(parent) = pending.pop() {
        for (index, parents) in edges.iter().enumerate() {
            if index == root_index || selected.contains(&index) {
                continue;
            }
            if parents
                .as_ref()
                .is_some_and(|parents| parents.contains(&parent))
            {
                selected.push(index);
                if descendants {
                    pending.push(index);
                }
            }
        }
    }
    Some(LogicalImplementationClasses {
        classes: selected
            .into_iter()
            .map(|index| inventory.classes[index].0)
            .collect(),
        incomplete,
    })
}

/// Join authentic conditional relation operands with independently current source
/// headers. All duplicate source candidates survive; no load/MRO order is inferred.
#[must_use]
pub fn related_logical_from_inventory(
    identity: &LogicalHierarchyClass,
    documents: &[OriginalHierarchyDocument<'_>],
    subtypes: bool,
) -> Option<LogicalHierarchyAdvice> {
    let inventory = LogicalHierarchyInventory::capture(documents)?;
    let mut roots = inventory
        .classes
        .iter()
        .enumerate()
        .filter(|(_, (class, document))| {
            identity.is_current(document.uri, document.source, document.analysis)
                && identity
                    .class(document.source, document.analysis)
                    .is_some_and(|root| std::ptr::eq(root, *class))
        })
        .map(|(index, _)| index);
    let root = roots.next()?;
    roots.next().is_none().then_some(())?;
    let mut incomplete = inventory.unrepresented || identity.has_unresolved_applicability();
    let selected = if subtypes {
        let mut selected = Vec::new();
        for index in 0..inventory.classes.len() {
            if index == root {
                continue;
            }
            match inventory.direct(index) {
                Some((parents, debt)) => {
                    incomplete |= debt;
                    if parents.contains(&root) {
                        selected.push(index);
                    }
                }
                None => incomplete = true,
            }
        }
        selected
    } else {
        let (selected, debt) = inventory.direct(root)?;
        incomplete |= debt;
        selected
    };
    let mut items = selected
        .into_iter()
        .map(|index| {
            let (class, document) = inventory.classes[index];
            logical_item_for_document(document.uri, class, document.source, document.analysis)
        })
        .collect::<Option<Vec<_>>>()?;
    for item in &mut items {
        if let Some(identity) = &mut item.logical_declaration {
            identity.inherited_applicability |= incomplete;
        }
    }
    items.sort_by(|left, right| {
        left.logical_declaration
            .as_ref()
            .map(LogicalHierarchyClass::uri)
            .cmp(
                &right
                    .logical_declaration
                    .as_ref()
                    .map(LogicalHierarchyClass::uri),
            )
            .then_with(|| {
                (
                    left.selection_range.start_line,
                    left.selection_range.start_character,
                )
                    .cmp(&(
                        right.selection_range.start_line,
                        right.selection_range.start_character,
                    ))
            })
    });
    Some(LogicalHierarchyAdvice { items, incomplete })
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

#[cfg(test)]
mod logical_source_selection_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn at(source: &str, offset: usize, analysis: &AnalysisResult) -> Vec<TypeHierarchyItem> {
        let position =
            LineIndex::new_lsp(source).position_at_utf16(u32::try_from(offset).unwrap(), source);
        prepare(source, position.line, position.character.get(), analysis)
    }

    #[test]
    fn logical_hierarchy_joins_actual_source_operands_and_coordinates_not_report_names() {
        let source = "namespace eval N {\noo::class create Base {}\noo::class create Child {superclass Base}\n}\nputs Base\n";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_retained_logical_declaration_advice());
        let selected = at(
            source,
            source.rfind("superclass Base").unwrap() + 11,
            &analysis,
        );
        assert_eq!(
            selected
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            vec!["::N::Base"]
        );
        assert!(at(source, source.rfind("puts Base").unwrap() + 5, &analysis).is_empty());
        analysis
            .all_classes
            .get_mut("::N::Base")
            .unwrap()
            .qualified_name = "report-only".into();
        let parents = supertypes("::N::Child", source, &analysis);
        assert_eq!(
            parents
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            vec!["::N::Base"]
        );
        assert!(supertypes("report-only", source, &analysis).is_empty());
        assert!(prepare(source, u32::MAX, 0, &analysis).is_empty());
    }

    #[test]
    fn logical_hierarchy_withholds_dynamic_or_unrepresented_relation_coverage() {
        for body in [
            "superclass $target",
            "superclass Missing",
            "superclass Base; mystery",
        ] {
            let source = format!("oo::class create Base {{}}\noo::class create Child {{{body}}}\n");
            let analysis = Analyser::new().analyse(&source, "tcl");
            assert!(
                supertypes("::Child", &source, &analysis).is_empty(),
                "{body}"
            );
        }
    }
    #[test]
    fn logical_hierarchy_lifts_lone_cr_source_positions() {
        let source = "set z 0;\roo::class create C {method run {} {}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let selected = at(source, source.find("create C").unwrap() + 7, &analysis);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].selection_range.start_line, 1);
        assert_eq!(selected[0].range.start_line, 1);
        assert_eq!(selected[0].range.end_line, 1);
    }
}

#[cfg(test)]
mod original_lsp_geometry_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_hierarchy_lifts_lone_cr_source_positions() {
        let source = "set z 0;\roo::class create C {method run {} {}}";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_classes.clear();
        analysis.superseded_classes.clear();
        analysis.all_procs.clear();
        let offset = u32::try_from(source.find("create C").unwrap() + 7).unwrap();
        let position = LineIndex::new_lsp(source).position_at_utf16(offset, source);
        let selected = prepare_for_document(
            "file:///classes.tcl",
            source,
            position.line,
            position.character.get(),
            &analysis,
        );
        assert_eq!(selected.len(), 1);
        assert!(selected[0].original_declaration.is_some());
        assert_eq!(selected[0].selection_range.start_line, 1);
        assert_eq!(selected[0].range.start_line, 1);
        assert_eq!(selected[0].range.end_line, 1);
    }
}

#[cfg(test)]
mod logical_workspace_hierarchy_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn selected(
        uri: &str,
        source: &str,
        needle: &str,
        analysis: &AnalysisResult,
    ) -> LogicalHierarchyClass {
        let offset = u32::try_from(source.find(needle).unwrap()).unwrap();
        let position = LineIndex::new_lsp(source).position_at_utf16(offset, source);
        prepare_for_document(
            uri,
            source,
            position.line,
            position.character.get(),
            analysis,
        )
        .remove(0)
        .logical_declaration
        .unwrap()
    }

    #[test]
    fn logical_workspace_hierarchy_preserves_real_uris_and_namespace_relation_operands() {
        let base_source = "namespace eval N {oo::class create Base {}}";
        let child_source = "namespace eval N {oo::class create Child {superclass Base}}";
        let mut base = Analyser::new().analyse(base_source, "tcl");
        let child = Analyser::new().analyse(child_source, "tcl");
        let root = selected("file:///child.tcl", child_source, "Child", &child);
        base.all_classes
            .get_mut("::N::Base")
            .unwrap()
            .qualified_name = "untrusted report".into();
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
        let parents = related_logical_from_inventory(&root, &documents, false).unwrap();
        assert_eq!(parents.items.len(), 1);
        assert_eq!(parents.items[0].name, "::N::Base");
        let identity = parents.items[0].logical_declaration.as_ref().unwrap();
        assert_eq!(identity.uri(), "file:///base.tcl");
        assert!(parents.items[0].original_declaration.is_none());
        let children = related_logical_from_inventory(identity, &documents, true).unwrap();
        assert_eq!(children.items.len(), 1);
        assert_eq!(
            children.items[0]
                .logical_declaration
                .as_ref()
                .unwrap()
                .uri(),
            "file:///child.tcl"
        );
    }

    #[test]
    fn logical_workspace_hierarchy_retains_duplicate_candidates_and_missing_target_debt() {
        let base_source = "oo::class create Base {}";
        let child_source = "oo::class create Child {superclass Base Missing}";
        let base = Analyser::new().analyse(base_source, "tcl");
        let child = Analyser::new().analyse(child_source, "tcl");
        let root = selected("file:///child.tcl", child_source, "Child", &child);
        let documents = [
            OriginalHierarchyDocument {
                uri: "file:///a.tcl",
                source: base_source,
                analysis: &base,
            },
            OriginalHierarchyDocument {
                uri: "file:///b.tcl",
                source: base_source,
                analysis: &base,
            },
            OriginalHierarchyDocument {
                uri: "file:///child.tcl",
                source: child_source,
                analysis: &child,
            },
        ];
        let parents = related_logical_from_inventory(&root, &documents, false).unwrap();
        assert!(parents.has_unresolved_applicability());
        assert_eq!(parents.items.len(), 2);
        assert_ne!(
            parents.items[0].logical_declaration.as_ref().unwrap().uri(),
            parents.items[1].logical_declaration.as_ref().unwrap().uri()
        );
        assert!(!root.is_current("file:///other.tcl", child_source, &child));
    }

    #[test]
    fn logical_workspace_hierarchy_uses_authored_slot_operations_and_preserves_local_occupants() {
        let declarations = "oo::class create Base {}; oo::class create Other {}";
        let source = "oo::class create Child {superclass Base; superclass Other}";
        let provider = Analyser::new().analyse(declarations, "tcl");
        let child = Analyser::new().analyse(source, "tcl");
        let root = selected("file:///child.tcl", source, "Child", &child);
        let documents = [
            OriginalHierarchyDocument {
                uri: "file:///base.tcl",
                source: declarations,
                analysis: &provider,
            },
            OriginalHierarchyDocument {
                uri: "file:///child.tcl",
                source,
                analysis: &child,
            },
        ];
        let parents = related_logical_from_inventory(&root, &documents, false).unwrap();
        assert_eq!(
            parents
                .items
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            vec!["::Other"]
        );
        let occupied = "proc Base {} {}; oo::class create Child {superclass Base}";
        let occupied_analysis = Analyser::new().analyse(occupied, "tcl");
        let root = selected("file:///child.tcl", occupied, "Child", &occupied_analysis);
        let documents = [
            documents[0],
            OriginalHierarchyDocument {
                uri: "file:///child.tcl",
                source: occupied,
                analysis: &occupied_analysis,
            },
        ];
        assert!(related_logical_from_inventory(&root, &documents, false).is_none());
    }

    #[test]
    fn logical_workspace_hierarchy_wire_does_not_accept_labels_foreign_owner_or_stale_input() {
        let source = "oo::class create Child {superclass Base}";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        let root = selected("file:///child.tcl", source, "Child", &analysis);
        let mut registry = crate::original_declaration::OriginalDeclarationRegistry::default();
        let handle = registry.issue_logical_class(root.clone()).unwrap();
        assert_eq!(
            registry.resolve_logical_class(handle, root.uri(), source, &analysis),
            Some(root.clone())
        );
        assert!(
            registry
                .resolve(handle, root.uri(), source, &analysis)
                .is_none()
        );
        assert!(
            registry
                .resolve_logical_class(handle, "file:///other.tcl", source, &analysis)
                .is_none()
        );
        assert!(
            registry
                .resolve_logical_class(
                    handle,
                    root.uri(),
                    &format!("{source}\n# changed"),
                    &analysis
                )
                .is_none()
        );
        analysis.resolved_input = None;
        assert!(
            registry
                .resolve_logical_class(handle, root.uri(), source, &analysis)
                .is_none()
        );
    }
}

#[cfg(test)]
mod conditional_local_hierarchy_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn conditional_relation_operand_keeps_namespace_and_original_labels_with_prefix_debt() {
        let source = "namespace eval N {oo::class create Base {}; mystery; oo::class create Child {superclass Base}}\nputs Base";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        let child = &analysis.all_classes["::N::Child"];
        assert!(analysis.logical_source_direct_classes(child).is_none());
        let statements = analysis
            .logical_source_class_relation_candidates(child)
            .unwrap();
        assert!(statements[0].operands()[0].has_earlier_unknown_mutations());
        let cursor = u32::try_from(source.find("superclass Base").unwrap() + 11).unwrap();
        let selected = logical_class_at("", source, &analysis, cursor).unwrap();
        assert!(std::ptr::eq(
            selected.class(source, &analysis).unwrap(),
            &raw const analysis.all_classes["::N::Base"]
        ));
        assert!(
            logical_class_at(
                "",
                source,
                &analysis,
                u32::try_from(source.rfind("Base").unwrap()).unwrap()
            )
            .is_none()
        );
        analysis
            .all_classes
            .get_mut("::N::Base")
            .unwrap()
            .qualified_name = "display-only".into();
        assert_eq!(
            supertypes("::N::Child", source, &analysis)
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            vec!["::N::Base"]
        );
        assert!(supertypes("display-only", source, &analysis).is_empty());
        let identity = LogicalHierarchyClass::capture(
            "",
            source,
            &analysis,
            &analysis.all_classes["::N::Child"],
        )
        .unwrap();
        let advice = related_logical_from_inventory(
            &identity,
            &[OriginalHierarchyDocument {
                uri: "",
                source,
                analysis: &analysis,
            }],
            false,
        )
        .unwrap();
        assert!(advice.has_unresolved_applicability());
        assert_eq!(advice.into_items().len(), 1);
    }

    #[test]
    fn conditional_relation_selection_withholds_dynamic_body_and_changed_owner() {
        for body in [
            "superclass $target",
            "superclass Missing",
            "superclass Base; mystery",
        ] {
            let source =
                format!("oo::class create Base {{}}\nmystery\noo::class create Child {{{body}}}");
            let analysis = Analyser::new().analyse(&source, "tcl");
            let cursor = u32::try_from(source.find("superclass ").unwrap() + 11).unwrap();
            assert!(
                logical_class_at("", &source, &analysis, cursor).is_none(),
                "{body}"
            );
        }
        let source = "oo::class create Base {}; mystery; oo::class create Child {superclass Base}";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        let cursor = u32::try_from(source.rfind("Base").unwrap()).unwrap();
        assert!(logical_class_at("", source, &analysis, cursor).is_some());
        assert!(logical_class_at("", &format!("{source}\n# changed"), &analysis, cursor).is_none());
        analysis
            .all_classes
            .get_mut("::Base")
            .unwrap()
            .member_set_incomplete = true;
        assert!(logical_class_at("", source, &analysis, cursor).is_none());
        let mut analysis = Analyser::new().analyse(source, "tcl");
        let config = analysis.body_lexer_config.unwrap();
        analysis.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
        assert!(logical_class_at("", source, &analysis, cursor).is_none());
    }
}

#[cfg(test)]
mod retained_hierarchy_applicability_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn prepared_operand_identity_keeps_debt_and_revalidates_both_current_snapshots() {
        let source = "oo::class create Base {}; mystery; oo::class create Child {superclass Base}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let at = LineIndex::new_lsp(source).position_at_utf16(
            u32::try_from(source.rfind("Base").unwrap()).unwrap(),
            source,
        );
        let advice = prepare_logical_advice_for_document(
            "file:///classes.tcl",
            source,
            at.line,
            at.character.get(),
            &analysis,
        )
        .unwrap();
        assert!(advice.has_unresolved_applicability());
        let identity = advice.possible_source_items()[0]
            .logical_declaration
            .as_ref()
            .unwrap();
        assert!(identity.has_unresolved_applicability());
        assert!(identity.is_current("file:///classes.tcl", source, &analysis));
        let mut receiver_changed = analysis.clone();
        receiver_changed
            .all_classes
            .get_mut("::Child")
            .unwrap()
            .member_set_incomplete = true;
        assert!(!identity.is_current("file:///classes.tcl", source, &receiver_changed));
        let mut provider_changed = analysis.clone();
        provider_changed
            .all_classes
            .get_mut("::Base")
            .unwrap()
            .member_set_incomplete = true;
        assert!(!identity.is_current("file:///classes.tcl", source, &provider_changed));
        assert!(!identity.is_current("file:///different.tcl", source, &analysis));
        assert!(!identity.is_current(
            "file:///classes.tcl",
            &format!("{source}\n# changed"),
            &analysis
        ));
    }
}

#[cfg(test)]
mod known_relation_selection_receipt_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn ordinary_relation_keeps_original_operand_and_rejects_changed_receiver() {
        let source = "oo::class create Base {}; oo::class create Child {superclass Base}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let cursor = u32::try_from(source.rfind("Base").unwrap()).unwrap();
        assert!(
            analysis
                .logical_source_class_at(
                    &tcl_lexer::SourceImage::document(source),
                    analysis.body_lexer_config.unwrap(),
                    cursor
                )
                .is_some()
        );
        let selection = logical_class_at("file:///classes.tcl", source, &analysis, cursor).unwrap();
        assert!(selection.selection.is_some());
        assert!(!selection.has_unresolved_applicability());
        let retained = selection.selection.as_ref().unwrap();
        assert_eq!(retained.operand.operand().span().start(), cursor);
        assert!(selection.is_current("file:///classes.tcl", source, &analysis));
        let mut receiver_changed = analysis.clone();
        receiver_changed
            .all_classes
            .get_mut("::Child")
            .unwrap()
            .member_set_incomplete = true;
        assert!(!selection.is_current("file:///classes.tcl", source, &receiver_changed));
        let mut provider_changed = analysis.clone();
        provider_changed
            .all_classes
            .get_mut("::Base")
            .unwrap()
            .member_set_incomplete = true;
        assert!(!selection.is_current("file:///classes.tcl", source, &provider_changed));
        let declaration = logical_class_at(
            "file:///classes.tcl",
            source,
            &analysis,
            u32::try_from(source.find("Base").unwrap()).unwrap(),
        )
        .unwrap();
        assert!(declaration.selection.is_none());
        assert!(declaration.is_current("file:///classes.tcl", source, &analysis));
    }
}

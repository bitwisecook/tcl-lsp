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

//! Readonly source references to one declaration, and the report of how completely they were covered.

use super::member_census::complete_member_source_census;
use super::{
    OriginalDeclarationDocument, OriginalDeclarationIdentity, OriginalDeclarationRole,
    invocation_targets_declaration, math_function_targets_declaration_in,
};
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::analyser::types::MemberSide;
use tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata;
use tcl_lexer::Span;

/// `superclass` / `mixin` operands in this document's original class bodies
/// that select `identity`'s class, through the same first-publication relation
/// rule the type hierarchy walks (`original_direct_metadata_relations_for_records`).
/// Reporting only: these operands are class-name references, not invocations.
fn class_relation_reference_spans(
    identity: &OriginalDeclarationIdentity,
    source: &str,
    analysis: &AnalysisResult,
) -> Vec<Span> {
    use tcl_compiler::analyser::types::OriginalSourceClassRelationKind;
    let Some(target) = identity.class_metadata(analysis) else {
        return Vec::new();
    };
    let Some((image, config)) = OriginalDeclarationIdentity::current(source, analysis) else {
        return Vec::new();
    };
    let records = analysis.original_class_declarations().collect::<Vec<_>>();
    let Some(root) = records
        .iter()
        .position(|record| std::ptr::eq(*record, target))
    else {
        return Vec::new();
    };
    let occupied = analysis
        .original_procedure_declarations()
        .map(SourceDeclarationMetadata::name)
        .collect::<Vec<_>>();
    let mut spans = Vec::new();
    for record in &records {
        for effect in record.metadata().original_relations.effects() {
            if effect.side() != MemberSide::Instance
                || !matches!(
                    effect.kind(),
                    OriginalSourceClassRelationKind::Superclass
                        | OriginalSourceClassRelationKind::Mixin
                )
            {
                continue;
            }
            for relation in effect.values().unwrap_or_default() {
                let selected = relation.lookup().first_matching_publications(
                    records
                        .iter()
                        .enumerate()
                        .map(|(index, record)| (record.name(), Some(index)))
                        .chain(occupied.iter().map(|name| (*name, None))),
                );
                if selected.as_slice() != [Some(root)] {
                    continue;
                }
                // The operand word itself, written in this document under
                // its current grammar (the same check a procedure name gets).
                if let Some(key) = relation.name_input().original_word_key()
                    && key.source_image() == &image
                    && key.lexer_config() == config
                {
                    spans.push(key.span());
                }
            }
        }
    }
    spans
}

/// Readonly local references to this exact source declaration. Terminal command
/// links are optional; original method selection uses the shared member owner.
#[must_use]
pub fn reference_spans(
    identity: &OriginalDeclarationIdentity,
    source: &str,
    analysis: &AnalysisResult,
    follow_links: bool,
) -> Vec<Span> {
    if !identity.is_current(identity.uri(), source, analysis) {
        return Vec::new();
    }
    match identity.role() {
        OriginalDeclarationRole::Procedure => {
            procedure_reference_spans(identity, source, analysis, follow_links)
        }
        OriginalDeclarationRole::Class => {
            identity
                .class_metadata(analysis)
                .map_or_else(Vec::new, |declaration| {
                    analysis
                        .command_invocations
                        .iter()
                        .filter(|invocation| {
                            invocation_targets_declaration(
                                source,
                                analysis,
                                invocation,
                                declaration,
                                follow_links,
                            )
                        })
                        .map(|invocation| invocation.range)
                        .collect()
                })
        }
        OriginalDeclarationRole::Method(_) => crate::method_symbol::source_queries(analysis)
            .into_iter()
            .filter_map(|query| {
                let std::ops::ControlFlow::Break(Some(candidate)) =
                    crate::method_symbol::candidate_for_analysis(analysis, &query)
                else {
                    return None;
                };
                (OriginalDeclarationIdentity::for_member_candidate(
                    identity.uri(),
                    source,
                    analysis,
                    &candidate,
                )
                .as_ref()
                    == Some(identity))
                .then_some(query.span())
            })
            .collect(),
        OriginalDeclarationRole::Property(_) => {
            tcl_compiler::analyser::class_hierarchy::original_metadata::
                original_source_self_property_references(source, analysis)
                .into_iter()
                .flatten()
                .filter_map(|reference| {
                    (OriginalDeclarationIdentity::for_property(
                        identity.uri(),
                        source,
                        analysis,
                        reference.declaring_class(),
                        reference.property(),
                    )
                    .as_ref()
                        == Some(identity))
                    .then_some(reference.selector_span())
                })
                .collect()
        }
        OriginalDeclarationRole::Special(_, _) => identity
            .class_metadata(analysis)
            .zip(identity.special_metadata(analysis))
            .map_or_else(Vec::new, |(class, special)| {
                crate::references::original_special_next_chain_spans(
                    source, analysis, class, special,
                )
            }),
        OriginalDeclarationRole::Document => Vec::new(),
    }
}

fn procedure_reference_spans(
    identity: &OriginalDeclarationIdentity,
    source: &str,
    analysis: &AnalysisResult,
    follow_links: bool,
) -> Vec<Span> {
    identity
        .procedure_metadata(analysis)
        .map_or_else(Vec::new, |declaration| {
            let mut spans: Vec<_> = analysis
                .command_invocations
                .iter()
                .filter(|invocation| {
                    invocation_targets_declaration(
                        source,
                        analysis,
                        invocation,
                        declaration,
                        follow_links,
                    )
                })
                .map(|invocation| invocation.range)
                .collect();
            for occurrence in crate::math_function_symbol::occurrences(source, analysis)
                .into_iter()
                .flatten()
            {
                if math_function_targets_declaration_in(
                    source,
                    analysis,
                    source,
                    analysis,
                    &occurrence,
                    declaration,
                    follow_links,
                ) {
                    spans.push(occurrence.span());
                }
            }
            spans.sort_by_key(|span| (span.start(), span.end()));
            spans.dedup();
            spans
        })
}

/// Coverage of the independent conditional source-reference facet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadonlyReferenceCoverage {
    /// This role uses its existing inventory; no member source facet was requested.
    ExistingInventory,
    /// The checked source walk and typed member tape cover every selector,
    /// and no other command in this document may dispatch or capture the
    /// member. Other documents remain outside this census.
    CompleteSourceOccurrenceCensus,
    /// Genuine whole-source ownership retains partial positive occurrences.
    Partial,
    /// The independent source facet is unavailable; existing positives remain.
    Unavailable,
}

/// Readonly occurrences with explicit independent source coverage.
/// Neither variant proves an occurrence census, absence or edit coverage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadonlyReferenceReport {
    pub(super) spans: Vec<Span>,
    pub(super) coverage: ReadonlyReferenceCoverage,
}

impl ReadonlyReferenceReport {
    /// Positive source extents only, independently of editable occurrences.
    #[must_use]
    pub fn into_spans(self) -> Vec<Span> {
        self.spans
    }
    /// Borrow the positive readonly source extents.
    #[must_use]
    pub fn spans(&self) -> &[Span] {
        &self.spans
    }
    /// Partial source availability stays separate from its positive count.
    #[must_use]
    pub const fn coverage(&self) -> ReadonlyReferenceCoverage {
        self.coverage
    }
    pub(crate) fn extend(&mut self, spans: impl IntoIterator<Item = Span>) {
        self.spans.extend(spans);
        self.spans.sort_by_key(|span| (span.start(), span.end()));
        self.spans.dedup();
    }
}

/// Exact property occurrences require a complete member source census.
/// Applicability remains separate.
fn complete_property_source_occurrences(
    identity: &OriginalDeclarationIdentity,
    source: &str,
    analysis: &AnalysisResult,
    properties: &[tcl_compiler::analyser::class_hierarchy::original_metadata::OriginalSourceSelfPropertyReference],
) -> Option<Vec<Span>> {
    let methods = tcl_compiler::analyser::class_hierarchy::original_metadata::
        original_source_self_method_references(source, analysis)?;
    complete_member_source_census(identity, source, analysis, &methods, properties).then_some(())?;
    Some(
        properties
            .iter()
            .filter_map(|reference| {
                (OriginalDeclarationIdentity::for_property(
                    identity.uri(),
                    source,
                    analysis,
                    reference.declaring_class(),
                    reference.property(),
                )
                .as_ref()
                    == Some(identity))
                .then_some(reference.selector_span())
            })
            .collect(),
    )
}

/// Add conditional original member-body source correspondences for reporting.
/// Existing declaration/reference and transformation inventories are unchanged.
#[must_use]
pub fn readonly_reference_report(
    identity: &OriginalDeclarationIdentity,
    source: &str,
    analysis: &AnalysisResult,
    follow_links: bool,
) -> Option<ReadonlyReferenceReport> {
    identity
        .is_current(identity.uri(), source, analysis)
        .then_some(())?;
    crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let mut report = ReadonlyReferenceReport {
        spans: reference_spans(identity, source, analysis, follow_links),
        coverage: ReadonlyReferenceCoverage::ExistingInventory,
    };
    if matches!(identity.role(), OriginalDeclarationRole::Property(_)) {
        match tcl_compiler::analyser::class_hierarchy::original_metadata::
            original_source_self_property_references(source, analysis)
        {
            Some(references) => {
                if let Some(occurrences) = complete_property_source_occurrences(
                    identity, source, analysis, &references,
                ) {
                    report.coverage = ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus;
                    report.extend(occurrences);
                } else {
                    report.coverage = ReadonlyReferenceCoverage::Partial;
                    report.extend(references.into_iter().filter_map(|reference| {
                        (OriginalDeclarationIdentity::for_property(
                            identity.uri(),
                            source,
                            analysis,
                            reference.declaring_class(),
                            reference.property(),
                        )
                        .as_ref()
                            == Some(identity))
                        .then_some(reference.selector_span())
                    }));
                }
            }
            None => report.coverage = ReadonlyReferenceCoverage::Unavailable,
        }
        return Some(report);
    }
    if identity.role() == OriginalDeclarationRole::Class {
        report.extend(class_relation_reference_spans(identity, source, analysis));
    }
    if identity.role() != OriginalDeclarationRole::Method(MemberSide::Instance)
        || identity.object_owner.is_some()
    {
        return Some(report);
    }
    let references = tcl_compiler::analyser::class_hierarchy::original_metadata::
        original_source_self_method_references(source, analysis);
    match references {
        Some(references) => {
            let complete = tcl_compiler::analyser::class_hierarchy::original_metadata::
                original_source_self_property_references(source, analysis)
                .is_some_and(|properties| {
                    complete_member_source_census(
                        identity,
                        source,
                        analysis,
                        &references,
                        &properties,
                    )
                });
            report.coverage = if complete {
                ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus
            } else {
                ReadonlyReferenceCoverage::Partial
            };
            report.extend(references.into_iter().filter_map(|reference| {
                (OriginalDeclarationIdentity::for_method(
                    identity.uri(),
                    source,
                    analysis,
                    reference.declaring_class(),
                    reference.method(),
                )
                .as_ref()
                    == Some(identity))
                .then_some(reference.selector_span())
            }));
        }
        None => report.coverage = ReadonlyReferenceCoverage::Unavailable,
    }
    if report.coverage != ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus
        && let Some(occurrences) = crate::original_member_rename::original_member_occurrence_census(
            OriginalDeclarationDocument {
                uri: identity.uri(),
                source,
                analysis,
            },
            identity,
        )
    {
        // The rename planner's complete walk found exactly these selectors.
        let mut reported = report.spans.clone();
        reported.retain(|span| *span != identity.span());
        reported.sort_by_key(|span| (span.start(), span.end()));
        reported.dedup();
        if reported == occurrences {
            report.coverage = ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus;
        }
    }
    Some(report)
}

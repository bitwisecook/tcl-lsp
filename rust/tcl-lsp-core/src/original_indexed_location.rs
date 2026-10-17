// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly indexed source facts revalidated against their current document.
//!
//! A URI retains the independent document owner. The captured image, complete
//! scanner configuration, Registry identity and actual original fact must agree before a
//! caller projects a location or counts it. This carrier grants no lookup,
//! native cell, namespace existence, Normal completion or source edit.

use tcl_compiler::analyser::{AnalysisResult, types::NamespaceRef};
use tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableOccurrence;
use tcl_lexer::{SourceImage, Span};

use crate::namespace_symbol::OriginalNamespaceSymbol;
use crate::workspace_index::WorkspaceDiagnosticSourceContext;

#[derive(Debug, Clone, PartialEq, Eq)]
enum SourceFact {
    RegistrySymbol(tcl_compiler::signature_scan::symbol_name::OriginalSourceSymbolDeclaration),
    Declaration(crate::original_declaration::OriginalDeclarationIdentity),
    VendorDeclaration {
        input: tcl_compiler::signature_scan::vendor_name::VendorSourceNameInput,
        purpose: tcl_syntax::naming::VendorSourceNamePurpose,
    },
    Variable(SignatureSourceVariableOccurrence),
    Namespace {
        row: NamespaceRef,
        implicit_parent: Option<OriginalNamespaceSymbol>,
    },
}

/// A retained indexed fact, requiring independent current source validation.
#[derive(Clone, PartialEq, Eq)]
pub struct OriginalIndexedSourceLocation {
    uri: String,
    context: WorkspaceDiagnosticSourceContext,
    fact: SourceFact,
}

impl std::fmt::Debug for OriginalIndexedSourceLocation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (kind, span) = match &self.fact {
            SourceFact::RegistrySymbol(row) => ("RegistrySymbol", row.span()),
            SourceFact::Declaration(row) => ("Declaration", row.span()),
            SourceFact::VendorDeclaration { input, .. } => ("VendorDeclaration", input.span()),
            SourceFact::Variable(row) => ("Variable", row.span()),
            SourceFact::Namespace { row, .. } => ("Namespace", row.span),
        };
        formatter
            .debug_struct("OriginalIndexedSourceLocation")
            .field("uri", &self.uri)
            .field("source_bytes", &self.context.image().len())
            .field("kind", &kind)
            .field("span", &span)
            .finish_non_exhaustive()
    }
}

impl OriginalIndexedSourceLocation {
    /// Retain a named Registry source card with its genuine selected definer.
    /// This role supplies no command slot or live symbol existence.
    #[must_use]
    pub fn from_registry_symbol(
        uri: impl Into<String>,
        context: &WorkspaceDiagnosticSourceContext,
        declaration: &tcl_compiler::signature_scan::symbol_name::OriginalSourceSymbolDeclaration,
    ) -> Option<Self> {
        declaration
            .matches_source(context.image(), context.config())
            .then_some(())?;
        Self::capture(
            uri,
            context,
            SourceFact::RegistrySymbol(declaration.clone()),
        )
    }

    /// Retain a canonical readonly declaration and its independent URI owner.
    #[must_use]
    pub fn from_declaration(
        context: &WorkspaceDiagnosticSourceContext,
        identity: &crate::original_declaration::OriginalDeclarationIdentity,
    ) -> Option<Self> {
        (identity.image() == context.image() && identity.config() == context.config())
            .then_some(())?;
        Self::capture(
            identity.uri(),
            context,
            SourceFact::Declaration(identity.clone()),
        )
    }

    /// Retain a hosted header's original purpose, including unavailable units.
    #[must_use]
    pub fn from_vendor_declaration(
        uri: impl Into<String>,
        context: &WorkspaceDiagnosticSourceContext,
        declaration: &crate::vendor_declaration::OriginalVendorDeclaration<'_>,
    ) -> Option<Self> {
        declaration
            .input()
            .matches_source(context.image(), context.config())
            .then_some(())?;
        Self::capture(
            uri,
            context,
            SourceFact::VendorDeclaration {
                input: declaration.input().clone(),
                purpose: declaration.purpose(),
            },
        )
    }

    /// Retain the selected variable occurrence without rendering its name.
    #[must_use]
    pub fn from_variable(
        uri: impl Into<String>,
        context: &WorkspaceDiagnosticSourceContext,
        row: &SignatureSourceVariableOccurrence,
    ) -> Option<Self> {
        Self::capture(uri, context, SourceFact::Variable(row.clone()))
    }

    /// Retain an exact namespace fact, independently of presentation fields.
    #[must_use]
    pub fn from_namespace(
        uri: impl Into<String>,
        context: &WorkspaceDiagnosticSourceContext,
        row: &NamespaceRef,
    ) -> Option<Self> {
        let input = row.original_name_input.as_ref()?;
        (row.name_policy == Some(input.policy())
            && row.source_context.is_some()
            && row.source_namespace.is_some())
        .then_some(())?;
        Self::capture(
            uri,
            context,
            SourceFact::Namespace {
                row: row.clone(),
                implicit_parent: None,
            },
        )
    }

    /// Retain a declaring descendant for readonly implicit-parent geometry.
    #[must_use]
    pub fn from_implicit_namespace_parent(
        uri: impl Into<String>,
        context: &WorkspaceDiagnosticSourceContext,
        row: &NamespaceRef,
        parent: &OriginalNamespaceSymbol,
    ) -> Option<Self> {
        (row.declares
            && row.name_policy == Some(parent.policy())
            && parent
                .scope()
                .is_strict_ancestor_of(row.source_namespace.as_ref()?, parent.policy())
                == Some(true))
        .then_some(())?;
        let mut captured = Self::from_namespace(uri, context, row)?;
        if let SourceFact::Namespace {
            implicit_parent, ..
        } = &mut captured.fact
        {
            *implicit_parent = Some(parent.clone());
        }
        Some(captured)
    }

    fn capture(
        uri: impl Into<String>,
        context: &WorkspaceDiagnosticSourceContext,
        fact: SourceFact,
    ) -> Option<Self> {
        let uri = uri.into();
        (!uri.is_empty() && context.uses_original_names()).then_some(Self {
            uri,
            context: context.clone(),
            fact,
        })
    }

    /// Independent document ownership; equal source does not merge URIs.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Declaration classification from the retained fact, not its label.
    #[must_use]
    pub fn is_declaration(&self) -> bool {
        match &self.fact {
            SourceFact::RegistrySymbol(_)
            | SourceFact::Declaration(_)
            | SourceFact::VendorDeclaration { .. } => true,
            SourceFact::Variable(row) => row.is_declaration(),
            SourceFact::Namespace { row, .. } => row.declares,
        }
    }

    /// A descendant declaration contributes an implicit parent occurrence.
    #[must_use]
    pub fn is_implicit_namespace_parent(&self) -> bool {
        matches!(
            &self.fact,
            SourceFact::Namespace {
                implicit_parent: Some(_),
                ..
            }
        )
    }

    /// Revalidate complete current source/configuration and exact fact membership.
    /// The caller separately retains this URI through document reads and rendering.
    #[must_use]
    pub fn validated_span(&self, source: &str, analysis: &AnalysisResult) -> Option<Span> {
        let current = WorkspaceDiagnosticSourceContext::for_analysis(analysis)?;
        (&current == &self.context
            && current.uses_original_names()
            && current.image() == &SourceImage::document(source))
            .then_some(())?;
        let span = match &self.fact {
            SourceFact::RegistrySymbol(declaration) => {
                (declaration.matches_registry(analysis.resolved_registry()?)
                    && analysis
                        .original_symbol_declarations()
                        .any(|current| current == declaration))
                .then_some(())?;
                declaration.span()
            }
            SourceFact::Declaration(identity) => {
                identity
                    .is_current(&self.uri, source, analysis)
                    .then_some(())?;
                identity.span()
            }
            SourceFact::VendorDeclaration { input, purpose } => {
                crate::vendor_declaration::declarations(source, analysis)?
                    .into_iter()
                    .any(|card| card.input() == input && card.purpose() == *purpose)
                    .then_some(())?;
                input.span()
            }
            SourceFact::Variable(row) => {
                (!analysis
                    .original_variable_symbol_conflicts
                    .contains(&row.span())
                    && analysis
                        .original_variable_symbols
                        .iter()
                        .any(|candidate| candidate == row))
                .then_some(())?;
                row.span()
            }
            SourceFact::Namespace {
                row,
                implicit_parent,
            } => {
                // Original fields identify the fact; optional UI text and
                // source_span cannot select or erase an original operand.
                let current_row = analysis
                    .namespace_refs
                    .iter()
                    .find(|candidate| same_namespace_fact(candidate, row))?;
                match implicit_parent {
                    Some(parent) => crate::namespace_symbol::namespace_implicit_parent_span_in(
                        source,
                        current_row,
                        parent.scope(),
                    )?,
                    None => current_row.span,
                }
            }
        };
        (span.start() <= span.end() && source.get(span.as_range()).is_some()).then_some(span)
    }
}

/// An actual indexed Logical source declaration, independently of Native facts.
/// This is conditional readonly declaration advice, never loading or dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalIndexedDeclarationLocation {
    uri: String,
    context: WorkspaceDiagnosticSourceContext,
    input: tcl_compiler::analyser::ResolvedAnalysisInput,
    declaration: LogicalDeclaration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LogicalDeclaration {
    Procedure(Box<tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration>),
    Class(Box<tcl_compiler::analyser::types::LogicalSourceClassDeclaration>),
}

impl LogicalIndexedDeclarationLocation {
    pub(crate) fn procedure(
        uri: &str,
        context: &WorkspaceDiagnosticSourceContext,
        input: &tcl_compiler::analyser::ResolvedAnalysisInput,
        declaration: &tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration,
    ) -> Option<Self> {
        declaration
            .matches_source(context.image(), input)
            .then_some(())?;
        declaration.original_name_word()?;
        declaration.original_definition_site()?;
        Some(Self {
            uri: uri.to_owned(),
            context: context.clone(),
            input: input.clone(),
            declaration: LogicalDeclaration::Procedure(Box::new(declaration.clone())),
        })
    }

    pub(crate) fn class(
        uri: &str,
        context: &WorkspaceDiagnosticSourceContext,
        input: &tcl_compiler::analyser::ResolvedAnalysisInput,
        declaration: &tcl_compiler::analyser::types::LogicalSourceClassDeclaration,
    ) -> Option<Self> {
        declaration
            .matches_source(context.image(), input)
            .then_some(())?;
        declaration.original_name_word()?;
        Some(Self {
            uri: uri.to_owned(),
            context: context.clone(),
            input: input.clone(),
            declaration: LogicalDeclaration::Class(Box::new(declaration.clone())),
        })
    }

    /// Independent document ownership, never derived from a declaration label.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub(crate) fn retains_procedure(
        &self,
        current: &tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration,
    ) -> bool {
        let LogicalDeclaration::Procedure(held) = &self.declaration else {
            return false;
        };
        current.matches_source(self.context.image(), &self.input)
            && current
                .original_definition_site()
                .zip(held.original_definition_site())
                .is_some_and(|(current, held)| std::ptr::eq(current, held))
    }

    pub(crate) fn retains_class(
        &self,
        current: &tcl_compiler::analyser::types::LogicalSourceClassDeclaration,
    ) -> bool {
        let LogicalDeclaration::Class(held) = &self.declaration else {
            return false;
        };
        current.matches_source(self.context.image(), &self.input)
            && std::ptr::eq(current.metadata(), held.metadata())
    }

    /// Recheck the exact current image, complete input and genuine declaration.
    /// The caller must also hold actual indexed allocation membership through
    /// its final provider observation; byte equality alone is not that owner.
    #[must_use]
    pub fn validated_span(&self, source: &str, analysis: &AnalysisResult) -> Option<Span> {
        let current = WorkspaceDiagnosticSourceContext::for_analysis(analysis)?;
        (current == self.context
            && current.image() == &SourceImage::document(source)
            && analysis.resolved_input.as_ref() == Some(&self.input)
            && self.input.has_logical_source_name_context())
        .then_some(())?;
        let span = match &self.declaration {
            LogicalDeclaration::Procedure(held) => {
                analysis
                    .logical_source_procedure_declarations()
                    .any(|candidate| candidate == held.as_ref())
                    .then_some(())?;
                held.original_name_word()?.span()
            }
            LogicalDeclaration::Class(held) => {
                analysis
                    .all_classes
                    .values()
                    .any(|class| {
                        analysis.logical_source_class_declaration(class) == Some(held.as_ref())
                    })
                    .then_some(())?;
                held.original_name_word()?.span()
            }
        };
        (span.start() < span.end() && source.get(span.as_range()).is_some()).then_some(span)
    }
}

/// A genuine Logical variable source occurrence, never a Native table fact.
/// Source naming geometry supplies no current cell, alias or edit authority.
#[derive(Debug, Clone)]
pub struct LogicalIndexedVariableLocation {
    uri: String,
    context: WorkspaceDiagnosticSourceContext,
    input: tcl_compiler::analyser::ResolvedAnalysisInput,
    occurrence: SignatureSourceVariableOccurrence,
}

impl LogicalIndexedVariableLocation {
    pub(crate) fn input(&self) -> &tcl_compiler::analyser::ResolvedAnalysisInput {
        &self.input
    }

    pub(crate) fn symbol(
        &self,
    ) -> &tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableSymbol {
        self.occurrence.symbol()
    }

    pub(crate) fn capture(
        uri: &str,
        context: &WorkspaceDiagnosticSourceContext,
        input: &tcl_compiler::analyser::ResolvedAnalysisInput,
        occurrence: &SignatureSourceVariableOccurrence,
    ) -> Option<Self> {
        (occurrence.symbol().is_namespace()
            && occurrence.matches_logical_source_view(context.image(), input))
        .then_some(Self {
            uri: uri.to_owned(),
            context: context.clone(),
            input: input.clone(),
            occurrence: occurrence.clone(),
        })
    }

    /// The independent document issuer, never rendered from a variable key.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Whether the original receiver was selected as a declaration occurrence.
    #[must_use]
    pub fn is_declaration(&self) -> bool {
        self.occurrence.is_declaration()
    }

    pub(crate) fn retains(&self, occurrence: &SignatureSourceVariableOccurrence) -> bool {
        self.occurrence.shares_logical_source_allocation(occurrence)
    }

    /// Current complete input/image and genuine occurrence membership. Index
    /// allocation and URI/provider currency remain independent obligations.
    #[must_use]
    pub fn validated_span(&self, source: &str, analysis: &AnalysisResult) -> Option<Span> {
        (WorkspaceDiagnosticSourceContext::for_analysis(analysis).as_ref() == Some(&self.context)
            && self.context.image() == &SourceImage::document(source)
            && analysis.resolved_input.as_ref() == Some(&self.input)
            && self
                .occurrence
                .matches_logical_source_view(self.context.image(), &self.input)
            && !analysis
                .original_variable_symbol_conflicts
                .contains(&self.occurrence.span())
            && analysis
                .original_variable_symbols
                .iter()
                .any(|current| current == &self.occurrence))
        .then_some(())?;
        let span = self.occurrence.span();
        (span.start() < span.end() && source.get(span.as_range()).is_some()).then_some(span)
    }
}

/// Actual Logical namespace-role geometry with its complete original issuer.
/// Independent URI/currentness checks remain mandatory before any span lift.
#[derive(Debug, Clone)]
pub struct LogicalIndexedNamespaceLocation {
    uri: String,
    context: WorkspaceDiagnosticSourceContext,
    occurrence: tcl_compiler::signature_scan::original_name::LogicalSourceNamespaceOccurrence,
    implicit_parent: Option<OriginalNamespaceSymbol>,
}

impl LogicalIndexedNamespaceLocation {
    pub(crate) fn capture(
        uri: &str,
        analysis: &AnalysisResult,
        row: &NamespaceRef,
    ) -> Option<Self> {
        let context = WorkspaceDiagnosticSourceContext::for_analysis(analysis)?;
        let occurrence =
            tcl_compiler::signature_scan::original_name::LogicalSourceNamespaceOccurrence::capture(
                analysis, row,
            )?;
        (context.image() == occurrence.image()).then_some(Self {
            uri: uri.to_owned(),
            context,
            occurrence,
            implicit_parent: None,
        })
    }

    /// The independently retained document issuer.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Declaration role selected by the original Registry and Ensure descriptor.
    #[must_use]
    pub fn is_declaration(&self) -> bool {
        self.occurrence.row().declares
    }

    pub(crate) fn input(&self) -> &tcl_compiler::analyser::ResolvedAnalysisInput {
        self.occurrence.input()
    }

    pub(crate) fn names(&self, symbol: &OriginalNamespaceSymbol) -> bool {
        self.occurrence.row().source_namespace.as_ref() == Some(symbol.scope())
            && self.occurrence.row().name_policy == Some(symbol.policy())
    }

    pub(crate) fn for_implicit_parent(&self, parent: &OriginalNamespaceSymbol) -> Option<Self> {
        let row = self.occurrence.row();
        (row.declares
            && row.name_policy == Some(parent.policy())
            && parent
                .scope()
                .is_strict_ancestor_of(row.source_namespace.as_ref()?, parent.policy())
                == Some(true))
        .then_some(())?;
        let mut location = self.clone();
        location.implicit_parent = Some(parent.clone());
        Some(location)
    }

    pub(crate) fn retains(&self, current: &Self) -> bool {
        self.uri == current.uri
            && self.context == current.context
            && self
                .occurrence
                .shares_original_allocation(&current.occurrence)
    }

    /// Recheck the genuine original role/context against the actual complete
    /// analysis. Report labels cannot provide original operands or parent spans.
    #[must_use]
    pub fn validated_span(&self, source: &str, analysis: &AnalysisResult) -> Option<Span> {
        (WorkspaceDiagnosticSourceContext::for_analysis(analysis).as_ref() == Some(&self.context)
            && self.context.image() == &SourceImage::document(source)
            && self.occurrence.is_current(analysis))
        .then_some(())?;
        let row = self.occurrence.row();
        let span = match &self.implicit_parent {
            Some(parent) => crate::namespace_symbol::namespace_implicit_parent_span_in(
                source,
                row,
                parent.scope(),
            )?,
            None => row.span,
        };
        (span.start() < span.end() && source.get(span.as_range()).is_some()).then_some(span)
    }
}

/// One original Logical class-object method declaration, without a Native
/// receiver, installed class, method lookup or dispatch-entry assertion.
#[derive(Debug, Clone)]
pub struct LogicalIndexedMemberLocation {
    uri: String,
    context: WorkspaceDiagnosticSourceContext,
    input: tcl_compiler::analyser::ResolvedAnalysisInput,
    self_selector: Option<tcl_compiler::analyser::types::LogicalSourceSelfSelector>,
    member: tcl_compiler::analyser::types::LogicalSourceMemberDeclaration,
}

impl LogicalIndexedMemberLocation {
    pub(crate) fn capture(
        uri: &str,
        context: &WorkspaceDiagnosticSourceContext,
        input: &tcl_compiler::analyser::ResolvedAnalysisInput,
        member: tcl_compiler::analyser::types::LogicalSourceMemberDeclaration,
    ) -> Option<Self> {
        member
            .class()
            .matches_source(context.image(), input)
            .then_some(())?;
        (member.class_object_source_visibility()
            == Some(tcl_registry::definer::DeclaredMemberVisibility::Public))
        .then_some(())?;
        member.original_name_word()?;
        member.original_body_word()?;
        Some(Self {
            uri: uri.to_owned(),
            context: context.clone(),
            input: input.clone(),
            self_selector: None,
            member,
        })
    }

    pub(crate) fn capture_self_source(
        uri: &str,
        context: &WorkspaceDiagnosticSourceContext,
        input: &tcl_compiler::analyser::ResolvedAnalysisInput,
        member: tcl_compiler::analyser::types::LogicalSourceMemberDeclaration,
        request: &tcl_compiler::analyser::types::LogicalSourceSelfSelector,
    ) -> Option<Self> {
        member
            .class()
            .matches_source(context.image(), input)
            .then_some(())?;
        (member.side() == request.caller().side()).then_some(())?;
        member.original_name_word()?;
        member.original_body_word()?;
        Some(Self {
            uri: uri.to_owned(),
            context: context.clone(),
            input: input.clone(),
            member,
            self_selector: Some(request.clone()),
        })
    }

    /// Retained declaration metadata for presentation after the guarded source lift.
    #[must_use]
    pub fn source_member(&self) -> &tcl_compiler::analyser::types::LogicalSourceMemberDeclaration {
        &self.member
    }

    /// Independent actual document issuer, never a rendered class key.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub(crate) fn retains_class(
        &self,
        current: &tcl_compiler::analyser::types::LogicalSourceClassDeclaration,
    ) -> bool {
        current.matches_source(self.context.image(), &self.input)
            && std::ptr::eq(current.metadata(), self.member.class().metadata())
            && current
                .member(self.member.side(), self.member.metadata())
                .as_ref()
                == Some(&self.member)
    }

    /// Current full source/input and original class/member vector correspondence.
    /// Actual caller/target index membership and URI/provider currency are separate.
    #[must_use]
    pub fn validated_span(&self, source: &str, analysis: &AnalysisResult) -> Option<Span> {
        (WorkspaceDiagnosticSourceContext::for_analysis(analysis).as_ref() == Some(&self.context)
            && self.context.image() == &SourceImage::document(source)
            && analysis.resolved_input.as_ref() == Some(&self.input)
            && self
                .member
                .class()
                .matches_source(self.context.image(), &self.input)
            && match &self.self_selector {
                Some(request) => request.caller().side() == self.member.side(),
                None => {
                    self.member.class_object_source_visibility()
                        == Some(tcl_registry::definer::DeclaredMemberVisibility::Public)
                }
            })
        .then_some(())?;
        analysis
            .all_classes
            .values()
            .any(|class| {
                analysis.logical_source_class_declaration(class) == Some(self.member.class())
            })
            .then_some(())?;
        let span = self.member.original_name_word()?.span();
        (span.start() < span.end() && source.get(span.as_range()).is_some()).then_some(span)
    }
}

fn same_namespace_fact(left: &NamespaceRef, right: &NamespaceRef) -> bool {
    left.original_name_input == right.original_name_input
        && left.source_context == right.source_context
        && left.source_namespace == right.source_namespace
        && left.name_policy == right.name_policy
        && left.span == right.span
        && left.declares == right.declares
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_indexed_variable_locations_require_complete_current_source_and_fact() {
        // Implementation contract: naming.editor.original-indexed-source-location
        // docs/design/analysis/name-resolution-proofs/original-indexed-source-location.md
        let source = r"namespace eval N {set v\uD800 1; set v\uD801 2}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let context = WorkspaceDiagnosticSourceContext::for_analysis(&analysis).unwrap();
        let row = analysis
            .original_variable_symbols
            .iter()
            .find(|row| {
                row.is_declaration() && row.original_name_input().bytes().ends_with(b"\xed\xa0\x80")
            })
            .expect("an authentic opaque source declaration");
        let first =
            OriginalIndexedSourceLocation::from_variable("file:///first.tcl", &context, row)
                .unwrap();
        let second =
            OriginalIndexedSourceLocation::from_variable("file:///second.tcl", &context, row)
                .unwrap();
        assert_eq!(first.validated_span(source, &analysis), Some(row.span()));
        assert_eq!(second.validated_span(source, &analysis), Some(row.span()));
        assert_ne!(first.uri(), second.uri());
        assert!(first.is_declaration());
        let mut missing = analysis.clone();
        missing
            .original_variable_symbols
            .retain(|candidate| candidate != row);
        assert!(first.validated_span(source, &missing).is_none());
        let mut conflict = analysis.clone();
        conflict.original_variable_symbol_conflicts.push(row.span());
        assert!(first.validated_span(source, &conflict).is_none());
        let changed_source = source.replace("D800", "D802");
        let changed = Analyser::new().analyse(&changed_source, "tcl8.6");
        assert!(first.validated_span(&changed_source, &changed).is_none());
        assert!(first.validated_span(&changed_source, &analysis).is_none());
        let mut changed_config = analysis.clone();
        changed_config
            .body_lexer_config
            .as_mut()
            .unwrap()
            .strict_quoting ^= true;
        assert!(first.validated_span(source, &changed_config).is_none());
    }

    #[test]
    fn original_indexed_namespace_locations_ignore_reports_and_reject_foreign_facts() {
        // Implementation contract: naming.editor.original-indexed-source-location
        // docs/design/analysis/name-resolution-proofs/original-indexed-source-location.md
        let source = r"namespace eval N\uD800 {}; namespace eval N\uD801 {}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let context = WorkspaceDiagnosticSourceContext::for_analysis(&analysis).unwrap();
        let row = analysis
            .namespace_refs
            .iter()
            .find(|row| row.declares)
            .unwrap();
        let captured =
            OriginalIndexedSourceLocation::from_namespace("file:///ns.tcl", &context, row).unwrap();
        let mut reporting = analysis.clone();
        for row in &mut reporting.namespace_refs {
            row.original_name.clear();
            row.qualified_name = "COLLIDING-LABEL".to_owned();
            row.source_span = None;
        }
        assert_eq!(captured.validated_span(source, &reporting), Some(row.span));
        let mut missing = analysis.clone();
        missing.namespace_refs.remove(0);
        assert!(captured.validated_span(source, &missing).is_none());
        let foreign_source = source.replace("D800", "D802");
        let foreign = Analyser::new().analyse(&foreign_source, "tcl8.6");
        let foreign_row = foreign
            .namespace_refs
            .iter()
            .find(|row| row.declares)
            .unwrap();
        let foreign_capture =
            OriginalIndexedSourceLocation::from_namespace("file:///ns.tcl", &context, foreign_row)
                .unwrap();
        assert!(foreign_capture.validated_span(source, &analysis).is_none());
        assert!(
            captured
                .validated_span(source, &AnalysisResult::default())
                .is_none()
        );
    }

    #[test]
    fn original_indexed_locations_reject_changed_registry_with_identical_source_and_config() {
        // Implementation contract: naming.editor.original-indexed-source-location
        // docs/design/analysis/name-resolution-proofs/original-indexed-source-location.md
        let source = r"namespace eval N\uD800 {}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let context = WorkspaceDiagnosticSourceContext::for_analysis(&analysis).unwrap();
        let row = analysis
            .namespace_refs
            .iter()
            .find(|row| row.declares)
            .unwrap();
        let captured =
            OriginalIndexedSourceLocation::from_namespace("file:///registry.tcl", &context, row)
                .unwrap();
        assert_eq!(captured.validated_span(source, &analysis), Some(row.span));
        let retained = analysis.resolved_input.as_ref().unwrap();
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "indexed_currency_marker",
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let changed_context = std::sync::Arc::new(
            retained
                .context_registry()
                .with_command_store(std::sync::Arc::new(registry)),
        );
        let changed_input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            retained.analyser_profile(),
            retained.unit_profile(),
            changed_context,
            retained.lexer_config(),
        );
        let changed = Analyser::new()
            .with_resolved_input(changed_input.clone())
            .analyse(source, "tcl8.6");
        let current = WorkspaceDiagnosticSourceContext::for_analysis(&changed).unwrap();
        assert_eq!(context.image(), current.image());
        assert_eq!(context.config(), current.config());
        assert_eq!(context.uses_original_names(), current.uses_original_names());
        assert_ne!(context, current);
        assert!(captured.validated_span(source, &changed).is_none());
        // Equal retained facts cannot replace the independently current store.
        let mut same_facts = analysis.clone();
        same_facts.resolved_input = Some(changed_input);
        assert_eq!(same_facts.namespace_refs, analysis.namespace_refs);
        assert!(captured.validated_span(source, &same_facts).is_none());
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(WorkspaceDiagnosticSourceContext::for_analysis(&missing).is_none());
        assert!(captured.validated_span(source, &missing).is_none());
    }

    #[test]
    fn original_indexed_implicit_parents_require_current_declaring_geometry() {
        // Implementation contract: naming.editor.original-indexed-source-location
        // docs/design/analysis/name-resolution-proofs/original-indexed-source-location.md
        let source = r"namespace eval N\uD800::child {}; namespace exists N\uD800";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let context = WorkspaceDiagnosticSourceContext::for_analysis(&analysis).unwrap();
        let cursor = u32::try_from(source.rfind("D800").unwrap()).unwrap();
        let std::ops::ControlFlow::Break(Some(parent)) =
            crate::namespace_symbol::select_at_offset(source, &analysis, cursor)
        else {
            panic!("a genuine original implicit parent")
        };
        let row = analysis
            .namespace_refs
            .iter()
            .find(|row| row.declares)
            .unwrap();
        let captured = OriginalIndexedSourceLocation::from_implicit_namespace_parent(
            "file:///parent.tcl",
            &context,
            row,
            &parent,
        )
        .unwrap();
        let span = captured.validated_span(source, &analysis).unwrap();
        assert_eq!(source.get(span.as_range()), Some(r"N\uD800"));
        let mut missing = analysis.clone();
        missing.namespace_refs.clear();
        assert!(captured.validated_span(source, &missing).is_none());
        assert!(
            captured
                .validated_span(&format!("{source}\n"), &analysis)
                .is_none()
        );
    }
}

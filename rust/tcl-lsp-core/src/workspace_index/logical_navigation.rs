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

//! Logical source views: command calls, declaration candidates, and variable, namespace and member locations.

use super::records::WorkspaceDiagnosticSourceContext;
use super::{WorkspaceClass, WorkspaceIndex, WorkspaceLogicalCommandCall};
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::Span;

/// Readonly location of a conditional Logical workspace declaration candidate.
/// Reporting fields cannot recreate the source request that selected it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalWorkspaceCommandCandidate {
    pub(super) revision: u64,
    pub(super) image: tcl_lexer::SourceImage,
    /// Independently indexed declaring document.
    pub(super) uri: String,
    /// Presentation name from the selected declaration.
    pub(super) qualified_name: String,
    /// Original declaration Name span in that document.
    pub(super) name_span: Span,
    /// Optional descriptive signature summary.
    detail: Option<String>,
    procedure: Option<tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration>,
    location: crate::original_indexed_location::LogicalIndexedDeclarationLocation,
}

impl LogicalWorkspaceCommandCandidate {
    /// The actual selected immutable source image, without loaded-provider claims.
    #[must_use]
    pub fn source_image(&self) -> &tcl_lexer::SourceImage {
        &self.image
    }

    /// Genuine selected source header; class/nonprocedure candidates remain
    /// distinct occupants and cannot become a procedure via reporting ranges.
    #[must_use]
    pub fn procedure_declaration(
        &self,
    ) -> Option<&tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration> {
        self.procedure.as_ref()
    }

    /// Independently indexed declaring document; display names cannot replace it.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Presentation name from the selected declaration.
    #[must_use]
    pub fn qualified_name(&self) -> &str {
        &self.qualified_name
    }

    /// Immutable original selected Name geometry in the declaring document.
    #[must_use]
    pub const fn name_span(&self) -> Span {
        self.name_span
    }

    /// Optional descriptive signature summary.
    #[must_use]
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }

    /// Current declaring bytes must still match before converting the original span.
    #[must_use]
    pub fn matches_source(&self, source: &str) -> bool {
        self.image.bytes() == source.as_bytes()
    }
}

/// Conditional own class-object method cards selected from an actual caller
/// recipe and independent indexed original class headers. No dispatch follows.
#[derive(Debug, Clone)]
pub struct WorkspaceLogicalClassMemberTargets {
    /// Actual retained caller source allocation, rechecked after asynchronous reads.
    pub call: WorkspaceLogicalCommandCall,
    /// Genuine direct public source method declarations, possibly unavailable/empty.
    pub targets: Vec<crate::original_indexed_location::LogicalIndexedMemberLocation>,
}

/// Original member-body caller and independently selected same-side declarations.
/// The caller is the existing genuine member allocation, not a command namespace.
#[derive(Debug, Clone)]
pub struct WorkspaceLogicalSelfMemberTargets {
    /// Exact original enclosing source member, rechecked through existing membership.
    pub caller: crate::original_indexed_location::LogicalIndexedMemberLocation,
    /// Conditional original source declarations, with no dispatch entry claim.
    pub targets: Vec<crate::original_indexed_location::LogicalIndexedMemberLocation>,
}

impl WorkspaceIndex {
    /// Match one actual indexed provider to the selected Logical declaration.
    /// Complete source/input, canonical member and document ownership stay exact.
    #[must_use]
    pub fn logical_member_provider_matches_target(
        &self,
        declaring_uri: &str,
        provider: &WorkspaceClass,
        target: &crate::call_hierarchy::LogicalCallTarget<'_>,
    ) -> bool {
        let Some(document) = self
            .slots
            .get(&provider.uri)
            .and_then(|slot| self.docs.get(*slot))
        else {
            return false;
        };
        let (Some(context), Some(input)) =
            (&document.diagnostic_source, &document.original_source_input)
        else {
            return false;
        };
        document
            .classes
            .iter()
            .any(|current| std::ptr::eq(current, provider))
            && target.matches_indexed_member_provider(declaring_uri, provider, context, input)
    }

    /// Declaration-side Logical names from this document's retained source views.
    /// Complete original occurrences select records; missing views grant no fallback.
    #[must_use]
    pub fn logical_declaration_source_names_at(
        &self,
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        offset: u32,
    ) -> Option<Vec<String>> {
        let config = analysis.body_lexer_config?;
        let image = tcl_lexer::SourceImage::document(source);
        if !analysis.allows_retained_logical_declaration_advice()
            || !analysis.matches_original_source_image(&image, config)
        {
            return Some(Vec::new());
        }
        let covers = |span: Span| span.start() <= offset && offset < span.end();
        let procedures = analysis
            .original_procedure_declarations()
            .filter(|record| covers(record.name_input().span()))
            .map(|record| (record.original_occurrence(), false, record.name().policy()));
        let classes = analysis
            .original_class_declarations()
            .filter(|record| covers(record.name_input().span()))
            .map(|record| (record.original_occurrence(), true, record.name().policy()));
        let mut selected = procedures.chain(classes);
        let (occurrence, class, policy) = selected.next()?;
        if selected.next().is_some() {
            return Some(Vec::new());
        }
        let Some(document) = self.slots.get(uri).and_then(|slot| self.docs.get(*slot)) else {
            return Some(Vec::new());
        };
        let Some(context) = document.diagnostic_source.as_ref() else {
            return Some(Vec::new());
        };
        if context.uses_original_names()
            || context.image() != &image
            || context.config() != config
            || document.original_source_input.as_ref() != analysis.resolved_input.as_ref()
        {
            return Some(Vec::new());
        }
        let records = document
            .original_procs
            .iter()
            .map(|record| (record.original_occurrence(), record.name(), false))
            .chain(
                document
                    .original_classes
                    .iter()
                    .map(|record| (record.original_occurrence(), record.name(), true)),
            );
        let mut slots = Vec::new();
        for (candidate, name, candidate_class) in records {
            if candidate == occurrence
                && candidate_class == class
                && name.policy() == policy
                && !slots.contains(name.slot())
            {
                slots.push(name.slot().clone());
            }
        }
        let names = slots
            .iter()
            .filter_map(|slot| {
                tcl_syntax::naming::native_command_source_spelling(policy.recipe(), slot)
            })
            .collect();
        Some(names)
    }

    /// Full analysis input retained by the actual indexed source issuer.
    /// Borrowing it supplies no runtime lookup, member entry or edit authority.
    #[must_use]
    pub fn source_analysis_input(
        &self,
        uri: &str,
    ) -> Option<&tcl_compiler::analyser::ResolvedAnalysisInput> {
        self.docs
            .get(*self.slots.get(uri)?)?
            .original_source_input
            .as_ref()
    }

    /// One represented Logical call in an exact indexed source/input view.
    /// No represented call, ambiguous views or unavailable source yields no
    /// receipt; none of those results proves an empty command census.
    #[must_use]
    pub fn logical_command_call_at(
        &self,
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        offset: u32,
    ) -> Option<WorkspaceLogicalCommandCall> {
        let input = analysis.resolved_input.as_ref()?;
        if !input.has_logical_source_name_context() {
            return None;
        }
        let context = WorkspaceDiagnosticSourceContext::for_analysis(analysis)?;
        if context.image().bytes() != source.as_bytes() {
            return None;
        }
        let document = self.docs.get(*self.slots.get(uri)?)?;
        let mut matches = document.logical_command_calls.iter().filter(|call| {
            call.input == *input
                && call.source == context
                && call.candidates.original_head().is_some_and(|head| {
                    let span = head.span();
                    span.start() <= offset && offset < span.end()
                })
        });
        let selected = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        Some(selected.clone())
    }

    /// Conditional external declaration cards selected by an actual retained
    /// Logical call. Occupied, deleted or unavailable source slots cannot be
    /// renewed from report names. An empty result is not a workspace census.
    #[must_use]
    pub fn logical_declaration_targets_for_call(
        &self,
        call: &WorkspaceLogicalCommandCall,
    ) -> Option<Vec<crate::original_indexed_location::LogicalIndexedDeclarationLocation>> {
        call.is_retained_in(self).then_some(())?;
        let coordinates = call.candidates().external_header_coordinates()?;
        let mut targets = Vec::new();
        for document in &self.docs {
            let (Some(context), Some(input)) =
                (&document.diagnostic_source, &document.original_source_input)
            else {
                continue;
            };
            if input != &call.input {
                continue;
            }
            for declaration in &document.logical_procedure_declarations {
                let Some(coordinate) = declaration.source_candidate_coordinate() else {
                    continue;
                };
                if coordinates.contains(&coordinate)
                    && let Some(target) = crate::original_indexed_location::LogicalIndexedDeclarationLocation::procedure(&document.uri, context, input, declaration) {
                        targets.push(target);
                    }
            }
            for declaration in document
                .classes
                .iter()
                .filter_map(|class| class.logical_declaration.as_ref())
            {
                if declaration
                    .source_candidate_coordinate()
                    .is_some_and(|coordinate| coordinates.contains(coordinate))
                    && let Some(target) =
                        crate::original_indexed_location::LogicalIndexedDeclarationLocation::class(
                            &document.uri,
                            context,
                            input,
                            declaration,
                        )
                {
                    targets.push(target);
                }
            }
        }
        Some(targets)
    }

    /// Actual immutable declaration allocation membership, independent of equal
    /// names or images. Adding a view keeps it; replacement/removal retracts it.
    #[must_use]
    pub fn logical_declaration_location_is_retained(
        &self,
        location: &crate::original_indexed_location::LogicalIndexedDeclarationLocation,
    ) -> bool {
        self.slots
            .get(location.uri())
            .and_then(|slot| self.docs.get(*slot))
            .is_some_and(|document| {
                document
                    .logical_procedure_declarations
                    .iter()
                    .any(|current| location.retains_procedure(current))
                    || document
                        .classes
                        .iter()
                        .filter_map(|class| class.logical_declaration.as_ref())
                        .any(|current| location.retains_class(current))
            })
    }

    /// Retain the actual indexed source recipe for this current Logical
    /// variable occurrence. Equal report keys cannot supply membership.
    #[must_use]
    pub fn logical_variable_location_for_occurrence(
        &self,
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        occurrence: &tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableOccurrence,
    ) -> Option<crate::original_indexed_location::LogicalIndexedVariableLocation> {
        let input = analysis.resolved_input.as_ref()?;
        let context = WorkspaceDiagnosticSourceContext::for_analysis(analysis)?;
        (context.image().bytes() == source.as_bytes()).then_some(())?;
        let document = self.docs.get(*self.slots.get(uri)?)?;
        document
            .original_variables
            .iter()
            .any(|current| occurrence.shares_logical_source_allocation(current))
            .then_some(())?;
        crate::original_indexed_location::LogicalIndexedVariableLocation::capture(
            uri, &context, input, occurrence,
        )
    }

    /// Conditional same-symbol source occurrences retain actual Logical naming
    /// recipes and complete input correspondence. This is no cell census.
    pub fn logical_variable_source_locations(
        &self,
        request: &crate::original_indexed_location::LogicalIndexedVariableLocation,
    ) -> Vec<crate::original_indexed_location::LogicalIndexedVariableLocation> {
        if !self.logical_variable_location_is_retained(request) {
            return Vec::new();
        }
        self.docs
            .iter()
            .flat_map(|document| {
                document.original_variables.iter().filter_map(|occurrence| {
                    (document.original_source_input.as_ref() == Some(request.input())
                        && occurrence.symbol() == request.symbol())
                    .then_some(())?;
                    crate::original_indexed_location::LogicalIndexedVariableLocation::capture(
                        &document.uri,
                        document.diagnostic_source.as_ref()?,
                        document.original_source_input.as_ref()?,
                        occurrence,
                    )
                })
            })
            .collect()
    }

    /// Actual immutable variable recipe allocation retained by this document.
    #[must_use]
    pub fn logical_variable_location_is_retained(
        &self,
        location: &crate::original_indexed_location::LogicalIndexedVariableLocation,
    ) -> bool {
        self.slots
            .get(location.uri())
            .and_then(|slot| self.docs.get(*slot))
            .is_some_and(|document| {
                document
                    .original_variables
                    .iter()
                    .any(|occurrence| location.retains(occurrence))
            })
    }

    /// One exact represented namespace-role occurrence in the caller's actual
    /// indexed source view. Missing scope/role/word remains unavailable.
    #[must_use]
    pub fn logical_namespace_location_at(
        &self,
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        offset: u32,
    ) -> Option<crate::original_indexed_location::LogicalIndexedNamespaceLocation> {
        let context = WorkspaceDiagnosticSourceContext::for_analysis(analysis)?;
        (context.image().bytes() == source.as_bytes()).then_some(())?;
        let document = self.docs.get(*self.slots.get(uri)?)?;
        let mut locations = analysis
            .namespace_refs
            .iter()
            .filter(|row| row.span.start() <= offset && offset < row.span.end())
            .filter_map(|row| {
                crate::original_indexed_location::LogicalIndexedNamespaceLocation::capture(
                    uri, analysis, row,
                )
            })
            .filter(|location| {
                document
                    .logical_namespace_locations
                    .iter()
                    .any(|current| location.retains(current))
            });
        let selected = locations.next()?;
        locations.next().is_none().then_some(selected)
    }

    /// Same typed source namespace under the actual request's complete input.
    /// Parent locations are independent positive geometry, never an absence proof.
    #[must_use]
    pub fn logical_namespace_source_locations(
        &self,
        request: &crate::original_indexed_location::LogicalIndexedNamespaceLocation,
        symbol: &crate::namespace_symbol::OriginalNamespaceSymbol,
    ) -> Vec<crate::original_indexed_location::LogicalIndexedNamespaceLocation> {
        if !request.names(symbol) || !self.logical_namespace_location_is_retained(request) {
            return Vec::new();
        }
        self.docs
            .iter()
            .flat_map(|document| document.logical_namespace_locations.iter())
            .filter(|location| location.input() == request.input() && location.names(symbol))
            .cloned()
            .collect()
    }

    /// Original declaring descendants may project a written implicit parent.
    #[must_use]
    pub fn logical_namespace_parent_locations(
        &self,
        request: &crate::original_indexed_location::LogicalIndexedNamespaceLocation,
        symbol: &crate::namespace_symbol::OriginalNamespaceSymbol,
    ) -> Vec<crate::original_indexed_location::LogicalIndexedNamespaceLocation> {
        if !request.names(symbol) || !self.logical_namespace_location_is_retained(request) {
            return Vec::new();
        }
        self.docs
            .iter()
            .flat_map(|document| document.logical_namespace_locations.iter())
            .filter(|location| location.input() == request.input())
            .filter_map(|location| location.for_implicit_parent(symbol))
            .collect()
    }

    /// Actual retained original namespace source allocation, not a report key.
    #[must_use]
    pub fn logical_namespace_location_is_retained(
        &self,
        location: &crate::original_indexed_location::LogicalIndexedNamespaceLocation,
    ) -> bool {
        self.slots
            .get(location.uri())
            .and_then(|slot| self.docs.get(*slot))
            .is_some_and(|document| {
                document
                    .logical_namespace_locations
                    .iter()
                    .any(|current| location.retains(current))
            })
    }

    fn logical_self_relation_classes(
        &self,
        analysis: &AnalysisResult,
        request: &tcl_compiler::analyser::types::LogicalSourceSelfSelector,
        receiver: &tcl_compiler::analyser::ClassDef,
    ) -> Option<Vec<tcl_compiler::analyser::types::LogicalSourceClassDeclaration>> {
        let input = analysis.resolved_input.as_ref()?;
        let mut supers = Vec::new();
        let mut mixins = Vec::new();
        for statement in request.relations() {
            let current = match statement.relation() {
                tcl_registry::definer::RelationSlot::Superclass => &mut supers,
                tcl_registry::definer::RelationSlot::Mixin => &mut mixins,
                tcl_registry::definer::RelationSlot::Filter => continue,
            };
            let mut selected = Vec::new();
            for operand in statement.operands() {
                match operand.target(analysis, receiver)? {
                    tcl_compiler::command_binding::LogicalSourceClassRelationTarget::Source(
                        class,
                    ) => {
                        selected.push(analysis.logical_source_class_declaration(class)?.clone());
                    }
                    tcl_compiler::command_binding::LogicalSourceClassRelationTarget::External(
                        coordinates,
                    ) => {
                        for document in &self.docs {
                            if document.original_source_input.as_ref() != Some(input) {
                                continue;
                            }
                            for class in document
                                .classes
                                .iter()
                                .filter_map(|class| class.logical_declaration.as_ref())
                            {
                                if class
                                    .source_candidate_coordinate()
                                    .is_some_and(|coordinate| coordinates.contains(coordinate))
                                {
                                    selected.push(class.clone());
                                }
                            }
                        }
                    }
                    tcl_compiler::command_binding::LogicalSourceClassRelationTarget::Registry => {}
                }
            }
            statement.apply_values(current, &selected);
        }
        let mut classes = vec![request.caller().class().clone()];
        classes.extend(supers);
        classes.extend(mixins);
        Some(classes)
    }

    /// Original self-selector advice from the actual member body and written
    /// class-relation operands. Results are source alternatives, never an MRO.
    #[must_use]
    pub fn logical_self_member_targets_at(
        &self,
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        cursor: u32,
    ) -> Option<WorkspaceLogicalSelfMemberTargets> {
        let request = analysis.logical_source_self_selector_at(source, cursor)?;
        let input = analysis.resolved_input.as_ref()?;
        let context = WorkspaceDiagnosticSourceContext::for_analysis(analysis)?;
        (context.image().bytes() == source.as_bytes()).then_some(())?;
        let caller =
            crate::original_indexed_location::LogicalIndexedMemberLocation::capture_self_source(
                uri,
                &context,
                input,
                request.caller().clone(),
                &request,
            )?;
        self.logical_member_location_is_retained(&caller)
            .then_some(())?;
        let receiver = analysis.logical_source_member_class(request.caller())?;
        let name = tcl_syntax::word_rules::original_static_word_unicode_value(
            request.original_selector(),
        )?;
        let classes = self.logical_self_relation_classes(analysis, &request, receiver)?;
        let mut targets = Vec::new();
        for document in &self.docs {
            let (Some(context), Some(input)) =
                (&document.diagnostic_source, &document.original_source_input)
            else {
                continue;
            };
            if input != analysis.resolved_input.as_ref()? {
                continue;
            }
            for class in document
                .classes
                .iter()
                .filter_map(|class| class.logical_declaration.as_ref())
            {
                if !class.matches_source(context.image(), input)
                    || !classes
                        .iter()
                        .any(|selected| std::ptr::eq(selected.metadata(), class.metadata()))
                {
                    continue;
                }
                let members = match request.caller().side() {
                    tcl_compiler::analyser::types::MemberSide::Instance => {
                        &class.metadata().methods
                    }
                    tcl_compiler::analyser::types::MemberSide::ClassObject => {
                        &class.metadata().class_methods
                    }
                };
                for method in members.values() {
                    let Some(member) = class.member(request.caller().side(), method) else {
                        continue;
                    };
                    if member
                        .original_name_word()
                        .and_then(tcl_syntax::word_rules::original_static_word_unicode_value)
                        .as_deref()
                        != Some(name.as_slice())
                    {
                        continue;
                    }
                    if let Some(location) = crate::original_indexed_location::LogicalIndexedMemberLocation::capture_self_source(
                        &document.uri, context, input, member, &request,
                    ) { targets.push(location); }
                }
            }
        }
        Some(WorkspaceLogicalSelfMemberTargets { caller, targets })
    }

    /// Literal selector source advice for an independently indexed class header.
    /// Only direct public class-object methods are represented; these cards are
    /// not a method dispatch chain, instance receiver or negative member census.
    #[must_use]
    pub fn logical_class_member_targets_at(
        &self,
        uri: &str,
        source: &str,
        analysis: &AnalysisResult,
        cursor: u32,
    ) -> Option<WorkspaceLogicalClassMemberTargets> {
        let input = analysis.resolved_input.as_ref()?;
        input.has_logical_source_name_context().then_some(())?;
        let context = WorkspaceDiagnosticSourceContext::for_analysis(analysis)?;
        (context.image().bytes() == source.as_bytes()).then_some(())?;
        let candidates =
            tcl_compiler::registry_invocation::source_structure::source_logical_command_candidates(
                source, analysis,
            )?;
        let mut selectors = candidates
            .iter()
            .filter_map(|call| call.external_class_member_selector_at(cursor));
        let selector = selectors.next()?;
        selectors.next().is_none().then_some(())?;
        let document = self.docs.get(*self.slots.get(uri)?)?;
        let mut calls = document.logical_command_calls.iter().filter(|call| {
            call.input == *input
                && call.source == context
                && selector.matches_call(call.candidates())
        });
        let call = calls.next()?;
        calls.next().is_none().then_some(())?;
        self.logical_class_member_targets(call, &selector)
    }

    fn logical_class_member_targets(
        &self,
        call: &WorkspaceLogicalCommandCall,
        selector: &tcl_compiler::command_binding::LogicalExternalClassMemberSelector,
    ) -> Option<WorkspaceLogicalClassMemberTargets> {
        (call.is_retained_in(self) && selector.matches_call(call.candidates())).then_some(())?;
        let coordinates = call.candidates().external_header_coordinates()?;
        let mut represented_class = false;
        let mut targets = Vec::new();
        for document in &self.docs {
            let (Some(context), Some(input)) =
                (&document.diagnostic_source, &document.original_source_input)
            else {
                continue;
            };
            if input != &call.input {
                continue;
            }
            for declaration in document
                .classes
                .iter()
                .filter_map(|class| class.logical_declaration.as_ref())
            {
                if !declaration.matches_source(context.image(), input)
                    || !declaration
                        .source_candidate_coordinate()
                        .is_some_and(|coordinate| coordinates.contains(coordinate))
                {
                    continue;
                }
                represented_class = true;
                for method in declaration.metadata().class_methods.values() {
                    let Some(member) = declaration.member(
                        tcl_compiler::analyser::types::MemberSide::ClassObject,
                        method,
                    ) else {
                        continue;
                    };
                    if member
                        .original_name_word()
                        .and_then(tcl_syntax::word_rules::original_static_word_unicode_value)
                        .as_deref()
                        != Some(selector.name())
                    {
                        continue;
                    }
                    if let Some(location) =
                        crate::original_indexed_location::LogicalIndexedMemberLocation::capture(
                            &document.uri,
                            context,
                            input,
                            member,
                        )
                    {
                        targets.push(location);
                    }
                }
            }
        }
        represented_class.then(|| WorkspaceLogicalClassMemberTargets {
            call: call.clone(),
            targets,
        })
    }

    /// Actual immutable declaring class/member allocation, independent of labels.
    #[must_use]
    pub fn logical_member_location_is_retained(
        &self,
        location: &crate::original_indexed_location::LogicalIndexedMemberLocation,
    ) -> bool {
        self.slots
            .get(location.uri())
            .and_then(|slot| self.docs.get(*slot))
            .is_some_and(|document| {
                document
                    .classes
                    .iter()
                    .filter_map(|class| class.logical_declaration.as_ref())
                    .any(|class| location.retains_class(class))
            })
    }

    /// Original Logical headers from current indexed source and input owners.
    /// Suggestions provide insertion data without loaded or callable commands.
    pub fn logical_procedure_source_candidates(
        &self,
    ) -> impl Iterator<
        Item = (
            &str,
            &tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration,
        ),
    > {
        self.docs.iter().flat_map(|document| {
            document
                .logical_procedure_declarations
                .iter()
                .filter_map(move |declaration| {
                    let context = document.diagnostic_source.as_ref()?;
                    let input = document.original_source_input.as_ref()?;
                    declaration
                        .matches_source(context.image(), input)
                        .then_some((document.uri.as_str(), declaration))
                })
        })
    }

    /// Current Logical procedure source cards for one exact original
    /// math-function occurrence. The occurrence must reborrow from the caller's
    /// actual `AnalysisResult`; candidate labels and report names cannot create
    /// this query. Positive cards grant no loaded function or dispatch result.
    #[must_use]
    pub fn logical_math_source_candidates(
        &self,
        occurrence: &tcl_compiler::command_binding::OriginalLogicalMathFunctionOccurrence,
        caller_analysis: &tcl_compiler::analyser::AnalysisResult,
        exclude_uri: &str,
    ) -> Option<
        Vec<(
            String,
            tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration,
        )>,
    > {
        Some(
            self.logical_math_source_cards(occurrence, caller_analysis, exclude_uri)?
                .into_iter()
                .filter_map(|candidate| {
                    Some((
                        candidate.uri.clone(),
                        candidate.procedure_declaration()?.clone(),
                    ))
                })
                .collect(),
        )
    }

    /// Retain the same actual selected declaration owner for diagnostic provider dependencies.
    #[must_use]
    pub fn logical_math_source_cards(
        &self,
        occurrence: &tcl_compiler::command_binding::OriginalLogicalMathFunctionOccurrence,
        caller_analysis: &tcl_compiler::analyser::AnalysisResult,
        exclude_uri: &str,
    ) -> Option<Vec<LogicalWorkspaceCommandCandidate>> {
        let image = occurrence.original_word().image();
        let source = image.try_text().ok()?;
        if caller_analysis
            .logical_math_function_at(source, occurrence.span().start())
            .as_ref()
            != Some(occurrence)
        {
            return None;
        }
        let coordinates = occurrence.external_header_coordinates()?;
        for coordinate in coordinates {
            let candidates = self
                .logical_procedure_source_candidates()
                .filter(|(uri, declaration)| {
                    *uri != exclude_uri
                        && self
                            .slots
                            .get(*uri)
                            .and_then(|slot| self.docs.get(*slot))
                            .is_some_and(|document| {
                                document.original_source_input.as_ref()
                                    == caller_analysis.resolved_input.as_ref()
                            })
                        && declaration.source_candidate_coordinate().as_ref() == Some(&coordinate)
                })
                .filter_map(|(uri, declaration)| self.logical_procedure_candidate(uri, declaration))
                .collect::<Vec<_>>();
            if !candidates.is_empty() {
                return Some(candidates);
            }
        }
        Some(Vec::new())
    }

    fn logical_procedure_candidate(
        &self,
        uri: &str,
        declaration: &tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration,
    ) -> Option<LogicalWorkspaceCommandCandidate> {
        let document = self.docs.get(*self.slots.get(uri)?)?;
        let context = document.diagnostic_source.as_ref()?;
        let input = document.original_source_input.as_ref()?;
        let location =
            crate::original_indexed_location::LogicalIndexedDeclarationLocation::procedure(
                uri,
                context,
                input,
                declaration,
            )?;
        let metadata = declaration.metadata();
        Some(LogicalWorkspaceCommandCandidate {
            revision: self.document_revision(uri)?,
            image: context.image().clone(),
            uri: uri.to_owned(),
            qualified_name: metadata.qualified_name.clone(),
            name_span: metadata.name_span,
            detail: Some(format!("({} params)", metadata.params.len())),
            procedure: Some(declaration.clone()),
            location,
        })
    }

    /// Conditional Logical declarations at the first matching original source coordinate.
    /// Every duplicate remains a candidate; source cards grant no loaded command,
    /// reached implementation or writable reference. Native query owners are separate.
    #[must_use]
    pub fn logical_command_candidates(
        &self,
        request: &tcl_compiler::command_binding::OriginalLogicalCommandCandidates,
        exclude_uri: &str,
    ) -> Vec<LogicalWorkspaceCommandCandidate> {
        let Some(coordinates) = request.external_header_coordinates() else {
            return Vec::new();
        };
        for coordinate in coordinates {
            let mut candidates = Vec::new();
            for (uri, declaration) in self.logical_procedure_source_candidates() {
                if uri != exclude_uri
                    && self.logical_request_matches_document_input(request, uri)
                    && declaration.source_candidate_coordinate().as_ref() == Some(coordinate)
                    && let Some(candidate) = self.logical_procedure_candidate(uri, declaration)
                {
                    candidates.push(candidate);
                }
            }
            for class in self
                .classes()
                .filter(|class| class.uri != exclude_uri && !class.via_define)
            {
                if self.logical_request_matches_document_input(request, &class.uri)
                    && class
                        .logical_declaration
                        .as_ref()
                        .and_then(|declaration| declaration.source_candidate_coordinate())
                        == Some(coordinate)
                {
                    let Some(context) = self.diagnostic_source_context(&class.uri) else {
                        continue;
                    };
                    candidates.push(LogicalWorkspaceCommandCandidate {
                        location: match class.logical_declaration.as_ref().and_then(|declaration| {
                            let document = self.docs.get(*self.slots.get(&class.uri)?)?;
                            crate::original_indexed_location::LogicalIndexedDeclarationLocation::class(
                                &class.uri, context, document.original_source_input.as_ref()?, declaration,
                            )
                        }) { Some(location) => location, None => continue },
                        revision: self
                            .document_revision(&class.uri)
                            .expect("indexed declaration revision"),
                        image: context.image().clone(),
                        uri: class.uri.clone(),
                        qualified_name: class.qualified_name.clone(),
                        name_span: class.name_span,
                        detail: None,
                        procedure: None,
                    });
                }
            }
            if !candidates.is_empty() {
                return candidates;
            }
        }
        Vec::new()
    }

    fn logical_request_matches_document_input(
        &self,
        request: &tcl_compiler::command_binding::OriginalLogicalCommandCandidates,
        uri: &str,
    ) -> bool {
        let Some(input) = self
            .slots
            .get(uri)
            .and_then(|slot| self.docs.get(*slot))
            .and_then(|document| document.original_source_input.as_ref())
        else {
            return false;
        };
        request
            .original_head()
            .is_some_and(|head| request.matches_source(head.image(), input))
    }

    /// The independently selected declaring revision still owns this candidate.
    #[must_use]
    pub fn logical_command_candidate_is_current(
        &self,
        candidate: &LogicalWorkspaceCommandCandidate,
    ) -> bool {
        self.logical_declaration_location_is_retained(&candidate.location)
            && self.document_revision(&candidate.uri) == Some(candidate.revision)
            && self
                .diagnostic_source_context(&candidate.uri)
                .is_some_and(|context| context.image() == &candidate.image)
    }
}

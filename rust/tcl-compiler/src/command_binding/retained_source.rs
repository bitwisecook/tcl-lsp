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

//! Shared original source interpretation and its complete reuse boundary.

use std::sync::Arc;

use super::{
    ModuleCommandBindings, SourceAnalysisEntry, SourceAnalysisOptions, SourceCommandBindings,
};
use crate::ir::{Module, Procedure, TopLevelKind};
use crate::var_resolve::VariableExecutionFrame;
use tcl_registry::{CommandRegistry, RegistrySemanticKey};

/// Immutable command-effect projection of the interpretation used by lowering.
/// Changing any original input withdraws reuse and reaches the same source owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedSourceModuleBindings {
    source: tcl_lexer::SourceImage,
    entry: SourceAnalysisEntry,
    supplied_source: Option<SuppliedSourceAnalysisIssuance>,
    grammar: tcl_lexer::LexerConfig,
    namespace: String,
    native_namespace: Option<tcl_core_types::ByteNamespacePath>,
    namespace_context: Option<super::SourceNamespaceKey>,
    kind: TopLevelKind,
    registry: RegistrySemanticKey,
    observed: ModuleCommandBindings,
    positioned: Arc<SourceCommandBindings>,
    procedures: std::collections::BTreeMap<crate::ir::ProcedureId, OriginalProcedureSource>,
    installed_procedures:
        std::collections::BTreeMap<crate::ir::ProcedureId, OriginalProcedureSource>,
    methods: std::collections::BTreeMap<crate::ir::ProcedureId, OriginalMethodSource>,
}

/// Original header and body coordinates retained by the real Module producer.
/// This source signature supplies no entered procedure or native frame.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalProcedureSource {
    body_kind: crate::ir::ProcedureBodyKind,
    publication: Option<crate::signature_scan::scope::SignatureSourceCommand>,
    declaration_site: Option<super::CommandAllocationSite>,
    name: String,
    qualified_name: String,
    parameters: Vec<String>,
    parameters_text: String,
    declaration: tcl_lexer::Span,
    body_offset: u32,
    body_text: Option<String>,
    namespace: Option<Box<super::SourceNamespaceKey>>,
    body_source: Option<Arc<super::ExecutedScriptSource>>,
    statements: Vec<tcl_lexer::Span>,
}

impl OriginalProcedureSource {
    fn retain(procedure: &Procedure) -> Self {
        Self {
            body_kind: procedure.body_kind,
            publication: procedure.publication.clone(),
            declaration_site: procedure.declaration_site.clone(),
            name: procedure.name.clone(),
            qualified_name: procedure.qualified_name.clone(),
            parameters: procedure.params.clone(),
            parameters_text: procedure.params_raw.clone(),
            declaration: procedure.span,
            body_offset: procedure.body_offset,
            body_text: procedure.body_source.clone(),
            namespace: procedure.body.namespace_context.clone(),
            body_source: procedure.body.executed_source.clone(),
            statements: procedure
                .body
                .statements
                .iter()
                .map(crate::ir::Statement::span)
                .collect(),
        }
    }

    fn matches(&self, procedure: &Procedure) -> bool {
        self.body_kind == procedure.body_kind
            && self.publication == procedure.publication
            && self.declaration_site == procedure.declaration_site
            && self.name == procedure.name
            && self.qualified_name == procedure.qualified_name
            && self.parameters == procedure.params
            && self.parameters_text == procedure.params_raw
            && self.declaration == procedure.span
            && self.body_offset == procedure.body_offset
            && self.body_text == procedure.body_source
            && self.namespace == procedure.body.namespace_context
            && self.body_source == procedure.body.executed_source
            && self.statements.iter().copied().eq(procedure
                .body
                .statements
                .iter()
                .map(crate::ir::Statement::span))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalMethodSource {
    definition_scope: Option<crate::ir::ProcedureId>,
    kind: crate::ir::MethodKind,
    declaration: Option<tcl_lexer::Span>,
    parameters: Vec<String>,
    execution_namespace: crate::ir::ExecutionNamespace,
    body: Arc<crate::ir::Script>,
    receiver_context:
        Option<Arc<super::original_receiver_body_context::OriginalReceiverBodyContext>>,
    original_parameters: Option<Arc<super::formal_topology::OriginalFormalTopology>>,
}

impl OriginalMethodSource {
    fn retain(method: &crate::ir::MethodDef) -> Self {
        Self {
            definition_scope: method.definition_scope,
            kind: method.kind,
            declaration: method.span,
            parameters: method.params.clone(),
            execution_namespace: method.execution_namespace.clone(),
            body: Arc::new(method.body.clone()),
            receiver_context: method.original_receiver_context.clone(),
            original_parameters: method.original_parameters.clone(),
        }
    }

    fn matches(&self, method: &crate::ir::MethodDef) -> bool {
        self.definition_scope == method.definition_scope
            && self.kind == method.kind
            && self.declaration == method.span
            && self.parameters == method.params
            && self.execution_namespace == method.execution_namespace
            && self.body.as_ref() == &method.body
            && self.receiver_context == method.original_receiver_context
            && self.original_parameters == method.original_parameters
    }
}

/// Issued only when lowering chooses the supplied-document factory itself.
/// An equal explicit entry is not an issuance of this source-analysis intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SuppliedSourceAnalysisIssuance {
    input: crate::analyser::ResolvedAnalysisInput,
    entry: SourceAnalysisEntry,
    grammar: tcl_lexer::LexerConfig,
    registry: RegistrySemanticKey,
}

impl SuppliedSourceAnalysisIssuance {
    pub(crate) fn issue(
        registry: &CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
        config: tcl_lexer::LexerConfig,
        profile: Option<&tcl_dialect::DialectProfile>,
    ) -> (SourceAnalysisEntry, Self) {
        let entry = SourceAnalysisEntry::for_supplied_source(registry, input, config, profile);
        let options = entry.options();
        let grammar = options.native_lexer_config(config);
        let dialect = options.source_invocation_dialect(grammar);
        let policy = options.execution_name_policy();
        let mut effective = entry.clone();
        effective.invocation_dialect = dialect;
        effective.execution_name_policy = policy;
        let issued = Self {
            input: input.clone(),
            entry: effective,
            grammar,
            registry: registry.snapshot().semantic_key(),
        };
        (entry, issued)
    }

    fn matches(&self, module: &Module, registry: &CommandRegistry) -> bool {
        self.registry == registry.snapshot().semantic_key()
            && self.grammar == module.native_lexer_config()
            && self.entry == module.source_entry
            && module.source_metadata_input.as_ref() == Some(&self.input)
            && self.entry.metadata_context.source_analysis_input() == Some(&self.input)
    }
}

/// Conditional fresh Native source analysis under its original interpreter.
/// This grants no runtime entry, physical store or completed invocation.
pub(crate) struct FreshNativeSourceAnalysis<'a> {
    retained: &'a RetainedSourceModuleBindings,
    input: &'a crate::analyser::ResolvedAnalysisInput,
    registry: &'a CommandRegistry,
}

impl FreshNativeSourceAnalysis<'_> {
    pub(crate) fn matches_tokens(
        &self,
        tokens: &crate::ir::CommandTokens,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    ) -> bool {
        self.retained.owns_original_analysis_attachment(tokens)
            && metadata.matches_registry(self.registry)
            && metadata.source_analysis_input().is_some_and(|input| {
                input == self.input || input == &self.input.for_nested_source()
            })
    }
}

impl RetainedSourceModuleBindings {
    /// Project the original fresh-document source contract, never an execution entry.
    pub(crate) fn fresh_native_source_analysis<'a>(
        &'a self,
        module: &'a Module,
        registry: &'a CommandRegistry,
    ) -> Option<FreshNativeSourceAnalysis<'a>> {
        let positioned = self.positioned_for_module(module, registry)?;
        let input = self.entry.metadata_context.source_analysis_input()?;
        if self.kind != TopLevelKind::Script
            || positioned.original_analysis_owner.is_none()
            || input.has_logical_source_name_context()
            || input.has_hosted_source_name_context()
            || tcl_registry::InvocationDialect::of_profile(input.unit_profile())
                .native_name_protocol()
                .is_none()
            || self.entry.native_entry.is_some()
            || self.entry.source_point_entry.is_some()
            || self.entry.unknown_entry
            || self.entry.logical_source_input.is_some()
            || self.entry.vendor_source_input.is_some()
        {
            return None;
        }
        self.supplied_source
            .as_ref()?
            .matches(module, registry)
            .then_some(())?;
        Some(FreshNativeSourceAnalysis {
            retained: self,
            input,
            registry,
        })
    }

    /// Reborrow the actual immutable CU-side attachment for full-report
    /// capture only. This grants no module, body, lookup or runtime authority.
    pub(crate) fn matches_full_report_capture_owner(
        &self,
        image: &tcl_lexer::SourceImage,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &CommandRegistry,
        owner: &Arc<super::OriginalSourceAnalysisOwner>,
    ) -> bool {
        self.source == *image
            && self.grammar == input.lexer_config()
            && self.entry.metadata_context.source_analysis_input() == Some(input)
            && self.registry == registry.snapshot().semantic_key()
            && self
                .positioned
                .original_analysis_owner()
                .is_some_and(|actual| Arc::ptr_eq(actual, owner))
            && owner.matches(&self.positioned, input)
            && self
                .positioned
                .matches_original_source_image(image, input.lexer_config())
    }

    pub(crate) fn retain(
        bindings: &SourceCommandBindings,
        module: &Module,
        registry: &CommandRegistry,
        supplied_source: Option<&SuppliedSourceAnalysisIssuance>,
    ) -> Option<Arc<Self>> {
        if bindings.root_origin.as_ref()?.source_image() != &module.source
            || bindings.lexer_config != Some(module.native_lexer_config())
            || bindings.compilation_scope != module.source_entry.compilation_scope
        {
            return None;
        }
        Some(Arc::new(Self {
            source: module.source.clone(),
            entry: module.source_entry.clone(),
            supplied_source: supplied_source
                .filter(|issued| issued.matches(module, registry))
                .cloned(),
            grammar: module.native_lexer_config(),
            namespace: module.top_level_namespace.clone(),
            native_namespace: module.native_namespace.clone(),
            namespace_context: module.top_level_namespace_context.clone(),
            kind: module.top_level_kind,
            registry: registry.snapshot().semantic_key(),
            observed: bindings.module_projection(module),
            positioned: Arc::new(bindings.clone()),
            procedures: module
                .original_source_procedure_bodies()
                .map(|(name, procedure)| (*name, OriginalProcedureSource::retain(procedure)))
                .collect(),
            installed_procedures: module
                .installed_procedure_body_units
                .keys()
                .filter(|id| module.procedure_ids.owns(**id))
                .filter_map(|id| {
                    module
                        .body_lowering(id)
                        .map(|procedure| (*id, OriginalProcedureSource::retain(procedure)))
                })
                .collect(),
            methods: module
                .methods
                .iter()
                .filter(|(id, _)| module.procedure_ids.owns(**id))
                .map(|(id, method)| (*id, OriginalMethodSource::retain(method)))
                .collect(),
        }))
    }

    /// Check the same exact original Module inputs without deriving a clone.
    /// This is a source-owner join, independently of any reached dispatch.
    pub(crate) fn matches_module(&self, module: &Module, registry: &CommandRegistry) -> bool {
        self.source == module.source
            && self.entry == module.source_entry
            && self.entry.metadata_context.source_analysis_input()
                == module.source_metadata_input.as_ref()
            && self.grammar == module.native_lexer_config()
            && self.namespace == module.top_level_namespace
            && self.native_namespace == module.native_namespace
            && self.namespace_context == module.top_level_namespace_context
            && self.kind == module.top_level_kind
            && self.registry == registry.snapshot().semantic_key()
    }

    /// Original top-level source namespace for artifact geometry. Reporting
    /// labels cannot select it; runtime admission remains independent.
    pub(crate) fn original_top_namespace_for_module(
        &self,
        module: &Module,
        registry: &CommandRegistry,
    ) -> Option<&super::SourceNamespaceKey> {
        (self.source == module.own_source()
            && self.entry == module.source_entry
            && self.entry.metadata_context.source_analysis_input()
                == module.source_metadata_input.as_ref()
            && self.grammar == module.native_lexer_config()
            && self.native_namespace == module.native_namespace
            && self.namespace_context == module.top_level_namespace_context
            && self.kind == module.top_level_kind
            && self.registry == registry.snapshot().semantic_key()
            && module.registry_snapshot.as_ref()?.semantic_key() == self.registry
            && self
                .positioned
                .matches_original_source_image(&self.source, self.grammar))
        .then_some(())?;
        self.namespace_context.as_ref()
    }

    /// Borrow the exact positioned interpretation from the original Lowerer.
    /// An absent or changed Module input is terminal for this projection;
    /// independent source interpretation has its own explicit ingress.
    pub(crate) fn positioned_for_module(
        &self,
        module: &Module,
        registry: &CommandRegistry,
    ) -> Option<&SourceCommandBindings> {
        // naming.source.retained-positioned-command-realm
        // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
        if !self.matches_module(module, registry)
            || module.registry_snapshot.as_ref()?.semantic_key() != self.registry
            || !self
                .positioned
                .matches_original_source_image(&module.source, self.grammar)
        {
            return None;
        }
        Some(&self.positioned)
    }

    /// Reborrow the selected source `ParamList` topology for one procedure
    /// still owned by this exact Module. Report names and raw parameter text
    /// never substitute for the original selected declaration layout.
    pub(crate) fn original_procedure_formals_for_module(
        &self,
        module: &Module,
        body_id: crate::ir::ProcedureId,
        registry: &CommandRegistry,
    ) -> Option<crate::signature_scan::formal_parameters::SourceProcedureFormalTopology> {
        let positioned = self.positioned_for_module(module, registry)?;
        let (_, procedure) = module
            .original_source_procedure_bodies()
            .find(|(id, _)| **id == body_id)?;
        let retained = self.procedures.get(&body_id)?;
        if !retained.matches(procedure)
            || !matches!(
                procedure.body_kind,
                crate::ir::ProcedureBodyKind::Named
                    | crate::ir::ProcedureBodyKind::OriginalDeclaration
            )
        {
            return None;
        }
        let site = procedure.declaration_site.as_ref()?;
        if site.source.source_image() != &module.source {
            return None;
        }
        if let Some(input) = module
            .source_metadata_input
            .as_ref()
            .filter(|input| input.has_logical_source_name_context())
        {
            return positioned
                .original_logical_formal_topology_at(site, registry, input)
                .map(crate::signature_scan::formal_parameters::SourceProcedureFormalTopology::Logical);
        }
        positioned
            .original_procedure_formals_at(site, registry)
            .map(crate::signature_scan::formal_parameters::SourceProcedureFormalTopology::Native)
    }

    /// Exact source-snapshot comparison for an authored test declaration.
    #[cfg(test)]
    pub(crate) fn matches_original_procedure(&self, procedure: &Procedure) -> bool {
        self.procedures
            .values()
            .any(|original| original.matches(procedure))
    }

    /// Exact Module-issued source body for MAY summaries. Reporting labels
    /// do not change the retained original header, body or statement geometry.
    pub(crate) fn matches_original_procedure_summary(
        &self,
        id: crate::ir::ProcedureId,
        procedure: &Procedure,
    ) -> bool {
        let Some(original) = self.procedures.get(&id) else {
            return false;
        };
        let mut current = OriginalProcedureSource::retain(procedure);
        current.qualified_name.clone_from(&original.qualified_name);
        current == *original
    }

    /// Installed analysis source under its separately issued body ID.
    pub(crate) fn matches_installed_procedure_summary(
        &self,
        id: crate::ir::ProcedureId,
        procedure: &Procedure,
    ) -> bool {
        procedure.body_kind == crate::ir::ProcedureBodyKind::Installed
            && self
                .installed_procedures
                .get(&id)
                .is_some_and(|original| OriginalProcedureSource::retain(procedure) == *original)
    }

    /// Original issued method header and body coordinates, without a receiver entry.
    pub(crate) fn matches_original_method_summary(
        &self,
        id: crate::ir::ProcedureId,
        method: &crate::ir::MethodDef,
    ) -> bool {
        self.methods
            .get(&id)
            .is_some_and(|original| original.matches(method))
    }

    /// Conditional Logical declaration coordinate under this exact issued body.
    /// This source slot supplies no Native publication or entered invocation.
    pub(crate) fn original_logical_source_slot_for_body(
        &self,
        module: &Module,
        body: crate::ir::ProcedureId,
        registry: &CommandRegistry,
    ) -> Option<super::SourceCommandKey> {
        if !module.procedure_ids.owns(body) {
            return None;
        }
        let positioned = self.positioned_for_module(module, registry)?;
        let input = module.source_metadata_input.as_ref()?;
        input.has_logical_source_name_context().then_some(())?;
        let (_, procedure) = module
            .original_source_procedure_bodies()
            .find(|(id, _)| **id == body)?;
        self.matches_original_procedure_summary(body, procedure)
            .then_some(())?;
        let original_slot = |id, procedure: &Procedure| {
            let site = procedure.declaration_site.as_ref()?;
            Arc::ptr_eq(&site.source, positioned.root_origin.as_ref()?).then_some(())?;
            if let Some(card) = module.original_logical_declaration_body_units.get(&id) {
                card.matches_procedure(module, procedure).then_some(())?;
                Some(super::SourceCommandKey::authored(
                    tcl_syntax::naming::authored_source_command_spelling(
                        card.summary_coordinate(),
                    )?,
                ))
            } else {
                positioned.original_logical_procedure_source_slot(procedure, registry, input)
            }
        };
        let slot = original_slot(body, procedure)?;
        let count = module
            .original_source_procedure_bodies()
            .filter(|(id, candidate)| original_slot(**id, candidate).as_ref() == Some(&slot))
            .count();
        (count == 1).then_some(slot)
    }

    pub(crate) fn owns_original_analysis_attachment(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> bool {
        tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.original_analysis_owner.as_ref())
            .zip(self.positioned.original_analysis_owner.as_ref())
            .is_some_and(|(actual, original)| Arc::ptr_eq(actual, original))
    }

    #[cfg(debug_assertions)]
    pub(crate) fn trace_original_body_compiler_census(
        &self,
        body: crate::ir::ProcedureId,
        span: tcl_lexer::Span,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &CommandRegistry,
    ) {
        self.positioned
            .trace_original_body_compiler_census(body, span, input, registry);
    }

    /// Independent source-syntax observations from the original compiler.
    /// Runtime and Native attachment remain unchanged and may be unavailable.
    pub(crate) fn original_compiler_syntax_alternatives<'a>(
        &'a self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &CommandRegistry,
    ) -> Option<Vec<super::compiler_inventory::OriginalCompilerArgumentSyntax<'a>>> {
        if !self.owns_original_analysis_attachment(tokens)
            || self.grammar != input.lexer_config()
            || self.registry != registry.snapshot().semantic_key()
            || self.entry.metadata_context.source_analysis_input() != Some(input)
            || !self
                .positioned
                .matches_original_source_image(&self.source, self.grammar)
        {
            return None;
        }
        self.positioned
            .original_compiler_syntax_alternatives(tokens, input, registry)
    }

    /// Authenticate the complete written source carrier under this Module's
    /// original interpretation. This supplies source ownership independently of
    /// compiler visitation, Native handler admission and read/store receipts.
    pub(crate) fn owns_original_tokens(&self, tokens: &crate::ir::CommandTokens) -> bool {
        // naming.source.original-direct-token-owner
        // docs/design/analysis/name-resolution-proofs/original-direct-token-owner.md
        let Some(binding) = tokens.source_binding.as_ref() else {
            return false;
        };
        if !self.owns_original_analysis_attachment(tokens)
            || binding.original_written_words.is_none()
            || !tokens.original_words_match_argv_text()
        {
            return false;
        }
        let Some(config) = binding.original_lexer_config_for_tokens(tokens) else {
            return false;
        };
        let Some(site) = binding.invocation_site() else {
            return false;
        };
        let Some(snapshot) = binding.lookup_state.as_ref() else {
            return false;
        };
        config == self.grammar
            && site.source.source_image() == &self.source
            && self.positioned.root_origin.as_ref() == Some(&site.source)
            && snapshot.state.current_source_origin.as_ref() == Some(&site.source)
            && snapshot.state.baseline == self.observed.baseline
    }

    /// Exact original declaration geometry in this current issued body. Its
    /// source preview supplies no reached invocation, frame or stored value.
    pub(crate) fn owns_original_body_tokens(
        &self,
        module: &Module,
        body: crate::ir::ProcedureId,
        tokens: &crate::ir::CommandTokens,
        registry: &CommandRegistry,
    ) -> bool {
        let Some(input) = module.source_metadata_input.as_ref() else {
            return false;
        };
        let Some((_, procedure)) = module
            .original_source_procedure_bodies()
            .find(|(id, _)| **id == body)
        else {
            return false;
        };
        let Some(binding) = tokens.source_binding.as_ref() else {
            return false;
        };
        if !self.matches_module(module, registry)
            || !self.matches_original_procedure_summary(body, procedure)
            || !self.owns_original_analysis_attachment(tokens)
            || !input.has_logical_source_name_context()
            || tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
            || binding.lookup_state.is_some()
            || binding.has_original_native_source_command()
        {
            return false;
        }
        if binding.has_original_logical_source_advice() {
            let Some(card) = module.original_logical_declaration_body_units.get(&body) else {
                return false;
            };
            if !card.matches_procedure(module, procedure) {
                return false;
            }
            let Some(words) = binding.original_logical_source_words(tokens, input, registry) else {
                return false;
            };
            let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                return false;
            };
            let Some(scope) = advice.logical_source_scope() else {
                return false;
            };
            return Arc::ptr_eq(&advice.site().source, &card.site().source)
                && std::ptr::eq(scope.scope_body(), card.scope().scope_body())
                && scope.matches_input(input)
                && scope.owns_words(advice.original_words());
        }
        let Some(metadata) = binding.original_declaration_metadata_for_source(tokens, registry)
        else {
            return false;
        };
        if metadata
            .source_analysis_input()
            .is_none_or(|held| held != input && held != &input.for_nested_source())
        {
            return false;
        }
        let Some(site) = binding.invocation_site() else {
            return false;
        };
        let Some(observations) = binding.declaration_layout_observations.as_deref() else {
            return false;
        };
        let Some(rows) = super::declaration_layout::original_declaration_layouts(observations)
        else {
            return false;
        };
        let mut seen = false;
        for row in rows {
            seen = true;
            let source_matches = match row.entry.as_ref() {
                super::declaration_layout::OriginalDiagnosticFrameEntry::DeclaredProcedure(
                    declared,
                ) => declared.matches_original_procedure_source(procedure, &module.source),
                super::declaration_layout::OriginalDiagnosticFrameEntry::Body(_) => {
                    procedure.body.executed_source.as_deref() == Some(row.entry.source())
                }
                _ => false,
            };
            if !source_matches
                || !row.entry.owns_source(&site.source, site.offset)
                || row.words.as_ref() != tokens.words()
                || row.snapshot.state.current_source_origin.as_ref() != Some(&site.source)
                || row.snapshot.state.baseline != self.observed.baseline
                || row.config.nested().normalized() != self.grammar.nested().normalized()
                || site
                    .source
                    .original_lexical_command(tokens.words(), site.offset, row.config)
                    .is_none()
            {
                return false;
            }
        }
        seen
    }

    pub(super) fn projection(
        &self,
        module: &Module,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Option<ModuleCommandBindings> {
        if !self.matches_module(module, registry) || self.entry.options() != options {
            return None;
        }
        let mut observed = self.observed.clone();
        observed.has_redefined_procedures = !module.redefined_procedures.is_empty();
        Some(observed)
    }
}

impl SourceCommandBindings {
    pub(super) fn module_projection(&self, module: &Module) -> ModuleCommandBindings {
        let mut observed = self.final_state.as_ref().clone();
        for point in &self.points {
            observed.join(&point.state);
        }
        for future in &self.deferred_outcomes {
            observed.join(&future.state);
        }
        observed.root_boundary_bindings = Arc::clone(&self.final_state.bindings);
        observed.has_redefined_procedures = !module.redefined_procedures.is_empty();
        observed
    }
}

/// Actual entry frame shared by lowering and later original-source queries.
pub(crate) fn source_analysis_frame(
    entry: Option<&tcl_runtime_api::NativeCompilationEntry>,
    namespace: &str,
    kind: TopLevelKind,
) -> VariableExecutionFrame {
    use tcl_runtime_api::native_compilation::NativeCompilationFrame;
    if let Some(entry) = entry {
        let selected = entry
            .namespaces
            .iter()
            .find(|row| row.token == entry.current_namespace);
        let Some(selected) = selected.filter(|selected| {
            entry.closed
                && (!selected.visible
                    || entry
                        .namespaces
                        .iter()
                        .filter(|row| row.visible && row.path == selected.path)
                        .count()
                        == 1)
        }) else {
            return VariableExecutionFrame::Unknown;
        };
        let Ok(namespace_key) = entry
            .retained_namespace_context(selected.token)
            .map(super::SourceNamespaceKey::Native)
        else {
            return VariableExecutionFrame::Unknown;
        };
        let namespace = namespace_key.display().unwrap_or_default();
        let identity = format!(
            "runtime:{}:{}:{}:{}",
            entry.interpreter.owner,
            entry.interpreter.interpreter,
            entry.epoch,
            entry.current_namespace
        );
        let frame = match entry.frame {
            NativeCompilationFrame::Global => VariableExecutionFrame::Global,
            NativeCompilationFrame::Namespace => VariableExecutionFrame::NamespaceActivation {
                namespace,
                identity,
            },
            NativeCompilationFrame::Procedure => VariableExecutionFrame::Procedure {
                namespace,
                identity,
            },
            NativeCompilationFrame::Unknown => VariableExecutionFrame::Unknown,
        };
        return frame.with_namespace_identity(namespace_key);
    }
    if kind == TopLevelKind::ProcedureBody {
        VariableExecutionFrame::Procedure {
            namespace: namespace.to_owned(),
            identity: "::top".to_owned(),
        }
    } else if namespace == "::" {
        VariableExecutionFrame::Global
    } else {
        VariableExecutionFrame::Namespace(namespace.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supplied_issuance_refuses_changed_effective_contract_and_full_input() {
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = context.commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            "set value VALUE",
            registry,
            false,
            "tcl8.6",
        );
        let module = &unit.ir_module;
        let retained = module.retained_source_bindings.as_ref().unwrap();
        let issued = retained.supplied_source.as_ref().unwrap();
        assert!(issued.matches(module, registry));
        let mut changed = module.clone();
        changed
            .source_entry
            .incoming_formals
            .push("prior-frame".to_owned());
        assert!(!issued.matches(&changed, registry));
        let mut changed = module.clone();
        changed.source_entry.unknown_entry = true;
        assert!(!issued.matches(&changed, registry));
        let mut changed = module.clone();
        let mut config = issued.input.lexer_config();
        config.strict_quoting = !config.strict_quoting;
        changed.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            issued.input.analyser_profile(),
            issued.input.unit_profile(),
            issued.input.context_registry(),
            config,
        ));
        assert!(!issued.matches(&changed, registry));
        let mut changed = module.clone();
        changed.lexer_config.strict_quoting = !changed.lexer_config.strict_quoting;
        assert!(!issued.matches(&changed, registry));
        let foreign = CommandRegistry::build_default();
        assert!(!issued.matches(module, &foreign));
        let mut changed = module.clone();
        changed.source_entry.execution_name_policy = None;
        assert!(!issued.matches(&changed, registry));
    }

    #[test]
    fn fresh_source_transfer_refuses_a_matched_non_script_kind() {
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = context.commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
            "set value VALUE",
            registry,
            false,
            "tcl8.6",
        );
        let mut module = unit.ir_module.clone();
        let mut retained = module
            .retained_source_bindings
            .as_ref()
            .unwrap()
            .as_ref()
            .clone();
        assert!(
            retained
                .fresh_native_source_analysis(&module, registry)
                .is_some()
        );
        retained.kind = TopLevelKind::ProcedureBody;
        module.top_level_kind = TopLevelKind::ProcedureBody;
        assert!(retained.positioned_for_module(&module, registry).is_some());
        assert!(
            retained
                .fresh_native_source_analysis(&module, registry)
                .is_none()
        );
    }

    const SOURCE: &str = "namespace eval N {proc p {} {return OK}}; N::p";

    fn lowered(registry: &CommandRegistry) -> Module {
        crate::lowering::lower_to_ir_with_config(
            SOURCE,
            registry,
            tcl_lexer::LexerConfig::default(),
        )
    }

    #[test]
    fn logical_source_slot_uses_the_issued_current_declaration_without_native_publication() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let source = "proc absent::a {x} {return $x}\nproc absent::b {} {return OK}\n";
        let build = || {
            crate::compilation_unit::CompilationUnit::build_with_analysis_input(
                source,
                crate::compilation_unit::UnitBuildOptions {
                    registry: context.commands(),
                    config,
                    dialect: Some(profile),
                    defer_top_level: false,
                    external_call_sites: None,
                    declared_commands: None,
                },
                None,
                &input,
            )
        };
        let unit = build();
        let module = &unit.ir_module;
        let retained = module.retained_source_bindings.as_deref().unwrap();
        let (&id, procedure) = module
            .original_source_procedure_bodies()
            .find(|(_, procedure)| procedure.name == "absent::a")
            .expect("actual original declaration a");
        assert!(procedure.publication.is_none());
        let slot = retained
            .original_logical_source_slot_for_body(module, id, context.commands())
            .expect("genuine conditional Logical source slot");
        assert_eq!(slot.authored_spelling(), Some("::absent::a"));
        assert!(
            module
                .original_logical_declaration_body_units
                .contains_key(&id)
        );
        let mut report = module.clone();
        report.body_units.get_mut(&id).unwrap().qualified_name = "report-only".into();
        assert_eq!(
            retained.original_logical_source_slot_for_body(&report, id, context.commands()),
            Some(slot)
        );
        for mutate in 0..5 {
            let mut changed = module.clone();
            let procedure = changed.body_units.get_mut(&id).unwrap();
            match mutate {
                0 => procedure.name = "b".into(),
                1 => procedure.params.push("extra".into()),
                2 => procedure.body_source = Some("return OTHER".into()),
                3 => procedure.declaration_site = None,
                _ => procedure.body_kind = crate::ir::ProcedureBodyKind::Named,
            }
            assert!(
                retained
                    .original_logical_source_slot_for_body(&changed, id, context.commands())
                    .is_none()
            );
        }
        let mut missing = module.clone();
        missing.source_metadata_input = None;
        assert!(
            retained
                .original_logical_source_slot_for_body(&missing, id, context.commands())
                .is_none()
        );
        let foreign = build();
        let (&foreign_id, _) = foreign
            .ir_module
            .original_source_procedure_bodies()
            .next()
            .unwrap();
        let foreign_procedure = foreign.ir_module.body_units.get(&foreign_id).unwrap();
        let mut foreign_origin = module.clone();
        foreign_origin
            .body_units
            .get_mut(&id)
            .unwrap()
            .declaration_site
            .as_mut()
            .unwrap()
            .source = Arc::clone(&foreign_procedure.declaration_site.as_ref().unwrap().source);
        assert!(
            retained
                .original_logical_source_slot_for_body(&foreign_origin, id, context.commands())
                .is_none()
        );
        assert!(!module.procedure_ids.owns(foreign_id));
        assert!(
            retained
                .original_logical_source_slot_for_body(module, foreign_id, context.commands())
                .is_none()
        );
    }

    #[test]
    fn named_source_body_keeps_its_stronger_layout_and_current_header() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let entry = SourceAnalysisEntry::for_logical_source(&input).unwrap();
        let source = "proc p {} {set value TEXT; return $value}\n";
        let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                config,
                dialect: Some(profile),
                defer_top_level: false,
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            &input,
        );
        let module = &unit.ir_module;
        let (&id, procedure) = module.procedures.iter().next().expect("actual named body");
        let retained = module.retained_source_bindings.as_deref().unwrap();
        for statement in &procedure.body.statements {
            let tokens = procedure
                .body
                .retained_source_tokens_for_statement(statement)
                .unwrap();
            assert!(retained.owns_original_body_tokens(module, id, tokens, context.commands()));
            let binding = tokens.source_binding.as_ref().unwrap();
            assert!(binding.has_original_source_layout_owner());
            assert!(!binding.has_original_logical_source_advice());
            assert!(
                binding
                    .original_logical_source_words(tokens, &input, context.commands())
                    .is_none()
            );
            assert!(
                binding
                    .original_declaration_metadata_for_source(tokens, context.commands())
                    .is_some()
            );
        }
        let tokens = procedure
            .body
            .retained_source_tokens_for_statement(procedure.body.statements.first().unwrap())
            .unwrap();
        for mutate in 0..5 {
            let mut changed = module.clone();
            let procedure = changed.procedures.get_mut(&id).unwrap();
            match mutate {
                0 => procedure.name.push_str("other"),
                1 => procedure.params.push("added".into()),
                2 => procedure.body_source = Some("return OTHER".into()),
                3 => {
                    procedure.body.namespace_context = Some(Box::new(
                        super::super::SourceNamespaceKey::authored("::other"),
                    ))
                }
                _ => procedure.publication = None,
            }
            assert!(!retained.owns_original_body_tokens(&changed, id, tokens, context.commands()));
        }
        let mut missing = module.clone();
        missing.source_metadata_input = None;
        assert!(!retained.owns_original_body_tokens(&missing, id, tokens, context.commands()));
    }

    #[test]
    fn direct_original_source_tokens_do_not_require_compiler_visitation() {
        // naming.source.original-direct-token-owner
        // docs/design/analysis/name-resolution-proofs/original-direct-token-owner.md
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile = tcl_registry::model::ingress::resolve_known_environment(engine)
                .expect("each fixture uses a known engine environment")
                .analyser_profile();
            let registry = CommandRegistry::build_default().project_for_profile(profile);
            let unit = crate::compilation_unit::CompilationUnit::build_for(
                "set x 0; llength $x; set x [expr {$x + 1}]",
                &registry,
                false,
            );
            let module = &unit.ir_module;
            let retained = module.retained_source_bindings.as_deref().unwrap();
            assert!(retained.matches_module(module, &registry), "{engine}");
            assert_eq!(
                module.source_entry.native_compilation.mode,
                tcl_registry::native_compilation::NativeCompilationMode::Direct,
                "{engine}",
            );
            assert!(module.source_entry.metadata_context.is_standalone());
            let script = &module.top_level;
            let tokens = script
                .retained_source_tokens_for_statement(script.statements.last().unwrap())
                .unwrap();
            let binding = tokens.source_binding.as_ref().unwrap();
            assert!(
                binding.original_compiler_source(tokens).is_none(),
                "{engine}"
            );
            assert!(retained.owns_original_tokens(tokens), "{engine}");
            assert!(
                binding
                    .original_invocation_metadata_at_point(tokens, &registry)
                    .unwrap()
                    .is_none(),
                "{engine}",
            );
            let nested = crate::value_shapes::command_substitution_tokens(
                &tokens.words()[2],
                Some(tokens),
                binding.original_lexer_config_for_tokens(tokens).unwrap(),
            )
            .unwrap();
            let [expression] = nested.as_slice() else {
                panic!("the original fixture contains exactly one expression: {engine}");
            };
            assert!(
                expression
                    .source_binding
                    .as_ref()
                    .unwrap()
                    .original_compiler_source(expression)
                    .is_none(),
                "{engine}",
            );
            assert!(retained.owns_original_tokens(expression), "{engine}");
        }
    }

    #[test]
    fn original_source_token_attachment_is_current_even_for_equal_source_inputs() {
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let source = "set x 0";
        let original =
            crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
        let other = crate::compilation_unit::CompilationUnit::build_for(source, &registry, false);
        let original_owner = original
            .ir_module
            .retained_source_bindings
            .as_deref()
            .unwrap();
        let other_owner = other.ir_module.retained_source_bindings.as_deref().unwrap();
        let original_tokens = original
            .ir_module
            .top_level
            .retained_source_tokens_for_statement(
                original.ir_module.top_level.statements.last().unwrap(),
            )
            .unwrap();
        let other_tokens = other
            .ir_module
            .top_level
            .retained_source_tokens_for_statement(
                other.ir_module.top_level.statements.last().unwrap(),
            )
            .unwrap();
        assert!(original_owner.owns_original_analysis_attachment(original_tokens));
        assert!(other_owner.owns_original_analysis_attachment(other_tokens));
        assert!(original_owner.owns_original_tokens(original_tokens));
        assert!(other_owner.owns_original_tokens(other_tokens));
        assert!(!original_owner.owns_original_analysis_attachment(other_tokens));
        assert!(!other_owner.owns_original_analysis_attachment(original_tokens));
        assert!(!original_owner.owns_original_tokens(other_tokens));
        let mut missing = original_tokens.clone();
        missing
            .source_binding
            .as_mut()
            .unwrap()
            .original_analysis_owner = None;
        assert!(!original_owner.owns_original_tokens(&missing));
    }

    #[test]
    fn original_source_token_owner_refuses_missing_and_changed_carriers() {
        // naming.source.original-direct-token-owner
        // docs/design/analysis/name-resolution-proofs/original-direct-token-owner.md
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let unit = crate::compilation_unit::CompilationUnit::build_for("set x 0", &registry, false);
        let module = &unit.ir_module;
        let retained = module.retained_source_bindings.as_deref().unwrap();
        let script = &module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        assert!(retained.owns_original_tokens(tokens));
        for change in 0..10 {
            let mut changed = tokens.clone();
            match change {
                0 => changed.source_binding = None,
                1 => changed.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedWrapper),
                2 => {
                    changed.word_exprs.pop();
                }
                3 => {
                    changed.argv_texts.pop();
                }
                4 => *changed.argv_texts.last_mut().unwrap() = "OTHER".to_owned(),
                5 => {
                    changed
                        .source_binding
                        .as_mut()
                        .unwrap()
                        .original_written_words = None
                }
                6 => {
                    let binding = changed.source_binding.as_mut().unwrap();
                    binding.compiler_lookup_state = binding.lookup_state.take();
                }
                7 => {
                    let snapshot = Arc::make_mut(
                        changed
                            .source_binding
                            .as_mut()
                            .unwrap()
                            .lookup_state
                            .as_mut()
                            .unwrap(),
                    );
                    snapshot.state.current_source_origin = None;
                }
                8 => {
                    let snapshot = Arc::make_mut(
                        changed
                            .source_binding
                            .as_mut()
                            .unwrap()
                            .lookup_state
                            .as_mut()
                            .unwrap(),
                    );
                    snapshot.state.current_source_origin =
                        Some(Arc::new(super::super::SourceOriginId::authored_image(
                            tcl_lexer::SourceImage::native(module.source.bytes()),
                        )));
                }
                _ => {
                    changed
                        .source_binding
                        .as_mut()
                        .unwrap()
                        .dispatch_site
                        .as_mut()
                        .unwrap()
                        .offset += 1
                }
            }
            assert!(!retained.owns_original_tokens(&changed), "carrier {change}");
        }
        let mut missing_root = retained.clone();
        Arc::make_mut(&mut missing_root.positioned).root_origin = None;
        assert!(!missing_root.owns_original_tokens(tokens));
        for change in 0..3 {
            let mut changed = retained.clone();
            match change {
                0 => changed.grammar.strict_quoting = !changed.grammar.strict_quoting,
                1 => changed.grammar.leading_bom = tcl_lexer::LeadingBom::Skip,
                _ => changed.grammar.base_offset += 1,
            }
            assert!(
                !changed.owns_original_tokens(tokens),
                "configuration {change}"
            );
        }
        assert!(retained.owns_original_tokens(tokens));
    }

    #[test]
    fn original_source_token_owner_joins_the_complete_retained_baseline() {
        // naming.source.original-direct-token-owner
        // docs/design/analysis/name-resolution-proofs/original-direct-token-owner.md
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let unit = crate::compilation_unit::CompilationUnit::build_for("set x 0", &registry, false);
        let script = &unit.ir_module.top_level;
        let tokens = script
            .retained_source_tokens_for_statement(script.statements.last().unwrap())
            .unwrap();
        let retained = unit.ir_module.retained_source_bindings.as_deref().unwrap();
        let foreign = CommandRegistry::build_default();
        for change in 0..6 {
            let mut changed = tokens.clone();
            let snapshot = Arc::make_mut(
                changed
                    .source_binding
                    .as_mut()
                    .unwrap()
                    .lookup_state
                    .as_mut()
                    .unwrap(),
            );
            let baseline = Arc::make_mut(&mut snapshot.state.baseline);
            match change {
                0 => {
                    baseline.metadata_context =
                        crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable
                }
                1 => baseline.unknown_entry = !baseline.unknown_entry,
                2 => baseline.native_compilation.loop_depth += 1,
                3 => {
                    baseline.invocation_realm =
                        tcl_dialect::model::InvocationRealm::InterpreterRuntime
                }
                4 => baseline.registry_snapshot = Some(foreign.snapshot().semantic_key()),
                _ => {
                    baseline.native_compilation.mode =
                        tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject
                }
            }
            assert!(
                !retained.owns_original_tokens(&changed),
                "baseline {change}"
            );
        }
        assert!(retained.owns_original_tokens(tokens));
    }

    #[test]
    fn positioned_module_projection_shares_the_actual_producer_and_detaches_writes() {
        // naming.source.retained-positioned-command-realm
        // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let module = lowered(registry);
        let retained = module.retained_source_bindings.as_ref().unwrap();
        let original = retained.positioned_for_module(&module, registry).unwrap();
        assert!(std::ptr::eq(original, retained.positioned.as_ref()));
        let sibling = module.clone();
        assert!(Arc::ptr_eq(
            &retained.positioned,
            &sibling
                .retained_source_bindings
                .as_ref()
                .unwrap()
                .positioned,
        ));
        let mut mutable = original.clone();
        assert!(Arc::ptr_eq(&mutable.final_state, &original.final_state));
        assert!(mutable.points.shares_storage(&original.points));
        assert!(
            mutable
                .declaration_layouts
                .shares_storage(&original.declaration_layouts)
        );
        Arc::make_mut(&mut mutable.final_state).mark_opaque_binding_mutation();
        assert!(mutable.final_state.opaque_binding_mutation);
        assert!(!original.final_state.opaque_binding_mutation);
        assert!(!Arc::ptr_eq(&mutable.final_state, &original.final_state));
        assert!(std::ptr::eq(
            original,
            retained.positioned_for_module(&module, registry).unwrap(),
        ));
    }

    #[test]
    fn positioned_module_projection_refuses_each_changed_original_input() {
        // naming.source.retained-positioned-command-realm
        // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let original = lowered(registry);
        let retained = original.retained_source_bindings.as_ref().unwrap();
        assert!(
            retained
                .positioned_for_module(&original, registry)
                .is_some()
        );
        let mutations: &[fn(&mut Module)] = &[
            |module| module.source = tcl_lexer::SourceImage::document("return OTHER"),
            |module| module.source = tcl_lexer::SourceImage::native(module.source.bytes()),
            |module| module.lexer_config.strict_quoting = !module.lexer_config.strict_quoting,
            |module| module.top_level_namespace = "::Other".to_owned(),
            |module| {
                module.native_namespace = Some(tcl_core_types::ByteNamespacePath::from_segments([
                    b"Other".as_slice(),
                ]))
            },
            |module| {
                module.top_level_namespace_context =
                    Some(super::super::SourceNamespaceKey::authored("::Different"))
            },
            |module| module.top_level_kind = TopLevelKind::ProcedureBody,
            |module| module.source_entry.unknown_entry = !module.source_entry.unknown_entry,
            |module| {
                module.source_entry.compilation_scope =
                    tcl_runtime_api::SourceCompilationScope::EnteredSource
            },
            |module| module.source_entry.native_compilation.loop_depth += 1,
            |module| module.registry_snapshot = None,
        ];
        for (index, mutate) in mutations.iter().enumerate() {
            let mut changed = original.clone();
            mutate(&mut changed);
            assert!(
                retained.positioned_for_module(&changed, registry).is_none(),
                "input {index}"
            );
        }
        let foreign = tcl_registry::model::ingress::static_context_for("tcl8.4").commands();
        assert!(retained.positioned_for_module(&original, foreign).is_none());
        let mut replaced_snapshot = original.clone();
        replaced_snapshot.registry_snapshot = Some(foreign.snapshot());
        assert!(
            retained
                .positioned_for_module(&replaced_snapshot, registry)
                .is_none()
        );
    }

    #[test]
    fn shared_projection_equals_fresh_original_source_in_every_engine_profile() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let registry = tcl_registry::model::ingress::static_context_for(engine).commands();
            let module = lowered(registry);
            assert!(module.retained_source_bindings.is_some(), "{engine}");
            let reused = ModuleCommandBindings::analyse(&module, registry);
            let mut fresh = module;
            fresh.retained_source_bindings = None;
            assert_eq!(
                reused,
                ModuleCommandBindings::analyse(&fresh, registry),
                "{engine}"
            );
        }
    }

    #[test]
    fn original_input_changes_withdraw_the_shared_projection() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let original = lowered(registry);
        let retained = original.retained_source_bindings.as_ref().unwrap();
        let mutations: &[fn(&mut Module)] = &[
            |module| module.source = tcl_lexer::SourceImage::document("return OTHER"),
            |module| {
                module.source = tcl_lexer::SourceImage::native(module.source.bytes());
            },
            |module| module.lexer_config.strict_quoting = !module.lexer_config.strict_quoting,
            |module| module.top_level_namespace = "::Other".to_owned(),
            |module| {
                module.top_level_namespace_context = Some(
                    super::super::SourceNamespaceKey::authored("::DifferentTypedNamespace"),
                );
            },
            |module| module.top_level_kind = TopLevelKind::ProcedureBody,
            |module| module.source_entry.unknown_entry = !module.source_entry.unknown_entry,
            |module| {
                module.source_entry.compilation_scope =
                    tcl_runtime_api::SourceCompilationScope::EnteredSource;
            },
            |module| module.source_entry.native_compilation.loop_depth += 1,
        ];
        for (index, mutate) in mutations.iter().enumerate() {
            let mut changed = original.clone();
            mutate(&mut changed);
            assert!(
                retained
                    .projection(&changed, registry, changed.source_entry.options())
                    .is_none(),
                "input {index}"
            );
        }
        let other = tcl_registry::model::ingress::static_context_for("tcl8.4").commands();
        assert!(
            retained
                .projection(&original, other, original.source_entry.options())
                .is_none()
        );
    }

    #[test]
    fn override_entry_contract_cannot_reuse_original_lowering() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let module = lowered(registry);
        let retained = module.retained_source_bindings.as_ref().unwrap();
        let mut options = module.source_entry.options();
        options.unknown_entry = !options.unknown_entry;
        assert!(retained.projection(&module, registry, options).is_none());
    }
}

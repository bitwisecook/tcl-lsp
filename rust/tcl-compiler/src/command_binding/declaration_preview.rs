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

//! Original procedure-body layouts independent of command installation.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use super::declaration_layout::original_declaration_layouts;
#[cfg(test)]
use super::declaration_layout::{
    DeclarationLayoutIssuer, DeclarationLayoutObservation, OriginalDiagnosticFrameEntry,
};
use super::{SourceCommandBindings, SourceLookupSnapshot};
use crate::registry_invocation::EffectiveInvocationWord;
#[cfg(test)]
use crate::registry_invocation::effective_invocation_word;
use tcl_names::command::CommandAllocationSite;
use tcl_names::origin::ExecutedScriptSource;

pub(super) mod logical_body_entry;
mod source_cut_body;
mod static_read_child;
pub(crate) use source_cut_body::OriginalSourceCutBodyRows;

/// A positioned conditional declaration recipe, never an implementation allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclaredProcedureBody {
    declaration: CommandAllocationSite,
    candidate: super::SourceCommandTarget,
    slot: tcl_names::command::SourceCommandKey,
    pub(super) source: ExecutedScriptSource,
    pub(super) namespace: tcl_names::namespace::SourceNamespaceKey,
    pub(super) parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    logical_name: Option<String>,
    logical_parameters_text: Option<String>,
    logical_parameters_word: Option<tcl_lexer::NativeWord>,
    pub(super) original_parameters: Option<super::formal_topology::OriginalFormalTopology>,
    pub(super) frame: tcl_names::variable::VariableExecutionFrame,
    authored_original_body_word: Option<tcl_lexer::NativeWord>,
}

#[derive(Debug, PartialEq, Eq)]
struct OriginalLogicalProcedureStaticFacet {
    input: crate::analyser::ResolvedAnalysisInput,
    source: Arc<tcl_names::origin::SourceOriginId>,
    frame: tcl_names::variable::VariableExecutionFrame,
    namespace: tcl_names::namespace::SourceNamespaceKey,
    variables: Arc<crate::var_resolve::ResolveContext>,
    config: tcl_lexer::LexerConfig,
    original: Vec<tcl_lexer::NativeWord>,
    effective: crate::registry_invocation::EffectiveCommandWords,
    values: Vec<String>,
    statics: Option<(
        usize,
        Vec<tcl_registry::native_procedure::StaticVariableDeclaration>,
    )>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OriginalAuthoredDeclarationSourceObligation {
    PrefixEffects,
    HandlerLookup,
    ObserverApplicability,
}

/// Original declaration geometry with independent, unclosed source applicability.
/// The original row supplies syntax, not an entered lookup or completed prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalAuthoredProcedureSourceGeometry {
    header: crate::ir::CommandTokens,
    declaration: super::source_transition_advice::OriginalConditionalProcedureDeclaration,
    namespace: tcl_names::namespace::SourceNamespaceKey,
    name: String,
    parameters_text: String,
    parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    body_word: tcl_lexer::NativeWord,
    config: tcl_lexer::LexerConfig,
    snapshot: Arc<SourceLookupSnapshot>,
    obligations: [OriginalAuthoredDeclarationSourceObligation; 3],
}

impl OriginalAuthoredProcedureSourceGeometry {
    pub(crate) fn body_word(&self) -> &tcl_lexer::NativeWord {
        &self.body_word
    }
    pub(crate) fn namespace(&self) -> &tcl_names::namespace::SourceNamespaceKey {
        &self.namespace
    }
    pub(crate) fn name(&self) -> &str {
        self.name.as_str()
    }
    pub(crate) fn parameters_text(&self) -> &str {
        self.parameters_text.as_str()
    }
    pub(crate) fn parameters(&self) -> &[tcl_syntax::formal_params::FormalParameter] {
        &self.parameters
    }
    pub(crate) fn reporting_label(&self) -> String {
        self.declaration.label.clone()
    }
    pub(crate) fn config(&self) -> tcl_lexer::LexerConfig {
        self.config
    }
    pub(crate) fn registry_snapshot(&self) -> Option<&tcl_registry::RegistrySnapshot> {
        self.snapshot.state.baseline.registry_owner.as_ref()
    }
    pub(crate) fn realm(&self) -> tcl_dialect::model::InvocationRealm {
        self.snapshot.realm
    }
    pub(crate) fn has_unclosed_source_coverage(&self) -> bool {
        !self.obligations.is_empty()
    }
}

impl super::SourceInvocationBinding {
    /// Eligibility of a present owner remains separate from conditional syntax.
    fn original_authored_declaration_point_eligibility(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<()> {
        // A present reached metadata owner remains terminal even though its
        // withdrawn physical layout does not issue this source-only receipt.
        if let Some(point) = &self.lookup_state {
            let baseline = &point.state.baseline;
            if !baseline.metadata_context.is_standalone()
                || baseline.native_entry.is_some()
                || baseline.logical_source_input.is_some()
                || baseline.vendor_source_input.is_some()
                || baseline.hosted_execution_context.is_some()
                || baseline.unknown_entry
                || point
                    .state
                    .current_source_origin
                    .as_ref()
                    .is_none_or(|source| {
                        self.invocation_site()
                            .is_none_or(|site| !Arc::ptr_eq(source, &site.source))
                    })
            {
                return None;
            }
            let retained = self.retained_registry_for_tokens(tokens)?;
            if retained.snapshot().semantic_key() != registry.snapshot().semantic_key() {
                return None;
            }
            self.original_invocation_metadata_at_point(tokens, registry)
                .ok()?;
            // A complete ordinary layout keeps its existing closed route.
            // Missing layout is not itself positive source authority.
            if self
                .original_authored_source_layout(tokens, registry)
                .is_some_and(|layout| !layout.has_unclosed_source_coverage())
            {
                return None;
            }
        }
        Some(())
    }

    pub(crate) fn original_authored_procedure_source_geometry(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<OriginalAuthoredProcedureSourceGeometry> {
        if self.has_original_native_source_command()
            || self.has_original_logical_source_advice()
            || tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
        {
            return None;
        }
        self.original_authored_declaration_point_eligibility(tokens, registry)?;
        let site = self.invocation_site()?;
        let config = self.original_lexer_config_for_tokens(tokens)?;
        let owner = self.original_analysis_owner.as_ref()?;
        if !Arc::ptr_eq(&owner.source, &site.source)
            || owner.config != config
            || !owner.metadata.is_standalone()
        {
            return None;
        }
        let rows = original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        let mut agreed = None;
        for row in rows {
            if row.config != config
                || row.words.as_ref() != tokens.words()
                || !row.snapshot.state.baseline.metadata_context.is_standalone()
                || row
                    .snapshot
                    .state
                    .baseline
                    .registry_owner
                    .as_ref()?
                    .semantic_key()
                    != registry.snapshot().semantic_key()
            {
                return None;
            }
            let declaration =
                super::source_transition_advice::original_conditional_procedure_declaration(
                    site, tokens, row, owner, registry,
                )?;
            if config.list_parse != tcl_dialect::ListParse::Strict {
                return None;
            }
            let name =
                tcl_syntax::word_rules::original_static_word_unicode_value(&declaration.name)?;
            let name = std::str::from_utf8(&name).ok()?.to_owned();
            let parameters_text = tcl_syntax::word_rules::original_static_word_unicode_value(
                &declaration.parameters,
            )?;
            let parameters_text = std::str::from_utf8(&parameters_text).ok()?.to_owned();
            let parameters =
                super::logical_definition::static_formals(&parameters_text, config.list_parse)?;
            let namespace =
                tcl_names::namespace::SourceNamespaceKey::authored_path(&declaration.namespace)?;
            let body_word = declaration.body.original_container().clone();
            let geometry = OriginalAuthoredProcedureSourceGeometry {
                header: tokens.clone(),
                declaration,
                namespace,
                name,
                parameters_text,
                parameters,
                body_word,
                config,
                snapshot: Arc::clone(&row.snapshot),
                obligations: [
                    OriginalAuthoredDeclarationSourceObligation::PrefixEffects,
                    OriginalAuthoredDeclarationSourceObligation::HandlerLookup,
                    OriginalAuthoredDeclarationSourceObligation::ObserverApplicability,
                ],
            };
            if agreed.as_ref().is_some_and(|old| old != &geometry) {
                return None;
            }
            agreed = Some(geometry);
        }
        agreed
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeclaredProcedureHeader {
    pub(super) source_slot: tcl_core_types::ByteCommandSlot,
    pub(super) name_input: crate::signature_scan::scope::SignatureSourceNameInput,
    pub(super) declaration: CommandAllocationSite,
    candidate: super::SourceCommandTarget,
    slot: tcl_names::command::SourceCommandKey,
    pub(super) namespace: tcl_names::namespace::SourceNamespaceKey,
    pub(super) parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    pub(super) original_parameters: Option<super::formal_topology::OriginalFormalTopology>,
    pub(super) frame: tcl_names::variable::VariableExecutionFrame,
    written: usize,
    value: String,
}

/// Checked original body scope for conditional source diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OriginalProcedureReadFlowEntry {
    pub(super) header: DeclaredProcedureHeader,
    pub(super) read: OriginalProcedureReadDeclaration,
    pub(super) snapshot: Arc<super::SourceLookupSnapshot>,
}

/// An original declaration may route a source read through a captured static
/// slot rather than the synthetic local used by an `OriginalDeclaration` CFG.
/// This receipt can withdraw that local-absence claim only; it supplies no
/// existing value, entered frame, constructor completion or physical cell.
pub(crate) struct OriginalProcedureStaticReadAlternatives {
    source_analysis: std::sync::Weak<super::SourceCommandBindings>,
    owner: Arc<super::OriginalSourceAnalysisOwner>,
    input: crate::analyser::ResolvedAnalysisInput,
    declaration: OriginalProcedureReadFlowEntry,
    names: std::collections::BTreeSet<String>,
    original_body_plan: std::sync::OnceLock<Option<tcl_lexer::NativeScriptWordsPlan>>,
}

macro_rules! static_read_guard {
    ($stage:literal, $value:expr) => {
        match $value {
            Some(value) => value,
            None => return trace_static_read_refusal($stage, None),
        }
    };
}

mod declared_body;
mod nested_layouts;
mod procedure_header;
mod read_declaration;
mod source_operand_layout;
mod static_read_alternatives;

use declared_body::declared_body;
pub(super) use declared_body::declared_procedure_frame;
use nested_layouts::NestedLayoutSyntaxKey;
pub(super) use nested_layouts::retains_successor_layout;
#[cfg(test)]
use nested_layouts::{
    NestedDeclarationLayoutKey, NestedDeclarationLayoutVisits, parent_declaration_layouts,
    unanimous_nested_layouts, unanimous_nested_layouts_from_parents,
};
pub(super) use procedure_header::declaration_tokens;
pub(crate) use read_declaration::OriginalProcedureReadDeclaration;
pub(super) use source_operand_layout::{
    OriginalDeclarationSourceOperandLayout, OriginalDeclarationSourceOperation,
};
use static_read_alternatives::trace_static_read_refusal;

/// A sealed declaration in the independently retained Logical source model.
/// It describes an original source slot, strict formals and deferred body;
/// it grants no Native publication, formal ABI, entered frame or completion.
#[derive(Debug, Clone)]
pub(crate) struct LogicalProcedureDeclaration {
    body: DeclaredProcedureBody,
}

/// Derived whole-word source layouts never participate in execution identity.
#[derive(Debug, Default)]
pub(super) struct OriginalDeclarationSourceOperandCache {
    pub(super) entries:
        std::sync::Mutex<BTreeMap<u32, Option<Arc<OriginalDeclarationSourceOperandLayout>>>>,
    final_state: std::sync::OnceLock<std::sync::Weak<super::ModuleCommandBindings>>,
}

/// A source-only stamp against one actual retained declaration inventory. Weak
/// identity never traverses cached facts, body snapshots or invocation bindings.
#[derive(Debug, Clone)]
pub(super) struct OriginalDeclarationSourceOperandAttachment {
    cache: std::sync::Weak<OriginalDeclarationSourceOperandCache>,
    offset: u32,
}

struct NestedLayoutCommand {
    site: CommandAllocationSite,
    tokens: Box<crate::ir::CommandTokens>,
    /// Shared by every visit: a braced body word carries its whole body text.
    words: Arc<[tcl_source::words::WordExpr]>,
}

/// A collector-local source tape, with no selected invocation or read/store facts.
#[derive(Default)]
struct NestedLayoutSyntax {
    commands: HashMap<NestedLayoutSyntaxKey, Option<Arc<[NestedLayoutCommand]>>>,
    #[cfg(test)]
    parsed_scripts: usize,
}

/// One original command layout in its complete immutable lookup world.
/// Children and continuation share this derivation; it grants no body entry.
struct OriginalDeclarationLayout {
    advice: super::OriginalCompilationLookupAdvice,
    flow: crate::registry_invocation::DeclarationInvocationFlow,
}

struct ProcedureOperands<'a> {
    effective: &'a crate::registry_invocation::EffectiveCommandWords,
    values: &'a [EffectiveInvocationWord],
    dialect: tcl_native_model::InvocationDialect,
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_argument_declaration_tests;

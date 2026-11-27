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

//! The original name of an SSA definition, the source definition of a body, fresh scalar proposals and activation-cell ownership.

use super::super::{Symbol, supplied_ssa_metadata};
use super::{OriginalSourceSsaDefinitionName, SsaSourceView};
use tcl_names::variable::VariableCellKey;
use tcl_registry::CommandRegistry;

/// Source name of one selected SSA value definition. This pairs an actual
/// definition cell with its retained written operand; it supplies no value,
/// current-read, rename or native representation capability.
#[derive(Debug, Clone)]
pub struct OriginalSsaDefinitionName {
    pub(in super::super) span: tcl_lexer::Span,
    pub(super) input: crate::signature_scan::scope::SignatureSourceNameInput,
    pub(in super::super) cell: VariableCellKey,
}

impl OriginalSsaDefinitionName {
    /// Complete written receiver extent in this function's source coordinates.
    #[must_use]
    pub const fn span(&self) -> tcl_lexer::Span {
        self.span
    }

    /// Independently retained source producer at the actual receiver ordinal.
    #[must_use]
    pub const fn original_name_input(
        &self,
    ) -> &crate::signature_scan::scope::SignatureSourceNameInput {
        &self.input
    }

    /// Actual SSA cell selected at this definition, without current contents.
    #[must_use]
    pub const fn cell(&self) -> &VariableCellKey {
        &self.cell
    }
}

/// Point-owned absence of a proposed ordinary scalar in an actually entered
/// local activation. The original operation/frame and name policy are retained
/// independently; this grants no new store, value, normal completion, source
/// movement or edit permission.
#[derive(Debug, Clone)]
pub struct OriginalFreshScalarVariableProposal {
    pub(in super::super) slot: crate::var_resolve::FreshScalarVariableSlot,
    site: tcl_names::command::CommandAllocationSite,
    pub(in super::super) config: tcl_lexer::LexerConfig,
}

impl OriginalFreshScalarVariableProposal {
    /// Proposed native name units with no display re-encoding.
    #[must_use]
    pub fn name(&self) -> &[u8] {
        self.slot.name()
    }
    /// Exact selected activation slot at this original operation point.
    #[must_use]
    pub fn cell(&self) -> &VariableCellKey {
        self.slot.cell()
    }
    /// Genuine original invocation at which absence was checked.
    #[must_use]
    pub fn site(&self) -> &tcl_names::command::CommandAllocationSite {
        &self.site
    }
    /// Independently selected native naming purpose.
    #[must_use]
    pub fn policy(&self) -> tcl_syntax::naming::NamePolicyProtocol {
        self.slot.policy()
    }
    /// Full retained source parser configuration.
    #[must_use]
    pub const fn lexer_config(&self) -> tcl_lexer::LexerConfig {
        self.config
    }
}

/// Point-owned insertion naming plan retaining both independent local-slot
/// availability and the actual registered scalar setter selection. It grants
/// no store completion, native preparation, value or source motion.
#[derive(Debug, Clone)]
pub struct OriginalFreshScalarAssignmentProposal {
    pub(in super::super) variable: OriginalFreshScalarVariableProposal,
    pub(in super::super) command: crate::command_binding::OriginalScalarAssignmentCommand,
}

impl OriginalFreshScalarAssignmentProposal {
    /// Proposed unoccupied scalar slot in the actual entered local frame.
    #[must_use]
    pub const fn variable(&self) -> &OriginalFreshScalarVariableProposal {
        &self.variable
    }

    /// Independently selected current registered setter and authored word.
    #[must_use]
    pub const fn command(&self) -> &crate::command_binding::OriginalScalarAssignmentCommand {
        &self.command
    }
}

#[cfg(debug_assertions)]
fn trace_original_definition_place(
    tokens: &crate::ir::CommandTokens,
    context: &crate::var_resolve::ResolveContext,
    written: usize,
    place: &tcl_names::place::Place,
    key: &tcl_names::variable::VariableCellKey,
) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_SSA_DEFINITION").is_some() {
        let selected = crate::var_resolve::canonical_binding_value_key(place);
        eprintln!(
            "original_definition site={} written={} policy={} frame={:?} place={:?} dynamic={} selected={} matches={} generation={:?}",
            tokens
                .words()
                .first()
                .map_or(0, |word| word.source().span.start()),
            written,
            context.execution_name_policy.is_some(),
            context.frame_kind,
            place.kind,
            place.dynamic,
            selected.is_some(),
            selected.as_ref() == Some(key),
            place.cell.as_ref().map(|cell| cell.generation)
        );
        if let (
            Some(selected),
            tcl_names::variable::VariableCellKey::Namespace { identity, simple },
        ) = (selected.as_ref(), key.root())
            && let tcl_names::variable::VariableCellKey::Namespace {
                identity: selected_identity,
                simple: selected_simple,
            } = selected.root()
        {
            eprintln!(
                "original_definition mismatch namespace_equal={} selected_name={:?} definition_name={:?} selected_lifetime={} definition_lifetime={}",
                selected_identity == identity,
                selected_simple.as_bytes(),
                simple.as_bytes(),
                matches!(
                    selected,
                    tcl_names::variable::VariableCellKey::Lifetime { .. }
                ),
                matches!(key, tcl_names::variable::VariableCellKey::Lifetime { .. }),
            );
        }
    }
}

impl SsaSourceView<'_> {
    /// Match an SSA definition under explicitly standalone catalogue selection.
    /// Supplied-input consumers use `original_definition_name_with_metadata_context`.
    /// Definition phase, effective-to-written ordinal and byte-cell identity
    /// come from the retained normal-transfer and original operand owners.
    /// Equal names, catalogue roles or a source span cannot issue this mapping.
    #[must_use]
    pub fn original_definition_name(
        self,
        symbol: Symbol,
        registry: &CommandRegistry,
    ) -> Option<OriginalSsaDefinitionName> {
        let metadata = registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile)
            .map(crate::registry_invocation::InvocationMetadataContext::from);
        self.original_definition_name_with_metadata_context(symbol, registry, metadata)
    }

    /// Join the exact original SSA definition under supplied availability and
    /// source grammar. Missing, foreign or incompatible metadata stays absent;
    /// physical cell and successful-transfer requirements remain independent.
    #[must_use]
    pub fn original_definition_name_with_metadata_context(
        self,
        symbol: Symbol,
        registry: &CommandRegistry,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    ) -> Option<OriginalSsaDefinitionName> {
        use tcl_registry::native_compilation::VariableOperandBindingPhase;
        let (block, index) = self.point?;
        let statement = self.ssa.blocks.get(&block)?.statements.get(index)?;
        statement.defs.get(&symbol)?;
        let points = self.ssa.point_contexts.as_ref()?;
        let before = points.context_before(block, index)?;
        let tokens = self.source_tokens()?;
        if tokens.synthetic.is_some() {
            return None;
        }
        let binding = tokens.source_binding.as_ref()?;
        let metadata = supplied_ssa_metadata(
            registry,
            binding.original_lexer_config_for_tokens(tokens)?,
            metadata,
        )?;
        let selected_invocation =
            crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                registry,
                Some(metadata),
                tokens,
            );
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_SSA_DEFINITION").is_some() {
            eprintln!(
                "original_definition admission site={} normal={} operand_owner={} before_policy={} key_kind={:?}",
                tokens
                    .words()
                    .first()
                    .map_or(0, |word| word.source().span.start()),
                selected_invocation.is_some(),
                binding
                    .original_variable_operands_for_tokens(tokens)
                    .is_some(),
                before.execution_name_policy.is_some(),
                match self.ssa.cell_key(symbol)?.root() {
                    tcl_names::variable::VariableCellKey::Activation { .. } => "activation",
                    tcl_names::variable::VariableCellKey::Namespace { .. } => "namespace",
                    tcl_names::variable::VariableCellKey::Authored(_) => "authored",
                    _ => "other",
                }
            );
        }
        let invocation = selected_invocation?;
        let context = match invocation.variable_binding_phase() {
            VariableOperandBindingPhase::AfterArguments => before,
            VariableOperandBindingPhase::NormalContinuation => binding
                .normal_variable_continuation()
                .unwrap_or_else(|| points.after_statement(block, index)),
            VariableOperandBindingPhase::BodyProtocol => return None,
        };
        let key = self.ssa.cell_key(symbol)?;
        let mut found = None;
        for (written, place) in invocation.written_definition_places(context, registry) {
            #[cfg(debug_assertions)]
            trace_original_definition_place(tokens, context, written, &place, key);
            if crate::var_resolve::canonical_binding_value_key(&place).as_ref() != Some(key) {
                continue;
            }
            let written = written.checked_add(1)?;
            let input = binding.original_written_name_input(tokens, written)?;
            let span = tokens.words().get(written)?.source().span;
            if found
                .as_ref()
                .is_some_and(|(old_span, old_input)| *old_span != span || old_input != &input)
            {
                return None;
            }
            found = Some((span, input));
        }
        let (span, input) = found?;
        Some(OriginalSsaDefinitionName {
            span,
            input,
            cell: key.clone(),
        })
    }

    /// Readonly definition geometry from the actual Native source body/schema.
    /// This separate source projection cannot replace a physical definition query.
    #[must_use]
    pub fn original_source_definition_for_body(
        self,
        symbol: Symbol,
        unit: &crate::compilation_unit::CompilationUnit,
        body: crate::ir::ProcedureId,
        registry: &CommandRegistry,
    ) -> Option<OriginalSourceSsaDefinitionName> {
        let function = unit.function_for_body(Some(body))?;
        if !std::ptr::eq(self.ssa, std::ptr::from_ref(&function.ssa)) {
            return None;
        }
        let module = &unit.ir_module;
        let card = module.original_native_declaration_body_units.get(&body)?;
        let procedure = module.body_units.get(&body)?;
        if !card.matches_procedure(module, procedure) {
            return None;
        }
        let input = module.source_metadata_input.as_ref()?;
        let metadata = function.invocation_metadata_context_for_module(registry, module)?;
        let function_input = metadata.source_analysis_input()?;
        if (function_input != input && function_input != &input.for_nested_source())
            || function.source_lexer_config().nested().normalized()
                != input.lexer_config().nested().normalized()
        {
            return None;
        }
        let (block, index) = self.point?;
        let statement = self.ssa.blocks.get(&block)?.statements.get(index)?;
        if !statement.defs.contains_key(&symbol) || statement.may_defs.contains(&symbol) {
            return None;
        }
        let slot = self.ssa.cell_key(symbol)?.authored_spelling()?;
        let mut tokens = self.source_tokens()?.clone();
        crate::lattice_rebase::rebase_command_tokens(&mut tokens, function.base_offset);
        let binding = tokens.source_binding.as_ref()?;
        if !binding.original_native_source_owner_for_body(&tokens, module, body, registry) {
            return None;
        }
        let words = binding.original_native_source_words(&tokens, input, registry)?;
        let mut found = None;
        for (argument, role) in words.written_argument_roles() {
            if role != tcl_registry::ArgRole::VarWrite {
                continue;
            }
            let operand = words.operands().get(argument)?.as_ref()?;
            let name = operand.input()?;
            let selected = name.policy().recipe().combined_variable_input(name.bytes());
            if selected.element().is_some()
                || selected.root().qualification()
                    != tcl_syntax::naming::NativeNameQualification::Unqualified
                || selected.root().selected() != slot.as_bytes()
            {
                continue;
            }
            let current = (operand.span(), name.clone());
            if found.as_ref().is_some_and(|previous| previous != &current) {
                return None;
            }
            found = Some(current);
        }
        let (span, input) = found?;
        Some(OriginalSourceSsaDefinitionName { span, input, body })
    }

    /// Propose an unused scalar slot at this genuine reached source operation.
    /// Complete entered local ownership and observer closure are required;
    /// namespace/global/event availability is deliberately not inferred.
    #[must_use]
    pub fn fresh_scalar_variable_proposal(
        self,
        native_name: &[u8],
        registry: &CommandRegistry,
    ) -> Option<OriginalFreshScalarVariableProposal> {
        let (block, index) = self.point?;
        let context = self
            .ssa
            .point_contexts
            .as_ref()?
            .context_before(block, index)?;
        let tokens = self.source_tokens()?;
        if tokens.synthetic.is_some() {
            return None;
        }
        let binding = tokens.source_binding.as_ref()?;
        if binding.runtime_reachability()
            != crate::command_binding::SourceRuntimeReachability::Reached
        {
            return None;
        }
        let site = binding.invocation_site()?.clone();
        let config = binding.original_lexer_config_for_tokens(tokens)?;
        let slot = context.fresh_scalar_variable_slot(native_name, registry)?;
        Some(OriginalFreshScalarVariableProposal { slot, site, config })
    }

    /// Propose an unused local scalar and independently select its current
    /// registered setter at the same original source operation. Neither
    /// receipt can replace the other's point, policy or parser configuration.
    #[must_use]
    pub fn fresh_scalar_assignment_proposal(
        self,
        native_name: &[u8],
        registry: &CommandRegistry,
    ) -> Option<OriginalFreshScalarAssignmentProposal> {
        let variable = self.fresh_scalar_variable_proposal(native_name, registry)?;
        let tokens = self.source_tokens()?;
        let binding = tokens.source_binding.as_ref()?;
        let command = binding.original_scalar_assignment_command(tokens, registry)?;
        if command.site() != variable.site()
            || command.lexer_config() != variable.lexer_config()
            || command.policy() != variable.policy()
            || !command.matches_original_point(binding)
        {
            return None;
        }
        Some(OriginalFreshScalarAssignmentProposal { variable, command })
    }

    /// Whether an activation-owned dependency belongs to this operation's
    /// actual frame. Callee summaries retain their private cells for effects;
    /// those cells do not become lexical reads in the caller. Missing frame
    /// provenance and non-activation cells provide no membership conclusion.
    #[must_use]
    pub(crate) fn owns_activation_cell(self, symbol: Symbol) -> Option<bool> {
        let VariableCellKey::Activation { identity, .. } = self.ssa.cell_key(symbol)?.root() else {
            return None;
        };
        let (block, index) = self.point?;
        let context = self
            .ssa
            .point_contexts
            .as_ref()?
            .context_before(block, index)?;
        Some(context.activation.as_ref()? == identity)
    }
}

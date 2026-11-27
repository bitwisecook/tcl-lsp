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

//! Target-neutral proof plans for procedure calls and materialisable SSA slots.
//!
//! This module consumes existing compiler lattices. It does not lower target
//! instructions and it does not recognise Tcl commands by spelling. Procedure
//! identities come from the lowered module, command mutation and trace hazards
//! come from their shared analyses, and formal-list semantics come from the
//! shared independently selected parameter grammar.

#[cfg(test)]
use std::collections::HashMap;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
use tcl_registry::SemanticOperationId;
#[cfg(test)]
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::model::semantic::SemanticContext;

use crate::cfg::BlockId;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::{CommandTokens, Statement};
use crate::registry_invocation::InvocationMetadataContext;
use crate::representation_plan::{SharingState, VarStorage};
use crate::semantic_optimisation::{SemanticOptimisationConfig, SemanticOptimisationPassId};
use crate::ssa::Symbol;
#[cfg(test)]
use crate::types::TypeKind;
use crate::types::{TypeLattice, TypeShape};
use crate::var_escape::{EscapeTag, ProcEscapeSummary, analyse_var_escape_cu_with_registry};
#[cfg(test)]
use tcl_names::command::BindingKind;

mod authored_source;
pub use authored_source::{
    OriginalAuthoredProgramSourceDecline, OriginalAuthoredProgramSourcePlan,
    OriginalAuthoredSourceResidual, OriginalAuthoredSourceStatement,
    OriginalAuthoredSourceStatementKind, OriginalAuthoredStorageTemplate,
};
mod declared_arguments;
pub use declared_arguments::{
    DeclaredArgumentDecision, DeclaredArgumentEvidence, DeclaredArgumentIdentity,
};

mod closed_program_coverage;
mod direct_calls;
mod materialisable_slots;
mod native_premises;
mod semantic_calls;

use closed_program_coverage::prove_closed_program_coverage;
pub use closed_program_coverage::{
    CfgStatementId, ClosedProgramCoverageDecision, ClosedProgramCoverageDecline,
    ClosedProgramCoverageEvidence, ClosedProgramStatementEvidence,
};
use direct_calls::collect_direct_calls;
pub(crate) use direct_calls::direct_call_argument_read;
pub use direct_calls::{
    DirectActualValue, DirectProcBodyDecision, DirectProcBodyDecline, DirectProcBodyEvidence,
    DirectProcDecision, DirectProcDecline, DirectProcEvidence, direct_call_argument_value,
    original_direct_expression_body_operations,
};
#[cfg(test)]
use direct_calls::{
    actual_fact, direct_call_tokens, direct_formal_bindings, propagate_actual_types,
};
use materialisable_slots::collect_materialisable_slots;
pub use materialisable_slots::{
    MaterialisableSlotAuthority, MaterialisableSlotDecision, MaterialisableSlotDecline,
    MaterialisableSlotEvidence, MaterialisationGuardRequirements, MaterialisationRecipe,
};
pub use native_premises::{NativeDecline, NativeDeclineReason, NativePremise};
use semantic_calls::collect_semantic_calls;
pub use semantic_calls::{
    SemanticCallArgument, SemanticCallDecision, SemanticCallDecline, SemanticCallEvidence,
    SemanticFormIdentity, semantic_operation_binding_is_trusted,
};

/// Stable identity of one CFG invocation, including an immediate command
/// substitution nested in one argument of the enclosing statement.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DirectCallSiteId {
    /// Genuine retained Module issuer, including the top-level body owner.
    pub module_id: tcl_core_types::ProcedureModuleId,
    /// Actual retained caller body; `None` denotes the top level.
    pub body_id: Option<crate::ir::ProcedureId>,
    /// Caller display, retained separately from body identity.
    pub function: String,
    /// CFG block identity.
    pub block: BlockId,
    /// Statement position in the block.
    pub statement_index: u32,
    /// `None` for the statement call itself, or the argument index containing
    /// the bracketed call.
    pub nested_argument: Option<u32>,
}

/// Stable source identity of an in-unit Tcl procedure definition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcedureIdentity {
    /// Exact Module-issued declaration identity, independent of its label.
    pub id: crate::ir::ProcedureId,
    /// Declaration display; no callable name or allocation authority.
    pub qualified_name: String,
    /// Definition source-span start.
    pub definition_start: u32,
    /// Definition source-span end.
    pub definition_end: u32,
}

/// Stable identity of one SSA value in one procedure.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SsaValueIdentity {
    /// Genuine retained Module issuer, including the top-level body owner.
    pub module_id: tcl_core_types::ProcedureModuleId,
    /// Actual retained body owning the local SSA namespace.
    pub body_id: Option<crate::ir::ProcedureId>,
    /// Body display; local symbols are joined through `body_id`.
    pub function: String,
    /// Function-local interned variable symbol.
    pub symbol: Symbol,
    /// SSA version.
    pub version: u32,
}

/// Whether compiled code is hosted by a live interpreter or owns the whole
/// Tcl program and its final state.
///
/// This is a proof premise, not a backend option. Top-level variables remain
/// observable after `::top` returns in [`Self::Hosted`], so that environment
/// can never infer native-only storage merely from local type information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonAotEnvironment {
    /// A caller may inspect or mutate interpreter state after compiled code.
    Hosted,
    /// The compiled program owns the interpreter lifetime and final state.
    SealedProgram,
}

impl CommonAotEnvironment {
    /// Whether this environment can participate in a later native-only proof.
    /// Escape, trace, representation, and boundary proofs remain mandatory.
    #[must_use]
    pub const fn permits_native_only(self) -> bool {
        matches!(self, Self::SealedProgram)
    }
}

/// Coverage deliberately excluded from this initial common tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonAotCoverageDecline {
    /// `TclOO` method dispatch requires its own guarded method identity.
    TclOoMethods,
    /// `apply` and namespace-eval body units do not denote direct proc commands.
    SyntheticBodyUnits,
}

impl CommonAotCoverageDecline {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TclOoMethods => "tcl-oo-methods",
            Self::SyntheticBodyUnits => "synthetic-body-units",
        }
    }
}

/// Target-neutral AOT proof evidence keyed by stable compiler IR identities.
#[derive(Debug, Clone, PartialEq)]
pub struct CommonAotProofPlan {
    environment: CommonAotEnvironment,
    direct_calls: BTreeMap<DirectCallSiteId, DirectProcDecision>,
    declined_direct_callees: BTreeSet<crate::ir::ProcedureId>,
    semantic_calls: BTreeMap<DirectCallSiteId, SemanticCallDecision>,
    closed_program_coverage: ClosedProgramCoverageDecision,
    materialisable_slots: BTreeMap<SsaValueIdentity, MaterialisableSlotDecision>,
    declared_arguments: BTreeMap<DeclaredArgumentIdentity, DeclaredArgumentDecision>,
    coverage_declines: Vec<CommonAotCoverageDecline>,
}

#[derive(Clone, Copy)]
enum AotMetadataSelection {
    Standalone(Option<SemanticContext>),
    Retained,
}

impl AotMetadataSelection {
    fn for_function<'a>(
        self,
        function: &'a FunctionUnit,
        registry: &tcl_registry::CommandRegistry,
        module: &crate::ir::Module,
    ) -> Option<InvocationMetadataContext<'a>> {
        match self {
            Self::Standalone(context) => context.map(InvocationMetadataContext::from),
            Self::Retained => function.invocation_metadata_context_for_module(registry, module),
        }
    }

    const fn standalone_context(self) -> Option<SemanticContext> {
        match self {
            Self::Standalone(context) => context,
            Self::Retained => None,
        }
    }
}

impl CommonAotProofPlan {
    /// Build proof evidence from existing common analyses.
    #[must_use]
    pub fn build(
        unit: &CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
        context: Option<SemanticContext>,
        config: SemanticOptimisationConfig,
        environment: CommonAotEnvironment,
    ) -> Self {
        Self::build_with_selection(
            unit,
            registry,
            AotMetadataSelection::Standalone(context),
            config,
            environment,
        )
    }

    /// Build from each function's actual retained availability and grammar.
    /// Missing, foreign or stale owners cannot supply metadata. Binding,
    /// dispatch, frame and Native execution obligations remain independent.
    #[must_use]
    pub fn build_with_retained_metadata(
        unit: &CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
        config: SemanticOptimisationConfig,
        environment: CommonAotEnvironment,
    ) -> Self {
        Self::build_with_selection(
            unit,
            registry,
            AotMetadataSelection::Retained,
            config,
            environment,
        )
    }

    fn build_with_selection(
        unit: &CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
        selection: AotMetadataSelection,
        config: SemanticOptimisationConfig,
        environment: CommonAotEnvironment,
    ) -> Self {
        // The registry this unit was lowered under, not `tcl8.6`: the three
        // calls below already take it, and an escape analysis answering from
        // a different dialect can under-report escaping variables (#2167).
        let escape = analyse_var_escape_cu_with_registry(unit, true, registry);
        let mutations = &unit.command_mutations;
        let direct = collect_direct_calls(
            unit,
            registry,
            selection,
            config,
            &escape,
            mutations,
            &unit.caller_scope.proc_binding_trust,
        );
        let direct_calls = direct.decisions;
        let declined_direct_callees = direct.declined_callees;
        let propagated = direct.propagated;

        let semantic_calls =
            collect_semantic_calls(unit, registry, selection, config, mutations, &direct_calls);

        let closed_program_coverage = prove_closed_program_coverage(
            unit,
            registry,
            environment,
            mutations,
            &direct_calls,
            &semantic_calls,
        );

        let materialisable_slots = collect_materialisable_slots(
            unit,
            &escape,
            &propagated,
            config,
            environment,
            &closed_program_coverage,
        );

        let declared_arguments =
            declared_arguments::collect(unit, registry, selection, &escape, &propagated, config);

        let mut coverage_declines = Vec::new();
        if !unit.methods.is_empty() {
            coverage_declines.push(CommonAotCoverageDecline::TclOoMethods);
        }
        if !unit.body_units.is_empty() {
            coverage_declines.push(CommonAotCoverageDecline::SyntheticBodyUnits);
        }
        Self {
            environment,
            direct_calls,
            declined_direct_callees,
            semantic_calls,
            closed_program_coverage,
            materialisable_slots,
            declared_arguments,
            coverage_declines,
        }
    }

    /// Compilation-environment premise under which this proof was built.
    #[must_use]
    pub const fn environment(&self) -> CommonAotEnvironment {
        self.environment
    }

    /// Direct-call decisions in stable IR order.
    pub fn direct_calls(&self) -> impl Iterator<Item = (&DirectCallSiteId, &DirectProcDecision)> {
        self.direct_calls.iter()
    }

    /// Whether any direct-call candidate naming `callee` was declined.
    ///
    /// A selected call site proves nothing about the *other* ways a procedure
    /// can be reached: a same-arity call through an alias, a rebound name, or
    /// a candidate declined for exceptional control flow still invokes it with
    /// values the selected sites never carry.  A consumer that joins facts
    /// across selected sites — caller operand ranges, propagated actual types —
    /// must therefore treat a decline naming the same callee as proof that the
    /// selected sites do not exhaust its callers.
    #[must_use]
    pub fn has_declined_direct_call_to(&self, callee: crate::ir::ProcedureId) -> bool {
        self.declined_direct_callees.contains(&callee)
    }

    /// Registry semantic-call decisions in stable CFG identity order.
    pub fn semantic_calls(
        &self,
    ) -> impl Iterator<Item = (&DirectCallSiteId, &SemanticCallDecision)> {
        self.semantic_calls.iter()
    }

    /// Return the semantic-call decision for one exact outer call site.
    #[must_use]
    pub fn semantic_call(&self, site: &DirectCallSiteId) -> Option<&SemanticCallDecision> {
        self.semantic_calls.get(site)
    }

    /// Exact sealed top-level statement accounting, when proved.
    #[must_use]
    pub const fn closed_program_coverage(&self) -> &ClosedProgramCoverageDecision {
        &self.closed_program_coverage
    }

    /// Materialisable-slot decisions in stable SSA order.
    pub fn materialisable_slots(
        &self,
    ) -> impl Iterator<Item = (&SsaValueIdentity, &MaterialisableSlotDecision)> {
        self.materialisable_slots.iter()
    }

    /// Proposed declared argument storage, independent of physical SSA slots.
    pub fn declared_arguments(
        &self,
    ) -> impl Iterator<Item = (&DeclaredArgumentIdentity, &DeclaredArgumentDecision)> {
        self.declared_arguments.iter()
    }

    /// Join an exact incoming read to its canonical original declaration.
    /// A slot label alone or a different source occurrence cannot select this
    /// proposed argument, and no physical SSA symbol is created.
    #[must_use]
    pub fn declared_argument_for_incoming_read(
        &self,
        procedure: &ProcedureIdentity,
        read: &crate::ssa::SsaIncomingSlotRead,
    ) -> Option<(&DeclaredArgumentIdentity, &DeclaredArgumentEvidence)> {
        self.declared_arguments
            .iter()
            .find_map(|(identity, decision)| {
                if &identity.procedure != procedure {
                    return None;
                }
                let DeclaredArgumentDecision::Selected(evidence) = decision else {
                    return None;
                };
                (evidence.reads().contains(read)
                    && evidence
                        .declaration()
                        .arguments()
                        .ordinal(read.slot.as_bytes())
                        == Some(identity.ordinal))
                .then_some((identity, evidence.as_ref()))
            })
    }

    /// Deliberately excluded semantic surfaces.
    #[must_use]
    pub fn coverage_declines(&self) -> &[CommonAotCoverageDecline] {
        &self.coverage_declines
    }
}

struct CallCandidate<'a> {
    id: DirectCallSiteId,
    block: BlockId,
    statement_index: u32,
    command: String,
    args: Vec<String>,
    tokens: Option<std::borrow::Cow<'a, CommandTokens>>,
}

fn function_units(unit: &CompilationUnit) -> impl Iterator<Item = (&str, &FunctionUnit)> {
    std::iter::once((unit.top_level.name.as_str(), &unit.top_level)).chain(
        unit.procedures
            .values()
            .map(|unit| (unit.name.as_str(), unit)),
    )
}

fn call_sites<'a>(
    unit: &CompilationUnit,
    caller: &str,
    function: &'a FunctionUnit,
    lexer_config: tcl_lexer::LexerConfig,
) -> Vec<CallCandidate<'a>> {
    let mut out = Vec::new();
    for (block, cfg_block) in &function.cfg.blocks {
        let Some(ssa_block) = function.ssa.blocks.get(block) else {
            continue;
        };
        for (index, statement) in cfg_block.statements.iter().enumerate() {
            if ssa_block.statements.get(index).is_none() {
                continue;
            }
            let original = crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
                &function.cfg.command_binding_sites,
                statement,
            );
            let (command, args) = if let Statement::Call { command, args, .. } = statement {
                (command.clone(), args.clone())
            } else {
                let Some(tokens) = original.filter(|tokens| {
                    tokens.synthetic.is_none() && tokens.words_align_with_argv_text()
                }) else {
                    continue;
                };
                let Some((head, arguments)) = tokens.argv_texts.split_first() else {
                    continue;
                };
                (head.clone(), arguments.to_vec())
            };
            let statement_index = u32::try_from(index).unwrap_or(u32::MAX);
            out.push(CallCandidate {
                id: DirectCallSiteId {
                    module_id: unit.ir_module.procedure_ids.module_id(),
                    body_id: function.body_id,
                    function: caller.to_owned(),
                    block: *block,
                    statement_index,
                    nested_argument: None,
                },
                block: *block,
                statement_index,
                command,
                args: args.clone(),
                tokens: original.map(std::borrow::Cow::Borrowed),
            });
            for argument_index in 0..args.len() {
                let Some(tokens) = nested_call_tokens(original, argument_index, lexer_config)
                else {
                    continue;
                };
                let Some((nested, nested_args)) = tokens.argv_texts.split_first() else {
                    continue;
                };
                out.push(CallCandidate {
                    id: DirectCallSiteId {
                        module_id: unit.ir_module.procedure_ids.module_id(),
                        body_id: function.body_id,
                        function: caller.to_owned(),
                        block: *block,
                        statement_index,
                        nested_argument: Some(u32::try_from(argument_index).unwrap_or(u32::MAX)),
                    },
                    block: *block,
                    statement_index,
                    command: nested.clone(),
                    args: nested_args.to_vec(),
                    tokens: Some(std::borrow::Cow::Owned(tokens)),
                });
            }
        }
    }
    out
}

fn nested_call_tokens(
    parent: Option<&CommandTokens>,
    argument: usize,
    config: tcl_lexer::LexerConfig,
) -> Option<CommandTokens> {
    let parent = parent?;
    let word = parent.words().get(argument.checked_add(1)?)?;
    let mut commands =
        crate::value_shapes::command_substitution_tokens(word, Some(parent), config)?;
    (commands.len() == 1).then(|| commands.remove(0))
}

#[cfg(test)]
mod tests;

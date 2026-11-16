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

//! Exact statement accounting of one sealed top-level program.

use super::direct_calls::{DirectActualValue, DirectProcDecision};
use super::semantic_calls::{
    SemanticCallArgument, SemanticCallDecision, semantic_operation_binding_is_trusted,
    semantic_operation_has_execution_trace,
};
use super::{CommonAotEnvironment, DirectCallSiteId, ProcedureIdentity, SsaValueIdentity};
use crate::analyses::LatticeValue;
use crate::cfg::{Block, BlockId};
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::Statement;
use crate::ssa::{SsaBlock, SsaStatement};
use std::collections::{BTreeMap, HashSet};
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::{SemanticOperationId, Traits};

/// Stable identity of one top-level CFG statement.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CfgStatementId {
    /// Qualified function name.
    pub function: String,
    /// CFG block containing the statement.
    pub block: BlockId,
    /// Statement ordinal within the block.
    pub statement_index: u32,
}

/// How one statement is accounted for by an exact sealed-program proof.
#[derive(Debug, Clone, PartialEq)]
pub enum ClosedProgramStatementEvidence {
    /// A procedure definition whose exact definition is the target of a
    /// selected direct call.
    DirectProcedureDefinition {
        /// Statement identity.
        statement: CfgStatementId,
        /// Selected procedure definition.
        procedure: ProcedureIdentity,
    },
    /// A constant definition whose SSA value is an exact selected direct-call
    /// actual.
    DirectActualConstant {
        /// Statement identity.
        statement: CfgStatementId,
        /// Exact caller SSA value.
        value: SsaValueIdentity,
        /// Exact SCCP value.
        constant: LatticeValue,
        /// Registry operation whose binding and trace proof authorises
        /// eliding the source assignment command.
        operation: SemanticOperationId,
    },
    /// A registry-resolved frameless boundary whose arguments are selected
    /// nested direct calls.
    SemanticBoundary {
        /// Exact outer call site.
        call: DirectCallSiteId,
        /// Registry operation executed at the boundary.
        operation: SemanticOperationId,
    },
}

/// Exact accounting of every statement in one closed top-level program.
#[derive(Debug, Clone, PartialEq)]
pub struct ClosedProgramCoverageEvidence {
    /// Sole reachable top-level block.
    pub block: BlockId,
    /// Every top-level statement, in source/CFG order.
    pub statements: Vec<ClosedProgramStatementEvidence>,
}

/// Why exact sealed-program statement accounting was unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosedProgramCoverageDecline {
    /// Hosted compilation preserves externally observable interpreter state.
    HostedEnvironment,
    /// The top-level CFG is not one closed linear block with normal completion.
    NonLinearControlFlow,
    /// At least one statement lacks one of the exact common proof categories.
    UncoveredStatement,
}

impl ClosedProgramCoverageDecline {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HostedEnvironment => "hosted-environment",
            Self::NonLinearControlFlow => "non-linear-control-flow",
            Self::UncoveredStatement => "uncovered-statement",
        }
    }
}

/// Exact coverage evidence or a conservative typed decline.
#[derive(Debug, Clone, PartialEq)]
pub enum ClosedProgramCoverageDecision {
    /// Every top-level statement is explicitly accounted for.
    Selected(ClosedProgramCoverageEvidence),
    /// A backend must retain general top-level execution.
    Declined(ClosedProgramCoverageDecline),
}

fn closed_program_linear_entry(function: &FunctionUnit) -> Option<(&Block, &SsaBlock)> {
    if !function.cfg.exception_edges.is_empty() {
        return None;
    }
    let block = function.cfg.blocks.get(&function.cfg.entry)?;
    let normal_completion = |terminator: &Option<crate::cfg::Terminator>| {
        matches!(
            terminator,
            None | Some(crate::cfg::Terminator::Return {
                value: None,
                expr: None,
                ..
            })
        )
    };
    let linear_shape = match &block.terminator {
        terminator if normal_completion(terminator) => function.cfg.blocks.len() == 1,
        Some(crate::cfg::Terminator::Goto { target, .. }) => {
            function.cfg.blocks.len() == 2
                && *target != function.cfg.entry
                && function.cfg.blocks.get(target).is_some_and(|exit| {
                    exit.statements.is_empty() && normal_completion(&exit.terminator)
                })
                && function
                    .ssa
                    .blocks
                    .get(target)
                    .is_some_and(|exit| exit.phis.is_empty() && exit.statements.is_empty())
        }
        _ => false,
    };
    linear_shape
        .then(|| function.ssa.blocks.get(&function.cfg.entry))
        .flatten()
        .filter(|ssa| ssa.statements.len() == block.statements.len())
        .map(|ssa| (block, ssa))
}

struct ClosedStatementInputs<'a> {
    pub(super) unit: &'a CompilationUnit,
    pub(super) registry: &'a tcl_registry::CommandRegistry,
    pub(super) mutations: &'a crate::command_binding::ModuleCommandMutations,
    pub(super) direct_calls: &'a BTreeMap<DirectCallSiteId, DirectProcDecision>,
    pub(super) semantic_calls: &'a BTreeMap<DirectCallSiteId, SemanticCallDecision>,
    pub(super) function: &'a FunctionUnit,
    direct_actuals: &'a HashSet<SsaValueIdentity>,
    set_operation: SemanticOperationId,
}

fn closed_constant_store(
    input: &ClosedStatementInputs<'_>,
    index: usize,
    statement: &Statement,
) -> bool {
    if matches!(statement, Statement::AssignConst { .. }) {
        return true;
    }
    let Some(tokens) = crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
        &input.function.cfg.command_binding_sites,
        statement,
    ) else {
        return false;
    };
    let site = DirectCallSiteId {
        module_id: input.unit.ir_module.procedure_ids.module_id(),
        body_id: input.function.body_id,
        function: input.function.name.clone(),
        block: input.function.cfg.entry,
        statement_index: u32::try_from(index).unwrap_or(u32::MAX),
        nested_argument: None,
    };
    if !matches!(
        input.semantic_calls.get(&site),
        Some(SemanticCallDecision::Selected(semantic)) if semantic.operation == input.set_operation
    ) {
        return false;
    }
    let Some(binding) = tokens.source_binding.as_ref() else {
        return false;
    };
    crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
        input.registry,
        input
            .function
            .invocation_metadata_context_for_module(input.registry, &input.unit.ir_module),
        tokens,
    )
    .and_then(|normal| normal.stored_value_literal(&binding.variable_context, input.registry))
    .is_some()
}

fn closed_native_store_binding(input: &ClosedStatementInputs<'_>, statement: &Statement) -> bool {
    let Statement::AssignConst { value, .. } = statement else {
        return false;
    };
    let Some(tokens) = crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
        &input.function.cfg.command_binding_sites,
        statement,
    ) else {
        return false;
    };
    let Some(binding) = tokens.source_binding.as_ref() else {
        return false;
    };
    binding
        .evaluated_command_word()
        .is_some_and(|head| binding.direct_registry_target(head).is_some())
        && crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
            input.registry,
            input
                .function
                .invocation_metadata_context_for_module(input.registry, &input.unit.ir_module),
            tokens,
        )
        .and_then(|normal| normal.stored_value_literal(&binding.variable_context, input.registry))
        .as_deref()
            == Some(value.as_str())
}

fn cover_closed_constant_store(
    input: &ClosedStatementInputs<'_>,
    statement: &Statement,
    ssa: &SsaStatement,
    statement_id: CfgStatementId,
) -> Option<ClosedProgramStatementEvidence> {
    if input.unit.ir_module.has_dynamic_trace
        || !semantic_operation_binding_is_trusted(
            input.registry,
            input.mutations,
            input.set_operation,
        )
        || semantic_operation_has_execution_trace(
            input.registry,
            &input.unit.ir_module,
            input.set_operation,
        )
        || (matches!(statement, Statement::AssignConst { .. })
            && !closed_native_store_binding(input, statement))
    {
        return None;
    }
    let (symbol, version) = ssa.defs.iter().find_map(|(symbol, version)| {
        let value = SsaValueIdentity {
            module_id: input.unit.ir_module.procedure_ids.module_id(),
            body_id: input.function.body_id,
            function: input.function.name.clone(),
            symbol: *symbol,
            version: *version,
        };
        input
            .direct_actuals
            .contains(&value)
            .then_some((*symbol, *version))
    })?;
    let constant = input.function.sccp.values.get(&(symbol, version))?.clone();
    Some(ClosedProgramStatementEvidence::DirectActualConstant {
        statement: statement_id,
        value: SsaValueIdentity {
            module_id: input.unit.ir_module.procedure_ids.module_id(),
            body_id: input.function.body_id,
            function: input.function.name.clone(),
            symbol,
            version,
        },
        constant,
        operation: input.set_operation,
    })
}

fn cover_closed_statement(
    input: &ClosedStatementInputs<'_>,
    index: usize,
    statement: &Statement,
    ssa: &SsaStatement,
) -> Option<ClosedProgramStatementEvidence> {
    let statement_index = u32::try_from(index).unwrap_or(u32::MAX);
    let statement_id = CfgStatementId {
        function: input.function.name.clone(),
        block: input.function.cfg.entry,
        statement_index,
    };
    if closed_constant_store(input, index, statement) {
        return cover_closed_constant_store(input, statement, ssa, statement_id);
    }
    let Statement::Call { .. } = statement else {
        return None;
    };
    let site = DirectCallSiteId {
        module_id: input.unit.ir_module.procedure_ids.module_id(),
        body_id: input.function.body_id,
        function: input.function.name.clone(),
        block: input.function.cfg.entry,
        statement_index,
        nested_argument: None,
    };
    let SemanticCallDecision::Selected(semantic) = input.semantic_calls.get(&site)? else {
        return None;
    };
    if semantic.operation == SemanticOperationId::StructuredLowering(LoweringHookId::Proc) {
        let procedure = input
            .direct_calls
            .values()
            .find_map(|decision| match decision {
                DirectProcDecision::Selected(evidence)
                    if evidence.callee.definition_start == statement.span().start()
                        && evidence.callee.definition_end == statement.span().end() =>
                {
                    Some(evidence.callee.clone())
                }
                _ => None,
            })?;
        return Some(ClosedProgramStatementEvidence::DirectProcedureDefinition {
            statement: statement_id,
            procedure,
        });
    }
    if !semantic.traits.contains(Traits::FRAMELESS_RUNTIME)
        || semantic.arguments.is_empty()
        || !semantic.arguments.iter().all(|argument| match argument {
            SemanticCallArgument::NestedDirect { call, .. } => matches!(
                input.direct_calls.get(call),
                Some(DirectProcDecision::Selected(_))
            ),
            SemanticCallArgument::Value { .. } => false,
        })
    {
        return None;
    }
    Some(ClosedProgramStatementEvidence::SemanticBoundary {
        call: site,
        operation: semantic.operation,
    })
}

pub(super) fn prove_closed_program_coverage(
    unit: &CompilationUnit,
    registry: &tcl_registry::CommandRegistry,
    environment: CommonAotEnvironment,
    mutations: &crate::command_binding::ModuleCommandMutations,
    direct_calls: &BTreeMap<DirectCallSiteId, DirectProcDecision>,
    semantic_calls: &BTreeMap<DirectCallSiteId, SemanticCallDecision>,
) -> ClosedProgramCoverageDecision {
    let decline = |reason| ClosedProgramCoverageDecision::Declined(reason);
    if environment != CommonAotEnvironment::SealedProgram {
        return decline(ClosedProgramCoverageDecline::HostedEnvironment);
    }
    let function = &unit.top_level;
    let Some((block, ssa_block)) = closed_program_linear_entry(function) else {
        return decline(ClosedProgramCoverageDecline::NonLinearControlFlow);
    };

    let direct_actuals: HashSet<SsaValueIdentity> = direct_calls
        .values()
        .filter_map(|decision| match decision {
            DirectProcDecision::Selected(evidence) => Some(&evidence.actual_values),
            DirectProcDecision::Declined(_) => None,
        })
        .flatten()
        .filter_map(|actual| match actual {
            DirectActualValue::Ssa(value) => Some(value.clone()),
            DirectActualValue::Unproven => None,
        })
        .collect();
    let set_operation = SemanticOperationId::StructuredLowering(LoweringHookId::Set);
    let input = ClosedStatementInputs {
        unit,
        registry,
        mutations,
        direct_calls,
        semantic_calls,
        function,
        direct_actuals: &direct_actuals,
        set_operation,
    };
    let mut statements = Vec::with_capacity(block.statements.len());
    for (index, (statement, ssa)) in block
        .statements
        .iter()
        .zip(&ssa_block.statements)
        .enumerate()
    {
        if !statement.is_executable_invocation() {
            continue;
        }
        let Some(evidence) = cover_closed_statement(&input, index, statement, ssa) else {
            return decline(ClosedProgramCoverageDecline::UncoveredStatement);
        };
        statements.push(evidence);
    }
    ClosedProgramCoverageDecision::Selected(ClosedProgramCoverageEvidence {
        block: function.cfg.entry,
        statements,
    })
}

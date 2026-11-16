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

//! Materialisable SSA slots: the evidence, recipe and decline types, slot allocation and the collector.

use super::closed_program_coverage::{
    ClosedProgramCoverageDecision, ClosedProgramStatementEvidence,
};
use super::{CommonAotEnvironment, SsaValueIdentity};
use crate::analyses::LatticeValue;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::intervals::{Interval, compute_intervals_with};
use crate::ir::Procedure;
use crate::representation_plan::{SharingState, VarStorage};
use crate::semantic_optimisation::{SemanticOptimisationConfig, SemanticOptimisationPassId};
use crate::ssa::ValueKey;
use crate::types::{TypeLattice, TypeShape, type_join};
use crate::var_escape::{EscapeTag, ProcEscapeSummary};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// One materialisable SSA slot authorised by common analysis.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialisableSlotEvidence {
    /// Existing local-slot allocation reused by both future backends.
    pub local_slot: u32,
    /// Variable display name retained for diagnostics.
    pub variable: String,
    /// Independent byte/frame or explicit authored allocation selecting the slot.
    pub authority: MaterialisableSlotAuthority,
    /// Exact singleton type shape used by materialisation.
    pub shape: TypeShape,
    /// Existing sparse-conditional-constant-propagation fact.
    pub constant: Option<LatticeValue>,
    /// Existing integer-range fact, when meaningful.
    pub interval: Option<Interval>,
    /// Authoritative storage selected by this common proof.
    pub storage: VarStorage,
    /// Exact recipe for reconstructing the runtime-visible Tcl value.
    pub recipe: MaterialisationRecipe,
    /// Live-interpreter epochs a later consumer must guard before using this
    /// slot without a continuously authoritative frame cell.
    pub runtime_guards: MaterialisationGuardRequirements,
}

/// Purpose of a selected allocation. Source rewrite ordinals never describe a
/// physical local cache, and an advisory label cannot construct a native receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterialisableSlotAuthority {
    /// Explicit logical analysis of a symbolic authored cell.
    Authored,
    /// Fixed argument ordinal for a newly compiled original declaration.
    SourceFormal {
        /// Original declaration, counted arguments and declaration compiler purpose.
        declaration: std::sync::Arc<
            crate::var_escape::original_slots::OriginalDeclaredProcedureArgumentSlots,
        >,
        /// Exact local activation represented by SSA.
        cell: crate::var_resolve::VariableCellKey,
    },
    /// Exact source cell accounted for by the sealed-program statement proof.
    SealedProgramCell {
        /// Original namespace and root storage key, without a label conversion.
        cell: crate::var_resolve::VariableCellKey,
    },
    /// Actual retained native frame layout and its required incarnation.
    NativeFrame(Box<crate::var_escape::original_slots::OriginalNativeFrameSlot>),
}

/// Runtime observability domains not discharged by source-only analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialisationGuardRequirements {
    /// Guard the interpreter's variable-trace generation.
    pub variable_trace_epoch: bool,
    /// Guard safe/child-interpreter policy and host callbacks that can expose
    /// frames independently of source-visible commands.
    pub interpreter_policy_epoch: bool,
}

/// How a materialisable slot recreates its runtime-visible Tcl value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialisationRecipe {
    /// Retain the original Tcl object. Publishing that same object preserves
    /// non-canonical string reps such as `02`, `+1`, and `0x10`; ordinary
    /// shared-object copy-on-write rules apply to later mutation.
    RetainOriginalTclObject {
        /// Ownership state the eventual lowering must preserve.
        sharing: SharingState,
    },
}

/// Why an SSA value cannot use a materialisable slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterialisableSlotDecline {
    /// This slot's analysis world retains implicit math prerequisites which
    /// the materialisation plan cannot validate before native execution.
    MathBindingPrerequisiteRequired,
    /// The pass is disabled by default and was not explicitly enabled.
    PassDisabled,
    /// Escape analysis did not allocate a local slot for this name.
    NoLocalSlot,
    /// A typed original cell lacks its own byte/frame allocation receipt.
    OriginalSlotUnavailable,
    /// This SSA version may be observed through a frame alias.
    EscapesToFrame,
    /// A literal or dynamic variable trace may observe representation changes.
    VariableTrace,
    /// Type tracking did not prove one exact representation shape.
    TypeNotSingleton,
    /// Exceptional CFG edges are not yet covered by slot materialisation.
    ExceptionalControlFlow,
    /// A hosted top-level variable remains observable after `::top` returns.
    HostedTopLevelObservable,
}

impl MaterialisableSlotDecline {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::MathBindingPrerequisiteRequired => "math-binding-prerequisite-required",
            Self::PassDisabled => "pass-disabled",
            Self::NoLocalSlot => "no-local-slot",
            Self::OriginalSlotUnavailable => "original-slot-unavailable",
            Self::EscapesToFrame => "escapes-to-frame",
            Self::VariableTrace => "variable-trace",
            Self::TypeNotSingleton => "type-not-singleton",
            Self::ExceptionalControlFlow => "exceptional-control-flow",
            Self::HostedTopLevelObservable => "hosted-top-level-observable",
        }
    }
}

/// Selection or typed decline for one SSA slot candidate.
#[derive(Debug, Clone, PartialEq)]
pub enum MaterialisableSlotDecision {
    /// Common analysis proved an exact materialisation recipe.
    Selected(Box<MaterialisableSlotEvidence>),
    /// The value stays boxed/frame-resident.
    Declined(MaterialisableSlotDecline),
}

fn ssa_value_keys(function: &FunctionUnit, proc_def: Option<&Procedure>) -> BTreeSet<ValueKey> {
    let mut keys = BTreeSet::new();
    for block in function.ssa.blocks.values() {
        for phi in &block.phis {
            keys.insert((phi.name, phi.version));
        }
        for statement in &block.statements {
            keys.extend(
                statement
                    .defs
                    .iter()
                    .map(|(symbol, version)| (*symbol, *version)),
            );
            keys.extend(
                statement
                    .uses
                    .iter()
                    .map(|(symbol, version)| (*symbol, *version)),
            );
        }
    }
    if let Some(proc_def) = proc_def {
        for parameter in &proc_def.params {
            if let Some(symbol) = function.ssa.var_symbol(parameter) {
                keys.insert((symbol, 0));
            }
        }
    }
    keys
}

pub(super) fn collect_materialisable_slots(
    unit: &CompilationUnit,
    escape: &HashMap<tcl_core_types::BodyFrameId, ProcEscapeSummary>,
    propagated: &HashMap<(crate::ir::ProcedureId, usize), Option<TypeLattice>>,
    config: SemanticOptimisationConfig,
    environment: CommonAotEnvironment,
    closed_program_coverage: &ClosedProgramCoverageDecision,
) -> BTreeMap<SsaValueIdentity, MaterialisableSlotDecision> {
    let mut slots = BTreeMap::new();
    for (qname, function) in std::iter::once((&unit.top_level.name, &unit.top_level)).chain(
        unit.procedures
            .values()
            .map(|function| (&function.name, function)),
    ) {
        let frame = if std::ptr::eq(function, std::ptr::from_ref(&unit.top_level)) {
            Some(tcl_core_types::BodyFrameId::Top(
                unit.ir_module.procedure_ids.module_id(),
            ))
        } else {
            function.body_id.map(tcl_core_types::BodyFrameId::Body)
        };
        let summary = frame.and_then(|frame| escape.get(&frame));
        let sealed_cells = if std::ptr::eq(function, std::ptr::from_ref(&unit.top_level))
            && environment == CommonAotEnvironment::SealedProgram
        {
            (|| {
                let ClosedProgramCoverageDecision::Selected(coverage) = closed_program_coverage
                else {
                    return None;
                };
                let mut cells = Vec::new();
                for statement in &coverage.statements {
                    if let ClosedProgramStatementEvidence::DirectActualConstant { value, .. } =
                        statement
                    {
                        let cell = function.ssa.cell_key(value.symbol)?;
                        if !cells.contains(cell) {
                            cells.push(cell.clone());
                        }
                    }
                }
                cells.truncate(crate::var_escape::LOCALS_ARRAY_CAP);
                Some(cells)
            })()
        } else {
            None
        };
        let local_slots = summary.map_or_default(|summary| summary.local_slots.clone());
        // The lowered module's own numeral grammar (`Module::dialect`) — the
        // same source `codegen_module` reads, so a slot decision made here and
        // the code emitted for it agree on what `0755` is.
        let intervals = compute_intervals_with(
            &function.cfg,
            &function.ssa,
            &function.sccp.values,
            unit.ir_module.number_syntax(),
        );
        for key in ssa_value_keys(
            function,
            function
                .body_id
                .and_then(|id| unit.ir_module.procedures.get(&id)),
        ) {
            let identity = SsaValueIdentity {
                module_id: unit.ir_module.procedure_ids.module_id(),
                body_id: function.body_id,
                function: qname.clone(),
                symbol: key.0,
                version: key.1,
            };
            let decision = materialisable_decision(&MaterialisableInputs {
                unit,
                function,
                key,
                summary,
                local_slots: &local_slots,
                sealed_cells: sealed_cells.as_deref(),
                propagated,
                intervals: &intervals,
                enabled: config.is_enabled(SemanticOptimisationPassId::MaterialisableSlot),
                environment,
            });
            slots.insert(identity, decision);
        }
    }
    slots
}

struct MaterialisableInputs<'a> {
    pub(super) unit: &'a CompilationUnit,
    pub(super) function: &'a FunctionUnit,
    key: ValueKey,
    pub(super) summary: Option<&'a ProcEscapeSummary>,
    local_slots: &'a BTreeMap<String, u32>,
    sealed_cells: Option<&'a [crate::var_resolve::VariableCellKey]>,
    pub(super) propagated: &'a HashMap<(crate::ir::ProcedureId, usize), Option<TypeLattice>>,
    intervals: &'a HashMap<ValueKey, Interval>,
    pub(super) enabled: bool,
    pub(super) environment: CommonAotEnvironment,
}

fn native_frame_slot_allocation(
    input: &MaterialisableInputs<'_>,
    cell: &crate::var_resolve::VariableCellKey,
) -> Option<(u32, MaterialisableSlotAuthority)> {
    use crate::var_escape::original_slots::OriginalNativeFrameSlot;
    // The entry layout belongs only to the root's actual active frame.
    // It cannot be lent to a separately declared procedure.
    if !std::ptr::eq(input.function, std::ptr::from_ref(&input.unit.top_level)) {
        return None;
    }
    let receipt = OriginalNativeFrameSlot::from_entry(&input.unit.ir_module, cell)?;
    let points = input.function.ssa.point_contexts.as_ref()?;
    let mut represented = false;
    for (&block, statements) in &input.function.ssa.blocks {
        for (index, statement) in statements.statements.iter().enumerate() {
            if statement.uses.contains_key(&input.key.0)
                || statement.defs.contains_key(&input.key.0)
            {
                let context = points.context_before(block, index)?;
                // A compiled name alone establishes neither link absence nor
                // quiet activation contents. Those domains remain separate.
                if !receipt.matches_current_activation(context)
                    || !context.activation_observers_closed()
                    || context.dynamic_bindings
                    || context.dynamic_traces
                    || !context.alias_bindings.is_empty()
                    || !context.name_alias_bindings.is_empty()
                {
                    return None;
                }
                represented = true;
            }
        }
    }
    represented.then(|| {
        (
            receipt.ordinal(),
            MaterialisableSlotAuthority::NativeFrame(Box::new(receipt)),
        )
    })
}

fn materialisable_slot_allocation(
    input: &MaterialisableInputs<'_>,
) -> Option<(u32, MaterialisableSlotAuthority)> {
    use crate::var_escape::original_slots::OriginalDeclaredProcedureArgumentSlots;
    use crate::var_resolve::VariableCellKey;
    let cell = input.function.ssa.cell_key(input.key.0)?;
    if std::ptr::eq(input.function, std::ptr::from_ref(&input.unit.top_level))
        && input.unit.ir_module.source_entry.native_entry.is_some()
    {
        return native_frame_slot_allocation(input, cell);
    }
    if std::ptr::eq(input.function, std::ptr::from_ref(&input.unit.top_level))
        && input.environment == CommonAotEnvironment::SealedProgram
    {
        if !matches!(
            cell,
            VariableCellKey::Authored(_) | VariableCellKey::Namespace { .. }
        ) {
            return None;
        }
        let ordinal = u32::try_from(
            input
                .sealed_cells?
                .iter()
                .position(|candidate| candidate == cell)?,
        )
        .ok()?;
        return Some((
            ordinal,
            MaterialisableSlotAuthority::SealedProgramCell { cell: cell.clone() },
        ));
    }
    match cell {
        VariableCellKey::Authored(_) => {
            let ordinal = *input
                .local_slots
                .get(input.function.ssa.var_name(input.key.0))?;
            Some((ordinal, MaterialisableSlotAuthority::Authored))
        }
        VariableCellKey::Activation { identity, simple } => {
            let procedure = input
                .unit
                .ir_module
                .procedures
                .get(&input.function.body_id?)?;
            let declaration = OriginalDeclaredProcedureArgumentSlots::from_module(
                &input.unit.ir_module,
                procedure,
            )?;
            let topology = declaration.topology();
            let ordinal = declaration.arguments().ordinal(simple.as_bytes())?;
            let points = input.function.ssa.point_contexts.as_ref()?;
            let mut represented = false;
            for (&block, statements) in &input.function.ssa.blocks {
                for (index, statement) in statements.statements.iter().enumerate() {
                    if statement.uses.contains_key(&input.key.0)
                        || statement.defs.contains_key(&input.key.0)
                    {
                        let context = points.context_before(block, index)?;
                        if context.activation.as_ref() != Some(identity)
                            || context.original_formal_topology.as_deref() != Some(topology)
                            || context.dynamic_bindings
                            || !context.activation_observers_closed()
                        {
                            return None;
                        }
                        represented = true;
                    }
                }
            }
            represented.then(|| {
                (
                    ordinal,
                    MaterialisableSlotAuthority::SourceFormal {
                        declaration: std::sync::Arc::new(declaration),
                        cell: cell.clone(),
                    },
                )
            })
        }
        _ => None,
    }
}

fn materialisable_decision(input: &MaterialisableInputs<'_>) -> MaterialisableSlotDecision {
    let decline = |reason| MaterialisableSlotDecision::Declined(reason);
    if !input.enabled {
        return decline(MaterialisableSlotDecline::PassDisabled);
    }
    if input.function.requires_native_math_binding_validation() {
        return decline(MaterialisableSlotDecline::MathBindingPrerequisiteRequired);
    }
    if std::ptr::eq(input.function, std::ptr::from_ref(&input.unit.top_level))
        && input.environment == CommonAotEnvironment::Hosted
    {
        return decline(MaterialisableSlotDecline::HostedTopLevelObservable);
    }
    if !input.function.cfg.exception_edges.is_empty() {
        return decline(MaterialisableSlotDecline::ExceptionalControlFlow);
    }
    let variable = input.function.ssa.var_name(input.key.0).to_owned();
    let Some(summary) = input.summary else {
        return decline(MaterialisableSlotDecline::NoLocalSlot);
    };
    if summary.tags.get(&variable) == Some(&EscapeTag::Frame) {
        return decline(MaterialisableSlotDecline::EscapesToFrame);
    }
    let Some((local_slot, authority)) = materialisable_slot_allocation(input) else {
        return decline(MaterialisableSlotDecline::OriginalSlotUnavailable);
    };
    if input.unit.ir_module.has_dynamic_variable_trace
        || input.unit.ir_module.traced_variables.contains(&variable)
    {
        return decline(MaterialisableSlotDecline::VariableTrace);
    }
    let mut ty = input
        .function
        .types
        .get(&input.key)
        .cloned()
        .unwrap_or_else(TypeLattice::unknown);
    if input.key.1 == 0
        && let Some(body_id) = input.function.body_id
        && let Some(proc_def) = input.unit.ir_module.procedures.get(&body_id)
        && let Some(index) = match &authority {
            MaterialisableSlotAuthority::SourceFormal { .. } => usize::try_from(local_slot).ok(),
            MaterialisableSlotAuthority::Authored => {
                proc_def.params.iter().position(|name| name == &variable)
            }
            MaterialisableSlotAuthority::SealedProgramCell { .. }
            | MaterialisableSlotAuthority::NativeFrame(_) => None,
        }
        && let Some(propagated) = input.propagated.get(&(body_id, index))
    {
        let Some(propagated) = propagated else {
            // A missing actual type on one reached caller is a retained poison
            // fact. Arithmetic use inside the body cannot establish the entry
            // object's representation for that caller.
            return decline(MaterialisableSlotDecline::TypeNotSingleton);
        };
        ty = type_join(&ty, propagated);
    }
    let Some(shape) = ty.single_shape().cloned() else {
        return decline(MaterialisableSlotDecline::TypeNotSingleton);
    };
    MaterialisableSlotDecision::Selected(Box::new(MaterialisableSlotEvidence {
        local_slot,
        variable,
        authority,
        shape,
        constant: input.function.sccp.values.get(&input.key).cloned(),
        interval: input
            .intervals
            .get(&input.key)
            .copied()
            .filter(|interval| !interval.is_bottom()),
        storage: VarStorage::MaterializableSlot,
        recipe: MaterialisationRecipe::RetainOriginalTclObject {
            sharing: SharingState::Shared,
        },
        runtime_guards: MaterialisationGuardRequirements {
            variable_trace_epoch: true,
            interpreter_policy_epoch: true,
        },
    }))
}

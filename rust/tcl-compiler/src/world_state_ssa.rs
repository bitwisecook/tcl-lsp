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

//! CFG-aware renaming of registry-owned mutable interpreter state.
//!
//! This is the world counterpart to cell SSA.  It consumes the target-neutral
//! executable IR, registry [`tcl_registry::InvocationFacts`], and completion
//! switches.  It deliberately has no command-name recognisers: a command,
//! namespace, trace, interpreter, or object transition arrives here only as a
//! registry [`tcl_registry::StateTransition`] fact.
//!
//! The builder is conservative in two important ways.  First, an invocation
//! with no closed registry transition declaration is a `Use(Any)` followed by
//! `Clobber(Any)`.  Secondly, a transition that may commit before an abrupt
//! completion is represented by a clobber-version on every abrupt edge.  That
//! version is the join of unchanged and changed state; the generic
//! [`crate::state_ssa::StateSsa`] substrate has one incoming value per CFG
//! predecessor, rather than a second hidden state representation.

use std::collections::{BTreeMap, BTreeSet};

use tcl_core_types::Code as CompletionCode;
use tcl_registry::StateTransitionCommit;
#[cfg(test)]
use tcl_registry::{
    InterpreterTransition, NamespaceTransition, NamespaceTransitionTarget, ObjectDispatchTarget,
    ObjectDispatchTransition, ObjectPrivateNamespace, StateTransition, StateTransitionDomain,
    StateTransitionFact, TransitionSubject,
};

use crate::cfg::BlockId;
use crate::effect_ssa::{WorldStateIntentKind, project_effect_footprint};
use crate::executable_ir::{
    ExecutableBlockId, ExecutableFunction, ExecutableInstruction, ExecutableIrValidationError,
    ExecutableTerminator,
};
#[cfg(test)]
use crate::state_ssa::StateOp;
use crate::state_ssa::adapters::WorldRegion;
#[cfg(test)]
use crate::state_ssa::adapters::{
    WorldInterpreterScope, WorldNamespaceScope, WorldRegionKind, WorldSubjectScope,
};
use crate::state_ssa::{CfgStatePosition, StateSite, StateSsa, StateSsaError};

mod instruction_intents;
mod renaming;
mod transition_intents;

#[cfg(test)]
use instruction_intents::word_evaluation_runs_commands;
use instruction_intents::{
    cell_intents, conservative_region_intents, executable_expr_intents_with_source,
    lowered_footprint_intents, original_word_evaluation_runs_commands,
};
#[cfg(test)]
use renaming::Symbol;
use transition_intents::project_invocation_intents;
pub use transition_intents::project_transition_facts;

/// One version-free transition operation, in descriptor order.
///
/// It intentionally keeps the registry's completion commit policy.  The CFG
/// builder decides which outgoing completion edges receive the operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionStateIntent {
    /// The transition's state operation.
    pub kind: WorldStateIntentKind,
    /// The mutable interpreter-world region affected by the transition.
    pub location: WorldRegion,
    /// Completion-edge commit semantics declared by the registry.
    pub commit: StateTransitionCommit,
}

struct InstructionPlanInputs {
    block: ExecutableBlockId,
    completion: crate::executable_ir::CompletionId,
    ordinary: Vec<(WorldStateIntentKind, WorldRegion)>,
    transition: Vec<TransitionStateIntent>,
}

/// Why a world-state build cannot safely interpret an executable CFG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldStateSsaDecline {
    /// The executable semantic CFG failed its own target-neutral validation.
    InvalidExecutableIr(ExecutableIrValidationError),
    /// A block index cannot be represented by the shared CFG [`BlockId`].
    BlockIdOverflow {
        /// Executable block index that did not fit the shared identifier.
        block: usize,
    },
    /// A CFG instruction or its ordered state operations cannot fit the
    /// shared `u32` state-site coordinate space.
    StateSiteOverflow {
        /// Containing executable block.
        block: ExecutableBlockId,
        /// Instruction position in that block.
        instruction: usize,
    },
    /// A transition-bearing invocation must end in a completion dispatch.
    MissingCompletionSwitch {
        /// Transition-bearing block without a completion switch.
        block: ExecutableBlockId,
    },
    /// The completion switch did not dispatch the invocation completion.
    CompletionSwitchMismatch {
        /// Block whose switch dispatches a different completion.
        block: ExecutableBlockId,
    },
    /// The completion switch does not explicitly retain the `TCL_OK` edge.
    MissingOkCompletionEdge {
        /// Block whose completion switch has no explicit `TCL_OK` arm.
        block: ExecutableBlockId,
    },
    /// A validated CFG edge was absent from the planner's deterministic
    /// successor table.
    MissingCfgEdge {
        /// Block selecting the missing edge.
        predecessor: usize,
        /// Target block absent from the predecessor's successor table.
        successor: usize,
    },
    /// A reachable non-entry block had no state predecessor during renaming.
    MissingStatePredecessor {
        /// Reachable block without a state predecessor.
        block: usize,
        /// State-location slot being renamed.
        slot: usize,
    },
    /// A state symbol which reaches a record was not assigned a version.
    MissingStateVersion,
    /// A state-SSA record could not be validated.
    InvalidStateSsa(StateSsaError),
}

/// A built world-state SSA result and the typed region-overlap policy used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableWorldStateSsa {
    /// Validated common state-SSA records.
    pub state: StateSsa<WorldRegion>,
    /// Locations tracked by the renamer, in deterministic discovery order.
    pub locations: Vec<WorldRegion>,
}

/// Build world-state SSA for one executable function.
///
/// The result is deterministic for an isomorphic CFG irrespective of
/// predecessor insertion order.  It computes the fixed point over reachable
/// blocks, including back edges, before materialising state phi records.
pub fn build_executable_world_state_ssa(
    function: &ExecutableFunction,
) -> Result<ExecutableWorldStateSsa, WorldStateSsaDecline> {
    function
        .validate()
        .map_err(WorldStateSsaDecline::InvalidExecutableIr)?;
    let mut planner = Planner::new(function)?;
    planner.plan()?;
    planner.rename()
}

#[derive(Debug, Clone)]
struct Token {
    kind: WorldStateIntentKind,
    location: usize,
    site: StateSite,
}

#[derive(Debug, Clone)]
struct TransitionToken {
    /// Edge-qualified exact or conservative join definition tokens.
    edge_tokens: BTreeMap<usize, usize>,
}

#[derive(Debug, Default, Clone)]
struct BlockPlan {
    base: Vec<usize>,
    transitions: Vec<TransitionToken>,
    transition_completion: Option<crate::executable_ir::CompletionId>,
}

struct Planner<'a> {
    function: &'a ExecutableFunction,
    locations: Vec<WorldRegion>,
    tokens: Vec<Token>,
    plans: Vec<BlockPlan>,
    successors: Vec<Vec<usize>>,
    edge_completion: Vec<Vec<EdgeCompletion>>,
    predecessors: Vec<Vec<usize>>,
    reachable: BTreeSet<usize>,
}

impl<'a> Planner<'a> {
    fn new(function: &'a ExecutableFunction) -> Result<Self, WorldStateSsaDecline> {
        for index in 0..function.blocks.len() {
            let _ = block_id(index)?;
        }
        let mut successors = vec![Vec::new(); function.blocks.len()];
        let mut predecessors = vec![Vec::new(); function.blocks.len()];
        let mut edge_completions = vec![Vec::new(); function.blocks.len()];
        for (index, block) in function.blocks.iter().enumerate() {
            if let Some(terminator) = &block.terminator {
                successors[index] = successors_of(terminator);
                successors[index].sort_unstable();
                successors[index].dedup();
                edge_completions[index] = successors[index]
                    .iter()
                    .map(|successor| edge_completion(terminator, *successor))
                    .collect();
                for successor in &successors[index] {
                    if *successor < predecessors.len() {
                        predecessors[*successor].push(index);
                    }
                }
            }
        }
        for predecessors in &mut predecessors {
            predecessors.sort_unstable();
            predecessors.dedup();
        }
        let mut reachable = BTreeSet::new();
        let mut pending = vec![function.entry.index()];
        while let Some(block) = pending.pop() {
            if block >= successors.len() || !reachable.insert(block) {
                continue;
            }
            pending.extend(successors[block].iter().rev().copied());
        }
        Ok(Self {
            function,
            locations: Vec::new(),
            tokens: Vec::new(),
            plans: vec![BlockPlan::default(); function.blocks.len()],
            successors,
            edge_completion: edge_completions,
            predecessors,
            reachable,
        })
    }

    fn plan(&mut self) -> Result<(), WorldStateSsaDecline> {
        for block_index in 0..self.function.blocks.len() {
            // Unreachable semantic actions must not allocate locations,
            // versions, or impose completion-shape requirements on the live
            // CFG.  Besides keeping diagnostics focused on executable code,
            // this ensures a dead transition cannot perturb the deterministic
            // numbering of the reachable state graph.
            if !self.reachable.contains(&block_index) {
                continue;
            }
            let mut ordinal = 0_u32;
            for instruction_index in 0..self.function.blocks[block_index].instructions.len() {
                let plan_inputs =
                    self.instruction_plan_inputs(block_index, instruction_index, ordinal)?;
                let Some(InstructionPlanInputs {
                    block,
                    completion,
                    ordinary,
                    transition,
                }) = plan_inputs
                else {
                    continue;
                };
                for (kind, location) in ordinary {
                    let token = self.token(
                        kind,
                        location,
                        Self::state_site(block, instruction_index, ordinal)?,
                    );
                    self.plans[block_index].base.push(token);
                    ordinal = next_ordinal(block, instruction_index, ordinal)?;
                }
                self.plan_transition_intents(
                    block_index,
                    block,
                    instruction_index,
                    &mut ordinal,
                    completion,
                    transition,
                )?;
            }
        }
        Ok(())
    }

    fn instruction_plan_inputs(
        &self,
        block_index: usize,
        instruction_index: usize,
        ordinal: u32,
    ) -> Result<Option<InstructionPlanInputs>, WorldStateSsaDecline> {
        let block = &self.function.blocks[block_index];
        let inputs = match &block.instructions[instruction_index] {
            ExecutableInstruction::Invoke(invoke) => {
                let state_site = Self::state_site(block.id, instruction_index, ordinal)?;
                project_invocation_intents(
                    &invoke.resolution,
                    &state_site,
                    block.id,
                    instruction_index,
                )
                .map(|(ordinary, transition)| {
                    Some((block.id, invoke.completion, ordinary, transition))
                })
            }
            ExecutableInstruction::ExecuteLowered(operation) => Ok(Some((
                block.id,
                operation.completion,
                lowered_footprint_intents(&operation.footprint),
                Vec::new(),
            ))),
            ExecutableInstruction::ExecuteOpaqueRegion(region) => Ok(Some((
                block.id,
                region.completion,
                conservative_region_intents(),
                Vec::new(),
            ))),
            ExecutableInstruction::CompleteEvaluatedRegion(completed) => {
                let state_site = Self::state_site(block.id, instruction_index, ordinal)?;
                let residual = completed.region.residual_effects.resolve();
                let intents = project_effect_footprint(&residual, &state_site, None)
                    .map_err(|_| WorldStateSsaDecline::StateSiteOverflow {
                        block: block.id,
                        instruction: instruction_index,
                    })?
                    .into_iter()
                    .map(|intent| (intent.kind, intent.location))
                    .collect();
                Ok(Some((
                    block.id,
                    completed.invocation.completion,
                    intents,
                    Vec::new(),
                )))
            }
            ExecutableInstruction::EvaluateWord {
                value,
                word,
                completion,
                ..
            } if original_word_evaluation_runs_commands(self.function, *value, word) => {
                // A word containing a command substitution (or an
                // opaque fragment that may hide one) runs arbitrary
                // commands during argv evaluation.  Without this
                // barrier a closed outer invocation such as a pure
                // list query would hide a nested `rename` from the
                // world graph.
                Ok(Some((
                    block.id,
                    *completion,
                    conservative_region_intents(),
                    Vec::new(),
                )))
            }
            // Structured control's own operations touch variable
            // cells and nothing else.  An operand that can run
            // commands is the one exception, and it takes the same
            // conservative barrier a command-substituting word does.
            ExecutableInstruction::EvaluateExpr {
                value,
                expr,
                completion,
                ..
            } => {
                let ordinary = executable_expr_intents_with_source(self.function, *value, expr);
                Ok(Some((block.id, *completion, ordinary, Vec::new())))
            }
            ExecutableInstruction::IterateLists {
                groups, completion, ..
            } => Ok(Some((
                block.id,
                *completion,
                cell_intents(
                    groups.iter().flat_map(|group| group.variables.iter()),
                    WorldStateIntentKind::Clobber,
                ),
                Vec::new(),
            ))),
            ExecutableInstruction::WriteCompletionCell {
                cell, completion, ..
            } => Ok(Some((
                block.id,
                *completion,
                cell_intents(std::iter::once(cell), WorldStateIntentKind::Clobber),
                Vec::new(),
            ))),
            _ => Ok(None),
        };
        inputs.map(|inputs| {
            inputs.map(
                |(block, completion, ordinary, transition)| InstructionPlanInputs {
                    block,
                    completion,
                    ordinary,
                    transition,
                },
            )
        })
    }

    fn plan_transition_intents(
        &mut self,
        block_index: usize,
        block: ExecutableBlockId,
        instruction: usize,
        ordinal: &mut u32,
        invocation_completion: crate::executable_ir::CompletionId,
        transitions: Vec<TransitionStateIntent>,
    ) -> Result<(), WorldStateSsaDecline> {
        if transitions.is_empty() {
            return Ok(());
        }
        let Some(ExecutableTerminator::CompletionSwitch {
            completion, cases, ..
        }) = self.function.blocks[block_index].terminator.as_ref()
        else {
            return Err(WorldStateSsaDecline::MissingCompletionSwitch { block });
        };
        if *completion != invocation_completion {
            return Err(WorldStateSsaDecline::CompletionSwitchMismatch { block });
        }
        if !cases.iter().any(|case| case.code == CompletionCode::Ok) {
            return Err(WorldStateSsaDecline::MissingOkCompletionEdge { block });
        }
        if self.plans[block_index].transition_completion.is_some() {
            return Err(WorldStateSsaDecline::CompletionSwitchMismatch { block });
        }
        self.plans[block_index].transition_completion = Some(*completion);
        for intent in transitions {
            self.plan_transition_intent(block_index, block, instruction, ordinal, intent)?;
        }
        Ok(())
    }

    fn plan_transition_intent(
        &mut self,
        block_index: usize,
        block: ExecutableBlockId,
        instruction: usize,
        ordinal: &mut u32,
        intent: TransitionStateIntent,
    ) -> Result<(), WorldStateSsaDecline> {
        if intent.kind == WorldStateIntentKind::Use {
            // A transition read is an execution-time dependency, independent
            // of completion outcome.
            let token = self.token(
                intent.kind,
                intent.location,
                Self::state_site(block, instruction, *ordinal)?,
            );
            self.plans[block_index].base.push(token);
            *ordinal = next_ordinal(block, instruction, *ordinal)?;
            return Ok(());
        }
        let mut edge_tokens = BTreeMap::new();
        for successor in self.successors[block_index].clone() {
            let mode = self.edge_mode(block_index, successor)?;
            let kind = transition_edge_kind(intent.kind, intent.commit, mode);
            let Some(kind) = kind else { continue };
            let token = self.token(
                kind,
                intent.location.clone(),
                Self::edge_site(block, successor, instruction, *ordinal)?,
            );
            *ordinal = next_ordinal(block, instruction, *ordinal)?;
            edge_tokens.insert(successor, token);
        }
        if !edge_tokens.is_empty() {
            self.plans[block_index]
                .transitions
                .push(TransitionToken { edge_tokens });
        }
        Ok(())
    }

    fn state_site(
        block: ExecutableBlockId,
        instruction: usize,
        ordinal: u32,
    ) -> Result<StateSite, WorldStateSsaDecline> {
        let instruction = u32::try_from(instruction)
            .map_err(|_| WorldStateSsaDecline::StateSiteOverflow { block, instruction })?;
        Ok(StateSite::statement(
            block_id(block.index())?,
            instruction,
            ordinal,
        ))
    }

    fn edge_site(
        predecessor: ExecutableBlockId,
        successor: usize,
        instruction: usize,
        ordinal: u32,
    ) -> Result<StateSite, WorldStateSsaDecline> {
        let instruction =
            u32::try_from(instruction).map_err(|_| WorldStateSsaDecline::StateSiteOverflow {
                block: predecessor,
                instruction,
            })?;
        Ok(StateSite::edge(
            block_id(predecessor.index())?,
            block_id(successor)?,
            CfgStatePosition::Statement {
                index: instruction,
                ordinal,
            },
        ))
    }

    fn token(
        &mut self,
        kind: WorldStateIntentKind,
        location: WorldRegion,
        site: StateSite,
    ) -> usize {
        let location = self.location(location);
        let index = self.tokens.len();
        self.tokens.push(Token {
            kind,
            location,
            site,
        });
        index
    }

    fn location(&mut self, location: WorldRegion) -> usize {
        if let Some(index) = self.locations.iter().position(|known| known == &location) {
            index
        } else {
            let index = self.locations.len();
            self.locations.push(location);
            index
        }
    }
}

fn block_id(index: usize) -> Result<BlockId, WorldStateSsaDecline> {
    u32::try_from(index)
        .map(BlockId)
        .map_err(|_| WorldStateSsaDecline::BlockIdOverflow { block: index })
}

pub(crate) fn successors_of(terminator: &ExecutableTerminator) -> Vec<usize> {
    match terminator {
        ExecutableTerminator::Goto(target) => vec![target.index()],
        ExecutableTerminator::Branch {
            then_target,
            else_target,
            ..
        } => {
            vec![then_target.index(), else_target.index()]
        }
        ExecutableTerminator::RegionChoice { enter, skip } => vec![enter.index(), skip.index()],
        ExecutableTerminator::CompletionSwitch { cases, default, .. } => cases
            .iter()
            .map(|case| case.target.index())
            .chain(std::iter::once(default.index()))
            .collect(),
        ExecutableTerminator::ReturnCompletion(_) => Vec::new(),
    }
}

/// Completion-arm categories merged for one predecessor/successor pair.
///
/// The executable CFG identifies predecessor edges by block, while a Tcl
/// completion switch can send both `OK` and an abrupt code to the same block.
/// Retaining these bits prevents an `OK`-only transition from becoming an
/// unconditional precise definition after target deduplication.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct EdgeCompletion {
    pub(crate) ok: bool,
    pub(crate) abrupt: bool,
}

fn transition_edge_kind(
    exact_kind: WorldStateIntentKind,
    commit: StateTransitionCommit,
    edge: EdgeCompletion,
) -> Option<WorldStateIntentKind> {
    if edge.ok && !edge.abrupt {
        Some(exact_kind)
    } else if edge.abrupt
        && (commit == StateTransitionCommit::MayCommitBeforeAbruptCompletion || edge.ok)
    {
        // This is an edge-qualified representation of unchanged +
        // exact-changed state, never a sequential write in the predecessor.
        Some(WorldStateIntentKind::Clobber)
    } else {
        None
    }
}

pub(crate) fn edge_completion(
    terminator: &ExecutableTerminator,
    successor: usize,
) -> EdgeCompletion {
    let ExecutableTerminator::CompletionSwitch { cases, default, .. } = terminator else {
        return EdgeCompletion::default();
    };
    let mut completion = EdgeCompletion {
        ok: false,
        abrupt: default.index() == successor,
    };
    for case in cases {
        if case.target.index() == successor {
            if case.code == CompletionCode::Ok {
                completion.ok = true;
            } else {
                completion.abrupt = true;
            }
        }
    }
    completion
}

fn next_ordinal(
    block: ExecutableBlockId,
    instruction: usize,
    ordinal: u32,
) -> Result<u32, WorldStateSsaDecline> {
    ordinal
        .checked_add(1)
        .ok_or(WorldStateSsaDecline::StateSiteOverflow { block, instruction })
}

#[cfg(test)]
mod tests;

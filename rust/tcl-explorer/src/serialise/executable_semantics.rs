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

//! The `semantic` view: target-neutral executable semantics and every typed decline.

use super::{snapshot_body_key, unproved_implementation_lookup};
use crate::ExplorerResult;
use crate::formatters::range_dict;
use serde_json::{Value, json};
use tcl_compiler::executable_ir::{
    ExecutableInstruction, ExecutableTerminator, InvocationResolution,
    OwnedInvocationResolutionUnresolved,
};
use tcl_compiler::semantic_analysis::ExecutableAnalysisAvailability;
use tcl_compiler::world_state_ssa::WorldStateSsaDecline;
use tcl_lexer::LineIndex;

fn executable_instruction_kind(instruction: &ExecutableInstruction) -> &'static str {
    match instruction {
        ExecutableInstruction::EvaluateWord { .. } => "evaluate-word",
        ExecutableInstruction::ExpandWord { .. } => "expand-word",
        ExecutableInstruction::BuildArgv { .. } => "build-argv",
        ExecutableInstruction::Invoke(_) => "invoke",
        ExecutableInstruction::ExecuteLowered(_) => "execute-lowered",
        ExecutableInstruction::ExecuteOpaqueRegion(_) => "execute-opaque-region",
        ExecutableInstruction::EvaluateExpr { .. } => "evaluate-expr",
        ExecutableInstruction::MatchPattern { .. } => "match-pattern",
        ExecutableInstruction::IterateLists { .. } => "iterate-lists",
        ExecutableInstruction::JoinCompletion { .. } => "join-completion",
        ExecutableInstruction::WriteCompletionCell { .. } => "write-completion-cell",
        ExecutableInstruction::CompleteStructuredRegion(_) => "complete-structured-region",
        ExecutableInstruction::CompleteEvaluatedRegion(_) => "complete-evaluated-region",
    }
}

fn executable_terminator_kind(terminator: Option<&ExecutableTerminator>) -> &'static str {
    match terminator {
        Some(ExecutableTerminator::Goto(_)) => "goto",
        Some(ExecutableTerminator::Branch { .. }) => "branch",
        Some(ExecutableTerminator::RegionChoice { .. }) => "region-choice",
        Some(ExecutableTerminator::CompletionSwitch { .. }) => "completion-switch",
        Some(ExecutableTerminator::ReturnCompletion(_)) => "return-completion",
        None => "missing",
    }
}

fn semantic_status(availability: &ExecutableAnalysisAvailability) -> &'static str {
    match availability {
        ExecutableAnalysisAvailability::Available(_) => "available",
        ExecutableAnalysisAvailability::WorldStateDeclined { .. } => "world-state-declined",
        ExecutableAnalysisAvailability::WorldStateNotRequired { .. } => "world-state-not-required",
        ExecutableAnalysisAvailability::ContextUnavailable => "context-unavailable",
        ExecutableAnalysisAvailability::SourceDeclined(_) => "source-declined",
        ExecutableAnalysisAvailability::SourceUnavailable => "source-unavailable",
    }
}

fn lowering_hook_label(hook: tcl_registry::hooks::LoweringHookId) -> &'static str {
    hook.as_str()
}

pub(super) fn invocation_resolution_label(
    resolution: &tcl_compiler::executable_ir::InvocationResolution,
) -> &'static str {
    match resolution {
        tcl_compiler::executable_ir::InvocationResolution::Resolved(_) => "resolved",
        tcl_compiler::executable_ir::InvocationResolution::Unresolved(
            OwnedInvocationResolutionUnresolved::ComputedHead { .. },
        ) => "unresolved-computed-head",
        tcl_compiler::executable_ir::InvocationResolution::Unresolved(
            OwnedInvocationResolutionUnresolved::UnknownLiteralHead { .. },
        ) => "unresolved-unknown-literal-head",
        tcl_compiler::executable_ir::InvocationResolution::Unresolved(
            OwnedInvocationResolutionUnresolved::UnprovedBinding { .. },
        ) => "unresolved-unproved-binding",
        tcl_compiler::executable_ir::InvocationResolution::Unresolved(
            OwnedInvocationResolutionUnresolved::UnprovedNativeImplementationLookup { .. },
        ) => "unresolved-unproved-native-implementation-lookup",
    }
}

fn world_state_decline_label(decline: &WorldStateSsaDecline) -> &'static str {
    match decline {
        WorldStateSsaDecline::InvalidExecutableIr(_) => "invalid-executable-ir",
        WorldStateSsaDecline::BlockIdOverflow { .. } => "block-id-overflow",
        WorldStateSsaDecline::StateSiteOverflow { .. } => "state-site-overflow",
        WorldStateSsaDecline::MissingCompletionSwitch { .. } => "missing-completion-switch",
        WorldStateSsaDecline::CompletionSwitchMismatch { .. } => "completion-switch-mismatch",
        WorldStateSsaDecline::MissingOkCompletionEdge { .. } => "missing-ok-completion-edge",
        WorldStateSsaDecline::MissingCfgEdge { .. } => "missing-cfg-edge",
        WorldStateSsaDecline::MissingStatePredecessor { .. } => "missing-state-predecessor",
        WorldStateSsaDecline::MissingStateVersion => "missing-state-version",
        WorldStateSsaDecline::InvalidStateSsa(_) => "invalid-state-ssa",
    }
}

fn source_decline_value(
    decline: &tcl_compiler::executable_ir::SourceCompatibilityDecline,
) -> Value {
    use tcl_compiler::executable_ir::SourceCompatibilityDecline;
    match decline {
        SourceCompatibilityDecline::EmptyScript => json!({"kind": "empty-script"}),
        SourceCompatibilityDecline::UnsupportedStatement {
            statement_index,
            kind,
        } => {
            json!({"kind": "unsupported-statement", "statementIndex": statement_index, "statementKind": kind})
        }
        SourceCompatibilityDecline::MissingCommandTokens { statement_index } => {
            json!({"kind": "missing-command-tokens", "statementIndex": statement_index})
        }
        SourceCompatibilityDecline::InconsistentCommandTokens { statement_index } => {
            json!({"kind": "inconsistent-command-tokens", "statementIndex": statement_index})
        }
        SourceCompatibilityDecline::MissingCommandHead { statement_index } => {
            json!({"kind": "missing-command-head", "statementIndex": statement_index})
        }
        SourceCompatibilityDecline::IncompleteRegistryResolution { statement_index } => {
            json!({"kind": "incomplete-registry-resolution", "statementIndex": statement_index})
        }
    }
}

fn semantic_decline_value(availability: &ExecutableAnalysisAvailability) -> Option<Value> {
    match availability {
        // The re-keyed sidecar reaches this state by carrying no resolved
        // environment at all, so there is no mask left to name in the
        // payload.
        ExecutableAnalysisAvailability::ContextUnavailable => {
            Some(json!({"kind": "context-unavailable"}))
        }
        ExecutableAnalysisAvailability::WorldStateDeclined { decline, .. } => Some(
            json!({"kind": "world-state-declined", "reason": world_state_decline_label(decline)}),
        ),
        ExecutableAnalysisAvailability::WorldStateNotRequired { .. } => {
            Some(json!({"kind": "world-state-not-required"}))
        }
        ExecutableAnalysisAvailability::SourceDeclined(decline) => {
            Some(source_decline_value(decline))
        }
        ExecutableAnalysisAvailability::SourceUnavailable => {
            Some(json!({"kind": "source-unavailable"}))
        }
        ExecutableAnalysisAvailability::Available(_) => None,
    }
}

/// Serialise target-neutral executable semantics and every typed decline.
/// World-state SSA is deliberately not copied here: its ownership is an
/// explicit separate Explorer tranche, while executable IR and proof status
/// are common compiler artefacts.
#[must_use]
pub fn serialise_semantic(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    Value::Array(
        result
            .all_snapshots()
            .iter()
            .map(|snap| {
                let availability = snap.unit.semantic_facts.executable();
                let function = availability.function();
                let blocks: Vec<Value> = function
                    .into_iter()
                    .flat_map(|function| function.blocks.iter())
                    .map(|block| {
                        json!({
                            "index": block.id.index(),
                            "instructions": block.instructions.iter().map(|instruction| json!({
                                "kind": executable_instruction_kind(instruction),
                            })).collect::<Vec<_>>(),
                            "terminator": executable_terminator_kind(block.terminator.as_ref()),
                        })
                    })
                    .collect();
                let invocations: Vec<Value> = snap
                    .unit
                    .semantic_facts
                    .executable()
                    .invocations()
                    .map(|invoke| {
                        json!({
                            "node": invoke.node.path(),
                            "resolution": invocation_resolution_label(&invoke.resolution),
                            "implementationLookup": match &invoke.resolution {
                                InvocationResolution::Resolved(_) => Value::Null,
                                InvocationResolution::Unresolved(reason) => unproved_implementation_lookup(reason),
                            },
                            "range": range_dict(invoke.source.span, li, source),
                        })
                    })
                    .collect();
                let lowered = snap
                    .unit
                    .semantic_facts
                    .executable()
                    .lowered_operations()
                    .map(|operation| {
                        json!({
                            "operation": lowering_hook_label(operation.descriptor),
                            "range": range_dict(operation.source.span, li, source),
                        })
                    })
                    .collect::<Vec<_>>();
                let opaque = snap
                    .unit
                    .semantic_facts
                    .executable()
                    .opaque_regions()
                    .map(|region| {
                        json!({
                            "descriptor": region.descriptor.map(lowering_hook_label),
                            "range": range_dict(region.source.span, li, source),
                        })
                    })
                    .collect::<Vec<_>>();
                json!({
                    "name": snap.name, "bodyId": snap.unit.body_id.map(tcl_compiler::ir::ProcedureId::ordinal), "bodyKey": snapshot_body_key(result, snap),
                    "kind": snap.kind.as_str(),
                    "status": semantic_status(availability),
                    "decline": semantic_decline_value(availability),
                    "complexityGuarded": snap.unit.complexity_guarded,
                    "semanticValues": snap.unit.semantic_values().map(|facts| json!({
                        "purpose": "known-contents-with-retained-producer-obligations",
                        "contentsCount": facts.contents_iter().count(),
                        "expressionCount": facts.expression_iter().count(),
                        "licensesErasure": false,
                    })),
                    "tier": snap.unit.tier.as_str(),
                    "dynamicNames": {
                        "writes": snap.unit.dynamic_names.writes,
                        "destroys": snap.unit.dynamic_names.destroys,
                        "reads": snap.unit.dynamic_names.reads,
                    },
                    "conditionalEvent": snap.unit.irules_event_body.as_ref().map(|event| json!({
                        "event": event.event(),
                        "purpose": "conditional-source-frame",
                        "licensesEnteredEvent": false,
                    })),
                    "methodFacts": snap.unit.method_facts.as_ref().map(|facts| json!({
                        "params": facts.params,
                        "instanceVars": facts.instance_vars.iter().cloned().collect::<Vec<_>>(),
                    })),
                    "blocks": blocks,
                    "invocations": invocations,
                    "lowered": lowered,
                    "opaque": opaque,
                })
            })
            .collect(),
    )
}

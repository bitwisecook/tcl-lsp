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

//! The world-state SSA view: typed regions, operations, invocations and availability.

use super::unproved_implementation_lookup;
use crate::ExplorerResult;
use serde_json::{Value, json};
use tcl_compiler::effect_ssa::WorldStateIntentKind;
use tcl_compiler::executable_ir::InvocationResolution;
use tcl_compiler::semantic_analysis::ExecutableAnalysisAvailability;
use tcl_compiler::state_ssa::adapters::{
    WorldInterpreterScope, WorldNamespaceScope, WorldRegion, WorldRegionKind, WorldSubjectScope,
};
use tcl_compiler::state_ssa::{CfgStatePosition, StateOp, StateSite};
use tcl_compiler::world_state_ssa::project_transition_facts;

/// The stable name of a tracked mutable-world domain.
pub(super) fn world_domain_name(kind: WorldRegionKind) -> &'static str {
    match kind {
        WorldRegionKind::InterpreterTopology => "interpreter-topology",
        WorldRegionKind::CommandBindings => "command-bindings",
        WorldRegionKind::NamespaceLookup => "namespace-lookup",
        WorldRegionKind::NamespaceUnknown => "namespace-unknown",
        WorldRegionKind::ExecutionTraces => "execution-traces",
        WorldRegionKind::VariableTraces => "variable-traces",
        WorldRegionKind::CommandTraces => "command-traces",
        WorldRegionKind::ObjectDispatch => "object-dispatch",
        WorldRegionKind::InterpreterPolicy => "interpreter-policy",
        WorldRegionKind::PackageState => "package-state",
        WorldRegionKind::HostCapabilities => "host-capabilities",
        WorldRegionKind::VariableStore => "variable-store",
        WorldRegionKind::InterpreterResult => "interpreter-result",
        WorldRegionKind::CompletionState => "completion-state",
        WorldRegionKind::ExternalResource(_) => "external-resource",
    }
}

fn serialise_interpreter_scope(scope: &WorldInterpreterScope) -> Value {
    match scope {
        WorldInterpreterScope::Current => json!({ "kind": "current" }),
        WorldInterpreterScope::Named(name) => json!({ "kind": "named", "name": name }),
        WorldInterpreterScope::Any => json!({ "kind": "any" }),
    }
}

fn serialise_namespace_scope(scope: &WorldNamespaceScope) -> Value {
    match scope {
        WorldNamespaceScope::Current => json!({ "kind": "current" }),
        WorldNamespaceScope::Named(name) => json!({ "kind": "named", "name": name }),
        WorldNamespaceScope::Any => json!({ "kind": "any" }),
    }
}

fn serialise_subject_scope(scope: &WorldSubjectScope) -> Value {
    match scope {
        WorldSubjectScope::Named(name) => json!({ "kind": "named", "name": name }),
        WorldSubjectScope::Wildcard => json!({ "kind": "wildcard" }),
    }
}

/// Serialise a typed world-state location without falling back to a display
/// string.  Consumers can therefore distinguish a literal identity from a
/// dynamic wildcard, which is essential when explaining a GVN abstention.
fn serialise_world_region(region: &WorldRegion) -> Value {
    match region {
        WorldRegion::Scoped {
            kind,
            interpreter,
            namespace,
            subject,
        } => {
            let mut value = json!({
                "kind": "scoped",
                "domain": world_domain_name(*kind),
                "interpreter": serialise_interpreter_scope(interpreter),
                "namespace": serialise_namespace_scope(namespace),
                "subject": serialise_subject_scope(subject),
            });
            if let WorldRegionKind::ExternalResource(target) = kind {
                // The external resource partition is an established registry
                // enum, not an untyped all-external bucket. Retain its exact
                // identity even though it has no Tcl namespace component.
                value["externalTarget"] = json!(target.as_str());
            }
            value
        }
        WorldRegion::InterpreterWildcard { interpreter } => json!({
            "kind": "interpreter-wildcard",
            "domain": "all",
            "interpreter": serialise_interpreter_scope(interpreter),
        }),
        WorldRegion::NamespaceSubtree {
            interpreter,
            namespace,
        } => json!({
            "kind": "namespace-subtree",
            "domain": "namespace-owned",
            "interpreter": serialise_interpreter_scope(interpreter),
            "namespace": serialise_namespace_scope(namespace),
        }),
        WorldRegion::NamespaceLineage {
            interpreter,
            namespace,
        } => json!({
            "kind": "namespace-lineage",
            "domain": "namespace-lookup",
            "interpreter": serialise_interpreter_scope(interpreter),
            "namespace": serialise_namespace_scope(namespace),
        }),
        WorldRegion::Any => json!({ "kind": "any", "domain": "all" }),
    }
}

fn serialise_cfg_position(position: CfgStatePosition) -> Value {
    match position {
        CfgStatePosition::Phi { ordinal } => json!({ "kind": "phi", "ordinal": ordinal }),
        CfgStatePosition::Statement { index, ordinal } => {
            json!({ "kind": "statement", "index": index, "ordinal": ordinal })
        }
        CfgStatePosition::Terminator { ordinal } => {
            json!({ "kind": "terminator", "ordinal": ordinal })
        }
    }
}

fn serialise_state_site(site: &StateSite) -> Value {
    match site {
        StateSite::Node { node, ordinal } => {
            json!({ "kind": "node", "path": node.path(), "ordinal": ordinal })
        }
        StateSite::Cfg(cfg) => json!({
            "kind": "cfg",
            "block": cfg.block.0,
            "position": serialise_cfg_position(cfg.position),
        }),
        StateSite::Edge(edge) => json!({
            "kind": "edge",
            "predecessor": edge.predecessor.0,
            "successor": edge.successor.0,
            "origin": serialise_cfg_position(edge.origin),
        }),
    }
}

fn serialise_world_operation(operation: &StateOp<WorldRegion>) -> Value {
    let mut value = json!({
        "kind": match operation {
            StateOp::Use(_) => "use",
            StateOp::Def(_) => "def",
            StateOp::Phi(_) => "phi",
            StateOp::Clobber(_) => "clobber",
        },
        "location": serialise_world_region(operation.location()),
        "site": serialise_state_site(operation.site()),
        "version": operation.version().raw(),
        "definesVersion": operation.defines_version(),
    });
    match operation {
        StateOp::Use(use_op) => value["reachingVersion"] = json!(use_op.reaching_version.raw()),
        StateOp::Phi(phi) => {
            value["incoming"] = Value::Array(
                phi.incoming
                    .iter()
                    .map(|(block, version)| json!({ "block": block.0, "version": version.raw() }))
                    .collect(),
            );
            value["includesInitial"] = json!(phi.includes_initial());
        }
        StateOp::Def(_) | StateOp::Clobber(_) => {}
    }
    value
}

fn transition_kind_name(transition: &tcl_registry::StateTransition) -> &'static str {
    match transition {
        tcl_registry::StateTransition::CommandBinding(_) => "command-binding",
        tcl_registry::StateTransition::Interpreter(_) => "interpreter",
        tcl_registry::StateTransition::VariableCellAlias(_) => "variable-cell-alias",
        tcl_registry::StateTransition::Namespace(_) => "namespace",
        tcl_registry::StateTransition::Trace(_) => "trace",
        tcl_registry::StateTransition::ObjectDispatch(_) => "object-dispatch",
        tcl_registry::StateTransition::Package(_) => "package",
        tcl_registry::StateTransition::Widen(_) => "widen",
    }
}

pub(super) fn registry_world_domain_name(domain: tcl_registry::WorldStateDomain) -> String {
    match domain {
        tcl_registry::WorldStateDomain::InterpreterTopology => "interpreter-topology".to_owned(),
        tcl_registry::WorldStateDomain::CommandBindings => "command-bindings".to_owned(),
        tcl_registry::WorldStateDomain::NamespaceLookup => "namespace-lookup".to_owned(),
        tcl_registry::WorldStateDomain::NamespaceUnknown => "namespace-unknown".to_owned(),
        tcl_registry::WorldStateDomain::ExecutionTraces => "execution-traces".to_owned(),
        tcl_registry::WorldStateDomain::VariableTraces => "variable-traces".to_owned(),
        tcl_registry::WorldStateDomain::CommandTraces => "command-traces".to_owned(),
        tcl_registry::WorldStateDomain::OoDispatch => "object-dispatch".to_owned(),
        tcl_registry::WorldStateDomain::InterpreterPolicy => "interpreter-policy".to_owned(),
        tcl_registry::WorldStateDomain::PackageState => "package-state".to_owned(),
        tcl_registry::WorldStateDomain::HostCapabilities => "host-capabilities".to_owned(),
        tcl_registry::WorldStateDomain::VariableStore => "variable-store".to_owned(),
        tcl_registry::WorldStateDomain::ThreadSharedStore => "thread-shared-store".to_owned(),
        tcl_registry::WorldStateDomain::InterpreterResult => "interpreter-result".to_owned(),
        tcl_registry::WorldStateDomain::CompletionState => "completion-state".to_owned(),
        tcl_registry::WorldStateDomain::LegacyExternal(target) => {
            format!("external-resource:{}", target.as_str())
        }
    }
}

fn serialise_result_stability(stability: tcl_registry::ResultStability) -> Value {
    match stability {
        tcl_registry::ResultStability::Unknown => json!({ "kind": "unknown", "domains": [] }),
        tcl_registry::ResultStability::ReferentiallyTransparent => {
            json!({ "kind": "referentially-transparent", "domains": [] })
        }
        tcl_registry::ResultStability::ReadsVersionedWorld(domains) => json!({
            "kind": "reads-versioned-world",
            "domains": domains.iter().copied().map(registry_world_domain_name).collect::<Vec<_>>(),
        }),
        tcl_registry::ResultStability::Volatile => json!({ "kind": "volatile", "domains": [] }),
    }
}

fn dispatch_dependency_name(domain: tcl_registry::DispatchDependencyDomain) -> &'static str {
    match domain {
        tcl_registry::DispatchDependencyDomain::CommandBinding => "command-binding",
        tcl_registry::DispatchDependencyDomain::NamespaceLookup => "namespace-lookup",
        tcl_registry::DispatchDependencyDomain::CommandTraces => "command-traces",
        tcl_registry::DispatchDependencyDomain::InterpreterPolicy => "interpreter-policy",
        tcl_registry::DispatchDependencyDomain::ObjectDispatch => "object-dispatch",
        tcl_registry::DispatchDependencyDomain::UnknownHandling => "unknown-handling",
    }
}

fn serialise_world_invocations(availability: &ExecutableAnalysisAvailability) -> Value {
    Value::Array(
        availability
            .invocations()
            .map(|invoke| match &invoke.resolution {
                InvocationResolution::Resolved(facts) => {
                    let (transition_knowledge, transitions) = facts.state_transitions.declared().map_or_else(
                        || ("unknown", Vec::new()),
                        |declared| (
                            "declared",
                            declared
                                .facts()
                                .iter()
                                .map(|fact| {
                                    let intents = project_transition_facts(std::slice::from_ref(fact));
                                    let (commit, abrupt_transfer) = match fact.commit {
                                        tcl_registry::StateTransitionCommit::OnOkOnly => ("on-ok-only", "unchanged"),
                                        tcl_registry::StateTransitionCommit::MayCommitBeforeAbruptCompletion => {
                                            ("may-commit-before-abrupt-completion", "join-with-transition")
                                        }
                                    };
                                    json!({
                                        "kind": transition_kind_name(&fact.transition),
                                        "commit": commit,
                                        "abruptTransfer": abrupt_transfer,
                                        "intents": intents.into_iter().map(|intent| json!({
                                            "kind": match intent.kind {
                                                WorldStateIntentKind::Use => "use",
                                                WorldStateIntentKind::Def => "def",
                                                WorldStateIntentKind::Clobber => "clobber",
                                            },
                                            "location": serialise_world_region(&intent.location),
                                            "commit": match intent.commit {
                                                tcl_registry::StateTransitionCommit::OnOkOnly => "on-ok-only",
                                                tcl_registry::StateTransitionCommit::MayCommitBeforeAbruptCompletion => "may-commit-before-abrupt-completion",
                                            },
                                        })).collect::<Vec<_>>(),
                                    })
                                })
                                .collect(),
                        ),
                    );
                    json!({
                        "resolution": "resolved",
                        "command": facts.canonical_command,
                        "node": invoke.node.path(),
                        "completion": invoke.completion.index(),
                        "transitionKnowledge": transition_knowledge,
                        "transitions": transitions,
                        "proof": {
                            "resultStability": serialise_result_stability(facts.result_stability),
                            "dispatchDependencies": facts.dispatch_dependencies.iter()
                                .map(dispatch_dependency_name).collect::<Vec<_>>(),
                            "argumentRolesComplete": facts.arg_roles_complete,
                        },
                    })
                }
                InvocationResolution::Unresolved(reason) => {
                    let abstention = match reason {
                        tcl_compiler::registry_invocation::OwnedInvocationResolutionUnresolved::ComputedHead { .. } => "computed-head",
                        tcl_compiler::registry_invocation::OwnedInvocationResolutionUnresolved::UnknownLiteralHead { .. } => "unknown-literal-head",
                        tcl_compiler::registry_invocation::OwnedInvocationResolutionUnresolved::UnprovedBinding { .. } => "unproved-binding",
                        tcl_compiler::registry_invocation::OwnedInvocationResolutionUnresolved::UnprovedNativeImplementationLookup { .. } => "unproved-native-implementation-lookup",
                    };
                    json!({
                        "resolution": "unresolved",
                        "node": invoke.node.path(),
                        "completion": invoke.completion.index(),
                        "proof": { "abstention": abstention, "implementationLookup": unproved_implementation_lookup(reason) },
                    })
                }
            })
            .collect(),
    )
}

fn availability_kind(availability: &ExecutableAnalysisAvailability) -> &'static str {
    match availability {
        ExecutableAnalysisAvailability::ContextUnavailable => "context-unavailable",
        ExecutableAnalysisAvailability::Available(_) => "available",
        ExecutableAnalysisAvailability::WorldStateDeclined { .. } => "world-state-declined",
        ExecutableAnalysisAvailability::WorldStateNotRequired { .. } => "world-state-not-required",
        ExecutableAnalysisAvailability::SourceDeclined(_) => "source-declined",
        ExecutableAnalysisAvailability::SourceUnavailable => "source-unavailable",
    }
}

fn availability_reason_kind(availability: &ExecutableAnalysisAvailability) -> Option<&'static str> {
    match availability {
        ExecutableAnalysisAvailability::ContextUnavailable
        | ExecutableAnalysisAvailability::Available(_)
        | ExecutableAnalysisAvailability::WorldStateNotRequired { .. }
        | ExecutableAnalysisAvailability::SourceUnavailable => None,
        ExecutableAnalysisAvailability::WorldStateDeclined { decline, .. } => Some(match decline {
            tcl_compiler::world_state_ssa::WorldStateSsaDecline::InvalidExecutableIr(_) => "invalid-executable-ir",
            tcl_compiler::world_state_ssa::WorldStateSsaDecline::BlockIdOverflow { .. } => "block-id-overflow",
            tcl_compiler::world_state_ssa::WorldStateSsaDecline::StateSiteOverflow { .. } => "state-site-overflow",
            tcl_compiler::world_state_ssa::WorldStateSsaDecline::MissingCompletionSwitch { .. } => "missing-completion-switch",
            tcl_compiler::world_state_ssa::WorldStateSsaDecline::CompletionSwitchMismatch { .. } => "completion-switch-mismatch",
            tcl_compiler::world_state_ssa::WorldStateSsaDecline::MissingOkCompletionEdge { .. } => "missing-ok-completion-edge",
            tcl_compiler::world_state_ssa::WorldStateSsaDecline::MissingCfgEdge { .. } => "missing-cfg-edge",
            tcl_compiler::world_state_ssa::WorldStateSsaDecline::MissingStatePredecessor { .. } => "missing-state-predecessor",
            tcl_compiler::world_state_ssa::WorldStateSsaDecline::MissingStateVersion => "missing-state-version",
            tcl_compiler::world_state_ssa::WorldStateSsaDecline::InvalidStateSsa(_) => "invalid-state-ssa",
        }),
        ExecutableAnalysisAvailability::SourceDeclined(decline) => Some(match decline {
            tcl_compiler::executable_ir::SourceCompatibilityDecline::EmptyScript => "empty-script",
            tcl_compiler::executable_ir::SourceCompatibilityDecline::UnsupportedStatement { .. } => "unsupported-statement",
            tcl_compiler::executable_ir::SourceCompatibilityDecline::MissingCommandTokens { .. } => "missing-command-tokens",
            tcl_compiler::executable_ir::SourceCompatibilityDecline::InconsistentCommandTokens { .. } => "inconsistent-command-tokens",
            tcl_compiler::executable_ir::SourceCompatibilityDecline::MissingCommandHead { .. } => "missing-command-head",
            tcl_compiler::executable_ir::SourceCompatibilityDecline::IncompleteRegistryResolution { .. } => "incomplete-registry-resolution",
        }),
    }
}

/// Serialise the compiler's existing typed world-state SSA sidecar.
///
/// The availability discriminant is always emitted, including every decline,
/// so front ends explain why proof data is absent rather than treating absent
/// operations as proof that no mutable world state exists.
pub(super) fn serialise_world_ssa(result: &ExplorerResult) -> Value {
    Value::Array(
        result
            .all_snapshots()
            .into_iter()
            .map(|snapshot| {
                let availability = snapshot.unit.semantic_facts.executable();
                let state = availability.world_state_ssa();
                json!({
                    "name": snapshot.name,
                    "kind": snapshot.kind.as_str(),
                    "availability": {
                        "kind": availability_kind(availability),
                        "hasExecutableIr": availability.function().is_some(),
                        "reasonKind": availability_reason_kind(availability),
                    },
                    "locations": state.map_or_else(Vec::new, |state| {
                        state.locations.iter().map(serialise_world_region).collect()
                    }),
                    "operations": state.map_or_else(Vec::new, |state| {
                        state.state.operations().iter().map(serialise_world_operation).collect()
                    }),
                    "invocations": serialise_world_invocations(availability),
                })
            })
            .collect(),
    )
}

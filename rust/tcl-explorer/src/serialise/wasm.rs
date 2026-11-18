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

//! The `wasm`, `aot` and `semanticOptimisations` views: the codegen plan, the native tier and the mixed-region plan.

use super::{retained_explorer_source, root_body_key};
use crate::ExplorerResult;
use serde_json::{Value, json};
use tcl_compiler::mixed_region_plan::{
    GuardedCandidateDecision, GuardedCandidateDecline, RegionPlan,
};
use tcl_compiler::semantic_analysis::MixedRegionPlanAvailability;
use tcl_compiler::semantic_optimisation::{SemanticOptimisationConfig, SemanticOptimisationPassId};
use tcl_lexer::LineIndex;

/// The `aot` view's payload: the codegen plan the `wasm` view's module header
/// carries, so the plan is serialised once and the two views cannot disagree.
pub(super) fn wasm_codegen_plan(wasm: &Value) -> Value {
    wasm.get(0)
        .and_then(|header| header.get("codegenPlan"))
        .cloned()
        .unwrap_or(Value::Null)
}

/// Serialise the `semanticOptimisations` view: one row per pass, in
/// [`SemanticOptimisationPassId::all`] order, with the state the shown
/// `wasm` module was built with.
///
/// This is the toggle surface. A front-end renders a checkbox per row from
/// `id` and `enabled` and sends the ids it wants back — it never needs its
/// own copy of the pass list, which is the drift this view exists to
/// prevent. `groups` names the presets `from_names` also accepts.
#[must_use]
pub fn serialise_semantic_optimisations(config: SemanticOptimisationConfig) -> Value {
    let passes: Vec<Value> = SemanticOptimisationPassId::all()
        .into_iter()
        .map(|pass| {
            serde_json::json!({
                "id": pass.as_str(),
                "enabled": config.is_enabled(pass),
            })
        })
        .collect();
    let groups: Vec<Value> = tcl_compiler::semantic_optimisation::PASS_GROUPS
        .iter()
        .map(|(id, members)| {
            serde_json::json!({
                "id": *id,
                "passes": members.iter().map(|p| p.as_str()).collect::<Vec<_>>(),
            })
        })
        .collect();
    serde_json::json!({ "passes": passes, "groups": groups })
}

/// Serialise the `wasm` view: drive the analysis-aware WASM emitter used by
/// `tcl compwasm` under `options` and emit the rich per-instruction explorer
/// shape (`wasm_explorer::wasm_to_explorer_json` — resolved `call` targets,
/// paired `br`/`br_if` targets, block-pairing indices, per-instruction
/// ranges). The synthetic `(module)` entry additionally carries the full WAT
/// `text`, so both the text renderer and the web GUI read one shape; the
/// module-wide counts (`functionCount` / `totalInstrCount`) and the type
/// section come from `wasm_to_explorer_json` itself.
///
/// `options` carries the semantic/AOT optimisation configuration, so the same
/// source can be shown as the generic lowering or as any pass selection —
/// which is what the Explorer's per-pass toggles change.
pub(super) fn serialise_wasm_with_options(
    result: &ExplorerResult,
    options: tcl_codegen::codegen::wasm::WasmCompileOptions,
) -> Value {
    use tcl_codegen::codegen::wasm::{WasmCodegenPlan, WasmSemanticDecline, compile_wasm};

    let Some(retained) = retained_explorer_source(result, &result.source) else {
        return Value::Null;
    };
    let registry = retained.registry;
    let mut wasm = compile_wasm(&result.unit, registry, options);
    let wat = wasm.to_wat();

    // Rich per-instruction entries (module header first, then functions).
    let li = LineIndex::new(&result.source);
    let mut entries = crate::wasm_explorer::wasm_to_explorer_json_named(
        &wasm,
        &li,
        &result.source,
        &result.unit.ir_module.procedures,
    );

    for entry in &mut entries {
        if entry["isTopLevel"].as_bool() == Some(true) {
            entry["bodyKey"] = json!(root_body_key(result));
        }
    }

    // Augment the synthetic `(module)` header with the WAT text the TUI
    // renderer reads (it reads `text` on the module entry and
    // `name`/`instrCount` on each function entry, both already present).
    if let Some(Value::Object(header)) = entries.first_mut() {
        header.insert("text".to_owned(), Value::String(wat));
        let (region_status, regions, region_decline) =
            serialise_wasm_region_plan(wasm.plan.region_plan());
        let native_declines = serialise_native_declines(wasm.plan.native_declines());
        let plan = match &wasm.plan {
            WasmCodegenPlan::NativeI64Add { native, .. } => json!({
                "kind": wasm.plan.as_str(),
                "operation": wasm.plan.operation_kind(),
                "semanticDecline": Value::Null,
                "nativeI64Add": {
                    "callee": native.callee.qualified_name,
                    "operands": [native.left, native.right],
                    "boundaryOperation": {
                        "kind": native.boundary_operation.kind_str(),
                        "id": native.boundary_operation.detail_str(),
                    },
                    "frameElided": native.frame_elided,
                    "closedProgramStatements": native.closed_program_statements,
                },
                "regionPlanStatus": region_status,
                "regionPlanDecline": region_decline,
                "regions": regions,
                "nativeDeclines": native_declines,
            }),
            WasmCodegenPlan::GenericInvoke { .. } => json!({
                "kind": wasm.plan.as_str(),
                "operation": wasm.plan.operation_kind(),
                "semanticDecline": Value::Null,
                "regionPlanStatus": region_status,
                "regionPlanDecline": region_decline,
                "regions": regions,
                "nativeDeclines": native_declines,
            }),
            WasmCodegenPlan::General {
                semantic_decline, ..
            } => {
                let evidence = match semantic_decline {
                    WasmSemanticDecline::Packaging(constraint) => json!({
                        "kind": semantic_decline.as_str(),
                        "detailKind": constraint.as_str(),
                    }),
                    WasmSemanticDecline::ExecutableUnavailable(decline) => json!({
                        "kind": semantic_decline.as_str(),
                        "availability": decline.as_str(),
                        "detailKind": decline.detail_kind(),
                    }),
                    WasmSemanticDecline::NativeCompilationAdmissionRequired
                    | WasmSemanticDecline::NativeCompilationSourceUnavailable
                    | WasmSemanticDecline::SemanticPlansDisabled
                    | WasmSemanticDecline::BackendSelection(_)
                    | WasmSemanticDecline::PlanLayout(_)
                    | WasmSemanticDecline::SelectorRegistration => json!({
                        "kind": semantic_decline.as_str(),
                        "detailKind": semantic_decline.detail_kind(),
                    }),
                };
                json!({
                    "kind": wasm.plan.as_str(),
                    "operation": Value::Null,
                    "semanticDecline": evidence,
                    "regionPlanStatus": region_status,
                    "regionPlanDecline": region_decline,
                    "regions": regions,
                    "nativeDeclines": native_declines,
                })
            }
        };
        let mut plan = plan;
        if let Value::Object(map) = &mut plan {
            map.insert(
                "nativeLowering".to_owned(),
                serialise_native_tier(&wasm.native, &root_body_key(result)),
            );
        }
        header.insert("codegenPlan".to_owned(), plan);
    }
    Value::Array(entries)
}

/// The native tier's per-function lowering and elision record: which
/// functions lowered, why the others declined, and per statement how it was
/// lowered and every cell-framing decision taken for it. Stable spellings
/// throughout.
fn serialise_native_tier(
    report: &tcl_compiler::native_lowering::NativeTierReport,
    root_key: &str,
) -> Value {
    use tcl_compiler::native_lowering::{FunctionStatus, NativeFunctionId, StatementOutcome};
    let functions: serde_json::Map<String, Value> = report
        .functions
        .iter()
        .map(|(id, named)| {
            let function = &named.report;
            let (key, body_id, is_top_level) = match id {
                NativeFunctionId::TopLevel => ("topLevel".to_owned(), None, true),
                NativeFunctionId::Procedure(id) => {
                    (format!("body:{}", id.ordinal()), Some(id.ordinal()), false)
                }
            };
            let (status, reason, detail) = match &function.status {
                FunctionStatus::Lowered => ("lowered", Value::Null, Value::Null),
                FunctionStatus::Declined(decline) => (
                    "declined",
                    json!(decline.as_str()),
                    decline.detail().map_or(Value::Null, |detail| json!(detail)),
                ),
            };
            let statements: Vec<Value> = function
                .statements
                .iter()
                .map(|statement| {
                    let reason = match statement.outcome {
                        StatementOutcome::EvalSource(reason) => json!(reason.as_str()),
                        _ => Value::Null,
                    };
                    json!({
                        "node": statement.node.as_ref().map(|node| node.path().to_vec()),
                        "instruction": statement.instruction,
                        "outcome": statement.outcome.as_str(),
                        "reason": reason,
                        "representations": statement.representations,
                        "cells": statement.cells.iter().map(|cell| json!({
                            "place": cell.place,
                            "access": cell.access.as_str(),
                            "storage": cell.storage.storage.as_str(),
                            "storageReason": cell.storage.reason.as_str(),
                            "barrier": cell.barrier.as_str(),
                            "shadowed": cell.shadowed,
                        })).collect::<Vec<_>>(),
                    })
                })
                .collect();
            (
                key,
                json!({
                    "name": named.name,
                    "bodyId": body_id,
                    "bodyKey": match id { NativeFunctionId::TopLevel => root_key.to_owned(), NativeFunctionId::Procedure(id) => id.reporting_key() },
                    "isTopLevel": is_top_level,
                    "status": status,
                    "reason": reason,
                    "detail": detail,
                    "binding": function.binding.as_str(),
                    "bindingReason": function
                        .binding
                        .reason()
                        .map_or(Value::Null, |reason| json!(reason.as_str())),
                    "statements": statements,
                }),
            )
        })
        .collect();
    json!({
        "enabled": report.enabled,
        "functions": Value::Object(functions),
    })
}

/// Every premise the sealed native i64 addition rejected, in evaluation
/// order. Empty when the addition was selected.
fn serialise_native_declines(declines: &[tcl_compiler::common_aot_plan::NativeDecline]) -> Value {
    Value::Array(
        declines
            .iter()
            .map(|decline| {
                json!({
                    "premise": decline.premise.as_str(),
                    "reason": decline.reason.as_str(),
                    "detail": native_decline_detail(decline),
                    "sites": decline.sites.iter().map(direct_site_identity).collect::<Vec<_>>(),
                })
            })
            .collect(),
    )
}

/// The typed extras a decline carries beyond its premise and reason.
fn native_decline_detail(decline: &tcl_compiler::common_aot_plan::NativeDecline) -> Value {
    use tcl_compiler::common_aot_plan::{
        DirectProcBodyDecline, DirectProcDecline, NativeDeclineReason, NativePremise,
    };
    match (&decline.premise, &decline.reason) {
        (NativePremise::Pass(pass), _) => json!({ "pass": pass.as_str() }),
        (_, NativeDeclineReason::ExcludedSurfaces(surfaces)) => json!({
            "surfaces": surfaces
                .iter()
                .copied()
                .map(tcl_compiler::common_aot_plan::CommonAotCoverageDecline::as_str)
                .collect::<Vec<_>>(),
        }),
        (
            _,
            NativeDeclineReason::DirectCall(DirectProcDecline::ArityMismatch { expected, actual }),
        ) => json!({ "expected": expected, "actual": actual }),
        (
            _,
            NativeDeclineReason::DirectBody(
                DirectProcBodyDecline::InternalDispatchUntrusted { operation }
                | DirectProcBodyDecline::InternalExecutionTrace { operation },
            ),
        ) => json!({ "operation": serialise_semantic_operation(Some(*operation)) }),
        _ => json!({}),
    }
}

fn direct_site_identity(site: &tcl_compiler::common_aot_plan::DirectCallSiteId) -> Value {
    json!({
        "function": site.function,
        "block": site.block.0,
        "statementIndex": site.statement_index,
        "nestedArgument": site.nested_argument,
    })
}

fn serialise_wasm_region_plan(availability: &MixedRegionPlanAvailability) -> (Value, Value, Value) {
    match availability {
        MixedRegionPlanAvailability::Available(plan) => (
            json!("available"),
            Value::Array(
                plan.regions()
                    .map(|(node, region)| {
                        let (slow_path, candidates) = match region {
                            RegionPlan::Invocation(invocation) => (
                                json!({
                                    "kind": "prebuilt-argv",
                                    "argv": argv_identity(invocation.slow_path().argv()),
                                    "completion": completion_identity(
                                        invocation.slow_path().completion()
                                    ),
                                    "wordReplay": false,
                                }),
                                Value::Array(
                                    invocation
                                        .candidates()
                                        .iter()
                                        .map(serialise_guarded_candidate)
                                        .collect(),
                                ),
                            ),
                            RegionPlan::Lowered(_)
                            | RegionPlan::Opaque(_)
                            | RegionPlan::Structured(_) => (Value::Null, Value::Array(Vec::new())),
                        };
                        json!({
                            "node": node.path(),
                            "selectedKind": region.selected_kind().as_str(),
                            "operation": serialise_semantic_operation(region.operation()),
                            "completion": completion_identity(region.completion()),
                            "slowPath": slow_path,
                            "candidates": candidates,
                        })
                    })
                    .collect(),
            ),
            Value::Null,
        ),
        MixedRegionPlanAvailability::ExecutableUnavailable => (
            json!("executable-unavailable"),
            Value::Array(Vec::new()),
            Value::Null,
        ),
        MixedRegionPlanAvailability::Declined(decline) => (
            json!("declined"),
            Value::Array(Vec::new()),
            json!({ "kind": decline.as_str() }),
        ),
    }
}

fn serialise_guarded_candidate(
    candidate: &tcl_compiler::mixed_region_plan::GuardedCandidateEvidence,
) -> Value {
    match candidate.decision() {
        GuardedCandidateDecision::Selected(evidence) => json!({
            "kind": candidate.kind().as_str(),
            "decision": "selected",
            "operation": serialise_semantic_operation(Some(evidence.operation())),
            "runtimeVersion": evidence.runtime_version().version_string(),
            "dispatchDependencies": evidence
                .dispatch_dependencies()
                .iter()
                .map(tcl_registry::DispatchDependencyDomain::as_str)
                .collect::<Vec<_>>(),
        }),
        GuardedCandidateDecision::Declined(decline) => {
            let detail = match decline {
                GuardedCandidateDecline::OperationIsNotIntrinsic { operation } => json!({
                    "operation": serialise_semantic_operation(Some(*operation)),
                }),
                GuardedCandidateDecline::DispatchStabilityUnavailable { dependencies } => json!({
                    "dispatchDependencies": dependencies
                        .iter()
                        .map(tcl_registry::DispatchDependencyDomain::as_str)
                        .collect::<Vec<_>>(),
                }),
                GuardedCandidateDecline::UnsupportedIntrinsic { intrinsic } => json!({
                    "operation": serialise_semantic_operation(Some(
                        tcl_registry::SemanticOperationId::Intrinsic(*intrinsic),
                    )),
                }),
                GuardedCandidateDecline::ProofObligations(failures) => json!({
                    "proofObligations": failures
                        .iter()
                        .copied()
                        .map(tcl_compiler::backend_registry::SelectionProofFailure::as_str)
                        .collect::<Vec<_>>(),
                }),
                GuardedCandidateDecline::InvocationUnresolved
                | GuardedCandidateDecline::DirectCalleeIdentityUnavailable
                | GuardedCandidateDecline::PassDisabled
                | GuardedCandidateDecline::CompletionContractUnsupported
                | GuardedCandidateDecline::RuntimeSemanticsUnavailable
                | GuardedCandidateDecline::GuardPlanInvalid(_) => json!({}),
            };
            json!({
                "kind": candidate.kind().as_str(),
                "decision": "declined",
                "reason": decline.as_str(),
                "detail": detail,
            })
        }
    }
}

fn serialise_semantic_operation(operation: Option<tcl_registry::SemanticOperationId>) -> Value {
    operation.map_or(Value::Null, |operation| {
        json!({
            "kind": operation.kind_str(),
            "id": operation.detail_str(),
        })
    })
}

fn completion_identity(completion: tcl_compiler::executable_ir::CompletionId) -> Value {
    json!({
        "kind": "tcl-completion",
        "function": completion.function().index(),
        "index": completion.index(),
    })
}

fn argv_identity(argv: tcl_compiler::executable_ir::ExecutableArgvId) -> Value {
    json!({
        "kind": "prebuilt-argv",
        "function": argv.function().index(),
        "index": argv.index(),
    })
}

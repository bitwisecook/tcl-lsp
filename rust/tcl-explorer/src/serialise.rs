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

//! JSON serialisation of an [`ExplorerResult`] into the explorer contract
//! shape (`docs/design/contracts/wasm-explorer-view.md` for the `wasm`
//! slice; the rest is the de-facto contract `explorer-core.js` reads).
//!
//! `serialise_result` assembles the top-level object from the per-view
//! `serialise_*` helpers.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use tcl_compiler::cfg::{Function, Terminator};
use tcl_compiler::cfg_layout::{build_cfg_edges, ordered_block_names};
use tcl_compiler::dataflow_graph::extract_function_dataflow;
use tcl_compiler::effect_ssa::WorldStateIntentKind;
use tcl_compiler::executable_ir::{
    ExecutableInstruction, ExecutableTerminator, InvocationResolution,
    OwnedInvocationResolutionUnresolved,
};
use tcl_compiler::gvn::{
    find_loop_invariants_for_cu, find_partial_redundancies_for_cu, find_redundancies_for_cu,
};
use tcl_compiler::interprocedural::InterproceduralAnalysis;
use tcl_compiler::interval_bounds::{find_divide_by_zero_with, find_interval_bounds_with};
use tcl_compiler::intervals::compute_intervals;
use tcl_compiler::ir::{Module, Script, Statement};
use tcl_compiler::irules_checks::{
    find_collect_flow_warnings, find_hoistable_set_warnings, find_http_flow_warnings,
    find_unguarded_drop_warnings, find_unnormalised_getter_warnings,
};
use tcl_compiler::loops::{build_loop_forest, dominates};
use tcl_compiler::memory_ssa::{MemoryLocationKind, MemoryOpKind, MemorySsaFunction};
use tcl_compiler::mixed_region_plan::{
    GuardedCandidateDecision, GuardedCandidateDecline, RegionPlan,
};
use tcl_compiler::optimiser::{apply_optimisations, find_dead_stores, optimise, optimise_by_pass};
use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_compiler::semantic_analysis::{
    ExecutableAnalysisAvailability, MixedRegionPlanAvailability,
};
use tcl_compiler::semantic_optimisation::{SemanticOptimisationConfig, SemanticOptimisationPassId};
use tcl_compiler::shimmer::{
    find_byte_array_warnings_for_cu, find_sharing_warnings_for_cu, find_shimmer_warnings_for_cu,
    find_thunking_warnings_for_cu, type_name,
};
use tcl_compiler::state_ssa::adapters::{
    WorldInterpreterScope, WorldNamespaceScope, WorldRegion, WorldRegionKind, WorldSubjectScope,
};
use tcl_compiler::state_ssa::{CfgStatePosition, StateOp, StateSite};
use tcl_compiler::taint::find_taint_warnings_for_cu;
use tcl_compiler::world_state_ssa::{WorldStateSsaDecline, project_transition_facts};
use tcl_lexer::{LexerConfig, LineIndex, Span, TokenType};
use tcl_registry::available_dialects;
// See the note in `lib.rs`: the explorer resolves against the active pack set.
use tcl_spectcl::bundled::active_registry_for_dialect as registry_for_dialect;
use tcl_syntax::expr::ast::render_expr;

use crate::ExplorerResult;
use crate::formatters::{
    format_lattice, format_return_shape, format_taint, format_type, preview, range_dict,
    stmt_color_class, stmt_kind, stmt_summary, type_kind_name,
};
use crate::views::{Severity, VIEW_META};

/// Serialise the `meta` view: dialect list, view-tab table, and the
/// severity vocabulary.
///
/// Dialects carry their catalog labels (`display_name` for menus,
/// `short_name` for toolbars) exactly like the `views` entries carry
/// theirs, so no GUI consumer needs its own name table. A name without a
/// catalog profile repeats itself as both labels.
#[must_use]
pub fn serialise_meta() -> Value {
    let dialects: Vec<Value> = available_dialects()
        .iter()
        .map(|d| {
            let profile = crate::environment::catalogue_profile_for_dialect(d);
            json!({
                "name": *d,
                "displayName": profile.map_or(*d, |p| p.display_name),
                "shortName": profile.map_or(*d, |p| p.short_name),
            })
        })
        .collect();
    let views: Vec<Value> = VIEW_META
        .iter()
        .map(|view| {
            json!({
                "id": view.id,
                "label": view.label,
                "payload": view.payload,
                "group": view.group,
                "renderKind": view.render_kind.as_str(),
            })
        })
        .collect();
    let severities: Vec<Value> = Severity::ALL
        .iter()
        .map(|s| Value::String(s.as_str().to_owned()))
        .collect();
    // Registry-owned presentation metadata. Compiler Explorer can annotate a
    // trait wherever a pipeline view exposes it without maintaining another
    // name, description, or grouping table in JavaScript.
    let traits: Vec<Value> = tcl_registry::traits::Trait::ALL
        .iter()
        .map(|item| {
            json!({
                "name": item.name(),
                "summary": item.summary(),
                "group": item.category().label(),
            })
        })
        .collect();
    json!({
        "dialects": dialects,
        "views": views,
        "severities": severities,
        "traits": traits,
        // The codegen-pass catalogue, so a front end can render its toggles
        // before the first compile — the same reason `dialects` is here
        // (issue #1183). The per-result `semanticOptimisations` view carries
        // the same rows plus the state the shown module was built with.
        "semanticOptimisations": serialise_semantic_optimisations(
            SemanticOptimisationConfig::new(),
        ),
    })
}

/// `range_dict` for an optional span, emitting `null` when absent
fn range_or_null(span: Option<Span>, li: &LineIndex, source: &str) -> Value {
    span.map_or(Value::Null, |s| range_dict(s, li, source))
}

/// Serialise an IR script (a list of statement nodes). Children are emitted
/// only for If/For/Switch.
fn serialise_script(script: &Script, li: &LineIndex, source: &str) -> Value {
    let nodes: Vec<Value> = script
        .statements
        .iter()
        .map(|stmt| {
            let mut node = json!({
                "kind": stmt_kind(stmt),
                "summary": stmt_summary(stmt),
                "colorClass": stmt_color_class(stmt),
                "range": range_dict(stmt.span(), li, source),
            });
            if let Some(children) = serialise_children(stmt, li, source) {
                node["children"] = Value::Array(children);
            }
            node
        })
        .collect();
    Value::Array(nodes)
}

/// The `children` array for the structured statements the IR view expands
/// (If/For/Switch); `None` for every other statement kind.
fn serialise_children(stmt: &Statement, li: &LineIndex, source: &str) -> Option<Vec<Value>> {
    match stmt {
        Statement::If {
            clauses,
            else_body,
            else_span,
            ..
        } => {
            let mut children: Vec<Value> = clauses
                .iter()
                .enumerate()
                .map(|(i, clause)| {
                    json!({
                        "label": format!("clause {}: {}", i + 1, preview(&render_expr(&clause.condition), 60)),
                        "range": range_dict(clause.condition_span, li, source),
                        "body": serialise_script(&clause.body, li, source),
                    })
                })
                .collect();
            if let Some(body) = else_body {
                children.push(json!({
                    "label": "else",
                    "range": range_or_null(*else_span, li, source),
                    "body": serialise_script(body, li, source),
                }));
            }
            Some(children)
        }
        Statement::For {
            init,
            init_span,
            condition,
            condition_span,
            next,
            next_span,
            body,
            body_span,
            ..
        } => Some(vec![
            json!({ "label": "init", "range": range_dict(*init_span, li, source), "body": serialise_script(init, li, source) }),
            json!({
                "label": format!("condition: {}", preview(&render_expr(condition), 60)),
                "range": range_dict(*condition_span, li, source),
                "body": [],
            }),
            json!({ "label": "next", "range": range_dict(*next_span, li, source), "body": serialise_script(next, li, source) }),
            json!({ "label": "body", "range": range_dict(*body_span, li, source), "body": serialise_script(body, li, source) }),
        ]),
        Statement::Switch {
            arms,
            default_body,
            default_span,
            ..
        } => {
            let mut children: Vec<Value> = arms
                .iter()
                .map(|arm| {
                    let kind = if arm.fallthrough {
                        "fallthrough"
                    } else {
                        "arm"
                    };
                    let body = arm
                        .body
                        .as_ref()
                        .map_or_else(|| Value::Array(vec![]), |b| serialise_script(b, li, source));
                    json!({
                        "label": format!("{kind}: {}", preview(&arm.pattern, 48)),
                        "range": range_dict(arm.pattern_span, li, source),
                        "body": body,
                    })
                })
                .collect();
            if let Some(body) = default_body {
                children.push(json!({
                    "label": "default",
                    "range": range_or_null(*default_span, li, source),
                    "body": serialise_script(body, li, source),
                }));
            }
            Some(children)
        }
        _ => None,
    }
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
fn serialise_wasm_with_options(
    result: &ExplorerResult,
    options: tcl_compiler::codegen::wasm::WasmCompileOptions,
) -> Value {
    use tcl_compiler::codegen::wasm::{WasmCodegenPlan, WasmSemanticDecline, compile_wasm};

    let registry_held = registry_for_dialect(&result.dialect);
    let registry = &*registry_held;
    let mut wasm = compile_wasm(&result.unit, registry, options);
    let wat = wasm.to_wat();

    // Rich per-instruction entries (module header first, then functions).
    let li = LineIndex::new(&result.source);
    let mut entries = crate::wasm_explorer::wasm_to_explorer_json(&wasm, &li, &result.source);

    // Augment the synthetic `(module)` header with the WAT text the TUI
    // renderer reads (it reads `text` on the module entry and
    // `name`/`instrCount` on each function entry, both already present).
    if let Some(Value::Object(header)) = entries.first_mut() {
        header.insert("text".to_owned(), Value::String(wat));
        let (region_status, regions, region_decline) =
            serialise_wasm_region_plan(wasm.plan.region_plan());
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
            }),
            WasmCodegenPlan::GenericInvoke { .. } => json!({
                "kind": wasm.plan.as_str(),
                "operation": wasm.plan.operation_kind(),
                "semanticDecline": Value::Null,
                "regionPlanStatus": region_status,
                "regionPlanDecline": region_decline,
                "regions": regions,
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
                    WasmSemanticDecline::SemanticPlansDisabled
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
                })
            }
        };
        let mut plan = plan;
        if let Value::Object(map) = &mut plan {
            map.insert(
                "nativeLowering".to_owned(),
                serialise_native_tier(&wasm.native),
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
fn serialise_native_tier(report: &tcl_compiler::native_lowering::NativeTierReport) -> Value {
    use tcl_compiler::native_lowering::{FunctionStatus, StatementOutcome};
    let functions: serde_json::Map<String, Value> = report
        .functions
        .iter()
        .map(|(name, function)| {
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
                name.clone(),
                json!({
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

/// Serialise the `ir` view.:
/// `{ topLevel: [...], procedures: { qname: { params, range, body } } }`.
#[must_use]
pub fn serialise_ir(module: &Module, li: &LineIndex, source: &str) -> Value {
    let mut procedures = Map::new();
    let mut qnames: Vec<&String> = module.procedures.keys().collect();
    qnames.sort();
    for qname in qnames {
        let proc = &module.procedures[qname];
        procedures.insert(
            qname.clone(),
            json!({
                "params": proc.params,
                "range": range_dict(proc.span, li, source),
                "body": serialise_script(&proc.body, li, source),
            }),
        );
    }
    json!({
        "topLevel": serialise_script(&module.top_level, li, source),
        "procedures": Value::Object(procedures),
    })
}

/// Serialise a block terminator.
fn terminator_dict(
    func: &Function,
    term: Option<&Terminator>,
    li: &LineIndex,
    source: &str,
) -> Value {
    match term {
        None => Value::Null,
        Some(Terminator::Goto { target, span }) => json!({
            "type": "goto",
            "target": func.block_name(*target),
            "range": range_or_null(*span, li, source),
        }),
        Some(Terminator::Branch {
            condition,
            true_target,
            false_target,
            span,
            ..
        }) => json!({
            "type": "branch",
            "condition": preview(&render_expr(condition), 80),
            "trueTarget": func.block_name(*true_target),
            "falseTarget": func.block_name(*false_target),
            "range": range_or_null(*span, li, source),
        }),
        Some(Terminator::Return { value, span, .. }) => json!({
            "type": "return",
            "value": value.as_ref().map(|v| preview(v, 60)),
            "range": range_or_null(*span, li, source),
        }),
    }
}

/// The successor block names of a terminator (terminator-only, no
/// exception edges).
fn block_successors<'a>(func: &'a Function, term: Option<&Terminator>) -> Vec<&'a str> {
    term.map(Terminator::successors)
        .unwrap_or_default()
        .into_iter()
        .map(|id| func.block_name(id))
        .collect()
}

/// Serialise the routed control-flow edges of `func`. Lanes come from the
/// shared `cfg_layout`.
fn serialise_cfg_edges(func: &Function, order: &[String]) -> Value {
    let edges: Vec<Value> = build_cfg_edges(func, order)
        .into_iter()
        .map(|e| {
            json!({
                "from": e.src,
                "to": e.dst,
                "fromPos": e.src_pos,
                "toPos": e.dst_pos,
                "kind": e.kind.as_str(),
                "lane": e.lane,
            })
        })
        .collect();
    Value::Array(edges)
}

/// Serialise the pre-SSA CFG view.:
/// one entry per function, blocks in creation order, with routed edges.
#[must_use]
pub fn serialise_cfg_pre_ssa(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let funcs: Vec<Value> = result
        .all_snapshots()
        .iter()
        .map(|snap| {
            let cfg = &snap.unit.cfg;
            let order = ordered_block_names(cfg);
            let entry_name = cfg.block_name(cfg.entry);
            let blocks: Vec<Value> = order
                .iter()
                .map(|bn| {
                    let block = cfg.block_by_name(bn).expect("ordered block exists");
                    let statements: Vec<Value> = block
                        .statements
                        .iter()
                        .map(|stmt| {
                            json!({
                                "summary": stmt_summary(stmt),
                                "colorClass": stmt_color_class(stmt),
                                "range": range_dict(stmt.span(), li, source),
                            })
                        })
                        .collect();
                    json!({
                        "name": bn,
                        "isEntry": bn == entry_name,
                        "statements": statements,
                        "terminator": terminator_dict(cfg, block.terminator.as_ref(), li, source),
                        "successors": block_successors(cfg, block.terminator.as_ref()),
                    })
                })
                .collect();
            json!({
                "name": snap.name,
                "kind": snap.kind.as_str(),
                "entry": entry_name,
                "blockCount": cfg.blocks.len(),
                "blocks": blocks,
                "edges": serialise_cfg_edges(cfg, &order),
            })
        })
        .collect();
    Value::Array(funcs)
}

/// Format a declared arity's arity string:
/// `"{min}+"` when unlimited, else `"{min}..{max}"`.
fn arity_str(arity: tcl_registry::Arity) -> String {
    if arity.is_unlimited() {
        format!("{}+", arity.min)
    } else {
        format!("{}..{}", arity.min, arity.max)
    }
}

/// Serialise the `renderedProperties` view: each SSA value's `may` / `must`
/// rendered-property flag names, per function. Values with no flags are
/// skipped, and functions with no entries are omitted. `iter_names()` yields
/// the set named flags in declaration order (NONE excluded).
#[must_use]
pub fn serialise_rendered_properties(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .all_snapshots()
        .iter()
        .filter_map(|snap| {
            let ssa = &snap.unit.ssa;
            let mut keys: Vec<_> = snap.unit.rendered_props.keys().collect();
            keys.sort_by(|a, b| ssa.var_name(a.0).cmp(ssa.var_name(b.0)).then(a.1.cmp(&b.1)));
            let entries: Vec<Value> = keys
                .iter()
                .filter_map(|key| {
                    let rp = &snap.unit.rendered_props[*key];
                    let may: Vec<&str> = rp.may.iter_names().map(|(n, _)| n).collect();
                    let must: Vec<&str> = rp.must.iter_names().map(|(n, _)| n).collect();
                    if may.is_empty() && must.is_empty() {
                        return None;
                    }
                    Some(json!({ "variable": ssa.var_name(key.0), "version": key.1, "may": may, "must": must }))
                })
                .collect();
            (!entries.is_empty()).then(|| json!({ "name": snap.name, "kind": snap.kind.as_str(), "entries": entries }))
        })
        .collect();
    Value::Array(funcs)
}

/// Renderer severity for a shimmer/thunking code. `S102` is the sole
/// danger-level code (rendered as `error`); all others are `warning`.
fn shimmer_severity(code: &str) -> &'static str {
    if code == "S102" { "error" } else { "warning" }
}

/// Serialise the `shimmer` view: intrep-shimmer (S100/S101), loop-thunking
/// (S102), and shared-value copy-on-write (S103) warnings, combining
/// the warning kinds into one list. This view is strictly gated by the
/// differential harness.
#[must_use]
pub fn serialise_shimmer(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let registry_held = registry_for_dialect(&result.dialect);
    let registry = &*registry_held;
    let mut out: Vec<Value> = Vec::new();
    for w in find_shimmer_warnings_for_cu(&result.unit, registry) {
        out.push(json!({
            "code": w.code.as_str(),
            "message": w.message,
            "range": range_dict(w.span, li, source),
            "severity": shimmer_severity(w.code.as_str()),
            "variable": w.variable,
            "fromType": type_name(w.from_type),
            "toType": type_name(w.to_type),
            "command": w.command,
            "inLoop": w.in_loop,
        }));
    }
    for w in find_thunking_warnings_for_cu(&result.unit, registry) {
        out.push(json!({
            "code": w.code.as_str(),
            "message": w.message,
            "range": range_dict(w.span, li, source),
            "severity": shimmer_severity(w.code.as_str()),
            "variable": w.variable,
            "typeA": type_name(w.type_a),
            "typeB": type_name(w.type_b),
        }));
    }
    // Shared-value copy-on-write (S103) — a mutation that duplicates the
    // whole value because another live variable still holds it.
    for w in find_sharing_warnings_for_cu(&result.unit, registry) {
        out.push(json!({
            "code": w.code.as_str(),
            "message": w.message,
            "range": range_dict(w.span, li, source),
            "severity": shimmer_severity(w.code.as_str()),
            "variable": w.variable,
            "sharedWith": w.shared_with,
            "command": w.command,
        }));
    }
    // Byte-array corruption (S110) — a correctness shimmer with the same
    // value-shape shape as the S100/S101 family.
    for w in find_byte_array_warnings_for_cu(&result.unit, registry) {
        out.push(json!({
            "code": w.code.as_str(),
            "message": w.message,
            "range": range_dict(w.span, li, source),
            "severity": shimmer_severity(w.code.as_str()),
            "variable": w.variable,
            "fromType": type_name(w.from_type),
            "toType": type_name(w.to_type),
            "command": w.command,
            "inLoop": w.in_loop,
        }));
    }
    Value::Array(out)
}

/// Build the optimised-source explorer result, or `None` when the
/// optimiser leaves the source unchanged. Shared by the `optimisedSource`,
/// `irOptimised`, and `cfgPreSsaOptimised` keys (the "double pipeline" —
/// re-run the whole pipeline on the rewritten source). Returns the result
/// plus the optimised source string (whose ranges the optimised views
/// index into).
fn optimised_result(result: &ExplorerResult) -> Option<(crate::ExplorerResult, String)> {
    let registry_held = registry_for_dialect(&result.dialect);
    let registry = &*registry_held;
    let optimised = apply_optimisations(&result.source, &optimise(&result.source, registry));
    if optimised == result.source {
        return None;
    }
    let unit = crate::run_pipeline(&optimised, &result.dialect);
    Some((unit, optimised))
}

/// The stable name of a tracked mutable-world domain.
fn world_domain_name(kind: WorldRegionKind) -> &'static str {
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
        tcl_registry::StateTransition::Widen(_) => "widen",
    }
}

fn registry_world_domain_name(domain: tcl_registry::WorldStateDomain) -> String {
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
                    };
                    json!({
                        "resolution": "unresolved",
                        "node": invoke.node.path(),
                        "completion": invoke.completion.index(),
                        "proof": { "abstention": abstention },
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
fn serialise_world_ssa(result: &ExplorerResult) -> Value {
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

/// Serialise the `gvn` view: redundant-computation hints (GVN/CSE + PRE +
/// LICM). — `{code, message, expression, range,
/// firstRange, severity: info}`. Composes the three `*_for_cu`
/// finders and de-duplicates on `(code, span, first_span)`. Optimiser-
/// derived, pinned by a Rust unit test.
#[must_use]
pub fn serialise_gvn(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let registry_held = registry_for_dialect(&result.dialect);
    let registry = &*registry_held;
    let dialect = crate::environment::known_profile_for_dialect(&result.dialect);
    let mut all = find_redundancies_for_cu(&result.unit, registry, dialect);
    all.extend(find_partial_redundancies_for_cu(
        &result.unit,
        registry,
        dialect,
    ));
    all.extend(find_loop_invariants_for_cu(&result.unit, registry, dialect));

    let mut seen = std::collections::HashSet::new();
    let out: Vec<Value> = all
        .iter()
        .filter(|w| {
            seen.insert((
                w.code,
                w.span.start(),
                w.span.end(),
                w.first_span.start(),
                w.first_span.end(),
            ))
        })
        .map(|w| {
            json!({
                "code": w.code.as_str(),
                "message": w.message,
                "expression": w.expression_text,
                "range": range_dict(w.span, li, source),
                "firstRange": range_dict(w.first_span, li, source),
                "severity": "info",
            })
        })
        .collect();
    Value::Array(out)
}

/// Renderer severity for a taint code.
/// (`T1*` prefix or `T3001`-`T3004` → error, else warning).
fn taint_severity(code: &str) -> &'static str {
    if code.starts_with("T1") || matches!(code, "T3001" | "T3002" | "T3003" | "T3004") {
        "error"
    } else {
        "warning"
    }
}

/// Serialise the `taint` view: information-flow sink warnings. Composes the
/// taint passes via `find_taint_warnings_for_cu`.
#[must_use]
pub fn serialise_taint(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let registry_held = registry_for_dialect(&result.dialect);
    let registry = &*registry_held;
    let dialect = crate::environment::known_profile_for_dialect(&result.dialect);
    let out: Vec<Value> = find_taint_warnings_for_cu(&result.unit, registry, dialect)
        .iter()
        .map(|w| {
            json!({
                "code": w.code.as_str(),
                "message": w.message,
                "range": range_dict(w.span, li, source),
                "severity": taint_severity(w.code.as_str()),
                "variable": w.variable,
                "sinkCommand": w.sink_command,
            })
        })
        .collect();
    Value::Array(out)
}

/// Serialise the `optimisations` view: the optimiser rewrites found for
/// the source. — `{code, message,
/// range, replacement}` per rewrite. Runs the `optimise` pass over
/// the cached per-dialect registry.
#[must_use]
pub fn serialise_optimisations(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let registry_held = registry_for_dialect(&result.dialect);
    let registry = &*registry_held;
    let opts: Vec<Value> = optimise(source, registry)
        .iter()
        .map(|o| {
            json!({
                "code": o.code.as_str(),
                "message": o.message,
                "range": range_dict(o.span, li, source),
                "replacement": o.replacement,
            })
        })
        .collect();
    Value::Array(opts)
}

/// Serialise the Rust-native `optimiserPasses` view: each optimiser pass in
/// `PassId::all()` order with the optimisations it produced (raw, before the
/// overlap arbitration the `optimisations` view applies).
///
/// **Rust-native** — surfaces the actual pipeline: which pass found what, in
/// execution order.
#[must_use]
pub fn serialise_optimiser_passes(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let registry_held = registry_for_dialect(&result.dialect);
    let registry = &*registry_held;
    let passes: Vec<Value> = optimise_by_pass(
        &result.unit,
        registry,
        crate::environment::known_profile_for_dialect(&result.dialect),
    )
    .iter()
    .map(|(pass, opts)| {
        let optimisations: Vec<Value> = opts
            .iter()
            .map(|o| {
                json!({
                    "code": o.code.as_str(),
                    "message": o.message,
                    "range": range_dict(o.span, li, source),
                    "replacement": o.replacement,
                })
            })
            .collect();
        json!({
            "id": pass.as_str(),
            "label": pass.label(),
            "count": optimisations.len(),
            "optimisations": optimisations,
        })
    })
    .collect();
    Value::Array(passes)
}

/// Serialise the `intervals` view: the integer-interval domain per tracked
/// SSA value, per function. — only bounded
/// (non-top) ranges are emitted; `lo`/`hi` are `null` for ±infinity.
#[must_use]
pub fn serialise_intervals(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .all_snapshots()
        .iter()
        .map(|snap| {
            let ssa = &snap.unit.ssa;
            let intervals = compute_intervals(&snap.unit.cfg, ssa, &snap.unit.sccp.values);
            let mut keys: Vec<_> = intervals.keys().collect();
            keys.sort_by(|a, b| ssa.var_name(a.0).cmp(ssa.var_name(b.0)).then(a.1.cmp(&b.1)));
            let entries: Vec<Value> = keys
                .iter()
                .filter_map(|key| {
                    let iv = intervals[*key];
                    if iv.is_top() || (iv.lo.is_none() && iv.hi.is_none()) {
                        return None;
                    }
                    Some(json!({ "variable": ssa.var_name(key.0), "version": key.1, "lo": iv.lo, "hi": iv.hi }))
                })
                .collect();
            json!({ "name": snap.name, "kind": snap.kind.as_str(), "entries": entries })
        })
        .collect();
    Value::Array(funcs)
}

/// Serialise the `taintTracking` view: the complete taint lattice for every
/// tracked SSA value, per function. Warning rendering remains separate in
/// `taintWarnings`; this payload is the durable analysis state.
#[must_use]
pub fn serialise_taint_tracking(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .all_snapshots()
        .iter()
        .filter_map(|snap| {
            let ssa = &snap.unit.ssa;
            let mut keys: Vec<_> = snap.unit.taints.keys().collect();
            keys.sort_by(|a, b| ssa.var_name(a.0).cmp(ssa.var_name(b.0)).then(a.1.cmp(&b.1)));
            let entries: Vec<Value> = keys
                .iter()
                .map(|key| {
                    let tl = &snap.unit.taints[*key];
                    json!({ "variable": ssa.var_name(key.0), "version": key.1, "taint": format_taint(tl) })
                })
                .collect();
            (!entries.is_empty()).then(|| json!({ "name": snap.name, "entries": entries }))
        })
        .collect();
    Value::Array(funcs)
}

/// Per-SSA-value `{version, lattice?, type?}` detail used by the post-SSA
/// CFG view's `uses`/`defs` maps.
fn ssa_value_detail(
    refs: &std::collections::HashMap<tcl_compiler::ssa::Symbol, tcl_compiler::ssa::Version>,
    sccp: &tcl_compiler::sccp::SccpResult,
    types: &std::collections::HashMap<
        tcl_compiler::ssa::ValueKey,
        tcl_compiler::types::TypeLattice,
    >,
    ssa: &tcl_compiler::ssa::SsaFunction,
) -> Value {
    let mut keys: Vec<(&tcl_compiler::ssa::Symbol, &u32)> = refs.iter().collect();
    keys.sort_by(|a, b| {
        ssa.var_name(*a.0)
            .cmp(ssa.var_name(*b.0))
            .then(a.1.cmp(b.1))
    });
    let mut out = Map::new();
    for (&sym, &ver) in keys {
        let mut d = Map::new();
        d.insert("version".to_owned(), json!(ver));
        if let Some(lat) = sccp.values.get(&(sym, ver)) {
            d.insert("lattice".to_owned(), json!(format_lattice(lat)));
        }
        if let Some(tl) = types.get(&(sym, ver))
            && tl.kind() != tcl_compiler::types::TypeKind::Unknown
        {
            d.insert("type".to_owned(), json!(format_type(tl)));
        }
        out.insert(ssa.var_name(sym).to_owned(), Value::Object(d));
    }
    Value::Object(out)
}

/// Serialise the post-SSA CFG view (`cfgPostSsa`): per-block phi nodes and
/// per-statement SSA `uses`/`defs` with lattice + type detail, plus a
/// function-level `analysis` block.
///
/// `analysis.deadStores` is the per-function liveness-based set
/// ([`tcl_compiler::dead_stores::liveness_dead_stores`]). The lattice/type
/// detail comes from the Rust
/// analyses; `constantBranches`/`unreachableBlocks`/`inferredTypes` come from
/// SCCP + the type lattice.
#[must_use]
pub fn serialise_cfg_post_ssa(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let registry_held = registry_for_dialect(&result.dialect);
    let registry = &*registry_held;
    let funcs: Vec<Value> = result
        .all_snapshots()
        .iter()
        .map(|snap| {
            let cfg = &snap.unit.cfg;
            let ssa = &snap.unit.ssa;
            let sccp = &snap.unit.sccp;
            let types = &snap.unit.types;
            let order = ordered_block_names(cfg);
            let entry_name = cfg.block_name(cfg.entry);

            let blocks: Vec<Value> = order
                .iter()
                .map(|bn| {
                    let block = cfg.block_by_name(bn).expect("ordered block exists");
                    let bid = cfg.block_id(bn);
                    let ssa_block = bid.and_then(|id| ssa.blocks.get(&id));

                    let phis: Vec<Value> = ssa_block
                        .map(|sb| {
                            sb.phis
                                .iter()
                                .map(|phi| {
                                    // Key the incoming map by predecessor block
                                    // name; sort by name for stable output.
                                    let mut inc: Vec<(&str, u32)> = phi
                                        .incoming
                                        .iter()
                                        .map(|(pred, &ver)| (cfg.block_name(*pred), ver))
                                        .collect();
                                    inc.sort_unstable();
                                    let phi_name = ssa.var_name(phi.name);
                                    let incoming: Map<String, Value> = inc
                                        .into_iter()
                                        .map(|(pred, ver)| {
                                            (pred.to_owned(), json!(format!("{phi_name}#{ver}")))
                                        })
                                        .collect();
                                    let phi_type = types
                                        .get(&(phi.name, phi.version))
                                        .map_or(Value::Null, |tl| json!(format_type(tl)));
                                    json!({
                                        "name": phi_name,
                                        "version": phi.version,
                                        "incoming": incoming,
                                        "type": phi_type,
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    let statements: Vec<Value> = block
                        .statements
                        .iter()
                        .enumerate()
                        .map(|(idx, stmt)| {
                            let (uses, defs) =
                                ssa_block.and_then(|sb| sb.statements.get(idx)).map_or_else(
                                    || (json!({}), json!({})),
                                    |ss| {
                                        (
                                            ssa_value_detail(&ss.uses, sccp, types, ssa),
                                            ssa_value_detail(&ss.defs, sccp, types, ssa),
                                        )
                                    },
                                );
                            json!({
                                "summary": stmt_summary(stmt),
                                "colorClass": stmt_color_class(stmt),
                                "range": range_dict(stmt.span(), li, source),
                                "uses": uses,
                                "defs": defs,
                            })
                        })
                        .collect();

                    json!({
                        "name": bn,
                        "isEntry": bn == entry_name,
                        "isUnreachable": !bid.is_some_and(|id| sccp.executable_blocks.contains(&id)),
                        "phis": phis,
                        "statements": statements,
                        "terminator": terminator_dict(cfg, block.terminator.as_ref(), li, source),
                        "successors": block_successors(cfg, block.terminator.as_ref()),
                    })
                })
                .collect();

            json!({
                "name": snap.name,
                "kind": snap.kind.as_str(),
                "entry": entry_name,
                "blockCount": cfg.blocks.len(),
                "blocks": blocks,
                "edges": serialise_cfg_edges(cfg, &order),
                "analysis": post_ssa_analysis(snap, registry),
            })
        })
        .collect();
    Value::Array(funcs)
}

/// Serialise the durable SSA dominator artefacts: immediate dominators,
/// dominance frontiers, and the materialised dominator tree.  Unlike the
/// post-SSA CFG summary, this retains the complete graph used by phi
/// placement and loop analysis, including method and synthetic body units.
#[must_use]
pub fn serialise_dominators(result: &ExplorerResult) -> Value {
    Value::Array(
        result
            .all_snapshots()
            .iter()
            .map(|snap| {
                let ssa = &snap.unit.ssa;
                let mut blocks: Vec<_> = ssa.idom.keys().copied().collect();
                blocks.sort_unstable();
                let rows: Vec<Value> = blocks
                    .iter()
                    .map(|&block| {
                        let idom = ssa.idom.get(&block).and_then(|parent| {
                            parent.map(|parent| ssa.block_name(parent).to_owned())
                        });
                        let mut frontier: Vec<String> = ssa
                            .dominance_frontier
                            .get(&block)
                            .into_iter()
                            .flat_map(|ids| ids.iter().map(|id| ssa.block_name(*id).to_owned()))
                            .collect();
                        frontier.sort_unstable();
                        let mut children: Vec<String> = ssa
                            .dominator_tree
                            .get(&block)
                            .into_iter()
                            .flat_map(|ids| ids.iter().map(|id| ssa.block_name(*id).to_owned()))
                            .collect();
                        children.sort_unstable();
                        json!({
                            "block": ssa.block_name(block),
                            "idom": idom,
                            "frontier": frontier,
                            "children": children,
                        })
                    })
                    .collect();
                json!({ "name": snap.name, "kind": snap.kind.as_str(), "entry": ssa.block_name(ssa.entry), "blocks": rows })
            })
            .collect(),
    )
}

/// Serialise the complete SCCP lattice and executable CFG facts. The SSA CFG
/// tab intentionally keeps a compact annotation; this view is the durable
/// proof surface for constants, reachability, and executable edges.
#[must_use]
pub fn serialise_sccp(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    Value::Array(
        result
            .all_snapshots()
            .iter()
            .map(|snap| {
                let ssa = &snap.unit.ssa;
                let mut values: Vec<_> = snap.unit.sccp.values.iter().collect();
                values.sort_by(|(a, _), (b, _)| {
                    ssa.var_name(a.0).cmp(ssa.var_name(b.0)).then(a.1.cmp(&b.1))
                });
                let values: Vec<Value> = values
                    .into_iter()
                    .map(|(&(symbol, version), lattice)| {
                        json!({
                            "variable": ssa.var_name(symbol),
                            "version": version,
                            "lattice": format_lattice(lattice),
                        })
                    })
                    .collect();
                let mut executable_blocks: Vec<String> = snap
                    .unit
                    .sccp
                    .executable_blocks
                    .iter()
                    .map(|id| ssa.block_name(*id).to_owned())
                    .collect();
                executable_blocks.sort_unstable();
                let mut executable_edges: Vec<Value> = snap
                    .unit
                    .sccp
                    .executable_edges
                    .iter()
                    .map(|(from, to)| json!({ "from": ssa.block_name(*from), "to": ssa.block_name(*to) }))
                    .collect();
                executable_edges.sort_by_key(std::string::ToString::to_string);
                let branches: Vec<Value> = snap
                    .unit
                    .sccp
                    .constant_branches
                    .iter()
                    .map(|branch| json!({
                        "block": branch.block,
                        "condition": preview(&branch.condition, 80),
                        "value": branch.value,
                        "takenTarget": branch.taken_target,
                        "notTakenTarget": branch.not_taken_target,
                        "range": branch.span.map(|span| range_dict(span, li, source)),
                    }))
                    .collect();
                json!({
                    "name": snap.name,
                    "kind": snap.kind.as_str(),
                    "values": values,
                    "executableBlocks": executable_blocks,
                    "executableEdges": executable_edges,
                    "constantBranches": branches,
                })
            })
            .collect(),
    )
}

/// Serialise def-use/liveness evidence without re-deriving it in the
/// front-end. Every chain is retained, including dead definitions and their
/// exact use-site kinds; the O109 collector is included as the optimiser's
/// authoritative dead-store result.
#[must_use]
pub fn serialise_liveness(result: &ExplorerResult) -> Value {
    let registry_held = registry_for_dialect(&result.dialect);
    let registry = &*registry_held;
    Value::Array(
        result
            .all_snapshots()
            .iter()
            .map(|snap| {
                let mut chains: Vec<_> = snap.unit.def_use.chains.values().collect();
                chains.sort_by(|a, b| a.key.cmp(&b.key));
                let chains: Vec<Value> = chains
                    .iter()
                    .map(|chain| {
                        json!({
                            "variable": chain.key.0,
                            "version": chain.key.1,
                            "definition": {
                                "block": chain.definition.block,
                                "kind": def_kind_label(chain.definition.kind),
                                "statementIndex": chain.definition.statement_index,
                            },
                            "uses": chain.uses.iter().map(|use_site| json!({
                                "block": use_site.block,
                                "kind": use_kind_label(use_site.kind),
                                "statementIndex": use_site.statement_index,
                                "class": use_class_label(use_site.class),
                            })).collect::<Vec<_>>(),
                        })
                    })
                    .collect();
                let dead_stores: Vec<Value> =
                    tcl_compiler::dead_stores::liveness_dead_stores(snap.unit, registry)
                        .iter()
                        .map(|dead| {
                            json!({
                                "block": dead.block,
                                "statementIndex": dead.statement_index,
                                "variable": dead.variable,
                                "version": dead.version,
                            })
                        })
                        .collect();
                json!({ "name": snap.name, "kind": snap.kind.as_str(), "chains": chains, "deadStores": dead_stores })
            })
            .collect(),
    )
}

fn def_kind_label(kind: tcl_compiler::def_use::DefKind) -> &'static str {
    match kind {
        tcl_compiler::def_use::DefKind::Statement => "statement",
        tcl_compiler::def_use::DefKind::Phi => "phi",
        tcl_compiler::def_use::DefKind::Parameter => "parameter",
    }
}

fn use_kind_label(kind: tcl_compiler::def_use::UseKind) -> &'static str {
    match kind {
        tcl_compiler::def_use::UseKind::Operand => "operand",
        tcl_compiler::def_use::UseKind::PhiIncoming => "phi-incoming",
        tcl_compiler::def_use::UseKind::Terminator => "terminator",
    }
}

fn use_class_label(class: tcl_compiler::ssa::UseClass) -> &'static str {
    match class {
        tcl_compiler::ssa::UseClass::Substituted => "substituted",
        tcl_compiler::ssa::UseClass::Quoted => "quoted",
    }
}

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
    }
}

fn executable_terminator_kind(terminator: Option<&ExecutableTerminator>) -> &'static str {
    match terminator {
        Some(ExecutableTerminator::Goto(_)) => "goto",
        Some(ExecutableTerminator::Branch { .. }) => "branch",
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

fn invocation_resolution_label(
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
        // The re-keyed sidecar (ledger C1 / §11.2 D1) reaches this state by
        // carrying no resolved environment at all, so there is no mask left to
        // name in the payload.
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
                    "name": snap.name,
                    "kind": snap.kind.as_str(),
                    "status": semantic_status(availability),
                    "decline": semantic_decline_value(availability),
                    "complexityGuarded": snap.unit.complexity_guarded,
                    "dynamicNames": {
                        "writes": snap.unit.dynamic_names.writes,
                        "destroys": snap.unit.dynamic_names.destroys,
                        "reads": snap.unit.dynamic_names.reads,
                    },
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

/// The function-level `analysis` block of the post-SSA CFG view.
///
/// `deadStores` is the per-function liveness-based set
/// ([`tcl_compiler::dead_stores::liveness_dead_stores`]).
fn post_ssa_analysis(
    snap: &crate::FunctionSnapshot,
    registry: &tcl_registry::CommandRegistry,
) -> Value {
    let sccp = &snap.unit.sccp;
    let constant_branches: Vec<Value> = sccp
        .constant_branches
        .iter()
        .map(|b| {
            json!({
                "block": b.block,
                "condition": preview(&b.condition, 60),
                "value": b.value,
                "takenTarget": b.taken_target,
                "notTakenTarget": b.not_taken_target,
            })
        })
        .collect();

    let cfg = &snap.unit.cfg;
    let mut unreachable: Vec<&str> = cfg
        .blocks
        .keys()
        .filter(|b| !sccp.executable_blocks.contains(*b))
        .map(|id| cfg.block_name(*id))
        .collect();
    unreachable.sort_unstable();

    // `inferredTypes`: known/shimmered SSA-value types keyed by `name#ver`.
    let ssa = &snap.unit.ssa;
    let mut tkeys: Vec<_> = snap.unit.types.keys().collect();
    tkeys.sort_by(|a, b| ssa.var_name(a.0).cmp(ssa.var_name(b.0)).then(a.1.cmp(&b.1)));
    let mut inferred = Map::new();
    for key in tkeys {
        let tl = &snap.unit.types[key];
        if matches!(
            tl.kind(),
            tcl_compiler::types::TypeKind::Known | tcl_compiler::types::TypeKind::Shimmered
        ) {
            inferred.insert(
                format!("{}#{}", ssa.var_name(key.0), key.1),
                json!(format_type(tl)),
            );
        }
    }

    // Liveness-based dead stores for this function, already in deterministic
    // block/statement order.
    let dead_store_values: Vec<Value> =
        tcl_compiler::dead_stores::liveness_dead_stores(snap.unit, registry)
            .iter()
            .map(|d| {
                json!({
                    "block": d.block,
                    "stmtIndex": d.statement_index,
                    "variable": d.variable,
                    "version": d.version,
                })
            })
            .collect();

    json!({
        "constantBranches": constant_branches,
        "deadStores": dead_store_values,
        "unreachableBlocks": unreachable,
        "inferredTypes": Value::Object(inferred),
    })
}

/// Serialise the `dataflow` view: the def-use data-flow graph, built over
/// `extract_dataflow_graph`.
///
/// Each graph stays paired with its stable function-owner kind, and alias
/// information comes from the retained Memory SSA when available.
#[must_use]
pub fn serialise_dataflow(result: &ExplorerResult) -> Value {
    let snaps = result.all_snapshots();
    let mut total_defs = 0u32;
    let mut total_uses = 0u32;
    let mut total_aliases = 0u32;
    let functions: Vec<Value> = snaps
        .iter()
        .map(|snap| {
            // Extract each graph beside its owner identity. Building a module
            // graph would sort only by display name and could pair a Tcl proc
            // with a same-named TclOO method's Memory SSA.
            let f = extract_function_dataflow(
                snap.name,
                &snap.unit.ssa,
                &snap.unit.def_use,
                Some(&snap.unit.sccp),
                snap.unit.memory_ssa.as_ref(),
                Some(&snap.unit.types),
            );
            total_defs = total_defs.saturating_add(f.total_defs);
            total_uses = total_uses.saturating_add(f.total_uses);
            total_aliases =
                total_aliases.saturating_add(u32::try_from(f.aliases.len()).unwrap_or(u32::MAX));
            let memory = snap
                .unit
                .memory_ssa
                .as_ref()
                .map_or(Value::Null, serialise_memory_ssa);
            let nodes: Vec<Value> = f
                .nodes
                .iter()
                .map(|n| {
                    json!({
                        "name": n.name,
                        "version": n.version,
                        "block": n.block,
                        "defKind": n.def_kind,
                        "statementIndex": n.statement_index,
                        "lattice": n.lattice,
                        "typeInfo": n.type_info,
                        "isDead": n.is_dead,
                        "useCount": n.use_count,
                    })
                })
                .collect();
            let edges: Vec<Value> = f
                .edges
                .iter()
                .map(|e| {
                    json!({
                        "fromName": e.from_name,
                        "fromVersion": e.from_version,
                        "toBlock": e.to_block,
                        "toStatementIndex": e.to_statement_index,
                        "edgeKind": e.edge_kind.as_str(),
                        "toName": e.to_name,
                        "toVersion": e.to_version,
                    })
                })
                .collect();
            let aliases: Vec<Value> = f
                .aliases
                .iter()
                .map(|a| {
                    json!({
                        "localName": a.local_name,
                        "localKind": a.local_kind,
                        "targetName": a.target_name,
                        "targetKind": a.target_kind,
                        "reason": a.reason,
                    })
                })
                .collect();
            json!({
                "name": f.function_name,
                "kind": snap.kind.as_str(),
                "nodes": nodes,
                "edges": edges,
                "aliases": aliases,
                "memorySsa": memory,
                "summary": {
                    "totalDefs": f.total_defs,
                    "totalUses": f.total_uses,
                    "deadDefs": f.dead_defs,
                    "aliasedVars": f.aliased_vars,
                },
            })
        })
        .collect();

    json!({
        "functions": functions,
        "summary": {
            "totalDefs": total_defs,
            "totalUses": total_uses,
            "totalAliases": total_aliases,
            "functionCount": snaps.len(),
        },
    })
}

fn memory_location_kind_label(kind: MemoryLocationKind) -> &'static str {
    match kind {
        MemoryLocationKind::Local => "local",
        MemoryLocationKind::Upvar => "upvar",
        MemoryLocationKind::Global => "global",
        MemoryLocationKind::NamespaceVar => "namespace-var",
        MemoryLocationKind::ArrayElement => "array-element",
        MemoryLocationKind::InstanceVar => "instance-var",
        MemoryLocationKind::Unknown => "unknown",
    }
}

fn memory_op_kind_label(kind: MemoryOpKind) -> &'static str {
    match kind {
        MemoryOpKind::Def => "def",
        MemoryOpKind::Use => "use",
        MemoryOpKind::Phi => "phi",
        MemoryOpKind::Clobber => "clobber",
    }
}

fn serialise_memory_ssa(memory: &MemorySsaFunction) -> Value {
    let alias_sets: Vec<Value> = memory
        .alias_sets
        .iter()
        .map(|set| {
            let locations: Vec<Value> = set
                .locations
                .iter()
                .map(|location| {
                    json!({
                        "kind": memory_location_kind_label(location.kind),
                        "name": location.name,
                        "qualifier": location.qualifier,
                    })
                })
                .collect();
            json!({ "reason": set.reason, "locations": locations })
        })
        .collect();
    let ops: Vec<Value> = memory
        .memory_ops
        .iter()
        .map(|op| {
            json!({
                "kind": memory_op_kind_label(op.kind),
                "location": {
                    "kind": memory_location_kind_label(op.location.kind),
                    "name": op.location.name,
                    "qualifier": op.location.qualifier,
                },
                "version": op.version,
                "reachingVersion": op.reaching_version,
                "block": op.block,
                "statementIndex": op.statement_index,
            })
        })
        .collect();
    json!({
        "aliasSets": alias_sets,
        "operations": ops,
        "counts": {
            "defs": memory.count_defs,
            "uses": memory.count_uses,
            "clobbers": memory.count_clobbers,
        },
        "wildcardAliasing": memory.has_wildcard_aliasing,
    })
}

/// Serialise the `irulesFlow` view: iRules flow / performance warnings —
/// `{code, message, range, severity}`, composing the five `irules_checks`
/// finders. Empty for non-iRules dialects (an empty list on the tcl corpus).
#[must_use]
pub fn serialise_irules_flow(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let registry_held = registry_for_dialect(&result.dialect);
    let registry = &*registry_held;
    let dialect = crate::environment::known_profile_for_dialect(&result.dialect);
    let cu = &result.unit;
    let mut warnings = find_unnormalised_getter_warnings(
        cu,
        registry,
        dialect.map(tcl_dialect::DialectProfile::surface_query),
    );
    warnings.extend(find_unguarded_drop_warnings(
        cu,
        dialect.map(tcl_dialect::DialectProfile::surface_query),
    ));
    warnings.extend(find_collect_flow_warnings(
        cu,
        registry,
        dialect.map(tcl_dialect::DialectProfile::surface_query),
    ));
    warnings.extend(find_http_flow_warnings(
        cu,
        dialect.map(tcl_dialect::DialectProfile::surface_query),
    ));
    warnings.extend(find_hoistable_set_warnings(
        cu,
        dialect.map(tcl_dialect::DialectProfile::surface_query),
    ));

    let out: Vec<Value> = warnings
        .iter()
        .map(|w| {
            json!({
                "code": w.code.as_str(),
                "message": w.message,
                "range": range_dict(w.span, li, source),
                "severity": "warning",
            })
        })
        .collect();
    Value::Array(out)
}

/// Serialise the retained cross-event connection scope, including each event
/// summary and the conservative cross-event/racy sets used by diagnostics.
#[must_use]
pub fn serialise_connection_scope(result: &ExplorerResult) -> Value {
    let Some(scope) = result.unit.connection_scope.as_ref() else {
        return json!({ "available": false, "summaries": [] });
    };
    let mut events: Vec<_> = scope.summaries.values().collect();
    events.sort_by(|a, b| a.event.cmp(&b.event));
    let summaries: Vec<Value> = events
        .iter()
        .map(|summary| {
            let mut defs: Vec<_> = summary.defs.iter().cloned().collect();
            defs.sort_unstable();
            let mut uses: Vec<_> = summary.uses_before_def.iter().cloned().collect();
            uses.sort_unstable();
            let mut unsets: Vec<_> = summary.unsets.iter().cloned().collect();
            unsets.sort_unstable();
            json!({ "event": summary.event, "defs": defs, "usesBeforeDef": uses, "unsets": unsets })
        })
        .collect();
    let sorted = |set: &std::collections::HashSet<String>| {
        let mut values: Vec<_> = set.iter().cloned().collect();
        values.sort_unstable();
        values
    };
    json!({
        "available": true,
        "summaries": summaries,
        "crossEventDefs": sorted(&scope.cross_event_defs),
        "crossEventImports": sorted(&scope.cross_event_imports),
        "racyStaticDefs": sorted(&scope.racy_static_defs),
    })
}

/// Serialise the `loops` view: the natural-loop forest per function, with
/// nesting depth, built over
/// `build_loop_forest`. Depth = 1 + the number of *other* loop headers
/// that dominate this loop's header.
#[must_use]
pub fn serialise_loops(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .all_snapshots()
        .iter()
        .map(|snap| {
            let executable = &snap.unit.sccp.executable_blocks;
            let forest = build_loop_forest(&snap.unit.cfg, &snap.unit.ssa, executable);
            let headers = forest.headers();
            let cfg = &snap.unit.cfg;
            let loops: Vec<Value> = forest
                .loops
                .iter()
                .map(|lp| {
                    // Loop nesting depth: count header loops whose header
                    // dominates this loop's header. `NaturalLoop` headers are
                    // block names; resolve each to its id for `dominates`.
                    let node = cfg.block_id(&lp.header);
                    let depth = 1 + headers
                        .iter()
                        .filter(|h| **h != lp.header)
                        .filter_map(|h| Some((cfg.block_id(h)?, node?)))
                        .filter(|&(dom, node)| dominates(&snap.unit.ssa, dom, node))
                        .count();
                    json!({
                        "header": lp.header,
                        "depth": depth,
                        "blockCount": lp.blocks.len(),
                        "blocks": lp.blocks,
                        "latches": lp.latches,
                    })
                })
                .collect();
            json!({ "name": snap.name, "kind": snap.kind.as_str(), "loops": loops })
        })
        .collect();
    Value::Array(funcs)
}

/// Serialise the `bounds` view: interval-driven out-of-range findings per
/// function.
///
/// Both interval-driven passes share the same SCCP-executable-block filter:
/// `find_interval_bounds_with` (W230/W231/W232 out-of-range index access) and
/// `find_divide_by_zero_with` (W233 provably-`[0,0]` divisor).
#[must_use]
pub fn serialise_bounds(result: &ExplorerResult) -> Value {
    // The document's own grammar and numerals: the bounds view re-lexes
    // list and index text and must read it as the unit did.
    let profile = crate::environment::profile_for_dialect(&result.dialect);
    let numbers = tcl_dialect::NumberSyntax::of_profile(Some(profile));
    let grammar = profile.grammar;
    let funcs: Vec<Value> = result
        .all_snapshots()
        .iter()
        .map(|snap| {
            let findings: Vec<Value> = find_interval_bounds_with(
                &snap.unit.cfg,
                &snap.unit.ssa,
                &snap.unit.sccp.values,
                &snap.unit.sccp.executable_blocks,
                snap.unit
                    .semantic_facts
                    .context()
                    .map(tcl_registry::model::semantic::SemanticContext::environment_id)
                    .and_then(crate::environment::catalogue_profile_for_dialect)
                    .and_then(tcl_dialect::DialectProfile::character_model),
             numbers, grammar,
            )
            .iter()
            .map(|f| {
                json!({
                    "code": f.code.as_str(),
                    "command": f.command,
                    "indexVar": f.index_var,
                    "lo": f.index_interval.lo,
                    "hi": f.index_interval.hi,
                    "length": f.length,
                    "reason": f.reason,
                })
            })
            .collect();
            let divzero: Vec<Value> = find_divide_by_zero_with(
                &snap.unit.cfg,
                &snap.unit.ssa,
                &snap.unit.sccp.values,
                &snap.unit.sccp.executable_blocks,
             numbers, grammar,
            )
            .iter()
            .map(|d| json!({ "code": "W233", "op": d.op }))
            .collect();
            json!({ "name": snap.name, "kind": snap.kind.as_str(), "findings": findings, "divzero": divzero })
        })
        .collect();
    Value::Array(funcs)
}

/// Serialise the `unitScope` view: who else can call this file's procedures.
///
/// Shows the registry-declared unit boundaries the file crosses
/// (`package provide`, `source`, `namespace export`, …), whether the host
/// supplied a cross-file view, and the merged call-site evidence per callee
/// with the seed verdict at each argument position — the inputs
/// `tcl_compiler::unit_scope::params_constants_from_call_sites` reads, so a
/// surprising (or surprisingly absent) constant fold can be traced to the
/// evidence that produced it (issue #977).
#[must_use]
pub fn serialise_unit_scope(result: &ExplorerResult) -> Value {
    let scope = &result.unit.caller_scope;
    // `scan_unit_linkage` already masks to `UNIT_LINKAGE_TRAITS`, so every
    // name here is a boundary — and `iter_names` is generated from the trait
    // declarations, so this cannot drift from them (#1034).
    let boundaries: Vec<Value> = scope
        .linkage
        .iter_names()
        .map(|k| Value::String(k.to_owned()))
        .collect();
    let callees: Vec<Value> = scope
        .call_sites
        .callees()
        .map(|name| {
            let evidence = scope
                .call_sites
                .get(name)
                .expect("callee listed by the evidence table");
            let max_index = evidence.slots.keys().copied().max().map_or(0, |m| m + 1);
            let positions: Vec<Value> = (0..max_index)
                .map(|index| {
                    json!({
                        "index": index.to_string(),
                        "verdict": match evidence.uniform_literal_at(index) {
                            Some(literal) => format!("uniform literal {literal:?}"),
                            None => "not uniform".to_owned(),
                        },
                    })
                })
                .collect();
            let arg_counts: Vec<Value> = evidence
                .arg_counts
                .iter()
                .map(|n| Value::String(n.to_string()))
                .collect();
            json!({ "name": name, "positions": positions, "argCounts": arg_counts })
        })
        .collect();
    json!({
        "boundaries": boundaries,
        "hasCrossFileEvidence": scope.has_cross_file_evidence,
        "seeding": if scope
            .linkage
            .intersects(tcl_registry::Traits::PROVIDES_PACKAGE | tcl_registry::Traits::EXPORTS_COMMAND)
        {
            "declined — the file publishes commands beyond any enumerable project"
        } else if !scope.has_cross_file_evidence
            && scope.linkage.intersects(tcl_registry::Traits::LOADS_EXTERNAL_UNIT)
        {
            "declined — another unit is loaded here and no cross-file view was supplied"
        } else {
            "allowed — evidence is treated as the complete caller set"
        },
        "callees": callees,
    })
}

/// Serialise the `interprocedural` view: per-procedure summaries followed
/// by `TclOO` method summaries.
#[must_use]
pub fn serialise_interproc(
    interproc: &InterproceduralAnalysis,
    param_constants: &BTreeMap<String, Vec<(String, u32, String)>>,
) -> Value {
    let mut out: Vec<Value> = Vec::new();

    let mut qnames: Vec<&String> = interproc.procedures.keys().collect();
    qnames.sort();
    for qname in qnames {
        let s = &interproc.procedures[qname];
        out.push(json!({
            "name": qname,
            "arity": arity_str(s.arity),
            "pure": s.pure,
            "foldable": s.can_fold_static_calls,
            "returnShape": format_return_shape(s),
            "calls": s.calls,
            "hasBarrier": s.has_barrier,
            "hasUnknownCalls": s.has_unknown_calls,
            "writesGlobal": s.writes_global,
            // The caller-uniform-literal SCCP seed this procedure was
            // analysed under — the fact that explains a folded condition on
            // a parameter (and, by its absence, an indirect call site the
            // scan could not enumerate).  See
            // `docs/design/compiler/interprocedural-call-site-seeding.md`.
            "paramConstants": param_constants
                .get(qname)
                .map(|seeds| {
                    seeds
                        .iter()
                        .map(|(param, _version, literal)| format!("{param} = {literal}"))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        }));
    }

    let mut mqnames: Vec<&String> = interproc.methods.keys().collect();
    mqnames.sort();
    for mqname in mqnames {
        let m = &interproc.methods[mqname];
        let mut writes_instance: Vec<&String> = m.writes_instance_vars.iter().collect();
        writes_instance.sort();
        out.push(json!({
            "name": mqname,
            "kind": "method",
            "methodKind": m.method_kind,
            "className": m.class_name,
            "pure": m.base.pure,
            "calls": m.base.calls,
            "hasBarrier": m.base.has_barrier,
            "hasUnknownCalls": m.base.has_unknown_calls,
            "writesGlobal": m.base.writes_global,
            "writesInstanceVars": writes_instance,
        }));
    }

    Value::Array(out)
}

/// Serialise the `types` view: every tracked SSA value's lattice type per
/// function, including `?`/`*` entries.
#[must_use]
pub fn serialise_types(result: &ExplorerResult) -> Value {
    let funcs: Vec<Value> = result
        .all_snapshots()
        .iter()
        .filter_map(|snap| {
            let mut entries: Vec<Value> = Vec::new();
            let rt = &snap.unit.return_type;
            entries.push(json!({
                "variable": "(return)",
                "version": 0,
                "type": format_type(rt),
                "kind": type_kind_name(rt.kind()),
            }));
            let ssa = &snap.unit.ssa;
            let mut keys: Vec<_> = snap.unit.types.keys().collect();
            keys.sort_by(|a, b| ssa.var_name(a.0).cmp(ssa.var_name(b.0)).then(a.1.cmp(&b.1)));
            for key in keys {
                let tl = &snap.unit.types[key];
                entries.push(json!({
                    "variable": ssa.var_name(key.0),
                    "version": key.1,
                    "type": format_type(tl),
                    "kind": type_kind_name(tl.kind()),
                }));
            }
            (!entries.is_empty()).then(
                || json!({ "name": snap.name, "kind": snap.kind.as_str(), "entries": entries }),
            )
        })
        .collect();
    Value::Array(funcs)
}

/// Serialise the `segments` view: the `SegmentedCommand` list with each
/// command's closer-inclusive range, source slice, words (in `word_piece`
/// form with per-word shape flags), and forward-attached preceding comment.
///
/// `name` is the first word's text; `subcommand` is always `null` (the CST
/// segment path does not resolve subcommands). `braced` is whether the
/// word's first fragment is a braced `{…}` (`Str`) token; `quoted` whether
/// its raw begins with `"` — both derived from the representative token's
/// first fragment.
#[must_use]
pub fn serialise_segments(source: &str, config: LexerConfig) -> Value {
    let bytes = source.as_bytes();
    let segments: Vec<Value> = segment_commands_with_offset_and_config(source, 0, config)
        .iter()
        .map(|seg| {
            let words: Vec<Value> = seg
                .argv
                .iter()
                .enumerate()
                .map(|(i, tok)| {
                    let start = tok.span.start() as usize;
                    json!({
                        "text": seg.texts[i],
                        "startOffset": tok.span.start(),
                        "endOffset": tok.span.end(),
                        "single": seg.single_token_word[i],
                        "braced": tok.kind == TokenType::Str,
                        "quoted": bytes.get(start) == Some(&b'"'),
                        "expand": seg.expand_word.as_ref().is_some_and(|e| e[i]),
                    })
                })
                .collect();
            let (start, end) = (seg.span.start() as usize, seg.span.end() as usize);
            json!({
                "name": seg.texts.first().cloned().unwrap_or_default(),
                "startOffset": seg.span.start(),
                "endOffset": seg.span.end(),
                "slice": source.get(start..end).unwrap_or(""),
                "precedingComment": seg.preceding_comment,
                "subcommand": Value::Null,
                "words": words,
            })
        })
        .collect();
    Value::Array(segments)
}

/// Serialise the `eventOrder` view: iRules `when EVENT [priority N] { body }`
/// handlers in canonical firing order.
///
/// Reuses the segmenter to find `when` commands (the body must be a braced
/// block) and the reuse-positive `EventRegistry::{order_events,
/// event_multiplicity}` for the canonical ordering / multiplicity class. Empty
/// for ordinary Tcl (no `when` blocks), so it is strictly gateable against the
/// Tcl corpus; the `when`-handler behaviour is pinned by a Rust unit test.
#[must_use]
pub fn serialise_event_order(source: &str, line_index: &LineIndex, dialect: &str) -> Value {
    use std::collections::HashMap;

    use tcl_registry::events::EventRegistry;

    /// One discovered `when` handler.
    struct Handler {
        priority: i64,
        /// File-order index among matched handlers (tie-breaker).
        idx: usize,
        /// The `EVENT` token's span (for the navigable range).
        span: Span,
    }

    let mut per_event: HashMap<String, Vec<Handler>> = HashMap::new();
    let command_registry = registry_for_dialect(dialect);
    let identities = tcl_compiler::realm::document_realm_bindings(
        source,
        crate::environment::analyser_profile_for_dialect(dialect),
        &command_registry,
    );
    for (matched, handler) in
        tcl_registry::events::top_level_when_handlers_with_registry_and_head_resolver(
            source,
            &command_registry,
            &identities,
        )
        .into_iter()
        .enumerate()
    {
        let event = handler.event;
        let span = handler.event_span;
        let priority = i64::from(handler.effective_priority);
        per_event.entry(event).or_default().push(Handler {
            priority,
            idx: matched,
            span,
        });
    }

    // Each event's handlers fire lowest-priority first, file order breaking ties.
    for handlers in per_event.values_mut() {
        handlers.sort_by_key(|h| (h.priority, h.idx));
    }

    let registry = EventRegistry::build();
    let keys: Vec<String> = per_event.keys().cloned().collect();
    let mut entries = Vec::new();
    for event in registry.order_events(&keys) {
        let multiplicity = registry.event_multiplicity(&event);
        let handlers = &per_event[&event];
        let mut prev_priority: Option<i64> = None;
        let mut offset = 0;
        for handler in handlers {
            if prev_priority == Some(handler.priority) {
                offset += 1;
            } else {
                offset = 0;
                prev_priority = Some(handler.priority);
            }
            entries.push(json!({
                "event": event,
                "base_priority": handler.priority,
                "priority_offset": offset,
                "multiplicity": multiplicity,
                "range": range_dict(handler.span, line_index, source),
            }));
        }
    }
    Value::Array(entries)
}

/// Serialise the Rust-native `structuralIndex` view: the lexer's structural
/// pre-scan (`tcl_lexer::structural_index`) — where commands begin, the
/// bracket/brace balance, and the inert spans where `[`/`]`/`{`/`}` are
/// *literal* (brace words, comments, `${…}`, escapes). This acceleration
/// layer drives incremental reparse.
///
/// `commandBoundaries` here is the **reparse split points**, not the
/// segmentation view: `command_boundaries` is registered in
/// `shared-utility-contracts-rust.md` as a dialect-blind byte scan of
/// stock 9.x structure, so on an F5 or 8.x document it can differ from
/// the dialect-resolved answer — which is what the `segments` and `cst`
/// views beside it show.
#[must_use]
pub fn serialise_structural_index(source: &str, li: &LineIndex) -> Value {
    use tcl_lexer::{BraceIndex, BracketIndex, command_boundaries, script_is_complete};

    fn pos(li: &LineIndex, off: u32) -> Value {
        let p = li.position_at(off);
        json!({ "line": p.line, "col": p.character.get(), "offset": p.offset })
    }
    fn inert_spans(li: &LineIndex, source: &str, spans: &[(u32, u32, bool)]) -> Value {
        Value::Array(
            spans
                .iter()
                .map(|&(start, end, terminated)| {
                    let (s, e) = (start as usize, end as usize);
                    json!({
                        "start": start,
                        "end": end,
                        "terminated": terminated,
                        "startPos": pos(li, start),
                        "text": source.get(s..e).unwrap_or(""),
                    })
                })
                .collect(),
        )
    }

    let brackets = BracketIndex::build(source);
    let braces = BraceIndex::build(source);
    let boundaries: Vec<Value> = command_boundaries(source)
        .iter()
        .map(|&off| pos(li, off))
        .collect();

    json!({
        "scriptComplete": script_is_complete(source),
        "commandBoundaries": boundaries,
        "brackets": {
            "unterminated": brackets.unterminated_count(),
            "structuralEvents": brackets.events().len(),
            "inertSpans": inert_spans(li, source, brackets.inert_spans()),
        },
        "braces": {
            "unterminated": braces.unterminated_count(),
            "structuralEvents": braces.events().len(),
            "inertSpans": inert_spans(li, source, braces.inert_spans()),
        },
    })
}

/// Serialise the Rust-native `sourceMap` view: the `LineIndex` span model
/// that powers O(1) offset ↔ line:col resolution (`tcl_lexer::SourceMap` /
/// `LineIndex`). Surfaces the line-start table the analyses resolve every
/// span through — the reference for debugging range/offset bugs.
#[must_use]
pub fn serialise_source_map(source: &str, li: &LineIndex) -> Value {
    let byte_length = u32::try_from(source.len()).unwrap_or(u32::MAX);
    let line_count = u32::try_from(li.line_count()).unwrap_or(u32::MAX);
    let lines: Vec<Value> = (0..line_count)
        .map(|line| {
            let start = li.line_start(line);
            let end = if line + 1 < line_count {
                li.line_start(line + 1)
            } else {
                byte_length
            };
            let text = source
                .get(start as usize..end as usize)
                .unwrap_or("")
                .trim_end_matches('\n');
            json!({
                "line": line,
                "start": start,
                "end": end,
                "length": end - start,
                "text": text,
            })
        })
        .collect();
    json!({
        "byteLength": byte_length,
        "lineCount": line_count,
        "lines": lines,
    })
}

fn serialise_source_map_units(result: &ExplorerResult) -> Value {
    result
        .all_snapshots()
        .iter()
        .map(|snap| {
            json!({
                "name": snap.name,
                "kind": snap.kind.as_str(),
                "baseOffset": snap.unit.base_offset,
            })
        })
        .collect()
}

/// Serialise the `stats` summary.
///
/// `deadStores` counts the optimiser's **O109** dead stores (there is no
/// standalone liveness pass) and the warning counts come from the Rust
/// analyses. `dataflow*` counts are omitted — `dataflow` is not implemented,
/// and such counts only apply when a dataflow graph is present.
fn serialise_stats(result: &ExplorerResult) -> Value {
    let registry_held = registry_for_dialect(&result.dialect);
    let registry = &*registry_held;
    let dialect = crate::environment::known_profile_for_dialect(&result.dialect);

    let unreachable: usize = result
        .all_snapshots()
        .iter()
        .map(|s| {
            s.unit
                .cfg
                .blocks
                .keys()
                .filter(|b| !s.unit.sccp.executable_blocks.contains(*b))
                .count()
        })
        .sum();

    let shimmer = find_shimmer_warnings_for_cu(&result.unit, registry).len()
        + find_thunking_warnings_for_cu(&result.unit, registry).len()
        + find_byte_array_warnings_for_cu(&result.unit, registry).len();
    let mut gvn = find_redundancies_for_cu(&result.unit, registry, dialect);
    gvn.extend(find_partial_redundancies_for_cu(
        &result.unit,
        registry,
        dialect,
    ));
    gvn.extend(find_loop_invariants_for_cu(&result.unit, registry, dialect));
    let taint = find_taint_warnings_for_cu(&result.unit, registry, dialect).len();
    let rewrites = optimise(&result.source, registry).len();
    // Dead stores are O109 optimiser findings (no standalone liveness pass).
    let dead_stores = find_dead_stores(&result.unit, registry, dialect).len();

    json!({
        "procedures": result.unit.ir_module.procedures.len(),
        "functions": result.all_snapshots().len(),
        "blocks": result.total_blocks(),
        "deadStores": dead_stores,
        "unreachableBlocks": unreachable,
        "rewrites": rewrites,
        "shimmerWarnings": shimmer,
        "gvnWarnings": gvn.len(),
        "taintWarnings": taint,
        "irulesFlowWarnings": 0,
    })
}

/// One source-callout annotation, before serialisation.
struct Ann {
    span: Span,
    label: String,
    kind: &'static str,
    severity: &'static str,
    priority: i32,
}

/// Collect compiler-barrier annotations from a script, recursing into
/// If/For/Switch bodies only.
fn walk_barriers(script: &Script, scope: &str, out: &mut Vec<Ann>) {
    for stmt in &script.statements {
        match stmt {
            Statement::Barrier { span, reason, .. } => out.push(Ann {
                span: *span,
                label: format!("{scope}: compiler barrier ({reason})"),
                kind: "barrier",
                severity: "warning",
                priority: 2,
            }),
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    walk_barriers(&c.body, scope, out);
                }
                if let Some(e) = else_body {
                    walk_barriers(e, scope, out);
                }
            }
            Statement::For {
                init, body, next, ..
            } => {
                walk_barriers(init, scope, out);
                walk_barriers(body, scope, out);
                walk_barriers(next, scope, out);
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        walk_barriers(b, scope, out);
                    }
                }
                if let Some(d) = default_body {
                    walk_barriers(d, scope, out);
                }
            }
            _ => {}
        }
    }
}

/// Serialise the `annotations` + `annotationsByLine` source-callout views.
/// Collects all sources (GUI default), then emits per-annotation entries
/// grouped by line.
///
/// Aggregates the optimiser / shimmer / gvn / taint sources. Dead-store
/// callouts come from the optimiser's **O109** findings (there is no
/// standalone liveness pass); constant-branch + unreachable-block callouts
/// come from `sccp`.
// One flat pass aggregating every optimiser/shimmer/gvn/taint callout by line;
// splitting the per-source arms would scatter one contract across helpers.
#[allow(clippy::too_many_lines)]
fn serialise_annotations(result: &ExplorerResult, li: &LineIndex, source: &str) -> (Value, Value) {
    let registry_held = registry_for_dialect(&result.dialect);
    let registry = &*registry_held;
    let dialect = crate::environment::known_profile_for_dialect(&result.dialect);
    let mut anns: Vec<Ann> = Vec::new();

    // Barriers (IR walk).
    walk_barriers(&result.unit.ir_module.top_level, "::top", &mut anns);
    let mut qnames: Vec<&String> = result.unit.ir_module.procedures.keys().collect();
    qnames.sort();
    for qname in qnames {
        walk_barriers(
            &result.unit.ir_module.procedures[qname].body,
            qname,
            &mut anns,
        );
    }

    // Dead stores (O109 optimiser findings — Rust has no standalone liveness
    // pass), constant branches, and unreachable blocks, per function.
    let dead_stores = find_dead_stores(&result.unit, registry, dialect);
    for snap in result.all_snapshots() {
        for dead in dead_stores.iter().filter(|d| d.function == snap.name) {
            let Some(block) = snap.unit.cfg.block_by_name(&dead.block) else {
                continue;
            };
            let Ok(idx) = usize::try_from(dead.statement_index) else {
                continue;
            };
            if let Some(stmt) = block.statements.get(idx) {
                anns.push(Ann {
                    span: stmt.span(),
                    label: format!(
                        "{}: dead store {}#{}",
                        snap.name, dead.variable, dead.version
                    ),
                    kind: "deadStore",
                    severity: "warning",
                    priority: 1,
                });
            }
        }
        for branch in &snap.unit.sccp.constant_branches {
            if let Some(span) = branch.span {
                let dir = if branch.value { "true" } else { "false" };
                anns.push(Ann {
                    span,
                    label: format!(
                        "{}: branch is always {dir}; takes {}",
                        snap.name, branch.taken_target
                    ),
                    kind: "constantBranch",
                    severity: "info",
                    priority: 0,
                });
            }
        }
        let mut unreachable: Vec<tcl_compiler::cfg::BlockId> = snap
            .unit
            .cfg
            .blocks
            .keys()
            .copied()
            .filter(|b| !snap.unit.sccp.executable_blocks.contains(b))
            .collect();
        unreachable.sort_unstable();
        for bn in unreachable {
            let block = &snap.unit.cfg.blocks[&bn];
            let bn_name = snap.unit.cfg.block_name(bn);
            let span = block
                .statements
                .first()
                .map(Statement::span)
                .or_else(|| block.terminator.as_ref().and_then(Terminator::span));
            if let Some(span) = span {
                anns.push(Ann {
                    span,
                    label: format!("{}: unreachable block {bn_name}", snap.name),
                    kind: "unreachable",
                    severity: "warning",
                    priority: 3,
                });
            }
        }
    }

    // Optimiser rewrites.
    for o in optimise(source, registry) {
        anns.push(Ann {
            span: o.span,
            label: format!(
                "{}: {} -> {}",
                o.code,
                o.message,
                preview(&o.replacement, 40)
            ),
            kind: "optimisation",
            severity: "info",
            priority: -1,
        });
    }
    // Shimmer + thunking.
    for w in find_shimmer_warnings_for_cu(&result.unit, registry) {
        let sev = shimmer_severity(w.code.as_str());
        anns.push(Ann {
            span: w.span,
            label: format!("{}: {}", w.code, w.message),
            kind: "shimmer",
            severity: sev,
            priority: if sev == "error" { 1 } else { 2 },
        });
    }
    for w in find_thunking_warnings_for_cu(&result.unit, registry) {
        let sev = shimmer_severity(w.code.as_str());
        anns.push(Ann {
            span: w.span,
            label: format!("{}: {}", w.code, w.message),
            kind: "thunking",
            severity: sev,
            priority: if sev == "error" { 1 } else { 2 },
        });
    }
    for w in find_byte_array_warnings_for_cu(&result.unit, registry) {
        let sev = shimmer_severity(w.code.as_str());
        anns.push(Ann {
            span: w.span,
            label: format!("{}: {}", w.code, w.message),
            kind: "shimmer",
            severity: sev,
            priority: if sev == "error" { 1 } else { 2 },
        });
    }
    // GVN.
    let mut gvn = find_redundancies_for_cu(&result.unit, registry, dialect);
    gvn.extend(find_partial_redundancies_for_cu(
        &result.unit,
        registry,
        dialect,
    ));
    gvn.extend(find_loop_invariants_for_cu(&result.unit, registry, dialect));
    for w in gvn {
        let label = if w.message.is_empty() {
            format!("{}: {}", w.code, w.expression_text)
        } else {
            format!("{}: {}", w.code, w.message)
        };
        anns.push(Ann {
            span: w.span,
            label,
            kind: "gvn",
            severity: "info",
            priority: 1,
        });
    }
    // Taint.
    for w in find_taint_warnings_for_cu(&result.unit, registry, dialect) {
        anns.push(Ann {
            span: w.span,
            label: format!("{}: {}", w.code, w.message),
            kind: "taint",
            severity: taint_severity(w.code.as_str()),
            priority: 0,
        });
    }

    // Sort: (start_offset, priority, span_width, label).
    anns.sort_by(|a, b| {
        a.span
            .start()
            .cmp(&b.span.start())
            .then(a.priority.cmp(&b.priority))
            .then((a.span.end() - a.span.start()).cmp(&(b.span.end() - b.span.start())))
            .then(a.label.cmp(&b.label))
    });

    let dicts: Vec<Value> = anns
        .iter()
        .map(|a| {
            json!({
                "range": range_dict(a.span, li, source),
                "label": a.label,
                "kind": a.kind,
                "severity": a.severity,
                "priority": a.priority,
            })
        })
        .collect();

    // Group annotation indices by 0-based source line.
    let mut by_line: Map<String, Value> = Map::new();
    for (idx, a) in anns.iter().enumerate() {
        let line = li.position_at(a.span.start()).line.to_string();
        by_line
            .entry(line)
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .unwrap()
            .push(json!(idx));
    }

    (Value::Array(dicts), Value::Object(by_line))
}

/// Serialise a full pipeline result to the explorer contract JSON, with
/// every semantic/AOT optimisation pass **off** — the generic lowering.
///
/// Use [`serialise_result_with_optimisations`] to see what a pass changes.
#[must_use]
pub fn serialise_result(result: &ExplorerResult) -> Value {
    serialise_result_with_optimisations(result, SemanticOptimisationConfig::new())
}

/// [`serialise_result`] with a chosen semantic/AOT optimisation
/// configuration.
///
/// The configuration reaches the `wasm` and `wasmOptimised` views (the
/// emitter's `WasmCompileOptions`) and is echoed in the
/// `semanticOptimisations` view, so a front-end can render a toggle per pass
/// and read back which ones the shown module was built with. Every other
/// view is target-neutral and unaffected.
// Assembles the whole explorer JSON contract field-by-field in one place;
// each stage adds one top-level key, so the length is inherent to the schema.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn serialise_result_with_optimisations(
    result: &ExplorerResult,
    optimisations: SemanticOptimisationConfig,
) -> Value {
    let options = tcl_compiler::codegen::wasm::WasmCompileOptions::hosted()
        .with_semantic_optimisations(optimisations);
    let li = LineIndex::new(&result.source);
    let mut out = Map::new();
    out.insert("meta".to_owned(), serialise_meta());
    out.insert(
        "ir".to_owned(),
        serialise_ir(&result.unit.ir_module, &li, &result.source),
    );
    out.insert(
        "cfgPreSsa".to_owned(),
        serialise_cfg_pre_ssa(result, &li, &result.source),
    );
    out.insert(
        "cfgPostSsa".to_owned(),
        serialise_cfg_post_ssa(result, &li, &result.source),
    );
    out.insert("dominators".to_owned(), serialise_dominators(result));
    out.insert(
        "sccp".to_owned(),
        serialise_sccp(result, &li, &result.source),
    );
    out.insert("liveness".to_owned(), serialise_liveness(result));
    out.insert(
        "semantic".to_owned(),
        serialise_semantic(result, &li, &result.source),
    );
    if let Some(interproc) = &result.unit.interproc {
        out.insert(
            "interprocedural".to_owned(),
            serialise_interproc(interproc, &result.unit.caller_scope.param_constants_by_proc),
        );
    }
    out.insert("unitScope".to_owned(), serialise_unit_scope(result));
    out.insert("worldSsa".to_owned(), serialise_world_ssa(result));
    out.insert("types".to_owned(), serialise_types(result));
    // Honour the document's dialect so the CST and segment views tokenise
    // `{*}` / iRules braces the same way the rest of the pipeline does. The
    // grammar comes off the same ingress-resolved profile `run_pipeline`
    // built the unit against, never a second by-name resolution.
    let lexer_config = LexerConfig::for_file_grammar(
        crate::environment::profile_for_dialect(&result.dialect).grammar,
    );
    out.insert(
        "cst".to_owned(),
        crate::cst::serialise_cst(&result.source, lexer_config),
    );
    out.insert(
        "segments".to_owned(),
        serialise_segments(&result.source, lexer_config),
    );
    // Rust-native: the lexer structural pre-scan.
    out.insert(
        "structuralIndex".to_owned(),
        serialise_structural_index(&result.source, &li),
    );
    // Rust-native: the LineIndex span model.
    let mut source_map = serialise_source_map(&result.source, &li);
    if let Value::Object(ref mut map) = source_map {
        map.insert("units".to_owned(), serialise_source_map_units(result));
    }
    out.insert("sourceMap".to_owned(), source_map);
    out.insert("loops".to_owned(), serialise_loops(result));
    out.insert("intervals".to_owned(), serialise_intervals(result));
    out.insert("bounds".to_owned(), serialise_bounds(result));
    out.insert(
        "renderedProperties".to_owned(),
        serialise_rendered_properties(result),
    );
    out.insert(
        "optimisations".to_owned(),
        serialise_optimisations(result, &li, &result.source),
    );
    // Rust-native: the optimiser pass pipeline.
    out.insert(
        "optimiserPasses".to_owned(),
        serialise_optimiser_passes(result, &li, &result.source),
    );
    out.insert(
        "shimmer".to_owned(),
        serialise_shimmer(result, &li, &result.source),
    );
    out.insert(
        "taintWarnings".to_owned(),
        serialise_taint(result, &li, &result.source),
    );
    out.insert("gvn".to_owned(), serialise_gvn(result, &li, &result.source));
    let taint_facts = serialise_taint_tracking(result);
    out.insert("taintFacts".to_owned(), taint_facts.clone());
    // Historical key retained for consumers that have not adopted the
    // coverage-gated descriptor yet.
    out.insert("taintTracking".to_owned(), taint_facts);
    out.insert("dataflow".to_owned(), serialise_dataflow(result));
    out.insert(
        "irulesFlow".to_owned(),
        serialise_irules_flow(result, &li, &result.source),
    );
    out.insert(
        "connectionScope".to_owned(),
        serialise_connection_scope(result),
    );
    out.insert(
        "eventOrder".to_owned(),
        serialise_event_order(&result.source, &li, &result.dialect),
    );
    out.insert(
        "asm".to_owned(),
        crate::asm::serialise_asm(result, &li, &result.source),
    );

    // Double-pipeline keys: re-run on the optimiser's rewritten source.
    let opt = optimised_result(result);
    out.insert(
        "optimisedSource".to_owned(),
        opt.as_ref()
            .map_or(Value::Null, |(_, s)| Value::String(s.clone())),
    );
    out.insert(
        "irOptimised".to_owned(),
        opt.as_ref().map_or(Value::Null, |(r, s)| {
            serialise_ir(&r.unit.ir_module, &LineIndex::new(s), s)
        }),
    );
    out.insert(
        "cfgPreSsaOptimised".to_owned(),
        opt.as_ref().map_or(Value::Null, |(r, s)| {
            serialise_cfg_pre_ssa(r, &LineIndex::new(s), s)
        }),
    );
    out.insert(
        "cfgPostSsaOptimised".to_owned(),
        opt.as_ref().map_or(Value::Null, |(r, s)| {
            serialise_cfg_post_ssa(r, &LineIndex::new(s), s)
        }),
    );
    out.insert(
        "asmOptimised".to_owned(),
        opt.as_ref().map_or(Value::Null, |(r, s)| {
            crate::asm::serialise_asm(r, &LineIndex::new(s), s)
        }),
    );

    // WASM views: drive the analysis-aware WASM emitter used by `tcl compwasm`
    // and surface its WAT plus the rich per-instruction
    // explorer shape (resolved `call`/branch targets, per-instruction ranges)
    // alongside per-function headers, which the text/`wasm` view renders.
    out.insert(
        "wasm".to_owned(),
        serialise_wasm_with_options(result, options),
    );
    out.insert(
        "wasmOptimised".to_owned(),
        opt.as_ref().map_or(Value::Null, |(r, _)| {
            serialise_wasm_with_options(r, options)
        }),
    );
    out.insert(
        "semanticOptimisations".to_owned(),
        serialise_semantic_optimisations(optimisations),
    );

    let (annotations, by_line) = serialise_annotations(result, &li, &result.source);
    out.insert("annotations".to_owned(), annotations);
    out.insert("annotationsByLine".to_owned(), by_line);
    out.insert("stats".to_owned(), serialise_stats(result));
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_pipeline;

    #[test]
    fn meta_lists_all_dialects_views_and_severities() {
        let meta = serialise_meta();
        // Every dialect the registry offers is exposed, and in its order.
        // Derived from `available_dialects` rather than pinned to a count:
        // the invariant worth holding is "the explorer drops none of them",
        // and a magic number only ever announces a new dialect (`spectcl`,
        // most recently) by turning CI red on the branch that adds it.
        let dialects: Vec<&str> = meta["dialects"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["name"].as_str().unwrap())
            .collect();
        assert_eq!(dialects, available_dialects());
        // Every entry carries its catalog labels, like the `views` entries.
        for entry in meta["dialects"].as_array().unwrap() {
            assert!(entry["displayName"].as_str().is_some_and(|s| !s.is_empty()));
            assert!(entry["shortName"].as_str().is_some_and(|s| !s.is_empty()));
        }
        // The original 27 views, plus World SSA, six durable compiler
        // artefact views, and the optimisation-pass toggle surface.
        assert_eq!(meta["views"].as_array().unwrap().len(), 35);
        assert_eq!(meta["severities"], json!(["error", "warning", "info"]));
        let traits = meta["traits"]
            .as_array()
            .expect("trait presentation metadata");
        assert_eq!(traits.len(), tcl_registry::traits::Trait::ALL.len());
        assert!(traits.iter().all(|item| {
            item["name"].as_str().is_some_and(|text| !text.is_empty())
                && item["summary"]
                    .as_str()
                    .is_some_and(|text| !text.is_empty())
                && item["group"].as_str().is_some_and(|text| !text.is_empty())
        }));
        // The parse-tree tab is the CST; there is no `greentree` entry.
        assert_eq!(
            meta["views"][0],
            json!({ "id": "cst", "label": "CST", "payload": "cst", "group": "compiler", "renderKind": "frontend" })
        );
        assert!(
            !meta["views"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["id"] == "greentree"),
            "greentree tab must be dropped"
        );
    }

    #[test]
    fn optimiser_passes_preserve_set_only_tk_profile() {
        let result = run_pipeline("proc recurse {} { recurse }\n", "tk");
        let serialised = serialise_result(&result);
        let passes = serialised["optimiserPasses"]
            .as_array()
            .expect("optimiser passes");
        assert!(
            !passes
                .iter()
                .flat_map(|pass| { pass["optimisations"].as_array().into_iter().flatten() })
                .any(|optimisation| optimisation["code"] == "O121"),
            "Tk must not offer tailcall: {passes:?}"
        );
    }

    #[test]
    fn wasm_view_exposes_canonical_semantic_plan_and_serialised_output() {
        let result = run_pipeline("string length hello", "tcl8.6");
        let wasm = serialise_result(&result)["wasm"].clone();
        let header = &wasm.as_array().expect("WASM entries")[0];

        assert_eq!(header["codegenPlan"]["kind"], "generic-invoke");
        assert!(header["codegenPlan"]["semanticDecline"].is_null());
        assert_eq!(header["codegenPlan"]["regionPlanStatus"], "available");
        let regions = header["codegenPlan"]["regions"]
            .as_array()
            .expect("per-region plan evidence");
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0]["node"], json!([0]));
        assert_eq!(regions[0]["selectedKind"], "generic-prebuilt-argv");
        assert_eq!(regions[0]["operation"]["kind"], "intrinsic");
        assert_eq!(regions[0]["operation"]["id"], "string-length");
        assert_eq!(regions[0]["slowPath"]["kind"], "prebuilt-argv");
        assert_eq!(regions[0]["slowPath"]["wordReplay"], false);
        assert_eq!(regions[0]["candidates"][0]["kind"], "guarded-direct");
        assert_eq!(
            regions[0]["candidates"][0]["reason"],
            "direct-callee-identity-unavailable"
        );
        assert_eq!(regions[0]["candidates"][1]["kind"], "guarded-intrinsic");
        assert_eq!(regions[0]["candidates"][1]["reason"], "pass-disabled");
        assert!(
            header["text"]
                .as_str()
                .expect("serialised WAT")
                .contains("tcl_invoke_argv")
        );
    }

    #[test]
    fn wasm_view_exposes_common_native_i64_selection_evidence() {
        use tcl_compiler::codegen::wasm::{SemanticOptimisationPassId, WasmCompileOptions};

        let result = run_pipeline(
            "proc add {b c} {return [expr {$b + $c}]}\nset d 2\nset e 4\nputs [add $d $e]\n",
            "tcl9.0",
        );
        let options = [
            SemanticOptimisationPassId::DirectProc,
            SemanticOptimisationPassId::MaterialisableSlot,
            SemanticOptimisationPassId::FrameElision,
            SemanticOptimisationPassId::NativeInteger,
            SemanticOptimisationPassId::SemanticOperationSpecialisation,
        ]
        .into_iter()
        .fold(
            WasmCompileOptions::hosted().for_sealed_program(),
            WasmCompileOptions::with_semantic_optimisation,
        );
        let wasm = serialise_wasm_with_options(&result, options);
        let plan = &wasm.as_array().expect("WASM entries")[0]["codegenPlan"];
        assert_eq!(plan["kind"], "native-i64-add");
        assert_eq!(plan["nativeI64Add"]["callee"], "::add");
        assert_eq!(plan["nativeI64Add"]["operands"], json!([2, 4]));
        assert_eq!(
            plan["nativeI64Add"]["boundaryOperation"],
            json!({ "kind": "intrinsic", "id": "channel-write" })
        );
        assert_eq!(plan["nativeI64Add"]["frameElided"], true);
        assert_eq!(plan["nativeI64Add"]["closedProgramStatements"], 4);
    }

    /// The toggle surface a front end renders from: every pass, in a stable
    /// order, with the state the shown module was built with — and the same
    /// catalogue in `meta`, so the panel can be drawn before the first
    /// compile.
    #[test]
    fn semantic_optimisations_view_carries_every_pass_and_its_state() {
        let result = run_pipeline("set a 1\nincr a\n", "tcl9.0");
        let off = serialise_result(&result);
        let ids: Vec<&str> = off["semanticOptimisations"]["passes"]
            .as_array()
            .expect("pass rows")
            .iter()
            .map(|row| row["id"].as_str().expect("a pass id"))
            .collect();
        assert_eq!(
            ids,
            SemanticOptimisationPassId::all()
                .iter()
                .map(|p| p.as_str())
                .collect::<Vec<_>>()
        );
        assert!(
            off["semanticOptimisations"]["passes"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["enabled"] == false),
            "the default is the generic lowering"
        );

        let on = serialise_result_with_optimisations(
            &result,
            SemanticOptimisationConfig::from_names("native-tier").expect("a valid group"),
        );
        let enabled: Vec<&str> = on["semanticOptimisations"]["passes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["enabled"] == true)
            .map(|row| row["id"].as_str().unwrap())
            .collect();
        assert_eq!(
            enabled,
            vec![
                "native-lowering",
                "representation-inference",
                "trace-barrier-elision",
                "cell-demotion"
            ]
        );
        // The selection reaches the emitter, not just the echo.
        assert_eq!(
            on["wasm"].as_array().expect("WASM entries")[0]["codegenPlan"]["nativeLowering"]["enabled"],
            true
        );

        // `meta` carries the catalogue with nothing enabled, so the panel can
        // be built before a compile lands (issue #1183's rule for dialects).
        let meta = serialise_meta();
        assert_eq!(
            meta["semanticOptimisations"]["passes"]
                .as_array()
                .unwrap()
                .len(),
            SemanticOptimisationPassId::all().len()
        );
        let groups: Vec<&str> = meta["semanticOptimisations"]["groups"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| g["id"].as_str().unwrap())
            .collect();
        assert_eq!(groups, vec!["native-tier", "all"]);
    }

    #[test]
    fn wasm_view_exposes_the_native_tier_lowering_record() {
        use tcl_compiler::codegen::wasm::WasmCompileOptions;

        let result = run_pipeline("set a 1\nincr a\nputs $a\n", "tcl9.0");
        let off = serialise_result(&result)["wasm"].clone();
        let off_plan = &off.as_array().expect("WASM entries")[0]["codegenPlan"];
        assert_eq!(off_plan["nativeLowering"]["enabled"], false);

        let wasm = serialise_wasm_with_options(&result, WasmCompileOptions::hosted().native_tier());
        let plan = &wasm.as_array().expect("WASM entries")[0]["codegenPlan"];
        let native = &plan["nativeLowering"];
        assert_eq!(native["enabled"], true);
        let top = &native["functions"]["::top"];
        assert_eq!(top["status"], "lowered");
        let statements = top["statements"].as_array().expect("statement records");
        assert!(statements.iter().any(|statement| {
            statement["instruction"] == "execute-lowered"
                && statement["outcome"] == "native"
                && statement["cells"][0]["access"] == "write"
                && statement["cells"][0]["storage"] == "cell"
                && statement["cells"][0]["storageReason"] == "top-level-global"
                && statement["cells"][0]["barrier"] == "elided:no-trace-reaches-cell"
        }));
        assert!(statements.iter().any(|statement| {
            statement["instruction"] == "invoke" && statement["outcome"] == "native-intrinsic"
        }));
    }

    #[test]
    fn wasm_view_exposes_typed_semantic_decline() {
        let result = run_pipeline("string length $value", "tcl8.6");
        let wasm = serialise_result(&result)["wasm"].clone();
        let plan = &wasm.as_array().expect("WASM entries")[0]["codegenPlan"];

        assert_eq!(plan["kind"], "general");
        assert_eq!(
            plan["semanticDecline"]["kind"],
            "backend-selection-declined"
        );
        assert_eq!(
            plan["semanticDecline"]["detailKind"],
            "no-viable-semantic-plan"
        );
        assert_eq!(plan["regionPlanStatus"], "available");
        assert_eq!(plan["regions"][0]["node"], json!([0]));
        assert_eq!(plan["regions"][0]["slowPath"]["kind"], "prebuilt-argv");
    }

    #[test]
    fn wasm_view_serialises_multiple_mixed_regions_in_node_order() {
        let result = run_pipeline(
            "puts one\nset value 2\nif {$enabled} {puts enabled}\nputs two",
            "tcl9.0",
        );
        let wasm = serialise_result(&result)["wasm"].clone();
        let plan = &wasm.as_array().expect("WASM entries")[0]["codegenPlan"];
        let regions = plan["regions"].as_array().expect("mixed regions");

        assert_eq!(
            regions
                .iter()
                .map(|region| region["node"].clone())
                .collect::<Vec<_>>(),
            vec![
                json!([0]),
                json!([1]),
                json!([2]),
                // The `if` body's invocation is a region nested under the
                // structured region's node.
                json!([2, 0, 0]),
                json!([3])
            ]
        );
        assert_eq!(regions[0]["selectedKind"], "generic-prebuilt-argv");
        assert_eq!(regions[1]["selectedKind"], "lowered");
        assert_eq!(regions[2]["selectedKind"], "structured");
        assert_eq!(regions[3]["selectedKind"], "generic-prebuilt-argv");
        assert_eq!(regions[4]["selectedKind"], "generic-prebuilt-argv");
        assert!(regions[1]["slowPath"].is_null());
        assert!(regions[2]["slowPath"].is_null());
    }

    /// The interprocedural view surfaces the caller-uniform-literal SCCP
    /// seed, and stops surfacing it when a dynamic dispatch reaches the
    /// same procedure with a different literal (issue #976) — the one fact
    /// that explains why a condition on a parameter did or did not fold.
    #[test]
    fn interproc_view_shows_the_param_constant_seed_and_its_withdrawal() {
        const HELPER: &str = "proc helper {mode} {\n\
             if {$mode eq \"prod\"} { set x 1 } else { set x 2 }\n\
             }\nhelper prod\nhelper prod\n";
        let seeded = serialise_result(&run_pipeline(HELPER, "tcl8.6"))["interprocedural"].clone();
        let entry = seeded
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == "::helper")
            .expect("::helper summarised");
        assert_eq!(entry["paramConstants"], json!(["mode = prod"]));

        let src = format!("{HELPER}set cmd helper\n$cmd dev\n");
        let withdrawn = serialise_result(&run_pipeline(&src, "tcl8.6"))["interprocedural"].clone();
        let entry = withdrawn
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == "::helper")
            .expect("::helper summarised");
        assert_eq!(
            entry["paramConstants"],
            json!([]),
            "the `$cmd dev` dispatch also reaches helper, with a differing literal",
        );
    }

    #[test]
    fn world_ssa_view_keeps_versions_transitions_and_typed_availability() {
        let data = serialise_result(&run_pipeline("puts hello\n", "tcl9.0"));
        let top = data["worldSsa"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["name"] == "::top")
            .expect("top-level world sidecar");
        assert!(
            top["availability"]["hasExecutableIr"].as_bool().unwrap(),
            "the availability payload must distinguish an absent graph from absent executable IR"
        );
        assert!(top["availability"]["kind"].is_string());
        assert!(
            !top["operations"].as_array().unwrap().is_empty(),
            "Explorer explicitly requests deep World SSA rather than the interactive no-graph path: {top}",
        );
        assert!(
            top["operations"]
                .as_array()
                .unwrap()
                .iter()
                .all(|operation| {
                    operation["location"]["domain"].is_string()
                        && operation["version"].is_number()
                        && operation["site"]["kind"].is_string()
                }),
            "world operations keep structured location, version, and CFG/site evidence",
        );
    }

    #[test]
    fn world_ssa_decline_preserves_transition_policy_evidence() {
        let data = serialise_result(&run_pipeline(
            "interp create child\ninterp hide child puts\n",
            "tcl9.0",
        ));
        let top = data["worldSsa"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["name"] == "::top")
            .unwrap();
        assert_eq!(top["availability"]["kind"], "world-state-declined");
        let transition = top["invocations"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|invoke| invoke["transitions"].as_array().unwrap())
            .next()
            .expect("registry transition projection");
        assert!(transition["kind"].is_string());
        assert!(transition["commit"].is_string());
        assert!(transition["abruptTransfer"].is_string());
    }

    #[test]
    fn serialise_result_includes_meta_and_ir() {
        let result = run_pipeline("set x 1", "tcl8.6");
        let value = serialise_result(&result);
        assert!(value.get("meta").is_some());
        let ir = &value["ir"];
        assert!(ir["topLevel"].is_array());
        assert!(ir["procedures"].is_object());
    }

    #[test]
    fn durable_compiler_views_are_present_and_cover_extra_units() {
        let source = "oo::class create C { method m {} { set x 1 } }\n\
            set f [list apply {{} { set y 2 }}]\n";
        let result = run_pipeline(source, "tcl8.6");
        crate::coverage::assert_durable_field_inventory(&result.unit);
        let value = serialise_result(&result);
        for key in ["dominators", "sccp", "liveness", "semantic"] {
            assert!(
                value.get(key).is_some(),
                "missing durable view payload {key}"
            );
        }
        let semantic = value["semantic"].as_array().expect("semantic array");
        assert!(semantic.iter().any(|f| f["name"] == "::top"));
        assert!(semantic.iter().any(|f| f["name"] == "::C::m"));
        let meta = value["meta"]["views"].as_array().expect("view metadata");
        for id in ["dominators", "sccp", "liveness", "semantic"] {
            assert!(
                meta.iter().any(|v| v["id"] == id),
                "missing tab metadata {id}"
            );
        }
        let source_map = &value["sourceMap"];
        assert!(
            source_map["units"]
                .as_array()
                .unwrap()
                .iter()
                .any(|unit| { unit["name"] == "::C::m" && unit["baseOffset"].is_number() })
        );

        let dataflow = serialise_result(&run_pipeline(
            "proc f {name} { upvar 1 $name local }",
            "tcl8.6",
        ));
        let f = dataflow["dataflow"]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["name"] == "::f")
            .expect("procedure data-flow function");
        assert_eq!(f["kind"], "procedure");
        assert!(f["memorySsa"]["operations"].is_array());
    }

    #[test]
    fn ir_expands_if_clauses_and_collects_procs() {
        let result = run_pipeline(
            "proc f {x} { if {$x > 0} { puts hi } else { puts lo } }",
            "tcl8.6",
        );
        let ir = serialise_result(&result)["ir"].clone();
        let body = &ir["procedures"]["::f"]["body"];
        let if_node = &body[0];
        assert_eq!(if_node["kind"], "IRIf");
        assert_eq!(if_node["colorClass"], "ir-control");
        // Two children: clause 1 and the else arm.
        let children = if_node["children"].as_array().unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0]["label"], "clause 1: $x > 0");
        assert_eq!(children[1]["label"], "else");
    }

    #[test]
    fn optimised_source_reflects_constant_fold() {
        let result = run_pipeline("set x [expr {1 + 2}]\nputs $x", "tcl8.6");
        let opt = serialise_result(&result)["optimisedSource"].clone();
        // The constant fold changes the source, so it is a non-null string.
        // SCCP proves `x` is `3` and its only use is propagated, so the
        // computed def couple-removes — the result is
        // `puts 3`, not `set x 3`.
        let s = opt.as_str().expect("optimised source string");
        assert!(s.contains("puts 3"), "{s:?}");
        assert!(!s.contains("set x"), "{s:?}");
    }

    #[test]
    fn optimised_source_null_when_unchanged() {
        let result = run_pipeline("puts hello", "tcl8.6");
        let v = serialise_result(&result);
        assert_eq!(v["optimisedSource"], Value::Null);
        // The double-pipeline keys are also null when nothing changed.
        assert_eq!(v["irOptimised"], Value::Null);
        assert_eq!(v["cfgPreSsaOptimised"], Value::Null);
    }

    #[test]
    fn optimised_ir_and_cfg_present_when_source_changes() {
        let result = run_pipeline("set x [expr {1 + 2}]\nputs $x", "tcl8.6");
        let v = serialise_result(&result);
        // The fold rewrites the source, so the optimised views populate and
        // their IR reflects the folded `set x 3`.
        assert!(v["irOptimised"]["topLevel"].is_array());
        assert!(v["cfgPreSsaOptimised"].is_array());
        assert!(!v["cfgPreSsaOptimised"].as_array().unwrap().is_empty());
    }

    #[test]
    fn dead_stores_surface_from_o109() {
        // `set x 1` is overwritten by `set x 2` before any read — the
        // optimiser's O109 finding surfaces in cfgPostSsa.analysis, stats,
        // and as a deadStore callout (Rust has no standalone liveness pass).
        let result = run_pipeline("proc f {} { set x 1; set x 2; return $x }", "tcl8.6");
        let value = serialise_result(&result);

        let dead: Vec<&Value> = value["cfgPostSsa"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|fn_| fn_["analysis"]["deadStores"].as_array().unwrap())
            .collect();
        assert_eq!(dead.len(), 1, "one dead store");
        assert_eq!(dead[0]["variable"], "x");
        assert_eq!(dead[0]["version"], 1);

        assert_eq!(value["stats"]["deadStores"], 1);
        assert!(
            value["annotations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["kind"] == "deadStore"),
            "a deadStore callout is emitted"
        );
    }

    #[test]
    fn no_dead_stores_when_value_is_read() {
        let result = run_pipeline("proc f {} { set x 1; return $x }", "tcl8.6");
        let value = serialise_result(&result);
        assert_eq!(value["stats"]["deadStores"], 0);
    }

    #[test]
    fn structural_index_reports_boundaries_and_balance() {
        // Two complete commands, balanced brackets/braces.
        let result = run_pipeline("set x {a b c}\nputs [llength $x]", "tcl8.6");
        let si = serialise_result(&result)["structuralIndex"].clone();
        assert_eq!(si["scriptComplete"], true);
        assert_eq!(si["commandBoundaries"].as_array().unwrap().len(), 2);
        assert_eq!(si["brackets"]["unterminated"], 0);
        assert_eq!(si["braces"]["unterminated"], 0);
    }

    #[test]
    fn structural_index_flags_incomplete_script() {
        // An unclosed brace leaves the script incomplete with an unterminated
        // inert span.
        let result = run_pipeline("proc f {} {\n  set x 1", "tcl8.6");
        let si = serialise_result(&result)["structuralIndex"].clone();
        assert_eq!(si["scriptComplete"], false);
        assert!(si["braces"]["unterminated"].as_i64().unwrap() >= 1);
    }

    #[test]
    fn source_map_maps_lines_to_byte_ranges() {
        let result = run_pipeline("set x 1\nputs $x\n", "tcl8.6");
        let sm = serialise_result(&result)["sourceMap"].clone();
        assert_eq!(sm["lineCount"], 3); // two lines + the trailing empty line
        let lines = sm["lines"].as_array().unwrap();
        assert_eq!(lines[0]["start"], 0);
        assert_eq!(lines[0]["text"], "set x 1");
        // Line 1 starts right after the first newline.
        assert_eq!(lines[1]["start"], 8);
        assert_eq!(lines[1]["text"], "puts $x");
        // Byte length covers the whole source.
        assert_eq!(sm["byteLength"], 16);
    }

    #[test]
    fn optimiser_passes_lists_every_pass_in_order() {
        // The Rust-native pass-pipeline view: all 9 passes in execution
        // order, each with the optimisations it produced.
        let result = run_pipeline("set x 1\nset y [expr {1 + 2}]\nputs $x$y", "tcl8.6");
        let passes = serialise_result(&result)["optimiserPasses"].clone();
        let arr = passes.as_array().unwrap();
        assert_eq!(arr.len(), 9, "all nine passes are listed");
        // Execution order is fixed (propagation first).
        assert_eq!(arr[0]["id"], "propagation");
        assert_eq!(arr[1]["id"], "branch_folding");
        // Each entry carries a label, count, and the optimisations array.
        for p in arr {
            assert!(p["label"].is_string());
            let opts = p["optimisations"].as_array().unwrap();
            assert_eq!(p["count"].as_u64().unwrap(), opts.len() as u64);
        }
        // Propagation folds the constant var-ref / interpolation here.
        let prop = &arr[0];
        assert!(prop["count"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn gvn_abstains_without_exact_common_invocation_provenance() {
        let result = run_pipeline(
            "set x [list 1 2 3]\nset a [llength $x]\nset b [llength $x]\nputs $a$b",
            "tcl8.6",
        );
        let gvn = serialise_result(&result)["gvn"].clone();
        let arr = gvn.as_array().unwrap();
        // The executable sidecar does not yet retain exact invocation nodes
        // for commands nested inside bracket substitutions. Production GVN
        // therefore fails closed instead of re-enabling its legacy textual
        // scanner and claiming an optimisation without common world facts.
        assert!(arr.is_empty());
    }

    #[test]
    fn irules_flow_empty_for_plain_tcl() {
        let result = run_pipeline("set x 1\nputs $x", "tcl8.6");
        assert_eq!(serialise_result(&result)["irulesFlow"], json!([]));
    }

    #[test]
    fn event_order_empty_for_plain_tcl() {
        let result = run_pipeline("set x 1\nputs $x", "tcl8.6");
        assert_eq!(serialise_result(&result)["eventOrder"], json!([]));
    }

    #[test]
    fn event_order_orders_when_handlers_by_firing_order() {
        // Two HTTP_REQUEST handlers (one with explicit priority) plus
        // RULE_INIT and HTTP_RESPONSE. Verified byte-for-byte for this snippet.
        let src = "when RULE_INIT { set ::x 1 }\n\
                   when HTTP_REQUEST priority 100 { log local0. a }\n\
                   when HTTP_REQUEST { log local0. b }\n\
                   when HTTP_RESPONSE { log local0. c }";
        let result = run_pipeline(src, "f5-irules");
        let order = serialise_result(&result)["eventOrder"].clone();
        let rows = order.as_array().expect("eventOrder is an array");
        assert_eq!(rows.len(), 4);

        // RULE_INIT fires first (init), then HTTP_REQUEST (priority 100 before
        // the default 500), then the default HTTP_REQUEST, then HTTP_RESPONSE.
        assert_eq!(rows[0]["event"], "RULE_INIT");
        assert_eq!(rows[0]["multiplicity"], "init");
        assert_eq!(rows[1]["event"], "HTTP_REQUEST");
        assert_eq!(rows[1]["base_priority"], 100);
        assert_eq!(rows[1]["multiplicity"], "per_request");
        assert_eq!(rows[2]["event"], "HTTP_REQUEST");
        assert_eq!(rows[2]["base_priority"], 500);
        assert_eq!(rows[3]["event"], "HTTP_RESPONSE");
        // The range points at the `EVENT` token, not `when`.
        assert_eq!(rows[0]["range"]["startOffset"], 5);
    }

    #[test]
    fn event_order_tie_break_increments_offset() {
        // Two handlers for the same event at the same (default) priority: the
        // second gets priority_offset 1.
        let src = "when HTTP_REQUEST { log local0. a }\n\
                   when HTTP_REQUEST { log local0. b }";
        let result = run_pipeline(src, "f5-irules");
        let order = serialise_result(&result)["eventOrder"].clone();
        let rows = order.as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["priority_offset"], 0);
        assert_eq!(rows[1]["priority_offset"], 1);
    }

    #[test]
    fn event_order_applies_outer_priority_until_it_changes() {
        let src = "priority 700\n\
                   when HTTP_REQUEST { log local0. first }\n\
                   priority 400\n\
                   when CLIENT_ACCEPTED { log local0. second }\n\
                   when HTTP_REQUEST { log local0. third }\n\
                   when HTTP_REQUEST priority 200 { log local0. inline }";
        let value = serialise_result(&run_pipeline(src, "f5-irules"));
        let rows = value["eventOrder"].as_array().unwrap();
        assert_eq!(rows.len(), 4, "repeated events remain distinct");
        let request: Vec<_> = rows
            .iter()
            .filter(|row| row["event"] == "HTTP_REQUEST")
            .map(|row| row["base_priority"].as_i64().unwrap())
            .collect();
        assert_eq!(request, [200, 400, 700]);
        assert_eq!(rows[0]["event"], "CLIENT_ACCEPTED");
        assert_eq!(rows[0]["base_priority"], 400);
    }

    #[test]
    fn event_order_uses_rooted_normalised_top_level_handlers_only() {
        let src = "::when http_request { if {1} { :::when client_data {} } }";
        let result = run_pipeline(src, "f5-irules");
        let value = serialise_result(&result);
        let rows = value["eventOrder"].as_array().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["event"], "HTTP_REQUEST");
    }

    #[test]
    fn event_order_ignores_non_braced_event_bodies() {
        let src = "set payload {when CLIENT_DATA {}}\n\
                   set q \"when SERVER_DATA {}\"\n\
                   when CLIENT_DATA bare_body\n\
                   when SERVER_DATA \"quoted body\"\n\
                   when HTTP_REQUEST priority 100 {}\n\
                   when HTTP_REQUEST {}";
        let value = serialise_result(&run_pipeline(src, "f5-irules"));
        let rows = value["eventOrder"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["event"], "HTTP_REQUEST");
        assert_eq!(rows[0]["base_priority"], 100);
        assert_eq!(rows[1]["event"], "HTTP_REQUEST");
        assert_eq!(rows[1]["base_priority"], 500);
    }

    #[test]
    fn dataflow_emits_nodes_and_summary() {
        let result = run_pipeline("set x 1\nset y [expr {$x + 1}]\nputs $y", "tcl8.6");
        let df = serialise_result(&result)["dataflow"].clone();
        assert!(df["functions"].is_array());
        let summary = &df["summary"];
        assert!(summary["totalDefs"].as_u64().unwrap() >= 2);
        assert_eq!(
            summary["functionCount"],
            df["functions"].as_array().unwrap().len()
        );
        let f0 = &df["functions"].as_array().unwrap()[0];
        assert!(f0["nodes"].is_array());
        assert!(f0["edges"].is_array());
        assert!(f0["aliases"].is_array());
    }

    #[test]
    fn loops_detects_for_loop_with_depth() {
        // `0 < 10` is statically true on entry, so analysis rotates the loop
        // (FP-RBS-18): the entry guard `for_header` moves out of the natural
        // loop and the back-edge target — the natural-loop header — becomes the
        // `for_body` block, with the `for_step` re-check as the latch.
        let result = run_pipeline("for {set i 0} {$i < 10} {incr i} { puts $i }", "tcl8.6");
        let loops = serialise_result(&result)["loops"].clone();
        let top = &loops.as_array().unwrap()[0];
        let lps = top["loops"].as_array().unwrap();
        assert_eq!(lps.len(), 1, "one natural loop for a single for-loop");
        let lp = &lps[0];
        assert_eq!(lp["depth"], 1);
        let header = lp["header"].as_str().unwrap();
        assert!(
            header.contains("for_body") || header.contains("header"),
            "rotated for-loop header is the body block, got {header}"
        );
        assert!(!lp["latches"].as_array().unwrap().is_empty());
        assert_eq!(
            lp["blockCount"].as_u64().unwrap(),
            lp["blocks"].as_array().unwrap().len() as u64
        );
    }

    #[test]
    fn loops_empty_for_straightline_code() {
        let result = run_pipeline("set x 1\nputs $x", "tcl8.6");
        let loops = serialise_result(&result)["loops"].clone();
        assert!(
            loops.as_array().unwrap()[0]["loops"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn bounds_emits_per_function_shape() {
        let result = run_pipeline("set l [list a b c]\nlindex $l 0", "tcl8.6");
        let bounds = serialise_result(&result)["bounds"].clone();
        let top = &bounds.as_array().unwrap()[0];
        assert_eq!(top["name"], "::top");
        assert!(top["findings"].is_array());
        // The Rust interval analysis emits no divide-by-zero findings.
        assert_eq!(top["divzero"], json!([]));
    }

    #[test]
    fn cfg_post_ssa_has_phis_uses_defs_and_analysis() {
        let result = run_pipeline("set x 1\nset y [expr {$x + 1}]\nputs $y", "tcl8.6");
        let post = serialise_result(&result)["cfgPostSsa"].clone();
        let top = &post.as_array().unwrap()[0];
        assert_eq!(top["name"], "::top");
        assert!(top["analysis"]["constantBranches"].is_array());
        assert!(top["analysis"]["deadStores"].is_array());
        assert!(top["analysis"]["inferredTypes"].is_object());
        let block = &top["blocks"].as_array().unwrap()[0];
        assert!(block["phis"].is_array());
        assert!(block["isEntry"].as_bool().unwrap());
        // A statement carries SSA uses/defs maps.
        let stmt = &block["statements"].as_array().unwrap()[0];
        assert!(stmt["uses"].is_object());
        assert!(stmt["defs"].is_object());
    }

    #[test]
    fn cfg_post_ssa_optimised_present_when_source_changes() {
        let result = run_pipeline("set x [expr {1 + 2}]\nputs $x", "tcl8.6");
        let v = serialise_result(&result);
        assert!(v["cfgPostSsaOptimised"].is_array());
    }

    #[test]
    fn stats_reports_function_and_block_counts() {
        let result = run_pipeline("proc f {} { return 1 }\nf", "tcl8.6");
        let stats = serialise_result(&result)["stats"].clone();
        assert_eq!(stats["procedures"], 1);
        // top-level + ::f.
        assert_eq!(stats["functions"], 2);
        assert!(stats["blocks"].as_u64().unwrap() >= 1);
        assert!(stats["rewrites"].is_number());
        assert!(stats["shimmerWarnings"].is_number());
    }

    #[test]
    fn annotations_collect_callouts_and_group_by_line() {
        let result = run_pipeline(
            "set a 5\nset b [expr {$a + 1}]\nset c [expr {$a + 1}]\nputs $b$c",
            "tcl8.6",
        );
        let v = serialise_result(&result);
        let anns = v["annotations"].as_array().unwrap();
        assert!(!anns.is_empty(), "expected source callouts");
        for a in anns {
            assert!(a["range"]["startOffset"].is_number());
            assert!(a["label"].is_string());
            assert!(a["kind"].is_string());
        }
        // byLine groups annotation indices under string line keys.
        let by_line = v["annotationsByLine"].as_object().unwrap();
        let total: usize = by_line.values().map(|v| v.as_array().unwrap().len()).sum();
        assert_eq!(total, anns.len());
    }

    #[test]
    fn asm_emits_structured_instructions_for_top_level() {
        let result = run_pipeline("set x 1\nputs $x", "tcl8.6");
        let asm = serialise_result(&result)["asm"].clone();
        let top = &asm.as_array().unwrap()[0];
        assert_eq!(top["name"], "::top");
        assert_eq!(top["kind"], "top");
        assert!(top["text"].as_str().unwrap().contains("ByteCode"));
        let ops: Vec<&str> = top["instructions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["kind"] == "instr")
            .map(|r| r["op"].as_str().unwrap())
            .collect();
        // The store and the invoke are present; each instr has a fullText.
        assert!(ops.contains(&"storeStk"));
        assert!(ops.contains(&"invokeStk1"));
        assert!(ops.contains(&"done"));
        let first_instr = top["instructions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["kind"] == "instr")
            .unwrap();
        assert!(
            first_instr["fullText"]
                .as_str()
                .unwrap()
                .starts_with("push1")
        );
    }

    #[test]
    fn asm_resolves_jump_targets() {
        // A conditional produces a jump whose target resolves to a row idx.
        let result = run_pipeline("if {$x} { puts hi }", "tcl8.6");
        let asm = serialise_result(&result)["asm"].clone();
        let top = &asm.as_array().unwrap()[0];
        let has_jump = top["instructions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["jumpTarget"].is_object());
        assert!(has_jump, "expected at least one resolved jump target");
    }

    #[test]
    fn taint_tracking_marks_exec_tainted_vars() {
        let result = run_pipeline("set x [exec ls]\nset y $x", "tcl8.6");
        let tracking = serialise_result(&result)["taintTracking"].clone();
        let top = &tracking.as_array().unwrap()[0];
        let vars: Vec<&str> = top["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["variable"].as_str().unwrap())
            .collect();
        assert!(vars.contains(&"x"));
        assert!(vars.contains(&"y"));
        assert_eq!(top["entries"][0]["taint"], "tainted");
    }

    #[test]
    fn taint_reports_eval_sink() {
        let result = run_pipeline("set x [gets stdin]\neval $x", "tcl8.6");
        let taint = serialise_result(&result)["taintWarnings"].clone();
        let arr = taint.as_array().unwrap();
        let t = arr
            .iter()
            .find(|w| w["code"] == "T100")
            .expect("T100 warning");
        assert_eq!(t["variable"], "x");
        assert_eq!(t["sinkCommand"], "eval");
        assert_eq!(t["severity"], "error");
    }

    #[test]
    fn shimmer_reports_intrep_conversion() {
        let result = run_pipeline("set x [list 1 2 3]\nincr x", "tcl8.6");
        let shimmer = serialise_result(&result)["shimmer"].clone();
        let arr = shimmer.as_array().unwrap();
        let s100 = arr
            .iter()
            .find(|w| w["code"] == "S100")
            .expect("S100 warning");
        assert_eq!(s100["variable"], "x");
        assert_eq!(s100["fromType"], "list");
        assert_eq!(s100["toType"], "int");
        assert_eq!(s100["command"], "incr");
        assert_eq!(s100["severity"], "warning");
    }

    #[test]
    fn optimisations_reports_constant_fold_with_range() {
        // A by-name read (`[set x]`) keeps the def alive, so the O101
        // constant-fold survives as its own reported rewrite rather than
        // being superseded by the dead-store coupling.
        let result = run_pipeline("set x [expr {1 + 2}]\nputs [set x]", "tcl8.6");
        let opts = serialise_result(&result)["optimisations"].clone();
        let arr = opts.as_array().unwrap();
        assert!(!arr.is_empty(), "expected at least one optimisation");
        // Every entry carries the contract fields.
        for o in arr {
            assert!(o["code"].is_string());
            assert!(o["message"].is_string());
            assert!(o["replacement"].is_string());
            assert!(o["range"]["startOffset"].is_number());
        }
        // The constant-fold rewrite is present.
        assert!(arr.iter().any(|o| o["code"] == "O101"));
    }

    #[test]
    fn rendered_properties_flags_slash_and_dash() {
        let result = run_pipeline("set p /var/log\nset u -flag", "tcl8.6");
        let rp = serialise_result(&result)["renderedProperties"].clone();
        let entries = rp.as_array().unwrap()[0]["entries"].as_array().unwrap();
        let p = entries.iter().find(|e| e["variable"] == "p").unwrap();
        assert!(
            p["may"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f == "HAS_FORWARD_SLASH")
        );
        let u = entries.iter().find(|e| e["variable"] == "u").unwrap();
        assert!(
            u["must"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f == "STARTS_WITH_DASH")
        );
    }

    #[test]
    fn intervals_reports_bounded_loop_counter() {
        let result = run_pipeline("for {set i 0} {$i < 10} {incr i} { puts $i }", "tcl8.6");
        let intervals = serialise_result(&result)["intervals"].clone();
        let top = &intervals.as_array().unwrap()[0];
        assert_eq!(top["name"], "::top");
        let entries = top["entries"].as_array().unwrap();
        // At least one bounded entry for `i` with a concrete lower bound.
        assert!(
            entries
                .iter()
                .any(|e| e["variable"] == "i" && e["lo"].is_number())
        );
    }

    #[test]
    fn segments_reports_words_with_shape_flags() {
        let segs = serialise_segments("string length \"hi\"", LexerConfig::default());
        let arr = segs.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        let cmd = &arr[0];
        assert_eq!(cmd["name"], "string");
        assert_eq!(cmd["subcommand"], Value::Null);
        let words = cmd["words"].as_array().unwrap();
        assert_eq!(words.len(), 3);
        assert_eq!(words[0]["text"], "string");
        assert_eq!(words[2]["quoted"], true);
        assert_eq!(words[2]["braced"], false);
    }

    #[test]
    fn segments_marks_braced_words() {
        let segs = serialise_segments("if {$x} { puts hi }", LexerConfig::default());
        let words = segs[0]["words"].as_array().unwrap();
        // `{$x}` is a braced word.
        assert_eq!(words[1]["braced"], true);
        assert_eq!(words[1]["quoted"], false);
    }

    #[test]
    fn types_reports_known_int_for_constant_assignments() {
        let result = run_pipeline("set x 1\nset y 2", "tcl8.6");
        let types = serialise_result(&result)["types"].clone();
        let top = types
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["name"] == "::top")
            .unwrap();
        let x = top["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["variable"] == "x")
            .unwrap();
        assert_eq!(x["type"], "int");
        assert_eq!(x["kind"], "known");
    }

    #[test]
    fn interproc_summarises_procs_with_arity_and_return_shape() {
        let result = run_pipeline("proc id {x} { return $x }\nid 5", "tcl8.6");
        let interproc = serialise_result(&result)["interprocedural"].clone();
        let add = interproc
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["name"] == "::id")
            .expect("::id summary present");
        assert_eq!(add["arity"], "1..1");
        assert_eq!(add["returnShape"], "passthrough(x)");
        assert!(add["pure"].is_boolean());
        assert!(add["calls"].is_array());
    }

    /// The Unit Scope view must show *why* the interprocedural seed fired:
    /// the registry-declared boundaries the file crosses, whether a
    /// cross-file view was supplied, and the per-position verdict (#977).
    #[test]
    fn unit_scope_reports_uniform_literals_and_no_boundary() {
        let result = run_pipeline(
            "proc helper {mode} { if {$mode eq \"prod\"} { set r 1 } }\nhelper prod\nhelper prod\n",
            "tcl8.6",
        );
        let scope = serialise_result(&result)["unitScope"].clone();
        assert_eq!(scope["boundaries"].as_array().unwrap().len(), 0);
        assert_eq!(scope["hasCrossFileEvidence"], false);
        assert!(
            scope["seeding"].as_str().unwrap().starts_with("allowed"),
            "{scope:?}"
        );
        let helper = scope["callees"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "::helper")
            .expect("::helper evidence present");
        assert_eq!(
            helper["positions"][0]["verdict"],
            "uniform literal \"prod\""
        );
    }

    /// The same file with a `package provide` crosses a registry-declared
    /// boundary, so the view must report the boundary and the declined seed.
    #[test]
    fn unit_scope_reports_a_registry_declared_boundary() {
        let result = run_pipeline(
            "package provide mylib 1.0\nproc helper {mode} { if {$mode eq \"prod\"} { set r 1 } }\nhelper prod\n",
            "tcl8.6",
        );
        let scope = serialise_result(&result)["unitScope"].clone();
        assert_eq!(
            scope["boundaries"].as_array().unwrap(),
            &vec![Value::String("PROVIDES_PACKAGE".to_owned())]
        );
        assert!(
            scope["seeding"].as_str().unwrap().starts_with("declined"),
            "{scope:?}"
        );
    }

    #[test]
    fn cfg_pre_ssa_has_entry_block_and_branch_terminator() {
        let result = run_pipeline("if {$x} { puts a } else { puts b }", "tcl8.6");
        let cfg = serialise_result(&result)["cfgPreSsa"].clone();
        let top = &cfg[0];
        assert_eq!(top["name"], "::top");
        assert_eq!(top["entry"], "entry_1");
        let entry = top["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["name"] == "entry_1")
            .unwrap();
        assert_eq!(entry["isEntry"], true);
        // The entry block branches on the `if` condition.
        assert_eq!(entry["terminator"]["type"], "branch");
        // Edges carry routing lanes from cfg_layout.
        assert!(
            top["edges"]
                .as_array()
                .unwrap()
                .iter()
                .all(|e| e["lane"].is_u64())
        );
    }

    #[test]
    fn ir_widens_braced_condition_range_to_closer() {
        // `{$x}` opens at the `{`; the range must cover the closing `}`.
        let src = "if {$x} { puts 1 }";
        let result = run_pipeline(src, "tcl8.6");
        let ir = serialise_result(&result)["ir"].clone();
        let cond_range = &ir["topLevel"][0]["children"][0]["range"];
        let start = usize::try_from(cond_range["startOffset"].as_u64().unwrap()).unwrap();
        let end = usize::try_from(cond_range["endOffset"].as_u64().unwrap()).unwrap();
        assert_eq!(&src[start..end], "{$x}");
    }
}

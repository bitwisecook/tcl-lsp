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

//! Units for bodies that are not procedures: `TclOO` methods, `apply` lambdas, `namespace eval` bodies and the cross-event connection scope.

use super::function_build::conditional_function_source_entry;
use super::lattice_request::{LatticeRequest, ProcLatticeCache};
use super::source_lowering::module_source_metadata_input;
use super::{
    BodyUnitContext, CompilationUnit, DeclaredBodyMemoContext, FunctionBuildInputs, FunctionUnit,
    MethodBodyFacts, UnitDialect, original_lattice_worker,
};
use crate::cfg::Function as CfgFunction;
use crate::ir::Module as IrModule;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tcl_registry::CommandRegistry;

#[derive(Clone, Copy)]
struct BodyMemoBuildInputs<'a> {
    pub(super) ir_module: &'a IrModule,
    pub(super) registry: &'a CommandRegistry,
    id: &'a crate::ir::ProcedureId,
    proc: &'a crate::ir::Procedure,
    pub(super) cfg: &'a CfgFunction,
    pub(super) qname: &'a str,
    pub(super) context: BodyUnitContext<'a>,
    memo_context: DeclaredBodyMemoContext<'a>,
    original_request_scope: Option<&'a original_lattice_worker::OriginalLatticeRequestScope>,
}

fn memoized_body_unit(
    cache: &mut Option<&mut ProcLatticeCache<'_>>,
    inputs: BodyMemoBuildInputs<'_>,
    body_bytes: usize,
    original_declaration_body: bool,
) -> Option<FunctionUnit> {
    if body_bytes > crate::ssa::DEEP_ANALYSIS_BODY_BYTES || !original_declaration_body {
        return None;
    }
    let BodyMemoBuildInputs {
        ir_module,
        registry,
        id,
        proc,
        cfg,
        qname,
        context,
        memo_context,
        original_request_scope,
    } = inputs;
    let memo = cache.as_mut()?;
    let (upvar_procs, proc_params, global_write_procs, command_bindings, procedure_summary_owners) =
        memo_context.prepared?;
    let source = ir_module.source.try_text().ok()?;
    let memo_config = context.config.nested().normalized();
    let memo_input = module_source_metadata_input(ir_module, registry)
        .map(crate::analyser::ResolvedAnalysisInput::for_nested_source);
    let mut body = proc.body.clone();
    crate::lattice_rebase::rebase_script(&mut body, -i64::from(proc.span.start()));
    let mut request = LatticeRequest {
        original_worker_request: None,
        module: ir_module,
        body_id: *id,
        qname,
        body: &body,
        source,
        original_body_offset: proc.span.start(),
        executable_body_offset: proc.body_offset,
        body_source: proc.body_source.as_deref(),
        irules_event_body: None,
        command_trust: context.command_trust,
        params: &proc.params,
        upvar_procs,
        proc_params,
        global_write_procs,
        command_bindings,
        procedure_summary_owners,
        source_metadata_input: memo_input.as_ref(),
        lexer_config: memo_config,
        dialect: memo_context.dialect,
        plain_command_dispatch: ir_module.plain_command_dispatch,
        param_constants: &[],
        known_classes: memo_context.known_classes,
        traced_variables: memo_context.traced_variables,
        analysis_context: memo_context.analysis_context,
        has_dynamic_variable_trace: ir_module.has_dynamic_variable_trace,
    };
    let original_request = original_lattice_worker::OriginalLatticeRequestSeal::capture(
        &request,
        original_request_scope.expect("actual memo context"),
        registry,
    );
    request.original_worker_request = Some(&original_request);
    let mut unit = memo(&request);
    unit.original_lattice_callback_admission = Some(
        LatticeRequest::accepts_original_worker_result(&request, &unit, registry),
    );
    crate::lattice_rebase::rebase_function_unit(&mut unit, i64::from(proc.span.start()));
    (unit.cfg == *cfg && !unit.sccp.reads_module).then_some(unit)
}

fn body_unit_entry_context(
    ir_module: &IrModule,
    registry: &CommandRegistry,
    id: &crate::ir::ProcedureId,
    proc: &crate::ir::Procedure,
    cfg: &CfgFunction,
) -> (bool, bool, crate::var_resolve::ResolveContext) {
    let original_declaration_body = (ir_module.original_declaration_body_units.contains_key(id)
        || ir_module
            .original_native_declaration_body_units
            .contains_key(id)
        || ir_module
            .original_logical_declaration_body_units
            .contains_key(id)
        || ir_module
            .original_hosted_declaration_body_units
            .contains_key(id)
        || ir_module
            .original_authored_declaration_body_units
            .contains_key(id))
        && ir_module
            .original_source_procedure_bodies()
            .any(|(owned, _)| owned == id);
    let declaration_body =
        original_declaration_body || ir_module.installed_procedure_body_units.contains_key(id);
    let mut entry_context = if original_declaration_body {
        conditional_function_source_entry(cfg, module_source_metadata_input(ir_module, registry))
    } else if declaration_body {
        if let Some(namespace) = proc.body.namespace_context.as_deref() {
            let frame = tcl_names::variable::VariableExecutionFrame::Procedure {
                namespace: namespace.display().unwrap_or_default(),
                identity: id.reporting_key(),
            }
            .with_namespace_identity(namespace.clone());
            crate::var_resolve::ResolveContext::default().in_frame(&frame)
        } else {
            crate::var_resolve::ResolveContext::default()
                .in_frame(&tcl_names::variable::VariableExecutionFrame::Unknown)
        }
    } else if let Some(namespace) = proc.body.namespace_context.as_deref() {
        let frame = if ir_module.lambda_body_units.contains(id) {
            tcl_names::variable::VariableExecutionFrame::Procedure {
                namespace: namespace.display().unwrap_or_default(),
                identity: id.reporting_key(),
            }
        } else {
            tcl_names::variable::VariableExecutionFrame::NamespaceActivation {
                namespace: namespace.display().unwrap_or_default(),
                identity: id.reporting_key(),
            }
        }
        .with_namespace_identity(namespace.clone());
        crate::var_resolve::ResolveContext::default().in_frame(&frame)
    } else {
        crate::var_resolve::ResolveContext::default()
            .in_frame(&tcl_names::variable::VariableExecutionFrame::Unknown)
    };
    entry_context.namespace_cells.closed = false;
    (original_declaration_body, declaration_body, entry_context)
}

fn body_unit_request_scope(
    ir_module: &IrModule,
    cache: &Option<&mut ProcLatticeCache<'_>>,
    memo_context: DeclaredBodyMemoContext<'_>,
) -> Option<original_lattice_worker::OriginalLatticeRequestScope> {
    cache.as_ref().and(memo_context.prepared).map(|context| {
        original_lattice_worker::OriginalLatticeRequestScope::capture(ir_module, context)
    })
}

impl CompilationUnit {
    /// Lower `TclOO` method bodies (populated in `ir_module.methods` by
    /// lowering) to per-method [`FunctionUnit`]s, using the same CFG → SSA →
    /// analysis pipeline as procs.  Kept in a separate map so the per-proc
    /// diagnostic passes (which iterate `procedures`) are unaffected — only the
    /// interproc purity summary and the O126 optimiser gate consume methods.
    /// Returns an empty map for non-OO sources (skipping the upvar-context scan).
    pub(super) fn build_method_units(
        ir_module: &IrModule,
        extra_callers: &[crate::unit_scope::ExtraCallSiteScanContext],
        ctx: BodyUnitContext<'_>,
    ) -> HashMap<crate::ir::ProcedureId, FunctionUnit> {
        let BodyUnitContext {
            registry,
            known_class_set,
            trace_facts,
            command_trust,
            config,
            native_source: _,
        } = ctx;
        if ir_module.methods.is_empty() {
            return HashMap::new();
        }
        // A callee reached via `my` / `next` is a method, never in the
        // upvar-procs table (the dispatch does not name its target), so its
        // `upvar 1 $refvar …` caller-frame definition is invisible to a
        // defs-based check, which would treat the dispatch as a no-op and
        // raise a false always-false I230 on `info exists` of an
        // upvar-defined local (on tclsh 9.0.4 / 8.6.14, after
        // `my Reference? $lookup ref` returns true, `[info exists ref]` in
        // the caller is 1).  Widen the
        // dispatch sites of exactly the methods whose reachable dispatch
        // surface meets a caller-frame-reaching (or unanalysable) class —
        // the same per-method barrier the optimiser's propagation gate
        // consumes, so the two stay one evidence rule. The CFG builder takes
        // this as a typed capability; the user-procedure summary map remains
        // free of synthetic command-name entries.
        ir_module
            .methods
            .iter()
            .map(|(id, method)| {
                let mqname = method.reporting_label();
                let cfg = extra_callers
                    .iter()
                    .find_map(|caller| caller.body_analysis_cfg(*id))
                    .expect("call-site scan prepares every method CFG")
                    .clone();
                let command_resolution_guarded = matches!(
                    method.execution_namespace,
                    crate::ir::ExecutionNamespace::RuntimeSelected
                )
                    && crate::ir_helpers::requires_runtime_command_namespace(
                        &method.body,
                        registry,
                    );
                // Body-byte half of the complexity guard (the block-count
                // half is applied inside `build`). A TclOO receiver-selected
                // method with relative command dependencies is guarded too:
                // its lexical fallback IR is useful for indexing, but cannot
                // soundly drive CFG topology, SCCP, diagnostics, or rewrites
                // when an object-local command may shadow that dependency.
                let body_bytes = method
                    .span
                    .map_or(0usize, |s| s.end().saturating_sub(s.start()) as usize);
                let mut fu = if command_resolution_guarded
                    || body_bytes > crate::ssa::DEEP_ANALYSIS_BODY_BYTES
                {
                    // The guarded unit still carries its method facts: the
                    // deep lattices are skipped, but every consumer of "which
                    // names are bound in this method's frame" must read the
                    // same carrier as the deep path.
                    let mut fu = FunctionUnit::trivial_guarded(&mqname, cfg).with_source_metadata(
                        module_source_metadata_input(ir_module, registry),
                        config,
                    );
                    fu.method_facts = Some(Arc::new(MethodBodyFacts::from_method(method)));
                    fu
                } else {
                    FunctionUnit::build_for_method(
                        &mqname,
                        cfg,
                        (*id, method),
                        UnitDialect {
                            registry,
                            config,
                            source_metadata_input: module_source_metadata_input(
                                ir_module, registry,
                            ),
                        },
                        known_class_set,
                        trace_facts,
                        command_trust,
                    )
                }
                .with_retained_semantic_analysis(
                    registry,
                    Some(&method.body),
                    crate::dispatch_proof::DispatchEntryAssumption::UnknownWorld,
                );
                fu.body_id = Some(*id);
                (*id, fu)
            })
            .collect()
    }

    /// Build cross-event facts for every retained event body.
    ///
    /// `None` lets non-iRules consumers skip the empty sweep.
    pub(super) fn build_connection_scope(
        procedures: &HashMap<crate::ir::ProcedureId, FunctionUnit>,
        registry: &CommandRegistry,
        module: &crate::ir::Module,
    ) -> Option<crate::connection_scope::ConnectionScope> {
        let when_procs: HashMap<crate::ir::ProcedureId, FunctionUnit> = procedures
            .iter()
            .filter(|(_, unit)| unit.irules_event_body.is_some())
            .map(|(id, unit)| (*id, unit.clone()))
            .collect();
        (!when_procs.is_empty()).then(|| {
            crate::connection_scope::build_connection_scope_for_module(
                &when_procs,
                registry,
                module,
            )
        })
    }

    /// Lower the synthetic *body units* (`apply` lambdas and `namespace eval`
    /// bodies, populated in [`crate::ir::Module::body_units`] by lowering) to
    /// per-body [`FunctionUnit`]s, using the same CFG → SSA → analysis pipeline
    /// as procs and methods.
    ///
    /// Each body runs in a *fresh* frame (its own locals), exactly like a proc
    /// or method body — so the CFG is built with `is_proc = true` and the
    /// module upvar/param context, matching [`Self::build_method_units`]. Kept
    /// in a separate map so no existing consumer (codegen, optimiser, per-proc
    /// diagnostics) is affected; only [`Self::body_function_units`] surfaces
    /// them. Returns an empty map when the module has no body units (the common
    /// case), skipping all work.
    pub(super) fn build_body_units(
        ir_module: &IrModule,
        extra_callers: &[crate::unit_scope::ExtraCallSiteScanContext],
        ctx: BodyUnitContext<'_>,
        mut cache: Option<&mut ProcLatticeCache<'_>>,
        memo_context: DeclaredBodyMemoContext<'_>,
    ) -> HashMap<crate::ir::ProcedureId, FunctionUnit> {
        let body_context = ctx;
        let BodyUnitContext {
            registry,
            known_class_set,
            trace_facts,
            command_trust,
            config,
            native_source,
        } = ctx;
        if ir_module.body_units.is_empty() {
            return HashMap::new();
        }
        let original_request_scope = body_unit_request_scope(ir_module, &cache, memo_context);
        let no_extra_escaping = HashSet::new();
        ir_module
            .body_units
            .iter()
            .map(|(id, proc)| {
                let qname = &proc.qualified_name;
                let cfg = extra_callers
                    .iter()
                    .find_map(|caller| caller.body_analysis_cfg(*id))
                    .expect("call-site scan prepares every body-unit CFG")
                    .clone();
                // Same body-byte complexity guard as procs/methods: a huge
                // generated lambda body contributes trivial lattices instead of
                // a deep (and slow) analysis.
                let body_bytes = proc.span.end().saturating_sub(proc.span.start()) as usize;
                let (original_declaration_body, _, entry_context) =
                    body_unit_entry_context(ir_module, registry, id, proc, &cfg);
                let native_source = native_source.filter(|_| original_declaration_body);
                // This body-only memo has no module-source query dependencies.
                // Keep both successful and refused new queries on the current view.
                let memoised = if native_source.is_some() {
                    None
                } else {
                    memoized_body_unit(
                        &mut cache,
                        BodyMemoBuildInputs {
                            ir_module,
                            registry,
                            id,
                            proc,
                            cfg: &cfg,
                            qname,
                            context: body_context,
                            memo_context,
                            original_request_scope: original_request_scope.as_ref(),
                        },
                        body_bytes,
                        original_declaration_body,
                    )
                };
                let mut fu = memoised
                    .unwrap_or_else(|| {
                        if body_bytes > crate::ssa::DEEP_ANALYSIS_BODY_BYTES {
                            FunctionUnit::trivial_guarded(qname, cfg).with_source_metadata(
                                module_source_metadata_input(ir_module, registry),
                                config,
                            )
                        } else {
                            FunctionUnit::build_full(
                                qname.clone(),
                                cfg,
                                FunctionBuildInputs {
                                    is_event_body: false,
                                    analysis_body: Some(crate::sccp::AnalysisBody::Procedure(*id)),
                                    initial_state:
                                        crate::sccp::ExistenceInitialState::DeclaredFrame,
                                    source_metadata_input: module_source_metadata_input(
                                        ir_module, registry,
                                    ),
                                    entry_context: Some(&entry_context),
                                    config,
                                    params: &proc.params,
                                    registry,
                                    param_constants: None,
                                    known_classes: known_class_set,
                                    extra_global_escaping: &no_extra_escaping,
                                    trace_facts,
                                    analysis_context: None,
                                    command_trust,
                                    object_state: None,
                                    initial_global: false,
                                    procedures: native_source,
                                },
                            )
                        }
                    })
                    .with_retained_semantic_analysis(
                        registry,
                        Some(&proc.body),
                        crate::dispatch_proof::DispatchEntryAssumption::UnknownWorld,
                    );
                fu.original_hosted_procedure_body = ir_module
                    .original_hosted_declaration_body_units
                    .get(id)
                    .filter(|card| card.matches_procedure(ir_module, proc, registry))
                    .cloned();
                (*id, fu.with_body_id(*id))
            })
            .collect()
    }

    /// Cross-event existence post-pass: `existence_constant_branches` runs per
    /// function and folds `[info exists VAR]` → false for any VAR not defined
    /// *in that event*. That is unsound for an iRules cross-event variable
    /// (set in another `when` handler), so drop those folds from `::when::*`
    /// procs once the connection scope is known — otherwise O101 rewrites
    /// `if {[info exists ans_cleared]}` to `if {0}` even though a sibling event
    /// set it (a miscompile).
    pub(super) fn drop_cross_event_existence_folds(
        procedures: &mut HashMap<crate::ir::ProcedureId, FunctionUnit>,
        connection_scope: Option<&crate::connection_scope::ConnectionScope>,
        registry: &CommandRegistry,
    ) {
        let Some(cs) = connection_scope else {
            return;
        };
        for (body_id, fu) in procedures.iter_mut() {
            let Some(event) = &fu.irules_event_body else {
                continue;
            };
            let entry = event.conditional_entry();
            let fallback_points;
            let points = if let Some(points) = &fu.ssa.point_contexts {
                points
            } else {
                fallback_points = crate::place_bridge::build_point_resolve_contexts_with_entry(
                    &fu.cfg, entry, registry,
                );
                &fallback_points
            };
            fu.sccp.constant_branches.retain(|cb| {
                // Consume the actual CFG condition and registry VarRead roles.
                // Literal strings resembling an info query never become reads.
                let Some((id, block)) = fu
                    .cfg
                    .blocks
                    .iter()
                    .find(|(_, block)| block.name == cb.block)
                else {
                    return false;
                };
                let Some(terminator) = &block.terminator else {
                    return false;
                };
                let context = points.before_terminator(*id);
                !crate::place_bridge::terminator_read_places(terminator, context, registry)
                    .iter()
                    .any(|place| cs.observes_cross_event_place(body_id, place))
            });
        }
    }
}

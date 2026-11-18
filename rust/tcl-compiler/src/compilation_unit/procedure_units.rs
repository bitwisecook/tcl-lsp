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

//! One `FunctionUnit` per procedure: the complexity guard and the lattice memo.

use super::lattice_request::{LatticeRequest, ProcLatticeCache};
use super::source_lowering::module_source_metadata_input;
use super::{
    FunctionUnit, ModuleTraceFacts, ProcedureBuildContext, UnitDialect, original_lattice_worker,
};
use crate::cfg::{CfgModule, Function as CfgFunction};
use crate::cfg_builder::PreparedCfgContext;
use crate::ir::Module as IrModule;
use std::collections::HashMap;
use std::sync::Arc;
use tcl_registry::CommandRegistry;

#[derive(Clone, Copy)]
struct ProcedureMemoInputs<'a> {
    ctx: &'a ProcedureBuildContext<'a>,
    id: &'a crate::ir::ProcedureId,
    pub(super) cfg: &'a CfgFunction,
    proc: Option<&'a crate::ir::Procedure>,
    pub(super) params: &'a [String],
    pub(super) qname: &'a str,
    pub(super) event_body: Option<&'a Arc<crate::ir::SourceIrulesEventBody>>,
    original_request_scope: Option<&'a original_lattice_worker::OriginalLatticeRequestScope>,
    memo_config: tcl_lexer::LexerConfig,
    memo_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    pub(super) body_offset: u32,
}

fn memoized_procedure_unit(
    cache: &mut Option<&mut ProcLatticeCache<'_>>,
    input: ProcedureMemoInputs<'_>,
    has_instance_global_writes: bool,
) -> Option<FunctionUnit> {
    let ProcedureMemoInputs {
        ctx,
        id,
        cfg,
        proc,
        params,
        qname,
        event_body,
        original_request_scope,
        memo_config,
        memo_input,
        body_offset,
    } = input;
    match (
        cache.as_mut(),
        proc,
        ctx.cfg_context,
        has_instance_global_writes,
        ctx.ir_module.source.try_text().ok(),
    ) {
        (
            Some(memo),
            Some(proc),
            Some((
                upvar_procs,
                proc_params,
                global_write_procs,
                command_bindings,
                procedure_summary_owners,
            )),
            false,
            Some(source),
        ) => {
            let mut body = proc.body.clone();
            crate::lattice_rebase::rebase_script(&mut body, -i64::from(body_offset));
            let mut request = LatticeRequest {
                original_worker_request: None,
                module: ctx.ir_module,
                body_id: *id,
                qname,
                body: &body,
                source,
                original_body_offset: body_offset,
                executable_body_offset: proc.body_offset,
                body_source: proc.body_source.as_deref(),
                irules_event_body: event_body,
                command_trust: ctx.command_trust,
                params,
                upvar_procs,
                proc_params,
                global_write_procs,
                command_bindings,
                procedure_summary_owners,
                source_metadata_input: memo_input,
                lexer_config: memo_config,
                dialect: ctx.dialect,
                plain_command_dispatch: ctx.ir_module.plain_command_dispatch,
                param_constants: &[],
                known_classes: ctx.known_classes,
                traced_variables: ctx.traced_variable_names,
                analysis_context: ctx.analysis_context,
                has_dynamic_variable_trace: ctx.ir_module.has_dynamic_variable_trace,
            };
            let original_request = original_lattice_worker::OriginalLatticeRequestSeal::capture(
                &request,
                original_request_scope.expect("actual memo context"),
                ctx.registry,
            );
            request.original_worker_request = Some(&original_request);
            let mut fu = memo(&request);
            fu.original_lattice_callback_admission = Some(
                LatticeRequest::accepts_original_worker_result(&request, &fu, ctx.registry),
            );
            crate::lattice_rebase::rebase_function_unit(&mut fu, i64::from(body_offset));
            let matches = fu.cfg == *cfg;
            #[cfg(debug_assertions)]
            if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_FUNCTION_LATTICE").is_some() {
                eprintln!(
                    "ORIGINAL_FUNCTION_LATTICE_ACCEPT qname={qname} event={} cfg_matches={matches}",
                    event_body.is_some()
                );
            }
            (matches && !fu.sccp.reads_module).then_some(fu)
        }
        _ => None,
    }
}

/// The module's procedures with their transfer summaries
/// ([`crate::interprocedural::ModuleProcedures`]), over the unit's lowering,
/// its prepared frame and global-write facts, and its command trust.
pub(super) fn module_procedures<'a>(
    (ir_module, cfg_module, _prepared): (&'a IrModule, &'a CfgModule, &'a PreparedCfgContext),
    (mutations, projection): (
        &'a crate::command_binding::ModuleCommandMutations,
        &'a crate::command_binding::ProcBindingTrustProjection,
    ),
    analysis_context: &'a crate::value_transfer::AnalysisContextKey,
    (registry, config): (&'a CommandRegistry, tcl_lexer::LexerConfig),
) -> crate::interprocedural::ModuleProcedures<'a> {
    crate::interprocedural::ModuleProcedures::new(crate::interprocedural::ModuleInputs {
        ir: ir_module,
        cfg: cfg_module,
        registry,
        mutations,
        projection,
        trace: ModuleTraceFacts::of(ir_module),
        config,
        analysis_context,
    })
}

fn retained_procedure_event_body(
    ctx: &ProcedureBuildContext<'_>,
    id: &crate::ir::ProcedureId,
    proc: Option<&crate::ir::Procedure>,
    config: tcl_lexer::LexerConfig,
) -> Option<Arc<crate::ir::SourceIrulesEventBody>> {
    ctx.ir_module
        .irules_event_bodies
        .get(id)
        .filter(|event| {
            proc.is_some_and(|procedure| {
                event.owns_procedure(procedure, &ctx.ir_module.source, config, ctx.registry)
            })
        })
        .cloned()
}

fn procedure_is_complexity_guarded(proc: Option<&crate::ir::Procedure>, cfg: &CfgFunction) -> bool {
    let body_bytes = proc.map_or(0usize, |p| {
        p.span.end().saturating_sub(p.span.start()) as usize
    });
    body_bytes > crate::ssa::DEEP_ANALYSIS_BODY_BYTES || crate::ssa::is_complexity_guarded(cfg)
}

fn retain_procedure_source_analysis(
    unit: FunctionUnit,
    context: &ProcedureBuildContext<'_>,
    procedure: Option<&crate::ir::Procedure>,
) -> FunctionUnit {
    unit.with_retained_semantic_analysis(
        context.registry,
        procedure.map(|procedure| &procedure.body),
        crate::dispatch_proof::DispatchEntryAssumption::UnknownWorld,
    )
}

struct ProcedureBuildInfo<'a> {
    id: &'a crate::ir::ProcedureId,
    pub(super) cfg: &'a CfgFunction,
    pub(super) params: &'a [String],
    proc: Option<&'a crate::ir::Procedure>,
    pub(super) qname: &'a str,
    pub(super) event_body: Option<Arc<crate::ir::SourceIrulesEventBody>>,
    pub(super) body_offset: u32,
    has_instance_global_writes: bool,
}

enum PreparedProcedureBuild<'a> {
    Guarded(crate::ir::ProcedureId, Box<FunctionUnit>),
    Ready(ProcedureBuildInfo<'a>),
}

fn prepare_procedure_build<'a>(
    ctx: &'a ProcedureBuildContext<'a>,
    id: &'a crate::ir::ProcedureId,
    cfg: &'a CfgFunction,
    config: tcl_lexer::LexerConfig,
) -> PreparedProcedureBuild<'a> {
    let params = ctx
        .ir_module
        .procedures
        .get(id)
        .map_or(&[][..], |p| p.params.as_slice());
    let proc = ctx.ir_module.procedures.get(id);
    let qname = cfg.name.as_str();
    let event_body = retained_procedure_event_body(ctx, id, proc, config);
    if procedure_is_complexity_guarded(proc, cfg) {
        let mut unit = FunctionUnit::trivial_guarded(qname, cfg.clone()).with_source_metadata(
            module_source_metadata_input(ctx.ir_module, ctx.registry),
            config,
        );
        unit.irules_event_body = event_body;
        return PreparedProcedureBuild::Guarded(*id, Box::new(unit.with_body_id(*id)));
    }
    let body_offset = proc.map_or(0, |p| p.span.start());
    let has_instance_global_writes = ctx
        .tainted_global_writes
        .get(id)
        .is_some_and(|writes| !writes.is_empty());
    PreparedProcedureBuild::Ready(ProcedureBuildInfo {
        id,
        cfg,
        params,
        proc,
        qname,
        event_body,
        body_offset,
        has_instance_global_writes,
    })
}

/// Build one [`FunctionUnit`] per procedure from a generic entry: a formal is
/// unknown inside the body whatever this file's calls pass, since the
/// procedure stays callable from a later `source`, a callback or another
/// file. The build goes through the per-procedure lattice memo when one is
/// available, and both are skipped for an oversized body.
pub(super) fn build_procedure_units(
    ctx: &ProcedureBuildContext<'_>,
    mut cache: Option<&mut ProcLatticeCache<'_>>,
    config: tcl_lexer::LexerConfig,
) -> HashMap<crate::ir::ProcedureId, FunctionUnit> {
    let original_request_scope = cache.as_ref().and(ctx.cfg_context).map(|context| {
        original_lattice_worker::OriginalLatticeRequestScope::capture(ctx.ir_module, context)
    });
    // Procedure memo bodies are relocated to offset zero. Preserve the exact
    // lexical policy while stripping only source-coordinate bookkeeping so a
    // moved but unchanged body keeps the same cache identity.
    let memo_config = config.nested().normalized();
    let memo_input = module_source_metadata_input(ctx.ir_module, ctx.registry)
        .map(crate::analyser::ResolvedAnalysisInput::for_nested_source);
    let mut procedures: HashMap<crate::ir::ProcedureId, FunctionUnit> = HashMap::new();
    for (id, cfg) in &ctx.cfg_module.procedures {
        let build = match prepare_procedure_build(ctx, id, cfg, config) {
            PreparedProcedureBuild::Guarded(id, unit) => {
                procedures.insert(id, *unit);
                continue;
            }
            PreparedProcedureBuild::Ready(build) => build,
        };
        let ProcedureBuildInfo {
            id,
            cfg,
            params,
            proc,
            qname,
            event_body,
            body_offset,
            has_instance_global_writes,
        } = build;
        let memoised = memoized_procedure_unit(
            &mut cache,
            ProcedureMemoInputs {
                ctx,
                id,
                cfg,
                proc,
                params,
                qname,
                event_body: event_body.as_ref(),
                original_request_scope: original_request_scope.as_ref(),
                memo_config,
                memo_input: memo_input.as_ref(),
                body_offset,
            },
            has_instance_global_writes,
        );
        let mut fu = memoised.unwrap_or_else(|| {
            FunctionUnit::build_procedure_in_module(
                (
                    *id,
                    proc.is_some_and(|body| body.body_kind == crate::ir::ProcedureBodyKind::Event),
                    qname,
                ),
                cfg.clone(),
                (params, None),
                UnitDialect {
                    registry: ctx.registry,
                    config,
                    source_metadata_input: module_source_metadata_input(
                        ctx.ir_module,
                        ctx.registry,
                    ),
                },
                ctx.known_class_set,
                ctx.module_facts(),
                ctx.procedures,
            )
        });
        // A memoised unit carries an offset-0 executable sidecar. Rebuild the
        // source-bearing portion after the normal CFG/SSA rebase so retained
        // word and provenance spans match this procedure's real source
        // position. Structural node identities and registry facts remain
        // position-independent; only source coordinates are refreshed.
        // Procedure bodies run only after arbitrary interposed top-level and
        // cross-file history, so their dispatch proofs start from an unknown
        // world until a workspace-aware entry contract exists.
        fu.irules_event_body = event_body;
        fu = retain_procedure_source_analysis(fu, ctx, proc);
        procedures.insert(*id, fu.with_body_id(*id));
    }
    procedures
}

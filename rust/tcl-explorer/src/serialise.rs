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

#[cfg(test)]
use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

#[cfg(test)]
use tcl_compiler::cfg::Terminator;
#[cfg(test)]
use tcl_compiler::executable_ir::InvocationResolution;
use tcl_compiler::executable_ir::OwnedInvocationResolutionUnresolved;
use tcl_compiler::gvn::{
    find_loop_invariants_for_cu, find_partial_redundancies_for_cu, find_redundancies_for_cu,
};
#[cfg(test)]
use tcl_compiler::interprocedural::InterproceduralAnalysis;
use tcl_compiler::optimiser::{apply_optimisations, find_dead_stores, optimise_unit};
use tcl_compiler::semantic_optimisation::SemanticOptimisationConfig;
#[cfg(test)]
use tcl_compiler::semantic_optimisation::SemanticOptimisationPassId;
use tcl_compiler::shimmer::{
    find_byte_array_warnings_for_cu, find_shimmer_warnings_for_cu, find_thunking_warnings_for_cu,
};
#[cfg(test)]
use tcl_compiler::state_ssa::adapters::WorldRegionKind;
use tcl_compiler::taint::taint_warning_report_for_cu;
use tcl_lexer::{LexerConfig, LineIndex, Span};
// See the note in `lib.rs`: the explorer resolves against the active pack set.

use crate::ExplorerResult;
use crate::formatters::range_dict;
use crate::views::{Severity, VIEW_META};

mod annotations;
mod cfg;
mod def_use;
mod executable_semantics;
mod front_end;
mod interprocedural;
mod irules;
mod optimiser;
mod value_lattices;
mod warnings;
mod wasm;
mod world_ssa;

use annotations::serialise_annotations;
#[cfg(test)]
use cfg::{block_successors, terminator_dict};
pub use cfg::{
    serialise_cfg_post_ssa, serialise_cfg_pre_ssa, serialise_dominators, serialise_loops,
};
pub use def_use::{serialise_dataflow, serialise_liveness};
#[cfg(test)]
use executable_semantics::invocation_resolution_label;
pub use executable_semantics::serialise_semantic;
use front_end::serialise_source_map_units;
pub use front_end::{
    serialise_ir, serialise_segments, serialise_source_map, serialise_structural_index,
};
pub use interprocedural::{serialise_interproc, serialise_unit_scope};
use irules::irules_flow_warnings;
pub use irules::{
    serialise_connection_scope, serialise_event_order, serialise_event_order_for_result,
    serialise_irules_flow,
};
pub use optimiser::{serialise_gvn, serialise_optimisations, serialise_optimiser_passes};
pub use value_lattices::{
    serialise_intervals, serialise_rendered_properties, serialise_sccp, serialise_taint_tracking,
    serialise_types,
};
pub use warnings::{serialise_bounds, serialise_shimmer, serialise_taint};
use warnings::{serialise_taint_replays, taint_replay_status};
pub use wasm::serialise_semantic_optimisations;
use wasm::{serialise_wasm_with_options, wasm_codegen_plan};
use world_ssa::serialise_world_ssa;
#[cfg(test)]
use world_ssa::{registry_world_domain_name, world_domain_name};

/// Checked readonly source inputs from the actual compilation producer.
/// Display labels cannot select a command store, profile or source owner.
pub(crate) struct RetainedExplorerSource<'a> {
    registry: &'a tcl_registry::CommandRegistry,
    input: &'a tcl_compiler::analyser::ResolvedAnalysisInput,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    config: LexerConfig,
    realm: std::sync::Arc<tcl_compiler::realm::CommandBindingRealm>,
}

impl<'a> RetainedExplorerSource<'a> {
    pub(crate) const fn registry(&self) -> &'a tcl_registry::CommandRegistry {
        self.registry
    }
}

pub(crate) fn retained_explorer_source<'a>(
    result: &'a ExplorerResult,
    source: &str,
) -> Option<RetainedExplorerSource<'a>> {
    // naming.source.retained-explorer-consumers
    // docs/design/analysis/name-resolution-proofs/retained-explorer-consumers.md
    let module = &result.unit.ir_module;
    if source != result.source
        || source != result.unit.source
        || module.source != tcl_lexer::SourceImage::document(source)
    {
        return None;
    }
    let registry = module.registry_snapshot.as_ref()?.registry();
    let realm = tcl_compiler::realm::document_realm_bindings_for_module(module, registry)?;
    let input = module.source_metadata_input.as_ref()?;
    tcl_compiler::registry_invocation::InvocationMetadataContext::for_module(registry, module)?;
    if input.lexer_config() != module.lexer_config || !realm.matches_resolved_analysis_input(input)
    {
        return None;
    }
    Some(RetainedExplorerSource {
        registry,
        input,
        dialect: module.dialect_profile,
        config: module.lexer_config,
        realm,
    })
}

/// A rewritten image is a separate source ingress. It retains the actual
/// metadata/store/grammar, but receives no execution receipt from the old image.
fn rewritten_explorer_result(
    source: &str,
    dialect_label: &str,
    retained: &RetainedExplorerSource<'_>,
) -> ExplorerResult {
    let profile = retained.input.unit_profile();
    let declared =
        tcl_compiler::analyser::utils::document_declared_surface(source, None, profile.name);
    let unit = tcl_compiler::compilation_unit::CompilationUnit::build_with_analysis_input(
        source,
        tcl_compiler::compilation_unit::UnitBuildOptions {
            registry: retained.registry,
            defer_top_level: false,
            config: retained.config,
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: Some(&declared),
        },
        None,
        retained.input,
    )
    .with_interprocedural(retained.registry, Some(profile))
    .with_retained_memory_ssa(retained.registry)
    .with_retained_deep_semantic_analysis(retained.registry);
    ExplorerResult {
        source: source.to_owned(),
        dialect: dialect_label.to_owned(),
        unit,
    }
}

/// Serialise the `meta` view: dialect list, view-tab table, and the
/// severity vocabulary.
///
/// Dialects are the selectable environments in selectable order, each carrying
/// its labels (`display_name` for menus, `short_name` for toolbars) exactly
/// like the `views` entries carry theirs, so no GUI consumer needs its own
/// name table.
#[must_use]
pub fn serialise_meta() -> Value {
    let dialects: Vec<Value> = tcl_registry::model::selectable_environments()
        .iter()
        .map(|environment| {
            json!({
                "name": environment.id.as_str(),
                "displayName": environment.display_name.as_ref(),
                "shortName": environment.short_name.as_ref(),
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
        // before the first compile — the same reason `dialects` is here.
        // The per-result `semanticOptimisations` view carries the same rows
        // plus the state the shown module was built with.
        "semanticOptimisations": serialise_semantic_optimisations(
            SemanticOptimisationConfig::new(),
        ),
    })
}

/// `range_dict` for an optional span, emitting `null` when absent
fn range_or_null(span: Option<Span>, li: &LineIndex, source: &str) -> Value {
    span.map_or(Value::Null, |s| range_dict(s, li, source))
}

fn root_body_key(result: &ExplorerResult) -> String {
    format!(
        "{}:root",
        result.unit.ir_module.procedure_ids.reporting_scope()
    )
}

fn snapshot_body_key(
    result: &ExplorerResult,
    snapshot: &crate::FunctionSnapshot<'_>,
) -> Option<String> {
    snapshot
        .unit
        .body_id
        .map(tcl_compiler::ir::ProcedureId::reporting_key)
        .or_else(|| {
            std::ptr::eq(snapshot.unit, &raw const result.unit.top_level)
                .then(|| root_body_key(result))
        })
}

fn unproved_implementation_lookup(reason: &OwnedInvocationResolutionUnresolved) -> Value {
    match reason {
        OwnedInvocationResolutionUnresolved::UnprovedNativeImplementationLookup { lookup } => {
            json!({
                "ensemble": lookup.ensemble, "member": lookup.member, "slot": lookup.slot,
                "command": lookup.command, "prepended": lookup.prepended,
            })
        }
        OwnedInvocationResolutionUnresolved::ComputedHead { .. }
        | OwnedInvocationResolutionUnresolved::UnknownLiteralHead { .. }
        | OwnedInvocationResolutionUnresolved::UnprovedBinding { .. } => Value::Null,
    }
}

/// Build the optimised-source explorer result, or `None` when the
/// optimiser leaves the source unchanged. Shared by the `optimisedSource`,
/// `irOptimised`, and `cfgPreSsaOptimised` keys (the "double pipeline" —
/// re-run the whole pipeline on the rewritten source). Returns the result
/// plus the optimised source string (whose ranges the optimised views
/// index into).
fn optimised_result(result: &ExplorerResult) -> Option<(crate::ExplorerResult, String)> {
    let retained = retained_explorer_source(result, &result.source)?;
    let optimised = apply_optimisations(
        &result.source,
        &optimise_unit(&result.unit, retained.registry, retained.dialect),
    );
    if optimised == result.source {
        return None;
    }
    let unit = rewritten_explorer_result(&optimised, &result.dialect, &retained);
    Some((unit, optimised))
}

/// Serialise the `stats` summary.
///
/// `deadStores` counts the optimiser's **O109** dead stores (there is no
/// standalone liveness pass) and the warning counts come from the Rust
/// analyses. `dataflow*` counts are omitted — `dataflow` is not implemented,
/// and such counts only apply when a dataflow graph is present.
fn serialise_stats(result: &ExplorerResult) -> Value {
    let Some(retained) = retained_explorer_source(result, &result.source) else {
        return Value::Null;
    };
    let registry = retained.registry;
    let dialect = retained.dialect;

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
    let taint = taint_warning_report_for_cu(&result.unit, registry, dialect);
    let rewrites = optimise_unit(&result.unit, registry, dialect).len();
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
        "taintWarnings": taint.warnings.len(),
        "taintReplayScope": "command-heads-and-stored-callbacks",
        "taintReplayStatus": taint_replay_status(&taint.replays),
        "taintReplayComplete": taint.replays.is_complete(),
        "irulesFlowWarnings": irules_flow_warnings(result, &retained).len(),
    })
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
    let options = tcl_codegen::codegen::wasm::WasmCompileOptions::hosted()
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
    // complete configuration comes from the actual lowered Module, retaining
    // independently supplied axes rather than resolving its display label.
    let lexer_config = result.unit.ir_module.lexer_config;
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
        serialise_structural_index(
            &result.source,
            &li,
            &result.unit.ir_module.source,
            lexer_config,
        ),
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
    out.insert(
        "taintReplays".to_owned(),
        serialise_taint_replays(result, &li, &result.source),
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
        serialise_event_order_for_result(result, &li),
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
    let wasm = serialise_wasm_with_options(result, options);
    out.insert("aot".to_owned(), wasm_codegen_plan(&wasm));
    out.insert("wasm".to_owned(), wasm);
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
mod tests;

#[cfg(test)]
mod original_configured_index_tests;

#[cfg(test)]
mod retained_explorer_source_tests;

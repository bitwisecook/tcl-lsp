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

//! Call sites another file contributes, resolved against this unit's procedures.

use super::call_recording::unresolved_command_handler;
use super::cfg_scan::scan_cfg_callers;
use super::extra_scan_contexts::{
    build_extra_call_site_scan_contexts, needs_extra_call_site_scan_contexts,
};
use super::scope_var_facts::collect_module_scope_var_facts;
use super::{
    CallSiteEvidence, CallSiteScanCtx, ExtraCallSiteScanContext, SourceCallSiteEntry,
    call_site_scan_functions, record_future_callers, record_runtime_selected_extra_callers,
    run_to_fixpoint,
};
use crate::cfg::CfgModule;
use crate::ir::Module as IrModule;
use std::collections::HashSet;
use tcl_registry::CommandRegistry;

/// Collect the call sites **another** file contributes, resolved against the
/// whole project's procedure names.
///
/// The cross-file half: a plain library file with no
/// `package provide` is `source`d by a file that calls its procs with a
/// different literal, and the library's own compilation unit — single-source
/// by construction — can never see that caller.  A host with a workspace view
/// (the LSP; the `tcl` CLI when given several files) runs this over every
/// other file and hands the result to
/// [`crate::compilation_unit::CompilationUnit`]'s build, which merges it into
/// the in-unit evidence before seeding.
///
/// `known` is the **project-wide** set of procedure qualified names whose
/// evidence the caller wants to collect. It does not certify their runtime
/// bindings, immutable bodies, or compiler-hook registration. Exact evidence
/// also requires a reached source-owned definition or actual runtime entry;
/// a name-only external declaration remains uncertain. `declared` is the **scanned file's
/// own** declaration set: a stub binds the file it is written in, so the
/// argument roles that decide what counts as a call site here are that
/// file's, not the host's.  The scan is deliberately the same
/// lowering → CFG → [`record_call_site_evidence`] path the in-unit scan takes
/// (including its `TclOO` method / `apply` / `namespace eval` body-unit
/// callers and its `ArgRole::Body` recursion), so cross-file evidence is
/// exactly what this unit would have collected had the two files been one.
#[must_use]
pub fn scan_source_call_sites<S: std::hash::BuildHasher>(
    source: &str,
    registry: &CommandRegistry,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
    dialect: &'static tcl_dialect::DialectProfile,
    known: &HashSet<String, S>,
    dispatch_reach: &[String],
) -> CallSiteEvidence {
    scan_source_call_sites_with_entry(
        source,
        registry,
        declared,
        dialect,
        known,
        dispatch_reach,
        SourceCallSiteEntry {
            entry: None,
            input: None,
        },
    )
}

/// Collect cross-file caller evidence with the driver's actual source entry.
/// Loaded provider and source implementation identity are retained throughout
/// lowering and CFG construction; a known procedure name alone grants no proof.
#[must_use]
pub fn scan_source_call_sites_with_source_entry<S: std::hash::BuildHasher>(
    source: &str,
    registry: &CommandRegistry,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
    dialect: &'static tcl_dialect::DialectProfile,
    known: &HashSet<String, S>,
    dispatch_reach: &[String],
    entry: &crate::command_binding::SourceAnalysisEntry,
) -> CallSiteEvidence {
    scan_source_call_sites_with_entry(
        source,
        registry,
        declared,
        dialect,
        known,
        dispatch_reach,
        SourceCallSiteEntry {
            entry: Some(entry),
            input: None,
        },
    )
}

/// Current external source declarations; these receipts grant no loaded provider.
#[derive(Debug, Clone)]
pub struct ExternalSourceProcedureFacts {
    input: crate::analyser::ResolvedAnalysisInput,
    image: tcl_lexer::SourceImage,
    declarations: Vec<
        crate::signature_scan::original_name::SourceDeclarationMetadata<crate::analyser::ProcDef>,
    >,
}

impl ExternalSourceProcedureFacts {
    /// Retain only original declarations from this complete input and source image.
    #[must_use]
    pub fn capture(
        input: &crate::analyser::ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        declarations: &[crate::signature_scan::original_name::SourceDeclarationMetadata<
            crate::analyser::ProcDef,
        >],
    ) -> Option<Self> {
        let context = input.context_registry();
        crate::registry_invocation::InvocationMetadataContext::for_source_input(
            context.commands(),
            input,
            input.lexer_config(),
            Some(input.unit_profile()),
        )?;
        if declarations.iter().any(|declaration| {
            declaration.name_input().source_image() != image
                || declaration.name_input().lexer_config() != input.lexer_config()
        }) {
            return None;
        }
        Some(Self {
            input: input.clone(),
            image: image.clone(),
            declarations: declarations.to_vec(),
        })
    }
}

/// Scan the actual source entry while preserving independently supplied external declarations.
/// External metadata only withdraws seeds; it never installs a command or proves argv.
#[must_use]
pub fn scan_source_call_sites_with_original_source<S: std::hash::BuildHasher>(
    source: &str,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
    known: &HashSet<String, S>,
    dispatch_reach: &[String],
    input: &crate::analyser::ResolvedAnalysisInput,
    entry: Option<&crate::command_binding::SourceAnalysisEntry>,
    external: &[ExternalSourceProcedureFacts],
) -> CallSiteEvidence {
    let context = input.context_registry();
    let mut evidence = scan_source_call_sites_with_entry(
        source,
        context.commands(),
        declared,
        input.unit_profile(),
        known,
        dispatch_reach,
        SourceCallSiteEntry {
            entry,
            input: Some(input),
        },
    );
    for facts in external {
        for declaration in &facts.declarations {
            if declaration.name_input().source_image() != &facts.image
                || declaration.name_input().lexer_config() != facts.input.lexer_config()
            {
                continue;
            }
            // A declaration cannot prove that this interpreter loaded its provider.
            // Current external reach therefore remains an opaque caller obligation.
            if known.contains(&declaration.metadata().qualified_name) {
                evidence
                    .external_opaque_declarations
                    .insert(declaration.declaration_site().clone());
            }
        }
    }
    evidence
}

/// Collect caller evidence with complete retained source metadata and the
/// independent driver entry. Missing metadata in the entry-only API retracts
/// seeds; this supplied input never grants execution or a loaded provider.
#[must_use]
pub fn scan_source_call_sites_with_source_input<S: std::hash::BuildHasher>(
    source: &str,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
    known: &HashSet<String, S>,
    dispatch_reach: &[String],
    input: &crate::analyser::ResolvedAnalysisInput,
    entry: &crate::command_binding::SourceAnalysisEntry,
) -> CallSiteEvidence {
    let context = input.context_registry();
    scan_source_call_sites_with_entry(
        source,
        context.commands(),
        declared,
        input.unit_profile(),
        known,
        dispatch_reach,
        SourceCallSiteEntry {
            entry: Some(entry),
            input: Some(input),
        },
    )
}

pub(super) fn lower_source_callers(
    source: &str,
    registry: &CommandRegistry,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
    dialect: &'static tcl_dialect::DialectProfile,
    supplied: SourceCallSiteEntry<'_>,
) -> (IrModule, CfgModule, Vec<ExtraCallSiteScanContext>) {
    let config = supplied.input.map_or_else(
        || tcl_lexer::LexerConfig::from_grammar(dialect.grammar),
        crate::analyser::ResolvedAnalysisInput::lexer_config,
    );
    let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
        .with_dialect(Some(dialect))
        .with_declared_commands(declared);
    if let Some(input) = supplied.input {
        lowerer = lowerer.with_resolved_analysis_input(input.clone());
    }
    if let Some(entry) = supplied.entry {
        lowerer.set_source_analysis_options(entry.options());
    }
    let mut ir_module = crate::lowering::lower_to_ir_with(lowerer, source);
    crate::specialise_factories::specialise_factories(&mut ir_module, registry);
    crate::inline_uplevel::inline_uplevel_passthrough(&mut ir_module, registry);
    let prepared_cfg_context = crate::cfg_builder::prepare_cfg_context_bundle(&ir_module, registry);
    let config = ir_module.lexer_config;
    let cfg_module = crate::cfg_builder::build_cfg_with_registry_and_context(
        &ir_module,
        false,
        registry,
        &prepared_cfg_context,
        config,
    );
    let cfg_context =
        needs_extra_call_site_scan_contexts(&ir_module).then_some(prepared_cfg_context);
    let extra =
        build_extra_call_site_scan_contexts(&ir_module, cfg_context.as_ref(), registry, config);
    (ir_module, cfg_module, extra)
}

fn scan_source_call_sites_with_entry<S: std::hash::BuildHasher>(
    source: &str,
    registry: &CommandRegistry,
    declared: Option<&tcl_registry::model::DeclaredSurface>,
    dialect: &'static tcl_dialect::DialectProfile,
    known: &HashSet<String, S>,
    dispatch_reach: &[String],
    supplied: SourceCallSiteEntry<'_>,
) -> CallSiteEvidence {
    let mut out = CallSiteEvidence::default();
    if known.is_empty() {
        return out;
    }
    // The explicit ingress profile owns native semantics as well as parsing.
    // A declaration catalogue without a profile cannot prove native dispatch.
    let projected;
    let registry = if registry.profile().is_none() && supplied.input.is_none() {
        projected = registry.project_for_profile(dialect);
        &projected
    } else {
        registry
    };
    let (ir_module, cfg_module, extra) =
        lower_source_callers(source, registry, declared, dialect, supplied);
    // The cross-file scan resolves a dispatch word exactly as the in-unit one
    // does — `scan_cfg_callers`/`record_call_site_evidence` are shared, so a
    // `set cmd helper; $cmd dev` in *another* file retracts this unit's seed
    // just as an in-unit one would.  Without the same var facts and fixpoint
    // here, an unreadable dispatch would stop retracting seeds across the
    // file boundary.
    // What this file's own unreadable dispatches can reach is a *project*
    // fact, not a file one — `source` puts two files in one interpreter in
    // both directions, so a library reaches its sourcer's procedures just as
    // its sourcer reaches the library's.  Only the host knows that graph, so
    // it supplies the set (`tcl_lsp_db::file_dispatch_reach`); with none
    // given, the file's own declarations are the honest bound.
    let reach: Vec<String> = if dispatch_reach.is_empty() {
        let mut own: Vec<String> = ir_module
            .procedures
            .values()
            .map(|procedure| procedure.qualified_name.clone())
            .collect();
        own.sort();
        own
    } else {
        dispatch_reach.to_vec()
    };
    // `known` here is the *project*-wide procedure set, so a handler defined
    // in any scanned file is visible — matching Tcl, where `::unknown` is one
    // command shared by the whole interpreter, not a per-file one.
    let unresolved_handler = unresolved_command_handler(registry, known);
    // naming.source.retained-positioned-command-realm
    // docs/design/analysis/name-resolution-proofs/retained-positioned-command-realm.md
    // Lowering owns this exact independent source ingress. Query that owner;
    // refusal retracts reachable caller seeds instead of replaying its text.
    let Some(identities) = crate::realm::document_realm_bindings_for_module(&ir_module, registry)
    else {
        out.record_unenumerable_caller(&reach);
        return out;
    };
    let var_facts = collect_module_scope_var_facts(
        &cfg_module,
        &extra,
        &tcl_registry::model::DocumentCommandSurface::new(registry, declared),
        &ir_module,
        &identities,
    );
    out = run_to_fixpoint(|previous| {
        let ctx = CallSiteScanCtx {
            known,
            registry,
            module: &ir_module,
            source_input: crate::registry_invocation::InvocationMetadataContext::for_module(
                registry, &ir_module,
            )
            .and_then(crate::registry_invocation::InvocationMetadataContext::source_analysis_input),
            identities: &identities,
            declared,
            var_facts: &var_facts,
            previous,
            procedures: &ir_module.procedures,
            unenumerable_reach: &reach,
            unresolved_handler,
        };
        let funcs = call_site_scan_functions(&cfg_module, &extra);
        let mut round = CallSiteEvidence::default();
        record_future_callers(&mut round, &ctx, &ir_module.future_call_sites);
        record_runtime_selected_extra_callers(&mut round, &ctx, &extra);
        scan_cfg_callers(&mut round, &ctx, funcs);
        round
    });

    out
}

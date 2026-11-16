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

use crate::interprocedural::test_body_identity::{fixture_id, fixture_id_if_present};

use super::*;
use crate::cfg_builder::build_cfg;

mod alias_effects;
mod call_recording;
mod nested_callers;
mod original_argument_callers;
mod original_cfg_callers;
mod param_seeds;
mod scope_var_facts;
mod source_scan;
mod unit_linkage;
mod unresolved_dispatch;

/// The Tcl 8.6 catalogue: a fixture that defines a procedure names a release.
fn registry() -> CommandRegistry {
    CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").expect("Tcl 8.6"))
}

fn retained_scope_fixture(source: &str, registry: &CommandRegistry) -> IrModule {
    let profile = registry
        .profile()
        .unwrap_or_else(tcl_dialect::DialectProfile::plain_tcl);
    let context = std::sync::Arc::new(
        crate::environment_ingress::context_for_profile(profile)
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    crate::lowering::lower_to_ir_with(
        crate::lowering::Lowerer::with_config(registry, config)
            .with_dialect(Some(profile))
            .with_resolved_analysis_input(crate::analyser::ResolvedAnalysisInput::new(
                profile, profile, context, config,
            )),
        source,
    )
}

fn surface(reg: &CommandRegistry) -> tcl_registry::model::DocumentCommandSurface<'_> {
    tcl_registry::model::DocumentCommandSurface::new(reg, None)
}

fn known(names: &[&str]) -> HashSet<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

fn params(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

fn seed(
    params: &[String],
    evidence: &CallSiteEvidence,
    qname: &str,
    linkage: Traits,
    has_cross_file_evidence: bool,
) -> Option<HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>> {
    let proc_binding_trust = crate::command_binding::ProcBindingTrustProjection::default();
    params_constants_from_call_sites(
        params,
        evidence,
        qname,
        &UnitCallerView {
            linkage,
            has_cross_file_evidence,
            proc_binding_trust: &proc_binding_trust,
            source: None,
            entered: None,
        },
    )
}

fn source_loader_entry(text: &str) -> crate::command_binding::SourceAnalysisEntry {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("native profile");
    crate::command_binding::SourceAnalysisEntry {
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        trusted_source_modules: vec![crate::command_binding::TrustedSourceModuleLoader::new(
            tcl_dialect::model::Family::Tcl,
            "library.tcl".into(),
            None,
            "workspace/library/revision-one".into(),
            &std::sync::Arc::from(text),
        )],
        ..crate::command_binding::SourceAnalysisEntry::default()
    }
}

fn scan_loaded_caller(
    source: &str,
    entry: &crate::command_binding::SourceAnalysisEntry,
) -> CallSiteEvidence {
    let registry = CommandRegistry::build_default();
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("native profile");
    let known = HashSet::from(["::helper".to_owned()]);
    let context = std::sync::Arc::new(
        crate::environment_ingress::context_for_profile(profile)
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );
    scan_source_call_sites_with_source_input(source, None, &known, &[], &input, entry)
}

/// Build the evidence for `src` under a named dialect's registry, the
/// way `CompilationUnit` does minus the method / body-unit callers
/// (which need a CFG context these unit tests do not exercise).
fn evidence_for_dialect(
    src: &str,
    dialect: &'static tcl_dialect::DialectProfile,
) -> CallSiteEvidence {
    let reg = tcl_registry::model::ingress::static_context_for_profile(dialect).commands();
    let config = tcl_lexer::LexerConfig::from_grammar(dialect.grammar);
    // The scan authenticates each statement against the module's own
    // resolved input, so the fixture lowers under one, over the same
    // command store it scans with.
    let input = crate::analyser::ResolvedAnalysisInput::new(
        dialect,
        dialect,
        std::sync::Arc::new(
            crate::environment_ingress::context_for_profile(dialect)
                .with_command_store(reg.snapshot().shared_registry()),
        ),
        config,
    );
    let ir = crate::lowering::lower_to_ir_with(
        crate::lowering::Lowerer::with_config(reg, config)
            .with_dialect(Some(dialect))
            .with_resolved_analysis_input(input),
        src,
    );
    let cfg_module = crate::cfg_builder::build_cfg(&ir, false);
    let identities = crate::realm::document_realm_bindings_with_source_entry(
        src,
        ir.lexer_config,
        reg,
        &ir.source_entry,
    );
    let identities = ir.source_metadata_input.as_ref().map_or_else(
        || identities.clone(),
        |input| {
            identities
                .clone()
                .with_resolved_analysis_input(input.clone())
        },
    );
    collect_call_site_constants(
        &cfg_module,
        &[],
        &ir.procedures,
        &ir.future_call_sites,
        CallSiteSourceContext {
            registry: reg,
            module: &ir,
            declared: None,
            input: ir.source_metadata_input.as_ref(),
            identities: &identities,
        },
    )
}

fn evidence(src: &str) -> CallSiteEvidence {
    evidence_for_dialect(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
}

/// Whether `index` still has a single agreed literal across every
/// recorded caller — the exact predicate the seed reads.  This model
/// records "a caller that never reached this position" in `arg_counts`
/// rather than in the slot, so probing the slot alone would miss it.
fn uniform(ev: &CallSiteEvidence, qname: &str, index: usize) -> Option<String> {
    ev.get(qname)?
        .uniform_literal_at(index)
        .map(ToOwned::to_owned)
}

/// The literal values recorded at `index` for `qname`, sorted, plus
/// whether that position is poisoned.
fn slot(ev: &CallSiteEvidence, qname: &str, index: usize) -> (Vec<String>, bool) {
    let Some(consts) = ev.get(qname).and_then(|e| e.slots.get(&index)) else {
        return (Vec::new(), false);
    };
    let mut values: Vec<String> = consts.values.iter().cloned().collect();
    values.sort();
    (values, consts.unknown)
}

fn supplied_unit_fixture(
    source: &str,
    context: &std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> IrModule {
    let registry = context.commands();
    crate::lowering::lower_to_ir_with(
        crate::lowering::Lowerer::with_config(
            registry,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
        )
        .with_dialect(registry.profile())
        .with_context_registry(std::sync::Arc::clone(context)),
        source,
    )
}

fn supplied_unit_input(
    current: &crate::analyser::ResolvedAnalysisInput,
    context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
) -> crate::analyser::ResolvedAnalysisInput {
    crate::analyser::ResolvedAnalysisInput::new(
        current.analyser_profile(),
        current.unit_profile(),
        context,
        current.lexer_config(),
    )
}

fn native_argument_caller_unit(
    source: &str,
) -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    crate::environment_ingress::RetainedNativeUnit,
) {
    let environment = tcl_registry::model::ingress::resolve_known_environment("tcl9.0").unwrap();
    let context = environment.default_context_registry();
    let profile = environment.unit_profile();
    let (owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(native)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_profile(Some(profile)),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        Some(&entry),
        std::sync::Arc::clone(&context),
    );
    (
        context,
        crate::environment_ingress::RetainedNativeUnit::new(unit, owner),
    )
}

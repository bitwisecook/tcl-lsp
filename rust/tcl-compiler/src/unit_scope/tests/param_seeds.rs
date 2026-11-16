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

//! The parameter-constant seed: formal layouts, omitted and trailing arguments, and boundary gating.

use super::*;

fn native_seed(
    params_raw: &str,
    grammar: Option<tcl_dialect::ParameterGrammar>,
    evidence: &CallSiteEvidence,
) -> Option<HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>> {
    let procedure = crate::ir::Procedure {
        name: "helper".into(),
        qualified_name: "::helper".into(),
        publication: None,
        declaration_site: None,
        body_kind: crate::ir::ProcedureBodyKind::Named,
        params: Vec::new(),
        span: tcl_lexer::Span::new(0, 0),
        body: crate::ir::Script::default(),
        params_raw: params_raw.into(),
        body_source: None,
        body_offset: 0,
        namespace_scoped: false,
        base_priority: 500,
    };
    let mut module = IrModule::default();
    let id = module.retain_procedure(procedure);
    let mut evidence = evidence.clone();
    if let Some(row) = evidence.get("::helper").cloned() {
        evidence.by_body.insert(id, row);
    }
    let trust = crate::command_binding::ProcBindingTrustProjection::default();
    params_constants_from_native_call_sites(
        &module,
        id,
        module.procedures.get(&id).expect("issued fixture body"),
        grammar,
        &evidence,
        &UnitCallerView {
            linkage: Traits::empty(),
            has_cross_file_evidence: false,
            proc_binding_trust: &trust,
            source: None,
            entered: None,
        },
    )
}

#[test]
fn native_formal_seed_distinguishes_zero_arguments_from_an_opaque_caller() {
    let mut evidence = CallSiteEvidence::default();
    evidence.record_call("::helper".into(), &[]);
    let grammar = Some(tcl_dialect::ParameterGrammar::Tcl);
    let seed = native_seed("{mode prod}", grammar, &evidence).expect("default binding");
    assert_eq!(
        seed.get(&("mode".into(), 0)),
        Some(&crate::analyses::LatticeValue::Const(
            crate::analyses::ConstValue::String("prod".into())
        ))
    );
    evidence.record_opaque_caller("::helper");
    assert!(native_seed("{mode prod}", grammar, &evidence).is_none());
}

#[test]
fn native_formal_seed_requires_a_proved_ordinary_activation_layout() {
    let mut evidence = CallSiteEvidence::default();
    evidence.record_call("::helper".into(), &["same".into(), "same".into()]);
    assert!(
        native_seed(
            "mode mode",
            Some(tcl_dialect::ParameterGrammar::Tcl),
            &evidence
        )
        .is_none()
    );
    for params in ["&mode other", "args mode", "{args rest} mode", "mode other"] {
        assert!(native_seed(params, Some(tcl_dialect::ParameterGrammar::Jim), &evidence).is_none());
    }
    assert!(native_seed("mode other", None, &evidence).is_none());
    assert!(
        native_seed(
            "mode other",
            Some(tcl_dialect::ParameterGrammar::Tcl),
            &evidence
        )
        .is_some()
    );
}

/// A call that omits a defaulted parameter binds it to its **default**, an
/// unknown value — so a literal at another call site cannot claim the
/// position. `binds_position` is what encodes that.
#[test]
fn an_omitted_argument_stops_a_position_being_uniform() {
    let reg = registry();
    let evidence = scan_source_call_sites(
        "proc helper {a {b x}} { return $a }\nhelper 1 two\nhelper 1\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::helper"]),
        &[],
    );
    let helper = evidence.get("::helper").expect("calls recorded");
    assert_eq!(helper.uniform_literal_at(0), Some("1"));
    assert_eq!(helper.arg_counts, [1, 2].into_iter().collect());
    assert!(!helper.binds_position(1));
    assert_eq!(helper.uniform_literal_at(1), None);
}

/// Only a **trailing** `args` is Tcl's variadic catch-all
/// (`TclCreateProc`, `generic/tclProc.c`) — in `proc f {args x}` the
/// first word is an ordinary parameter and both positions may be seeded.
#[test]
fn only_a_trailing_args_stops_the_seed() {
    let reg = registry();
    let evidence = scan_source_call_sites(
        "proc helper {args x} {}\nhelper one two\nhelper one two\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::helper"]),
        &[],
    );
    let leading = seed(
        &params(&["args", "x"]),
        &evidence,
        "::helper",
        Traits::empty(),
        false,
    )
    .expect("both positions seeded");
    assert_eq!(leading.len(), 2);
    let trailing = seed(
        &params(&["x", "args"]),
        &evidence,
        "::helper",
        Traits::empty(),
        false,
    )
    .expect("the leading parameter is still seeded");
    assert_eq!(trailing.len(), 1);
    assert!(trailing.contains_key(&("x".to_owned(), 0)));
}

/// A registry-declared boundary declines the seed on its own, and a
/// host-supplied cross-file view re-enables it — but only for the kind of
/// boundary that view can actually bound.
///
/// Publishing this file's commands as an API surface (`PROVIDES_PACKAGE`
/// / `EXPORTS_COMMAND`) admits callers no project scan covers — another
/// checkout can `package require` this one — so it declines outright.
/// Pulling another unit in (`LOADS_EXTERNAL_UNIT`) names a caller the
/// host's project normally *does* contain, so it defers to the evidence.
#[test]
fn boundary_gating_distinguishes_publishing_from_loading() {
    let reg = registry();
    let evidence = scan_source_call_sites(
        "proc helper {mode} {}\nhelper prod\nhelper prod\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::helper"]),
        &[],
    );
    let p = params(&["mode"]);
    assert!(
        seed(&p, &evidence, "::helper", Traits::empty(), false).is_some(),
        "no boundary at all: the visible callers are the whole story"
    );
    for boundary in [Traits::PROVIDES_PACKAGE, Traits::EXPORTS_COMMAND] {
        for has_view in [false, true] {
            assert!(
                seed(&p, &evidence, "::helper", boundary, has_view).is_none(),
                "{boundary:?} publishes to callers no enumeration bounds \
                     (cross-file view: {has_view})"
            );
        }
    }
    assert!(
        seed(
            &p,
            &evidence,
            "::helper",
            Traits::LOADS_EXTERNAL_UNIT,
            false
        )
        .is_none(),
        "a loaded unit is an unenumerated caller without a host view"
    );
    assert!(
        seed(&p, &evidence, "::helper", Traits::LOADS_EXTERNAL_UNIT, true).is_some(),
        "with the project enumerated, the loaded unit's callers are in the evidence"
    );
}

#[test]
fn an_omitted_defaulted_argument_poisons_its_slot() {
    let ev = evidence("proc helper {a {b DEFAULT}} { return $a }\nhelper one\n");
    assert_eq!(slot(&ev, "::helper", 0), (vec!["one".into()], false));
    assert_eq!(
        uniform(&ev, "::helper", 1),
        None,
        "the omitted argument leaves b bound to its default",
    );
}

fn supplied_unit_callers(
    source: &str,
    module: &IrModule,
    input: Option<&crate::analyser::ResolvedAnalysisInput>,
) -> CallSiteEvidence {
    let registry = module.resolved_registry();
    let cfg = crate::cfg_builder::build_cfg(module, false);
    let identities = crate::realm::document_realm_bindings_with_source_entry(
        source,
        module.lexer_config,
        registry,
        &module.source_entry,
    );
    let identities = input.map_or_else(
        || identities.clone(),
        |input| {
            identities
                .clone()
                .with_resolved_analysis_input(input.clone())
        },
    );
    collect_call_site_constants(
        &cfg,
        &[],
        &module.procedures,
        &module.future_call_sites,
        CallSiteSourceContext {
            registry,
            module,
            declared: None,
            input,
            identities: &identities,
        },
    )
}

#[test]
fn unit_caller_seeds_require_the_retained_metadata_owner() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Actual source-owned caller operands, not a complete Native command world.
    let context = crate::environment_ingress::context_for_profile(
        tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
    );
    let source = "proc helper {mode} {return $mode}; helper FIXED";
    let module = supplied_unit_fixture(source, &context);
    let input = module.source_metadata_input.as_ref().unwrap();
    let current = supplied_unit_callers(source, &module, Some(input));
    assert_eq!(uniform(&current, "::helper", 0).as_deref(), Some("FIXED"));
    let foreign = supplied_unit_input(
        input,
        crate::environment_ingress::context_for_profile(
            tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
        ),
    );
    for withheld in [Some(&foreign), None] {
        let evidence = supplied_unit_callers(source, &module, withheld);
        assert!(evidence.get("::helper").unwrap().opaque_caller);
        assert_eq!(uniform(&evidence, "::helper", 0), None);
    }
}

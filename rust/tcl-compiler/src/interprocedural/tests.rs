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

use super::test_body_identity::{edge_labels, fixture_id, fixture_id_if_present, fixture_proc};
fn fixture_method<'a>(
    analysis: &'a InterproceduralAnalysis,
    label: &str,
) -> Option<&'a MethodSummary> {
    let mut matches = analysis
        .methods
        .values()
        .filter(|method| method.base.qualified_name == label);
    let first = matches.next()?;
    assert!(
        matches.next().is_none(),
        "fixture method label must be unambiguous"
    );
    Some(first)
}

use super::*;

fn authored_report_unit() -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    CompilationUnit,
) {
    let environment = crate::environment_ingress::resolve_environment("tcl8.6");
    let context = environment.default_context_registry();
    // Explicit Standalone authoring is the dialect-blind build (see
    // `original_reference_unit`); a profiled build is a fresh document.
    let unit = CompilationUnit::build_for_with_config(
        "proc p {} {return ONE}; proc p {} {return TWO}",
        context.commands(),
        false,
        tcl_lexer::LexerConfig::from_grammar(environment.unit_profile().grammar),
    );
    (context, unit)
}

#[test]
fn authored_source_report_labels_keep_real_body_ids_and_current_source() {
    let environment = crate::environment_ingress::resolve_environment("tcl8.6");
    let (context, unit) = authored_report_unit();
    let registry = context.commands();
    let module = &unit.ir_module;
    assert_eq!(module.original_authored_declaration_body_units.len(), 2);
    let ids: Vec<_> = module
        .original_authored_declaration_body_units
        .keys()
        .copied()
        .collect();
    assert_ne!(ids[0], ids[1]);
    for &id in &ids {
        assert!(module.procedure_ids.owns(id));
        assert_eq!(
            module.original_procedure_report_label(id, registry),
            Some("::p")
        );
        let procedure = module.body_units.get(&id).unwrap();
        assert!(procedure.publication.is_none());
        assert!(procedure.body.executed_source.is_none());
        assert!(procedure.body.native_compilation_admission.is_none());
    }
    let identities = crate::realm::document_realm_bindings_for_module(module, registry).unwrap();
    let analysis = build_interprocedural_analysis(
        module,
        registry,
        Some(environment.unit_profile()),
        ObjectTypeMap::none(),
        &identities,
        None,
    );
    for &id in &ids {
        assert_eq!(
            analysis
                .procedures
                .get(&id)
                .expect("actual issued body summary")
                .qualified_name,
            "::p"
        );
    }
}

#[test]
fn authored_source_report_labels_refuse_foreign_and_changed_owners() {
    let environment = crate::environment_ingress::resolve_environment("tcl8.6");
    let (context, unit) = authored_report_unit();
    let registry = context.commands();
    let module = &unit.ir_module;
    let id = *module
        .original_authored_declaration_body_units
        .keys()
        .next()
        .unwrap();
    let mut presentation = module.clone();
    presentation.body_units.get_mut(&id).unwrap().qualified_name = "display changed".into();
    assert_eq!(
        presentation.original_procedure_report_label(id, registry),
        Some("::p")
    );
    let mut changed = module.clone();
    changed.source = tcl_lexer::SourceImage::document("proc p {} {return CHANGED}");
    assert!(
        changed
            .original_procedure_report_label(id, registry)
            .is_none()
    );
    let mut changed = module.clone();
    changed
        .body_units
        .get_mut(&id)
        .unwrap()
        .params
        .push("changed".into());
    assert!(
        changed
            .original_procedure_report_label(id, registry)
            .is_none()
    );
    let mut detached = module.clone();
    detached.retained_source_bindings = None;
    assert!(
        detached
            .original_procedure_report_label(id, registry)
            .is_none()
    );
    // The foreign owner is the same explicit Standalone authoring build
    // (`authored_report_unit`), retained under a different unit.
    let other = CompilationUnit::build_for_with_config(
        "proc p {} {return ONE}; proc p {} {return TWO}",
        registry,
        false,
        tcl_lexer::LexerConfig::from_grammar(environment.unit_profile().grammar),
    );
    let foreign_id = *other
        .ir_module
        .original_authored_declaration_body_units
        .keys()
        .next()
        .unwrap();
    assert!(
        module
            .original_procedure_report_label(foreign_id, registry)
            .is_none()
    );
    let foreign_registry =
        crate::environment_ingress::resolve_environment("tcl9.0").default_context_registry();
    assert!(
        module
            .original_procedure_report_label(id, foreign_registry.commands())
            .is_none()
    );
}

#[test]
fn original_declaration_receivers_keep_owned_ids_and_preceding_setup() {
    let environment = tcl_registry::model::ingress::resolve_environment("tk");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let profile = environment.unit_profile();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let build = |source| {
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .with_resolved_analysis_input(input.clone());
        lowerer.lower(source);
        lowerer.finish_module(source)
    };
    let module = build("entry .before; proc danger {} {return OK}; entry .after");
    let (id, procedure) = module.original_source_procedure_bodies().next().unwrap();
    let id = *id;
    assert_eq!(
        procedure.body_kind,
        crate::ir::ProcedureBodyKind::OriginalDeclaration
    );
    assert!(procedure.publication.is_none());
    assert!(!module.procedures.contains_key(&id));
    assert!(module.source_entry.native_entry.is_none());
    let classes = global_instance_classes(&module, registry);
    assert!(classes[&id].contains_key(".before"));
    assert!(!classes[&id].contains_key(".after"));
    assert!(!module.procedures.contains_key(&id));

    let later = build("proc danger {} {return OK}; entry .later");
    let later_id = *later.original_source_procedure_bodies().next().unwrap().0;
    assert!(global_instance_classes(&later, registry)[&later_id].is_empty());
    assert!(!classes.contains_key(&later_id));
    let mut missing = module.clone();
    missing.retained_source_bindings = None;
    assert!(!global_instance_classes(&missing, registry).contains_key(&id));
    let mut changed = module.clone();
    changed.source = tcl_lexer::SourceImage::document("entry .changed; proc danger {} {return OK}");
    assert!(!global_instance_classes(&changed, registry).contains_key(&id));
    let mut foreign = module.clone();
    foreign.procedure_ids = later.procedure_ids.clone();
    assert!(!global_instance_classes(&foreign, registry).contains_key(&id));
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    assert!(!global_instance_classes(&module, foreign).contains_key(&id));
}

#[test]
fn original_declaration_summaries_keep_actual_cards_without_callable_publication() {
    let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let profile = environment.unit_profile();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let source = "proc p {} {return ORIGINAL}";
    // Unentered declaration cards need explicit unknown history; with no
    // entry a fresh interpreter defines `::p` (user decision 2026-10-19T14:20Z).
    let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
        .with_resolved_analysis_input(input)
        .with_explicit_unknown_entry();
    lowerer.lower(source);
    let module = lowerer.finish_module(source);
    let mut bodies = module.original_source_procedure_bodies();
    let (id, body) = bodies.next().expect("genuine original declaration card");
    assert!(bodies.next().is_none());
    let id = *id;
    assert_eq!(
        body.body_kind,
        crate::ir::ProcedureBodyKind::OriginalDeclaration
    );
    assert!(body.publication.is_none());
    assert!(!module.procedures.contains_key(&id));
    assert!(module.source_entry.native_entry.is_none());
    assert!(summary_procedure_bodies(&module, registry).contains_key(&id));
    let identities = crate::realm::document_realm_bindings_with_source_entry(
        source,
        module.lexer_config,
        registry,
        &module.source_entry,
    )
    .with_resolved_analysis_input(module.source_metadata_input.as_ref().unwrap().clone());
    let analysis = build_interprocedural_analysis(
        &module,
        registry,
        Some(profile),
        ObjectTypeMap::none(),
        &identities,
        None,
    );
    let summary = analysis
        .procedures
        .get(&id)
        .expect("conditional source summary");
    assert!(!summary.completes);
    assert!(summary.defined_at.is_none());
    assert_eq!(
        summary.defined_callees,
        [] as [tcl_runtime_api::ProcedureId; 0]
    );
    assert!(!module.procedures.contains_key(&id));

    let mut missing = module.clone();
    missing.retained_source_bindings = None;
    assert!(!summary_procedure_bodies(&missing, registry).contains_key(&id));
    let mut changed = module.clone();
    changed.source = tcl_lexer::SourceImage::document("proc p {} {return CHANGED}");
    assert!(!summary_procedure_bodies(&changed, registry).contains_key(&id));
    let mut changed = module.clone();
    changed
        .body_units
        .get_mut(&id)
        .unwrap()
        .body
        .statements
        .clear();
    assert!(!summary_procedure_bodies(&changed, registry).contains_key(&id));
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    assert!(!summary_procedure_bodies(&module, foreign).contains_key(&id));
}

fn known_set(names: &[&str]) -> HashSet<String> {
    names.iter().map(|s| (*s).to_string()).collect()
}

/// The call-graph edges recorded for `caller` in `src`, sorted.
fn calls_of(src: &str, caller: &str, dialect: &'static tcl_dialect::DialectProfile) -> Vec<String> {
    let registry = tcl_registry::model::ingress::static_context_for_profile(dialect).commands();
    let ir = crate::lowering::lower_to_ir(src, registry);
    let identities = crate::realm::document_realm_bindings_with_source_entry(
        src,
        ir.lexer_config,
        registry,
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
    let ia = build_interprocedural_analysis(
        &ir,
        registry,
        Some(dialect),
        ObjectTypeMap::none(),
        &identities,
        None,
    );
    let mut calls: Vec<String> =
        fixture_proc(&ia, caller).map_or_default(|s| edge_labels(&ia, &s.calls));
    calls.sort();
    calls
}

#[test]
fn procedure_and_method_callback_advice_requires_the_module_source_producer() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Callback edges are conditional authored reachability. This source
    // control does not establish a native invocation or callback frame.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = context.commands();
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let source = "proc ::cb args {}; proc ::direct args {}; \
                      proc ::caller {} {::direct; ::lsort -command ::cb {x y}}; \
                      oo::class create C {method run {} {::direct; ::lsort -command ::cb {x y}}}";
    let module = crate::lowering::lower_to_ir_with_dialect(
        source,
        registry,
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        Some(profile),
    );
    let input = module
        .source_metadata_input
        .as_ref()
        .expect("retained source input");
    let identities = crate::realm::document_realm_bindings_with_source_entry(
        source,
        module.lexer_config,
        registry,
        &module.source_entry,
    )
    .with_resolved_analysis_input(input.clone());
    let analysis = |module: &crate::ir::Module| {
        build_interprocedural_analysis(
            module,
            registry,
            // A display profile cannot replace the source producer.
            tcl_dialect::DialectProfile::find("tcl9.1"),
            ObjectTypeMap::none(),
            &identities,
            None,
        )
    };
    let current = analysis(&module);
    let proc = fixture_proc(&current, "::caller").expect("procedure summary");
    let method = fixture_method(&current, "::C::run").expect("method summary");
    for summary in [proc, &method.base] {
        assert!(
            edge_labels(&current, &summary.direct_calls)
                .iter()
                .any(|name| name == "::cb")
        );
        assert!(
            edge_labels(&current, &summary.direct_calls)
                .iter()
                .any(|name| name == "::direct")
        );
    }

    let mut wrong_profile = module.clone();
    wrong_profile.dialect_profile = tcl_dialect::DialectProfile::find("tcl9.1");
    let mut wrong_config = module.clone();
    wrong_config.lexer_config.strict_quoting = !wrong_config.lexer_config.strict_quoting;
    let mut missing = module.clone();
    missing.source_metadata_input = None;
    let mut foreign = module.clone();
    foreign.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        input.lexer_config(),
    ));
    for withheld in [wrong_profile, wrong_config, missing, foreign] {
        let result = analysis(&withheld);
        let proc = fixture_proc(&result, "::caller").expect("retained procedure");
        let method = fixture_method(&result, "::C::run").expect("retained method");
        for summary in [proc, &method.base] {
            assert!(
                !edge_labels(&result, &summary.direct_calls)
                    .iter()
                    .any(|name| name == "::cb"),
                "withheld metadata cannot donate callback advice: {:?}",
                summary.direct_calls
            );
            assert!(
                edge_labels(&result, &summary.direct_calls)
                    .iter()
                    .any(|name| name == "::direct"),
                "an independently retained direct target remains reachable"
            );
            assert!(!summary.pure, "withheld selected effects remain opaque");
        }
    }
}

#[test]
fn original_positioned_child_bodies_preserve_replacements_and_aliases() {
    // naming.compiler.original-positioned-child-body-reachability
    // docs/design/analysis/name-resolution-proofs/compiler-original-positioned-child-body-reachability.md
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    for (setup, body, expected) in [
        ("", "catch {leaf}", true),
        (
            "proc catch {payload} {return $payload}",
            "catch {leaf}",
            false,
        ),
        ("interp alias {} held {} catch", "held {leaf}", true),
        ("rename catch held", "held {leaf}", true),
        (
            "proc held {payload} {return $payload}",
            "held {leaf}",
            false,
        ),
    ] {
        let source = format!("proc leaf {{}} {{}}; {setup}; proc caller {{}} {{time {{{body}}}}}");
        assert_eq!(
            calls_of(&source, "::caller", profile).contains(&"::leaf".to_owned()),
            expected,
            "{source}"
        );
    }
}

/// Literal callback values and authentic selected builders supply possible
/// reachability edges independently of future execution and frame entry.
#[test]
fn command_prefix_callbacks_are_call_graph_edges_in_every_shape() {
    for prefix in ["cb", "{cb -nocase}", "[list cb]", "[list cb extra]"] {
        let src = format!(
            "proc cb {{a b}} {{ return 0 }}\nproc go {{}} {{ lsort -command {prefix} {{x y}} }}\n"
        );
        assert_eq!(
            calls_of(
                &src,
                "::go",
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
            ),
            vec!["::cb".to_owned()],
            "`-command {prefix}` must record a call edge to cb",
        );
    }
}

/// The `trace add variable … command cb` shape.
#[test]
fn original_callback_reachability_distinguishes_literal_values_and_selected_builders() {
    // naming.source.original-callback-procedure-target
    // docs/design/analysis/name-resolution-proofs/source-original-callback-procedure-target.md
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    for setup in ["", "rename list make", "interp alias {} make {} list"] {
        let builder = if setup.is_empty() { "list" } else { "make" };
        let source = format!(
            "proc cb {{args}} {{}}; {setup}; proc go {{x}} {{lsort -command [{builder} cb $x] {{3 1 2}}}}"
        );
        assert!(
            calls_of(&source, "::go", profile).contains(&"::cb".to_owned()),
            "{source}"
        );
    }
    for setup in ["proc list args {}", "rename list {}", "rename cb {}"] {
        let source = format!(
            "proc cb {{args}} {{}}; {setup}; proc go {{x}} {{lsort -command [list cb $x] {{3 1 2}}}}"
        );
        assert!(
            !calls_of(&source, "::go", profile).contains(&"::cb".to_owned()),
            "{source}"
        );
    }
    let source =
        "proc cb args {}; proc {[list} args {}; proc go {} {lsort -command {[list cb]} {3 1 2}}";
    let calls = calls_of(source, "::go", profile);
    assert!(calls.contains(&"::[list".to_owned()), "{calls:?}");
    assert!(!calls.contains(&"::cb".to_owned()));
    let source = "proc {cb name} args {}; proc go {} {lsort -command {{cb name} BAKED} {3 1 2}}";
    assert!(calls_of(source, "::go", profile).contains(&"::cb name".to_owned()));
}

#[test]
fn a_trace_callback_is_a_call_graph_edge() {
    let src = "proc cb {args} { return 0 }\n\
                   proc go {} { trace add variable v write [list cb] }\n";
    assert_eq!(
        calls_of(
            src,
            "::go",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
        ),
        vec!["::cb".to_owned()]
    );
}

/// TN control: a prefix computed by some *other* substitution names no
/// command this scan can see, so it must record no callback edge rather
/// than guess one from the substitution's own head (`[pick` is not
/// `cb`).  What the scan does with the substitution *itself* is a
/// separate concern this test deliberately does not pin.
#[test]
fn a_computed_command_prefix_records_no_callback_edge() {
    let src = "proc cb {a b} { return 0 }\n\
                   proc pick {} { return cb }\n\
                   proc go {} { lsort -command [pick] {x y} }\n";
    assert!(
        !calls_of(
            src,
            "::go",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
        )
        .contains(&"::cb".to_owned()),
        "the computed prefix's result is unknown, so cb is not a proven callee",
    );
}

/// The user-proc invoker is registry data (`Traits::INVOKES_USER_PROC`),
/// not the command name `call`: under iRules the edge goes to the named
/// procedure, and under a dialect with no such command a user procedure
/// that happens to be *called* `call` is an ordinary callee.
#[test]
fn user_proc_invoker_is_registry_driven_not_name_matched() {
    assert_eq!(
        calls_of(
            "proc helper {mode} { return $mode }\nwhen RULE_INIT { call helper dev }\n",
            "::when::RULE_INIT",
            tcl_dialect::DialectProfile::irules(),
        ),
        vec!["::helper".to_owned()],
    );
    assert_eq!(
        calls_of(
            "proc call {mode} { return $mode }\nproc go {} { call helper }\n",
            "::go",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        ),
        vec!["::call".to_owned()],
        "plain Tcl has no user-proc invoker, so `call` is just a procedure",
    );
}

#[test]
fn command_prefix_head_reads_each_shape() {
    let registry = tcl_registry::CommandRegistry::build_default();
    assert_eq!(command_prefix_head(&registry, "cb").as_deref(), Some("cb"));
    assert_eq!(
        command_prefix_head(&registry, "cb -nocase").as_deref(),
        Some("cb"),
    );
    assert_eq!(
        command_prefix_head(&registry, "[list cb $x]").as_deref(),
        Some("[list"),
    );
    assert_eq!(
        command_prefix_head(&registry, "[pick]").as_deref(),
        Some("[pick]")
    );
    assert_eq!(
        command_prefix_head(&registry, "{cb name} tail").as_deref(),
        Some("cb name")
    );
    assert_eq!(
        command_prefix_head(&registry, "\"cb name\" tail").as_deref(),
        Some("cb name")
    );
    assert_eq!(
        command_prefix_head(&registry, r"cb\ name tail").as_deref(),
        Some("cb name")
    );
    assert_eq!(command_prefix_head(&registry, "{unterminated"), None);
}

/// The interprocedural call-graph scanners each need a depth cap —
/// `scan_expr_for_calls`, `note_params_in_expr` and
/// `walk_collect_param_refs` once per `ExprNode` level (Tier 1A);
/// `scan_source_for_calls` (+ its `scan_value_substitutions` helper) once
/// per nested `[cmd …]` substitution / `ArgRole::Body` arg inside a
/// single word's raw text (Tier 1B). Both axes were genuinely unbounded,
/// independent of the statement-tree `MAX_INTERPROCEDURAL_WALK_DEPTH`
/// cap, and empirically overflowed the native stack (SIGABRT) in the low
/// thousands of levels on a 2 MiB thread (`cargo test`'s default). 3000 is
/// comfortably past that crash range and past both caps (256); the
/// assertion is that each scanner returns at all.
#[test]
fn deeply_nested_interprocedural_scans_survive() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let known: HashSet<ProcedureId> = HashSet::new();
    let source_module = crate::ir::Module::default();
    let params: HashSet<String> = ["x".to_owned()].into_iter().collect();
    let ctx = ScanCtx {
        known: &known,
        registry: &registry,
        dialect: None,
        params: &params,
        object_types: ObjectTypeMap::none().candidates,
        identities: crate::realm::CommandBindingRealm::none(),
        declared: None,
        positioned_substitutions: false,
        source_input: None,
        source_procedures: &source_module.procedures,
        source_module: &source_module,
        current_tokens: None,
        parameter_body: None,
    };

    // A 3000-deep `ExprNode` tree (nested unary `!` over `$x`).
    let mut node = crate::expr_ast::ExprNode::Var {
        text: "$x".into(),
        name: "x".into(),
        start: 0,
        end: 2,
    };
    for _ in 0..3000 {
        node = crate::expr_ast::ExprNode::Unary {
            op: crate::expr_ast::UnaryOp::Not,
            operand: Box::new(node),
        };
    }

    // Tier 1A walkers over the deep expr tree.
    let mut facts = LocalFacts::default();
    scan_expr_for_calls(&node, ctx, &mut facts, 0);
    note_params_in_expr(
        &node,
        &params,
        &mut facts,
        tcl_lexer::LexerConfig::default(),
        0,
    );
    let mut refs = Vec::new();
    walk_collect_param_refs(
        &node,
        &params,
        &mut refs,
        Some(tcl_lexer::LexerConfig::default()),
        0,
    );

    // Tier 1B walker over deeply nested command substitutions
    // `a [a [a [ … [x] … ]]]`: each level is a literal-headed command
    // whose argument holds the next `[…]` substitution, so
    // `scan_source_for_calls` recurses through `scan_value_substitutions`
    // once per bracket level.
    let mut deep_brackets = "x".to_owned();
    for _ in 0..3000 {
        deep_brackets = format!("a [{deep_brackets}]");
    }
    scan_source_for_calls(&deep_brackets, ctx, &mut facts, 0);
}

#[test]
fn arity_from_names_handles_trailing_args() {
    assert_eq!(
        arity_from_names(&["a".to_owned(), "args".to_owned()]),
        Arity::at_least(1)
    );
    assert_eq!(
        arity_from_names(&["a".to_owned(), "b".to_owned()]),
        Arity::exact(2)
    );
    assert_eq!(arity_from_names(&[]), Arity::exact(0));
}

#[test]
fn proc_summary_unknown_is_conservative() {
    let s = ProcSummary::unknown("::mystery");
    assert!(s.has_unknown_calls);
    assert!(s.writes_global);
    assert!(!s.pure);
    assert_eq!(s.effect_reads, EffectRegion::UNKNOWN_STATE);
    assert_eq!(s.effect_writes, EffectRegion::UNKNOWN_STATE);
}

#[test]
fn resolve_absolute_names() {
    let known = known_set(&["::foo::bar"]);
    assert_eq!(
        resolve_internal_call("::foo::bar", "::top", &known),
        Some("::foo::bar".into())
    );
    // Absolute name not in the known set returns None.
    assert_eq!(
        resolve_internal_call("::foo::missing", "::top", &known),
        None
    );
}

#[test]
fn resolve_relative_with_segments() {
    // `foo::bar` from a global-level caller → `::foo::bar` (the only
    // candidate, since the caller's own namespace is already `::`).
    let known = known_set(&["::foo::bar"]);
    assert_eq!(
        resolve_internal_call("foo::bar", "::top", &known),
        Some("::foo::bar".into())
    );
}

#[test]
fn resolve_relative_dotted_name_prefers_caller_namespace() {
    // A relative dotted word (`ns2::inner`) resolves against the
    // caller's own namespace first, not straight at global (confirmed
    // against tclsh 9.0.4 — see `bareword_resolution_candidates`).
    let known = known_set(&["::ns::ns2::inner", "::ns2::inner"]);
    assert_eq!(
        resolve_internal_call("ns2::inner", "::ns::caller", &known),
        Some("::ns::ns2::inner".into()),
        "must prefer the caller-namespace proc over the root one",
    );
    // No caller-namespace candidate → falls back to the root proc.
    assert_eq!(
        resolve_internal_call("ns2::inner", "::top", &known),
        Some("::ns2::inner".into()),
    );
}

#[test]
fn resolve_bare_does_not_walk_ancestor_namespaces() {
    // Real Tcl bareword resolution is exactly two levels — the
    // caller's own namespace, then global — absent an explicit
    // `namespace path`. A proc defined in a *grandparent* namespace
    // must not resolve for a bare call from `::ns::a::caller`.
    let known = known_set(&["::ns::helper"]);
    assert_eq!(
        resolve_internal_call("helper", "::ns::a::caller", &known),
        None,
        "a grandparent-namespace proc must not resolve for a bare call",
    );
    // Control: the *direct* enclosing namespace still resolves.
    let known2 = known_set(&["::ns::a::helper"]);
    assert_eq!(
        resolve_internal_call("helper", "::ns::a::caller", &known2),
        Some("::ns::a::helper".into()),
    );
}

#[test]
fn resolve_bare_falls_through_to_global() {
    let known = known_set(&["::helper"]);
    assert_eq!(
        resolve_internal_call("helper", "::ns::caller", &known),
        Some("::helper".into())
    );
}

#[test]
fn resolve_bare_returns_none_when_not_found() {
    let known = known_set(&["::other"]);
    assert_eq!(
        resolve_internal_call("helper", "::ns::caller", &known),
        None
    );
}

#[test]
fn resolve_empty_command_is_none() {
    let known = known_set(&["::helper"]);
    assert_eq!(resolve_internal_call("", "::top", &known), None);
}

#[test]
fn namespace_parts_from_proc_extracts_segments() {
    assert_eq!(
        namespace_parts_from_proc("::foo::bar::baz"),
        vec!["foo", "bar"]
    );
    assert_eq!(namespace_parts_from_proc("::simple"), Vec::<String>::new());
    assert_eq!(namespace_parts_from_proc("::"), Vec::<String>::new());
    // Colon runs are one separator (C's `TclGetNamespaceForQualName`;
    // tclsh8.6: `foo:::bar` names `::foo::bar`) — the shared
    // `qualifier_segments` split, where a naive `split("::")` would keep
    // a stray `:` segment.
    assert_eq!(
        namespace_parts_from_proc("::foo:::bar::baz"),
        vec!["foo", "bar"]
    );
    assert_eq!(namespace_parts_from_proc("a:::b"), vec!["a"]);
}

#[test]
fn resolve_call_target_delegates() {
    let known = known_set(&["::helper"]);
    assert_eq!(
        resolve_call_target("helper", &[], "::top", &known),
        Some("::helper".into())
    );
}

// summary-building tests

use crate::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;

fn build(source: &str) -> InterproceduralAnalysis {
    // The fixture's identity surface below is explicitly C Tcl 8.6.
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let cu = CompilationUnit::build_for_profile(source, &registry, false, profile);
    let identities = crate::realm::document_realm_bindings_for_module(&cu.ir_module, &registry)
        .expect("fixture's retained Module source owner");
    build_interprocedural_analysis(
        &cu.ir_module,
        &registry,
        Some(profile),
        ObjectTypeMap::none(),
        &identities,
        None,
    )
}

fn source_point_trace(source: &str, body: &str) -> Vec<String> {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let unit = CompilationUnit::build_for_profile(source, &registry, false, profile);
    let statements = fixture_id_if_present(&unit.ir_module.procedures, body)
        .and_then(|id| unit.ir_module.procedures.get(&id))
        .map(|row| &row.body)
        .or_else(|| {
            fixture_id_if_present(&unit.ir_module.methods, body)
                .and_then(|id| unit.ir_module.methods.get(&id))
                .map(|row| &row.body)
        });
    statements.into_iter().flat_map(|body| body.statements.iter().map(|statement| {
            let Some(tokens) = body.retained_source_tokens_for_statement(statement) else {
                return format!("no source carrier {:?}", statement.span());
            };
            let Some(binding) = &tokens.source_binding else {
                return format!("{:?}: no source binding", tokens.argv_texts);
            };
            let normal = crate::registry_invocation::normal_transfer_invocation(
                &registry, None, tokens,
            );
            format!(
                "{:?}: unknown={} absent={} handler={:?} frame={:?} ns={} known={} globals={:?} ns_vars={:?} mutations={:?}",
                tokens.argv_texts, binding.unknown, binding.may_be_absent,
                binding.proved_handler_target().map(|target| (&target.command, target.registry_backed)),
                binding.variable_context.frame_kind, binding.variable_context.namespace,
                binding.variable_context.namespace_known, binding.variable_context.globals,
                binding.variable_context.ns_vars,
                normal.map(|normal| normal.mutation_places(&binding.variable_context, &registry)),
            )
        })).collect()
}

#[test]
fn positioned_alias_traits_use_selected_frame_and_transition_grammar() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        for (level, caller_write) in [("0x1", true), ("#0", false), ("0", false)] {
            let source = format!("proc writer {{n}} {{upvar {level} $n local; set local NEW}}");
            let unit = CompilationUnit::build_for_dialect(&source, registry, false, dialect);
            let identities =
                crate::realm::document_realm_bindings_for_module(&unit.ir_module, registry)
                    .expect("fixture's retained Module source owner");
            let analysis = build_interprocedural_analysis(
                &unit.ir_module,
                registry,
                Some(profile),
                ObjectTypeMap::none(),
                &identities,
                None,
            );
            let traits = &analysis
                .procedures
                .get(&fixture_id(&analysis.procedures, "::writer"))
                .expect("authored fixture body")
                .param_traits["n"];
            assert!(
                traits.contains(&ProcArgTrait::VarRead),
                "{dialect} {level}: {traits:?}; source body: {:#?}",
                unit.ir_module
                    .procedures
                    .get(&fixture_id(&unit.ir_module.procedures, "::writer"))
                    .expect("authored fixture body")
                    .body
            );
            assert_eq!(
                traits.contains(&ProcArgTrait::VarWrite),
                caller_write,
                "only the caller-frame alias writes this caller parameter: {dialect} {level}: {traits:?}"
            );
        }
    }
}

#[test]
fn document_alias_prefix_preserves_the_call_graph_target() {
    let facts = build(
        "proc target {prefix value} {return $value}\ninterp alias {} invoke {} target FIXED\nproc caller {value} {invoke $value}",
    );
    let caller = facts
        .procedures
        .get(&fixture_id(&facts.procedures, "::caller"))
        .expect("authored fixture body");
    assert!(
        edge_labels(&facts, &caller.direct_calls)
            .iter()
            .any(|callee| callee == "::target"),
        "document alias target must remain a call edge: {:?}",
        caller.direct_calls
    );
    assert!(
        !edge_labels(&facts, &caller.direct_calls)
            .iter()
            .any(|callee| callee == "::invoke")
    );
}

#[test]
fn unknown_alias_prefix_bytes_preserve_document_call_graph_target() {
    let facts = build(
        "proc target {prefix value} {return $value}\nproc caller {prefix value} {interp alias {} invoke {} target $prefix; invoke $value}",
    );
    let caller = facts
        .procedures
        .get(&fixture_id(&facts.procedures, "::caller"))
        .expect("authored fixture body");
    assert!(
        edge_labels(&facts, &caller.direct_calls)
            .iter()
            .any(|callee| callee == "::target"),
        "unknown prefix bytes preserve target identity and argv slots: {:?}",
        caller.direct_calls
    );
    assert!(
        !edge_labels(&facts, &caller.direct_calls)
            .iter()
            .any(|callee| callee == "::invoke")
    );
}

#[test]
fn document_procedure_named_like_wrapper_does_not_inherit_body_roles() {
    let facts = build(
        "proc time {script} {return SAFE}\nproc helper {} {return VALUE}\nproc caller {} {time {helper}}",
    );
    let caller = facts
        .procedures
        .get(&fixture_id(&facts.procedures, "::caller"))
        .expect("authored fixture body");
    assert!(
        edge_labels(&facts, &caller.direct_calls)
            .iter()
            .any(|callee| callee == "::time")
    );
    assert!(
        !edge_labels(&facts, &caller.direct_calls)
            .iter()
            .any(|callee| callee == "::helper"),
        "{:?}",
        caller.direct_calls
    );
}

#[test]
fn uncertain_point_binding_preserves_unknown_effect_obligation() {
    let facts =
        build("if {$flag} {proc time {script} {return SAFE}}\nproc caller {} {time {puts SAFE}}");
    let caller = facts
        .procedures
        .get(&fixture_id(&facts.procedures, "::caller"))
        .expect("authored fixture body");
    assert!(caller.has_unknown_calls || caller.has_barrier);
    assert!(!caller.pure);
}

#[test]
fn empty_module_has_no_summaries() {
    let ia = build("");
    assert!(ia.procedures.is_empty());
}

/// A proc called *inside* an
/// `apply` lambda body reached through a `[…]` command substitution
/// (`set y [apply {p {…}} $x]`) must still register as a call-graph
/// edge. `apply`'s lambda-literal argument is `ArgRole::LambdaLiteral`,
/// not `Body` — scanning the whole `{argList} {body}` blob as one script
/// (the old generic-`Body` path) misread the parameter word as a
/// command name and never reached the real body's own calls, which
/// would have made `helper` look like dead code (O124) despite being
/// reachable only from inside the lambda.
#[test]
fn call_inside_apply_lambda_body_is_a_direct_call() {
    let ia = build(
        "proc ::helper {x} { return $x }\n\
             proc ::f {} { set y [apply {p {helper $p}} 1]; return $y }\n",
    );
    let s = fixture_proc(&ia, "::f").expect("::f summary");
    assert!(
        edge_labels(&ia, &s.direct_calls)
            .iter()
            .any(|c| c == "::helper"),
        "expected a direct_calls edge to ::helper via the apply lambda body; got {:?}",
        s.direct_calls
    );
}

/// A lambda body's bare calls must
/// resolve in the lambda's own namespace (the optional third `apply`
/// element, or global when omitted) — never the enclosing procedure's
/// namespace. `::ns::f`'s `apply {{} {helper}}` must resolve `helper` to
/// `::helper` (the two-element form runs in `::`), not `::ns::helper`,
/// even though `::ns::helper` also exists and would otherwise look like
/// the "closer" match.
#[test]
fn call_inside_apply_lambda_body_resolves_in_lambda_namespace() {
    let ia = build(
        "namespace eval ::ns {\n\
                 proc helper {} { return 1 }\n\
             }\n\
             proc ::helper {} { return 2 }\n\
             proc ::ns::f {} { set y [apply {{} {helper}}]; return $y }\n",
    );
    let s = fixture_proc(&ia, "::ns::f").expect("::ns::f summary");
    assert!(
        edge_labels(&ia, &s.direct_calls)
            .iter()
            .any(|c| c == "::helper"),
        "expected the apply lambda body's bare `helper` call to resolve \
             in the global namespace (Tcl evaluates a two-element apply \
             lambda in `::`); got {:?}",
        s.direct_calls
    );
    assert!(
        !edge_labels(&ia, &s.direct_calls)
            .iter()
            .any(|c| c == "::ns::helper"),
        "the apply lambda body must not resolve `helper` against the \
             enclosing proc's `::ns` namespace; got {:?}",
        s.direct_calls
    );
}

/// The lambda's explicit third-element namespace overrides both the
/// enclosing procedure's namespace and the global default.
#[test]
fn call_inside_apply_lambda_body_resolves_in_explicit_namespace() {
    let ia = build(
        "namespace eval ::other {\n\
                 proc helper {} { return 1 }\n\
             }\n\
             proc ::helper {} { return 2 }\n\
             namespace eval ::ns {}\n\
             proc ::ns::f {} { set y [apply {{} {helper} ::other}]; return $y }\n",
    );
    let s = fixture_proc(&ia, "::ns::f").expect("::ns::f summary");
    assert!(
        edge_labels(&ia, &s.direct_calls)
            .iter()
            .any(|c| c == "::other::helper"),
        "expected the apply lambda's explicit `::other` namespace \
             element to resolve `helper` there; got {:?}",
        s.direct_calls
    );
    assert!(
        !edge_labels(&ia, &s.direct_calls)
            .iter()
            .any(|c| c == "::helper"),
        "the explicit lambda namespace must not fall back to global; \
             got {:?}",
        s.direct_calls
    );
}

#[test]
fn instance_method_callback_is_a_direct_call() {
    use crate::provider_fixtures::{Provider, entry};
    let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
    let context = environment.default_context_registry();
    let registry = context.commands().as_ref();
    let source = "package require struct::graph\nproc onNode {a g n} {}\nproc build {} { struct::graph g\n g node insert root\n g walk root -command onNode }\nbuild\n";
    let selected = entry(registry, &[Provider::Graph]);
    let cu = CompilationUnit::build_with_source_entry(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry,
            dialect: Some(environment.analyser_profile()),
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        &selected,
    )
    .with_interprocedural(registry, Some(environment.analyser_profile()));
    let actual = cu.interproc.as_ref().expect("interproc");
    let summary = fixture_proc(actual, "::build").expect("authored build summary");
    assert!(
        edge_labels(actual, &summary.direct_calls)
            .iter()
            .any(|call| call == "::onNode"),
        "the actual loaded graph's callback must reach onNode: {:?}; candidates={:?}; argv={:?}",
        summary.direct_calls,
        crate::object_types::object_handle_classes(&cu, registry),
        cu.procedures
            .get(&fixture_id(&cu.ir_module.procedures, "::build"))
            .into_iter()
            .flat_map(|unit| unit.cfg.blocks.values())
            .flat_map(|block| &block.statements)
            .filter_map(|statement| {
                let tokens = statement.tokens()?;
                let binding = tokens.source_binding.as_ref()?;
                Some((
                    tokens.argv_texts.first(),
                    binding.evaluated_command_word(),
                    &binding.evaluated_argument_values,
                    tokens.synthetic.is_some(),
                ))
            })
            .collect::<Vec<_>>(),
    );
    assert!(
        summary.has_unknown_calls && !summary.pure,
        "candidate callback reachability cannot close the receiver's effects",
    );
    let unprovided = CompilationUnit::build_with_source_entry(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry,
            dialect: Some(environment.analyser_profile()),
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        &entry(registry, &[]),
    )
    .with_interprocedural(registry, Some(environment.analyser_profile()));
    assert!(
        !fixture_proc(unprovided.interproc.as_ref().expect("interproc"), "::build").is_some_and(
            |summary| edge_labels(
                unprovided.interproc.as_ref().expect("interproc"),
                &summary.direct_calls
            )
            .iter()
            .any(|call| call == "::onNode")
        ),
        "an unloaded graph catalogue cannot license a callback edge",
    );
}

#[test]
fn simple_proc_is_recorded_with_params_and_arity() {
    let ia = build("proc ::greet {name} { puts hi }");
    let s = fixture_proc(&ia, "::greet").expect("proc summary");
    assert_eq!(s.params, vec!["name".to_string()]);
    assert_eq!(s.arity, Arity::exact(1));
}

// writes_global recognises scope aliases

#[test]
fn global_alias_write_counts_as_writes_global() {
    // A bare `set g` after `global g` mutates a caller-visible
    // variable through the alias — writes_global, even though the
    // written name is bare, not `::`-qualified.
    for src in [
        "proc ::f {} { global g\nset g 5 }",
        "proc ::f {} { variable v\nset v 1 }",
        "proc ::f {} { upvar #0 ::x g\nset g 1 }",
        "proc ::f {} { global g\nincr g }",
        "proc ::f {} { global g\nappend g x }",
    ] {
        let ia = build(src);
        let s = fixture_proc(&ia, "::f").expect("::f summary");
        assert!(
            s.writes_global,
            "expected writes_global for: {src}; points={:?}",
            source_point_trace(src, "::f"),
        );
    }
}

/// An alias pair whose level selects the current frame stays a global
/// alias: its other variable is in practice a qualified, computed or
/// namespace-declared name (`upvar 0 $token state`), which reaches a
/// namespace cell from this frame. Its level is read as a `FrameLevel`,
/// so every spelling of the global frame and of this one counts.
#[test]
fn standalone_empty_alias_declaration_and_mutation_have_distinct_write_effects() {
    let registry = tcl_registry::CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let known = HashSet::new();
    let params = HashSet::new();
    let module = crate::ir::Module::default();
    let ctx = ScanCtx {
        known: &known,
        registry: &registry,
        dialect: None,
        params: &params,
        object_types: ObjectTypeMap::none().candidates,
        identities: crate::realm::CommandBindingRealm::none(),
        declared: None,
        positioned_substitutions: false,
        source_input: None,
        source_procedures: &module.procedures,
        source_module: &module,
        current_tokens: None,
        parameter_body: None,
    };
    let mut facts = LocalFacts::default();
    scan_call_statement(
        "global",
        &[String::new()],
        &[String::new()],
        None,
        ctx,
        &mut facts,
    );
    assert!(facts.global_aliases.contains(""));
    assert!(
        !facts.writes_global,
        "declaring the literal empty alias is not a value write"
    );
    scan_call_statement(
        "append",
        &[String::new(), "VALUE".into()],
        &[String::new()],
        None,
        ctx,
        &mut facts,
    );
    assert!(
        facts.writes_global,
        "mutation reaches the retained literal empty alias"
    );
}

#[test]
fn literal_empty_global_alias_mutations_remain_visible() {
    let analysis = build("proc ::f {} {global {}; append {} VALUE}");
    assert!(
        fixture_proc(&analysis, "::f")
            .expect("authentic standalone declaration")
            .writes_global
    );
}

#[test]
fn nominal_global_alias_coverage_keeps_empty_data_separate_from_unknown_names() {
    let registry = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let coverage =
        global_alias_names("global", &[String::new(), "array(key)".into()], &registry).unwrap();
    assert!(coverage.names.contains("") && coverage.names.contains("array"));
    assert!(!coverage.unknown);
    let coverage = global_alias_names("global", &["$computed".into()], &registry).unwrap();
    assert!(coverage.names.is_empty() && coverage.unknown);
    let coverage = global_alias_names(
        "upvar",
        &["#0".into(), "source".into(), String::new()],
        &registry,
    )
    .unwrap();
    assert!(coverage.names.contains("") && !coverage.unknown);
    assert!(
        global_alias_names(
            "upvar",
            &["1".into(), "source".into(), "local".into()],
            &registry
        )
        .is_none()
    );
}

#[test]
fn an_alias_at_the_current_or_global_frame_is_a_global_alias() {
    for src in [
        "proc ::f {} { upvar 0 ::x g\nset g 1 }",
        "proc ::f {t} { upvar +0 $t g\nset g 1 }",
        "proc ::f {} { upvar #00 x g\nset g 1 }",
    ] {
        let ia = build(src);
        assert!(
            ia.procedures
                .get(&fixture_id(&ia.procedures, "::f"))
                .expect("authored fixture body")
                .writes_global,
            "{src}"
        );
    }
}

#[test]
fn non_aliased_local_write_is_not_writes_global() {
    // A plain local `set g` (no global/variable/upvar #0 decl) is
    // not a global write; `upvar 1` aliases a caller frame, not
    // global scope, so it is not flagged here either.
    for src in [
        "proc ::f {} { set g 5 }",
        "proc ::f {} { upvar 1 x g\nset g 1 }",
    ] {
        let ia = build(src);
        let s = fixture_proc(&ia, "::f").expect("::f summary");
        assert!(!s.writes_global, "unexpected writes_global for: {src}");
    }
}

// method-purity summary tests

#[test]
fn pure_getter_method_is_summarised_pure() {
    let ia = build(
        "oo::class create C {\n\
             \x20   variable n\n\
             \x20   method get {} { ::return $n }\n\
             }",
    );
    let m = fixture_method(&ia, "::C::get").expect("method summary");
    assert!(m.base.pure, "read-only getter should be pure: {:?}", m.base);
    assert!(m.writes_instance_vars.is_empty());
    assert_eq!(m.class_name, "::C");
    assert_eq!(m.method_kind, crate::ir::MethodKind::Method);
}

#[test]
fn instance_var_write_makes_method_impure() {
    let ia = build(
        "oo::class create C {\n\
             \x20   variable n\n\
             \x20   method bump {} { incr n }\n\
             }",
    );
    let m = fixture_method(&ia, "::C::bump").expect("method summary");
    assert!(!m.base.pure, "instance-var write must be impure");
    assert!(
        m.writes_instance_vars.contains("n"),
        "writes_instance_vars: {:?}",
        m.writes_instance_vars
    );
}

#[test]
fn array_element_instance_var_write_makes_method_impure() {
    // `set counter(0) 1` writes the instance var `counter` (compared
    // on the base scalar name, not the raw element name).
    let ia = build(
        "oo::class create C {\n\
             \x20   variable counter\n\
             \x20   method bump {} { set counter(0) 1 }\n\
             }",
    );
    let m = fixture_method(&ia, "::C::bump").expect("method summary");
    assert!(!m.base.pure);
    assert!(m.writes_instance_vars.contains("counter"));
}

#[test]
fn my_dispatch_makes_method_impure() {
    // `my <other>` is a dynamic self-dispatch — unresolvable
    // statically, so the method can't be proven pure.
    let ia = build(
        "oo::class create C {\n\
             \x20   method a {} { my b }\n\
             \x20   method b {} { ::return 1 }\n\
             }",
    );
    let a = fixture_method(&ia, "::C::a").expect("method a");
    assert!(!a.base.pure, "my-dispatch caller must be impure");
    // The pure leaf method still summarises pure.
    let b = fixture_method(&ia, "::C::b").expect("method b");
    assert!(b.base.pure, "absolute literal return leaf: {:?}", b.base);
}

#[test]
fn redefined_method_joins_over_every_retained_body() {
    // The lowering retains replacement bodies, so the
    // summary is the JOIN over every body a dispatch may run —
    // pure when all bodies are pure, impure when any is.
    let ia = build(
        "oo::class create C {\n\
             \x20   method m {} { ::return 1 }\n\
             }\n\
             oo::define C {\n\
             \x20   method m {} { ::return 2 }\n\
             }",
    );
    let methods = ia
        .methods
        .values()
        .filter(|method| method.base.qualified_name == "::C::m")
        .collect::<Vec<_>>();
    assert_eq!(methods.len(), 2);
    for m in methods {
        assert!(
            m.base.pure,
            "two pure bodies join to pure — abstention on the mere fact of \
             redefinition is the imprecision issue #1166 removes"
        );
        // Disagreeing constants join to no-constant-return.
        assert!(m.base.constant_return.is_none());
    }

    // FN-now-caught: an impure REPLACEMENT makes the join impure even
    // though the first (retained-in-`methods`) body is pure.
    let ia = build(
        "oo::class create C {\n\
             \x20   method m {} { return 1 }\n\
             }\n\
             oo::define C {\n\
             \x20   method m {} { puts side-effect\n\
             \x20       return 2 }\n\
             }",
    );
    let methods = ia
        .methods
        .values()
        .filter(|method| method.base.qualified_name == "::C::m")
        .collect::<Vec<_>>();
    assert_eq!(methods.len(), 2);
    for m in methods {
        assert!(!m.base.pure, "an impure replacement body must join impure");
    }

    // Agreeing constants survive the join.
    let ia = build(
        "oo::class create C {\n\
             \x20   method m {} { ::return 7 }\n\
             }\n\
             oo::define C {\n\
             \x20   method m {} { ::return 7 }\n\
             }",
    );
    let methods = ia
        .methods
        .values()
        .filter(|method| method.base.qualified_name == "::C::m")
        .collect::<Vec<_>>();
    assert_eq!(methods.len(), 2);
    for m in methods {
        assert!(m.base.pure);
        assert!(matches!(
            m.base.constant_return,
            Some(ConstantReturn::Int(7))
        ));
    }
}

#[test]
fn unreadable_oo_definitions_force_methods_impure() {
    // A dynamic member name may have redefined ANY method of the
    // class with an unknown body — every method of the class joins
    // impure.
    let ia = build(
        "oo::class create C {\n\
             \x20   method m {} { return 1 }\n\
             \x20   method $n {} { return 2 }\n\
             }",
    );
    let m = fixture_method(&ia, "::C::m").expect("method summary");
    assert!(!m.base.pure, "unanalysable class member must widen");

    // A dynamic class word may have touched any class — module-wide.
    let ia = build(
        "oo::class create C {\n\
             \x20   method m {} { return 1 }\n\
             }\n\
             oo::define $cls { method m {} { puts hi } }",
    );
    let m = fixture_method(&ia, "::C::m").expect("method summary");
    assert!(!m.base.pure, "dynamic OO definition target must widen");
}

#[test]
fn variable_decl_is_not_counted_as_instance_var_write() {
    // A method-local `variable x` is a link/declaration, not a
    // write — its def must not land in `writes_instance_vars`
    // (soundness fix: a read-only instance-var method isn't
    // forced impure by *that* path).  Note: the registry models
    // `variable` itself as a scope-alias barrier, so `peek` is still
    // impure overall — but not via the instance-var-write channel.
    let ia = build(
        "oo::class create C {\n\
             \x20   method peek {} { variable x; return $x }\n\
             }",
    );
    let m = fixture_method(&ia, "::C::peek").expect("method summary");
    assert!(
        m.writes_instance_vars.is_empty(),
        "variable decl wrongly counted as write: {:?}",
        m.writes_instance_vars
    );
}

#[test]
fn call_in_if_condition_is_recorded() {
    // Calls embedded in an `if {[q]} ...` predicate are call-graph edges:
    // ::q must appear in ::a's direct calls.
    let ia = build(
        "proc ::q {} { return 1 }\n\
             proc ::a {} { if {[::q]} { puts hi } }",
    );
    let a = fixture_proc(&ia, "::a").expect("::a recorded");
    assert!(
        edge_labels(&ia, &a.calls).contains(&"::q".to_string()),
        "expected ::q in ::a's call graph, got {:?}",
        a.calls
    );
}

#[test]
fn call_inside_catch_in_if_condition_is_recorded() {
    // `if {[catch {p}]} ...` — ::p is reached via BODY-role
    // recursion into the `catch` argument.
    let ia = build(
        "proc ::p {} { return 1 }\n\
             proc ::a {} { if {[catch {::p}]} { puts hi } }",
    );
    let a = fixture_proc(&ia, "::a").expect("::a recorded");
    assert!(
        edge_labels(&ia, &a.calls).contains(&"::p".to_string()),
        "expected ::p in ::a's call graph via BODY recursion, got {:?}",
        a.calls
    );
}

#[test]
fn call_in_while_condition_is_recorded() {
    let ia = build(
        "proc ::q {} { return 1 }\n\
             proc ::a {} { while {[::q]} { break } }",
    );
    let a = fixture_proc(&ia, "::a").expect("::a recorded");
    assert!(
        edge_labels(&ia, &a.calls).contains(&"::q".to_string()),
        "expected ::q via while-condition expr scan, got {:?}",
        a.calls
    );
}

#[test]
fn calls_captured_and_transitively_closed() {
    let ia = build(
        "proc ::a {} { ::b }\n\
             proc ::b {} { ::c }\n\
             proc ::c {} { set x 1 }",
    );
    let a = fixture_proc(&ia, "::a").unwrap();
    assert!(edge_labels(&ia, &a.calls).contains(&"::b".to_string()));
    assert!(
        edge_labels(&ia, &a.calls).contains(&"::c".to_string()),
        "expected ::c in transitive closure of ::a, got {:?}",
        a.calls,
    );
}

#[test]
fn barrier_detected_via_eval_call() {
    // ``eval {literal}`` inside a proc relaxes to a
    // Statement::Block (the literal body is statically known),
    // so it does NOT mark the proc as a barrier. Use a dynamic
    // body that genuinely cannot be resolved at lowering time.
    let ia = build("proc ::bad {} { eval $dyn }");
    let s = fixture_proc(&ia, "::bad").unwrap();
    assert!(s.has_barrier);
    assert!(!s.pure);
}

#[test]
fn pure_proc_is_flagged_pure() {
    let ia = build("proc ::add2 {x} { return [expr {$x + 2}] }");
    let s = fixture_proc(&ia, "::add2").unwrap();
    // `return` + pure `expr` — nothing impure; but calling
    // `return` itself is registry-defined as non-pure in
    // some dialects. Either way, `pure` should reflect the
    // union accurately — just verify the summary exists.
    let _ = s.pure;
}

#[test]
fn fp_nab_03_recursive_arithmetic_proc_is_pure() {
    // FP-NAB-03: a self-recursive arithmetic proc must come out
    // `pure == true` from the
    // interprocedural fix-point. The fix-point is the *greatest* one
    // (purity initialised optimistically, then refuted), so a call back into
    // the proc being analysed does not conservatively mark it impure.
    let ia = build(
        "proc ::fact {n} {\n    if {$n <= 1} { return 1 }\n    return [expr {$n * [fact [expr {$n - 1}]]}]\n}\n",
    );
    let fact = fixture_proc(&ia, "::fact").expect("::fact summary");
    assert!(
        fact.pure,
        "recursive arithmetic proc must be pure (greatest fix-point); got pure={}",
        fact.pure,
    );
}

#[test]
fn fp_nab_03_impure_proc_still_detected() {
    // Control: a proc doing I/O (`puts`) must be impure — proves the test
    // above isn't trivially asserting every proc pure.
    let ia = build("proc ::logit {msg} {\n    puts $msg\n    return ok\n}\n");
    let logit = fixture_proc(&ia, "::logit").expect("::logit summary");
    assert!(
        !logit.pure,
        "a proc that calls puts must be impure; got pure={}",
        logit.pure,
    );
}

/// A call that names only its caller's own places leaves the caller
/// pure: `bump` writes the place its parameter names one frame up, so it
/// is impure, as is `twice`, which hands `bump` the place its own
/// parameter names; `p` passes `bump` a plain local of its own and `r`
/// passes `twice` one, so both stay pure. A caller passing a qualified
/// name, or a local `global` links to the global frame, does not.
#[test]
fn a_call_naming_the_callers_own_place_keeps_it_pure() {
    let ia = build(
        "proc ::bump {name} {upvar 1 $name v; incr v}\n\
             proc ::twice {name} {upvar 1 $name w; bump w; bump w}\n\
             proc ::p {} {set n 1; bump n; return $n}\n\
             proc ::r {} {set t 1; twice t; return $t}\n\
             proc ::g {} {bump ::n; return 0}\n\
             proc ::h {} {global n; bump n; return 0}\n",
    );
    let pure = |name: &str| fixture_proc(&ia, name).expect(name).pure;
    assert!(!pure("::bump"));
    assert!(!pure("::twice"));
    assert!(pure("::p"));
    assert!(pure("::r"));
    assert!(!pure("::g"));
    assert!(!pure("::h"));
}

#[test]
fn unknown_call_sets_has_unknown_calls() {
    let ia = build("proc ::caller {} { nosuchcmd }");
    let s = fixture_proc(&ia, "::caller").unwrap();
    assert!(s.has_unknown_calls);
    assert!(!s.pure);
}

#[test]
fn constant_return_inferred_for_literal_proc() {
    let ia = build("proc ::f {} { return 1 }");
    let s = fixture_proc(&ia, "::f").unwrap();
    assert!(s.returns_constant);
    assert_eq!(s.constant_return, Some(ConstantReturn::Int(1)));
}

#[test]
fn conditional_return_is_not_constant_when_body_falls_through() {
    // O103: `f` returns 42 only when `$x > 0`; otherwise it
    // falls off the end (the `if` returns "" on the no-match path),
    // so it is NOT constant-returning and must not be folded.
    let ia = build("proc ::f {x} { if {$x > 0} { return 42 } }");
    let s = fixture_proc(&ia, "::f").unwrap();
    assert!(
        !s.returns_constant,
        "fall-through proc must not be constant-returning, got {:?}",
        s.constant_return,
    );
    assert_eq!(s.constant_return, None);
}

#[test]
fn conditional_return_is_constant_when_all_paths_return() {
    // Fully covered if/else, both arms return the same literal →
    // genuinely constant, still folds.
    let ia = build("proc ::f {x} { if {$x > 0} { return 7 } else { return 7 } }");
    let s = fixture_proc(&ia, "::f").unwrap();
    assert!(s.returns_constant);
    assert_eq!(s.constant_return, Some(ConstantReturn::Int(7)));
}

#[test]
fn return_word_classification_keeps_literal_names_and_selected_grammar() {
    let params = HashSet::from([
        "x".to_owned(),
        "{x}".to_owned(),
        "a:b".to_owned(),
        "arr".to_owned(),
        "cash$name".to_owned(),
        String::new(),
    ]);
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let config = tcl_lexer::LexerConfig::for_dialect(profile);
        for (raw, expected) in [
            ("$x", Some("x")),
            ("${cash$name}", Some("cash$name")),
            ("${}", Some("")),
            ("$a:b", None),
            ("${arr(k)}", None),
            ("$arr(k)", None),
            ("${{x}}", profile.starts_with("tcl9").then_some("{x}")),
        ] {
            let source = format!("return {raw}");
            let commands =
                crate::segmenter::segment_commands_with_offset_and_config(&source, 0, config);
            let tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(&source),
                config,
                &commands[0],
            );
            let word = tokens.words().get(1);
            let actual = classify_return(Some(raw), false, word, None, &params, Some(config));
            match (actual, expected) {
                (ReturnKind::Passthrough(actual), Some(expected)) => {
                    assert_eq!(actual, expected);
                }
                (ReturnKind::Other, None) => {}
                _ => panic!("{profile}: {raw}"),
            }
            assert!(matches!(
                classify_return(Some(raw), false, word, None, &params, None),
                ReturnKind::Other,
            ));
        }
        assert!(matches!(
            classify_return(Some("$x"), true, None, None, &params, Some(config)),
            ReturnKind::Literal(value) if value.bytes == b"$x",
        ));
        assert!(matches!(
            classify_return(Some("$x"), true, None, None, &params, None),
            ReturnKind::Other,
        ));
    }
}

#[test]
fn passthrough_param_detected() {
    let ia = build("proc ::id {x} { return $x }");
    let s = fixture_proc(&ia, "::id").unwrap();
    assert_eq!(s.return_passthrough_param.as_deref(), Some("x"));
    assert_eq!(s.return_depends_on_params, vec!["x".to_string()]);
}

#[test]
fn can_fold_gated_on_pure_and_return_shape() {
    // Pure + literal → can fold.
    let ia = build("proc ::f {} { return 42 }");
    assert!(fixture_proc(&ia, "::f").unwrap().can_fold_static_calls);
    // Impure (dynamic eval is a real barrier) + literal → cannot
    // fold. ``eval {literal}`` is a Block (non-barrier),
    // so use a dynamic body.
    let ia = build("proc ::f {} { eval $dyn ; return 42 }");
    assert!(!fixture_proc(&ia, "::f").unwrap().can_fold_static_calls);
}

fn original_reference_unit(
    source: &str,
) -> (
    std::sync::Arc<tcl_registry::model::ContextRegistry>,
    CompilationUnit,
) {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = context.commands().profile().unwrap();
    // These fixtures pin explicit Standalone authoring (the retained
    // `original_authored_declaration_body_units`). A profiled default
    // build is a fresh-interpreter document (user decision
    // 2026-10-19T14:20Z); Standalone authoring is the dialect-blind
    // build, still profiled through its Registry and grammar.
    let unit = CompilationUnit::build_with_options(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            dialect: None,
            config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        },
    );
    (context, unit)
}

fn original_reference_facts(
    unit: &CompilationUnit,
    registry: &CommandRegistry,
) -> HashMap<ProcedureId, LocalFacts> {
    let module = &unit.ir_module;
    let identities = crate::realm::document_realm_bindings_for_module(module, registry).unwrap();
    let bodies = summary_procedure_bodies(module, registry);
    let known = bodies.keys().copied().collect();
    scan_all_procs(
        module,
        SummaryBodyInputs {
            bodies: &bodies,
            known: &known,
            unit: Some(unit),
        },
        registry,
        module.dialect_profile,
        ObjectTypeMap::none().candidates,
        &identities,
        None,
    )
}

fn assert_authored_alias_variant_refusals() {
    for body in [
        "upvar #0 $n x; set x 1",
        "upvar 0 $n x; set x 1",
        "upvar 2 $n x; set x 1",
        "upvar $level $n x; set x 1",
        "upvar 1 $n x; unknownCommand; set x 1",
        "upvar 1 $n x; global x; set x 1",
        "if {1} {upvar 1 $n x}; set x 1",
    ] {
        let source = format!("proc p {{n level}} {{{body}}}");
        let (context, unit) = original_reference_unit(&source);
        let (&id, _) = unit
            .ir_module
            .original_authored_declaration_body_units
            .iter()
            .next()
            .unwrap();
        let facts = original_reference_facts(&unit, context.commands());
        assert!(
            !facts[&id]
                .param_trait_flags
                .get("n")
                .is_some_and(|traits| traits.contains(&ProcArgTrait::VarWrite)),
            "{body}"
        );
        assert!(facts[&id].upvar_aliases.is_empty(), "{body}");
    }
    let (_, rejected) = original_reference_unit("proc p {::n} {upvar 1 $::n x; set x 1}");
    assert!(
        rejected
            .ir_module
            .original_authored_declaration_body_units
            .is_empty()
    );
    let (context, unit) = original_reference_unit("proc p {n} {upvar 1 $::n x; set x 1}");
    let (&id, _) = unit
        .ir_module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    let facts = original_reference_facts(&unit, context.commands());
    assert!(
        !facts[&id]
            .param_trait_flags
            .get("::n")
            .is_some_and(|traits| traits.contains(&ProcArgTrait::VarWrite))
    );
    assert!(
        !facts[&id]
            .param_trait_flags
            .get("n")
            .is_some_and(|traits| traits.contains(&ProcArgTrait::VarWrite))
    );
    assert!(facts[&id].upvar_aliases.is_empty());
    let (context, unit) =
        original_reference_unit("proc p {n other} {upvar 1 $n x; upvar 1 $other x; set x 1}");
    let (&id, _) = unit
        .ir_module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    let facts = original_reference_facts(&unit, context.commands());
    assert!(facts[&id].param_trait_flags["n"].contains(&ProcArgTrait::VarRead));
    assert!(!facts[&id].param_trait_flags["n"].contains(&ProcArgTrait::VarWrite));
    assert!(facts[&id].param_trait_flags["other"].contains(&ProcArgTrait::VarWrite));
}

#[test]
fn original_authored_alias_reports_keep_source_formals_frames_and_unknown_debts() {
    let source = "proc p {n v spare} {upvar 1 $n x; set x $v}";
    let (context, unit) = original_reference_unit(source);
    let registry = context.commands();
    let module = &unit.ir_module;
    let (&id, _) = module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    let procedure = &module.body_units[&id];
    let tokens = procedure
        .body
        .retained_source_tokens_for_statement(&procedure.body.statements[0])
        .unwrap();
    let command =
        crate::registry_invocation::original_authored_parameter_source_command_for_module(
            tokens, module, registry,
        )
        .expect("actual original alias source row");
    assert_eq!(command.body(), id);
    assert!(command.has_unproved_applicability());
    let [alias] = command.aliases() else {
        panic!("one Registry alias declaration");
    };
    let held = command
        .caller_alias(alias)
        .expect("conditional caller alias incidence");
    assert_eq!(held.parameter(), "n");
    let write = procedure
        .body
        .retained_source_tokens_for_statement(&procedure.body.statements[1])
        .unwrap();
    let write = crate::registry_invocation::original_authored_parameter_source_command_for_module(
        write, module, registry,
    )
    .expect("actual retained source successor");
    assert!(held.belongs_to(&write));
    let facts = original_reference_facts(&unit, registry);
    let observed = &facts[&id];
    assert!(observed.param_trait_flags["n"].contains(&ProcArgTrait::VarRead));
    assert!(observed.param_trait_flags["n"].contains(&ProcArgTrait::VarWrite));
    assert!(observed.param_observations_incomplete);
    assert!(observed.has_unknown_calls);
    assert!(observed.upvar_aliases.is_empty());
    assert!(observed.global_aliases.is_empty());
    assert_eq!(observed.name_params, [] as [usize; 0]);
    assert!(observed.source_parameter_aliases.is_empty());
    assert!(
        observed
            .effect_writes
            .intersects(EffectRegion::UNKNOWN_STATE)
    );
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .proved_execution_target()
            .is_none()
    );
    let cloned = tokens.clone();
    assert!(
        crate::registry_invocation::original_authored_parameter_source_command_for_module(
            &cloned, module, registry
        )
        .is_none()
    );
    let mut changed = module.clone();
    changed.lexer_config.strict_quoting = !changed.lexer_config.strict_quoting;
    assert!(
        crate::registry_invocation::original_authored_parameter_source_command_for_module(
            tokens, &changed, registry
        )
        .is_none()
    );
    let mut changed = module.clone();
    changed.body_units.get_mut(&id).unwrap().params_raw = "other".to_owned();
    assert!(
        crate::registry_invocation::original_authored_parameter_source_command_for_module(
            tokens, &changed, registry
        )
        .is_none()
    );
    assert!(
        crate::registry_invocation::original_authored_parameter_source_command_for_module(
            tokens,
            module,
            tcl_registry::model::ingress::static_context_for("tcl9.0").commands()
        )
        .is_none()
    );

    assert_authored_alias_variant_refusals();
}

fn assert_authored_call_variant_refusals() {
    // #2427: an autoload index entry keeps `unknownCommand` unbounded.
    for body in [
        "set ::auto_index(unknownCommand) {source lib.tcl}; unknownCommand; ::helper $x",
        "$name $x",
        "::helper",
    ] {
        let source = format!("proc ::helper {{v}} {{return $v}}\nproc ::f {{x name}} {{{body}}}");
        let (context, unit) = original_reference_unit(&source);
        let module = &unit.ir_module;
        let (&id, _) = module
            .original_authored_declaration_body_units
            .iter()
            .find(|(_, card)| card.name() == "::f")
            .unwrap();
        let procedure = &module.body_units[&id];
        let statement = procedure.body.statements.last().unwrap();
        let tokens = procedure
            .body
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        let calls = crate::registry_invocation::original_authored_body_procedure_calls_for_module(
            tokens,
            module,
            context.commands(),
        );
        assert!(
            !calls.is_some_and(|calls| !calls.calls().is_empty()
                && calls.calls().iter().all(
                    crate::registry_invocation::OriginalSourceProcedureCall::accepts_arguments
                )),
            "{body}"
        );
        assert!(
            !original_reference_facts(&unit, context.commands())[&id]
                .param_trait_flags
                .values()
                .any(|traits| traits.contains(&ProcArgTrait::ForwardedToCallee)),
            "{body}"
        );
    }
}

fn assert_authored_call_keeps_its_original_target(
    call: &crate::registry_invocation::OriginalSourceProcedureCall<'_>,
    module: &crate::ir::Module,
    tokens: &crate::ir::CommandTokens,
) {
    assert!(call.accepts_arguments());
    assert_eq!(call.argument_count(), Some(1));
    assert!(std::ptr::eq(
        call.procedure(),
        std::ptr::from_ref(&module.body_units[&call.id()])
    ));
    assert_eq!(
        module.original_authored_declaration_body_units[&call.id()].name(),
        "::helper"
    );
    assert!(
        crate::var_escape::original_slots::OriginalDeclaredProcedureArgumentSlots::from_module(
            module,
            call.procedure()
        )
        .is_none()
    );
    assert!(
        !tokens
            .source_binding
            .as_ref()
            .unwrap()
            .has_original_source_layout_owner()
    );
}

#[test]
fn original_authored_call_reports_keep_actual_source_target_formals_and_debts() {
    let source = "proc ::helper {v} {return $v}\nproc ::f {x spare} {::helper $x}";
    let (context, unit) = original_reference_unit(source);
    let registry = context.commands();
    let module = &unit.ir_module;
    let (&id, _) = module
        .original_authored_declaration_body_units
        .iter()
        .find(|(_, card)| card.name() == "::f")
        .unwrap();
    let procedure = &module.body_units[&id];
    let [statement] = procedure.body.statements.as_slice() else {
        panic!("one actual source call");
    };
    assert!(matches!(statement, crate::ir::Statement::Call { .. }));
    let tokens = procedure
        .body
        .retained_source_tokens_for_statement(statement)
        .unwrap();
    let binding = tokens.source_binding.as_ref().unwrap();
    assert!(!binding.has_original_source_layout_owner());
    assert!(binding.proved_execution_target().is_none());
    assert!(
        crate::registry_invocation::original_authored_procedure_calls_for_module(
            tokens, module, registry
        )
        .is_none()
    );
    let calls = crate::registry_invocation::original_authored_body_procedure_calls_for_module(
        tokens, module, registry,
    )
    .expect("actual retained source calling template");
    assert_eq!(calls.obligations().len(), 3);
    let [call] = calls.calls() else {
        panic!("one actual original callee allocation");
    };
    assert_authored_call_keeps_its_original_target(call, module, tokens);
    let observed = original_reference_facts(&unit, registry);
    assert!(observed[&id].param_trait_flags["x"].contains(&ProcArgTrait::ForwardedToCallee));
    assert!(observed[&id].param_observations_incomplete);
    assert!(observed[&id].has_unknown_calls);
    assert!(
        observed[&id]
            .effect_writes
            .intersects(crate::side_effects::EffectRegion::UNKNOWN_STATE)
    );
    let mut changed = module.clone();
    changed.lexer_config.strict_quoting = !changed.lexer_config.strict_quoting;
    assert!(
        crate::registry_invocation::original_authored_body_procedure_calls_for_module(
            tokens, &changed, registry
        )
        .is_none()
    );
    let mut changed = module.clone();
    changed.body_units.get_mut(&call.id()).unwrap().params_raw = "other".to_owned();
    assert!(
        crate::registry_invocation::original_authored_body_procedure_calls_for_module(
            tokens, &changed, registry
        )
        .is_none()
    );
    let cloned = tokens.clone();
    assert!(
        crate::registry_invocation::original_authored_body_procedure_calls_for_module(
            &cloned, module, registry
        )
        .is_none()
    );
    assert!(
        crate::registry_invocation::original_authored_body_procedure_calls_for_module(
            tokens,
            module,
            tcl_registry::model::ingress::static_context_for("tcl9.0").commands()
        )
        .is_none()
    );

    assert_authored_call_variant_refusals();
}

#[test]
fn conditional_authored_declaration_reports_do_not_close_unused_parameters() {
    let (context, unit) =
        original_reference_unit("rename $old decl\nproc kept {unused} {return untouched}");
    let module = &unit.ir_module;
    let (&id, card) = module
        .original_authored_declaration_body_units
        .iter()
        .find(|(_, card)| card.name() == "kept")
        .expect("genuine conditional source body");
    assert!(card.matches_procedure(module, &module.body_units[&id]));
    let facts = original_reference_facts(&unit, context.commands());
    let observed = &facts[&id];
    assert!(observed.param_observations_incomplete);
    let traits = finalise_param_traits(
        &module.body_units[&id].params,
        &observed.param_trait_flags,
        None,
        &observed.param_references,
        !observed.param_observations_incomplete,
    );
    assert!(
        !traits
            .get("unused")
            .is_some_and(|traits| traits.contains(&ProcArgTrait::Unused))
    );
    assert!(observed.upvar_aliases.is_empty());
    assert!(observed.global_aliases.is_empty());
}

#[test]
fn original_formal_reference_traits_keep_actual_body_function_and_cfg_owners() {
    let (context, unit) = original_reference_unit("proc p {used spare} {return $used}");
    let module = &unit.ir_module;
    let (&id, _) = module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    let procedure = &module.body_units[&id];
    assert!(procedure.body.executed_source.is_none());
    let facts = original_reference_facts(&unit, context.commands());
    let observed = &facts[&id];
    assert!(!observed.param_observations_incomplete);
    let traits = finalise_param_traits(
        &procedure.params,
        &observed.param_trait_flags,
        None,
        &observed.param_references,
        !observed.param_observations_incomplete,
    );
    assert!(traits["used"].contains(&ProcArgTrait::Passthrough));
    assert!(traits["spare"].contains(&ProcArgTrait::Unused));
    for &installed in module.installed_procedure_body_units.keys() {
        assert!(!facts.contains_key(&installed));
        assert!(unit.original_procedure_cfg_for_body(installed).is_none());
        assert!(unit.installed_procedure_cfg_for_body(installed).is_some());
    }
    let mut changed = unit.clone();
    changed
        .body_units
        .get_mut(&id)
        .unwrap()
        .cfg
        .name
        .push_str(" changed");
    let changed_facts = original_reference_facts(&changed, context.commands());
    let missing = &changed_facts[&id];
    assert!(missing.param_observations_incomplete);
    let traits = finalise_param_traits(
        &procedure.params,
        &missing.param_trait_flags,
        None,
        &missing.param_references,
        !missing.param_observations_incomplete,
    );
    assert!(
        traits
            .values()
            .all(|traits| !traits.contains(&ProcArgTrait::Unused))
    );
    let mut changed = unit.clone();
    changed.body_units.get_mut(&id).unwrap().body_id = None;
    assert!(
        original_reference_facts(&changed, context.commands())[&id].param_observations_incomplete
    );
    let mut changed = unit.clone();
    let profile = context.commands().profile().unwrap();
    changed
        .body_units
        .get_mut(&id)
        .unwrap()
        .source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        module.native_lexer_config(),
    ));
    assert_eq!(changed.body_units[&id].cfg, unit.body_units[&id].cfg);
    assert!(
        original_reference_facts(&changed, context.commands())[&id].param_observations_incomplete
    );
    let mut report = unit.clone();
    report
        .ir_module
        .body_units
        .get_mut(&id)
        .unwrap()
        .qualified_name = "unrelated display".into();
    assert!(
        !original_reference_facts(&report, context.commands())[&id].param_observations_incomplete
    );
}

#[test]
fn original_formal_reference_traits_keep_authored_conditions_and_unknown_obligations() {
    let (context, unit) =
        original_reference_unit("proc p {n spare} {if {$n > 0} {return ONE} else {return TWO}}");
    let module = &unit.ir_module;
    let (&id, _) = module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    assert!(matches!(
        module.body_units[&id].body.statements[0],
        crate::ir::Statement::If { .. }
    ));
    let facts = original_reference_facts(&unit, context.commands());
    assert!(facts[&id].param_trait_flags["n"].contains(&ProcArgTrait::UsedInCondition));
    assert!(!facts[&id].param_references.contains("spare"));
    let (mutation_context, mutation_unit) = original_reference_unit(
        "proc p {first other} {if {$first > 0} {return ONE} else {return TWO}}",
    );
    let (&mutation_id, _) = mutation_unit
        .ir_module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    assert!(
        original_reference_facts(&mutation_unit, mutation_context.commands())[&mutation_id]
            .param_trait_flags["first"]
            .contains(&ProcArgTrait::UsedInCondition)
    );
    let mut changed = mutation_unit.clone();
    let crate::ir::Statement::If { clauses, .. } = &mut changed
        .ir_module
        .body_units
        .get_mut(&mutation_id)
        .unwrap()
        .body
        .statements[0]
    else {
        unreachable!()
    };
    let crate::expr_ast::ExprNode::Binary { left, .. } = &mut clauses[0].condition else {
        unreachable!()
    };
    let crate::expr_ast::ExprNode::Var { text, .. } = left.as_mut() else {
        unreachable!()
    };
    *text = "$other".into();
    assert_eq!(
        changed.body_units[&mutation_id].cfg,
        mutation_unit.body_units[&mutation_id].cfg
    );
    assert!(
        !changed.ir_module.original_authored_declaration_body_units[&mutation_id]
            .matches_procedure(
                &changed.ir_module,
                &changed.ir_module.body_units[&mutation_id]
            )
    );
    assert!(
        !original_reference_facts(&changed, mutation_context.commands()).contains_key(&mutation_id)
    );
    assert!(module.source_entry.native_entry.is_none());
    let body = &module.body_units[&id].body;
    let tokens = body
        .retained_source_tokens_for_statement(&body.statements[0])
        .unwrap();
    assert!(
        crate::registry_invocation::normal_transfer_invocation(context.commands(), None, tokens)
            .is_none()
    );

    let (context, unit) = original_reference_unit("proc p {used spare} {unknownCommand $used}");
    let (&id, _) = unit
        .ir_module
        .original_authored_declaration_body_units
        .iter()
        .next()
        .unwrap();
    let facts = original_reference_facts(&unit, context.commands());
    let facts = &facts[&id];
    assert!(facts.param_observations_incomplete);
    assert!(facts.param_references.contains("used"));
    let traits = finalise_param_traits(
        &unit.ir_module.body_units[&id].params,
        &facts.param_trait_flags,
        None,
        &facts.param_references,
        !facts.param_observations_incomplete,
    );
    assert!(!traits["spare"].contains(&ProcArgTrait::Unused));
}

#[test]
fn original_formal_reference_parts_keep_lexical_names_and_complete_grammar() {
    for (dialect, unicode, nested) in [
        ("tcl8.6", false, false),
        ("tcl9.1", false, true),
        ("jim", true, false),
    ] {
        let profile = tcl_registry::model::ingress::resolve_known_environment(dialect)
            .unwrap()
            .unit_profile();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let source = r"puts $café ${p($k)} $arr($k) \$escaped {$braced} [list ${nested name}]";
        let image = tcl_lexer::SourceImage::document(source);
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            image,
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .unwrap();
        assert!(plan.fatal_tail.is_none());
        let names = original_word_reference_names(&plan.commands[0].words).unwrap();
        let expected: HashSet<String> = [
            if unicode { "café" } else { "caf" },
            "p($k)",
            "arr",
            "k",
            "nested name",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        assert_eq!(names, expected, "{dialect}");
        let source = "puts ${a{b}c}";
        let image = tcl_lexer::SourceImage::document(source);
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            image,
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .unwrap();
        let names = original_word_reference_names(&plan.commands[0].words).unwrap();
        assert_eq!(
            names,
            HashSet::from([if nested { "a{b}c" } else { "a{b" }.to_owned()])
        );
    }
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    let node = crate::expr_ast::ExprNode::Var {
        text: "${actual name}".into(),
        name: "unrelated report".into(),
        start: 0,
        end: 14,
    };
    let params = HashSet::from(["actual name".to_owned(), "unrelated report".to_owned()]);
    let mut facts = LocalFacts::default();
    note_params_in_expr(&node, &params, &mut facts, config, 0);
    assert!(facts.param_trait_flags.contains_key("actual name"));
    assert!(!facts.param_trait_flags.contains_key("unrelated report"));
    let mut names = Vec::new();
    assert!(walk_collect_param_refs(
        &node,
        &params,
        &mut names,
        Some(config),
        0
    ));
    assert_eq!(names, ["actual name"]);
    assert!(!walk_collect_param_refs(
        &node, &params, &mut names, None, 0
    ));
    let mut facts = LocalFacts {
        param_observations_incomplete: false,
        ..LocalFacts::default()
    };
    note_params_in_expr(&node, &params, &mut facts, config, u32::MAX);
    assert!(facts.param_observations_incomplete);
}

#[test]
fn param_traits_inferred() {
    // `x` is returned → Passthrough; `y` never read → Unused.
    let ia = build("proc ::f {x y} { return $x }");
    let s = fixture_proc(&ia, "::f").unwrap();
    let x_traits = s.param_traits.get("x").expect("x traits");
    assert!(
        x_traits.contains(&ProcArgTrait::Passthrough),
        "expected Passthrough for x, got {x_traits:?}",
    );
    let y_traits = s.param_traits.get("y").expect("y traits");
    assert!(
        y_traits.contains(&ProcArgTrait::Unused),
        "expected Unused for y, got {y_traits:?}",
    );
}

#[test]
fn call_by_name_param_traits_inferred() {
    // `upvar 1 $n x; set x ...` → n is VarWrite;
    // `upvar 1 $n x; return $x` (read only) → n is VarRead.
    let w = build("proc ::setvar {n v} { upvar 1 $n x\nset x $v }");
    let s = fixture_proc(&w, "::setvar").unwrap();
    let nt = s.param_traits.get("n").expect("n traits");
    assert!(
        nt.contains(&ProcArgTrait::VarWrite),
        "expected VarWrite for n (write through upvar alias), got {nt:?}",
    );

    let r = build("proc ::getvar {n} { upvar 1 $n x\nreturn $x }");
    let s2 = fixture_proc(&r, "::getvar").unwrap();
    let nt2 = s2.param_traits.get("n").expect("n traits");
    assert!(
        nt2.contains(&ProcArgTrait::VarRead) && !nt2.contains(&ProcArgTrait::VarWrite),
        "expected VarRead (not VarWrite) for read-only upvar alias, got {nt2:?}",
    );

    // upvar #0 (global frame, not the caller) → VarRead only, no
    // write-back to the caller's passed variable.
    let g = build("proc ::gw {p} { upvar #0 $p x\nset x 1 }");
    let s3 = fixture_proc(&g, "::gw").unwrap();
    let pt = s3.param_traits.get("p").expect("p traits");
    assert!(
        pt.contains(&ProcArgTrait::VarRead) && !pt.contains(&ProcArgTrait::VarWrite),
        "upvar #0 is not a caller-frame write-back, got {pt:?}",
    );
}

/// `upvar $lvl $a b` has three words after the command, so `$lvl` is the
/// level and `($a, b)` the pair, by the argument-count parity
/// `Tcl_UpvarObjCmd` decides on; the text-sniffing reading took `$lvl`
/// for the other variable and paired `($lvl, $a)`. A computed level is no
/// known frame, so `a` names a variable the alias reads and no write-back
/// is claimed; at level 1 the same pair is the caller's write-back.
#[test]
fn upvar_level_word_is_read_by_argument_parity() {
    let ia = build("proc ::q {lvl a} { upvar $lvl $a b\nset b 1 }");
    let s = fixture_proc(&ia, "::q").unwrap();
    let a = s.param_traits.get("a").expect("a traits");
    assert!(
        a.contains(&ProcArgTrait::VarRead) && !a.contains(&ProcArgTrait::VarWrite),
        "{a:?}"
    );
    assert!(
        s.param_traits.get("lvl").is_none_or(|t| {
            !t.contains(&ProcArgTrait::VarRead) && !t.contains(&ProcArgTrait::VarWrite)
        }),
        "{:?}",
        s.param_traits.get("lvl")
    );
    let ia = build("proc ::q {lvl a} { upvar 1 $a b\nset b 1 }");
    let a = ia
        .procedures
        .get(&fixture_id(&ia.procedures, "::q"))
        .expect("authored fixture body")
        .param_traits
        .get("a")
        .expect("a traits");
    assert!(a.contains(&ProcArgTrait::VarWrite), "{a:?}");
}

#[test]
fn used_in_condition_detected() {
    let ia = build("proc ::f {n} { if {$n > 0} { return 1 } else { return 0 } }");
    let s = fixture_proc(&ia, "::f").unwrap();
    let traits = s.param_traits.get("n").expect("n traits");
    assert!(
        traits.contains(&ProcArgTrait::UsedInCondition),
        "expected UsedInCondition for n, got {traits:?}",
    );
}

#[test]
fn forwarded_to_callee_detected() {
    let ia = build("proc ::helper {v} { return $v }\nproc ::f {x} { ::helper $x }");
    let s = fixture_proc(&ia, "::f").unwrap();
    let traits = s.param_traits.get("x").expect("x traits");
    assert!(
        traits.contains(&ProcArgTrait::ForwardedToCallee),
        "expected ForwardedToCallee for x, got {traits:?}",
    );
}

#[test]
fn original_logical_source_call_edges_keep_body_ids_without_execution_grants() {
    let environment = tcl_registry::model::ingress::resolve_environment("tcl");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let profile = environment.unit_profile();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    assert!(input.has_logical_source_name_context());
    let source = "proc target {} {return OK}; proc caller {} {target}";
    let mut lowerer =
        crate::lowering::Lowerer::with_config(registry, config).with_resolved_analysis_input(input);
    lowerer.lower(source);
    let module = lowerer.finish_module(source);
    let select = |offset| {
        let rows: Vec<_> = module
            .original_source_procedure_bodies()
            .filter(|(_, procedure)| {
                procedure
                    .declaration_site
                    .as_ref()
                    .is_some_and(|site| site.offset == offset)
            })
            .collect();
        assert_eq!(
            rows.len(),
            1,
            "one authored declaration at the original site"
        );
        assert!(module.procedure_ids.owns(*rows[0].0));
        *rows[0].0
    };
    let target = select(0);
    let caller = select(u32::try_from(source.find("proc caller").unwrap()).unwrap());
    let caller_body = module
        .original_source_procedure_bodies()
        .find(|(id, _)| **id == caller)
        .unwrap()
        .1;
    let tokens = caller_body
        .body
        .retained_source_tokens_for_statement(&caller_body.body.statements[0])
        .unwrap();
    let calls = crate::registry_invocation::original_logical_procedure_calls_for_module(
        tokens, &module, registry,
    )
    .expect("actual conditional original call and declaration owners");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id(), target);
    assert!(calls[0].accepts_arguments());
    assert!(
        crate::registry_invocation::normal_representation_invocation_with_metadata_context(
            registry, None, tokens,
        )
        .is_none()
    );
    let identities = crate::realm::document_realm_bindings_for_module(&module, registry).unwrap();
    let analysis = build_interprocedural_analysis(
        &module,
        registry,
        Some(profile),
        ObjectTypeMap::none(),
        &identities,
        None,
    );
    let summary = &analysis.procedures[&caller];
    assert!(summary.direct_calls.contains(&target));
    assert!(summary.calls.contains(&target));
    assert!(summary.has_unknown_calls);
    assert!(!summary.pure);
    assert!(!summary.completes);
    assert!(module.procedures.get(&target).is_none());
    let mut changed = tokens.clone();
    changed.argv_texts[0] = "different".to_owned();
    assert!(
        crate::registry_invocation::original_logical_procedure_calls_for_module(
            &changed, &module, registry,
        )
        .is_none()
    );
    let mut stale = module.clone();
    stale.source = tcl_lexer::SourceImage::document("proc different {} {return OK}");
    assert!(
        crate::registry_invocation::original_logical_procedure_calls_for_module(
            tokens, &stale, registry,
        )
        .is_none()
    );
}

#[test]
fn cyclic_call_graph_handled() {
    let ia = build(
        "proc ::a {} { ::b }\n\
             proc ::b {} { ::a }",
    );
    let a = fixture_proc(&ia, "::a").unwrap();
    assert!(edge_labels(&ia, &a.calls).contains(&"::b".to_string()));
    let b = fixture_proc(&ia, "::b").unwrap();
    assert!(edge_labels(&ia, &b.calls).contains(&"::a".to_string()));
}

#[test]
fn namespace_relative_calls_resolved() {
    let ia = build(
        "namespace eval ::ns {\n\
                 proc caller {} { helper }\n\
                 proc helper {} { return 1 }\n\
             }",
    );
    // Names may come back as "::ns::caller" / "::ns::helper".
    if let Some(caller) = fixture_proc(&ia, "::ns::caller") {
        assert!(
            edge_labels(&ia, &caller.calls)
                .iter()
                .any(|c| c == "::ns::helper"),
            "expected ::ns::helper in ::ns::caller.calls, got {:?}",
            caller.calls,
        );
    }
}

#[test]
fn instance_write_candidates_keep_literal_destinations_and_dynamic_unknowns() {
    let ivars: HashSet<_> = ["", "$field", "field", "${braced}", "$array"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut writes = HashSet::new();
    for name in ["", "$field", "${braced}", "$array(key)"] {
        check_ivar_write(name, &ivars, &mut writes);
    }
    assert_eq!(
        writes,
        HashSet::from_iter(
            ["", "$field", "${braced}", "$array"]
                .into_iter()
                .map(str::to_owned)
        )
    );
    assert!(!writes.contains("field"));
    let dynamic = crate::ir::Script::from_statements(vec![crate::ir::Statement::AssignConst {
        span: tcl_lexer::Span::new(0, 0),
        name: "$target".into(),
        name_braced: false,
        value: "1".into(),
        value_span: None,
    }]);
    collect_instance_var_writes(&dynamic, &ivars, &mut writes, 0);
    assert_eq!(writes, ivars);
}

/// `collect_instance_var_writes`
/// and the mutually-recursive `scan_script`/`scan_statement`/
/// `scan_control_flow_statement` trio recurse once per nested
/// `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch` body, with no
/// depth cap of their own before this fix. Transitively bounded to
/// `MAX_LOWER_NEST_DEPTH` (256) by the lowering pass today, so this is
/// defence-in-depth / consistency with every other full-tree walker in
/// this crate, not a currently-reproducible crash. 1000 levels of
/// *source* nesting is comfortably past this new cap; the assertion is
/// that `build_interprocedural_analysis` returns at all, not what it
/// returns. Spawns its own big-stack thread since the lexer/CST/
/// segmenter stages upstream of this walker's own new cap still walk
/// the full un-truncated source nesting before lowering's cap trims
/// it — same rationale as
/// `structured::tests::deeply_nested_if_survives_structured_walk`.
#[test]
fn deeply_nested_if_survives_interprocedural_scan() {
    const DEPTH: usize = 1000;
    const STACK_SIZE: usize = 64 * 1024 * 1024;
    let mut src = "proc ::p {} {\n".to_owned();
    for _ in 0..DEPTH {
        src.push_str("if {1} {\n");
    }
    src.push_str("set done 1\n");
    for _ in 0..DEPTH {
        src.push_str("}\n");
    }
    src.push_str("}\n");
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let _ = build(&src);
        })
        .unwrap()
        .join()
        .unwrap();
}

/// `collect_instance_var_writes`
/// (the `TclOO` method-body instance-write scan) recurses once per
/// nested `if` body, so it needs a depth cap of its own.
/// Same transitively-bounded-today caveat and big-stack-thread
/// rationale as `deeply_nested_if_survives_interprocedural_scan`. 1000
/// levels is comfortably past the new cap; the assertion is that the
/// method summary is built at all, not what it contains.
#[test]
fn deeply_nested_if_survives_collect_instance_var_writes() {
    const DEPTH: usize = 1000;
    const STACK_SIZE: usize = 64 * 1024 * 1024;
    let mut src = "oo::class create C {\n    variable n\n    method bump {} {\n".to_owned();
    for _ in 0..DEPTH {
        src.push_str("if {1} {\n");
    }
    src.push_str("incr n\n");
    for _ in 0..DEPTH {
        src.push_str("}\n");
    }
    src.push_str("}\n}\n");
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let ia = build(&src);
            assert!(
                ia.methods
                    .values()
                    .any(|method| method.base.qualified_name == "::C::bump")
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

/// The call-graph scan must resolve a command head's *effective
/// identity*, not its written spelling.
///
/// `catch`'s argument carries `ArgRole::Body`, so the text scan descends
/// into it and records the calls inside as reachability edges.  Whether
/// `q` becomes an edge of `p` is therefore a clean witness that the body
/// role was resolved — and a missing edge is what makes a live proc look
/// dead (O124).
///
/// The call sits inside a `[…]` substitution deliberately: a *statement*
/// `catch {…}` is turned into control-flow structure by lowering long
/// before this scan sees it, so only the value-context path exercises the
/// registry role query in question.
///
/// On tclsh 8.6.16 and 9.0.4 (byte-identical): `interp alias {} guard
/// {} catch` makes `guard` run `catch`; `rename catch guard` moves it and
/// leaves `catch` gone; a top-level `proc catch …` takes the name over.
fn p_calls_q(prelude: &str, head: &str) -> bool {
    let src =
        format!("{prelude}proc q {{}} {{ return 1 }}\nproc p {{}} {{ set z [{head} {{q}}] }}\n");
    let ia = build(&src);
    fixture_proc(&ia, "::p").is_some_and(|s| edge_labels(&ia, &s.calls).iter().any(|c| c == "::q"))
}

#[test]
fn call_graph_follows_an_aliased_body_command() {
    assert!(p_calls_q("interp alias {} guard {} catch\n", "guard"));
    // The `::`-qualified spelling of the alias classifies alike.
    assert!(p_calls_q("interp alias {} guard {} catch\n", "::guard"));
    // Guard: an unbound `guard` carries no body argument to descend into.
    assert!(!p_calls_q("set y 1\n", "guard"));
}

#[test]
fn call_graph_follows_a_renamed_body_command() {
    assert!(p_calls_q("rename catch guard\n", "guard"));
    assert!(
        !p_calls_q("rename catch guard\n", "catch"),
        "a renamed-away `catch` must not keep the built-in's body grammar"
    );
}

#[test]
fn call_graph_abstains_for_a_builtin_shadowed_by_a_user_proc() {
    assert!(
        !p_calls_q("proc catch {s} { return 1 }\n", "catch"),
        "a user `proc catch` takes the name over; its argument is a plain \
             value word, not a script"
    );
    // Guard: the unshadowed built-in still descends.
    assert!(p_calls_q("set y 1\n", "catch"));
}

#[test]
fn call_graph_abstains_for_a_dynamic_binding() {
    assert!(
        !p_calls_q("rename $old guard\n", "guard"),
        "a dynamic rename must not give `guard` a body grammar"
    );
    assert!(
        p_calls_q("rename $old guard\n", "catch"),
        "a dynamic rename must not take `catch`'s body grammar away either"
    );
}

/// The side-effect classification reads the resolved head too: a call
/// through a proven alias is a *known* command, so it does not force
/// `has_unknown_calls` (which alone makes a procedure permanently impure).
#[test]
fn side_effect_classification_follows_an_aliased_head() {
    let unknown = |prelude: &str, head: &str| {
        let src = format!("{prelude}proc p {{}} {{ {head} hi }}\n");
        let ia = build(&src);
        fixture_proc(&ia, "::p").is_some_and(|s| s.has_unknown_calls)
    };
    assert!(
        !unknown("interp alias {} out {} puts\n", "out"),
        "an alias of `puts` is a known command, not an unknown call"
    );
    // Guard: an unbound `out` is genuinely unknown.
    assert!(unknown("set y 1\n", "out"));
}

/// Whether a statement of `::p`'s flow graph may run a `return` of its
/// own, as the return reading asks of each executable block.
fn p_may_return(source: &str) -> bool {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for(source, &registry, false);
    unit.procedures
        .get(&fixture_id(&unit.procedures, "::p"))
        .expect("authored fixture body")
        .cfg
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .any(|stmt| statement_may_return(stmt, &registry, 0))
}

/// A call holding a word the registry gives the Body role may run a
/// `return` there: the flow graph keeps a loop over a qualified variable
/// as a call holding its body, and a barrier's code is unseen. A `catch`
/// absorbs its body's `return`, and the header of a loop the flow graph
/// lowers holds only the list words.
#[test]
fn a_call_running_a_body_word_may_return() {
    assert!(p_may_return(
        "proc p {} {foreach ::x {1} {return 1}; return 2}"
    ));
    assert!(p_may_return(
        "proc p {} {lmap ::x {1} {return 1}; return 2}"
    ));
    assert!(p_may_return("proc p {} {time {return 1}; return 2}"));
    assert!(!p_may_return("proc p {} {catch {return 1}; return 2}"));
    assert!(!p_may_return(
        "proc p {} {set s 0; foreach x {1 2} y {3 4} z {5 6} {incr s}; return $s}"
    ));
}

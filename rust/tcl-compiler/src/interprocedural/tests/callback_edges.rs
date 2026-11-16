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

//! Call-graph edges from callbacks: command prefixes, traces, registry-declared invokers and instance methods.

use super::*;

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

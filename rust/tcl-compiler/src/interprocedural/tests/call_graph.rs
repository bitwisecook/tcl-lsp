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

//! Direct call edges: alias prefixes, `apply` lambda bodies, logical source calls, cycles, namespaces and rebinding of a body command.

use super::*;

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
fn simple_proc_is_recorded_with_params_and_arity() {
    let ia = build("proc ::greet {name} { puts hi }");
    let s = fixture_proc(&ia, "::greet").expect("proc summary");
    assert_eq!(s.params, vec!["name".to_string()]);
    assert_eq!(s.arity, Arity::exact(1));
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

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

//! Literal bodies held in locals: propagation into `eval` and `uplevel`, retention and invalidation.

use super::*;

#[test]
fn const_prop_uplevel_resolves_set_var_body() {
    // ``set body {set x 1}; uplevel 1 $body`` inside a
    // proc — the const-map records ``body`` and the uplevel
    // folds in the literal as an UpFrame.
    let m = lower_to_ir("proc f {} { set body {set x 1}\n uplevel 1 $body }", &reg());
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    // proc body: [AssignConst(body), UpFrame { body: [...] }]
    let last = proc.body.statements.last().expect("body has statements");
    match last {
        Statement::UpFrame { body, .. } => {
            assert!(!body.statements.is_empty(), "expected lowered body");
        }
        other => panic!("expected UpFrame, got {other:?}"),
    }
}

#[test]
fn const_prop_eval_resolves_set_var_body() {
    // ``eval $body`` with const-mapped body folds to a
    // Statement::Block.
    let m = lower_to_ir("proc f {} { set body {set x 1}\n eval $body }", &reg());
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    let last = proc.body.statements.last().expect("body has statements");
    match last {
        Statement::Block { body, .. } => {
            assert!(!body.statements.is_empty(), "expected lowered block");
        }
        other => panic!("expected Block, got {other:?}"),
    }
}

#[test]
fn const_prop_eval_brace_body_emits_block() {
    // ``eval {body}`` in a proc context lowers to Block.
    let m = lower_to_ir("proc f {} { eval {set x 1} }", &reg());
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    assert!(matches!(proc.body.statements[0], Statement::Block { .. }));
}

#[test]
fn const_prop_disabled_at_top_level() {
    // The const-map is gated on ``proc_depth > 0``. At
    // top-level, ``set body {set x 1}; uplevel 1 $body``
    // does NOT relax — the uplevel falls back to default
    // dispatch (Call / Barrier).
    let m = lower_to_ir("set body {set x 1}\nuplevel 1 $body", &reg());
    // statements[0] = AssignConst(body), [1] = uplevel call
    assert!(matches!(
        m.top_level.statements[1],
        Statement::Call { .. } | Statement::Barrier { .. }
    ));
}

#[test]
fn const_prop_invalidated_on_reassignment() {
    // ``set body {a}; set body $other; uplevel 1 $body`` — the
    // second ``set`` invalidates the binding (RHS is a $var,
    // not a brace literal), so the uplevel can't fold.
    let m = lower_to_ir(
        "proc f {} { set body {a}\n set body $other\n uplevel 1 $body }",
        &reg(),
    );
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    let last = proc.body.statements.last().expect("body");
    assert!(
        !matches!(last, Statement::UpFrame { .. }),
        "expected fallback after re-assignment, got {last:?}",
    );
}

#[test]
fn const_prop_eval_list_literal() {
    // ``eval [list set x 42]`` recognised as static body.
    let m = lower_to_ir("proc f {} { eval [list set x 42] }", &reg());
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    assert!(matches!(proc.body.statements[0], Statement::Block { .. }));
}

#[test]
fn const_prop_eval_list_with_dynamic_arg_rejected() {
    // ``eval [list set x $v]`` — dynamic ``\$v`` rejects the
    // list-literal shape. Falls back to runtime barrier.
    let m = lower_to_ir("proc f {} { eval [list set x $v] }", &reg());
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    assert!(!matches!(proc.body.statements[0], Statement::Block { .. }));
}

#[test]
fn const_prop_eval_non_list_command_rejected() {
    // ``eval [foo arg]`` — inner command isn't ``list``;
    // can't synthesise a body. Falls back to runtime barrier.
    let m = lower_to_ir("proc f {} { eval [foo arg] }", &reg());
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    assert!(!matches!(proc.body.statements[0], Statement::Block { .. }));
}

#[test]
fn const_prop_does_not_leak_into_nested_proc() {
    // A ``set body {literal}`` in the outer proc must
    // NOT appear to a nested ``proc inner``'s
    // barrier-relaxation gate as a tracked literal.
    let m = lower_to_ir(
        "proc outer {} {\n  set body {set x 1}\n  proc inner {} { uplevel 1 $body }\n}",
        &reg(),
    );
    let inner = fixture_id_if_present(&m.procedures, "::inner")
        .and_then(|id| m.procedures.get(&id))
        .expect("inner registered");
    // The inner uplevel must remain a Call/Barrier — NOT an
    // UpFrame. If the const-map leaked, the inner body would
    // be folded as UpFrame { body: [set x 1], .. }.
    let last = inner.body.statements.last().expect("body");
    assert!(
        !matches!(last, Statement::UpFrame { .. }),
        "outer scope's const-map must not leak into nested proc, got {last:?}",
    );
}

#[test]
fn const_prop_inherited_into_catch_body() {
    // Child scope (catch body) inherits parent's const-map.
    let m = lower_to_ir(
        "proc f {} { set body {set x 1}\n catch { uplevel 1 $body } }",
        &reg(),
    );
    let proc = fixture_id_if_present(&m.procedures, "::f")
        .and_then(|id| m.procedures.get(&id))
        .expect("proc registered");
    // Look for a Catch wrapping an UpFrame.
    let catch_stmt = proc
        .body
        .statements
        .iter()
        .find(|s| matches!(s, Statement::Catch { .. }))
        .expect("expected Catch");
    if let Statement::Catch { body, .. } = catch_stmt {
        assert!(
            body.statements
                .iter()
                .any(|s| matches!(s, Statement::UpFrame { .. })),
            "expected UpFrame inside catch body, got {body:?}",
        );
    }
}

#[test]
fn local_body_literal_reference_lookup_uses_whole_scalar_source_geometry() {
    let registry = reg();
    let mut lowerer = Lowerer::new(&registry);
    lowerer.proc_depth = 1;
    lowerer.const_map_stack.push(HashMap::from([(
        "body".to_owned(),
        "return original".to_owned(),
    )]));
    for reference in ["$body", "${body}"] {
        assert_eq!(
            lowerer.const_map_lookup(reference).as_deref(),
            Some("return original")
        );
    }
    for reference in [
        "body",
        "$body suffix",
        "${body}suffix",
        "$body(index)",
        "${body(index)}",
        "$",
        "${body",
        "$::body",
    ] {
        assert_eq!(lowerer.const_map_lookup(reference), None, "{reference}");
    }
    lowerer.proc_depth = 0;
    assert_eq!(lowerer.const_map_lookup("$body"), None);
}

#[test]
fn local_body_literal_retention_requires_actual_assignment_and_decodes_the_written_key() {
    let config = tcl_lexer::LexerConfig::default();
    let source = r"set b\x6fdy {return original}";
    let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let seg = &commands[0];
    let assignment = Statement::AssignConst {
        span: seg.span,
        name: seg.args()[0].clone(),
        name_braced: false,
        value: seg.args()[1].clone(),
        value_span: None,
    };
    assert_eq!(
        assigned_local_body_literal(seg, &assignment, config),
        Some(("body".to_owned(), "return original".to_owned()))
    );
    let opaque = Statement::Barrier {
        span: seg.span,
        reason: "opaque invocation".to_owned(),
        command: "set".to_owned(),
        canonical_command: None,
        args: seg.args().to_vec(),
        tokens: None,
    };
    assert_eq!(assigned_local_body_literal(seg, &opaque, config), None);
}

#[test]
fn local_body_literal_invalidation_distinguishes_literal_sigils_and_decodes_write_keys() {
    let config = tcl_lexer::LexerConfig::default();
    let mut scope = HashMap::from([("body".to_owned(), "return original".to_owned())]);
    let write = |name: &str, name_braced: bool| Statement::AssignConst {
        span: tcl_lexer::Span::new(0, 0),
        name: name.to_owned(),
        name_braced,
        value: "replacement".to_owned(),
        value_span: None,
    };
    invalidate_const_map_for(&write("$body", true), &mut scope, config);
    assert!(scope.contains_key("body"));
    invalidate_const_map_for(&write("::body", true), &mut scope, config);
    assert!(scope.contains_key("body"));
    invalidate_const_map_for(&write(r"b\x6fdy", false), &mut scope, config);
    assert!(scope.is_empty());
    scope.insert("body".to_owned(), "return original".to_owned());
    invalidate_const_map_for(&write("body(index)", true), &mut scope, config);
    assert!(scope.is_empty());
    scope.insert("body".to_owned(), "return original".to_owned());
    invalidate_const_map_for(&write("$computed", false), &mut scope, config);
    assert!(scope.is_empty());
}

#[test]
fn local_body_literal_generic_call_with_empty_defs_withdraws_all_retained_values() {
    let config = tcl_lexer::LexerConfig::default();
    let registry = reg();
    let mut lowerer = Lowerer::new(&registry);
    let source = "unknown_writer $computed replacement";
    lowerer.word_space = WordSpace::new(source, 0, lowerer.source_channel);
    let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let call = lowerer.lower_default(&commands[0], "::");
    assert!(matches!(&call, Statement::Call { defs, .. } if defs.is_empty()));
    let mut scope = HashMap::from([("body".to_owned(), "return original".to_owned())]);
    invalidate_const_map_for(&call, &mut scope, config);
    assert!(scope.is_empty());
}

#[test]
fn local_body_literal_quiet_logical_source_retains_factory_names_and_bodies() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let source = "proc factory {} {set name {LiteralChild}; set body {return original}; proc $name {} $body}";
    let mut lowerer = Lowerer::with_config(context.commands(), input.lexer_config())
        .with_resolved_analysis_input(input);
    lowerer.lower(source);
    let module = lowerer.finish_module(source);
    let child = fixture_id_if_present(&module.procedures, "::LiteralChild")
        .and_then(|id| module.procedures.get(&id))
        .expect("genuine quiet Logical source retains the original child");
    assert_eq!(child.body_source.as_deref(), Some("return original"));
    assert!(module.source_entry.native_entry.is_none());
}

#[test]
fn local_body_literal_opcode_without_original_quiet_source_owner_cannot_retain_contents() {
    let registry = reg();
    let source = "set body {return original}";
    let mut lowerer = Lowerer::new(&registry);
    lowerer.proc_depth = 1;
    lowerer.const_map_stack.push(HashMap::from([(
        "previous".to_owned(),
        "return stale".to_owned(),
    )]));
    lowerer.word_space = WordSpace::new(source, 0, lowerer.source_channel);
    let commands =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, lowerer.config);
    let seg = &commands[0];
    let synthetic = Statement::AssignConst {
        span: seg.span,
        name: seg.args()[0].clone(),
        name_braced: false,
        value: seg.args()[1].clone(),
        value_span: None,
    };
    lowerer.update_const_map(seg, &synthetic);
    assert!(lowerer.const_map_stack.last().unwrap().is_empty());
    assert_eq!(lowerer.original_native_written_argument_value(seg, 1), None);
}

#[test]
fn local_body_literal_expression_and_value_evaluation_withdraw_unclosed_effects() {
    let registry = reg();
    let config = tcl_lexer::LexerConfig::default();
    for source in [
        "expr {[overwrite]}",
        "expr {$watched}",
        "set other [overwrite]",
        "set other $watched",
        "incr other $watched",
    ] {
        let module = lower_to_ir(source, &registry);
        let statement = module
            .top_level
            .statements
            .first()
            .expect("original statement");
        assert!(!matches!(statement, Statement::AssignConst { .. }));
        let mut scope = HashMap::from([("body".to_owned(), "return original".to_owned())]);
        invalidate_const_map_for(statement, &mut scope, config);
        assert!(scope.is_empty(), "unclosed effects: {source}");
    }
}

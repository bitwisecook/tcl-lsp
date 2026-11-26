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

//! Lowering plain scripts into blocks: straight-line code, `return`, `if`, structured writes and loop initialisers.

use super::*;

#[test]
fn structured_cfg_writes_keep_literal_name_data_and_dynamic_unknowns() {
    for (name, root) in [
        ("", ""),
        ("$counter", "$counter"),
        ("${braced}", "${braced}"),
        ("$array(key)", "$array"),
    ] {
        let statement = Statement::AssignConst {
            span: Span::new(0, 0),
            name: name.into(),
            name_braced: true,
            value: "1".into(),
            value_span: None,
        };
        assert_eq!(
            flow_facts_stmt(&statement).0,
            BTreeSet::from([root.to_owned()])
        );
        let mutation = Statement::Incr {
            span: Span::new(0, 0),
            name: name.into(),
            name_braced: true,
            amount: Some("1".into()),
            amount_braced: true,
            safe_on_uninit: false,
        };
        let registry = CommandRegistry::build_default();
        assert_eq!(
            CfgBuilder::new(true, &registry).init_written_names(&mutation),
            Some(vec![root.to_owned()])
        );
    }
    let dynamic = Statement::Incr {
        span: Span::new(0, 0),
        name: "$counter".into(),
        name_braced: false,
        amount: Some("1".into()),
        amount_braced: true,
        safe_on_uninit: false,
    };
    assert!(flow_facts_stmt(&dynamic).0.is_empty());
    let registry = CommandRegistry::build_default();
    assert_eq!(
        CfgBuilder::new(true, &registry).init_written_names(&dynamic),
        None
    );
}

fn native_for_statement(source: &str, registry: &CommandRegistry) -> Statement {
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let module = crate::lowering::lower_procedure_target_module_for_bytecode_with_options(
        tcl_runtime_api::ProcedureCompileTarget {
            source,
            namespace: "",
            parameters: &[],
        },
        registry,
        config,
        Some(profile),
        false,
        None,
    );
    let statement = module.top_level.statements.into_iter().next().unwrap();
    let Statement::For { raw_tokens, .. } = &statement else {
        panic!("original bytecode for grammar: {statement:?}");
    };
    assert!(raw_tokens.as_deref().is_some());
    statement
}

#[test]
fn for_initialisation_preserves_exact_scalar_bindings() {
    let registry = native_registry();
    let builder = CfgBuilder::new(true, &registry);
    for (name, scalar) in [
        ("a", true),
        ("$a", true),
        ("", true),
        ("a(key)", false),
        ("$a(key)", false),
    ] {
        let source = format!("for {{set {{{name}}} {{010}}}} {{1}} {{}} {{}}");
        let Statement::For { init, .. } = native_for_statement(&source, &registry) else {
            unreachable!();
        };
        let bindings = builder.for_initialisation_bindings(&init);
        assert_eq!(bindings.len(), usize::from(scalar), "{source}");
        if scalar {
            assert!(
                matches!(bindings.get(name), Some(crate::tcl_expr_eval::EnvValue::Str(value)) if value.bytes == "010" && value.numeric.is_none()),
                "{source}: {bindings:?}"
            );
        }
    }
}

#[test]
fn for_initialisation_declines_unproved_conditions() {
    let registry = CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let builder = CfgBuilder::new(true, &registry);
    for (initial, condition, expected) in [
        ("010", "1", true),
        ("010", "$i == 8", false),
        ("NaN", "$i", false),
        ("", "NaN", false),
        ("{x}y", "1 ni $i", false),
        (
            "01000000000000000000000",
            "$i != 9223372036854775808",
            false,
        ),
    ] {
        let source = format!("for {{set i {{{initial}}}}} {{{condition}}} {{}} {{}}");
        assert_eq!(
            builder.for_runs_at_least_once(&native_for_statement(&source, &registry)),
            expected,
            "{source}",
        );
    }
}

#[test]
fn empty_script_produces_entry_exit() {
    let func = build_test_cfg_function("::test", &Script::new(), true);
    assert_eq!(func.name, "::test");
    assert!(func.blocks.contains_key(&func.entry));
    // entry → exit
    assert!(func.blocks.len() >= 2);
}

#[test]
fn linear_script() {
    let script = Script::from_statements(vec![
        Statement::AssignConst {
            span: Span::new(0, 7),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        },
        Statement::AssignConst {
            span: Span::new(8, 15),
            name: "y".into(),
            name_braced: false,
            value: "2".into(),
            value_span: None,
        },
    ]);
    let func = build_test_cfg_function("::test", &script, true);
    // Entry block should have both statements.
    let entry = &func.blocks[&func.entry];
    assert_eq!(entry.statements.len(), 2);
}

#[test]
fn return_terminates() {
    let script = Script::from_statements(vec![
        Statement::AssignConst {
            span: Span::new(0, 7),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        },
        Statement::Return {
            expr_base: None,
            tokens: None,
            span: Span::new(8, 16),
            value: Some("$x".into()),
            value_word: None,
            expr: None,
            command_binding: None,
            braced: false,
        },
        Statement::AssignConst {
            span: Span::new(17, 24),
            name: "y".into(),
            name_braced: false,
            value: "2".into(), // dead code
            value_span: None,
        },
    ]);
    let func = build_test_cfg_function("::test", &script, true);
    let entry = &func.blocks[&func.entry];
    // Only one statement before the return terminator.
    assert_eq!(entry.statements.len(), 1);
    assert!(matches!(entry.terminator, Some(Terminator::Return { .. })));
}

#[test]
fn if_creates_branches() {
    let script = Script::from_statements(vec![Statement::If {
        span: Span::new(0, 30),
        clauses: vec![IfClause {
            condition: ExprNode::Var {
                text: "$x".into(),
                name: "x".into(),
                start: 0,
                end: 2,
            },
            condition_span: Span::new(3, 5),
            body: Script::from_statements(vec![Statement::AssignConst {
                span: Span::new(7, 14),
                name: "y".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            }]),
            body_span: Span::new(6, 15),
            condition_base: None,
        }],
        else_body: None,
        else_span: None,
    }]);
    let func = build_test_cfg_function("::test", &script, true);
    // Should have at least: entry, if_then, if_next, if_end, exit
    assert!(func.blocks.len() >= 4);
    // Entry block should have a Branch terminator.
    let entry = &func.blocks[&func.entry];
    assert!(
        matches!(entry.terminator, Some(Terminator::Branch { .. })),
        "entry should branch; got {:?}",
        entry.terminator
    );
}

#[test]
fn build_cfg_module() {
    let module = Module::default();
    let cfg = build_cfg(&module, false);
    assert_eq!(cfg.top_level.name, "::top");
    assert!(cfg.procedures.is_empty());
}

#[test]
fn foreach_synthetic_call_records_iterator_group_sizes() {
    // ``foreach {a b} L1 c L2 { … }`` has two iterator groups
    // — the first binds two vars, the second binds one.  The
    // synthesised header `Statement::Call` must record the
    // group sizes via ``foreach_groups`` so codegen can
    // reconstruct the original pairing.
    let body = Script::from_statements(vec![Statement::AssignConst {
        span: Span::new(0, 0),
        name: "x".into(),
        name_braced: false,
        value: "1".into(),
        value_span: None,
    }]);
    let script = Script::from_statements(vec![Statement::Foreach {
        span: Span::new(0, 0),
        iterators: vec![
            ForeachIterator {
                vars: vec!["a".into(), "b".into()],
                list_arg: "L1".into(),
                list_braced: false,
            },
            ForeachIterator {
                vars: vec!["c".into()],
                list_arg: "L2".into(),
                list_braced: false,
            },
        ],
        body,
        body_span: Span::new(0, 0),
        is_lmap: false,
        raw_args: vec![],
        is_dict_iteration: false,
        is_array_iteration: false,
        raw_tokens: None,
    }]);
    let func = build_test_cfg_function("::test", &script, true);
    let mut found_groups: Option<Vec<usize>> = None;
    for block in func.blocks.values() {
        for stmt in &block.statements {
            if let Statement::Call {
                command,
                foreach_groups,
                ..
            } = stmt
                && command == "foreach"
            {
                found_groups = foreach_groups.clone();
            }
        }
    }
    assert_eq!(found_groups, Some(vec![2, 1]));
}

#[test]
fn try_handler_return_options_terminates() {
    // A `try` handler ending in `return -code error …` (lowered to a
    // "return with options" barrier) returns from the proc, so it must
    // *terminate* its block rather than fall through to the post-`try`
    // join. Regression for the spurious `try_handler → try_end` edge +
    // phi (e.g. `auto_mkindex`). Analysis builds only (`build_cfg` sets
    // `faithful_exceptions`); codegen leaves the barrier as a plain stmt.
    let module =
        lower_module("proc f {} { try { set a 1 } on error {} { return -code error boom } }");
    let cfg = build_cfg(&module, false);
    let func = fixture_id_if_present(&cfg.procedures, "::f")
        .and_then(|id| cfg.procedures.get(&id))
        .expect("::f cfg");
    let handler = func
        .blocks
        .values()
        .find(|b| b.name.starts_with("try_handler"))
        .expect("try_handler block");
    assert!(
        matches!(
            handler.terminator,
            Some(Terminator::Complete {
                route: tcl_registry::completion_route::InvocationCompletionRoute::Return(
                    tcl_registry::completion_route::ReturnCompletionRoute {
                        eventual_code: tcl_registry::CompletionCode::Error,
                        remaining_level: 1,
                    }
                ),
                ..
            })
        ),
        "return-options handler must terminate (no fall-through to try_end), got {:?}",
        handler.terminator,
    );
}

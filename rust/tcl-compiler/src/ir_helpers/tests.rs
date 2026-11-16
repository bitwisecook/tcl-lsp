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

use crate::interprocedural::test_body_identity::fixture_id;

use super::*;
use crate::ir::{ForeachIterator, IfClause, Script};
use tcl_lexer::Span;

#[test]
fn residual_body_wrapper_does_not_repeat_argument_substitutions() {
    let registry = CommandRegistry::build_default();
    let mut statement = Statement::Call {
        span: Span::new(0, 16),
        command: "puts".to_owned(),
        canonical_command: None,
        args: vec!["[set x 1]".to_owned()],
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: Some(CommandTokens::marker(
            crate::ir::SyntheticMarker::EvaluatedArguments,
        )),
        foreach_groups: None,
    };
    assert_eq!(
        evaluated_command_substitution_surfaces(&statement, &registry).texts,
        ["[set x 1]"]
    );
    statement.tokens_mut().unwrap().synthetic = Some(crate::ir::SyntheticMarker::EvaluatedWrapper);
    assert_eq!(
        evaluated_command_substitution_surfaces(&statement, &registry).texts,
        [] as [&str; 0]
    );
}

#[test]
fn defs_from_empty_script() {
    let script = Script::new();
    assert_eq!(defs_from_ir_script(&script), [] as [std::string::String; 0]);
}

/// `expr_has_command` and
/// `collect_expr_commands` each recurse once per `ExprNode` level, so both
/// need a depth cap. A tree built directly is unbounded (the
/// Pratt parser caps its own output at 256) and empirically overflowed
/// the native stack (SIGABRT) in the low thousands of levels on a 2 MiB
/// thread. 3000 is past that crash range and past `MAX_EXPR_NODE_DEPTH`
/// (256); the assertion is that each returns at all.
#[test]
fn deeply_nested_expr_command_walks_survive() {
    use crate::expr_ast::UnaryOp;
    let mut node = ExprNode::Command {
        text: "[x]".into(),
        start: 0,
        end: 3,
    };
    for _ in 0..3000 {
        node = ExprNode::Unary {
            op: UnaryOp::Not,
            operand: Box::new(node),
        };
    }
    let _ = expr_has_command(&node);
    let mut cmds = Vec::new();
    collect_expr_commands(&node, &mut cmds);
}

#[test]
fn defs_from_assign() {
    let script = Script::from_statements(vec![Statement::AssignConst {
        span: Span::new(0, 7),
        name: "x".into(),
        name_braced: false,
        value: "1".into(),
        value_span: None,
    }]);
    assert_eq!(defs_from_ir_script(&script), vec!["x"]);
}

#[test]
fn defs_from_call_defs() {
    let script = Script::from_statements(vec![Statement::Call {
        span: Span::new(0, 10),
        command: "set".into(),
        canonical_command: None,
        args: vec!["x".into(), "1".into()],
        defs: vec!["x".into()],
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }]);
    assert_eq!(defs_from_ir_script(&script), vec!["x"]);
}

#[test]
fn defs_from_nested_if() {
    let script = Script::from_statements(vec![Statement::If {
        span: Span::new(0, 30),
        clauses: vec![IfClause {
            condition: ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            },
            condition_span: Span::new(3, 4),
            body: Script::from_statements(vec![Statement::AssignConst {
                span: Span::new(6, 13),
                name: "y".into(),
                name_braced: false,
                value: "2".into(),
                value_span: None,
            }]),
            body_span: Span::new(5, 14),
            condition_base: None,
        }],
        else_body: None,
        else_span: None,
    }]);
    assert_eq!(defs_from_ir_script(&script), vec!["y"]);
}

/// `collect_defs_from_script`
/// recurses once per nested `If`/`For`/`While`/`Foreach`/`Catch`/`Try`/
/// `Switch` body, so it needs a depth cap of its own.
/// Transitively bounded to `MAX_LOWER_NEST_DEPTH` (256) by the lowering
/// pass today, so this is defence-in-depth / consistency with every
/// other full-tree walker in this crate, not a currently-reproducible
/// crash. 2000 levels is comfortably past this new cap; the assertion
/// is that `defs_from_ir_script` returns at all, not what it returns.
#[test]
fn deeply_nested_if_survives_defs_from_ir_script() {
    const DEPTH: usize = 2000;
    let leaf = Statement::AssignConst {
        span: Span::new(0, 0),
        name: "leaf".into(),
        name_braced: false,
        value: "1".into(),
        value_span: None,
    };
    let mut script = Script::from_statements(vec![leaf]);
    for _ in 0..DEPTH {
        script = Script::from_statements(vec![Statement::If {
            span: Span::new(0, 0),
            clauses: vec![IfClause {
                condition: ExprNode::Literal {
                    text: "1".into(),
                    start: 0,
                    end: 1,
                },
                condition_span: Span::new(0, 0),
                body: script,
                body_span: Span::new(0, 0),
                condition_base: None,
            }],
            else_body: None,
            else_span: None,
        }]);
    }

    // 2000 levels of nesting exceeds the new 256-deep cap, so the
    // innermost `leaf` def is never reached — the assertion is that
    // this returns at all (no stack overflow), not what it returns.
    let _ = defs_from_ir_script(&script);
}

#[test]
fn defs_from_foreach() {
    let script = Script::from_statements(vec![Statement::Foreach {
        span: Span::new(0, 30),
        iterators: vec![ForeachIterator {
            vars: vec!["k".into(), "v".into()],
            list_arg: "$d".into(),
            list_braced: false,
        }],
        body: Script::new(),
        body_span: Span::new(20, 25),
        is_lmap: false,
        raw_args: vec![],
        is_dict_iteration: false,
        is_array_iteration: false,
        raw_tokens: None,
    }]);
    assert_eq!(defs_from_ir_script(&script), vec!["k", "v"]);
}

#[test]
fn ir_destination_inventory_keeps_literal_data_and_refuses_computed_names() {
    let assign = |name: &str, braced| Statement::AssignConst {
        span: Span::new(0, 0),
        name: name.into(),
        name_braced: braced,
        value: "VALUE".into(),
        value_span: None,
    };
    let script = Script::from_statements(vec![
        assign("$literal", true),
        assign("${literal}", true),
        assign("", true),
        assign("arr($index)", true),
        assign("$computed", false),
        Statement::Foreach {
            span: Span::new(0, 0),
            iterators: vec![ForeachIterator {
                vars: vec![
                    "$iterator".into(),
                    "${iterator}".into(),
                    String::new(),
                    "array(key)".into(),
                ],
                list_arg: "ONE TWO THREE FOUR".into(),
                list_braced: true,
            }],
            body: Script::from_statements(vec![assign("nested", false)]),
            body_span: Span::new(0, 0),
            is_lmap: false,
            raw_args: Vec::new(),
            is_dict_iteration: false,
            is_array_iteration: false,
            raw_tokens: None,
        },
    ]);
    assert_eq!(
        defs_from_ir_script(&script),
        [
            "$literal",
            "${literal}",
            "",
            "arr($index)",
            "$iterator",
            "${iterator}",
            "",
            "array(key)",
            "nested"
        ]
    );
    assert!(structural_variable_write_projection(&script.statements[4]).opaque_variable_frame);
}

#[test]
fn defs_from_catch() {
    let script = Script::from_statements(vec![Statement::Catch {
        span: Span::new(0, 30),
        body: Script::from_statements(vec![Statement::AssignConst {
            span: Span::new(7, 14),
            name: "inner".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        }]),
        body_span: Span::new(6, 15),
        result_var: Some("result".into()),
        options_var: None,
        raw_args: vec![],
        tokens: None,
    }]);
    let d = defs_from_ir_script(&script);
    assert!(d.contains(&"inner".to_string()));
    assert!(d.contains(&"result".to_string()));
}

#[test]
fn tokenise_simple_command() {
    let words = tokenise_to_words("set x 1", &CommandRegistry::build_default());
    let texts: Vec<&str> = words.iter().map(|w| w.text.as_str()).collect();
    assert_eq!(texts, vec!["set", "x", "1"]);
    assert!(words.iter().all(|w| !w.substituted));
}

#[test]
fn tokenise_keeps_both_spellings_and_the_substitution_flag() {
    let words = tokenise_to_words("set $x {a $b}", &CommandRegistry::build_default());
    let texts: Vec<&str> = words.iter().map(|w| w.text.as_str()).collect();
    let raws: Vec<&str> = words.iter().map(|w| w.raw.as_str()).collect();
    // The reconstructed text canonicalises the substitution to its braced
    // spelling (`$x` → `${x}`, the same form an IR statement's arguments
    // carry) and de-braces a braced word; raw keeps what was written. A
    // delimited word retains its complete written wrapper independently
    // of the variable/component span convention.
    assert_eq!(texts, vec!["set", "${x}", "a $b"]);
    assert_eq!(raws, vec!["set", "$x", "{a $b}"]);
    assert_eq!(
        words.iter().map(|w| w.substituted).collect::<Vec<_>>(),
        vec![false, true, false]
    );
    assert!(words[2].braced_literal, "`{{a $b}}` is a literal word");
}

#[test]
fn tokenise_preserves_release_aware_argument_expansion() {
    let config_for = |dialect| {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        LexerConfig::from_grammar(registry.profile().unwrap().grammar)
    };

    let tcl90 = tokenise_command_words("cmd {*}$x {*}{a b}", config_for("tcl9.0"));
    let words = &tcl90[0];
    assert_eq!(
        words
            .iter()
            .map(CommandWord::invocation_word)
            .map(InvocationWord::kind)
            .collect::<Vec<_>>(),
        vec![
            tcl_registry::InvocationWordKind::Literal,
            tcl_registry::InvocationWordKind::Expanded,
            tcl_registry::InvocationWordKind::Expanded,
        ]
    );

    let tcl84 = tokenise_command_words("cmd {*}$x {*}{a b}", config_for("tcl8.4"));
    assert!(
        tcl84[0].iter().all(|word| !word.expanded),
        "Tcl 8.4 predates argument expansion"
    );
}

#[test]
fn source_substitution_roles_and_footprints_use_retained_availability() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional source roles/footprints do not prove dispatch or a Native cell.
    let mut registry = CommandRegistry::build_default();
    registry.insert(tcl_registry::CommandSpec {
        name: "gated-expr",
        surface: registry.get("dict").unwrap().surface,
        ..registry.get("expr").unwrap().clone()
    });
    registry.insert(tcl_registry::CommandSpec {
        name: "gated-write",
        surface: registry.get("dict").unwrap().surface,
        ..registry.get("set").unwrap().clone()
    });
    let registry = std::sync::Arc::new(registry);
    let source = "set result [gated-expr {[gated-write {$scalar(open} VALUE]}]";
    let module = crate::lowering::lower_to_ir(source, &registry);
    let statement = &module.top_level.statements[0];
    for (environment, available) in [("tcl8.4", false), ("tcl9.0", true)] {
        let context = tcl_registry::model::ingress::static_context_for(environment)
            .with_command_store(std::sync::Arc::clone(&registry));
        let inventory = evaluated_command_substitutions_with_metadata_context(
            statement,
            &registry,
            Some((&context).into()),
            LexerConfig::default(),
        );
        let nested: Vec<_> = inventory
            .all_commands()
            .filter(|words| words.first().and_then(CommandWord::literal) == Some("gated-write"))
            .collect();
        assert_eq!(nested.len(), usize::from(available), "{environment}");
        if available {
            let footprint = variable_write_effects_from_commands_with_metadata_context(
                nested,
                &registry,
                Some((&context).into()),
            );
            assert_eq!(footprint.names, ["$scalar(open"]);
            assert!(!footprint.opaque);
        } else {
            assert!(inventory.opaque);
            let words =
                tokenise_command_words("gated-write {$scalar(open} VALUE", LexerConfig::default());
            let footprint = variable_write_effects_from_commands_with_metadata_context(
                &words,
                &registry,
                Some((&context).into()),
            );
            assert_eq!(footprint.names, [] as [std::string::String; 0]);
            assert!(footprint.opaque);
        }
    }
    assert!(
        evaluated_command_substitutions_with_metadata_context(
            statement,
            &registry,
            None,
            LexerConfig::default()
        )
        .opaque
    );
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0");
    assert!(
        evaluated_command_substitutions_with_metadata_context(
            statement,
            &registry,
            Some(foreign.into()),
            LexerConfig::default()
        )
        .opaque
    );
}

#[test]
fn source_substitution_descent_retains_config_and_target_refusal() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let registry = std::sync::Arc::new(CommandRegistry::build_default());
    let context = tcl_registry::model::ingress::static_context_for("tcl9.0")
        .with_command_store(std::sync::Arc::clone(&registry));
    let module = crate::lowering::lower_to_ir("set result [list {*}{a b}]", &registry);
    let statement = &module.top_level.statements[0];
    for (environment, expanded) in [("tcl8.4", false), ("tcl9.0", true)] {
        let config = LexerConfig::from_grammar(
            tcl_registry::model::ingress::static_context_for(environment)
                .commands()
                .profile()
                .unwrap()
                .grammar,
        );
        let inventory = evaluated_command_substitutions_with_metadata_context(
            statement,
            &registry,
            Some((&context).into()),
            config,
        );
        assert_eq!(
            inventory.all_commands().flatten().any(|word| word.expanded),
            expanded
        );
    }
    let module = crate::lowering::lower_to_ir("set result [expr {[incr hidden]}]", &registry);
    let refuse = |_head: &str| None;
    let inventory = evaluated_command_substitutions_with_heads_and_metadata_context(
        &module.top_level.statements[0],
        &registry,
        Some(&refuse),
        Some((&context).into()),
        LexerConfig::default(),
    );
    assert!(inventory.opaque);
    assert!(
        !inventory
            .all_commands()
            .any(|words| words.first().and_then(CommandWord::literal) == Some("incr"))
    );
}

#[test]
fn variable_write_projection_marks_expanded_set_opaque() {
    let registry = CommandRegistry::build_default();
    let commands = tokenise_command_words("set {*}{x 99}", LexerConfig::default());
    let effects = variable_write_effects_from_commands(&commands, &registry);
    assert_eq!(effects.names, [] as [std::string::String; 0]);
    assert!(effects.opaque);
}

#[test]
fn variable_write_projection_keeps_all_trailing_regexp_targets() {
    let registry = CommandRegistry::build_default();
    let commands = tokenise_command_words(
        "regexp {(a)(b)} ab whole first second",
        LexerConfig::default(),
    );
    let effects = variable_write_effects_from_commands(&commands, &registry);
    assert_eq!(effects.names, ["whole", "first", "second"]);
    assert!(!effects.opaque);
}

#[test]
fn collect_expr_command_nodes() {
    let expr = ExprNode::Command {
        text: "[set x 1]".into(),
        start: 0,
        end: 9,
    };
    let mut cmds = Vec::new();
    collect_expr_commands(&expr, &mut cmds);
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0], "[set x 1]");
}

#[test]
fn compiled_word_surfaces_only_execute_when_unbraced() {
    let registry = CommandRegistry::build_default();
    let commands = |braced| {
        expression_command_substitutions(
            &ExprNode::CompiledWord {
                text: "prefix[set x 1]".into(),
                braced,
            },
            &registry,
            None,
        )
        .commands
    };

    assert_eq!(commands(false)[0][0].text, "set");
    assert_eq!(commands(true).len(), 0);
}

/// The commands of `surface` a statement's substitution recovers, under
/// the 8.6 registry.
fn recovered(surface: &str) -> EvaluatedCommandSubstitutions {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    command_substitutions_in_surfaces_with_replay(&[surface], false, false, registry, None, None)
}

/// The head spelling of each command, sorted.
fn heads<'a>(commands: impl IntoIterator<Item = &'a Vec<CommandWord>>) -> Vec<&'a str> {
    let mut heads: Vec<&str> = commands
        .into_iter()
        .filter_map(|words| words.first())
        .map(|word| word.text.as_str())
        .collect();
    heads.sort_unstable();
    heads
}

/// The script a `catch` protects, and the body of a `try`, run once in the
/// frame the substitution is written in, so their commands are the
/// statement's: `[catch {incr x}]` writes `x`, and so does `[catch "incr
/// x"]`, whose word substitutes nothing. Nothing else is descended by a
/// command's name — a handler, a `finally`, an `if` arm, a loop body, an
/// `eval` — and a script that is run-time data is not known text: the walk
/// is incomplete.
#[test]
fn a_protected_script_or_loop_body_is_descended_and_no_other_body_is() {
    let one = recovered("[catch {incr x}]");
    assert_eq!(heads(&one.commands), ["catch"]);
    assert_eq!(heads(&one.in_frame_commands), ["incr"]);
    assert_eq!(one.in_frame_texts, ["incr x"]);

    let two = recovered("[catch {catch {incr x}}]");
    assert_eq!(heads(&two.in_frame_commands), ["catch", "incr"]);
    assert_eq!(two.in_frame_texts, ["catch {incr x}", "incr x"]);

    let in_expr = recovered("[catch {expr {[incr x] + 1}}]");
    assert_eq!(heads(&in_expr.in_frame_commands), ["expr", "incr"]);

    let in_substitution = recovered("[catch {set y [incr x]}]");
    assert_eq!(heads(&in_substitution.in_frame_commands), ["incr", "set"]);

    let body_only = recovered("[try {incr x} on error {} {incr y} finally {incr z}]");
    assert_eq!(heads(&body_only.in_frame_commands), ["incr"]);
    assert_eq!(body_only.in_frame_texts, ["incr x"]);

    let quoted = recovered("[catch \"incr x; set y 2\"]");
    assert_eq!(heads(&quoted.in_frame_commands), ["incr", "set"]);
    assert_eq!(quoted.in_frame_texts, ["incr x; set y 2"]);
    assert!(!quoted.opaque);
    let nested_quoted = recovered("[catch \"catch {incr x}\"]");
    assert_eq!(heads(&nested_quoted.in_frame_commands), ["catch", "incr"]);

    // A loop's body runs zero or more times in this frame, so it is
    // descended as a protected script is; its header's names are bound.
    let looped = recovered("[foreach v {1} {incr x}]");
    assert_eq!(heads(&looped.protected_commands), ["incr"]);
    assert_eq!(looped.may_bound_names, ["v"]);
    let tested = recovered("[while {1} {incr x}]");
    assert_eq!(heads(&tested.protected_commands), ["incr"]);
    assert_eq!(tested.may_bound_names, Vec::<String>::new());

    for other in ["[if {1} {incr x}]", "[eval {incr x}]"] {
        let embedded = recovered(other);
        assert!(
            !heads(&embedded.in_frame_commands).contains(&"incr")
                && embedded
                    .in_frame_texts
                    .iter()
                    .all(|text| !text.contains("incr"))
                && !embedded.opaque,
            "{other}"
        );
    }
    for computed in [
        "[catch $script]",
        "[catch [list incr x]]",
        "[catch \"incr $name\"]",
        "[catch \"incr x\\n\"]",
        "[catch {*}$script]",
        "[try $script on error {} {}]",
    ] {
        let embedded = recovered(computed);
        assert!(
            embedded.opaque
                && !heads(&embedded.in_frame_commands).contains(&"incr")
                && embedded
                    .in_frame_texts
                    .iter()
                    .all(|text| !text.contains("incr")),
            "{computed}"
        );
    }
    assert!(!recovered("[catch {incr x}]").opaque);
}

#[test]
fn complete_substitution_inventory_preserves_expression_sibling_order() {
    let registry = CommandRegistry::build_default();
    let embedded = command_substitutions_in_surfaces_with_replay(
        &["[expr {[rename aaa {}; rename eval aaa; set _ 0]}] [aaa {set x 6}]"],
        false,
        false,
        &registry,
        None,
        None,
    );
    let heads = |commands: Vec<&Vec<CommandWord>>| {
        commands
            .into_iter()
            .map(|words| words[0].text.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        heads(embedded.all_commands().collect()),
        ["rename", "rename", "set", "expr", "aaa"]
    );
    assert_eq!(heads(embedded.commands.iter().collect()), ["expr", "aaa"]);
    assert_eq!(
        heads(embedded.in_frame_commands.iter().collect()),
        ["rename", "rename", "set"]
    );
    assert!(!embedded.opaque);
}

#[test]
fn standalone_expression_write_names_preserve_literal_operand_data() {
    let registry = CommandRegistry::build_default();
    for (source, expected) in [
        ("[set {$literal} 1]", "$literal"),
        ("[set {} 1]", ""),
        ("[catch {set {$literal} 1}]", "$literal"),
    ] {
        let expression = ExprNode::Command {
            text: source.to_owned(),
            start: 0,
            end: u32::try_from(source.len()).unwrap(),
        };
        let names = defs_from_expr(&expression, &registry);
        assert!(
            names.iter().any(|name| name == expected),
            "{source}: {names:?}"
        );
        assert!(!names.iter().any(|name| name == "literal"));
    }
    let expression = ExprNode::Command {
        text: "[set $selector 1]".to_owned(),
        start: 0,
        end: 17,
    };
    assert!(
        !defs_from_expr(&expression, &registry)
            .iter()
            .any(|name| name == "selector")
    );
}

#[test]
fn selected_extra_store_and_registered_trace_effects_remain_opaque() {
    use tcl_registry::world_effect::{
        EffectAccess, EffectAccessMode, EffectFootprint, InterpreterScope, NamespaceScope,
        SubjectScope, WorldEffectDescriptor, WorldStateDomain,
    };
    fn extra_store(_: tcl_registry::InvocationArguments<'_>) -> EffectFootprint {
        let mut effects = EffectFootprint::default();
        effects.add_access(EffectAccess::new(
            WorldStateDomain::VariableStore,
            EffectAccessMode::Write,
            InterpreterScope::Current,
            NamespaceScope::named("::other"),
            SubjectScope::named("foreign"),
        ));
        effects
    }
    fn coalesced_store(_: tcl_registry::InvocationArguments<'_>) -> EffectFootprint {
        let mut effects = EffectFootprint::default();
        effects.add_access(EffectAccess::variable_store_wildcard(
            EffectAccessMode::Write,
        ));
        effects
    }
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let original = tcl_registry::model::ingress::context_for_profile(profile);
    for resolver in [
        Some(extra_store as tcl_registry::world_effect::WorldEffectResolver),
        Some(coalesced_store),
        None,
    ] {
        let mut registry = original.commands().project_for_profile(profile);
        let mut setter = registry.get("set").unwrap().clone();
        setter.world_effects = Some(WorldEffectDescriptor {
            resolver,
            ..WorldEffectDescriptor::VARIABLE_WRITE
        });
        if resolver.is_none() {
            setter.arg_roles = &[];
            setter.arg_role_resolver = None;
            setter.arg_role_count_resolver = None;
            setter.arg_role_layout_resolver = None;
            setter.assigns_variable_at = None;
            setter.forms = &[];
            setter.arg_role_resolver_roles = &[];
        }
        registry.insert(setter);
        let context =
            std::sync::Arc::new(original.with_command_store(std::sync::Arc::new(registry)));
        let registry = context.commands();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input =
            crate::analyser::ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
        let module = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(registry, config)
                .for_bytecode_backend()
                .with_resolved_analysis_input(input),
            "set counter 0",
        );
        let statement = &module.top_level.statements[0];
        let tokens = module
            .top_level
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        let writes = original_statement_possible_name_writes(statement, tokens, registry);
        assert!(
            writes.opaque,
            "additional store effect cannot borrow baseline role coverage"
        );
    }
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, original.clone(), config);
    let module = crate::lowering::lower_to_ir_with(
        crate::lowering::Lowerer::with_config(original.commands(), config)
            .for_bytecode_backend()
            .with_resolved_analysis_input(input),
        "trace add variable counter write cb; set counter 0",
    );
    let statement = module.top_level.statements.last().unwrap();
    let tokens = module
        .top_level
        .retained_source_tokens_for_statement(statement)
        .unwrap();
    let variables = &tokens.source_binding.as_ref().unwrap().variable_context;
    assert!(
        variables.dynamic_traces
            || !variables.traced.is_empty()
            || !variables.trace_registrations.is_empty()
            || !variables.trace_registration_receivers.is_empty()
            || !variables.possible_trace_registrations.is_empty()
    );
    assert!(original_statement_possible_name_writes(statement, tokens, original.commands()).opaque);
}

#[test]
fn native_original_scalar_effects_keep_complete_names_and_refuse_changed_words() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let registry = context.commands();
    let (_owner, native) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(native)),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: crate::environment_ingress::authoring_native_compilation(),
        ..Default::default()
    };
    let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
        .for_bytecode_backend()
        .with_resolved_analysis_input(input);
    lowerer.set_source_analysis_options(entry.options());
    let module = crate::lowering::lower_to_ir_with(lowerer, "set counter 0");
    assert!(module.source_entry.native_entry.is_some());
    let statement = &module.top_level.statements[0];
    let tokens = module
        .top_level
        .retained_source_tokens_for_statement(statement)
        .unwrap();
    let complete = original_invocation_possible_name_writes(tokens, registry)
        .expect("actual complete Native metadata/candidate/effect owner");
    assert!(!complete.opaque);
    assert!(complete.names.iter().any(|name| name == "counter"));
    let mut writes = std::collections::BTreeSet::new();
    assert!(!crate::ssa::nested_writes(
        &module.top_level,
        registry,
        &mut writes
    ));
    assert!(writes.contains("counter"));
    let mut changed = tokens.clone();
    changed.argv_texts[1].push_str(" changed");
    assert!(original_invocation_possible_name_writes(&changed, registry).is_none());
}

#[test]
fn original_write_name_candidates_retain_unknown_effects_and_source_refusals() {
    let registry = CommandRegistry::build_default();
    let module =
        crate::lowering::lower_to_ir("proc p {} {set observed [set {$literal} 1]}", &registry);
    let (_, procedure) = module.original_source_procedure_bodies().next().unwrap();
    let statement = &procedure.body.statements[0];
    let tokens = procedure
        .body
        .retained_source_tokens_for_statement(statement)
        .unwrap();
    let names = original_statement_possible_name_writes(statement, tokens, &registry);
    assert!(names.opaque);
    assert!(names.names.iter().any(|name| name == "observed"));
    assert!(names.names.iter().any(|name| name == "$literal"));
    assert!(!names.names.iter().any(|name| name == "literal"));
    let mut changed = tokens.clone();
    changed.argv_texts[1].push_str(" changed");
    let refused = original_statement_possible_name_writes(statement, &changed, &registry);
    assert!(refused.opaque && refused.names.is_empty());
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let refused = original_statement_possible_name_writes(statement, tokens, foreign);
    assert!(refused.opaque && refused.names.is_empty());
}

#[test]
fn standalone_typed_write_coverage_preserves_primitive_data_and_nested_control() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let assign = |name: &str| Statement::AssignConst {
        span: Span::new(0, 0),
        name: name.into(),
        name_braced: true,
        value: "VALUE".into(),
        value_span: None,
    };
    let script = Script::from_statements(vec![
        assign("$literal"),
        assign(""),
        Statement::If {
            span: Span::new(0, 0),
            clauses: vec![crate::ir::IfClause {
                condition: crate::expr_parser::parse_expr("$counter < 3", None),
                condition_span: Span::new(0, 0),
                condition_base: None,
                body: Script::from_statements(vec![assign("nested")]),
                body_span: Span::new(0, 0),
            }],
            else_body: None,
            else_span: None,
        },
    ]);
    let mut names = BTreeSet::new();
    assert!(!crate::ssa::nested_writes(&script, registry, &mut names));
    assert_eq!(
        names,
        BTreeSet::from([String::new(), "$literal".into(), "nested".into()])
    );
}

#[test]
fn standalone_typed_write_coverage_keeps_calls_and_present_source_terminal() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let assign = Statement::AssignConst {
        span: Span::new(0, 0),
        name: "counter".into(),
        name_braced: false,
        value: "0".into(),
        value_span: None,
    };
    let standalone = Script::from_statements(vec![assign.clone()]);
    assert!(standalone_typed_statement_writes_are_covered(
        &standalone,
        &standalone.statements[0],
        registry
    ));
    for condition in ["[set unexpected 1]", "sin(0)", "$arr([set key 1])"] {
        let script = Script::from_statements(vec![Statement::If {
            span: Span::new(0, 0),
            clauses: vec![crate::ir::IfClause {
                condition: crate::expr_parser::parse_expr(condition, None),
                condition_span: Span::new(0, 0),
                condition_base: None,
                body: Script::new(),
                body_span: Span::new(0, 0),
            }],
            else_body: None,
            else_span: None,
        }]);
        assert!(
            crate::ssa::nested_writes(&script, registry, &mut BTreeSet::new()),
            "{condition}"
        );
    }
    let call = Script::from_statements(vec![Statement::Call {
        span: Span::new(0, 0),
        command: "set".into(),
        canonical_command: None,
        args: vec!["counter".into(), "0".into()],
        defs: vec!["counter".into()],
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }]);
    assert!(crate::ssa::nested_writes(
        &call,
        registry,
        &mut BTreeSet::new()
    ));
    let module = crate::lowering::lower_to_ir("set counter 0", registry);
    let mut source = module.top_level;
    assert!(source.executed_source.is_some());
    source.statements = vec![assign];
    source.command_binding_sites = crate::ir::CommandBindingSites::default();
    assert!(!standalone_typed_statement_writes_are_covered(
        &source,
        &source.statements[0],
        registry
    ));
    assert!(crate::ssa::nested_writes(
        &source,
        registry,
        &mut BTreeSet::new()
    ));
}

#[test]
fn defs_from_expr_set_command() {
    let reg = CommandRegistry::build_default();
    let expr = ExprNode::Command {
        text: "[set x 1]".into(),
        start: 0,
        end: 9,
    };
    let d = defs_from_expr(&expr, &reg);
    assert!(
        d.contains(&"x".to_string()),
        "should find set's VarWrite; got {d:?}"
    );
}

/// Out-vars a condition-embedded `[cmd …]` contributes, as
/// [`condition_command_out_vars`] sees them.
fn cond_out_vars(cmd_text: &str) -> Vec<String> {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let expr = ExprNode::Command {
        text: format!("[{cmd_text}]"),
        start: 0,
        end: 0,
    };
    condition_command_out_vars(&expr, registry)
}

#[test]
fn expanded_words_do_not_manufacture_condition_out_vars() {
    // Tcl 9.0.4 expands the list into extra `catch` arguments, reports
    // wrong-arity, and creates neither `x` nor `result`. Source positions
    // before expansion therefore cannot be used as registry role indices.
    assert_eq!(
        cond_out_vars("catch {*}{set x 1} result"),
        [] as [std::string::String; 0]
    );
}

/// The registry's `ArgRole::VarWrite` query must answer for every command
/// a `catch` / `scan` / `gets` / `regexp` name list would cover.
#[test]
fn registry_query_covers_the_deleted_hardcoded_four() {
    for (cmd_text, expected) in [
        ("catch {error x} msg opts", vec!["msg", "opts"]),
        ("scan $t {%d %d} a b", vec!["a", "b"]),
        ("gets $fp line", vec!["line"]),
        ("regexp {(a)(b)} $s m p q", vec!["m", "p", "q"]),
        // The switch-skipping the hand-rolled `regexp` loop did by hand
        // is the registry resolver's job now.
        ("regexp -nocase -- $re $s m", vec!["m"]),
    ] {
        let got = cond_out_vars(cmd_text);
        for name in &expected {
            assert!(
                got.iter().any(|v| v == name),
                "`{cmd_text}` must still yield `{name}`; got {got:?}"
            );
        }
    }
}

/// …and it exceeds that list: commands the hardcoded four never covered,
/// each verified against tclsh 9.0.4 / 8.6.14 first.
#[test]
fn registry_query_exceeds_the_deleted_hardcoded_four() {
    for (cmd_text, expected) in [
        // `proc bar {lst} {if {[set idx [lsearch $lst foo]] > -1} {puts $idx}}`
        // `bar {a foo b}` → 1
        ("set idx [lsearch $lst foo]", vec!["idx"]),
        // `proc loopy {} {set i 0; while {[incr i] < 3} {puts $i}}` → 1 2
        ("incr i", vec!["i"]),
        // `proc lass {lst} {lassign $lst a b; puts "$a-$b"}`
        // `lass {1 2}` → 1-2
        ("lassign $lst a b", vec!["a", "b"]),
        // `proc binscan {} {binary scan "AB" H2H2 hi lo; puts "$hi $lo"}`
        // → 41 42
        ("binary scan $d H2H2 hi lo", vec!["hi", "lo"]),
        // `regsub` writes its result variable.
        ("regsub $re $s $r out", vec!["out"]),
    ] {
        let got = cond_out_vars(cmd_text);
        for name in &expected {
            assert!(
                got.iter().any(|v| v == name),
                "`{cmd_text}` must yield `{name}`; got {got:?}"
            );
        }
    }
}

/// A `Plain` body (`catch`'s script) runs in the caller's own frame, so
/// its writes are harvested; a destroyer is not a definition.
#[test]
fn condition_out_vars_descends_plain_bodies_but_skips_destroyers() {
    let got = cond_out_vars("catch {set inner 1} msg");
    assert!(got.iter().any(|v| v == "inner"), "got {got:?}");
    assert!(got.iter().any(|v| v == "msg"), "got {got:?}");

    // tclsh 9.0.4 / 8.6.14:
    // `proc t {} {if {[catch {unset gone}]} {puts $gone}}`
    // `catch {t} err` → 1, err → `can't read "gone": no such variable`
    let got = cond_out_vars("catch {unset gone}");
    assert!(
        !got.iter().any(|v| v == "gone"),
        "`unset` destroys rather than defines; got {got:?}"
    );
}

/// A dynamic target contributes no *name* — `push_out_var` only accepts a
/// bare identifier, so `set $n 1` in a condition stays silent rather than
/// inventing a def called `n`.
#[test]
fn condition_out_vars_ignores_a_dynamic_target() {
    let got = cond_out_vars("set $n 1");
    assert_eq!(got.len(), 0, "got {got:?}");
}

/// A command word that is itself a substitution names
/// an unknown command, so nothing may be harvested from it — not even when
/// its *content* spelling collides with a builtin.
///
/// tclsh 9.0.4 / 8.6.14: `proc f {set} {if {[$set length foo]} {puts
/// $length}}; f string` runs `string length foo` and then errors
/// `can't read "length": no such variable`.
#[test]
fn condition_out_vars_ignores_a_substituted_command_head() {
    for cmd_text in ["$set length foo", "$catch {error x} msg", "[pick] $fp line"] {
        let got = cond_out_vars(cmd_text);
        assert_eq!(
            got.len(),
            0,
            "`{cmd_text}` has a computed head; nothing is knowable about \
what it writes, got {got:?}"
        );
    }
}

/// TP control for the pair above: the same words under a *literal* head
/// keep harvesting, so the guard keys on substitution rather than on the
/// spelling.
#[test]
fn condition_out_vars_still_harvests_a_literal_head() {
    assert!(
        cond_out_vars("set length foo")
            .iter()
            .any(|v| v == "length")
    );
    assert!(
        cond_out_vars("catch {error x} msg")
            .iter()
            .any(|v| v == "msg")
    );
    assert!(cond_out_vars("gets $fp line").iter().any(|v| v == "line"));
}

fn assert_absolute_method_ir(module: &crate::ir::Module) {
    assert!(
        matches!(
            module
                .methods
                .get(&fixture_id(&module.methods, "::C::exact"))
                .expect("authored fixture body")
                .body
                .statements
                .first(),
            Some(Statement::Return { expr: Some(_), .. })
        ),
        "absolute nested expr must retain typed return IR: {:#?}",
        module
            .methods
            .get(&fixture_id(&module.methods, "::C::exact"))
            .expect("authored fixture body")
            .body
    );
    assert!(
        matches!(
            module
                .methods
                .get(&fixture_id(&module.methods, "::C::exact_info"))
                .expect("authored fixture body")
                .body
                .statements
                .first(),
            Some(Statement::If { .. })
        ),
        "absolute if must retain typed control IR: {:#?}",
        module
            .methods
            .get(&fixture_id(&module.methods, "::C::exact_info"))
            .expect("authored fixture body")
            .body
    );
    assert!(
        matches!(
            module
                .methods
                .get(&fixture_id(&module.methods, "::C::exact_regex"))
                .expect("authored fixture body")
                .body
                .statements
                .first(),
            Some(Statement::AssignConst { .. })
        ),
        "absolute set must retain typed assignment IR: {:#?}",
        module
            .methods
            .get(&fixture_id(&module.methods, "::C::exact_regex"))
            .expect("authored fixture body")
            .body
    );
}

#[test]
fn runtime_selected_method_namespace_inventory_requires_absolute_heads() {
    let registry = tcl_registry::default_registry();
    let module = crate::lowering::lower_to_ir(
        "interp alias {} e {} expr\n\
             oo::class create C {\n\
                 method helper {} {::return 42}\n\
                 method m {} {::set unused [my helper]; ::puts done}\n\
                 method dynamic {cmd} {$cmd value}\n\
                 method global_body {} {::uplevel #0 {set x 1}}\n\
                 method exact {p} {::return [::expr {$p + 1}]}\n\
                 method exact_info {p} {::if {[::info exists p]} {::puts yes}}\n\
                 method exact_regex {s} {::set re {a+}; ::regexp $re $s}\n\
                 method nested_alias {p} {::set x [e {$p + 1}]}\n\
             }",
        registry,
    );
    assert!(!requires_runtime_command_namespace(
        &module
            .methods
            .get(&fixture_id(&module.methods, "::C::helper"))
            .expect("authored fixture body")
            .body,
        registry
    ));
    assert!(
        requires_runtime_command_namespace(
            &module
                .methods
                .get(&fixture_id(&module.methods, "::C::m"))
                .expect("authored fixture body")
                .body,
            registry
        ),
        "the receiver namespace can shadow TclOO's injected `my` command"
    );
    assert!(
        !requires_runtime_command_namespace(
            &module
                .methods
                .get(&fixture_id(&module.methods, "::C::dynamic"))
                .expect("authored fixture body")
                .body,
            registry
        ),
        "a computed head is already an opaque CFG barrier, not a lexical fallback dependency"
    );
    assert!(
        !requires_runtime_command_namespace(
            &module
                .methods
                .get(&fixture_id(&module.methods, "::C::global_body"))
                .expect("authored fixture body")
                .body,
            registry
        ),
        "uplevel #0 selects the exact global namespace for its nested body"
    );
    for name in ["::C::exact", "::C::exact_info", "::C::exact_regex"] {
        let body = &module
            .methods
            .get(&fixture_id(&module.methods, name))
            .expect("authored fixture body")
            .body;
        assert!(
            !requires_runtime_command_namespace(body, registry),
            "absolute-only method {name} must remain analysable: {body:#?}"
        );
    }
    assert!(
        requires_runtime_command_namespace(
            &module
                .methods
                .get(&fixture_id(&module.methods, "::C::nested_alias"))
                .expect("authored fixture body")
                .body,
            registry
        ),
        "a relative nested binding consumed by typed lowering can be shadowed by the object namespace"
    );
    assert_absolute_method_ir(&module);
}

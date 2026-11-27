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

#[test]
fn original_argv_disposition_requires_the_canonical_effect_carrier() {
    use super::*;
    for marker in [
        SyntheticMarker::UnseenCall,
        SyntheticMarker::RegistryBarrier,
        SyntheticMarker::UpvarInvalidate,
        SyntheticMarker::GlobalFrameScript,
        SyntheticMarker::CallerFrameOpaque,
    ] {
        let original = Statement::Barrier {
            span: Span::new(5, 10),
            reason: "widening".to_owned(),
            command: "display only".to_owned(),
            canonical_command: None,
            args: Vec::new(),
            tokens: Some(CommandTokens::marker(marker)),
        };
        assert_eq!(
            original.original_argv_inventory_disposition(),
            OriginalArgvInventoryDisposition::EffectOnly(marker)
        );
        let executable = original.is_executable_invocation();
        for axis in 0..3 {
            let mut altered = original.clone();
            let Statement::Barrier {
                args,
                canonical_command,
                tokens,
                ..
            } = &mut altered
            else {
                unreachable!()
            };
            match axis {
                0 => args.push("actual argument".to_owned()),
                1 => *canonical_command = Some("actual command".to_owned()),
                _ => tokens
                    .as_mut()
                    .unwrap()
                    .argv_texts
                    .push("actual argv".to_owned()),
            }
            assert_eq!(
                altered.original_argv_inventory_disposition(),
                OriginalArgvInventoryDisposition::SourceRequired
            );
            assert_eq!(altered.is_executable_invocation(), executable);
        }
    }
    let condition = Statement::Barrier {
        span: Span::new(0, 0),
        reason: "condition".to_owned(),
        command: "display".to_owned(),
        canonical_command: None,
        args: Vec::new(),
        tokens: Some(CommandTokens::marker(SyntheticMarker::Condition)),
    };
    assert_eq!(
        condition.original_argv_inventory_disposition(),
        OriginalArgvInventoryDisposition::SourceRequired
    );
}

use super::*;
use tcl_source::words::{Provenance, SourceSite, WordExpr, WordOpacity};

#[test]
fn original_logical_declaration_cards_cover_missing_namespace_without_publication() {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let source = "proc ::missing::p {argument} {set ::written VALUE}";
    let module = crate::lowering::lower_to_ir_with(
        crate::lowering::Lowerer::with_config(context.commands(), config)
            .with_resolved_analysis_input(input),
        source,
    );
    assert!(module.source_entry.native_entry.is_none());
    assert!(module.procedures.is_empty());
    let (id, card) = module
        .original_logical_declaration_body_units
        .iter()
        .next()
        .expect("actual readonly source card");
    let procedure = module.body_units.get(id).unwrap();
    assert_eq!(procedure.name, "::missing::p");
    assert_eq!(procedure.params, vec!["argument"]);
    assert!(procedure.publication.is_none());
    assert!(procedure.body.executed_source.is_none());
    assert!(procedure.body.native_compilation_failure.is_none());
    assert!(
        procedure
            .body
            .namespace_context
            .as_deref()
            .unwrap()
            .native_context()
            .is_none()
    );
    assert!(card.matches_procedure(&module, procedure));
    assert!(
        module
            .original_source_procedure_bodies()
            .any(|(body, _)| body == id)
    );
    let retained = module.retained_source_bindings.as_ref().unwrap();
    let bindings = retained
        .positioned_for_module(&module, context.commands())
        .unwrap();
    assert_eq!(
        bindings.installed_procedure_body_units(),
        [] as [crate::command_binding::SourceInstalledProcedureBody; 0]
    );
    let mut stale = module.clone();
    stale
        .body_units
        .get_mut(id)
        .unwrap()
        .params
        .push("changed".to_owned());
    assert!(
        !stale
            .original_source_procedure_bodies()
            .any(|(body, _)| body == id)
    );
    let mut missing = module.clone();
    missing.source_metadata_input = None;
    assert!(
        !missing
            .original_source_procedure_bodies()
            .any(|(body, _)| body == id)
    );
}

#[test]
fn statement_carriers_require_unanimous_ownership_and_keep_synthetic_phases_distinct() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let module = crate::lowering::lower_to_ir_with_dialect(
        "set x 1",
        registry,
        LexerConfig::from_grammar(profile.grammar),
        Some(profile),
    );
    let mut script = module.top_level;
    let original = script.statements[0].clone();
    assert!(original.tokens().is_none(), "typed store consumed its argv");
    let carrier = script
        .retained_source_tokens_for_statement(&script.statements[0])
        .expect("retained typed store carrier")
        .clone();
    assert!(
        script
            .retained_source_tokens_for_statement(&original)
            .is_none()
    );

    let mut function = crate::cfg::Function::new("::top", "entry");
    function.command_binding_sites = script.command_binding_sites.iter().cloned().collect();
    function
        .blocks
        .get_mut(&function.entry)
        .unwrap()
        .statements
        .push(original.clone());
    assert_eq!(
        function.source_input_tokens_at(function.entry, 0),
        Some(&carrier)
    );

    let mut conflict = function.command_binding_sites[0].clone();
    conflict.source_tokens.as_mut().unwrap().argv_texts[0].replace_range(.., "other");
    function.command_binding_sites.push(conflict.clone());
    assert!(function.source_input_tokens_at(function.entry, 0).is_none());
    let mut sites = script
        .command_binding_sites
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    sites.push(conflict);
    script.command_binding_sites = sites.into();
    assert!(
        script
            .retained_source_tokens_for_statement(&script.statements[0])
            .is_none()
    );

    let mut phase = carrier;
    phase.synthetic = Some(SyntheticMarker::EvaluatedArguments);
    let boundary = Statement::Barrier {
        span: original.span(),
        reason: "captured original argv".to_owned(),
        command: "set".to_owned(),
        canonical_command: None,
        args: vec!["x".to_owned(), "1".to_owned()],
        tokens: Some(phase.clone()),
    };
    function.blocks.get_mut(&function.entry).unwrap().statements[0] = boundary.clone();
    assert_eq!(
        function.source_input_tokens_at(function.entry, 0),
        Some(&phase)
    );
    script.statements[0] = boundary;
    assert!(
        script
            .retained_source_tokens_for_statement(&script.statements[0])
            .is_none()
    );
    assert!(function.source_input_tokens_at(function.entry, 1).is_none());
}

#[test]
fn braced_argument_query_rejects_command_head_and_out_of_range_indices() {
    let source = "{lappend} {name} {$value} $other";
    let config = LexerConfig::default();
    let commands = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
    let tokens = CommandTokens::from_segmented(&SourceMap::new(source), config, &commands[0]);
    assert!(tokens.arg_is_braced_literal(0));
    assert!(tokens.arg_is_braced_literal(1));
    assert!(!tokens.arg_is_braced_literal(2));
    assert!(!tokens.arg_is_braced_literal(3));
    assert!(!tokens.arg_is_braced_literal(usize::MAX));
}

#[test]
fn original_word_carrier_keeps_actual_quotes_escapes_and_expansion_presentations() {
    // naming.source.original-direct-token-owner
    // docs/design/analysis/name-resolution-proofs/original-direct-token-owner.md
    // This only checks a genuine written token carrier's compatibility
    // projections. It supplies no source provenance or evaluated argv.
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let config = LexerConfig::from_grammar(dialect.lexer_grammar);
        for source in [
            r#"list ONE "λ\u0041" {$value} pre[set v ONE]post"#,
            r#"list $value "$value" {x\y} \u0000"#,
        ] {
            let commands =
                tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
            assert_eq!(commands.len(), 1);
            let tokens =
                CommandTokens::from_segmented(&SourceMap::new(source), config, &commands[0]);
            assert!(
                tokens.original_words_match_argv_text(),
                "{version:?}: {source}"
            );
        }
        if dialect.lexer_grammar.expand_syntax {
            let source = "list {*}{ONE TWO}";
            let commands =
                tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
            let tokens =
                CommandTokens::from_segmented(&SourceMap::new(source), config, &commands[0]);
            assert!(matches!(tokens.words()[1], WordExpr::Expand { .. }));
            assert!(tokens.original_words_match_argv_text());
            assert_eq!(
                tokens.words().len(),
                2,
                "written words are not evaluated expansion slots"
            );
        }
    }
}

#[test]
fn original_word_carrier_refuses_same_count_text_and_word_changes() {
    // naming.source.original-direct-token-owner
    // docs/design/analysis/name-resolution-proofs/original-direct-token-owner.md
    let source = "list ONE";
    let config = LexerConfig::default();
    let commands = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
    let original = CommandTokens::from_segmented(&SourceMap::new(source), config, &commands[0]);
    assert!(original.original_words_match_argv_text());
    let mut changed = original.clone();
    changed.argv_texts[1] = "OTHER".to_owned();
    assert!(
        changed.words_align_with_argv_text(),
        "count-only compatibility remains distinct"
    );
    assert!(!changed.original_words_match_argv_text());
    let mut changed = original.clone();
    changed.argv_texts.pop();
    assert!(!changed.original_words_match_argv_text());
    let mut changed = original.clone();
    changed.word_exprs.pop();
    assert!(!changed.original_words_match_argv_text());
    let mut changed = original.clone();
    let source = changed.word_exprs[1].source().clone();
    changed.word_exprs[1] = WordExpr::Opaque {
        text: "OTHER".to_owned(),
        source,
        reason: WordOpacity::LossySnapshot,
    };
    assert!(changed.words_align_with_argv_text());
    assert!(!changed.original_words_match_argv_text());
    let mut consistent = original;
    consistent.synthetic = Some(SyntheticMarker::EvaluatedWrapper);
    assert!(
        consistent.original_words_match_argv_text(),
        "consistency alone grants no original purpose"
    );
}

#[test]
fn module_value_rules_retain_actual_axes_before_assistance_names() {
    let assistance = crate::environment_ingress::resolve_environment("tcl9.1").analyser_profile();
    let mut module = Module {
        dialect: Some("tcl8.4".into()),
        dialect_profile: Some(assistance),
        lexer_config: tcl_lexer::LexerConfig::for_profile(Some(assistance)),
        ..Module::default()
    };
    assert_eq!(
        module.number_syntax(),
        tcl_dialect::NumberSyntax::of_profile(Some(assistance))
    );
    let actual = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
    module.source_entry.invocation_dialect = Some(actual);
    assert_eq!(module.number_syntax(), actual.numbers);
    assert_eq!(module.word_values(), actual.word_values);
    let jim = tcl_registry::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
    );
    module.source_entry.invocation_dialect = Some(jim);
    assert_eq!(module.number_syntax(), jim.numbers);
    assert_eq!(module.word_values(), jim.word_values);
}

#[test]
fn quoted_variable_identity_uses_the_reference_site() {
    let source = "list \"${a}\" \"$a[upvar 0 y a]$a\" {$a}";
    let config = LexerConfig::default();
    let commands = tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
    let tokens = CommandTokens::from_segmented(&SourceMap::new(source), config, &commands[0]);
    let (spelling, read) = tokens.words()[1]
        .sole_variable_substitution()
        .expect("sole quoted read");
    assert_eq!(spelling, "${a}");
    assert_eq!(read, &SourceSite::source(Span::new(6, 9)));
    assert!(tokens.words()[2].sole_variable_substitution().is_none());
    assert!(tokens.words()[3].sole_variable_substitution().is_none());
}

#[test]
fn structured_variable_references_preserve_original_syntax() {
    let source = r#"list $a "${a}" "$a" $a(k) ${a(k)} "$a($i)" "$a$a" {$a}"#;
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let config =
            LexerConfig::for_profile(Some(registry.profile().expect("actual native grammar")));
        let commands =
            tcl_source::segment::segment_commands_with_offset_and_config(source, 0, config);
        let tokens = CommandTokens::from_segmented(&SourceMap::new(source), config, &commands[0]);
        for (index, expected) in [
            (1, "$a"),
            (2, "${a}"),
            (3, "$a"),
            (4, "$a(k)"),
            (5, "${a(k)}"),
            (6, "$a($i)"),
        ] {
            let (spelling, site) = tokens.words()[index]
                .sole_variable_substitution()
                .expect("one original reference");
            assert_eq!(spelling, expected, "{dialect}, word {index}");
            assert_eq!(site.provenance, Provenance::Source);
            let start = site.span.start() as usize;
            assert!(source[start..].starts_with(spelling));
        }
        // Two lexical reads cannot acquire one read identity; a braced
        // data word supplies none, despite containing the same name.
        assert!(tokens.words()[7].sole_variable_substitution().is_none());
        assert!(tokens.words()[8].sole_variable_substitution().is_none());
        assert_eq!(tokens.argv_texts[1], "${a}");
        assert_eq!(tokens.words()[1].legacy_text(), "${a}");
        assert_eq!(
            tokens.words()[1].sole_variable_substitution().unwrap().0,
            "$a"
        );
    }
}

#[test]
fn empty_script() {
    let script = Script::new();
    assert_eq!(script.statements, [] as [crate::ir::Statement; 0]);
}

#[test]
fn script_from_statements() {
    let stmts = vec![Statement::AssignConst {
        span: Span::new(0, 10),
        name: "x".into(),
        name_braced: false,
        value: "1".into(),
        value_span: None,
    }];
    let script = Script::from_statements(stmts);
    assert_eq!(script.statements.len(), 1);
}

#[test]
fn for_each_statement_visits_nested_bodies_in_order() {
    // `if {c} { inner1 } else { inner2 }` — the visitor must see the
    // `If` itself plus both nested-body calls, in source order.
    let inner1 = Statement::Call {
        span: Span::new(0, 0),
        command: "inner1".into(),
        canonical_command: None,
        args: Vec::new(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    let inner2 = Statement::Call {
        span: Span::new(0, 0),
        command: "inner2".into(),
        canonical_command: None,
        args: Vec::new(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    let if_stmt = Statement::If {
        span: Span::new(0, 0),
        clauses: vec![IfClause {
            condition: ExprNode::Raw { text: "c".into() },
            condition_span: Span::new(0, 0),
            body: Script::from_statements(vec![inner1]),
            body_span: Span::new(0, 0),
            condition_base: None,
        }],
        else_body: Some(Script::from_statements(vec![inner2])),
        else_span: None,
    };
    let script = Script::from_statements(vec![if_stmt]);

    let mut seen: Vec<String> = Vec::new();
    for_each_statement(&script, &mut |stmt| {
        let name = match stmt {
            Statement::If { .. } => "if".to_owned(),
            Statement::Call { command, .. } => command.clone(),
            _ => "other".to_owned(),
        };
        seen.push(name);
    });
    assert_eq!(seen, vec!["if", "inner1", "inner2"]);
}

#[test]
fn retained_region_phases_have_one_visitor_owner() {
    use crate::execution_region::{
        EvaluatedBodyRegion, ExecutionRegionDependency, RegionSelection,
    };
    let phase = |name: &str| {
        Script::from_statements(vec![Statement::AssignConst {
            span: Span::new(0, 0),
            name: name.into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        }])
    };
    let region = EvaluatedBodyRegion::captured_lifecycle(
        SourceSite::source(Span::new(0, 10)),
        phase("setup"),
        phase("body"),
        phase("cleanup"),
        RegionSelection::MaySkip,
        vec![ExecutionRegionDependency::Implementation("proved".into())],
    );
    let tokens = CommandTokens::from_lossy_parts(
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        None,
    )
    .with_evaluated_body(region);
    let original = Statement::Barrier {
        span: Span::new(0, 10),
        reason: "wrapper".into(),
        command: "wrapper".into(),
        canonical_command: None,
        args: Vec::new(),
        tokens: Some(tokens),
    };
    let mut boundary = original.clone();
    boundary.tokens_mut().unwrap().synthetic = Some(SyntheticMarker::EvaluatedWrapper);
    let mut script = Script::from_statements(vec![original, boundary]);
    let mut names = Vec::new();
    for_each_statement(&script, &mut |statement| {
        if let Statement::AssignConst { name, .. } = statement {
            names.push(name.clone());
        }
    });
    assert_eq!(names, ["setup", "body", "cleanup"]);
    let mut scripts = 0;
    for_each_script_mut(&mut script, &mut |_| scripts += 1);
    assert_eq!(scripts, 4);
}

/// `for_each_statement` recurses
/// once per nested `If`/`For`/`While`/`Foreach`/`Catch`/`Try`/`Switch`/
/// `UpFrame` body, so it needs a depth cap of its own.
/// Transitively bounded to `MAX_LOWER_NEST_DEPTH` (256) by the lowering
/// pass today (every `Script` this crate hands to `for_each_statement`
/// is built by `crate::lowering`), so this is defence-in-depth /
/// consistency with every other full-tree walker in this crate, not a
/// currently-reproducible crash. This test builds the nested `Script`
/// directly (bypassing lowering) so it genuinely exercises
/// `for_each_statement`'s own new cap rather than lowering's; 2000
/// levels is comfortably past the new 256-level cap. The assertion is
/// that the call returns at all, not what it returns.
#[test]
fn deeply_nested_if_survives_for_each_statement() {
    const DEPTH: usize = 2000;
    let leaf = Statement::Call {
        span: Span::new(0, 0),
        command: "leaf".into(),
        canonical_command: None,
        args: Vec::new(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    let mut script = Script::from_statements(vec![leaf]);
    for _ in 0..DEPTH {
        script = Script::from_statements(vec![Statement::If {
            span: Span::new(0, 0),
            clauses: vec![IfClause {
                condition: ExprNode::Raw { text: "1".into() },
                condition_span: Span::new(0, 0),
                body: script,
                body_span: Span::new(0, 0),
                condition_base: None,
            }],
            else_body: None,
            else_span: None,
        }]);
    }

    let mut count = 0usize;
    for_each_statement(&script, &mut |_| count += 1);
    assert!(count > 0, "must still visit statements within the cap");
    assert!(count <= DEPTH + 1, "must not exceed the number of nodes");
}

#[test]
fn for_each_statement_descends_into_upframe_body() {
    // FN guard: `UpFrame` (a static-body `uplevel
    // ?level? {...}`) has a nested `body: Script` just like `Block` /
    // `While` / `Catch` / `Foreach`, but was missing from the grouped
    // match arm — the visitor stopped at the `UpFrame` statement itself
    // and never walked its body, so a whole-module scan built on this
    // visitor (e.g. `var_observability::scan_module_global_names`)
    // silently missed anything declared inside a static `uplevel` body.
    let inner = Statement::Call {
        span: Span::new(0, 0),
        command: "inner".into(),
        canonical_command: None,
        args: Vec::new(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    let upframe = Statement::UpFrame {
        span: Span::new(0, 0),
        frame_shift: 0,
        absolute: true,
        body: Script::from_statements(vec![inner]),
        tokens: None,
    };
    let script = Script::from_statements(vec![upframe]);

    let mut seen: Vec<String> = Vec::new();
    for_each_statement(&script, &mut |stmt| {
        let name = match stmt {
            Statement::UpFrame { .. } => "upframe".to_owned(),
            Statement::Call { command, .. } => command.clone(),
            _ => "other".to_owned(),
        };
        seen.push(name);
    });
    assert_eq!(seen, vec!["upframe", "inner"]);
}

#[test]
fn statement_span_accessor() {
    let stmt = Statement::Call {
        span: Span::new(5, 20),
        command: "puts".into(),
        canonical_command: None,
        args: vec!["hello".into()],
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    assert_eq!(stmt.span(), Span::new(5, 20));
}

#[test]
fn assign_const_roundtrip() {
    let stmt = Statement::AssignConst {
        span: Span::new(0, 7),
        name: "x".into(),
        name_braced: false,
        value: "42".into(),
        value_span: None,
    };
    if let Statement::AssignConst { name, value, .. } = &stmt {
        assert_eq!(name, "x");
        assert_eq!(value, "42");
    } else {
        panic!("wrong variant");
    }
}

#[test]
fn barrier_with_tokens() {
    let tokens = CommandTokens::from_lossy_parts(
        vec![Span::new(0, 4), Span::new(5, 10)],
        vec!["eval".into(), "body".into()],
        vec![tcl_lexer::TokenType::Esc, tcl_lexer::TokenType::Str],
        vec![true, true],
        vec![Span::new(0, 4), Span::new(4, 5), Span::new(5, 10)],
        None,
    );
    let stmt = Statement::Barrier {
        span: Span::new(0, 10),
        reason: "eval".into(),
        command: "eval".into(),
        canonical_command: None,
        args: vec!["body".into()],
        tokens: Some(tokens.clone()),
    };
    if let Statement::Barrier {
        tokens: Some(t), ..
    } = &stmt
    {
        assert_eq!(t.argv_texts, vec!["eval", "body"]);
    } else {
        panic!("expected tokens");
    }
}

#[test]
fn if_statement_structure() {
    let stmt = Statement::If {
        span: Span::new(0, 50),
        clauses: vec![IfClause {
            condition: ExprNode::Literal {
                text: "1".into(),
                start: 3,
                end: 4,
            },
            condition_span: Span::new(3, 4),
            body: Script::from_statements(vec![Statement::Call {
                span: Span::new(6, 15),
                command: "puts".into(),
                canonical_command: None,
                args: vec!["yes".into()],
                defs: Vec::new(),
                reads: Vec::new(),
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
            }]),
            body_span: Span::new(5, 16),
            condition_base: None,
        }],
        else_body: None,
        else_span: None,
    };
    assert_eq!(stmt.span(), Span::new(0, 50));
    if let Statement::If { clauses, .. } = &stmt {
        assert_eq!(clauses.len(), 1);
    }
}

#[test]
fn for_loop_structure() {
    let stmt = Statement::For {
        span: Span::new(0, 40),
        init: Script::from_statements(vec![Statement::AssignConst {
            span: Span::new(5, 12),
            name: "i".into(),
            name_braced: false,
            value: "0".into(),
            value_span: None,
        }]),
        init_span: Span::new(4, 13),
        condition: ExprNode::Binary {
            op: crate::expr_ast::BinOp::Lt,
            left: Box::new(ExprNode::Var {
                text: "$i".into(),
                name: "i".into(),
                start: 0,
                end: 2,
            }),
            right: Box::new(ExprNode::Literal {
                text: "10".into(),
                start: 5,
                end: 7,
            }),
        },
        condition_span: Span::new(14, 22),
        next: Script::from_statements(vec![Statement::Incr {
            span: Span::new(24, 30),
            name: "i".into(),
            name_braced: false,
            amount: None,
            amount_braced: false,
            safe_on_uninit: false,
        }]),
        next_span: Span::new(23, 31),
        body: Script::new(),
        body_span: Span::new(32, 34),
        raw_args: Vec::new(),
        raw_tokens: None,
        condition_base: None,
    };
    assert_eq!(stmt.span(), Span::new(0, 40));
}

#[test]
fn switch_mode_parse() {
    assert_eq!(SwitchMode::from_str_lossy("exact"), SwitchMode::Exact);
    assert_eq!(SwitchMode::from_str_lossy("glob"), SwitchMode::Glob);
    assert_eq!(SwitchMode::from_str_lossy("regexp"), SwitchMode::Regexp);
    assert_eq!(SwitchMode::from_str_lossy("unknown"), SwitchMode::Exact);
}

#[test]
fn switch_mode_str() {
    assert_eq!(SwitchMode::Exact.as_str(), "exact");
    assert_eq!(SwitchMode::Glob.as_str(), "glob");
    assert_eq!(SwitchMode::Regexp.as_str(), "regexp");
}

#[test]
fn method_kind_from_effect() {
    use tcl_registry::definer::{CallableRole as R, MemberReceiver as S};
    let cases = [
        (R::Method, S::Instance, Some(MethodKind::Method)),
        (R::Method, S::TypeObject, Some(MethodKind::ClassMethod)),
        (R::Procedure, S::TypeObject, Some(MethodKind::ClassMethod)),
        (R::Procedure, S::Instance, Some(MethodKind::ClassMethod)),
        (R::Constructor, S::Instance, Some(MethodKind::Constructor)),
        (R::Destructor, S::Instance, Some(MethodKind::Destructor)),
        (R::Method, S::Both, None),
        (R::Constructor, S::TypeObject, None),
        (R::Destructor, S::TypeObject, None),
        (R::Accessor, S::Instance, None),
        (R::Mutator, S::Instance, None),
    ];
    for (role, receiver, kind) in cases {
        assert_eq!(
            MethodKind::from_effect(role, receiver),
            kind,
            "{role:?} on {receiver:?}"
        );
    }
}

#[test]
fn method_kind_spellings_are_the_analysers() {
    let spellings: Vec<&str> = [
        MethodKind::Method,
        MethodKind::ClassMethod,
        MethodKind::Constructor,
        MethodKind::Destructor,
    ]
    .into_iter()
    .map(MethodKind::as_str)
    .collect();
    assert_eq!(
        spellings,
        ["method", "classmethod", "constructor", "destructor"]
    );
}

#[test]
fn when_event_name_simple() {
    assert_eq!(when_event_name("::when::HTTP_REQUEST"), "HTTP_REQUEST");
}

#[test]
fn when_event_name_indexed() {
    assert_eq!(when_event_name("::when::HTTP_REQUEST#1"), "HTTP_REQUEST");
}

#[test]
fn when_event_name_no_prefix() {
    assert_eq!(when_event_name("HTTP_REQUEST"), "HTTP_REQUEST");
}

#[test]
fn procedure_construction() {
    let proc = Procedure {
        name: "greet".into(),
        qualified_name: "::greet".into(),
        publication: None,
        declaration_site: None,
        body_kind: ProcedureBodyKind::Named,
        params: vec!["name".into()],
        span: Span::new(0, 50),
        body: Script::from_statements(vec![Statement::Call {
            span: Span::new(20, 40),
            command: "puts".into(),
            canonical_command: None,
            args: vec!["Hello $name".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        }]),
        params_raw: "name".into(),
        body_source: Some("puts \"Hello $name\"".into()),
        body_offset: 0,
        namespace_scoped: false,
        base_priority: 500,
    };
    assert_eq!(proc.name, "greet");
    assert_eq!(proc.params, vec!["name"]);
}

#[test]
fn module_default() {
    let module = Module::default();
    assert_eq!(module.top_level.statements, [] as [crate::ir::Statement; 0]);
    assert_eq!(module.procedures.len(), 0);
    assert_eq!(module.methods.len(), 0);
    assert_eq!(module.redefined_procedures.len(), 0);
    assert_eq!(module.redefined_methods.len(), 0);
}

#[test]
fn foreach_with_iterators() {
    let stmt = Statement::Foreach {
        span: Span::new(0, 30),
        iterators: vec![ForeachIterator {
            vars: vec!["k".into(), "v".into()],
            list_arg: "$dict".into(),
            list_braced: false,
        }],
        body: Script::new(),
        body_span: Span::new(25, 28),
        is_lmap: false,
        raw_args: Vec::new(),
        is_dict_iteration: true,
        is_array_iteration: false,
        raw_tokens: None,
    };
    if let Statement::Foreach {
        iterators,
        is_dict_iteration,
        ..
    } = &stmt
    {
        assert_eq!(iterators.len(), 1);
        assert_eq!(iterators[0].vars, vec!["k", "v"]);
        assert!(is_dict_iteration);
    }
}

#[test]
fn catch_statement() {
    let stmt = Statement::Catch {
        span: Span::new(0, 30),
        body: Script::new(),
        body_span: Span::new(6, 12),
        result_var: Some("result".into()),
        options_var: Some("opts".into()),
        raw_args: Vec::new(),
        tokens: None,
    };
    if let Statement::Catch {
        result_var,
        options_var,
        ..
    } = &stmt
    {
        assert_eq!(result_var.as_deref(), Some("result"));
        assert_eq!(options_var.as_deref(), Some("opts"));
    }
}

#[test]
fn try_with_handlers() {
    let stmt = Statement::Try {
        span: Span::new(0, 60),
        body: Script::new(),
        body_span: Span::new(4, 10),
        handlers: vec![TryHandler {
            kind: HandlerMatch::CompletionCode,
            match_arg: "error".into(),
            trap_pattern: None,
            var_name: Some("e".into()),
            options_var: None,
            body: Script::new(),
            body_span: Span::new(30, 40),
            fallthrough: false,
        }],
        finally_body: Some(Script::new()),
        finally_span: Some(Span::new(50, 58)),
        raw_args: Vec::new(),
    };
    if let Statement::Try {
        handlers,
        finally_body,
        ..
    } = &stmt
    {
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0].kind, HandlerMatch::CompletionCode);
        assert!(finally_body.is_some());
    }
}

#[test]
fn switch_statement() {
    let stmt = Statement::Switch {
        subject_braced: false,
        raw_arg_braced: Vec::new(),
        raw_arg_quoted: Vec::new(),
        command: "switch".into(),
        span: Span::new(0, 80),
        subject: "$cmd".into(),
        subject_span: Span::new(7, 11),
        arms: vec![
            SwitchArm {
                pattern: "start".into(),
                pattern_braced: true,
                pattern_span: Span::new(13, 18),
                body: Some(Script::new()),
                body_span: Some(Span::new(19, 22)),
                fallthrough: false,
            },
            SwitchArm {
                pattern: "stop".into(),
                pattern_braced: true,
                pattern_span: Span::new(23, 27),
                body: None,
                body_span: None,
                fallthrough: true,
            },
        ],
        default_body: Some(Script::new()),
        default_span: Some(Span::new(50, 60)),
        mode: SwitchMode::Exact,
        nocase: false,
        raw_args: Vec::new(),
        patterns_braced: true,
    };
    if let Statement::Switch { arms, mode, .. } = &stmt {
        assert_eq!(arms.len(), 2);
        assert!(arms[1].fallthrough);
        assert_eq!(*mode, SwitchMode::Exact);
    }
}

#[test]
fn clone_preserves_equality() {
    let stmt = Statement::AssignConst {
        span: Span::new(0, 10),
        name: "x".into(),
        name_braced: false,
        value: "1".into(),
        value_span: None,
    };
    let cloned = stmt.clone();
    assert_eq!(stmt, cloned);
}

#[test]
fn return_with_expr() {
    let stmt = Statement::Return {
        expr_base: None,
        tokens: None,
        span: Span::new(0, 20),
        value: None,
        value_word: None,
        expr: Some(ExprNode::Binary {
            op: crate::expr_ast::BinOp::Add,
            left: Box::new(ExprNode::Var {
                text: "$a".into(),
                name: "a".into(),
                start: 0,
                end: 2,
            }),
            right: Box::new(ExprNode::Literal {
                text: "1".into(),
                start: 5,
                end: 6,
            }),
        }),
        command_binding: None,
        braced: false,
    };
    if let Statement::Return { expr: Some(e), .. } = &stmt {
        let vars = e.vars_with_grammar(tcl_dialect::LexerGrammar::default());
        assert!(vars.contains("a"));
    }
}

// canonical_command_or_source helper

#[test]
fn canonical_command_or_source_falls_back_to_source_when_none() {
    let stmt = Statement::Call {
        span: Span::new(0, 3),
        command: "foo".into(),
        canonical_command: None,
        args: Vec::new(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    assert_eq!(stmt.canonical_command_or_source(), "foo");
}

#[test]
fn canonical_command_or_source_returns_canonical_when_some() {
    let stmt = Statement::Call {
        span: Span::new(0, 3),
        command: "foo".into(),
        canonical_command: Some("::ns::bar".into()),
        args: Vec::new(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    assert_eq!(stmt.canonical_command_or_source(), "::ns::bar");
}

#[test]
fn canonical_command_or_source_works_on_barrier() {
    let stmt = Statement::Barrier {
        span: Span::new(0, 6),
        reason: "eval".into(),
        command: "eval".into(),
        canonical_command: Some("eval".into()),
        args: vec!["body".into()],
        tokens: None,
    };
    assert_eq!(stmt.canonical_command_or_source(), "eval");
}

#[test]
fn canonical_command_or_source_returns_empty_for_non_call_non_barrier() {
    let stmt = Statement::AssignConst {
        span: Span::new(0, 5),
        name: "x".into(),
        name_braced: false,
        value: "1".into(),
        value_span: None,
    };
    assert_eq!(stmt.canonical_command_or_source(), "");
}

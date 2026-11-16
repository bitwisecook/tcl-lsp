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

use super::*;
use crate::cfg_builder::{
    CfgCommandClasses, build_cfg_function as build_cfg_function_for_registry,
};
use crate::ir::{ForeachIterator, Script, SwitchArm, SwitchMode, TryHandler};
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;

fn build_cfg_function(name: &str, script: &Script, inline_loops: bool) -> crate::cfg::Function {
    build_cfg_function_for_registry(
        name,
        script,
        inline_loops,
        &CommandRegistry::build_default(),
        false,
    )
}

#[test]
fn original_try_handler_codes_keep_source_grammar_over_catalogue_availability() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Source-clause selection only; no native completion was executed.
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let registry = context.commands();
    let source = "try {} on 8 {} {} on 010 {} {}";
    for (numbers, expected, shadowed) in [
        (
            tcl_dialect::NumberSyntax::Tcl85,
            tcl_core_types::Code::Other(8),
            true,
        ),
        (
            tcl_dialect::NumberSyntax::Tcl90,
            tcl_core_types::Code::Other(10),
            false,
        ),
    ] {
        let mut profile = tcl_dialect::DialectProfile::plain_tcl().clone();
        profile.grammar.numbers = numbers;
        let profile = profile.intern();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .with_resolved_analysis_input(input.clone());
        let module = lowerer.lower(source).clone();
        let Statement::Try { handlers, .. } = &module.top_level.statements[0] else {
            panic!("genuine source try clauses: {:?}", module.top_level);
        };
        assert_eq!(handlers.len(), 2);
        let mut builder = CfgBuilder::new(false, registry).with_lexer_config(config);
        builder.command_classes = CfgCommandClasses::from_source_input(registry, Some(&input));
        assert_eq!(builder.handler_code(&handlers[1]), Some(expected));
        assert_eq!(builder.handler_chain(handlers).preempted(1), shadowed);
        builder.command_classes = CfgCommandClasses::from_source_input(registry, None);
        assert_eq!(builder.handler_code(&handlers[1]), None);
        assert!(!builder.handler_chain(handlers).preempted(1));
        assert_eq!(
            builder.handler_code(&handlers[0]),
            Some(tcl_core_types::Code::Other(8))
        );
        let foreign = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry(),
            config,
        );
        builder.command_classes = CfgCommandClasses::from_source_input(registry, Some(&foreign));
        assert_eq!(builder.handler_code(&handlers[1]), None);
        builder.command_classes = CfgCommandClasses::from_source_input(registry, Some(&input));
        let changed = tcl_lexer::LexerConfig {
            expand_syntax: !config.expand_syntax,
            ..config
        };
        builder = builder.with_lexer_config(changed);
        assert_eq!(builder.handler_code(&handlers[1]), None);
    }
}

#[test]
fn for_loop_creates_header_body_step() {
    let script = Script::from_statements(vec![Statement::For {
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
            op: BinOp::Lt,
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
        raw_args: vec![],
        raw_tokens: None,
        condition_base: None,
    }]);

    let func = build_cfg_function("::test", &script, true);
    // Should have blocks for: entry, for_header, for_body, for_step, for_end, exit
    assert!(func.blocks.len() >= 5, "got {} blocks", func.blocks.len());
    // A for_header block should have a Branch terminator.
    let header = func
        .blocks
        .values()
        .find(|b| b.name.starts_with("for_header"))
        .expect("should have for_header block");
    assert!(matches!(header.terminator, Some(Terminator::Branch { .. })));
}

#[test]
fn while_loop_creates_header_body() {
    let script = Script::from_statements(vec![Statement::While {
        span: Span::new(0, 20),
        condition: ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        },
        condition_span: Span::new(6, 7),
        body: Script::new(),
        body_span: Span::new(9, 11),
        raw_args: vec![],
        raw_tokens: None,
        condition_base: None,
    }]);

    let func = build_cfg_function("::test", &script, true);
    let header = func
        .blocks
        .values()
        .find(|b| b.name.starts_with("while_header"))
        .expect("should have while_header");
    assert!(matches!(header.terminator, Some(Terminator::Branch { .. })));
}

#[test]
fn foreach_creates_loop() {
    let script = Script::from_statements(vec![Statement::Foreach {
        span: Span::new(0, 30),
        iterators: vec![ForeachIterator {
            vars: vec!["x".into()],
            list_arg: "$list".into(),
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

    let func = build_cfg_function("::test", &script, true);
    let header = func
        .blocks
        .values()
        .find(|b| b.name.starts_with("foreach_header"))
        .expect("should have foreach_header");
    // Header should have a synthetic Call for iteration vars.
    assert!(header.statements.iter().any(|s| matches!(
        s,
        Statement::Call { command, .. } if command == "foreach"
    )));
}

#[test]
fn switch_exact_creates_branches() {
    let script = Script::from_statements(vec![Statement::Switch {
        subject_braced: false,
        raw_arg_braced: Vec::new(),
        raw_arg_quoted: Vec::new(),
        command: "switch".into(),
        span: Span::new(0, 50),
        subject: "$x".into(),
        subject_span: Span::new(7, 9),
        arms: vec![
            SwitchArm {
                pattern: "a".into(),
                pattern_braced: true,
                pattern_span: Span::new(11, 12),
                body: Some(Script::new()),
                body_span: Some(Span::new(13, 15)),
                fallthrough: false,
            },
            SwitchArm {
                pattern: "b".into(),
                pattern_braced: true,
                pattern_span: Span::new(16, 17),
                body: Some(Script::new()),
                body_span: Some(Span::new(18, 20)),
                fallthrough: false,
            },
        ],
        default_body: None,
        default_span: None,
        mode: SwitchMode::Exact,
        nocase: false,
        raw_args: vec![],
        patterns_braced: true,
    }]);

    let func = build_cfg_function("::test", &script, true);
    // Entry should have a Branch (first arm test).
    let entry = &func.blocks[&func.entry];
    assert!(matches!(entry.terminator, Some(Terminator::Branch { .. })));
}

/// Build a one-arm switch (body reads `$y`) in the given mode.
fn glob_regexp_switch(mode: SwitchMode) -> Script {
    Script::from_statements(vec![Statement::Switch {
        subject_braced: false,
        raw_arg_braced: Vec::new(),
        raw_arg_quoted: Vec::new(),
        command: "switch".into(),
        span: Span::new(0, 40),
        subject: "$x".into(),
        subject_span: Span::new(7, 9),
        arms: vec![SwitchArm {
            pattern: "a*".into(),
            pattern_braced: true,
            pattern_span: Span::new(11, 13),
            body: Some(Script::from_statements(vec![Statement::Call {
                span: Span::new(15, 22),
                command: "puts".into(),
                canonical_command: None,
                args: vec!["$y".into()],
                defs: vec![],
                reads: vec!["y".into()],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
            }])),
            body_span: Some(Span::new(14, 23)),
            fallthrough: false,
        }],
        default_body: None,
        default_span: None,
        mode,
        nocase: false,
        raw_args: vec!["$x".into(), "a*".into(), "{puts $y}".into()],
        patterns_braced: true,
    }])
}

#[test]
fn switch_glob_stays_opaque() {
    // Glob/regexp switches are kept opaque (a single `Statement::Switch` in
    // the block, no expanded arm blocks / branches). Codegen emits a generic
    // `switch` invoke for them.
    let func = build_cfg_function("::test", &glob_regexp_switch(SwitchMode::Glob), true);
    let entry = &func.blocks[&func.entry];
    assert_eq!(entry.statements.len(), 1, "opaque switch is one statement");
    assert!(
        matches!(entry.statements[0], Statement::Switch { .. }),
        "glob switch should stay an opaque Statement::Switch"
    );
    // The block falls through (no branch dispatch); `build_function` adds a
    // goto to the synthetic trailing exit block.
    assert!(
        matches!(entry.terminator, Some(Terminator::Goto { .. })),
        "opaque switch block falls through via a goto, not a branch dispatch"
    );
    // Only the entry + the synthetic trailing exit block exist (no arm
    // blocks — codegen emits a generic invoke from the statement).
    assert_eq!(func.blocks.len(), 2);
}

#[test]
fn switch_regexp_stays_opaque() {
    let func = build_cfg_function("::test", &glob_regexp_switch(SwitchMode::Regexp), true);
    let entry = &func.blocks[&func.entry];
    assert!(matches!(entry.statements[0], Statement::Switch { .. }));
    assert!(matches!(entry.terminator, Some(Terminator::Goto { .. })));
    // Exact switches without fall-through keep the real expanded jump table.
    let exact = build_cfg_function("::test", &glob_regexp_switch(SwitchMode::Exact), true);
    assert!(
        matches!(
            exact.blocks[&exact.entry].terminator,
            Some(Terminator::Branch { .. })
        ),
        "exact non-fall-through switch still expands to a branch dispatch"
    );
}

/// The operands of the first dispatch branch of the `switch` in `source`,
/// lowered and built for analysis under `dialect`'s own registry; `None`
/// where the statement stayed one opaque statement.
fn dispatch_operands(source: &str, dialect: &str) -> Option<(ExprNode, ExprNode)> {
    dispatch_operands_built(source, dialect, true)
}

/// [`dispatch_operands`], built for analysis (`faithful`) or for codegen.
fn dispatch_operands_built(
    source: &str,
    dialect: &str,
    faithful: bool,
) -> Option<(ExprNode, ExprNode)> {
    use tcl_registry::model::ingress::{resolve_environment, static_context_for};
    let registry = static_context_for(dialect).commands();
    let profile = resolve_environment(dialect).analyser_profile();
    // A fresh interpreter has no `x`; an unread `$x` subject would only
    // ever raise, so give it a value the source does not state.
    let source = format!("set x [gets stdin]\n{source}");
    let module = crate::lowering::lower_to_ir_with_dialect(
        &source,
        registry,
        LexerConfig::for_profile(registry.profile()),
        Some(profile),
    );
    let builder = crate::cfg_builder::CfgBuilder::new(false, registry)
        .with_lexer_config(LexerConfig::for_profile(registry.profile()));
    let mut builder = if faithful {
        builder.with_faithful_exceptions()
    } else {
        builder
    };
    let func = builder.build_function("::test", &module.top_level);
    match &func.blocks[&func.entry].terminator {
        Some(Terminator::Branch {
            condition:
                ExprNode::Binary {
                    op: BinOp::StrEq,
                    left,
                    right,
                },
            ..
        }) => Some(((**left).clone(), (**right).clone())),
        _ => None,
    }
}

/// The chain compares the values of a `switch`'s words, carried braced so
/// nothing reads them again: a bare or quoted word is its escapes decoded,
/// a braced one its content with the continuation collapsed, and an escaped
/// `$` is data. A word that substitutes stays its spelling, which no
/// evaluator folds, and a word whose value is its spelling keeps the
/// operand it always had.
#[test]
fn the_chain_compares_the_values_of_a_switch_words() {
    let value = |text: &str| ExprNode::CompiledWord {
        text: text.into(),
        braced: true,
    };
    for (source, subject, pattern) in [
        (r#"switch a\nb {"a\nb" {puts hit}}"#, "a\nb", "a\nb"),
        (r#"switch -- "a\tb" a\tb {puts hit}"#, "a\tb", "a\tb"),
        (r#"switch {a\b} {"a\\b" {puts hit}}"#, r"a\b", r"a\b"),
        ("switch {a\\\nb} {{a b} {puts hit}}", "a b", "a b"),
        (r"switch a\$b {a\$b {puts hit}}", "a$b", "a$b"),
    ] {
        let (left, right) = dispatch_operands(source, "tcl8.6").expect("a dispatch chain");
        assert_eq!((left, right), (value(subject), value(pattern)), "{source}");
    }
    let (left, right) =
        dispatch_operands("switch a${x} {abc {puts hit}}", "tcl8.6").expect("a dispatch chain");
    assert_eq!(
        left,
        ExprNode::CompiledWord {
            text: "a${x}".into(),
            braced: false
        }
    );
    assert_eq!(right, value("abc"));
    let (left, _) =
        dispatch_operands("switch abc {abc {puts hit}}", "tcl8.6").expect("a dispatch chain");
    assert_eq!(
        left,
        ExprNode::CompiledWord {
            text: "abc".into(),
            braced: false
        }
    );
}

/// Before 8.5 `switch` reads every leading word that starts with `-` as an
/// option, however many words follow, so a subject whose value may start
/// that way — a variable, or a literal whose escape decodes to `-` — is not
/// flattened under a release that may be before 8.5, unless `--` ended the
/// run; any other subject is, and so is every subject from 8.5. From 8.5
/// the scan stops with two words left, which a pattern and its body fill,
/// so with the arms as words the subject is inside it on every release. A
/// profile that names no release is read as one that may be 8.4.
#[test]
fn a_subject_a_release_may_read_as_an_option_stays_one_statement() {
    for (dialect, chained) in [
        ("tcl8.4", false),
        ("f5-irules", false),
        ("tk", false),
        ("tcl8.5", true),
        ("tcl8.6", true),
        ("tcl9.0", true),
    ] {
        for source in [
            "switch $x {a {puts A}}",
            "switch -exact $x {a {puts A}}",
            r"switch \x2dglob {a {puts A}}",
        ] {
            assert_eq!(
                dispatch_operands(source, dialect).is_some(),
                chained,
                "{dialect}: {source}"
            );
        }
    }
    for dialect in ["tcl8.4", "f5-irules", "tk", "tcl8.5", "tcl8.6", "tcl9.0"] {
        for source in [
            "switch $x a {puts A} b {puts B}",
            "switch -exact $x a {puts A}",
            r"switch \x2dglob a {puts A} b {puts B}",
        ] {
            assert!(
                dispatch_operands(source, dialect).is_none(),
                "{dialect}: {source}"
            );
        }
        for source in [
            "switch -- $x a {puts A} b {puts B}",
            "switch abc a {puts A} b {puts B}",
            "switch [gets stdin] a {puts A} b {puts B}",
        ] {
            assert!(
                dispatch_operands(source, dialect).is_some(),
                "{dialect}: {source}"
            );
        }
    }
    for dialect in ["tcl8.4", "f5-irules", "tk", "tcl8.6"] {
        for source in [
            "switch -- $x {a {puts A}}",
            "switch -exact -- $x {a {puts A}}",
            "switch abc {a {puts A}}",
            "switch [gets stdin] {a {puts A}}",
            "switch {$x} {a {puts A}}",
        ] {
            assert!(
                dispatch_operands(source, dialect).is_some(),
                "{dialect}: {source}"
            );
        }
    }
}

/// Codegen flattens a `switch` only where the release's compile procedure
/// compiles it: never in 8.4, and from 8.5 the two-word form or options
/// that `--` closes.
#[test]
fn codegen_flattens_the_switch_the_release_compiles() {
    for (dialect, source, flattened) in [
        ("tcl8.4", "switch $x {a {puts A}}", false),
        ("tcl8.4", "switch -- $x a {puts A}", false),
        ("tcl8.5", "switch $x {a {puts A}}", true),
        ("tcl8.6", "switch -exact $x {a {puts A}}", false),
        ("tcl8.6", "switch -exact -- $x {a {puts A}}", true),
        ("tcl8.6", "switch $x a {puts A}", false),
        ("tcl9.0", "switch -- $x a {puts A} b {puts B}", true),
    ] {
        assert_eq!(
            dispatch_operands_built(source, dialect, false).is_some(),
            flattened,
            "{dialect}: {source}"
        );
    }
}

/// A registry with no profile declares no target at all, so the rule reads
/// no release there and the statement is flattened.
#[test]
fn a_registry_with_no_profile_reads_no_release_for_the_option_scan() {
    let registry = CommandRegistry::build_default();
    let module =
        crate::lowering::lower_to_ir("set x [gets stdin]\nswitch $x {a {puts A}}", &registry);
    let func =
        build_cfg_function_for_registry("::test", &module.top_level, false, &registry, false);
    assert!(matches!(
        func.blocks[&func.entry].terminator,
        Some(Terminator::Branch { .. })
    ));
}

#[test]
fn try_finally_creates_finally_block() {
    let script = Script::from_statements(vec![Statement::Try {
        span: Span::new(0, 40),
        body: Script::new(),
        body_span: Span::new(4, 6),
        handlers: vec![],
        finally_body: Some(Script::from_statements(vec![Statement::Call {
            span: Span::new(20, 35),
            command: "cleanup".into(),
            canonical_command: None,
            args: vec![],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        }])),
        finally_span: Some(Span::new(18, 36)),
        raw_args: vec![],
    }]);

    let func = build_cfg_function("::test", &script, true);
    // Should have a try_finally block.
    assert!(
        func.blocks
            .values()
            .any(|b| b.name.starts_with("try_finally")),
        "should have try_finally block"
    );
}

#[test]
fn try_with_handler() {
    let script = Script::from_statements(vec![Statement::Try {
        span: Span::new(0, 60),
        body: Script::new(),
        body_span: Span::new(4, 6),
        handlers: vec![TryHandler {
            kind: crate::ir::HandlerMatch::CompletionCode,
            match_arg: "error".into(),
            trap_pattern: None,
            var_name: Some("e".into()),
            options_var: None,
            body: Script::new(),
            body_span: Span::new(30, 35),
            fallthrough: false,
        }],
        finally_body: None,
        finally_span: None,
        raw_args: vec![],
    }]);

    let func = build_cfg_function("::test", &script, true);
    assert!(
        func.blocks
            .values()
            .any(|b| b.name.starts_with("try_handler")),
        "should have try_handler block"
    );
}

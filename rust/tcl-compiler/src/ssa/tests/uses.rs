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

//! `uses_of` and the classification of each read as substituted or quoted, including opaque `switch` arms.

use super::*;

#[test]
fn uses_of_assign_const() {
    let reg = default_registry();
    let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
    let stmt = Statement::AssignConst {
        span: Span::new(0, 10),
        name: "x".into(),
        name_braced: false,
        value: "1".into(),
        value_span: None,
    };
    let uses = uses_of(&stmt, &mut scanner, &reg);
    assert_eq!(uses.len(), 0, "constant assignment reads nothing");
}

#[test]
fn uses_of_assign_value_with_var() {
    let reg = default_registry();
    let mut scanner = VarReferenceScanner::new(VarScanOptions {
        include_var_read_roles: true,
        recurse_cmd_substitutions: true,
        include_reads_before_write: false,
        element_qualified: false,
    });
    let stmt = Statement::AssignValue {
        span: Span::new(0, 15),
        name: "y".into(),
        name_braced: false,
        value: "$x".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    let uses = uses_of(&stmt, &mut scanner, &reg);
    assert!(uses.contains(&"x".to_string()), "should read $x");
    assert!(
        !uses.contains(&"y".to_string()),
        "should not read $y (it's defined)"
    );
}

#[test]
fn uses_of_incr() {
    let reg = default_registry();
    let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
    let stmt = Statement::Incr {
        span: Span::new(0, 10),
        name: "i".into(),
        name_braced: false,
        amount: None,
        amount_braced: false,
        safe_on_uninit: false,
    };
    let uses = uses_of(&stmt, &mut scanner, &reg);
    // incr reads and writes — reads_own_def
    assert!(uses.contains(&"i".to_string()), "incr reads the variable");
}

#[test]
fn uses_of_return_with_value() {
    let reg = default_registry();
    let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
    let stmt = Statement::Return {
        tokens: None,
        span: Span::new(0, 15),
        value: Some("$result".into()),
        value_word: None,
        expr: None,
        expr_base: None,
        command_binding: None,
        braced: false,
    };
    let uses = uses_of(&stmt, &mut scanner, &reg);
    assert!(uses.contains(&"result".to_string()));
}

#[test]
fn uses_of_expr_eval() {
    let reg = default_registry();
    let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
    let stmt = Statement::ExprEval {
        span: Span::new(0, 20),
        command_binding: tcl_runtime_api::CommandBindingIdentity::new("expr", "expr"),
        expr: ExprNode::Binary {
            op: crate::expr_ast::BinOp::Add,
            left: Box::new(ExprNode::Var {
                text: "$a".into(),
                name: "a".into(),
                start: 0,
                end: 2,
            }),
            right: Box::new(ExprNode::Var {
                text: "$b".into(),
                name: "b".into(),
                start: 5,
                end: 7,
            }),
        },
        expr_base: None,
    };
    let uses = uses_of(&stmt, &mut scanner, &reg);
    assert!(uses.contains(&"a".to_string()));
    assert!(uses.contains(&"b".to_string()));
}

// UseClass classification

/// A `Call` whose argument words are `args`, with per-word token kinds
/// derived from the word text: `{…}` lexes to `Str` (brace-quoted, the one
/// form Tcl leaves wholly unsubstituted), everything else to `Esc`.
fn call_with_words(command: &str, args: &[&str]) -> Statement {
    let source = std::iter::once(command)
        .chain(args.iter().copied())
        .collect::<Vec<_>>()
        .join(" ");
    let segment = tcl_source::segment::segment_commands(&source).remove(0);
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(&source),
        tcl_lexer::LexerConfig::default(),
        &segment,
    );
    Statement::Call {
        span: segment.span,
        command: command.to_owned(),
        canonical_command: None,
        args: segment.texts.into_iter().skip(1).collect(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: Some(tokens),
        foreach_groups: None,
    }
}

fn classify(stmt: &Statement, reg: &CommandRegistry, name: &str) -> Option<UseClass> {
    let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
    uses_of_classified(stmt, &mut scanner, reg)
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, class)| class)
}

/// A braced word at a role the callee does **not** evaluate in this frame
/// is a `Quoted` use: still recorded (liveness must assume it may be
/// evaluated later) but not a read here.
/// tclsh-proof: tclsh8.6.14 — `puts {$y}` prints `$y` with `y` undefined.
#[test]
fn uses_of_classified_braced_data_word_is_quoted() {
    let reg = default_registry()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let stmt = call_with_words("puts", &["{$y}"]);
    assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
}

/// The same name in an unbraced word is a definite read.
#[test]
fn uses_of_classified_unbraced_word_is_substituted() {
    let reg = default_registry();
    let stmt = call_with_words("puts", &["$y"]);
    assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Substituted));
}

/// A braced `Expr`-role word really does substitute — `expr` evaluates it
/// against the caller's variables — so it stays `Substituted`.
#[test]
fn uses_of_classified_braced_expr_word_is_substituted() {
    let reg = default_registry();
    let stmt = call_with_words("expr", &["{$a + $b}"]);
    assert_eq!(classify(&stmt, &reg, "a"), Some(UseClass::Substituted));
    assert_eq!(classify(&stmt, &reg, "b"), Some(UseClass::Substituted));
}

/// A command the registry does not describe carries no role information,
/// so its braced word is *unclassified*: it may be a script that runs in
/// this frame. A name the word sets itself is that script's own local —
/// the un-hooked definer shape — so it is `Quoted`.
#[test]
fn uses_of_classified_unknown_definer_body_local_is_quoted() {
    let reg = default_registry();
    let stmt = call_with_words(
        "mydefiner",
        &["::foo::bar", "{optlist}", "{set y 1; return $y}"],
    );
    assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
}

/// TN control — a name the unclassified word does **not** set stays a
/// read: a wrapper handing the word to an `uplevel`-ing worker really does
/// evaluate it in this frame, and tclsh errors on the unset name.
#[test]
fn uses_of_classified_unknown_command_free_read_stays_substituted() {
    let reg = default_registry();
    let stmt = call_with_words("wrapper", &["myf", "{puts $myf}"]);
    assert_eq!(classify(&stmt, &reg, "myf"), Some(UseClass::Substituted));
}

#[test]
fn source_ownership_classification_uses_the_retained_lexical_config() {
    // naming.variable.ssa-retained-lexical-config
    // docs/design/analysis/name-resolution-proofs/variable-ssa-retained-lexical-config.md
    let registry = default_registry();
    let tcl = tcl_lexer::LexerConfig::for_dialect("tcl9.0");
    let jim = tcl_lexer::LexerConfig {
        base_offset: 87,
        base_line: 4,
        base_col: 12,
        leading_bom: tcl_lexer::LeadingBom::Skip,
        ..tcl_lexer::LexerConfig::for_dialect("jim")
    };
    // These unchanged, brace-balanced source words exercise command
    // separators and list parsing. They are lexical classification controls,
    // not a proof that the unknown wrapper executes either body.
    for (body, tcl_class, jim_class) in [
        (
            "{set\u{000b}x 1; puts $x}",
            UseClass::Quoted,
            UseClass::Substituted,
        ),
        (
            "{foreach \"{x}y\" {1 2} {}; puts $x}",
            UseClass::Substituted,
            UseClass::Quoted,
        ),
    ] {
        let statement = call_with_words("wrapper", &[body]);
        for (config, expected) in [(tcl, tcl_class), (jim, jim_class)] {
            let mut scanner = VarReferenceScanner::with_config(VarScanOptions::default(), config);
            let class = uses_of_classified(&statement, &mut scanner, &registry)
                .into_iter()
                .find(|(name, _)| name == "x")
                .map(|(_, class)| class);
            assert_eq!(class, Some(expected), "{body:?}, {config:?}");
            assert_eq!(scanner.lexer_config(), config.nested().normalized());
        }
    }
}

#[test]
fn nested_source_expression_roles_use_the_retained_lexical_config() {
    // naming.variable.ssa-retained-lexical-config
    // docs/design/analysis/name-resolution-proofs/variable-ssa-retained-lexical-config.md
    let registry = default_registry();
    let statement = call_with_words("puts", &["[expr\u{000b} {$nested}]"]);
    for (config, expected) in [
        (tcl_lexer::LexerConfig::for_dialect("tcl9.0"), true),
        (tcl_lexer::LexerConfig::for_dialect("jim"), false),
    ] {
        // Jim's actual head is `expr` followed by a vertical-tab byte.
        // Registry source roles cannot replace that head with `expr`.
        let mut scanner = VarReferenceScanner::with_config(VarScanOptions::default(), config);
        let names = uses_of_classified(&statement, &mut scanner, &registry);
        assert_eq!(
            names
                .iter()
                .any(|(name, class)| name == "nested" && *class == UseClass::Substituted),
            expected,
            "{config:?}: {names:?}",
        );
    }
}

/// A name reached both ways in one statement is a definite read.
#[test]
fn uses_of_classified_substituted_wins_over_quoted() {
    let reg = default_registry();
    let stmt = call_with_words("puts", &["{$y}", "$y"]);
    assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Substituted));
}

/// A braced `return` value is literal.
/// tclsh-proof: tclsh8.6.14 — `proc f {} { return {$y} }; puts [f]` prints
/// `$y` with `y` undefined.
#[test]
fn uses_of_classified_braced_return_value_is_quoted() {
    let reg = default_registry();
    let stmt = Statement::Return {
        tokens: None,
        span: Span::new(0, 15),
        value: Some("$y".into()),
        value_word: None,
        expr: None,
        expr_base: None,
        command_binding: None,
        braced: true,
    };
    assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
}

/// Build a one-arm opaque `switch` around `body`, with `pattern` as the
/// arm's pattern and the canonical braced arm block.
fn opaque_switch(subject: &str, pattern: &str, body: crate::ir::Script) -> Statement {
    Statement::Switch {
        subject_braced: false,
        raw_arg_braced: Vec::new(),
        raw_arg_quoted: Vec::new(),
        command: "switch".into(),
        span: Span::new(0, 40),
        subject: subject.into(),
        subject_span: Span::new(0, 1),
        arms: vec![crate::ir::SwitchArm {
            pattern: pattern.into(),
            pattern_braced: true,
            pattern_span: Span::new(0, 1),
            body: Some(body),
            body_span: Some(Span::new(0, 1)),
            fallthrough: false,
        }],
        default_body: None,
        default_span: None,
        mode: crate::ir::SwitchMode::Glob,
        nocase: false,
        raw_args: Vec::new(),
        patterns_braced: true,
    }
}

/// An arm body is the one script that reaches SSA
/// un-lowered, and its reads must arrive classified exactly as the same
/// word would be outside the arm. A braced data word is `Quoted`.
/// tclsh-proof: tclsh 9.0.4 — `proc f {z} { switch -glob $z { a* { puts
/// {$b} } } }; f abc` prints `$b` with `b` undefined.
#[test]
fn uses_of_classified_opaque_switch_arm_braced_data_word_is_quoted() {
    let reg = default_registry()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let mut body = crate::ir::Script::new();
    body.statements.push(call_with_words("puts", &["{$y}"]));
    let stmt = opaque_switch("$z", "a*", body);
    assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
    // The subject is a genuine read of this frame.
    assert_eq!(classify(&stmt, &reg, "z"), Some(UseClass::Substituted));
}

/// TP control — the substituting spelling of the same arm word stays a
/// definite read, so the classification is threaded, not dropped.
#[test]
fn uses_of_classified_opaque_switch_arm_unbraced_word_is_substituted() {
    let reg = default_registry();
    let mut body = crate::ir::Script::new();
    body.statements.push(call_with_words("puts", &["$y"]));
    let stmt = opaque_switch("$z", "a*", body);
    assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Substituted));
}

/// A `Statement::Foreach` inside an opaque arm is walked by
/// `reads_in_stmt` rather than lowered, so `ForeachIterator::list_braced`
/// is the extra fact that walk needs: a braced value word is
/// literal list text, recorded `Quoted` so liveness still honours it.
#[test]
fn uses_of_classified_opaque_switch_arm_braced_foreach_list_is_quoted() {
    let reg = default_registry();
    let mut body = crate::ir::Script::new();
    body.statements.push(Statement::Foreach {
        span: Span::new(0, 20),
        iterators: vec![crate::ir::ForeachIterator {
            vars: vec!["n".into()],
            list_arg: "a $y c".into(),
            list_braced: true,
        }],
        body: crate::ir::Script::new(),
        body_span: Span::new(0, 1),
        is_lmap: false,
        raw_args: Vec::new(),
        is_dict_iteration: false,
        is_array_iteration: false,
        raw_tokens: None,
    });
    let stmt = opaque_switch("$z", "a*", body);
    assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Quoted));
}

/// TP control for the loop value word — the substituting spelling is a
/// definite read even inside an opaque arm.
#[test]
fn uses_of_classified_opaque_switch_arm_substituted_foreach_list_is_substituted() {
    let reg = default_registry();
    let mut body = crate::ir::Script::new();
    body.statements.push(Statement::Foreach {
        span: Span::new(0, 20),
        iterators: vec![crate::ir::ForeachIterator {
            vars: vec!["n".into()],
            list_arg: "a $y c".into(),
            list_braced: false,
        }],
        body: crate::ir::Script::new(),
        body_span: Span::new(0, 1),
        is_lmap: false,
        raw_args: Vec::new(),
        is_dict_iteration: false,
        is_array_iteration: false,
        raw_tokens: None,
    });
    let stmt = opaque_switch("$z", "a*", body);
    assert_eq!(classify(&stmt, &reg, "y"), Some(UseClass::Substituted));
}

/// A pattern from the canonical braced arm block is a
/// literal list element; supplied as separate words it substitutes.
#[test]
fn uses_of_classified_switch_pattern_class_follows_patterns_braced() {
    let reg = default_registry();
    let braced = opaque_switch("$z", "$p*", crate::ir::Script::new());
    assert_eq!(classify(&braced, &reg, "p"), Some(UseClass::Quoted));
    let mut worded = opaque_switch("$z", "$p*", crate::ir::Script::new());
    if let Statement::Switch {
        patterns_braced, ..
    } = &mut worded
    {
        *patterns_braced = false;
    }
    assert_eq!(classify(&worded, &reg, "p"), Some(UseClass::Substituted));
}

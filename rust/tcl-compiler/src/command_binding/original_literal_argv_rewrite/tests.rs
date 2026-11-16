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

fn receipt(source: &str, profile: &str) -> Option<OriginalLiteralExpressionBracing> {
    let analysis = crate::analyser::Analyser::new().analyse(source, profile);
    let config = analysis.body_lexer_config?;
    let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let command = commands.last()?;
    let mut tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, command);
    analysis
        .retained_command_realm()?
        .stamp_original_tokens(&mut tokens);
    tokens
        .source_binding
        .as_ref()?
        .original_literal_expression_bracing(&tokens, analysis.resolved_registry()?)
}

fn gap_receipt(source: &str, candidate: &str) -> Option<OriginalExpressionGapCompaction> {
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let config = analysis.body_lexer_config?;
    let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    crate::registry_invocation::source_structure::original_expression_gap_compaction(
        source,
        &analysis,
        commands.last()?,
        candidate,
    )
}

fn operator_receipt(
    source: &str,
    profile: &str,
    candidate: &str,
    kind: crate::registry_invocation::source_structure::OriginalExpressionOperatorRewriteKind,
) -> Option<crate::registry_invocation::source_structure::OriginalExpressionOperatorRewrite> {
    let analysis = crate::analyser::Analyser::new().analyse(source, profile);
    let config = analysis.body_lexer_config?;
    let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    crate::registry_invocation::source_structure::original_expression_operator_rewrite(
        source,
        &analysis,
        commands.last()?,
        0,
        candidate,
        kind,
    )
}

#[test]
fn original_editor_operator_rewrite_keeps_reflection_free_bodies_and_leaf_order() {
    // naming.core.original-expression-source-selection
    // docs/design/analysis/name-resolution-proofs/original-expression-source-selection.md
    use crate::registry_invocation::source_structure::OriginalExpressionOperatorRewriteKind::{
        DeMorgan, InvertComparison,
    };
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for (source, candidate, kind) in [
            ("expr {!(1 && 0)}", "!1 || !0", DeMorgan),
            ("expr {!1 || !0}", "!(1 && 0)", DeMorgan),
            ("expr {!(0 || 1)}", "!0 && !1", DeMorgan),
            ("expr {!0 && !1}", "!(0 || 1)", DeMorgan),
            ("if {!(1 && 0)} {}", "!1 || !0", DeMorgan),
            ("if {!1 || !0} {} else {}", "!(1 && 0)", DeMorgan),
            ("if {1 == 0} {}", "1 != 0", InvertComparison),
            ("expr {1 != 0}", "1 == 0", InvertComparison),
            ("expr {1 eq 0}", "1 ne 0", InvertComparison),
            ("expr {!($a && $b)}", "!$a || !$b", DeMorgan),
            ("if {$a == $b} {puts yes}", "$a != $b", InvertComparison),
            ("if {!(1 && 0)} {puts yes}", "!1 || !0", DeMorgan),
        ] {
            let proof = operator_receipt(source, profile, candidate, kind)
                .unwrap_or_else(|| panic!("{profile}: {source} -> {candidate}"));
            assert_eq!(proof.replacement_source_word(), format!("{{{candidate}}}"));
            assert_eq!(
                proof.original_operand().image(),
                &SourceImage::document(source)
            );
        }
    }
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        assert!(
            operator_receipt("expr {1 in {1 2}}", profile, "1 ni {1 2}", InvertComparison)
                .is_some(),
            "{profile}"
        );
    }
    assert!(
        operator_receipt(
            "expr {1 in {1 2}}",
            "tcl8.4",
            "1 ni {1 2}",
            InvertComparison
        )
        .is_none()
    );
}

#[test]
fn original_editor_operator_rewrite_refuses_reflection_effects_and_reordered_leaves() {
    // naming.core.original-expression-source-selection
    // docs/design/analysis/name-resolution-proofs/original-expression-source-selection.md
    use crate::registry_invocation::source_structure::OriginalExpressionOperatorRewriteKind::{
        DeMorgan, InvertComparison,
    };
    for (source, candidate, kind) in [
        ("expr {!(1 && 0)}", "!0 || !1", DeMorgan),
        ("expr {!(1 && 0)}", "!1 && !0", DeMorgan),
        ("expr {!(1 && 0)}", "!1 || !2", DeMorgan),
        ("expr {!($a([child]) && 1)}", "!$a([child]) || !1", DeMorgan),
        ("expr {!([child] && 1)}", "![child] || !1", DeMorgan),
        ("expr {!(rand() && 1)}", "!rand() || !1", DeMorgan),
        (r#"expr {!("$a" && 1)}"#, r#"!"$a" || !1"#, DeMorgan),
        ("expr {!(invalid && 1)}", "!invalid || !1", DeMorgan),
        ("expr {!(1 / 0 && 1)}", "!(1 / 0) || !1", DeMorgan),
        ("proc expr args {}; expr {!(1 && 0)}", "!1 || !0", DeMorgan),
        ("unknown_mutation; expr {!(1 && 0)}", "!1 || !0", DeMorgan),
        (
            "trace add execution expr enter callback; expr {!(1 && 0)}",
            "!1 || !0",
            DeMorgan,
        ),
        (
            "trace add variable a read callback; expr {!(1 && 0)}",
            "!1 || !0",
            DeMorgan,
        ),
        ("proc p {} {expr {!(1 && 0)}}", "!1 || !0", DeMorgan),
        ("expr {!(1 && 0)}; info frame 0", "!1 || !0", DeMorgan),
        ("if {!(1 && 0)} {info frame -1}", "!1 || !0", DeMorgan),
        ("if {!(1 && 0)} {$body}", "!1 || !0", DeMorgan),
        ("if {!(1 && 0)}", "!1 || !0", DeMorgan),
        ("if {!(1 && 0)} {} else", "!1 || !0", DeMorgan),
        ("if {!(1 && 0)} {} {} {}", "!1 || !0", DeMorgan),
        (
            "if {1 == 1} {} elseif {[incr x]} {}",
            "1 != 1",
            InvertComparison,
        ),
        (
            "if {1 == 1} {} elseif {[error LATER]} {}",
            "1 != 1",
            InvertComparison,
        ),
        (
            "if {!(1 && 0)} {} elseif {[info frame -1]} {}",
            "!1 || !0",
            DeMorgan,
        ),
        ("if {1 < 0} {}", "1 >= 0", InvertComparison),
        ("if {$a < $b} {}", "$a >= $b", InvertComparison),
        (
            "if {$a == $b} {puts [info frame 0]}",
            "$a != $b",
            InvertComparison,
        ),
        ("if {$a == $b} {proc_call}", "$a != $b", InvertComparison),
        ("if {1 == 0} {}", "0 != 1", InvertComparison),
        ("if {1 == 0} {}", "1 != 2", InvertComparison),
    ] {
        assert!(
            operator_receipt(source, "tcl8.6", candidate, kind).is_none(),
            "{source} -> {candidate}"
        );
    }
}

#[test]
fn original_boolean_inversion_retains_leaf_and_dispatch_owners() {
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    let receipt = |source: &str, candidate: &str| {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let config = analysis.body_lexer_config?;
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        crate::registry_invocation::source_structure::original_expression_boolean_comparison_inversion(
                source, &analysis, commands.last()?, 0, candidate,
            )
    };
    for (source, candidate) in [
        ("if {!($a eq \"x\")} {}", "$a ne\"x\""),
        ("if {!($a ne \"x\")} {}", "$a eq\"x\""),
        ("if {!($a == $b)} {}", "$a!=$b"),
        ("if {!($a != $b)} {}", "$a==$b"),
        ("set a 1; set b 2; expr {!($a == $b)}", "$a!=$b"),
        ("set a 1; set b 2; expr {!($a != $b)}", "$a==$b"),
        ("if {!($a in $b)} {}", "$a ni$b"),
        ("if {!($a ni $b)} {}", "$a in$b"),
    ] {
        let proof = receipt(source, candidate).expect("actual boolean inversion owner");
        assert_eq!(proof.replacement_source_word(), format!("{{{candidate}}}"));
        assert!(proof.matches_source(
            &SourceImage::document(source),
            proof.original_operand().config()
        ));
        assert!(!proof.matches_source(
            &SourceImage::document("different"),
            proof.original_operand().config()
        ));
    }
    for (source, candidate) in [
        ("if {!($a eq \"x\")} {}", "$a ne\"y\""),
        ("if {!($a eq $b)} {}", "$b ne$a"),
        ("if {!($a == $b)} {}", "$b!=$a"),
        ("if {!($a == 1)} {}", "$a!=2"),
        ("proc expr args {}; expr {!($a == $b)}", "$a!=$b"),
        (
            "set auto_index(unknown_mutation) {source lib.tcl}; unknown_mutation; expr {!($a == $b)}",
            "$a!=$b",
        ),
        (
            "trace add execution expr enter callback; expr {!($a == $b)}",
            "$a!=$b",
        ),
        ("if {!($a < $b)} {}", "$a >= $b"),
        ("if {!($a && $b)} {}", "!$a||!$b"),
        ("proc if args {}; if {!($a eq $b)} {}", "$a ne$b"),
        (
            "set auto_index(unknown_mutation) {source lib.tcl}; unknown_mutation; if {!($a eq $b)} {}",
            "$a ne$b",
        ),
        (
            "trace add execution if enter callback; if {!($a eq $b)} {}",
            "$a ne$b",
        ),
        ("if {!($a eq)} {}", "$a ne$b"),
    ] {
        assert!(
            receipt(source, candidate).is_none(),
            "{source} -> {candidate}"
        );
    }
}

#[test]
fn original_boolean_inversion_retains_selected_version_floor() {
    let receipt = |source: &str, profile: &str, candidate: &str| {
        let analysis = crate::analyser::Analyser::new().analyse(source, profile);
        let config = analysis.body_lexer_config?;
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        crate::registry_invocation::source_structure::original_expression_boolean_comparison_inversion(
                source, &analysis, commands.last()?, 0, candidate,
            )
    };
    assert!(receipt("if {!($a eq $b)} {}", "tcl8.4", "$a ne$b").is_some());
    for (source, candidate) in [
        ("if {!($a in $b)} {}", "$a ni$b"),
        ("if {!($a ni $b)} {}", "$a in$b"),
    ] {
        assert!(receipt(source, "tcl8.4", candidate).is_none());
        assert!(receipt(source, "tcl8.5", candidate).is_some());
    }
    for (source, candidate) in [
        ("if {!($a eq \"unterminated)} {}", "$a ne\"unterminated"),
        ("if {!($a in $b)} {}", "$a ni"),
    ] {
        assert!(receipt(source, "tcl8.6", candidate).is_none());
    }
}

#[test]
fn original_boolean_inversion_checks_leaf_token_coordinates() {
    let parser = ExprParseContext::for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile(),
    );
    let config = LexerConfig::for_profile(Some(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile(),
    ));
    let original = b"!($left eq $right)";
    let candidate = b"$left ne$right";
    let CheckedExprParse::Parsed(tree) =
        tcl_syntax::expr::parser::parse_expr_bytes_checked_with_context(original, &parser)
    else {
        panic!("checked original");
    };
    let CheckedExprParse::Parsed(mut proposed) =
        tcl_syntax::expr::parser::parse_expr_bytes_checked_with_context(candidate, &parser)
    else {
        panic!("checked candidate");
    };
    let original_tokens = expression_syntax_signature(original, &parser, config).unwrap();
    let candidate_tokens = expression_syntax_signature(candidate, &parser, config).unwrap();
    assert!(same_boolean_comparison_inversion(
        &tree,
        &proposed,
        &original_tokens,
        &candidate_tokens
    ));
    let ExprNode::Binary { left, .. } = &mut proposed else {
        panic!("candidate binary");
    };
    let ExprNode::Var { start, .. } = left.as_mut() else {
        panic!("original variable leaf");
    };
    *start += 1;
    assert!(!same_boolean_comparison_inversion(
        &tree,
        &proposed,
        &original_tokens,
        &candidate_tokens
    ));
}

#[test]
fn syntax_expression_gaps_keep_token_data_and_dispatch_guards() {
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    let receipt = |source: &str, candidate: &str| {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let config = analysis.body_lexer_config?;
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        crate::registry_invocation::source_structure::original_expression_syntax_gap_compaction(
            source,
            &analysis,
            commands.last()?,
            0,
            candidate,
        )
    };
    let source = "set a 2; expr {$a + 1}";
    let proof = receipt(source, "$a+1").expect("selected syntax gaps without erasing reads");
    assert_ne!(proof.removed_gaps(), []);
    assert!(proof.matches_source(
        &SourceImage::document(source),
        proof.original_operand().config()
    ));
    for (source, candidate) in [
        ("expr {{a  b} eq {a  b}}", "{a b} eq {a b}"),
        ("expr {1 + 2}", "1+3"),
        ("expr {1 +}", "1+"),
        ("proc expr args {}; expr {1 + 2}", "1+2"),
        (
            "set auto_index(unknown_mutation) {source lib.tcl}; unknown_mutation; expr {1 + 2}",
            "1+2",
        ),
        (
            "trace add execution expr enter callback; expr {1 + 2}",
            "1+2",
        ),
    ] {
        assert!(
            receipt(source, candidate).is_none(),
            "{source} -> {candidate}"
        );
    }
}

#[test]
fn original_expression_gap_tree_checks_issued_leaf_positions() {
    let parser =
        ExprParseContext::for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let config = LexerConfig::for_profile(tcl_dialect::DialectProfile::find("tcl8.6"));
    let original = b" 1 + 2 * 3 ";
    let CheckedExprParse::Parsed(tree) =
        tcl_syntax::expr::parser::parse_expr_bytes_checked_with_context(original, &parser)
    else {
        panic!("original checked Native tree");
    };
    assert!(same_expression_gaps(
        original, b"1+2*3", &parser, config, &tree
    ));
    for candidate in [b"1+2*4".as_slice(), b"1-2*3", b"1+2* 3 + 0"] {
        assert!(!same_expression_gaps(
            original, candidate, &parser, config, &tree
        ));
    }
    let mut wrong_position = tree.clone();
    let ExprNode::Binary { left, .. } = &mut wrong_position else {
        panic!("original binary");
    };
    let ExprNode::Literal { start, .. } = left.as_mut() else {
        panic!("original literal");
    };
    *start += 1;
    assert!(!same_expression_gaps(
        original,
        b"1+2*3",
        &parser,
        config,
        &wrong_position
    ));
    let strings = b"{a  b} eq {a  b}";
    let CheckedExprParse::Parsed(tree) =
        tcl_syntax::expr::parser::parse_expr_bytes_checked_with_context(strings, &parser)
    else {
        panic!("original checked strings");
    };
    assert!(!same_expression_gaps(
        strings,
        b"{a b}eq{a b}",
        &parser,
        config,
        &tree
    ));
}

#[test]
fn original_expression_gaps_retain_pure_native_completion_and_exact_input() {
    let source = "expr { 1 + 2 * 3 }";
    let proof = gap_receipt(source, "1+2*3").expect("pure original finite expression");
    assert_eq!(proof.replacement_source_word(), "{1+2*3}");
    assert_eq!(proof.original_operand().bytes(), b"{ 1 + 2 * 3 }");
    assert_ne!(proof.removed_gaps(), []);
    let config = proof.original_operand().config();
    assert!(proof.matches_source(&SourceImage::document(source), config));
    let mut changed = config;
    changed.strict_quoting = !changed.strict_quoting;
    assert!(!proof.matches_source(&SourceImage::document(source), changed));
    assert!(!proof.matches_source(&SourceImage::document("expr {1+2*3}"), config));
    let mut changed_registry = CommandRegistry::build_default();
    changed_registry.insert_ambient_package("gap-test", "1");
    assert!(!proof.matches_registry(&changed_registry));
    for candidate in ["1+3*3", "1 + 2 * 3 + 0", "1+2*"] {
        assert!(gap_receipt(source, candidate).is_none(), "{candidate}");
    }
    // Whitespace within an expression string is token data, never a gap.
    assert!(gap_receipt("expr {{a  b} eq {a  b}}", "{a b}eq{a b}").is_none());
}

#[test]
fn original_expression_gaps_decline_reads_callbacks_errors_and_source_reflection() {
    for (source, candidate) in [
        ("expr {$a + 1}", "$a+1"),
        ("expr {[callback] + 1}", "[callback]+1"),
        ("expr {abs( 1 )}", "abs(1)"),
        ("expr {1 / 0}", "1/0"),
        ("expr {1 +}", "1+"),
        ("expr {1 + 2}; info frame 0", "1+2"),
        ("proc p {} {expr {1 + 2}}; p", "1+2"),
        (
            "trace add execution expr enter callback; expr {1 + 2}",
            "1+2",
        ),
        ("unknown_mutation; expr {1 + 2}", "1+2"),
        ("proc expr args {}; expr {1 + 2}", "1+2"),
        (
            "expr {1\\
 + 2}",
            "1+2",
        ),
    ] {
        assert!(gap_receipt(source, candidate).is_none(), "{source}");
    }
}

#[test]
fn original_literal_expression_bracing_preserves_native_argv_and_current_owners() {
    // Implementation contract: naming.refactor.original-literal-expression-bracing
    // docs/design/analysis/name-resolution-proofs/original-literal-expression-bracing.md
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let source = "expr \"1 + 2\"";
        let proof = receipt(source, profile)
            .unwrap_or_else(|| panic!("literal bracing missing: {profile}"));
        assert_eq!(proof.native_value(), b"1 + 2");
        assert_eq!(proof.replacement_source_word(), "{1 + 2}");
        assert_eq!(proof.original_operand().bytes(), b"\"1 + 2\"");
        let config = proof.original_operand().config();
        assert!(proof.matches_source(&SourceImage::document(source), config));
        assert!(!proof.matches_source(&SourceImage::native(source.as_bytes()), config));
        assert!(!proof.matches_source(
            &SourceImage::document(&format!("# moved\n{source}")),
            config
        ));
        let mut changed = config;
        changed.strict_quoting = !changed.strict_quoting;
        assert!(!proof.matches_source(&SourceImage::document(source), changed));
    }
}

#[test]
fn original_literal_expression_bracing_declines_substitution_functions_and_handler_changes() {
    // docs/design/analysis/name-resolution-proofs/braced-error-context-publication.md
    // docs/design/analysis/name-resolution-proofs/braced-handler-and-observer-boundaries.md

    // Implementation contract: naming.refactor.original-literal-expression-bracing
    // docs/design/analysis/name-resolution-proofs/original-literal-expression-bracing.md
    for source in [
        "expr {$value}",
        "expr \"$value + 2\"",
        "expr \"\\$value + 2\"",
        "expr \"\\[callback\\]\"",
        "expr \"abs(1)\"",
        // Native captured errorInfo/options differ after changing spelling:
        // naming.expression.braced-error-context-publication.
        r#"expr "1 / 0""#,
        r#"expr """#,
        // Native info-body observer distinguishes these spellings:
        // naming.expression.braced-handler-and-observer-boundaries.
        r#"proc p {} {expr "1 + 2"}; p"#,
        r#"expr "1 + 2"; info frame 0"#,
        "expr 1 + 2",
        "expr {*}{1 2}",
        "expr \"1 +\"",
        "proc expr {args} {return SHADOW}; expr \"1 + 2\"",
        "trace add execution expr enter observer; expr \"1 + 2\"",
        "unknown_mutation; expr \"1 + 2\"",
    ] {
        assert!(receipt(source, "tcl8.6").is_none(), "{source}");
    }
}

#[test]
fn original_literal_bracing_normality_and_source_reflection_are_independent() {
    // docs/design/analysis/name-resolution-proofs/braced-error-context-publication.md
    // docs/design/analysis/name-resolution-proofs/braced-handler-and-observer-boundaries.md

    // Implementation contract: naming.refactor.original-literal-expression-bracing
    // docs/design/analysis/name-resolution-proofs/original-literal-expression-bracing.md
    // Captured natives: naming.expression.braced-error-context-publication
    // and naming.expression.braced-handler-and-observer-boundaries.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let generation = tcl_registry::model::ingress::static_context_for(profile);
        let registry = generation.commands();
        let dialect = tcl_registry::InvocationDialect::of_profile(registry.profile().unwrap());
        let config = LexerConfig::for_profile(registry.profile());
        let policy =
            crate::tcl_expr_eval::FoldPolicy::for_retained_entry(registry, Some(dialect), &config);
        let context = dialect.expression_parse_context(None);
        for (bytes, normal) in [(b"1 + 2".as_slice(), true), (b"1 / 0".as_slice(), false)] {
            let CheckedExprParse::Parsed(tree) =
                tcl_syntax::expr::parser::parse_expr_bytes_checked_with_context(bytes, &context)
            else {
                panic!("selected native grammar: {profile}");
            };
            assert_eq!(
                finite_native_literal_result(&tree, policy),
                normal,
                "{profile}: {bytes:?}"
            );
        }
        for source in [
            r#"expr "1 / 0""#,
            r#"proc p {} {expr "1 + 2"}; p"#,
            r#"expr "1 + 2"; info frame 0"#,
        ] {
            assert!(receipt(source, profile).is_none(), "{profile}: {source}");
        }
    }
}

#[test]
fn authored_braced_argument_roundtrip_keeps_input_channels_and_escape_boundaries_separate() {
    for version in tcl_dialect::TclVersion::ALL {
        let policy = NamePolicyProtocol::authored_tcl(version);
        let config = LexerConfig::for_dialect(version.dialect_name());
        for image in [
            SourceImage::document(""),
            SourceImage::native(b"".as_slice()),
        ] {
            for value in [b"1 + 2".as_slice(), b"{plain}", b"a\\b"] {
                assert!(
                    braced_source_word(value, &image, config, policy).is_some(),
                    "{version:?} {:?} {value:?}",
                    image.channel()
                );
            }
            assert_eq!(
                braced_source_word(b"a\0b", &image, config, policy).is_some(),
                image.channel() == tcl_lexer::SourceChannel::NativeValue,
            );
            assert_eq!(
                braced_source_word(b"a\xc0\x80b", &image, config, policy).is_some(),
                image.channel() == tcl_lexer::SourceChannel::Document,
            );
            assert!(braced_source_word(b"a}b", &image, config, policy).is_none());
            assert!(braced_source_word(b"a\\\nb", &image, config, policy).is_none());
        }
    }
}

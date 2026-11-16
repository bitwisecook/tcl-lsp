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

/// Depth coverage: `collect_expr_command_texts` recurses once per
/// `ExprNode` level, so without a depth cap it overflows the native stack
/// (SIGABRT) in the low thousands of levels on a 2 MiB thread.  A tree
/// built directly is unbounded (the Pratt parser caps its own output at
/// 256); 3000 is past that crash range and past `MAX_EXPR_NODE_DEPTH`
/// (256); the assertion is that it returns at all.
#[test]
fn deeply_nested_collect_expr_command_texts_survives() {
    let mut node = ExprNode::Command {
        text: "[x]".into(),
        start: 0,
        end: 3,
    };
    for _ in 0..3000 {
        node = ExprNode::Unary {
            op: crate::expr_ast::UnaryOp::Not,
            operand: Box::new(node),
        };
    }
    let mut out = Vec::new();
    collect_expr_command_texts(&node, &mut out);
}

#[test]
fn opaque_body_local_set_requires_original_body_owner() {
    // naming.source.original-editor-body-structure
    // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
    // Conditional source suppression only; no entered interpreter or store.
    let source = "interp eval {} {set x 1; puts $x}\n";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
    let config = analysis.body_lexer_config.unwrap();
    let context = analysis.resolved_input.as_ref().unwrap().context_registry();
    let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let mut tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        &segments[0],
    );
    analysis
        .retained_command_realm()
        .unwrap()
        .stamp_original_tokens(&mut tokens);
    let stmt = crate::ir::Statement::Barrier {
        span: tokens.argv[0],
        reason: "original current-interpreter body".to_owned(),
        command: "interp".to_owned(),
        canonical_command: Some("::interp".to_owned()),
        args: vec![
            "eval".to_owned(),
            String::new(),
            "set x 1; puts $x".to_owned(),
        ],
        tokens: Some(tokens.clone()),
    };
    assert!(barrier_body_locally_sets(
        Some(&stmt),
        "x",
        source,
        &analysis,
        &context,
        Some(&tokens)
    ));
    assert!(!barrier_body_locally_sets(
        Some(&stmt),
        "x",
        source,
        &analysis,
        &context,
        None
    ));
    assert!(!barrier_body_locally_sets(
        Some(&stmt),
        "x",
        "interp eval {} {set y 1; puts $y}\n",
        &analysis,
        &context,
        Some(&tokens)
    ));
}

#[test]
fn original_body_ownership_keeps_installer_horizon_and_child_refusal() {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    // Conditional suppression, never an entered child or completed store.
    for (source, target, expected) in [
        ("interp eval {} {puts $missing}", "missing", false),
        ("interp eval {} {set missing}", "missing", true),
        (
            "interp alias {} put {} set {$literal}; interp eval {} {put VALUE}",
            "$literal",
            true,
        ),
        (
            "interp alias {} put {} set café; rename set {}; interp eval {} {put VALUE}",
            "café",
            false,
        ),
        (
            "interp create child; interp eval child {set x VALUE}",
            "x",
            false,
        ),
        ("interp eval {} {set {café(open} VALUE}", "café(open", true),
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let config = analysis.body_lexer_config.unwrap();
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            segments.last().unwrap(),
        );
        analysis
            .retained_command_realm()
            .unwrap()
            .stamp_original_tokens(&mut tokens);
        let stmt = crate::ir::Statement::Barrier {
            span: segments.last().unwrap().span,
            reason: "conditional body".into(),
            command: segments.last().unwrap().texts[0].clone(),
            canonical_command: None,
            args: Vec::new(),
            tokens: Some(tokens.clone()),
        };
        assert_eq!(
            barrier_body_locally_sets(
                Some(&stmt),
                target,
                source,
                &analysis,
                &context,
                Some(&tokens)
            ),
            expected,
            "{source}"
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(!barrier_body_locally_sets(
            Some(&stmt),
            target,
            source,
            &missing,
            &context,
            Some(&tokens)
        ));
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        assert!(!barrier_body_locally_sets(
            Some(&stmt),
            target,
            source,
            &analysis,
            &foreign,
            Some(&tokens)
        ));
    }
}

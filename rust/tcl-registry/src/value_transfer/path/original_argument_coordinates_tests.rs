// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::value_transfer::{LiteralInputs, OperandId, evaluate_literal};

#[test]
fn original_path_values_keep_the_first_operand_in_both_selected_layouts() {
    // naming.navigation.retained-path-source-inventory

    // Pure value coordinates, without source lookup or filesystem execution.
    for (route, selector, values, expected) in [
        (&FILE_JOIN, "join", vec!["ROOT", "child"], "ROOT/child"),
        (&FILE_JOIN, "join", vec!["/ROOT"], "/ROOT"),
        (&FILE_DIRNAME, "dirname", vec!["/ROOT/child"], "/ROOT"),
        (&FILE_TAIL, "tail", vec!["/ROOT/child"], "child"),
        (
            &FILE_EXTENSION,
            "extension",
            vec!["/ROOT/child.tcl"],
            ".tcl",
        ),
        (
            &FILE_ROOTNAME,
            "rootname",
            vec!["/ROOT/child.tcl"],
            "/ROOT/child",
        ),
    ] {
        assert_eq!(
            evaluate_literal(route, "file", Some(selector), &values, None).as_deref(),
            Some(expected)
        );
        assert_eq!(
            evaluate_literal(route, "", None, &values, None).as_deref(),
            Some(expected)
        );
    }
    for selector in [None, Some("join")] {
        assert!(evaluate_literal(&FILE_JOIN, "", selector, &[], None).is_none());
    }
    for selector in [None, Some("dirname")] {
        assert!(evaluate_literal(&FILE_DIRNAME, "", selector, &[], None).is_none());
        assert!(evaluate_literal(&FILE_DIRNAME, "", selector, &["ROOT", "child"], None).is_none());
    }
}

#[test]
fn original_path_coordinates_keep_unknown_first_values_and_platform_declines_terminal() {
    // naming.navigation.retained-path-source-inventory

    // An unknown original first value cannot be skipped to obtain a result.
    let values = ["/ROOT", "child"];
    for (selector, first) in [(None, 0), (Some("join"), 1)] {
        let input =
            LiteralInputs::new("file", selector, &values, None).with_unproven(OperandId(first));
        assert!(matches!(
            FILE_JOIN.evaluate(&input, &mut Budget::evaluation()),
            EvalAnswer::Declined(_)
        ));
        for values in [["C:/ROOT", "child"], ["/ROOT", "C:/child"]] {
            assert!(evaluate_literal(&FILE_JOIN, "file", selector, &values, None).is_none());
        }
    }
}

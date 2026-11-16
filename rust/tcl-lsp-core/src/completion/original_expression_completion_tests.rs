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
use tcl_compiler::analyser::Analyser;

#[test]
fn incomplete_root_expression_offers_registry_math_functions() {
    for (source, needle, expected) in [
        ("if {ma\n", "ma", true),
        ("expr {si\n", "si", true),
        ("proc if args {}; if {ma\n", "ma", false),
        ("set data {ma\n", "ma", false),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let cursor = u32::try_from(source.rfind(needle).unwrap() + needle.len()).unwrap();
        let items = completions(
            source,
            0,
            cursor,
            &analysis,
            analysis.resolved_registry(),
            None,
            analysis.resolved_profile().unwrap(),
        );
        assert_eq!(
            items
                .iter()
                .any(|item| item.detail.as_deref() == Some("expr math function")),
            expected,
            "{source}"
        );
    }
}

#[test]
fn incomplete_nested_expression_offers_math_functions() {
    let source = "set a [expr {si\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let items = completions(
        source,
        0,
        15,
        &analysis,
        analysis.resolved_registry(),
        None,
        analysis.resolved_profile().unwrap(),
    );
    assert!(items.iter().any(|item| item.label == "sin"));

    let source = "proc sizzler {} {}\nset a [expr {si\n";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let items = completions(
        source,
        1,
        15,
        &analysis,
        analysis.resolved_registry(),
        None,
        analysis.resolved_profile().unwrap(),
    );
    assert!(items.iter().any(|item| item.label == "sin"));
    assert!(!items.iter().any(|item| item.label == "sizzler"));
}

#[test]
// Implementation contract: naming.core.original-expression-source-selection
// docs/design/analysis/name-resolution-proofs/original-expression-source-selection.md
fn original_expression_completion_does_not_borrow_a_shadowed_role() {
    for (source, expected) in [
        ("expr {si}", true),
        ("proc expr args {}; expr {si}", false),
        ("set data {si}", false),
    ] {
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let cursor = u32::try_from(source.rfind("si").unwrap() + 2).unwrap();
        let registry = analysis.resolved_registry().unwrap();
        let items = completions(
            source,
            0,
            cursor,
            &analysis,
            Some(registry),
            None,
            analysis.resolved_profile().unwrap(),
        );
        assert_eq!(
            items
                .iter()
                .any(|item| item.detail.as_deref() == Some("expr math function")),
            expected,
            "{source}"
        );
    }
}

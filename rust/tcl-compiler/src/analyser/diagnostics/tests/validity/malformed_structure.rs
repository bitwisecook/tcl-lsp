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

//! E006: an invalid formal parameter list.

use super::*;

#[test]
fn e007_needs_the_actual_store_and_the_same_initialised_constant() {
    let source = "const fixed 3\nset fixed\nset fixed 4\n";
    for profile in ["tcl9.0", "tcl9.1"] {
        let mut analyser = crate::analyser::Analyser::new();
        let result = analyser.analyse(source, profile);
        let matches = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::E007)
            .collect::<Vec<_>>();
        let [diagnostic] = matches.as_slice() else {
            panic!("expected one constant write for {profile}: {matches:?}");
        };
        assert_eq!(
            diagnostic.span.start() as usize,
            source.rfind("fixed").unwrap()
        );
        assert_eq!(&source[diagnostic.span.as_range()], "fixed");
    }
    assert!(!has_code(source, "tcl8.6", "E007"));
    assert!(has_code(source, "tcl8.6", "W002"));
    for source in [
        "const fixed 3; puts $fixed; set fixed 4",
        "puts ready; const fixed 3; set fixed 4",
        "const fixed 3; puts -nonewline stdout ready; set fixed 4",
        "const fixed 3; chan put stdout ready; set fixed 4",
        "const fixed 3; chan puts stdout ready; set fixed 4",
        "const fixed 3; read stdin; set fixed 4",
        "const fixed 3; exec true >@stdout; set fixed 4",
        "const fixed 3; catch {puts -nonewline stdout ready}; set fixed 4",
        "const fixed 3; update; set fixed 4",
        "const fixed 3; update idletasks; set fixed 4",
        "const fixed 3; vwait ready; set fixed 4",
        "const fixed 3; proc cmp args {rename set savedSet; proc set args {return 4}; return 0}; lsort -command cmp {1 2}; set fixed 4",
        "const fixed 3; rename set oldSet; proc set args {}; set fixed 4",
        "namespace eval Box {const fixed 3}; namespace delete Box; namespace eval Box {set fixed 4}",
    ] {
        assert!(
            !has_code(source, "tcl9.1", "E007"),
            "unexpected E007 for {source:?}"
        );
    }
}

#[test]
fn e006_rejects_invalid_literal_formal_parameter_lists_from_registry_roles() {
    // Tcl 9.0.4 rejects a formal specifier with three fields, a namespace
    // qualified name, and an array element.  Duplicate parameter names remain
    // legal, and a value computed at runtime is deliberately outside the
    // analyser's proof boundary.
    for src in [
        "proc invalid {{a b c}} {}\n",
        "proc invalid {a::b} {}\n",
        "proc invalid {a(x)} {}\n",
    ] {
        assert!(has_code(src, "tcl9.0", "E006"), "expected E006 for {src:?}");
    }
    for src in [
        "proc duplicate {a a} {}\n",
        "set params {a::b}\nproc dynamic $params {}\n",
    ] {
        assert!(!has_code(src, "tcl9.0", "E006"), "must abstain for {src:?}");
    }
}

#[test]
fn e006_requires_unanimous_actual_formal_grammar_after_alias_mapping() {
    assert!(has_code(
        "interp alias {} declare {} proc p; declare {{a b c}} {}",
        "tcl8.6",
        "E006",
    ));
    for source in [
        "rename proc original; original proc args {return ordinary}; proc p {{a b c}} {}",
        "if {$unknown} {rename proc original; original proc args {return ordinary}}; proc p {{a b c}} {}",
        "proc p {*}$unknown",
        "external p {{a b c}} {}",
    ] {
        assert!(
            !has_code(source, "tcl8.6", "E006"),
            "opaque grammar: {source}"
        );
    }
}

#[test]
fn e006_offers_only_the_structurally_unambiguous_parameter_repair() {
    let mut analyser = crate::analyser::Analyser::new();
    let result = analyser.analyse("proc invalid {{a b c}} {}\n", "tcl9.0");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::E006)
        .expect("E006 is emitted");

    assert_eq!(diagnostic.fixes.len(), 1);
    assert_eq!(diagnostic.fixes[0].new_text, "{a b c}");
    assert_eq!(
        diagnostic.fixes[0].safety,
        crate::analyser::types::FixSafety::RequiresReview
    );

    for source in ["proc invalid {a::b} {}\n", "proc invalid {a(x)} {}\n"] {
        let result = analyser.analyse(source, "tcl9.0");
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagCode::E006)
            .expect("E006 is emitted");
        assert_eq!(diagnostic.fixes.len(), 0, "must abstain for {source:?}");
    }
}

#[test]
fn e006_discovers_definition_body_and_lambda_parameter_shapes() {
    // These are not ordinary top-level command arguments.  The TclOO method
    // shape comes from DefinitionBodyGrammar::member / MemberSpec::arg_roles;
    // `apply`'s nested first field comes from ArgRole::LambdaLiteral.
    assert!(has_code(
        "oo::class create C { method invalid {{a b c}} {} }\n",
        "tcl9.0",
        "E006",
    ));
    assert!(has_code(
        "oo::class create C {}\noo::define C method invalid {{a b c}} {}\n",
        "tcl9.0",
        "E006",
    ));
    assert!(has_code("apply {{{a b c}} {return 1}}\n", "tcl9.0", "E006",));
}

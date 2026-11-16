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

//! `dialect` blocks: the ladder and axes, unknown axes and values, and a block duplicating a compiled release.

use super::*;

/// A `dialect` block parses its ladder and its axes against the closed
/// vocabulary.
#[test]
fn a_dialect_block_loads_its_ladder_and_axes() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n dialect picol2 {\n release 2.0\n \
             release 2.1 -build Unknown\n axis expand_syntax off\n \
             axis braced_var first-close\n axis array_index tcl8\n axis numbers tcl90\n \
             axis escapes tcl90\n axis expr_comments hash\n \
             axis bom_skip on\n }\n command demo { arity 1 }\n}",
    );
    assert!(pack.notices.is_empty(), "{:?}", pack.notices);
    let dialect = &pack.dialects[0];
    assert_eq!(dialect.name, "picol2");
    assert_eq!(dialect.releases.len(), 2);
    assert_eq!(dialect.releases[1].build, BuildProfileId::Unknown);
    assert_eq!(dialect.axis("expand_syntax"), Some("off"));
    assert_eq!(dialect.axis("nonesuch"), None);
    let grammar = dialect.to_grammar().expect("every value has a backing");
    assert!(!grammar.expand_syntax);
    assert_eq!(grammar.braced_var, BracedVarStyle::FirstClose);
    assert_eq!(grammar.array_index, tcl_dialect::ArrayIndexSyntax::Tcl8);
}

/// An unknown axis, or an unknown value on a known axis, is §6.1's
/// semantic class: the whole block is rejected, the notice names the
/// axis, and the pack's other content still loads.
#[test]
fn an_unknown_axis_or_value_rejects_the_dialect_block() {
    for (body, needle) in [
        (
            "axis no_such_axis on",
            "is not in the closed axis vocabulary",
        ),
        ("axis numbers tcl99", "is not a value of `numbers`"),
    ] {
        let pack = evaluate_pack(&format!(
            "speclib probe 2.0 {{\n dialect probe-dialect {{\n {body}\n }}\n \
                 command demo {{ arity 1 }}\n}}"
        ));
        assert!(pack.dialects.is_empty(), "{body}: {:?}", pack.dialects);
        assert!(pack.command("demo").is_some(), "{body}: the rest loads");
        assert!(
            pack.notices.iter().any(|n| n.message.contains(needle)),
            "{body}: {:?}",
            pack.notices
        );
    }
}

/// §2's classification gate: a `dialect` block whose axes reproduce a
/// compiled release is not a dialect at all, and the notice names the
/// environment it should have been.
#[test]
fn a_dialect_block_duplicating_a_compiled_release_is_rejected() {
    let pack = evaluate_pack(
        "speclib probe 2.0 {\n dialect my-tcl {\n release 1.0\n \
             axis expand_syntax on\n axis braced_var first-close\n \
             axis numbers tcl85\n axis escapes tcl86\n \
             axis array_index tcl8\n axis expr_comments none\n axis bom_skip off\n \
             axis irules_brace_separator off\n }\n command demo { arity 1 }\n}",
    );
    assert!(pack.dialects.is_empty(), "{:?}", pack.dialects);
    assert!(
        pack.notices.iter().any(|n| {
            n.message.contains("declares the grammar of tcl 8.6")
                && n.message.contains("environment my-tcl { core tcl 8.6")
        }),
        "{:?}",
        pack.notices
    );
}

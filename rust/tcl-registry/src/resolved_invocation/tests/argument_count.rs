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

//! Argument counting: arity count layout, positional arity behind an option boundary and reserved tails.

use super::*;

#[test]
fn source_arity_count_layout_keeps_axis_prefix_data_and_expansion_bounds() {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    let owner = crate::model::ingress::static_context_for("tcl8.6");
    for (head, arguments, minimum, indeterminate, operands) in [
        (
            "set",
            vec![Literal("name"), Dynamic, Expanded, Literal("extra")],
            3,
            true,
            vec![0, 1, 3],
        ),
        ("string", vec![Literal("le"), Dynamic], 1, false, vec![1]),
        ("fconfigure", vec![], 0, false, vec![]),
        ("fconfigure", vec![Dynamic], 1, false, vec![0]),
    ] {
        let resolution = owner.commands().resolve_structured_invocation(
            crate::InvocationWords::structured(Literal(head), &arguments),
            Some(owner.context().authoring_query()),
        );
        let selected = resolution.resolved().unwrap();
        let count = selected.authored_source_arity().unwrap().count;
        assert_eq!(
            (count.minimum, count.indeterminate, count.operands),
            (minimum, indeterminate, operands),
            "{head}"
        );
    }
    let arguments = [Dynamic, Literal("-buffering"), Literal("none")];
    let resolution = owner.commands().resolve_structured_invocation(
        crate::InvocationWords::structured(Literal("fconfigure"), &arguments),
        Some(owner.context().authoring_query()),
    );
    let selected = resolution.resolved().unwrap();
    let all = selected
        .authored_source_count_for_arity(crate::Arity::any())
        .unwrap();
    assert_eq!(all.operands, vec![0, 1, 2]);
    let positional = selected
        .authored_source_count_for_arity(crate::Arity::any().with_positionals())
        .unwrap();
    assert_eq!(positional.operands, vec![0]);
    let arguments = [Expanded, Literal("pattern"), Dynamic];
    let resolution = owner.commands().resolve_structured_invocation(
        crate::InvocationWords::structured(Literal("regexp"), &arguments),
        Some(owner.context().authoring_query()),
    );
    assert!(
        resolution
            .resolved()
            .unwrap()
            .authored_source_count_for_arity(crate::Arity::any().with_positionals())
            .is_none()
    );
}

#[test]
fn positional_arity_uses_proved_option_boundary_without_inventing_dynamic_values() {
    use crate::InvocationWord::{Dynamic, DynamicNonOption, Expanded, Literal};
    let registry = CommandRegistry::build_default();
    let cases: &[(&[crate::InvocationWord<'_>], Option<u16>)] = &[
        (
            &[Literal("create"), Literal("-safe"), Literal("jail")],
            Some(1),
        ),
        (
            &[Literal("create"), Literal("-safe"), Literal("-safe")],
            Some(0),
        ),
        (
            &[Literal("create"), Literal("--"), Literal("-safe")],
            Some(1),
        ),
        (&[Literal("create"), Literal("--"), Dynamic], Some(1)),
        (
            &[Literal("create"), Literal("-safe"), DynamicNonOption],
            Some(1),
        ),
        (&[Literal("create"), Dynamic], None),
        (&[Literal("create"), Literal("-safe"), Dynamic], None),
        (&[Literal("create"), Literal("-unknown")], None),
        (&[Literal("create"), Expanded], None),
        (
            &[Literal("create"), Literal("jail"), Literal("extra")],
            Some(2),
        ),
    ];
    for &(arguments, expected) in cases {
        let invocation = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("interp"), arguments),
                None,
            )
            .resolved()
            .expect("literal interp selector resolves");
        assert_eq!(
            invocation.argument_count_for_arity(),
            expected,
            "{arguments:?}"
        );
        assert_eq!(
            invocation.facts().arity_accepts_frozen_arguments(),
            expected.map(|count| count <= 1),
            "{arguments:?}"
        );
    }
    let arguments = [Literal("-nonewline"), Dynamic];
    let invocation = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(Literal("read"), &arguments),
            None,
        )
        .resolved()
        .expect("literal read resolves");
    assert_eq!(
        invocation.argument_count_for_arity(),
        Some(2),
        "raw argument signatures retain their authored count axis"
    );
}

#[test]
fn leading_flags_are_not_positionals_and_the_final_word_never_is_one() {
    use crate::InvocationWord::Literal;
    let registry = CommandRegistry::build_default();
    // tclsh 8.4-9.0: a leading flag is not a positional, and the final
    // word never is a flag (`puts -nonewline` prints `-nonewline`).
    for (head, arguments, expected) in [
        (
            "puts",
            vec![Literal("-nonewline"), Literal("stdout"), Literal("x")],
            2,
        ),
        ("puts", vec![Literal("-nonewline")], 1),
        (
            "file",
            vec![
                Literal("link"),
                Literal("-symbolic"),
                Literal("a"),
                Literal("b"),
            ],
            2,
        ),
        ("file", vec![Literal("link"), Literal("-hard")], 1),
        (
            "tcl::prefix",
            vec![
                Literal("match"),
                Literal("-exact"),
                Literal("t"),
                Literal("s"),
            ],
            2,
        ),
        (
            "interp",
            vec![
                Literal("cancel"),
                Literal("-unwind"),
                Literal("--"),
                Literal("x"),
            ],
            1,
        ),
    ] {
        let invocation = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal(head), &arguments),
                None,
            )
            .resolved()
            .expect("literal command resolves");
        assert_eq!(
            invocation.argument_count_for_arity(),
            Some(expected),
            "{head} {arguments:?}"
        );
    }
}

#[test]
fn positional_count_preserves_native_reserved_tail_and_case_layout() {
    use crate::InvocationWord::{Dynamic, Literal};
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = crate::InvocationDialect::for_version(version);
        let registry = crate::CommandRegistry::build_default();
        let cases: &[(&str, &[crate::InvocationWord<'_>], u16)] = &[
            ("lsort", &[Literal("-integer"), Dynamic], 1),
            ("lsort", &[Literal("-integer")], 1),
            ("lsearch", &[Literal("-exact"), Dynamic, Dynamic], 2),
            ("lsearch", &[Literal("-exact"), Dynamic], 2),
            (
                "switch",
                &[Literal("-exact"), Literal("--"), Dynamic, Literal("x {}")],
                2,
            ),
        ];
        for &(head, arguments, expected) in cases {
            let words =
                crate::InvocationWords::structured(Literal(head), arguments).with_dialect(dialect);
            let invocation = registry
                .resolve_structured_invocation(words, dialect.authoring_query())
                .resolved()
                .expect("native literal head");
            assert_eq!(
                invocation.argument_count_for_arity(),
                Some(expected),
                "{version:?} {head}"
            );
            assert_eq!(
                invocation.facts().arity_accepts_frozen_arguments(),
                Some(true)
            );
        }
        let arguments = [Literal("-exact"), Literal("-exact {}")];
        let invocation = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(Literal("switch"), &arguments)
                    .with_dialect(dialect),
                dialect.authoring_query(),
            )
            .resolved()
            .expect("native switch");
        assert_eq!(
            invocation.argument_count_for_arity(),
            if version == tcl_dialect::TclVersion::V8_4 {
                None
            } else {
                Some(2)
            }
        );
    }
}

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

use crate::ArgRole;
#[test]
fn selected_logical_result_binding_axis_keeps_native_effects_unknown() {
    // naming.minifier.logical-formal-binding-alpha
    // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context = crate::model::ingress::context_for_profile(profile);
    assert!(
        tcl_native_model::InvocationDialect::of_profile(profile)
            .native_name_protocol()
            .is_none()
    );
    for (arguments, expected) in [
        (Vec::new(), true),
        (vec![crate::InvocationWord::Dynamic], true),
        (
            vec![
                crate::InvocationWord::Literal("-code"),
                crate::InvocationWord::Literal("ok"),
                crate::InvocationWord::Dynamic,
            ],
            false,
        ),
        (
            vec![
                crate::InvocationWord::Literal("-errorinfo"),
                crate::InvocationWord::Dynamic,
            ],
            false,
        ),
        (vec![crate::InvocationWord::Expanded], false),
    ] {
        let resolution = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(
                crate::InvocationWord::Literal("return"),
                &arguments,
            ),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
        let schema = resolution.resolved().unwrap();
        assert_eq!(
            schema.authored_source_result_preserves_variable_bindings(),
            expected
        );
        if expected {
            assert_eq!(
                schema.authored_source_argument_roles(),
                (
                    if arguments.is_empty() {
                        Vec::new()
                    } else {
                        vec![(0, crate::ArgRole::Result)]
                    },
                    true
                )
            );
        }
        assert!(schema.facts().effects.requires_world_barrier());
    }
}
#[test]
fn structured_conditional_and_output_roles_keep_unknown_payloads() {
    // naming.source.authored-registry-role-projection
    // docs/design/analysis/name-resolution-proofs/authored-registry-role-projection.md
    use crate::InvocationWord::{Dynamic, Expanded, Literal};
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let owner = crate::model::ingress::static_context_for(profile);
        let dialect =
            tcl_native_model::InvocationDialect::of_profile(owner.commands().profile().unwrap());
        for (head, arguments, expected) in [
            (
                "if",
                vec![Dynamic, Literal("then"), Dynamic],
                vec![
                    (0, ArgRole::Expr),
                    (1, ArgRole::Keyword),
                    (2, ArgRole::Body),
                ],
            ),
            (
                "if",
                vec![
                    Dynamic,
                    Literal("{}"),
                    Literal("elseif"),
                    Dynamic,
                    Literal("then"),
                    Dynamic,
                    Literal("else"),
                    Dynamic,
                ],
                vec![
                    (0, ArgRole::Expr),
                    (1, ArgRole::Body),
                    (2, ArgRole::Keyword),
                    (3, ArgRole::Expr),
                    (4, ArgRole::Keyword),
                    (5, ArgRole::Body),
                    (6, ArgRole::Keyword),
                    (7, ArgRole::Body),
                ],
            ),
            ("puts", vec![Dynamic], vec![]),
            (
                "puts",
                vec![Literal("-nonewline"), Dynamic, Dynamic],
                vec![(1, ArgRole::Channel)],
            ),
            (
                "puts",
                vec![Literal("stdout"), Dynamic],
                vec![(0, ArgRole::Channel)],
            ),
        ] {
            let selected = owner.commands().resolve_structured_invocation(
                crate::InvocationWords::structured(Literal(head), &arguments).with_dialect(dialect),
                Some(owner.context().authoring_query()),
            );
            let selected = selected.resolved().unwrap();
            assert_eq!(
                selected.authored_source_argument_roles(),
                (expected, true),
                "{profile} {head}"
            );
        }
        for (head, arguments) in [
            ("if", vec![Dynamic, Dynamic]),
            ("if", vec![Dynamic, Literal("{}"), Dynamic]),
            ("if", vec![Expanded, Literal("{}")]),
            ("puts", vec![Dynamic, Dynamic]),
            ("puts", vec![Expanded]),
        ] {
            let selected = owner.commands().resolve_structured_invocation(
                crate::InvocationWords::structured(Literal(head), &arguments).with_dialect(dialect),
                Some(owner.context().authoring_query()),
            );
            assert!(
                !selected
                    .resolved()
                    .unwrap()
                    .authored_source_argument_roles()
                    .1,
                "{profile} {head}"
            );
        }
    }
}

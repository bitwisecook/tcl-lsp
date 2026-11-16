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

fn payload_roles(values: &[&str]) -> Vec<(u8, ArgRole)> {
    if values.first() == Some(&"body") {
        vec![(1, ArgRole::Body)]
    } else {
        vec![(1, ArgRole::VarRead)]
    }
}

#[test]
fn arbitrary_literal_hook_never_observes_a_computed_empty_placeholder() {
    let mut registry = crate::CommandRegistry::build_default();
    registry.insert(crate::CommandSpec {
        name: "payload-dependent-source-roles",
        arity: crate::Arity::any(),
        arg_role_resolver: Some(payload_roles),
        ..crate::CommandSpec::DEFAULT
    });
    for (selector, expected) in [("body", ArgRole::Body), ("", ArgRole::VarRead)] {
        let values = [
            crate::InvocationWord::Literal(selector),
            crate::InvocationWord::Literal("operand"),
        ];
        let selected = registry
            .resolve_structured_invocation(
                crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("payload-dependent-source-roles"),
                    &values,
                ),
                None,
            )
            .resolved()
            .unwrap();
        assert_eq!(selected.arg_roles(), Some(vec![(1, expected)]));
    }
    let values = [
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Literal("operand"),
    ];
    let selected = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(
                crate::InvocationWord::Literal("payload-dependent-source-roles"),
                &values,
            ),
            None,
        )
        .resolved()
        .unwrap();
    assert!(selected.arg_roles().is_none());
    assert!(!selected.authored_source_argument_roles().1);
}

#[test]
fn existing_count_owned_roles_still_accept_unknown_operand_values() {
    let registry = crate::CommandRegistry::build_default();
    let values = [
        crate::InvocationWord::Dynamic,
        crate::InvocationWord::Literal("destination"),
    ];
    let selected = registry
        .resolve_structured_invocation(
            crate::InvocationWords::structured(crate::InvocationWord::Literal("lassign"), &values),
            None,
        )
        .resolved()
        .unwrap();
    assert!(
        selected
            .arg_roles()
            .unwrap()
            .contains(&(1, ArgRole::VarWrite))
    );
}

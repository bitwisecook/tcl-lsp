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

//! `control::do` command.
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "control::do body ?option test?",
}];

/// `while` / `until` — the loop-sense keyword between `body` and `test`.
const OPTION_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "while",
        detail: "Repeat while test is true.",
        min_tcl: None,
    },
    ArgValue {
        value: "until",
        detail: "Repeat until test becomes true.",
        min_tcl: None,
    },
];

/// Argument roles for `control::do body ?option test?`.
///
/// The `?option test?` tail is only meaningful as a complete pair, so
/// the `option` (`while`/`until`) keyword and the `test` expression
/// roles are assigned only for the full three-word form. A malformed
/// two-word call (`body` + a lone `option`) leaves the second word
/// unroled — it must not be highlighted as a keyword just because it
/// sits at that position.
fn control_do_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let mut roles = vec![(0, ArgRole::Body)];
    if args.len() >= 3 {
        roles.push((1, ArgRole::Keyword));
        roles.push((2, ArgRole::Expr));
    }
    roles
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "control::do",
        dialects: None,
        arity: Arity::new(1, 3),
        // `body` is a script evaluated in the caller's frame (Plain),
        // `option` is the `while`/`until` keyword, and `test` is an
        // expression evaluated like `expr`. Roles are resolved
        // dynamically so the option/test roles only apply to the
        // complete `body option test` form.
        arg_role_resolver: Some(control_do_arg_roles),
        arg_values: &[(1, OPTION_VALUES)],
        arg_types: &[(
            2,
            ArgTypeHint {
                expected: Some(TclType::Boolean),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        hover: Some(HoverSnippet {
            summary: "Evaluate a script repeatedly while/until a condition holds.",
            synopsis: &["control::do body ?option test?"],
            snippet: "Evaluates body repeatedly until test becomes true (until) or as long as test is true (while), depending on option. With option and test omitted, body runs exactly once. The body always runs at least once.",
            source: "tcllib control package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("control"),
        required_package: Some("control"),
        ..CommandSpec::DEFAULT
    }
}

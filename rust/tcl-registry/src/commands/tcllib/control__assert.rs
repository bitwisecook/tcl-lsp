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

//! `control::assert` command.
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "control::assert expr ?arg arg ...?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "control::assert",
        dialects: None,
        arity: Arity::at_least(1),
        // `expr` is evaluated as an expression, like `expr` itself.
        arg_roles: &[(0, ArgRole::Expr)],
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Boolean),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        hover: Some(HoverSnippet {
            summary: "Raise an error if a boolean expression is false (when enabled).",
            synopsis: &["control::assert expr ?arg arg ...?"],
            snippet: "When enabled, evaluates expr as a boolean expression and raises an error if it is false. Any trailing arguments form the failure message. When disabled, behaves like control::no-op.",
            source: "tcllib control package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("control"),
        required_package: Some("control"),
        ..CommandSpec::DEFAULT
    }
}

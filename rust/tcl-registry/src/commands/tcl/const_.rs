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

//! `const` — define a constant variable (Tcl 9 / TIP 677).

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "const varName value",
}];

/// Command spec for `const`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "const",
        // Intentionally universal (`dialects: None`) rather than Tcl-9.0-gated:
        // kept dialect-agnostic so it stays valid inside iRules events. See
        // `tcl9_commands_gated_to_tcl90` in registry.rs.
        dialects: None,
        arity: Arity::new(2, 2),
        assigns_variable_at: Some(0),
        arg_roles: &[(0, ArgRole::VarWrite)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Define a constant variable.",
            synopsis: &["const varName value"],
            snippet: "",
            source: "Tcl const(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

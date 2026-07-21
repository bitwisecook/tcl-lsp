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

//! `ledit` — replace elements in a list variable, in place (Tcl 9.0+, TIP 631).

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ledit listVar first last ?element element ...?",
    dialects: None,
}];

/// Command spec for `ledit`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ledit",
        // `ledit` reads the list variable's current value, replaces a range,
        // and writes the result back — a read-before-write of `listVar`.
        traits: Traits::READS_BEFORE_WRITE,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(3),
        assigns_variable_at: Some(0),
        arg_roles: &[(0, ArgRole::VarWrite)],
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet {
            summary: "Replace elements in a list variable, in place.",
            synopsis: &["ledit listVar first last ?element element ...?"],
            snippet: "ledit replaces zero or more elements (between indices first and last) of the list stored in listVar with the element arguments, updating the variable and returning its new value.",
            source: "Tcl 9 man page ledit.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        arg_types: &[
            (
                0,
                ArgTypeHint {
                    expected: Some(TclType::List),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        ..CommandSpec::DEFAULT
    }
}

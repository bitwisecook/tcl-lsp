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

//! `lpop` — get and remove an element from a list variable (Tcl 9.0+, TIP 523).

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lpop varName ?index ...?",
    dialects: None,
}];

/// Command spec for `lpop`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lpop",
        // `lpop` reads the list's current value before removing an element.
        traits: Traits::READS_BEFORE_WRITE,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(1),
        assigns_variable_at: Some(0),
        arg_roles: &[(0, ArgRole::VarWrite)],
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        return_type: Some(TclType::String),
        // `lpop` returns the *removed element* but leaves the variable holding
        // the shortened *list*.  Type the write as the list it always becomes,
        // not the popped element's return type (issue #867).
        var_write_typing: VarWriteTyping::Fixed(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet {
            summary: "Get and remove an element in a list variable.",
            synopsis: &["lpop varName ?index ...?"],
            snippet: "",
            source: "Tcl 9 man page lpop.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

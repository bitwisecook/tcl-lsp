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

//! `lreverse` — reverse a list.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lreverse list",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lreverse",
        const_fold: Some(crate::const_fold::fold_lreverse),
        traits: Traits::FRAMELESS_RUNTIME | Traits::PURE,
        // Added in Tcl 8.5 (TIP 272).
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::exact(1),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Reverse the order of a list",
            synopsis: &["lreverse list"],
            snippet: "The lreverse command returns a list that has the same elements as its input list, list, except with the elements in the reverse order.",
            source: "Tcl man page lreverse.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..CommandSpec::DEFAULT
    }
}

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

//! `join` — join list elements into a string.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "join list ?joinString?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "join",
        byte_array_effect: ByteArrayEffect::Coerces,
        const_fold: Some(crate::const_fold::fold_join),
        traits: Traits::FRAMELESS_RUNTIME | Traits::PURE | Traits::CSE_CANDIDATE,
        arity: Arity::new(1, 2),
        return_type: Some(TclType::String),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
            },
        )],
        hover: Some(HoverSnippet {
            summary: "Create a string by joining together list elements",
            synopsis: &["join list ?joinString?"],
            snippet: "The list argument must be a valid Tcl list.",
            source: "Tcl man page join.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

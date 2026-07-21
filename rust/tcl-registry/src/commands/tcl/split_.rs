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

//! `split` — split a string into a list.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "split string ?splitChars?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "split",
        byte_array_effect: ByteArrayEffect::Coerces,
        const_fold: Some(crate::const_fold::fold_split),
        traits: Traits::FRAMELESS_RUNTIME | Traits::PURE | Traits::CSE_CANDIDATE,
        arity: Arity::new(1, 2),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Split a string into a proper Tcl list",
            synopsis: &["split string ?splitChars?"],
            snippet: "Returns a list created by splitting string at each character that is in the splitChars argument.",
            source: "Tcl man page split.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::String),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..CommandSpec::DEFAULT
    }
}

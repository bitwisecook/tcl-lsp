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

//! `next` — call the next method in the chain.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "next ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "next",
        traits: Traits::LANGUAGE_KEYWORD.union(Traits::TCLOO_NEXT_CHAIN),
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::any(),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "invoke the next implementation of a method",
            synopsis: &["next ?arg ...?"],
            snippet: "The next command is used within the body of a method to call the next implementation of that method in the method resolution order (MRO). Arguments are passed to the next implementation.",
            source: "Tcl man page next.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

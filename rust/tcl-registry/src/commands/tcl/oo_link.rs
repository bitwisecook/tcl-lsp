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

//! `link` — expose a method as a bareword command in the object's own namespace.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "link linkName ?linkName ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "link",
        traits: Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "expose a method as a bareword command in the object's own namespace",
            synopsis: &[
                "link linkName ?linkName ...?",
                "link {linkName targetName} ?...?",
            ],
            snippet: "The link command is used within the body of a method, constructor, or destructor to create a command in the object's own private namespace that invokes a method on the current object. Each linkName is either a plain name (aliasing to the method of the same name) or a two-element list {linkName targetName} aliasing linkName to a differently-named method.",
            source: "Tcl man page link.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

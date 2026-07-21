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

//! `nextto` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "nextto",
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "invoke a specific superclass implementation of a method",
            synopsis: &["nextto class ?arg ...?"],
            snippet: "The nextto command is like next but invokes a specific class's implementation of the current method rather than the next in the MRO.",
            source: "Tcl man page next.n",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "nextto class ?arg ...?",
            dialects: None,
        }],
        // The leading `class` word is a symbolic name, structurally marking
        // this form as carrying an explicit MRO-search-start class — how
        // the analyser's `queue_next_arity_candidate` tells `nextto` apart
        // from bare `next` without matching on the command name.
        arg_roles: &[(0, ArgRole::Name)],
        traits: Traits::LANGUAGE_KEYWORD.union(Traits::TCLOO_NEXT_CHAIN),
        ..CommandSpec::DEFAULT
    }
}

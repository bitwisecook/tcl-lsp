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

//! `findclass` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "findclass",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Searches a data group list for a member that starts with the specified string and returns the data-group member string.",
            synopsis: &["findclass STRING DATA_GROUP (SEPARATOR)?"],
            snippet: "Searches a data group list for a member whose key matches the specified\nstring, and if a match is found, returns the data-group member string.\n\nNote: findclass has been deprecated in v10 in favor of the new\nclass commands. The class command offers better functionality and\nperformance than findclass\nOnly the key value of the data group list member (the portion up to the\nfirst separator character, which defaults to space unless otherwise\nspecified) is compared to the specified string to determine a match.",
            source: "https://clouddocs.f5.com/api/irules/findclass.html",
            examples: "when HTTP_REQUEST {\n  set location [findclass [HTTP::uri] URIredirects_dg \" \"]\n  if { $location ne \"\" } {\n    HTTP::redirect $location\n  }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "findclass STRING DATA_GROUP (SEPARATOR)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::DataGroup,
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        deprecated_replacement: Some("class match / class search"),
        ..CommandSpec::DEFAULT
    }
}

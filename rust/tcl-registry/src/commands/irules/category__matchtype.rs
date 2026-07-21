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

//! `CATEGORY::matchtype` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CATEGORY::matchtype",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get the type of match found.",
            synopsis: &["CATEGORY::matchtype TYPE"],
            snippet: "This iRules command is intended to be used with the CATEGORY_MATCHED event and will store the match result in the specified variable. It will return one of \"custom\", \"request_default\", or \"request_default_and_custom\". This tells the admin what kind of match was made when the CATEGORY_MATCHED event was raised – custom category match, match from the Websense categorization engine, or both. (requires SWG license)",
            source: "https://clouddocs.f5.com/api/irules/CATEGORY__matchtype.html",
            examples: "when CATEGORY_MATCHED {\n    CATEGORY::matchtype type_var\n        if { $type_var eq \"custom\" } {\n            log local0. \"Custom category match was found.\"\n        }\n}",
            return_value: "Returns one of \"custom\", \"request_default\", \"request_default_and_custom\"",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["CATEGORY"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "CATEGORY::matchtype TYPE",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ClassificationState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}

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

//! `CATEGORY::analytics` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CATEGORY::analytics",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Controls response analytics engine.",
            synopsis: &["CATEGORY::analytics BOOL_VALUE"],
            snippet: "Enables or disables the analytics server on a per request basis (requires SWG license)",
            source: "https://clouddocs.f5.com/api/irules/CATEGORY__analytics.html",
            examples: "when HTTP_REQUEST {\n    set this_uri http://[HTTP::host][HTTP::uri]\n    set reply [CATEGORY::lookup $this_uri]\n    log local0. \"uri $this_uri returns category=$reply\"\n    if { $reply equals \"Adult Material\" } {\n        CATEGORY::analytics enable\n    }\n}",
            return_value: "No return value",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["CATEGORY", "FASTHTTP"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "CATEGORY::analytics BOOL_VALUE",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ClassificationState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

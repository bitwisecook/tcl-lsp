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

//! `HTTP::status` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::status",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the response status code.",
            synopsis: &["HTTP::status"],
            snippet: "Returns the response status code as defined in RFC2616",
            source: "https://clouddocs.f5.com/api/irules/HTTP__status.html",
            examples: "when HTTP_RESPONSE {\n  if { [HTTP::status] == 404 } {\n    HTTP::redirect \"http://www.example.com/not_found.html\"\n }\n}",
            return_value: "Returns the response status code.",
        }),
        // Measured on the appliance: the rule compiler refuses
        // `HTTP::status` in `HTTP_REQUEST` with `command is not valid in
        // current event context (HTTP_REQUEST)` — there is no response
        // status while the request is still being processed
        // (`docs/design/bigip-irule-parser-measurements.md` §8, which
        // names this cell as *"exactly the mistakes an editor should
        // catch"*).
        excluded_events: &["HTTP_REQUEST"],
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            // `LB_SELECTED` implies no HTTP profile, yet the rule
            // compiler accepts `HTTP::status` there (§8).
            also_in: &["LB_SELECTED", "MR_INGRESS"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "HTTP::status",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpStatus,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

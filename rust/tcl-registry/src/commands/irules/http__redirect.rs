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

//! `HTTP::redirect` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::redirect",
        traits: Traits::DIAGRAM_ACTION,
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Redirects an HTTP request or response to the specified URL.",
            synopsis: &["HTTP::redirect REDIRECT_URL"],
            snippet: "Redirects an HTTP request or response to the specified URL. Note that\nthis command sends the response to the client immediately. Therefore,\nyou cannot specify this command multiple times in an iRule, nor can you\nspecify any other commands that modify header or content after you\nspecify this command.\nThis command will always use a 302 response code. If you wish to use a\ndifferent one (e.g. 301), you will need to craft a response using\n[HTTP::respond].\nIf the client is a typical web browser, it will reflect the new URL\nthat you specify.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__redirect.html",
            examples: "when HTTP_RESPONSE {\n  if { [HTTP::status] == 404} {\n    HTTP::redirect \"http://www.example.com/newlocation.html\"\n  }\n}",
            return_value: "",
        }),
        // Tainted redirect URL → open-redirect (IRULE3004).
        taint_output_sink: Some("IRULE3004"),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &["LB_FAILED", "NAME_RESOLVED"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "HTTP::redirect REDIRECT_URL",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ResponseCommit,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Client,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

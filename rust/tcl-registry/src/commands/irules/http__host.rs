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

//! `HTTP::host` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::host",
        traits: Traits::PURE
            .union(Traits::CSE_CANDIDATE)
            .union(Traits::DIAGRAM_ACTION),
        surface: Some(SpecSurface::IRULES),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary: "Returns the value contained in the Host header of an HTTP request.",
            synopsis: &["HTTP::host ?name?"],
            snippet: "Returns the value contained in the Host header of an HTTP request. This\ncommand replaces the BIG-IP 4.X variable http_host.\nThe Host header always contains the requested host name (which may be a\nHost Domain Name string or an IP address), and will also contain the\nrequested service port whenever a non-standard port is specified (other\nthan 80 for HTTP, other than 443 for HTTPS). When present, the\nnon-standard port is appended to the requsted name as a numeric string\nwith a colon separating the 2 values (just as it would appear in the\nbrowser's address bar):\n  * Host: host.domain.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__host.html",
            examples: "when HTTP_REQUEST {\n  if { [HTTP::uri] contains \"secure\"} {\n    HTTP::redirect \"https://[HTTP::host][HTTP::uri]\"\n }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            flow: false,
        }),
        forms: &[
            FormSpec {
                kind: FormKind::Getter,
                synopsis: "HTTP::host",
                ..FormSpec::DEFAULT
            },
            FormSpec {
                kind: FormKind::Setter,
                synopsis: "HTTP::host <name>",
                ..FormSpec::DEFAULT
            },
        ],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpHeader,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}

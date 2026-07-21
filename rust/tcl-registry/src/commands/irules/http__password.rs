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

//! `HTTP::password` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::password",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the password part of HTTP basic authentication.",
            synopsis: &["HTTP::password"],
            snippet: "Returns the password part of HTTP basic authentication.\nAs described in RFC2617 the username and password in basic\nauthentication is sent by the client in the Authorization header. The\nclient base64 encodes the username and password in the format of:\nAuthorization: Basic base64encoding(username:password)\nThe HTTP::username command parses and base64 decodes the username.\nThe HTTP::password command parses and base64 decodes the password.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__password.html",
            examples: "when CLIENT_ACCEPTED {\n  set auth_sid [AUTH::start pam default_radius]\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "HTTP::password",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpHeader,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}

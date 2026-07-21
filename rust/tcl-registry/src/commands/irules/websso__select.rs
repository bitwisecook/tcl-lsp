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

//! `WEBSSO::select` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "WEBSSO::select",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Use specified SSO configuration object to do SSO for the HTTP request.",
            synopsis: &["WEBSSO::select WEBSSO_OBJECT"],
            snippet: "This command causes APM to use specified SSO configuration object to do\nSSO for the HTTP request. Admin should make sure that the selected SSO\nmethod works for the specified request (and is enabled on backend\nserver request is going to). The scope of this iRule command is per\nHTTP request. Admin needs to execute it for each HTTP request.",
            source: "https://clouddocs.f5.com/api/irules/WEBSSO__select.html",
            examples: "when ACCESS_ACL_ALLOWED {\n    set req_uri [HTTP::uri]\n    if { $req_uri starts_with \"/owa\" } {\n        if { $req_uri eq \"/owa/auth/logon.aspx?url=https://mysite.com/owa/&reason=0\" } {\n            WEBSSO::select owa_form_base_sso\n        } elseif { $req_uri eq \"/owa/auth/logon.aspx?url=https://mysite.com/ecp/&reason=0\" } {\n            WEBSSO::select ecp_form_base_sso\n        }\n    }\n    unset req_uri\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ACCESS", "HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "WEBSSO::select WEBSSO_OBJECT",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}

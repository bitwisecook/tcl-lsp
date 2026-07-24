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

//! `LSN::port` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Explicitly set the translation address regardless of the configured LSN pool.",
            synopsis: &["LSN::port TRANSLATION_PORT"],
            snippet: "Explicitly set the translation address regardless of the configured LSN pool.\n\nThe LSN::port command can be used while processing CLIENT_DATA. This event can occur before and after address translation. If this command is used after translation has occurred an error is thrown.\n\nLSN::port <translation_port>",
            source: "https://clouddocs.f5.com/api/irules/LSN__port.html",
            examples: "",
            return_value: "LSN::port",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["LSN"],
            also_in: &[
                "AUTH_RESULT",
                "AUTH_WANTCREDENTIAL",
                "CACHE_REQUEST",
                "CACHE_UPDATE",
                "CLIENTSSL_CLIENTCERT",
                "CLIENTSSL_HANDSHAKE",
                "CLIENT_ACCEPTED",
                "CLIENT_DATA",
                "HTTP_CLASS_FAILED",
                "HTTP_CLASS_SELECTED",
                "HTTP_REQUEST",
                "HTTP_REQUEST_DATA",
                "LB_SELECTED",
                "MR_INGRESS",
                "RTSP_REQUEST",
                "RTSP_REQUEST_DATA",
                "SIP_REQUEST",
                "STREAM_MATCHED",
            ],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "LSN::port TRANSLATION_PORT",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::LsnState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}

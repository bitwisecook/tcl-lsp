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

//! `LSN::inbound` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::inbound",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disable inbound mapping for translation address and port associated with the current connection.",
            synopsis: &["LSN::inbound disable"],
            snippet: "Disable inbound mapping for translation address and port associated with the current connection.",
            source: "https://clouddocs.f5.com/api/irules/LSN__inbound.html",
            examples: "when HTTP_REQUEST {\n    LSN::inbound disable\n}",
            return_value: "LSN::inbound disable - Inbound connections can be permitted for a particular LSN pool to provide end-point independent filtering, described in RFC 4787.",
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
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "LSN::inbound disable",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::LsnState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

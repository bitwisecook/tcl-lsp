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

//! `DIAMETER::respond` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::respond",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sends message to client or server (based on context).",
            synopsis: &[
                "DIAMETER::respond DIAMETER_VERSION RFLAG_BINARY PFLAG_BINARY EFLAG_BINARY TFLAG_BINARY",
            ],
            snippet: "This iRule command creates and sends a new message to the client or\nserver.\n\nWhen called from clientside events, the new message is sent to the client.\nWhen called from serverside events, the new message is sent to the server.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__respond.html",
            examples: "when DIAMETER_INGRESS {\n    # DIAMETER::avp create  \"avpname|code\" \"v\" \"m\" \"p\" \"vendorid\" \"data\" \"type\"\n    # 2 = DO_NOT_WANT_TO_TALK_TO_YOU\n    set goaway [DIAMETER::avp create \"disconnect-cause\" 0 1 0 0 2 integer32]\n    set version 1\n    # 282 = Disconnect-Peer-Request\n    set code 282\n    set origin_host [DIAMETER::avp create \"origin-host\" 0 1 0 0 \"bigip6.core.example.com\" string]\n    set origin_realm [DIAMETER::avp create \"origin-realm\" 0 1 0 0 \"example.com\" string]\n    set appid 16777215",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER", "MR"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "DIAMETER::respond DIAMETER_VERSION RFLAG_BINARY PFLAG_BINARY EFLAG_BINARY TFLAG_BINARY",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

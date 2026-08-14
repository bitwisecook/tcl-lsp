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

//! `WS::response` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "protocol",
        arity: Arity::exact(0),
        detail: "Get Sec-WebSocket-Protocol header value.",
        synopsis: "WS::response protocol",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "extension",
        arity: Arity::exact(0),
        detail: "Get Sec-WebSocket-Extensions header value.",
        synopsis: "WS::response extension",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "version",
        arity: Arity::exact(0),
        detail: "Get Sec-WebSocket-Version header value.",
        synopsis: "WS::response version",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "key",
        arity: Arity::exact(0),
        detail: "Get Sec-WebSocket-Accept header value.",
        synopsis: "WS::response key",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "valid",
        arity: Arity::exact(0),
        detail: "Check if WebSocket upgrade was successful.",
        synopsis: "WS::response valid",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::response",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command returns the values of the various Websocket header fields seen in a server response.",
            synopsis: &["WS::response ('protocol' | 'extension' | 'version' | 'key' | 'valid' )"],
            snippet: "WS::response protocol\n    Returns the value of Sec-WebSocket-Protocol header field in server response.\n\nWS::response extension\n    Returns the value of Sec-WebSocket-Extensions header field in server response.\n\nWS::response version\n    Returns the value of Sec-WebSocket-Version header field in server response.\n\nWS::response key\n    Returns the value of Sec-WebSocket-Accept header field in server response.\n\nWS::response valid\n    Returns whether the client request and server response resulted in a successful Websocket upgrade.",
            source: "https://clouddocs.f5.com/api/irules/WS__response.html",
            examples: "when WS_RESPONSE {\n    if { [WS::response key] equals \"s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\"} {\n        WS::enabled false\n    }\n}",
            return_value: "This command can be used to lookup the values of various Websocket header fields seen in a server response.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            synopsis: "WS::response <field>",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

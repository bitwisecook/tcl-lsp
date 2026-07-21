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

//! `WS::request` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "protocol",
        arity: Arity::exact(0),
        detail: "Get Sec-WebSocket-Protocol header value.",
        synopsis: "WS::request protocol",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "extension",
        arity: Arity::exact(0),
        detail: "Get Sec-WebSocket-Extensions header value.",
        synopsis: "WS::request extension",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "version",
        arity: Arity::exact(0),
        detail: "Get Sec-WebSocket-Version header value.",
        synopsis: "WS::request version",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "key",
        arity: Arity::exact(0),
        detail: "Get Sec-WebSocket-Key header value.",
        synopsis: "WS::request key",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::request",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command returns the values of the various Websocket header fields seen in a client request.",
            synopsis: &["WS::request ('protocol' | 'extension' | 'version' | 'key' )"],
            snippet: "WS::request protocol\n    Returns the value of Sec-WebSocket-Protocol header field in client request.\n\nWS::request extension\n    Returns the value of Sec-WebSocket-Extensions header field in client request.\n\nWS::request version\n    Returns the value of Sec-WebSocket-Version header field in client request.\n\nWS::request key\n    Returns the value of Sec-WebSocket-Key header field in client request.",
            source: "https://clouddocs.f5.com/api/irules/WS__request.html",
            examples: "when WS_REQUEST {\n    if { [WS::request protocol] equals \"chat\" } {\n        WS::enabled false\n    }\n}",
            return_value: "This command can be used to lookup the values of various Websocket header fields seen in a client request.",
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
            kind: FormKind::Default,
            synopsis: "WS::request <field>",
            dialects: None,
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}

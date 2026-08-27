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

//! `TCP::notify` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::notify",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Sends a message to upper layers of iRule processing.",
            synopsis: &["TCP::notify (request | response | eom)"],
            snippet: "This command has two uses, which are unrelated to one another:\n  1. to indicate the end of a message when TCP message-based load-balancing is in effect;\n  2. to raise the USER_REQUEST or USER_RESPONSE event.\n\nThe BIG-IP LTM module supports TCP message-based load-balancing. This is enabled by applying an mblb profile to an LTM Virtual Server that also has a tcp profile applied. Note that currently (up to and including 11.4.1), the mblb profile can only be added using tmsh. There is no mechanism for adding it via the Web UI.",
            source: "https://clouddocs.f5.com/api/irules/TCP__notify.html",
            examples: "when CLIENT_ACCEPTED {\n        set message_length              0\n        set collected_length            0\n        set at_msg_start                1\n\n        TCP::collect\n}",
            return_value: "None.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &[],
            also_in: &["SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "TCP::notify (request | response | eom)",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

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

//! `SIPALG::nonregister_subscriber_listener` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIPALG::nonregister_subscriber_listener",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets or sets the value of flag enabling creating an ephemeral listener for nonregistered subscribers.",
            synopsis: &[
                "SIPALG::nonregister_subscriber_listener",
                "SIPALG::nonregister_subscriber_listener (BOOLEAN)",
            ],
            snippet: "Gets or sets the value of flag enabling creating an ephemeral listener for nonregistered subscribers.",
            source: "https://clouddocs.f5.com/api/irules/SIPALG__nonregister_subscriber_listener.html",
            examples: "when SIP_REQUEST {\n    log local0. \"nonregister_subscriber_listener is [SIPALG::nonregister_subscriber_listener]\"\n}",
            return_value: "Returns 1, or 0",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SIP"],
            also_in: &["CLIENT_ACCEPTED", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SIPALG::nonregister_subscriber_listener",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}

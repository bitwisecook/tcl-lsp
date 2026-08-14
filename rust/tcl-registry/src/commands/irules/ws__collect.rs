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

//! `WS::collect` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::collect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command can be used to collect payload of current Websocket frame.",
            synopsis: &["WS::collect ('frame' (LENGTH)? )"],
            snippet: "WS::collect frame\nCollects the entire Websocket frame payload.\n\nNote that if multiple iRules invoke WS::collect simultaneously,\n(perhaps by being called by the same event in multiple iRule scripts)\nthen the result is undefined.  This is because the amount of payload\ncollected for the WS_CLIENT_DATA or WS_SERVER_DATA event cannot\nsatisfy the perhaps differing amounts wanted by the callers. iRules\nshould arbitrate amoungst themselves to prevent this situation from\noccuring, and have only one WS::collect call outstanding at a time.",
            source: "https://clouddocs.f5.com/api/irules/WS__collect.html",
            examples: "when WS_CLIENT_FRAME {\n    WS::collect frame\n}",
            return_value: "",
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
            synopsis: "WS::collect ('frame' (LENGTH)? )",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        data_collection: Some(WS_COLLECT),
        ..CommandSpec::DEFAULT
    }
}

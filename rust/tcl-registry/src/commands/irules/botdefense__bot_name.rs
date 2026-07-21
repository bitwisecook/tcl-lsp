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

//! `BOTDEFENSE::bot_name` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::bot_name",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the name assigned to the detected bot, browser or mobile application.",
            synopsis: &["BOTDEFENSE::bot_name"],
            snippet: "Returns the name assigned to the detected bot, browser or mobile application. The name is derived from the detected signature if detected, or the User-Agent string in combination with the detected anomalies.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__bot_name.html",
            examples: "# EXAMPLE: Log the Bot name and Device ID of the client, upon each request, if it is known.\nwhen BOTDEFENSE_ACTION {\n    log local0.info \"Bot [BOTDEFENSE::bot_name] with Device ID [ BOTDEFENSE::device_id] from IP [ IP::client_addr ] visited [HTTP::uri ]\"\n}",
            return_value: "The name assigned to the bot, browser or mobile application that sent the request.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["BOTDEFENSE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "BOTDEFENSE::bot_name",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}

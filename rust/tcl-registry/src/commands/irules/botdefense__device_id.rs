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

//! `BOTDEFENSE::device_id` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::device_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the Device ID of the client, as retrieved from the request.",
            synopsis: &["BOTDEFENSE::device_id"],
            snippet: "Returns a number, representing the Device ID of the client, as retrieved from the request. If the Device ID is unknown, 0 is returned. By default, the Device ID is collected from the client, if it is enabled in the configuration. However, this can be overridden using the BOTDEFENSE::cs_attribute command.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__device_id.html",
            examples: "# EXAMPLE: Log the Device ID of the client, upon each request, if it is known.\nwhen BOTDEFENSE_REQUEST {\n    if {[BOTDEFENSE::device_id] != 0} {\n        set log \"botdefense device_id of client IP [IP::client_addr] is\"\n        append log \" [BOTDEFENSE::device_id]\"\n        HSL::send $hsl $log\n    }\n}",
            return_value: "The number representing the device ID of the client that sent the request, or 0 if there is no such value",
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
            synopsis: "BOTDEFENSE::device_id",
            dialects: None,
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

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

//! `RADIUS::code` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "RADIUS::code",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command returns the RADIUS message code",
            synopsis: &["RADIUS::code"],
            snippet: "This command returns the RADIUS message code",
            source: "https://clouddocs.f5.com/api/irules/RADIUS__code.html",
            examples: "when CLIENT_ACCEPTED {\n    if { [RADIUS::code] == 4 } {\n        set rd 0\n        # Extract the APN information from the AVP\n        set called_station_id [RADIUS::avp 30 \"string\"]\n        if {$called_station_id == \"station1\"} {\n            set rd 1\n        } elseif {$called_station_id == \"station2\"} {\n            set rd 2\n        }\n        # Overwrite the default route domain value with the new value.\n        RADIUS::rtdom $rd\n    }\n}",
            return_value: "returns radius message code.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[
                "CLIENT_ACCEPTED",
                "CLIENT_CLOSED",
                "CLIENT_DATA",
                "SERVER_CLOSED",
                "SERVER_CONNECTED",
                "SERVER_DATA",
            ],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "RADIUS::code",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

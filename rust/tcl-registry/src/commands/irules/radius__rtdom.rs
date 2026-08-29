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

//! `RADIUS::rtdom` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "RADIUS::rtdom",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command overwrites the default route-domain ID in RADIUS scenario with given value",
            synopsis: &["RADIUS::rtdom (ROUTE_DOMAIN)?"],
            snippet: "This command overwrites the default route-domain ID in RADIUS scenario with given value",
            source: "https://clouddocs.f5.com/api/irules/RADIUS__rtdom.html",
            examples: "when CLIENT_ACCEPTED {\n        if { [RADIUS::code] == 4 } {\n            set rd 0\n            # Extract the APN information from the AVP\n            set called_station_id [RADIUS::avp 30 \"string\"]\n            if {$called_station_id == \"station1\"} {\n                set rd 1\n            } elseif {$called_station_id == \"station2\"} {\n                set rd 2\n            }\n            # Overwrite the default route domain value with the new value.\n            RADIUS::rtdom $rd\n        }\n    }",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "RADIUS::rtdom (ROUTE_DOMAIN)?",
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

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

//! `L7CHECK::protocol` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "L7CHECK::protocol",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Set or get L7 protocol value.",
            synopsis: &["L7CHECK::protocol set VALUE", "L7CHECK::protocol get"],
            snippet: "The L7CHECK::protocol commands allow you to set or retrieve L7 protocol value.",
            source: "https://clouddocs.f5.com/api/irules/L7CHECK__protocol.html",
            examples: "when L7CHECK_CLIENT_DATA {\n    if { [L7CHECK::protocol get] == \"https\" } {\n        pool clients_https\n    } else {\n        pool clients_non_https\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["CONNECTOR", "L7CHECK"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "L7CHECK::protocol set VALUE",
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

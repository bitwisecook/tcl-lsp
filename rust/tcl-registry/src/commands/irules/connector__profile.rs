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

//! `CONNECTOR::profile` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CONNECTOR::profile",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get connector profile name.",
            synopsis: &["CONNECTOR::profile"],
            snippet: "CONNECTOR::profile\n    Get the connector profile name in the current context.",
            source: "https://clouddocs.f5.com/api/irules/CONNECTOR__profile.html",
            examples: "when CONNECTOR_OPEN {\n                if {([CONNECTOR::profile] eq \"/Common/connector_profile_1\")} {\n                    log local0. \"CONNECTOR_OPEN raised by connector_profile_1\"\n                }\n            }",
            return_value: "CONNECTOR::profile Return the connector profile name.",
        }),
        forms: &[FormSpec {
            synopsis: "CONNECTOR::profile",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

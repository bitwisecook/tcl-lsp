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

//! `FLOWTABLE::limit` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOWTABLE::limit",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns configured connection limits.",
            synopsis: &[
                "FLOWTABLE::limit virtual (VIRTUAL_SERVER_OBJ)?",
                "FLOWTABLE::limit route_domain (ROUTE_DOMAIN_NAME)?",
            ],
            snippet: "This iRules command returns configured connection limits\nNote: When virtual server or route domain name is omitted the commands\nuse virtual or route domain of the current connection. Specifying the\nname incurs significant performance hit.",
            source: "https://clouddocs.f5.com/api/irules/FLOWTABLE__limit.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "FLOWTABLE::limit virtual (VIRTUAL_SERVER_OBJ)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::FlowState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

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

//! `ACCESS::flowid` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::flowid",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the flow id for SSL Orchestrator using APM logging framework.",
            synopsis: &["ACCESS::flowid (FID)?"],
            snippet: "ACCESS::flowid [FID]\n\nCalculates the flow id from the IFC and 4-tuple information, if it doesn't\nexist already, and stores it in the opaque storage for the connflow.\nRequires APM to be provisioned.\n\nCommand Syntax\n\nACCESS::flowid\n\n    * Returns the flow id, if it exists, or calculates it, then stores it in\n      the opaque data structure for the connflow.\n\nACCESS::flowid <FID>\n\n    * Sets the flow id to FID",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__flowid.html",
            examples: "when HTTP_REQUEST {\n    ACCESS::flowid \"example\"\n    set ctx(FID) [ACCESS::flowid]\n}",
            return_value: "The flow id is returned",
        }),
        forms: &[FormSpec {
            synopsis: "ACCESS::flowid (FID)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

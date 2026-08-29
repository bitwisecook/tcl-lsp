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

//! `BOTDEFENSE::cs_attribute` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::cs_attribute",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Queries for or sets attributes for the client-side challenge.",
            synopsis: &["BOTDEFENSE::cs_attribute 'device_id' (BOOLEAN)?"],
            snippet: "Queries for or sets attributes for the client-side challenge. These attributes are only effective if a client-side action is taken on the current request.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__cs_attribute.html",
            examples: "# EXAMPLE: Make sure that the data for the device_id is always collected when taking a client-side action.\nwhen BOTDEFENSE_REQUEST {\n    BOTDEFENSE::cs_attribute device_id enable\n}",
            return_value: "* When called with an argument the command overrides the decision of Bot Defense whether to collect device id. * When called without an argument, the command returns whether Bot Defense attempts to collect the device id during the request (initiate response).",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["BOTDEFENSE"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "BOTDEFENSE::cs_attribute 'device_id' (BOOLEAN)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            connection_side: ConnectionSide::Client,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

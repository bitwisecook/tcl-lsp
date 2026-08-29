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

//! `BOTDEFENSE::enable` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::enable",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enables processing by Bot Defense on the connection.",
            synopsis: &["BOTDEFENSE::enable"],
            snippet: "Enables processing and blocking of the request by Bot Defense, for the duration of the current TCP connection, or until BOTDEFENSE::disable is called.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__enable.html",
            examples: "# EXAMPLE: Re-enable Bot Defense on the connection if a request arrives with a certain URL prefix.\nwhen HTTP_REQUEST {\n    if {[HTTP::uri] starts_with \"/t/\"} {\n        BOTDEFENSE::enable\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "BOTDEFENSE::enable",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            writes: true,
            connection_side: ConnectionSide::Client,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

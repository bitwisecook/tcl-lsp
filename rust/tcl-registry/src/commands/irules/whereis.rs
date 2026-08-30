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

//! `whereis` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "whereis",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns geographical information on an IP address.",
            synopsis: &["whereis (ldns | IP_ADDR)"],
            snippet: "Returns the geographic location of a specific IP address.\nFor more information on using whereis in LTM, you can check Jason\nRahm's article\n\nLegal usage notes\n\n   The data is purchased by F5 for use on BIG-IP systems and products for\n   traffic management. The key to understanding EULA compliance is to\n   figure out where the geolocation decision is being made.",
            source: "https://clouddocs.f5.com/api/irules/whereis.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "whereis (ldns | IP_ADDR)",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::BigipConfig,
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

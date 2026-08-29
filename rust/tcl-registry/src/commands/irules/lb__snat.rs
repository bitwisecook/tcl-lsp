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

//! `LB::snat` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::snat",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns information on the SNAT configuration for the current connection.",
            synopsis: &["LB::snat"],
            snippet: "This command returns information on the SNAT configuration for the current connection.\n\nPossible output values are those which can be set by the snat and snatpool commands.",
            source: "https://clouddocs.f5.com/api/irules/LB__snat.html",
            examples: "when CLIENT_ACCEPTED {\n    # Check if SNAT is enabled on the VIP\n    if {[LB::snat] eq \"none\"}{\n        log local0. \"Snat disabled on [virtual name]\"\n    } else {\n        log local0. \"Snat enabled on [virtual name].  Currently set to [LB::snat]\"\n    }\n}",
            return_value: "LB::snat",
        }),
        forms: &[FormSpec {
            synopsis: "LB::snat",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SnatSelection,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Server,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

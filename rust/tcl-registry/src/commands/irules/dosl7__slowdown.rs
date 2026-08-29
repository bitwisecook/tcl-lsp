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

//! `DOSL7::slowdown` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DOSL7::slowdown",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Adds source IP extracted from current connection to greylist.",
            synopsis: &["DOSL7::slowdown RATE TIMEOUT"],
            snippet: "Adds source IP extracted from current connection to greylist. TCP slowdown will be applied according to supplied RATE (in percents) and TIMEOUT (in seconds).\nA RATE represents amount of incoming data packets to be dropped to perform slowdown.",
            source: "https://clouddocs.f5.com/api/irules/DOSL7__slowdown.html",
            examples: "when HTTP_REQUEST {\n                 if { [HTTP::uri] contains \"heavy.php\" } {\n                     DOSL7::slowdown 30 60\n                 }\n             }",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "DOSL7::slowdown RATE TIMEOUT",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Dosl7State,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

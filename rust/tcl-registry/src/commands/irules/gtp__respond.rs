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

//! `GTP::respond` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::respond",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sends the GTP message back to the remote node of this connection.",
            synopsis: &["GTP::respond MESSAGE"],
            snippet: "Sends this GTP message back to the remote node of this connection.\nIf this is clientside flow, send it back to client that initiated the connection.\nIf this is serverside flow, send it to the remote node that is connected to.",
            source: "https://clouddocs.f5.com/api/irules/GTP__respond.html",
            examples: "when GTP_SIGNALLING_EGRESS {\n    set t2 [GTP::new 2 10]\n    GTP::respond $t2\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "GTP::respond MESSAGE",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

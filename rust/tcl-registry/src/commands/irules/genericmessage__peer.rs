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

//! `GENERICMESSAGE::peer` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "GENERICMESSAGE::peer",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets the peer's route name.",
            synopsis: &["GENERICMESSAGE::peer name (NAME)?"],
            snippet: "The GENERICMESSAGE::peer command returns or sets the peer's route name\nin the message routing framework. The peer name will be automatically\nset as the source address of each message.",
            source: "https://clouddocs.f5.com/api/irules/GENERICMESSAGE__peer.html",
            examples: "when CLIENT_ACCEPTED {\n    GENERICMESSAGE::peer name \"[IP::remote_addr]:[TCP::remote_port]\"\n}",
            return_value: "Returns the peer's route name.",
        }),
        forms: &[FormSpec {
            synopsis: "GENERICMESSAGE::peer name (NAME)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::MessageState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

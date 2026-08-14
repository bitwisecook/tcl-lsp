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

//! `MR::connection_mode` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::connection_mode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the connection mode.",
            synopsis: &["MR::connection_mode"],
            snippet: "returns the connection mode of the current connection and the number of\nas configured in the peer object used to create the connection. Valid\nconnection modes as \"per-peer\", \"per-blade\", \"per-tmm\" or \"per-client\".\nFor incoming connections, it will return \"per-peer\".",
            source: "https://clouddocs.f5.com/api/irules/MR__connection_mode.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"[MR::connection_instance] [MR::connection_mode]\"\n}",
            return_value: "returns the connection mode",
        }),
        forms: &[FormSpec {
            synopsis: "MR::connection_mode",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::MessageState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

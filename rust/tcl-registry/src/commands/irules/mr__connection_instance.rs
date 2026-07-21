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

//! `MR::connection_instance` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::connection_instance",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the connection instance and the number of connections.",
            synopsis: &["MR::connection_instance"],
            snippet: "returns the connection instance number of the current connection and the number of\nconnections as configured in the peer object used to create the connection.\nThe return will be formated as \"<instance> of <num_connections>\".\nFor incoming connections, it will return \"0 of 1\".",
            source: "https://clouddocs.f5.com/api/irules/MR__connection_instance.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"[MR::connection_instance] [MR::connection_mode]\"\n}",
            return_value: "returns the connection instance number and the number of connections formatted as \"<instance> of <num_connections>\".",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "MR::connection_instance",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::MessageState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}

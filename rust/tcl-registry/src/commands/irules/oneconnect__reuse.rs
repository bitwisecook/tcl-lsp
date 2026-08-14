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

//! `ONECONNECT::reuse` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ONECONNECT::reuse",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Controls server-side connection reuse.",
            synopsis: &["ONECONNECT::reuse (BOOL_VALUE)?"],
            snippet: "This command controls whether server-side connections are picked from\nthe pool of idle connections, and whether idle server-side connections\nare returned to the pool or closed when a client connection detaches or\ncloses. It will also display the current status of connection reuse, if\ncalled without any options.\nFor information on how to control the detaching behavior, see\nONECONNECT::detach.\nThe semantics of this command depend on the context in which it is\nbeing executed. Refer to Considering Context Part 1 and\nConsidering Context Part 2 for more information on contexts.",
            source: "https://clouddocs.f5.com/api/irules/ONECONNECT-reuse.html",
            examples: "when HTTP_REQUEST {\nif {[HTTP::method] equals GET } {\n      ONECONNECT::reuse enable\n   } else {\n      ONECONNECT::reuse disable\n   }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "ONECONNECT::reuse (BOOL_VALUE)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Server,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

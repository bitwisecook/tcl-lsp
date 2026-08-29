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

//! `NSH::md1` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "NSH::md1",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets/Get the MD1 context for NSH.",
            synopsis: &["NSH::md1 DIRECTION UNSIGNED_INT UNSIGNED_INT (METADATA)?"],
            snippet: "Set: MD1 context for NSH. Offset, length and data string as arguments.\n            Get: MD1 context from NSH. Only offset and length as arguments.",
            source: "https://clouddocs.f5.com/api/irules/NSH__md1.html",
            examples: "ntext for NSH.\n            when CLIENT_ACCEPTED {\n                set str {1234567890123456}\n                NSH::md1 serverside_egress 1 16 [binary format a* $str]\n                set myctx1 [NSH::md1 serverside_egress 1 16]\n            }",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "NSH::md1 DIRECTION UNSIGNED_INT UNSIGNED_INT (METADATA)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

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

//! `GTP::parse` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::parse",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Creates a new GTP message from a byte stream.",
            synopsis: &["GTP::parse BYTE_STREAM"],
            snippet: "Creates a new GTP message from a byte stream.\nReturns a TCL object of type \"GTP-Message\"",
            source: "https://clouddocs.f5.com/api/irules/GTP__parse.html",
            examples: "when CLIENT_ACCEPTED {\n    set payload [UDP::payload]\n    set t2 [GTP::parse $payload]\n    log local0. \"GTP type [GTP::header type -message $t2]\"\n    log local0. \"GTP teid [GTP::header teid -message $t2]\"\n}",
            return_value: "Returns a TCL object of type \"GTP-Message\"",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "GTP::parse BYTE_STREAM",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}

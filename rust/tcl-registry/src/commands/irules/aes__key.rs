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

//! `AES::key` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AES::key",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Creates an AES key to encrypt/decrypt data.",
            synopsis: &["AES::key ('128' | '192' | '256')?"],
            snippet: "Creates an AES key of the specified length for use in\nencryption/decryption operations.",
            source: "https://clouddocs.f5.com/api/irules/AES__key.html",
            examples: "when RULE_INIT {\n    set ::key [AES::key 128]\n}",
            return_value: "Returns the created key.",
        }),
        forms: &[FormSpec {
            synopsis: "AES::key ('128' | '192' | '256')?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

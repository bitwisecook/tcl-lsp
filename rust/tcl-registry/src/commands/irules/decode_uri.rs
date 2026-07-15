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

//! `decode_uri` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "decode_uri",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Decodes the specified string using HTTP URI encoding.",
            synopsis: &["decode_uri ANY_CHARS"],
            snippet: "Decodes the specified string using HTTP URI encoding per RFC2616 and\nreturns the result. This is a BIG-IP 4.x variable, provided for\nbackward-compatibiliy. You can use the equivalent 9.X commmand\nURI::decode instead.",
            source: "https://clouddocs.f5.com/api/irules/decode_uri.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "decode_uri ANY_CHARS",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
        }],
        deprecated_replacement: Some("URI::decode"),
        deprecated_replacement_drop_in: true,
        traits: Traits::IS_UNESCAPE,
        ..CommandSpec::DEFAULT
    }
}

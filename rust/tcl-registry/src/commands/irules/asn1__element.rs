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

//! `ASN1::element` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASN1::element",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns ASN1.1 record elements.",
            synopsis: &[
                "ASN1::element init ('BER' | 'DER')",
                "ASN1::element next ELEMENT (NUM_ELEMENTS)?",
                "ASN1::element byte_offset ELEMENT (OFFSET)?",
                "ASN1::element tag ELEMENT",
            ],
            snippet: "This command returns ASN1.1 record elements.\n\nASN1::element init encodingType\n\n     * Returns an element (Tcl_Obj) handle used by the remaining commands.\n       encodingType specifies the encoding type that subsequent commands\n       should use (BER|DER).\n\nASN1::element next element ?numberOfElements?\n\n     * Returns the next element found after element. If numberOfElements\n       is specified, the command will move ahead that many elements,\n       otherwise, the default is 1.\n\nASN1::element byte_offset element ?offset?\n\n     * Returns the byte offset within the payload.",
            source: "https://clouddocs.f5.com/api/irules/ASN1__element.html",
            examples: "when CLIENT_ACCEPTED {\n  TCP::collect\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ASN1::element init ('BER' | 'DER')",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}

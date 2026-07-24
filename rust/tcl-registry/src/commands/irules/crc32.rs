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

//! `crc32` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "crc32",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the crc32 checksum for the specified string.",
            synopsis: &["crc32 ANY_CHARS"],
            snippet: "The crc32 command calculates a 32-bit cyclic redundancy check value\n(CRC) for the bytes in a string using the well-known CRC-32 (Ethernet\nCRC) scheme. The polynomial is 0x04c11db7, the CRC register is\ninitialized with 0xffffffff, the input bytes are taken msb-first, and\nthe result is the complement of the final register value reflected.\n(crc32 implements the scheme called \"CRC-32\" in this Catalogue of\nParametrised CRC Algorithms.)\ncrc32 returns a number, or the empty string if an error occurs.",
            source: "https://clouddocs.f5.com/api/irules/crc32.html",
            examples: "when HTTP_REQUEST {\n   # Create a hash value for the host based on crc32\n   # This could also be based on md5 or any other implementation\n   # of a hash like djb or something.\nset key [crc32 [HTTP::host]]\n\n   # Modulo the hash value by 1 - odd goes to one member, even another\nset key [expr {$key & 1}]\n\n   # Route the request to the pool member based on the modulus\n   # of the hash value.\nswitch $key {\n0 { pool my_pool member 1.2.3.4:80 }\n1 { pool my_pool member 5.6.7.8:80 }\n   }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "crc32 ANY_CHARS",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}

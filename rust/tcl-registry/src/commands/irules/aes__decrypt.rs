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

//! `AES::decrypt` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AES::decrypt",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Decrypts the data using the previously-created AES key.",
            synopsis: &["AES::decrypt KEY DATA"],
            snippet: "Decrypt the data using an AES key.",
            source: "https://clouddocs.f5.com/api/irules/AES__decrypt.html",
            examples: "when HTTP_REQUEST {\n  set key \"AES 128 43047ad71173be644498b98de6a32fe3\"\n  set decryptedData [AES::decrypt $key $encryptedData]\n  log local0. \"The decrypted data is $decryptedData\"\n}",
            return_value: "Returns the decrypted data.",
        }),
        forms: &[FormSpec {
            synopsis: "AES::decrypt KEY DATA",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

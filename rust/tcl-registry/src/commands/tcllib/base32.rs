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

//! tcllib `base32` / `base32::hex` packages — base32 (RFC 4648) codecs.
//!
//! Public command surface from tcllib `modules/base32/{base32,base32hex}.man`.
//! Both codecs are pure string transforms.  Requires Tcl 8.5+.

use crate::prelude::*;

/// A pure base32 codec command.
fn codec(
    name: &'static str,
    pkg: &'static str,
    synopsis: &'static [&'static str],
    summary: &'static str,
    return_value: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        traits: Traits::PURE,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary,
            synopsis,
            snippet: "",
            source: "tcllib base32 package",
            examples: "",
            return_value,
        }),
        tcllib_package: Some(pkg),
        required_package: Some(pkg),
        ..CommandSpec::DEFAULT
    }
}

/// All `base32` / `base32::hex` command specs.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        codec(
            "base32::encode",
            "base32",
            &["base32::encode string"],
            "Encode a string using standard base32 (RFC 4648).",
            "The base32-encoded representation of the input.",
        ),
        codec(
            "base32::decode",
            "base32",
            &["base32::decode estring"],
            "Decode a standard base32 (RFC 4648) string.",
            "The decoded binary string.",
        ),
        codec(
            "base32::hex::encode",
            "base32::hex",
            &["base32::hex::encode string"],
            "Encode a string using extended-hex base32 (RFC 4648).",
            "The extended-hex base32-encoded representation of the input.",
        ),
        codec(
            "base32::hex::decode",
            "base32::hex",
            &["base32::hex::decode estring"],
            "Decode an extended-hex base32 (RFC 4648) string.",
            "The decoded binary string.",
        ),
    ]
}

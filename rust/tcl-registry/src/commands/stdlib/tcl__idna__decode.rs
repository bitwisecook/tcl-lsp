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

//! `tcl::idna::decode` command (`cookiejar` package, bundled since Tcl 8.6).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::idna::decode",
        traits: Traits::PURE,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Decode a hostname from IDNA format to Unicode.",
            synopsis: &["tcl::idna::decode hostname"],
            snippet: "",
            source: "Tcl stdlib cookiejar package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("cookiejar"),
        ..CommandSpec::DEFAULT
    }
}

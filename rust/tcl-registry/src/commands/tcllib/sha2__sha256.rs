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

//! `sha2::sha256` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "sha2::sha256 ?options? ?--? string",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "sha2::sha256",
        traits: Traits::PURE,
        surface: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Compute the SHA-256 hash of a string or file.",
            synopsis: &[
                "sha2::sha256 ?-hex|-bin? ?-channel channel | -file filename | ?--? string?",
            ],
            snippet: "",
            source: "tcllib sha2 package",
            examples: "",
            return_value: "The SHA-256 hash as a hex or binary string.",
        }),
        forms: FORMS,
        // The commands are `::sha2::*`, but tcllib provides them under
        // `package require sha256` (`sha1/sha256.tcl` ends with
        // `package provide sha256 1.0.6`); no `sha2` package exists.
        tcllib_package: Some("sha256"),
        required_package: Some("sha256"),
        ..CommandSpec::DEFAULT
    }
}

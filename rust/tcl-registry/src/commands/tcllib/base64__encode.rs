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

//! `base64::encode` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "base64::encode ?-maxlen maxlen? ?-wrapchar wrapchar? data",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "base64::encode",
        dialects: None,
        arity: Arity::new(1, 5),
        hover: Some(HoverSnippet {
            summary: "Encode binary data as a base64 string.",
            synopsis: &["base64::encode ?-maxlen maxlen? ?-wrapchar wrapchar? data"],
            snippet: "",
            source: "tcllib base64 package",
            examples: "set encoded [base64::encode $binaryData]",
            return_value: "A base64-encoded string.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("base64"),
        required_package: Some("base64"),
        ..CommandSpec::DEFAULT
    }
}

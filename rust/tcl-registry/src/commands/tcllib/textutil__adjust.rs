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

//! `textutil::adjust` — the flattened, umbrella-only alias.
//!
//! This is **not** the `textutil::adjust` *package*'s own command (that is
//! `::textutil::adjust::adjust`, registered as `"textutil::adjust::adjust"`
//! in `misc_ext.rs`'s `TEXTUTIL__ADJUST_CMDS`, gated on `required_package:
//! "textutil::adjust"`). It is the bare `::textutil::adjust` re-export the
//! `textutil` *umbrella* package creates via `namespace import -force
//! adjust::adjust ...` (tcllib-2.0 `modules/textutil/textutil.tcl`) — real
//! and callable, but only after `package require textutil`, never after
//! `package require textutil::adjust` alone (issue #923 idx 3/4; confirmed
//! against tclsh 9.0.4 + real tcllib-2.0: `package require textutil::adjust`
//! creates no bare `::textutil::adjust` command, only the three-segment
//! `::textutil::adjust::adjust`).
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "textutil::adjust string ?options?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::adjust",
        surface: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Adjust a text block to a given line length. Re-exported alias for `textutil::adjust::adjust`, provided by the `textutil` umbrella package (not by `package require textutil::adjust` alone).",
            synopsis: &[
                "textutil::adjust string ?-length num? ?-justify left|right|center|plain? ?-hyphenate bool? ?-full bool? ?-strictlength bool?",
            ],
            snippet: "",
            source: "tcllib textutil package",
            examples: "set wrapped [textutil::adjust $text -length 72]",
            return_value: "The adjusted text.",
        }),
        forms: FORMS,
        tcllib_package: Some("textutil"),
        required_package: Some("textutil"),
        ..CommandSpec::DEFAULT
    }
}

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

//! `textutil::indent` — the flattened, umbrella-only alias.
//!
//! There is no `textutil::indent` *package* — `indent` lives inside the
//! `textutil::adjust` package (`::textutil::adjust::indent`, registered as
//! `"textutil::adjust::indent"` in `misc_ext.rs`'s `TEXTUTIL__ADJUST_CMDS`,
//! gated on `required_package: "textutil::adjust"`). This entry is the bare
//! `::textutil::indent` re-export the `textutil` *umbrella* package creates
//! via `namespace import -force adjust::indent` (tcllib-2.0
//! `modules/textutil/textutil.tcl`) — real and callable, but only after
//! `package require textutil`, never after `package require
//! textutil::adjust` alone (issue #923 idx 3/4; confirmed against tclsh
//! 9.0.4 + real tcllib-2.0).
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "textutil::indent text prefix ?skip?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::indent",
        dialects: None,
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet {
            summary: "Indent each line of text by a given prefix. Re-exported alias for `textutil::adjust::indent`, provided by the `textutil` umbrella package (not by `package require textutil::adjust` alone).",
            synopsis: &["textutil::indent text prefix ?skip?"],
            snippet: "",
            source: "tcllib textutil package",
            examples: "set indented [textutil::indent $text \"    \"]",
            return_value: "The indented text.",
        }),
        forms: FORMS,
        tcllib_package: Some("textutil"),
        required_package: Some("textutil"),
        ..CommandSpec::DEFAULT
    }
}

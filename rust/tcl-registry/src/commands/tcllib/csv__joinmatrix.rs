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

//! `csv::joinmatrix` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "csv::joinmatrix matrix ?sepChar? ?quoteChar? ?quoteStyle?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::joinmatrix",
        dialects: None,
        arity: Arity::new(1, 4),
        hover: Some(HoverSnippet {
            summary: "Join a matrix object into CSV-formatted lines.",
            synopsis: &["csv::joinmatrix matrix ?sepChar? ?quoteChar? ?quoteStyle?"],
            snippet: "",
            source: "tcllib csv package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("csv"),
        required_package: Some("csv"),
        ..CommandSpec::DEFAULT
    }
}

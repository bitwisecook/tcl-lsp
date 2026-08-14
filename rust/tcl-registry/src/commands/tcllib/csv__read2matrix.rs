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

//! `csv::read2matrix` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "csv::read2matrix ?-alternate? chan m ?sepChar? ?expand?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::read2matrix",
        dialects: None,
        arity: Arity::new(2, 5),
        hover: Some(HoverSnippet {
            summary: "Read CSV data from a channel into a matrix object.",
            synopsis: &["csv::read2matrix ?-alternate? chan m ?sepChar? ?expand?"],
            snippet: "",
            source: "tcllib csv package",
            examples: "",
            return_value: "The number of lines read.",
        }),
        forms: FORMS,
        tcllib_package: Some("csv"),
        required_package: Some("csv"),
        ..CommandSpec::DEFAULT
    }
}

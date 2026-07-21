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

//! `report::styles` — list the defined report styles.
//
// VERIFIED: tcllib report(n).  `::report::styles` returns the names of all
// styles known to the package.  `package require Tcl 8.5 9`.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report::styles",
    dialects: None,
}];

/// Command spec for `report::styles`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report::styles",
        arity: Arity::exact(0),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Returns a list of the names of all styles known to the package.",
            synopsis: &["report::styles"],
            snippet: "The list reflects the styles defined via `report::defstyle` at the time \
                      of the call.",
            source: "tcllib report package",
            examples: "",
            return_value: "A list of defined style names.",
        }),
        forms: FORMS,
        tcllib_package: Some("report"),
        required_package: Some("report"),
        ..CommandSpec::DEFAULT
    }
}

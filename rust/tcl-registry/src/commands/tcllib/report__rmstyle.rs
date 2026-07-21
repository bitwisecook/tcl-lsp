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

//! `report::rmstyle` — delete a report style.
//
// VERIFIED: tcllib report(n).  `::report::rmstyle styleName` deletes the named
// style.  `package require Tcl 8.5 9`, so gated out of tcl8.4.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report::rmstyle styleName",
    dialects: None,
}];

/// Command spec for `report::rmstyle`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report::rmstyle",
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Name)],
        hover: Some(HoverSnippet {
            summary: "Deletes the style styleName.",
            synopsis: &["report::rmstyle styleName"],
            snippet: "The style must have been defined by `report::defstyle`; removing it does \
                      not affect reports already created with it.",
            source: "tcllib report package",
            examples: "",
            return_value: "The empty string.",
        }),
        forms: FORMS,
        tcllib_package: Some("report"),
        required_package: Some("report"),
        ..CommandSpec::DEFAULT
    }
}

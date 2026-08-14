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

//! `report::stylearguments` — introspect a style's formal arguments.
//
// VERIFIED: tcllib report(n).  `::report::stylearguments styleName` returns the
// list of arguments associated with the style.  `package require Tcl 8.5 9`.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "report::stylearguments styleName",
    ..FormSpec::DEFAULT
}];

/// Command spec for `report::stylearguments`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report::stylearguments",
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Name)],
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Returns the list of arguments associated with the style styleName.",
            synopsis: &["report::stylearguments styleName"],
            snippet: "The result is the formal parameter list the style was defined with via \
                      `report::defstyle styleName arguments script`.",
            source: "tcllib report package",
            examples: "",
            return_value: "The style's formal argument list.",
        }),
        forms: FORMS,
        tcllib_package: Some("report"),
        required_package: Some("report"),
        ..CommandSpec::DEFAULT
    }
}

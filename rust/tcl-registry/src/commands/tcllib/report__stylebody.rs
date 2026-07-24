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

//! `report::stylebody` — introspect a style's definition script.
//
// VERIFIED: tcllib report(n).  `::report::stylebody styleName` returns the
// script associated with the style.  `package require Tcl 8.5 9`.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report::stylebody styleName",
    dialects: None,
}];

/// Command spec for `report::stylebody`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report::stylebody",
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Name)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Returns the script associated with the style styleName.",
            synopsis: &["report::stylebody styleName"],
            snippet: "The result is the body script the style was defined with via \
                      `report::defstyle styleName arguments script`.",
            source: "tcllib report package",
            examples: "",
            return_value: "The style's definition script.",
        }),
        forms: FORMS,
        tcllib_package: Some("report"),
        required_package: Some("report"),
        ..CommandSpec::DEFAULT
    }
}

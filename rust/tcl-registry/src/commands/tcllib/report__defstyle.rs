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

//! `report::defstyle` — define a report style.
//
// VERIFIED: tcllib report(n). `::report::defstyle styleName arguments
// script` takes exactly three arguments. `arguments` is a proc-style
// formal parameter list; its parameters are "available in the script as
// variables". `script` is a Tcl body evaluated in a definition context
// (a safe interpreter exposing report configuration methods and every
// previously-defined style as a command) — not the caller's frame.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "report::defstyle styleName arguments script",
    ..FormSpec::DEFAULT
}];

/// Command spec for `report::defstyle`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report::defstyle",
        arity: Arity::exact(3),
        // styleName names the style (which becomes a command usable in
        // later style scripts); `arguments` is a proc-style parameter
        // list; `script` is a Tcl body to recurse into.
        arg_roles: &[
            (0, ArgRole::Name),
            (1, ArgRole::ParamList),
            (2, ArgRole::Body),
        ],
        // The style script runs in the report package's definition
        // context (a safe interpreter), not the caller's frame — so it
        // must opt out of the enclosing block's data flow, exactly like
        // `proc` and `snit::method` bodies.
        body_kind: BodyKind::Structural,
        // `DEFERS_BODY`: the style script is stored against `styleName` and
        // runs when a report applies that style, not at the definition.
        // tclsh 8.6.16 / 9.0.4, byte-identical: `proc p {} {
        // report::defstyle s {} {error stop}; set ::reached 1 }` sets
        // `::reached` (issue #1672 audit).
        traits: Traits::DEFERS_BODY,
        hover: Some(HoverSnippet {
            summary: "Defines the new style styleName.",
            synopsis: &["report::defstyle styleName arguments script"],
            snippet: "`arguments` is a formal parameter list bound as variables in `script`; \
                      `script` configures the report using style and configuration commands.",
            source: "tcllib report package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("report"),
        required_package: Some("report"),
        // The style script runs in a safe interpreter that aliases the report
        // configuration methods (`top`, `data`, `columns`, …) as commands, plus
        // every previously-defined style.  That command set is registry data,
        // so the analyser / LSP resolve those heads inside the body instead of
        // flagging them unknown (#806).
        body_scope: Some(&crate::scoped::REPORT_DEFSTYLE_ENV),
        ..CommandSpec::DEFAULT
    }
}

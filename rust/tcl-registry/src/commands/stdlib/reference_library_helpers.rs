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

//! Library helpers available from the upstream Tcl instalment.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

pub fn specs() -> [CommandSpec; 3] {
    [
        CommandSpec {
            name: "http::mapReply",
            surface: Some(SpecSurface::ALL_TCL),
            required_package: Some("http"),
            arity: Arity::exact(1),
            hover: Some(HoverSnippet::brief(
                "Encode a string for use in an HTTP query.",
                &["http::mapReply string"],
                "Tcl stdlib http package",
            )),
            ..CommandSpec::DEFAULT
        },
        CommandSpec {
            name: "pkg_compareExtension",
            surface: Some(SpecSurface::TCL84),
            arity: Arity::new(1, 2),
            hover: Some(HoverSnippet::brief(
                "Compare a filename extension using the platform's case rules.",
                &["pkg_compareExtension fileName ?ext?"],
                "Tcl 8.4 package.tcl",
            )),
            ..CommandSpec::DEFAULT
        },
        CommandSpec {
            name: "tclLdAout",
            surface: Some(SpecSurface::TCL84),
            arity: Arity::new(0, 3),
            hover: Some(HoverSnippet::brief(
                "Link a shared Tcl extension for an a.out system.",
                &["tclLdAout ?cc? ?shlib_suffix? ?shlib_cflags?"],
                "Tcl 8.4 ldAout.tcl",
            )),
            ..CommandSpec::DEFAULT
        },
    ]
}

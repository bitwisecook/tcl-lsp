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

//! `vopt` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "vopt ?+acc? ?-o optimized_name? ?-debugdb? ?-L library? top_module",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "vopt",
        dialects: Some(DialectSet::TCL85),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Optimize a compiled design for simulation.",
            &["vopt ?+acc? ?-o optimized_name? ?-debugdb? ?-L library? top_module"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

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

//! `phys_opt_design` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "phys_opt_design ?-directive directive? ?-fanout_opt? ?-placement_opt? ?-rewire? ?-critical_cell_opt? ?-dsp_register_opt? ?-bram_register_opt? ?-hold_fix?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "phys_opt_design",
        dialects: Some(DialectSet::TCL85),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Run physical optimization after placement.",
            &[
                "phys_opt_design ?-directive directive? ?-fanout_opt? ?-placement_opt? ?-rewire? ?-critical_cell_opt?",
            ],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

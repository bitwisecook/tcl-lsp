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

//! `create_generated_clock` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "create_generated_clock ?-name name? -source master_pin ?-edges edge_list? ?-divide_by factor? ?-multiply_by factor? ?-duty_cycle percent? ?-invert? ?-add? source_objects",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_generated_clock",
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create a generated clock object.",
            &[
                "create_generated_clock ?-name name? -source master_pin ?-edges edge_list? ?-divide_by factor? ?-mult",
            ],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

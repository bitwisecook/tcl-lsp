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

//! `yaml::yaml2dict` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "yaml::yaml2dict ?-file? yamlText",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yaml::yaml2dict",
        surface: None,
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Parse a YAML string and return a Tcl dict.",
            synopsis: &["yaml::yaml2dict ?-file? yamlText"],
            snippet: "",
            source: "tcllib yaml package",
            examples: "set data [yaml::yaml2dict $yamlString]",
            return_value: "A Tcl dict representing the YAML structure.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("yaml"),
        required_package: Some("yaml"),
        ..CommandSpec::DEFAULT
    }
}

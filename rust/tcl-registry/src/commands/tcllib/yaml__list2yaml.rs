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

//! `yaml::list2yaml` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "yaml::list2yaml listValue ?indent? ?wordwrap?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yaml::list2yaml",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet {
            summary: "Convert a Tcl list to a YAML string.",
            synopsis: &["yaml::list2yaml listValue ?indent? ?wordwrap?"],
            snippet: "",
            source: "tcllib yaml package",
            examples: "",
            return_value: "A YAML-formatted string.",
        }),
        forms: FORMS,
        tcllib_package: Some("yaml"),
        required_package: Some("yaml"),
        ..CommandSpec::DEFAULT
    }
}

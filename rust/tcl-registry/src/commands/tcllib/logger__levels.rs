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

//! `logger::levels` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "logger::levels",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::levels",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Return the list of valid log levels.",
            synopsis: &["logger::levels"],
            snippet: "",
            source: "tcllib logger package",
            examples: "",
            return_value: "The list: debug info notice warn error critical alert emergency.",
        }),
        forms: FORMS,
        tcllib_package: Some("logger"),
        required_package: Some("logger"),
        ..CommandSpec::DEFAULT
    }
}

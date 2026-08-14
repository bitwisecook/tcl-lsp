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

//! `uri::geturl` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[
    SideEffect {
        target: SideEffectTarget::FileIo,
        reads: true,
        ..SideEffect::DEFAULT
    },
    SideEffect {
        target: SideEffectTarget::NetworkIo,
        reads: true,
        ..SideEffect::DEFAULT
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "uri::geturl url ?options...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uri::geturl",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Fetch the contents of a URI.",
            synopsis: &["uri::geturl url ?options...?"],
            snippet: "",
            source: "tcllib uri package",
            examples: "",
            return_value: "The fetched content.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("uri"),
        required_package: Some("uri"),
        ..CommandSpec::DEFAULT
    }
}

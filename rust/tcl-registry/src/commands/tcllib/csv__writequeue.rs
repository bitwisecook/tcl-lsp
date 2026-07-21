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

//! `csv::writequeue` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "csv::writequeue q chan ?sepChar? ?quoteChar?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::writequeue",
        dialects: None,
        arity: Arity::new(2, 4),
        hover: Some(HoverSnippet {
            summary: "Write a queue object to a channel in CSV format.",
            synopsis: &["csv::writequeue q chan ?sepChar? ?quoteChar?"],
            snippet: "",
            source: "tcllib csv package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("csv"),
        required_package: Some("csv"),
        ..CommandSpec::DEFAULT
    }
}

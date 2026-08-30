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

//! `log_user` command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
const OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-info",
    value: OptionValue::flag(),
    detail: "Return current setting.",
    surface: None,
    aliases: &[],
    lifecycle: Lifecycle::UNSPECIFIED,
    min_abbrev: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "log_user ?-info | 0 | 1?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "log_user",
        surface: Some(SpecSurface::EXPECT),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary: "Control whether send/expect output is logged to stdout.",
            synopsis: &["log_user -info", "log_user 0|1"],
            snippet: "With ``1`` (default), output is sent to stdout. With ``0``, output is suppressed.",
            source: "Expect log_user(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}

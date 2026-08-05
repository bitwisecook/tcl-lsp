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

//! `exp_continue` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-continue_timer",
    value: OptionValue::flag(),
    detail: "Do not restart the timeout timer.",
    dialects: None,
    aliases: &[],
    lifecycle: Lifecycle::UNSPECIFIED,
    min_abbrev: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "exp_continue ?-continue_timer?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exp_continue",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary: "Continue matching within an expect body instead of returning.",
            synopsis: &["exp_continue ?-continue_timer?"],
            snippet: "Used inside an ``expect`` body to re-enter the pattern matching loop. With ``-continue_timer``, the timeout timer is not restarted.",
            source: "Expect exp_continue(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}

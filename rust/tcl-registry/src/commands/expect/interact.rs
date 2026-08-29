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

//! `interact` command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-re",
        value: OptionValue::flag(),
        detail: "Match as regular expression.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-ex",
        value: OptionValue::flag(),
        detail: "Match as exact string.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-input",
        value: OptionValue::value("spawn_id"),
        detail: "Specify input source.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-output",
        value: OptionValue::value("spawn_id"),
        detail: "Specify output destination.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-u",
        value: OptionValue::value("spawn_id"),
        detail: "Connect user to the specified process.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-o",
        value: OptionValue::flag(),
        detail: "Apply to output.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-i",
        value: OptionValue::value("spawn_id"),
        detail: "Specify spawn id.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-echo",
        value: OptionValue::flag(),
        detail: "Echo characters.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-nobuffer",
        value: OptionValue::flag(),
        detail: "Do not buffer input.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-f",
        value: OptionValue::flag(),
        detail: "Force — do not flush.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-F",
        value: OptionValue::flag(),
        detail: "Force — flush.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-reset",
        value: OptionValue::flag(),
        detail: "Reset terminal modes.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "interact ?-opts? ?string body ...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "interact",
        surface: Some(SpecSurface::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Give control of the current process to the user for interactive use.",
            synopsis: &["interact ?-opts? ?string body ...?", "interact"],
            snippet: "Connects the user's terminal to the spawned process. With string/body pairs, intercepts matching input and executes the body instead.",
            source: "Expect interact(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}

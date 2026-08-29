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

//! `expect_after` command.
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
        name: "-gl",
        value: OptionValue::flag(),
        detail: "Match as glob (default).",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-nocase",
        value: OptionValue::flag(),
        detail: "Case-insensitive matching.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-i",
        value: OptionValue::value("spawn_id"),
        detail: "Specify the spawn id.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-info",
        value: OptionValue::flag(),
        detail: "Return current patterns.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "expect_after ?-opts? pattern body ?pattern body ...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "expect_after",
        surface: Some(SpecSurface::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Define patterns tested after each expect command.",
            synopsis: &["expect_after ?-opts? pattern body ?pattern body ...?"],
            snippet: "",
            source: "Expect expect_after(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        case_list: Some(&CaseListSpec::EXPECT),
        analyser_hook: Some(crate::hooks::AnalyserHookId::Switch),
        ..CommandSpec::DEFAULT
    }
}

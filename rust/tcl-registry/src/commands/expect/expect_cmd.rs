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

//! `expect` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[
    // `-brace` is deliberately an exact-only outer-shape selector, not a
    // clause flag: it makes one braced word a pattern/action list.
    OptionSpec {
        name: "-brace",
        value: OptionValue::flag(),
        detail: "Interpret the following braced word as pattern/action pairs.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: Some(6),
    },
    OptionSpec {
        name: "-nobrace",
        value: OptionValue::flag(),
        detail: "Treat a following braced word as one pattern, not a clause list.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-regexp",
        value: OptionValue::flag(),
        detail: "Match pattern as a Tcl regular expression.",
        dialects: None,
        aliases: &["-re"],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-exact",
        value: OptionValue::flag(),
        detail: "Match pattern as an exact string.",
        dialects: None,
        aliases: &["-ex"],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-glob",
        value: OptionValue::flag(),
        detail: "Match pattern as a glob (default).",
        dialects: None,
        aliases: &["-gl"],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-nocase",
        value: OptionValue::flag(),
        detail: "Case-insensitive matching.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-timeout",
        value: OptionValue::value("seconds"),
        detail: "Override the timeout for this expect.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-i",
        value: OptionValue::value("spawn_id"),
        detail: "Specify the spawn id to expect from.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-indices",
        value: OptionValue::flag(),
        detail: "Store match indices in expect_out.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-iread",
        value: OptionValue::flag(),
        detail: "Use the current indirect-read mode for this clause.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-timestamp",
        value: OptionValue::flag(),
        detail: "Record the match timestamp for this clause.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-notransfer",
        value: OptionValue::flag(),
        detail: "Do not consume matched output.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "expect ?-opts? pattern body ?pattern body ...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "expect",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Wait for output matching a pattern from a spawned process.",
            synopsis: &[
                "expect ?-opts? pattern body ?pattern body ...?",
                "expect -re {regexp} { actions }",
                "expect timeout { timeout_actions }",
                "expect eof { eof_actions }",
            ],
            snippet: "Waits until one of the patterns matches the output of the current spawned process, then executes the corresponding body. Special patterns: ``timeout``, ``eof``, ``default``, ``full_buffer``, ``null``.",
            source: "Expect expect(1)",
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

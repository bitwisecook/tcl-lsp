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

//! `expect_user` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-re",
        value: OptionValue::flag(),
        detail: "Match as regular expression.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-ex",
        value: OptionValue::flag(),
        detail: "Match as exact string.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-gl",
        value: OptionValue::flag(),
        detail: "Match as glob (default).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-nocase",
        value: OptionValue::flag(),
        detail: "Case-insensitive matching.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-timeout",
        value: OptionValue::value("seconds"),
        detail: "Override the timeout.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-indices",
        value: OptionValue::flag(),
        detail: "Store match indices.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-notransfer",
        value: OptionValue::flag(),
        detail: "Do not consume matched output.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "expect_user ?-opts? pattern body ?pattern body ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "expect_user",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Expect input from the user (standard input).",
            synopsis: &["expect_user ?-opts? pattern body ?pattern body ...?"],
            snippet: "",
            source: "Expect expect_user(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        case_list: Some(&CaseListSpec::EXPECT),
        ..CommandSpec::DEFAULT
    }
}

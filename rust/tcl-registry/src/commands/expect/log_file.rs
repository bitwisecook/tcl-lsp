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

//! `log_file` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-a",
        takes_value: false,
        value_hint: "",
        detail: "Append to existing log file.",
        dialects: None,
    },
    OptionSpec {
        name: "-noappend",
        takes_value: false,
        value_hint: "",
        detail: "Overwrite existing log file.",
        dialects: None,
    },
    OptionSpec {
        name: "-open",
        takes_value: true,
        value_hint: "fileId",
        detail: "Log to an already-open Tcl file id.",
        dialects: None,
    },
    OptionSpec {
        name: "-leaveopen",
        takes_value: false,
        value_hint: "",
        detail: "Leave the file open on close.",
        dialects: None,
    },
    OptionSpec {
        name: "-info",
        takes_value: false,
        value_hint: "",
        detail: "Return current log file settings.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "log_file ?-option ...? ?file?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "log_file",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Control logging of session output to a file.",
            synopsis: &["log_file ?-option ...? ?file?", "log_file -info"],
            snippet: "",
            source: "Expect log_file(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}

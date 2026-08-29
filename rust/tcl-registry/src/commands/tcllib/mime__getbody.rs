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

//! `mime::getbody` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "mime::getbody token ?options?",
    ..FormSpec::DEFAULT
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-decode",
        value: OptionValue::flag(),
        detail: "Decode the body according to its content-transfer-encoding.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-command",
        // Async mode.  mime invokes the prefix via `uplevel #0 $cmd [list …]`
        // at seven sites (mime.tcl 1671-1754): the terminal `[list end]`
        // appends 1 word, while `[list data $chunk]` / `[list error $reason]`
        // append 2 — because `uplevel` re-splits the list word.  So the min is
        // 1 → AtLeast(1) (never fires "too few" against a 1-arg callback; the
        // package's own default `getbodyaux {token reason {fragment {}}}` is
        // exactly this shape).
        value: OptionValue::deferred_command_prefix_n("prefix", AppendedArity::AtLeast(1)),
        detail: "Process the body asynchronously: the prefix is invoked with a \
reason keyword (data / end / error) and, for data / error, one payload word.",
        ..OptionSpec::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::getbody",
        surface: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Return the body of a MIME part.",
            synopsis: &["mime::getbody token ?-decode? ?-command cmdprefix?"],
            snippet: "",
            source: "tcllib mime package",
            examples: "",
            return_value: "The body content.",
        }),
        forms: FORMS,
        options: OPTIONS,
        tcllib_package: Some("mime"),
        required_package: Some("mime"),
        ..CommandSpec::DEFAULT
    }
}

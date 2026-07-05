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

//! `lsort` — sort a list.
use crate::prelude::*;

/// The full set of `lsort` options (all 12), so completion and
/// option-aware arity work. These are the DEFAULT-form options.
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-ascii",
        takes_value: false,
        value_hint: "",
        detail: "Compare as ASCII strings (the default).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-dictionary",
        takes_value: false,
        value_hint: "",
        detail: "Compare using dictionary-style ordering (case-insensitive, embedded numbers).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-integer",
        takes_value: false,
        value_hint: "",
        detail: "Compare elements as integers.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-real",
        takes_value: false,
        value_hint: "",
        detail: "Compare elements as floating-point values.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-nocase",
        takes_value: false,
        value_hint: "",
        detail: "Case-insensitive comparison.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-increasing",
        takes_value: false,
        value_hint: "",
        detail: "Sort in increasing order (the default).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-decreasing",
        takes_value: false,
        value_hint: "",
        detail: "Sort in decreasing order.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-indices",
        takes_value: false,
        value_hint: "",
        detail: "Return the indices of the sorted elements rather than the elements.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-unique",
        takes_value: false,
        value_hint: "",
        detail: "Retain only the last of a run of equal elements.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-command",
        takes_value: true,
        value_hint: "cmdPrefix",
        detail: "Use a custom comparison command prefix.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-index",
        takes_value: true,
        value_hint: "index",
        detail: "Sort on a sub-element selected by index path.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-stride",
        takes_value: true,
        value_hint: "length",
        detail: "Treat the list as groups of this many elements and sort by group.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

/// The single DEFAULT invocation form.
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lsort ?options? list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lsort",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        options: OPTIONS,
        forms: FORMS,
        hover: Some(HoverSnippet::brief(
            "Sort the elements of a list.",
            &["lsort ?option ...? list"],
            "Tcl lsort(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}

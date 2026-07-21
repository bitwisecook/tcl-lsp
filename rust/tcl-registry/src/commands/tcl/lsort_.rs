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
        value: OptionValue::flag(),
        detail: "Compare as ASCII strings (the default).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-dictionary",
        value: OptionValue::flag(),
        detail: "Compare using dictionary-style ordering (case-insensitive, embedded numbers).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-integer",
        value: OptionValue::flag(),
        detail: "Compare elements as integers.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-real",
        value: OptionValue::flag(),
        detail: "Compare elements as floating-point values.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-nocase",
        value: OptionValue::flag(),
        detail: "Case-insensitive comparison.",
        // Added to `lsort` in Tcl 8.5.
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-increasing",
        value: OptionValue::flag(),
        detail: "Sort in increasing order (the default).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-decreasing",
        value: OptionValue::flag(),
        detail: "Sort in decreasing order.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-indices",
        value: OptionValue::flag(),
        detail: "Return the indices of the sorted elements rather than the elements.",
        // Added to `lsort` in Tcl 8.5.
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-unique",
        value: OptionValue::flag(),
        detail: "Retain only the last of a run of equal elements.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-command",
        // Invoked as `cmdPrefix elem1 elem2` → 2 appended args.
        value: OptionValue::command_prefix_n("cmdPrefix", AppendedArity::Exactly(2)),
        detail: "Use a custom comparison command prefix.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-index",
        value: OptionValue::value("index"),
        detail: "Sort on a sub-element selected by index path.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-stride",
        value: OptionValue::value("length"),
        detail: "Treat the list as groups of this many elements and sort by group.",
        // Added to `lsort` in Tcl 8.6 (TIP 326 — TIP 351 is `lsearch`'s
        // later, 9.0+ `-stride`, not this one).
        dialects: Some(DialectSet::TCL86_PLUS),
        aliases: &[],
        min_version: None,
    },
];

/// The single DEFAULT invocation form.
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lsort ?options? list",
    dialects: None,
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

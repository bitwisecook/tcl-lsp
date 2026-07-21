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

//! `lsearch` — search a list for a pattern.
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lsearch ?options? list pattern",
    dialects: None,
}];

/// Option table for `lsearch`.  Most flags exist since Tcl 8.4;
/// `-stride` was added to `lsearch` in Tcl 9.0 (TIP 351 — NOT 8.6; the 8.6
/// `lsort -stride` is the separate, earlier TIP 326, and tclsh8.6 rejects
/// `lsearch -stride`), and is dialect-gated so W004 fires on
/// `lsearch -stride` in pre-9.0 dialects.
static OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-all",
        value: OptionValue::flag(),
        detail: "Return list of all matching indices.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-ascii",
        value: OptionValue::flag(),
        detail: "ASCII string comparison.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-bisect",
        value: OptionValue::flag(),
        detail: "Binary search a sorted list (implies -sorted).",
        dialects: Some(DialectSet::TCL86_PLUS),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-decreasing",
        value: OptionValue::flag(),
        detail: "List is sorted in decreasing order (with -sorted).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-dictionary",
        value: OptionValue::flag(),
        detail: "Dictionary-order comparison.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-exact",
        value: OptionValue::flag(),
        detail: "Exact equality match.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-glob",
        value: OptionValue::flag(),
        detail: "Glob-pattern match (default).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-increasing",
        value: OptionValue::flag(),
        detail: "List is sorted in increasing order (with -sorted).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    // `-index`, `-nocase`, `-subindices` were added to `lsearch` in Tcl 8.5.
    OptionSpec {
        name: "-index",
        value: OptionValue::value("indexList"),
        detail: "Compare against nested sub-element at indexList.",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-inline",
        value: OptionValue::flag(),
        detail: "Return matching values instead of indices.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-integer",
        value: OptionValue::flag(),
        detail: "Integer comparison.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-nocase",
        value: OptionValue::flag(),
        detail: "Case-insensitive comparison.",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-not",
        value: OptionValue::flag(),
        detail: "Invert the sense of the match.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-real",
        value: OptionValue::flag(),
        detail: "Floating-point comparison.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-regexp",
        value: OptionValue::flag(),
        detail: "Regular-expression match.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-sorted",
        value: OptionValue::flag(),
        detail: "List is sorted; use binary search.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-start",
        value: OptionValue::value("index"),
        detail: "Start the search at the given index.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    // `lsearch -stride` is Tcl 9.0-only, TIP 351 (tclsh8.6 rejects it with
    // "bad option -stride"; the 8.6 `lsort -stride` is the separate TIP 326).
    OptionSpec {
        name: "-stride",
        value: OptionValue::value("strideLength"),
        detail: "Treat the list as a sequence of fixed-size records.",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-subindices",
        value: OptionValue::flag(),
        detail: "Combine result with -index value for nested addressing.",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        min_version: None,
    },
    // NOTE: `lsearch` does NOT declare `--` in its option table.
    // This keeps W304 (missing-option-terminator) silent for
    // `lsearch -exact $x pattern` — the existing
    // `analyse_no_w304_for_lsearch` regression test depends on this.
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lsearch",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(2),
        return_type: Some(TclType::Int),
        options: OPTIONS,
        hover: Some(HoverSnippet::brief(
            "Search a list for a pattern.",
            &["lsearch ?option ...? list pattern"],
            "Tcl lsearch(1)",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

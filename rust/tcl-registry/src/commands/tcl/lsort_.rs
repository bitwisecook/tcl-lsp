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

/// Option table for `lsort` (all 12 documented switches). Cross-checked
/// against the TclCmd/lsort manpages for Tcl 8.4, 8.5, 8.6, 9.0, and 9.1
/// (the raw HTML body for 9.0 and 9.1 is byte-identical apart from the
/// version banner — no textual delta between those two):
/// - `-ascii`, `-dictionary`, `-integer`, `-real`, `-command`,
///   `-increasing`, `-decreasing`, `-unique` are documented in every
///   version (8.4-9.1), unchanged in shape. `-dictionary`'s own text
///   gains an explicit "Overrides the -nocase option" sentence from 8.6
///   onward, but the underlying behaviour was already true from 8.5 (when
///   `-nocase` first existed) — `-nocase`'s own 8.5 text already said it
///   "has no effect" combined with `-dictionary`/`-integer`/`-real`, this
///   is a documentation-placement change, not a behaviour change.
/// - `-nocase` and `-indices` first appear in the Tcl 8.5 manpage (absent
///   from 8.4's option list). `-index` itself is documented from 8.4, but
///   the 8.5 manpage adds the ability to pass a *list* of indices for
///   nested sublist access (`-index {0 1}`, "as if … passed to lindex");
///   8.4's text and examples only ever show a single index or `end`/
///   `end-N`.
/// - `-stride` first appears in the Tcl 8.6 manpage (TIP 326) and must be
///   `>= 2` — NOT the same option as `lsearch`'s later, Tcl-9.0-only
///   `-stride` (TIP 351, default 1 = no grouping; see `lsearch_.rs`).
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-ascii",
        value: OptionValue::flag(),
        detail: "Compare using Unicode code-point (raw string) order — the default. The flag name is a holdover from Tcl's original ASCII-only implementation; it is not restricted to ASCII text.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-dictionary",
        value: OptionValue::flag(),
        detail: "Dictionary-style comparison: like -ascii but case-insensitive except as a tie-breaker, and embedded numbers compare as integers rather than character-by-character (bigBoy sorts between bigbang and bigboy; x10y sorts between x9y and x11y). Takes precedence over -nocase.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-integer",
        value: OptionValue::flag(),
        detail: "Convert each element to an integer and compare numerically; an element that doesn't convert is an error.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-real",
        value: OptionValue::flag(),
        detail: "Convert each element to a floating-point value and compare numerically; an element that doesn't convert is an error.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-nocase",
        value: OptionValue::flag(),
        detail: "Case-insensitive comparison. Only affects -ascii comparisons — has no effect combined with -dictionary, -integer, or -real.",
        // Added to `lsort` in Tcl 8.5 (absent from the 8.4 manpage's
        // option list).
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-increasing",
        value: OptionValue::flag(),
        detail: "Sort in increasing order, smallest items first (the default).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-decreasing",
        value: OptionValue::flag(),
        detail: "Sort in decreasing order, largest items first.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-indices",
        value: OptionValue::flag(),
        detail: "Return the sorted positions (indices) into list instead of the elements themselves.",
        // Added to `lsort` in Tcl 8.5.
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-unique",
        value: OptionValue::flag(),
        detail: "Retain only the last element of each run of duplicates in the sorted result. Duplicates are determined by the comparison in use — e.g. with -index 0, {1 a} and {1 b} count as duplicates and only {1 b} is kept.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-command",
        // Invoked as `cmdPrefix elem1 elem2` → 2 appended args (identical
        // wording in every fetched version: "the two elements appended as
        // additional arguments").
        value: OptionValue::command_prefix_n("cmdPrefix", AppendedArity::Exactly(2)),
        detail: "Use cmdPrefix as the comparator: invoked with the two elements being compared appended as additional arguments, and must return an integer less than, equal to, or greater than zero if the first is respectively less than, equal to, or greater than the second. lsort is reentrant, so cmdPrefix may itself call lsort.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-index",
        value: OptionValue::value("indexList"),
        detail: "Sort by the sub-element at indexList within each list element (itself treated as a sublist, unless -stride is also given) instead of the whole element; indexList accepts end/end-N and, since Tcl 8.5, may itself be a list of indices for nested sublist access (as if passed to lindex). Combined with -stride, the index is relative to each group. Much more efficient than an equivalent -command comparator.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-stride",
        value: OptionValue::Takes(OptionArg {
            integer: Some(IntegerDomain::Range(2, i64::MAX)),
            hint: "strideLength",
            ..OptionArg::DEFAULT
        }),
        detail: "Treat list as consecutive groups of strideLength elements (strideLength must be at least 2, and list's length a multiple of it), keeping each group's element order fixed while sorting whole groups by their first element, or by the -index'th element within each group when -index is also given.",
        // Added to `lsort` in Tcl 8.6 (TIP 326 — TIP 351 is `lsearch`'s
        // later, 9.0+ `-stride`, not this one).
        dialects: Some(DialectSet::TCL86_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
];

/// The single invocation form — `lsort ?options? list` is unchanged
/// across Tcl 8.4 through 9.1.
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lsort ?options? list",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lsort",
        dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)),
        // NOT `Traits::PURE` / `Traits::CSE_CANDIDATE`: unlike `lsearch`
        // (which has no comparator option), `-command cmdPrefix` lets a
        // call site name an arbitrary command that runs with the
        // interpreter's full state as part of the sort — `tcl-vm/src/
        // cmd_list.rs`'s `vm_compare` dispatches it via `vm.dispatch`
        // with no purity check. `classify_side_effects`
        // (`tcl-compiler/src/side_effects.rs`) treats a `PURE`-tagged
        // command as pure for *every* call site regardless of its actual
        // arguments (there is no per-option purity carve-out), so
        // `lsort -command sideEffectingProc $list` would otherwise be
        // wrongly eligible for CSE/dead-code elimination. Matches the
        // sibling command-prefix-bearing specs (`chan`, `socket`,
        // `fcopy`, `trace`), none of which carry `PURE`/`CSE_CANDIDATE`.
        //
        // `FRAMELESS_RUNTIME` (unlike `PURE`) stays: `cmd_lsort` is a
        // dedicated Rust runtime helper that never falls back to a
        // generic eval path, and `-command`'s `vm.dispatch` call invokes
        // the comparator as its own ordinary nested call (not an
        // `uplevel`-style scope walk back into the caller's frame), so
        // calling `lsort` itself still needs no runtime frame.
        traits: Traits::FRAMELESS_RUNTIME | Traits::BYTE_COMPILED,
        arity: Arity::at_least(1),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        options: OPTIONS,
        forms: FORMS,
        hover: Some(HoverSnippet {
            summary: "Sort the elements of a list.",
            synopsis: &["lsort ?options? list"],
            snippet: "Sorts using a stable merge sort (O(n log n)); equal elements keep their relative input order, except that -unique discards all but the last of each run of duplicates. -ascii (Unicode code-point order) and -increasing are the defaults; -dictionary, -integer, -real, or -command select an alternate comparison. -nocase (Tcl 8.5+) folds case, but only affects -ascii comparisons — it has no effect combined with -dictionary, -integer, or -real. -index sorts by a sub-element instead of the whole list element — since Tcl 8.5 it may itself be a list of indices for nested access — and, combined with -stride (Tcl 8.6+), is relative to each fixed-size group rather than the whole element. -indices (Tcl 8.5+) returns element positions instead of the elements themselves. lsort is reentrant, so a -command comparator may itself call lsort.",
            source: "Tcl lsort(n)",
            examples: "lsort {b a c}\nlsort -integer -unique {3 1 4 1 5 9 2 6}\nlsort -integer -index 1 {{First 24} {Second 18} {Third 30}}\nlsort -stride 2 -index 1 -integer {carrot 10 apple 50 banana 25}",
            return_value: "A new list containing the same elements as list, permuted into sorted order (or, with -indices, the list of indices that would produce that order).",
        }),
        ..CommandSpec::DEFAULT
    }
}

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

//! `lsearch` — search a list for an element that matches a pattern.
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lsearch ?options? list pattern",
    dialects: None,
}];

/// Option table for `lsearch`. Cross-checked against the TclCmd/lsearch
/// manpages for Tcl 8.4, 8.5, 8.6, 9.0, and 9.1 (the synopsis itself —
/// `lsearch ?options? list pattern` — is identical in all five):
/// - `-all`, `-ascii`, `-decreasing`, `-dictionary`, `-exact`, `-glob`,
///   `-increasing`, `-inline`, `-integer`, `-not`, `-real`, `-regexp`,
///   `-sorted`, `-start` are documented in every version (8.4-9.1),
///   unchanged in shape — though `-start`'s accepted index syntax widens
///   in 8.5 (see its `detail` below).
/// - `-index`, `-nocase`, `-subindices` first appear in the Tcl 8.5
///   manpage (absent from 8.4's option list).
/// - `-bisect` first appears in the Tcl 8.6 manpage (absent from both 8.4
///   and 8.5).
/// - `-stride` was added to `lsearch` in Tcl 9.0 (TIP 351 — NOT 8.6; the
///   8.6 `lsort -stride` is the separate, earlier TIP 326, and tclsh8.6
///   rejects `lsearch -stride`), and is dialect-gated so W004 fires on
///   `lsearch -stride` in pre-9.0 dialects.
static OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-all",
        value: OptionValue::flag(),
        detail: "Return every matching index (ascending order) instead of just the first; with -inline, every matching value.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-ascii",
        value: OptionValue::flag(),
        detail: "Compare as Unicode text (the flag name is historical, not literally ASCII-only); the default contents style, meaningful only with -exact or -sorted.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    // `-bisect` first appears in the Tcl 8.6 manpage — absent from both
    // the 8.4 and 8.5 option lists.
    OptionSpec {
        name: "-bisect",
        value: OptionValue::flag(),
        detail: "Binary search for an inexact match: the last index <= pattern (increasing list) or >= pattern (decreasing list), or -1 if none qualify. Implies -sorted; cannot combine with -all or -not.",
        dialects: Some(DialectSet::TCL86_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-decreasing",
        value: OptionValue::flag(),
        detail: "List is sorted in decreasing order (with -sorted).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-dictionary",
        value: OptionValue::flag(),
        detail: "Dictionary-style comparison (with -exact/-sorted); differs from -ascii only under -sorted, since values are only dictionary-equal when exactly equal.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-exact",
        value: OptionValue::flag(),
        detail: "Exact equality match.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-glob",
        value: OptionValue::flag(),
        detail: "Glob-pattern match (default).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-increasing",
        value: OptionValue::flag(),
        detail: "List is sorted in increasing order (with -sorted; the default sort order).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    // `-index`, `-nocase`, `-subindices` were added to `lsearch` in Tcl 8.5
    // (absent from the 8.4 manpage's option list).
    OptionSpec {
        name: "-index",
        value: OptionValue::value("indexList"),
        detail: "Search/compare a nested sub-element addressed by indexList (a path of lindex/lset-style indices) instead of the whole element.",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-inline",
        value: OptionValue::flag(),
        detail: "Return the matching value(s) instead of index(es); empty string if nothing matches.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-integer",
        value: OptionValue::flag(),
        detail: "Integer comparison (with -exact/-sorted).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-nocase",
        value: OptionValue::flag(),
        detail: "Case-insensitive comparison (with -exact/-sorted); has no effect combined with -dictionary, -integer, or -real.",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-not",
        value: OptionValue::flag(),
        detail: "Invert the match: return the first non-matching element's index (or value) instead of the first match.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-real",
        value: OptionValue::flag(),
        detail: "Floating-point comparison (with -exact/-sorted).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-regexp",
        value: OptionValue::flag(),
        detail: "Regular-expression match.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-sorted",
        value: OptionValue::flag(),
        detail: "List is sorted; use a faster search. Mutually exclusive with -glob/-regexp, and behaves like -exact when combined with -all or -not.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-start",
        value: OptionValue::value("index"),
        detail: "Start the search at index. Tcl 8.4 accepts a literal integer, end, or end-N; Tcl 8.5+ accepts full string-index arithmetic (the same rules as `string index`).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    // `lsearch -stride` is Tcl 9.0-only, TIP 351 (tclsh8.6 rejects it with
    // "bad option -stride"; the 8.6 `lsort -stride` is the separate TIP 326).
    OptionSpec {
        name: "-stride",
        value: OptionValue::Takes(OptionArg {
            integer: Some(IntegerDomain::Range(1, i64::MAX)),
            hint: "strideLength",
            ..OptionArg::DEFAULT
        }),
        detail: "Treat the list as fixed-size groups of strideLength elements, matching each group's first element (or, with -index, the element -index selects within it); the reported index always points to the group's first element. strideLength must be >= 1 (default 1, no grouping) and evenly divide the list length.",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-subindices",
        value: OptionValue::flag(),
        detail: "With -index, report the full index path (usable with lindex/lset) to a match instead of just the top-level index; no effect without -index.",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    // NOTE: `lsearch` does NOT declare `--` in its option table.
    // This keeps W304 (missing-option-terminator) silent for
    // `lsearch -exact $x pattern` — the existing
    // `analyse_no_w304_for_lsearch` regression test depends on this.
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lsearch",
        dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(2),
        return_type: Some(TclType::Int),
        // Default matching style is glob (`-glob`) in every version;
        // `PatternType::Glob`'s own doc comment names `lsearch` as the
        // canonical example. `-exact`/`-regexp` can select a different
        // style per call, but the registry's `pattern_type` is a single
        // per-command fact, so it records the default.
        pattern_type: Some(PatternType::Glob),
        options: OPTIONS,
        hover: Some(HoverSnippet {
            summary: "Search a list for an element matching a pattern.",
            synopsis: &["lsearch ?options? list pattern"],
            snippet: "Returns the index of the first element of list that matches pattern, or -1 if none match. -exact, -glob (the default), -regexp, and -sorted select the matching style; when more than one is given, the last one wins. -sorted assumes an increasing, ASCII-ordered list unless -decreasing/-dictionary/-integer/-real/-nocase say otherwise, and behaves like -exact whenever -all or -not is also given. -all returns every matching index instead of just the first, and -inline returns the matching value(s) instead of index(es) — with either, a search that finds nothing returns the empty string rather than -1. -index (Tcl 8.5+) searches a nested sub-element of each list element instead of the whole element, and -bisect (Tcl 8.6+) turns a sorted search into an inexact nearest-match lookup. -stride (Tcl 9.0+) treats list as fixed-size groups, matching only against each group's first (or -index-selected) element.",
            source: "Tcl lsearch(n)",
            examples: "lsearch {a b c d e} c\nlsearch -all -inline -not -exact {a b c a d e a f g a} a\nlsearch -sorted -bisect {1 3 5 7 9} 6\nlsearch -index 1 -inline {{a20 red} {b35 blue} {c47 green}} b*",
            return_value: "The index of the first matching element, or -1 if none match. With -all, every matching index (or, combined with -inline, every matching value) instead of just the first — empty instead of -1 when nothing matches. With -inline alone, the matching value itself instead of its index.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

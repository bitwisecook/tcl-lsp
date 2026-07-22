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

//! `glob` — return names of files that match patterns.

use crate::prelude::*;

/// Tcl 8.6 (TIP 323, "Do Nothing Gracefully") made the pattern list itself
/// optional — `glob ?switches?` with zero patterns is syntactically legal
/// from 8.6 on (it simply matches nothing, subject to `-nocomplain`/the
/// Tcl 9.0 default change on the `-nocomplain` `OptionSpec` below). Tcl 8.4
/// and 8.5 still require at least one `pattern` word. The command's own
/// `arity` stays `Arity::any()` (the 8.6+ shape) rather than the stricter
/// 8.4/8.5 floor, so a legitimate 8.6+/9.x zero-pattern call is never
/// misflagged; these two forms carry the precise per-era synopsis instead.
const FORMS: &[FormSpec] = &[
    FormSpec {
        kind: FormKind::Default,
        synopsis: "glob ?switches? ?pattern ...?",
        dialects: Some(DialectSet::TCL86_PLUS),
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "glob ?switches? pattern ?pattern ...?",
        dialects: Some(DialectSet::TCL84.union(DialectSet::TCL85)),
    },
];

/// `-types`' value is a Tcl list combining two independently-matched
/// vocabularies (see the option's own `detail`): the Unix find-style kind
/// letters (`b`/`c`/`d`/`f`/`l`/`p`/`s`) are OR'ed together — a file need
/// only satisfy one, since a file has exactly one kind — while the
/// permission words (`r`/`w`/`x`/`readonly`/`hidden`) are AND'ed — a file
/// must satisfy every one given. A list value can't be checked against a
/// flat per-word enum, so this is completion/hover data only: `glob` has
/// no `closed_value_args`/closed `OptionArg` entry for this option, since a
/// legitimate multi-word list (e.g. `{d r}`) would otherwise be
/// misreported as an invalid value. Stable across 8.4–9.1 — every fetched
/// manpage documents the identical letter/word set.
const TYPES_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "b",
        detail: "Block special file.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "c",
        detail: "Character special file.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "d",
        detail: "Directory.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "f",
        detail: "Plain file.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "l",
        detail: "Symbolic link.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "p",
        detail: "Named pipe (FIFO).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "s",
        detail: "Socket.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "r",
        detail: "Permission form: readable by the current process.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "w",
        detail: "Permission form: writable by the current process.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "x",
        detail: "Permission form: executable by the current process.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "readonly",
        detail: "Permission form: the file is marked read-only.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "hidden",
        detail: "Permission form: a hidden file — a leading-dot name on Unix, or the OS hidden attribute on Windows.",
        ..ArgValue::DEFAULT
    },
];

/// Command spec for `glob`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "glob",
        // Universal (`None`), deliberately: `glob` is banned in F5 iRules
        // (the TMM sandbox has no real filesystem), but that exclusion is
        // modelled subtractively via `IRULES_DISABLED_COMMANDS` in
        // `tcl-dialect/src/profile.rs` — which names `glob` explicitly and
        // whose own doc comment states the specs it lists stay plain
        // universal data. `tests/dialect_profile.rs`'s
        // `irules_disable_list_is_load_bearing` asserts exactly this: every
        // banned name must still resolve under the bare `IRULES` mask, so
        // gating this field instead would silently break that contract.
        // Every other modelled dialect (Expect, Tk, the EDA vendor
        // consoles, F5 iApps, F5 tmsh, incr Tcl) hosts a real filesystem-
        // backed Tcl and has no `glob` restriction of its own (empty
        // `disabled_commands` in every other `DialectProfile` entry).
        dialects: None,
        traits: Traits::BYTE_COMPILED | Traits::SAFE_INTERP_HIDDEN,
        // 8.4/8.5 require at least one `pattern` word; Tcl 8.6+ (TIP 323)
        // accepts zero. `Arity` has no per-dialect gate of its own, so this
        // takes the 8.6+ (and both 9.x) shape — the majority of, and
        // currently-developed, supported dialects — rather than
        // misflagging a legitimate modern zero-pattern call; the exact
        // per-era requirement is documented precisely on `FORMS` above.
        arity: Arity::any(),
        return_type: Some(TclType::List),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-directory",
                    value: OptionValue::value("dir"),
                    detail: "Search for files matching the patterns starting in the given directory, so a directory name containing glob-sensitive characters need not be quoted. Cannot be combined with -path.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-join",
                    value: OptionValue::flag(),
                    detail: "Join the pattern arguments, after option processing, into a single pattern using directory separators, instead of matching each pattern independently.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-nocomplain",
                    value: OptionValue::flag(),
                    detail: "Return an empty list instead of raising an error when no files match. In Tcl 8.4 through 8.6 this switch is required to avoid that error; from Tcl 9.0 (TIP 637) an empty result is never an error by default, so the switch is still accepted but has no effect.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-path",
                    value: OptionValue::value("pathPrefix"),
                    detail: "Search for files whose name begins with the given path prefix and whose remainder matches the patterns — useful for finding files that share a root name but differ in extension. Cannot be combined with -directory.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-tails",
                    value: OptionValue::flag(),
                    detail: "Return only the part of each matched name that follows the last directory named by -directory or -path, instead of the full path.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-types",
                    value: OptionValue::Takes(OptionArg {
                        values: TYPES_VALUES,
                        closed: false,
                        hint: "typeList",
                        ..OptionArg::DEFAULT
                    }),
                    detail: "Filter matches against typeList, a Tcl list combining two vocabularies: Unix find-style kind letters (b/c/d/f/l/p/s), of which a match need satisfy only one, and permission words (r/w/x/readonly/hidden), of which a match must satisfy all. Legacy macOS type/creator codes are also accepted.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "Marks the end of switches. The argument following this one will be treated as a pattern even if it starts with -.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        hover: Some(HoverSnippet {
            summary: "Return names of files that match patterns.",
            synopsis: &[
                "glob ?switches? ?pattern ...?",
                "glob ?switches? pattern ?pattern ...?",
            ],
            snippet: "Performs file name globbing similar to csh or bash, matching each pattern against names that actually exist and returning the union of matches as a list; no order is guaranteed, so pipe the result through lsort if a sorted list is needed.\n\n-directory and -path anchor the search elsewhere without needing to quote glob-sensitive characters in the anchor itself, and are mutually exclusive. -types filters the match set: kind letters (b/c/d/f/l/p/s) are OR'ed together, while permission words (r/w/x/readonly/hidden) must all be satisfied. On Unix a leading '.' in a name (or right after '/') is only matched by an explicit '.' or -types hidden.\n\nIn Tcl 8.4 and 8.5 at least one pattern is required; Tcl 8.6 (TIP 323) made the pattern list itself optional too. Without -nocomplain, a call whose result would be empty is an error in Tcl 8.4 through 8.6; from Tcl 9.0 (TIP 637) an empty result is never an error, so -nocomplain is still accepted but has no effect.",
            source: "Tcl glob(n)",
            examples: "glob *.tcl\nglob -directory [file home] *.tcl\nglob -nocomplain -types d *\nglob -types {f r} *.log",
            return_value: "A list of the matching file names, in no particular order.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

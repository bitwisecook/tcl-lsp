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

//! `lremove` — remove elements from a list by index (Tcl 9.0+, TIP 367).
//
// Manpage comparison across Tcl 8.4, 8.5, 8.6, 9.0, and 9.1
// (tcl-lang.org's TclCmd/lremove.html, `.htm` for the 8.6 tree, each
// fetched directly rather than assumed): the 8.4, 8.5, and 8.6 URLs each
// resolve to the site's standard "URL Not Found" page rather than a real
// command page — `lremove` genuinely does not exist before 9.0. The 9.0
// and 9.1 pages serve byte-identical NAME/SYNOPSIS/DESCRIPTION/EXAMPLES/
// SEE ALSO/KEYWORDS text (only the version-banner breadcrumb differs,
// "9.0.4" vs "9.1b0"), so nothing about `lremove` changed between the two
// releases. TIP 367 ("A Command to Remove Elements from a List")
// originally targeted the abandoned Tcl 8.7 release — tcl-lang.org still
// hosts an orphaned tcl8.7 copy of this manpage from that plan — and the
// command landed for real when Tcl 9.0 shipped instead. No dialect
// directory (irules/, expect/, eda_*/, iapps/, tk/, itcl/) references
// `lremove`, and none of those dialects model a base Tcl core version of
// 9.0 or later (`DialectProfile::expr_grammar_base` tops out at 8.6
// for every one of them), so the plain `TCL90_PLUS` gate below already
// excludes every one of them.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "lremove list ?index ...?",
    ..FormSpec::DEFAULT
}];

/// Command spec for `lremove`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lremove",
        // `list` is a *value* argument, not a variable name — like
        // `lreplace`/`linsert`/`lindex`, `lremove` reads nothing and
        // writes nothing; it only ever returns a new list. It has its
        // own dedicated manual page (a real "Tcl Built-In Command", not
        // a Tcl-coded library proc like `foreachLine`/`readFile`), so it
        // carries `BYTE_COMPILED` the same way its 9.0-native siblings
        // `lpop`/`ledit` do. It is not wired into `tcl-vm` (no
        // `cmd_lremove` in `tcl-vm/src/cmd_list.rs`), so — unlike
        // `lreplace`/`linsert`/`lindex`, which are — it does not carry
        // `FRAMELESS_RUNTIME`. Its shape is exactly `lindex`'s `list
        // ?index ...?`, and the signature scanner's factory-candidate
        // check only requires a 4-token call whose first argument has no
        // `$`/`[` and whose *last* argument is braced
        // (`tcl_compiler::signature_scan::handlers::
        // maybe_record_factory_candidate`) — so a 2-index call with a
        // literal list and a braced final index, e.g. `lremove {a b c d
        // e} end-1 {1}`, hits that shape exactly, the same collision
        // `lindex`/`lpop`/`ledit` already guard against, so `lremove`
        // needs `NOT_PROC_FACTORY` too. Deterministic given its
        // arguments, so a repeated call is worth flagging as redundant
        // like its `lindex`/`linsert`/`lsort` siblings (`CSE_CANDIDATE`).
        traits: Traits::PURE
            | Traits::BYTE_COMPILED
            | Traits::NOT_PROC_FACTORY
            | Traits::CSE_CANDIDATE,
        surface: Some(SpecSurface::TCL90_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        // Every returned element is one of `list`'s (arg 0) own
        // elements — just a subset of them — the same element-type
        // relationship `lrange`/`lassign` declare for their own
        // subset-of-container results.
        return_elements: Some(ReturnElements::SubListOf { container_arg: 0 }),
        hover: Some(HoverSnippet {
            summary: "Remove elements from a list by index.",
            synopsis: &["lremove list ?index ...?"],
            snippet: "Returns a new list formed from list with the elements at the given index positions removed. Any number of index arguments may be given, including none, and they may appear in any order and repeat; an element named more than once is still removed only once, with every removal resolved against the original list simultaneously rather than by re-indexing after each one. Each index uses the same syntax as string index — a plain integer, end, or a simple end-relative or offset expression such as end-1 — with 0 addressing the first element and end the last. list itself is left unmodified; lremove always returns a new list.",
            source: "Tcl lremove(n)",
            examples: "lremove {a b c d e} 2                ;# a b d e\nlremove {a b c d e} end-1 1          ;# a c e\nlremove {a b c d e} 2 end-2          ;# a b d e — 2 and end-2 both name c, removed only once",
            return_value: "A new list consisting of every element of list except those at the given indices.",
        }),
        forms: FORMS,
        arg_types: &[
            (
                0,
                ArgTypeHint {
                    expected: Some(TclType::List),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        ..CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT
    }
}

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

//! `ledit` — replace elements in a list variable, in place (Tcl 9.0+, TIP 631).
//
// Manpage: Tcl 9.0 and 9.1 `ledit(n)` — byte-identical text in both trees
// (the only difference between the two fetched pages is the version-banner
// breadcrumb). Confirmed absent from the 8.4/8.5/8.6 manpage trees: each of
// those URLs (including the `.htm` 8.6 variant) resolves to the site's
// standard "URL Not Found" page rather than a real command page, matching
// TIP 631 landing for the 9.0 release.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "ledit listVar first last ?value value ...?",
    ..FormSpec::DEFAULT
}];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

/// Command spec for `ledit`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ledit",
        // `ledit` reads the list variable's current value, replaces a
        // range, and writes the result back — a read-before-write of
        // `listVar`, like `lappend`/`append`/`incr` and unlike `lset`'s
        // multi-level index path: the manpage is explicit that "indices
        // into sublists are not supported", so `listVar` is touched as a
        // single flat name rather than through the frame's hash bucket the
        // way `lset`/`lassign` are (`Traits::FRAME_HASH_BUILTIN`,
        // deliberately not stamped here). The native VM builtin
        // (`tcl-vm/src/cmd_list.rs::cmd_ledit`) reads and writes the
        // variable directly with no eval fallback, opens no new call
        // frame, and never invokes a user-supplied callback, so it is a
        // genuine `FRAMELESS_RUNTIME` command. A bare `listVar first last`
        // call (no trailing values) is a `HEAD NAME BRACED BRACED`
        // invocation shape (e.g. `ledit x {0} {1}`), so it also needs
        // `NOT_PROC_FACTORY` to keep the proc-factory signature scanner
        // from mistaking it for a `proc`-defining wrapper call.
        traits: Traits::READS_BEFORE_WRITE
            | Traits::FIRST_ARG_VARNAME
            | Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(3),
        assigns_variable_at: Some(0),
        arg_roles: &[(0, ArgRole::VarWrite)],
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Replace a range of elements in a list variable, in place.",
            synopsis: &["ledit listVar first last ?value value ...?"],
            snippet: "Fetches the list stored in listVar and replaces the inclusive range from first to last with the value arguments, then stores the result back in listVar and returns it. first and last use the same index syntax as string index — a plain integer, or end/end-N/end+N with simple +/- arithmetic; unlike lpop, lset, and lindex, indices into sublists are not supported, so only the top-level list is edited. An index below 0 is treated as the position before the first element, which allows prepending; an index past the last element is treated as one past the end, which allows appending. When last is less than first, the value arguments are inserted before first and nothing is deleted. With no value arguments, the elements in the range are simply deleted. listVar must already exist — unlike lappend, ledit does not create it.",
            source: "Tcl ledit(n)",
            examples: "set lst {c d e f g}\nledit lst -1 -1 a b\n# lst is now \"a b c d e f g\"  (prepended)\nledit lst end+1 end+1 h i\n# lst is now \"a b c d e f g h i\"  (appended)\nledit lst 2 3\n# lst is now \"a b e f g h i\"  (deleted)\nledit lst 2 3 x y z\n# lst is now \"a b x y z g h i\"  (replaced)",
            return_value: "The modified list, which is also the new value stored in listVar.",
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
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        ..CommandSpec::DEFAULT
    }
}

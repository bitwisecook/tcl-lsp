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

//! `join` — join list elements into a string.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "join list ?joinString?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "join",
        // `Some(DialectSet::ALL_TCL.union(DialectSet::IRULES))` is
        // deliberate, not an oversight: `join` is a pure list-manipulation
        // command — none of the filesystem/process/interp surface (`open`,
        // `exec`, `file`, `glob`, `namespace`, …) that iRules bans — so it
        // carries the `IRULES` bit explicitly and resolves under the bare
        // `IRULES` mask, rather than being one of the sandbox-banned
        // commands whose group is a bare `ALL_TCL` (no `IRULES` bit). None of
        // the irules/, expect/, tk/, itcl/, iapps/, or eda_*/ command packs
        // declares an override or an extra form for it, so it resolves
        // identically everywhere.
        dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)),
        byte_array_effect: ByteArrayEffect::Coerces,
        const_fold: Some(crate::const_fold::fold_join),
        traits: Traits::FRAMELESS_RUNTIME | Traits::PURE | Traits::CSE_CANDIDATE,
        // `join list ?joinString?` — synopsis, argument defaults, and
        // documented behaviour are byte-for-byte identical across the
        // join.n manual page in Tcl 8.4, 8.5, 8.6, 9.0, and 9.1: no option
        // or form was ever added, removed, or had its default changed.
        arity: Arity::new(1, 2),
        return_type: Some(TclType::String),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        hover: Some(HoverSnippet {
            summary: "Create a string by joining together list elements.",
            synopsis: &["join list ?joinString?"],
            snippet: "list must be a valid Tcl list; an invalid list raises an error. joinString is inserted between each adjacent pair of elements and defaults to a single space when omitted. Each element contributes only its own string value, so a nested sublist element is not itself re-joined — a list nested two levels deep is flattened by exactly one level, not recursively, as shown in the second example below.",
            source: "Tcl join(n)",
            examples: "join {1 2 3 4 5} \", \"\njoin {1 {2 3} 4 {5 {6 7} 8}}",
            return_value: "A string formed by joining the elements of list together, with joinString (a single space by default) between each adjacent pair.",
        }),
        forms: FORMS,
        ..CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT
    }
}

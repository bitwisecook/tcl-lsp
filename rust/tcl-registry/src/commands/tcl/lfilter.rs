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

//! `lfilter` — select elements from a list based on an expression (Tcl 9.1).
//!
//! A loop like `lmap`, but the body/expression is a boolean predicate and the
//! command returns the sublist of values for which it is true.  Verified
//! against C Tcl 9.1b0 `generic/tclBasic.c` (`Tcl_LfilterObjCmd`,
//! `TclCompileLfilterCmd`, byte-compiled, `CMD_IS_SAFE`) and `doc/lfilter.n`.

use crate::prelude::*;

// The loop itself is neutral; the predicate/body's effects are tracked by
// recursing into its `Body` arg role.  Mirrors `lmap`.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: false,
    writes: false,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lfilter varname list expression",
}];

/// Dynamic arg role resolver: the last argument is the predicate body, like
/// `lmap`.
fn lfilter_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() >= 3 {
        u8::try_from(args.len() - 1)
            .map(|last| vec![(last, ArgRole::Body)])
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

/// Command spec for `lfilter` (Tcl 9.1).
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lfilter",
        traits: Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::HAS_LOOP_BODY
            | Traits::NEVER_INLINE_BODY
            | Traits::LOOP_LIST_HEADER,
        dialects: Some(DialectSet::TCL91),
        arity: Arity::at_least(3),
        arg_role_resolver: Some(lfilter_arg_roles),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Select elements from a list based on an expression",
            synopsis: &[
                "lfilter varname list expression",
                "lfilter varlist1 list1 ?varlist2 list2 ...? body",
            ],
            snippet: "The lfilter command implements a loop where the loop variable(s) take on values from one or more lists, and returns the sublist of values for which the boolean expression / body is true.",
            source: "Tcl man page lfilter.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}

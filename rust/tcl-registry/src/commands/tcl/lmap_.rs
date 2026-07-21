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

//! `lmap` — iterate over all elements in one or more lists and collect results.

use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: false,
    writes: false,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lmap varname list body",
    dialects: None,
}];

/// Dynamic arg role resolver: last argument is the body script.
fn lmap_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() >= 3 {
        u8::try_from(args.len() - 1)
            .map(|last| vec![(last, ArgRole::Body)])
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

/// Command spec for `lmap`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lmap",
        traits: Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::HAS_LOOP_BODY
            | Traits::NEVER_INLINE_BODY
            | Traits::LOOP_LIST_HEADER,
        dialects: Some(DialectSet::TCL86_PLUS),
        // `varList list ?varList list ...? command` — identical grammar to
        // `foreach`: n varList/list pairs (n >= 1) plus one command body, so a
        // valid count is odd and >= 3.  An even count is `wrong # args`
        // (verified against tclsh 9.0.4: `lmap a b c d` → `wrong # args: should
        // be "lmap varList list ?varList list ...? command"`).  Previously
        // `at_least(3)`, which missed the odd/even parity `foreach` enforces.
        arity: Arity::stepped(3, Arity::UNLIMITED, 2),
        arg_role_resolver: Some(lmap_arg_roles),
        // See `foreach`'s identical comment — index 0 is a fixed key read by
        // `shimmer::use_site::foreach_header_expected_type`, not a real
        // source-position argument index.
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        lowering_hook: Some(crate::hooks::LoweringHookId::Lmap),
        return_type: Some(TclType::List),
        var_write_typing: VarWriteTyping::ElementsOf { container_arg: 0 },
        hover: Some(HoverSnippet {
            summary: "Iterate over all elements in one or more lists and collect results",
            synopsis: &[
                "lmap varname list body",
                "lmap varlist1 list1 ?varlist2 list2 ...? body",
            ],
            snippet: "The lmap command implements a loop where the loop variable(s) take on values from one or more lists, and the loop returns a list of results collected from each iteration.",
            source: "Tcl man page lmap.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}

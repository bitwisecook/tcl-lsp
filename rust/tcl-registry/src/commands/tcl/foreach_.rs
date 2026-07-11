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

//! `foreach` — iterate over one or more lists.

use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "foreach varList list ?varList list ...? body",
}];

/// Dynamic arg role resolver: last argument is always the body.
fn foreach_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() >= 3 {
        u8::try_from(args.len() - 1)
            .map(|last| vec![(last, ArgRole::Body)])
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

/// Command spec for `foreach`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "foreach",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::HAS_LOOP_BODY
            | Traits::NEVER_INLINE_BODY
            | Traits::LOOP_LIST_HEADER
            | Traits::WASM_EMITS_NOTHING,
        // `varList list ?varList list ...? body` — an odd count from 3
        // (n varList/list pairs, n >= 1, + 1 body — confirmed against
        // tclsh 8.6.14: `foreach a $l1 b $l2 body extra` (6 args) fails
        // "wrong # args").
        arity: Arity::stepped(3, Arity::UNLIMITED, 2),
        arg_role_resolver: Some(foreach_arg_roles),
        // Index 0 here is a fixed key, not a real source-position argument
        // index: the CFG builder lowers a `foreach` header to a synthetic
        // `Statement::Call` whose `args` are *only* the list arguments (one
        // per iterator group — the var-lists live in `defs`, not `args`; see
        // `cfg_builder::cfg_lower::lower_foreach`). Every list argument
        // expects the same List intrep (`$l` forces it via
        // `TclListObjGetElements`, exactly like `llength`'s operand), so
        // `shimmer::use_site::foreach_header_expected_type` reads this one
        // entry and applies it uniformly to every iterator group, including
        // the later ones a positional per-index table couldn't reach.
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
            },
        )],
        lowering_hook: Some(crate::hooks::LoweringHookId::Foreach),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Iterate over list elements with one or more loop variables.",
            synopsis: &[
                "foreach varList list ?varList list ...? body",
                "foreach varlist1 list1 ?varlist2 list2 ...? body",
            ],
            snippet: "Variables are assigned from list elements; `body` runs once per assignment group.",
            source: "Tcl foreach(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}

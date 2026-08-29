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
use tcl_dialect::model::{SpecSurface};
use tcl_dialect::surface;
use tcl_dialect::model::Family;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "foreach varlist1 list1 ?varlist2 list2 ...? body",
    ..FormSpec::DEFAULT
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
/// `?varlist list?...` repeats before the trailing body: the variable specs
/// sit at every other argument from 0, and the body — the last word — is
/// excluded.  The role resolver marks that body; this declares the repeating
/// head so no consumer has to re-derive the stride from the command's name
/// (issue #1185).
static REPEATED: &[RepeatedArgLayout] = &[RepeatedArgLayout {
    exclude_trailing: 1,
    ..RepeatedArgLayout::strided(ArgRole::LoopVarList, 0, 2)
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "foreach",
        surface: Some(surface![SpecSurface::core_in(Family::Tcl, &[("8.4", Some("9.2"))]), SpecSurface::core(Family::F5Irules)]),
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::HAS_LOOP_BODY
            | Traits::NEVER_INLINE_BODY
            | Traits::LOOP_LIST_HEADER,
        // `varList list ?varList list ...? body` — an odd count from 3
        // (n varList/list pairs, n >= 1, + 1 body — confirmed against
        // tclsh 8.6.14: `foreach a $l1 b $l2 body extra` (6 args) fails
        // "wrong # args").
        arity: Arity::stepped(3, Arity::UNLIMITED, 2),
        arg_role_resolver: Some(foreach_arg_roles),
        repeated_args: REPEATED,
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
                transparent_from: &[],
            },
        )],
        lowering_hook: Some(crate::hooks::LoweringHookId::Foreach),
        return_type: Some(TclType::String),
        var_write_typing: VarWriteTyping::ElementsOf { container_arg: 0 },
        hover: Some(HoverSnippet {
            summary: "Iterate over one or more lists, assigning loop variables from each.",
            synopsis: &[
                "foreach varname list body",
                "foreach varlist1 list1 ?varlist2 list2 ...? body",
            ],
            snippet: "In the simple form, varname takes on each value of list in turn and body runs once per value. In the general form, each varlist/list pair is handled independently: on every iteration, the variables of each varlist are assigned consecutive values from its corresponding list, as if by lindex. Iteration continues until every value from every list has been used exactly once — enough passes to exhaust the longest list — and a list too short for its varlist supplies empty strings for the missing elements on later passes. break and continue inside body behave exactly as they do in for. foreach itself always returns an empty string, regardless of what body does.",
            source: "Tcl foreach(n)",
            examples: "foreach x {a b c} {\n    puts $x\n}\nforeach {name value} {height 6 width 8} {\n    puts \"$name = $value\"\n}\nforeach x {1 2 3} y {a b} {\n    puts \"$x $y\"\n}",
            return_value: "An empty string.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Foreach),
        ..CommandSpec::DEFAULT
    }
}

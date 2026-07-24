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
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lmap varlist1 list1 ?varlist2 list2 ...? body",
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
        // `BYTE_COMPILED`: dedicated bytecode handling, not the generic
        // invoke path — the codegen loop-block emitter recognises `lmap`
        // exactly like `foreach` (`command == "foreach" || command ==
        // "lmap"` in `tcl_compiler::codegen::emitter::loop_blocks`),
        // stripping the body's trailing pop and emitting `LMAP_COLLECT` so
        // each iteration's result is gathered VM-side; aliasing the head to
        // `$cmd` would defeat that and change semantics, exactly the
        // hazard `BYTE_COMPILED` protects `foreach` from in the minifier.
        // `NOT_PROC_FACTORY`: the simple form (`lmap x {a b c} {body}`) is
        // a four-token `HEAD NAME ARGS BODY` call, incidentally identical
        // in shape to `foreach`'s simple form and to a tcllib-style
        // factory wrapper (`DEFC name args body`) — without this trait the
        // signature scanner would misread it as a candidate factory call.
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::HAS_LOOP_BODY
            | Traits::NEVER_INLINE_BODY
            | Traits::LOOP_LIST_HEADER,
        // `lmap` was added by TIP 267 for Tcl 8.6 — the 8.4 and 8.5
        // TclCmd/lmap.html manpage trees both return a plain 404 (no
        // lmap.n existed yet), while 8.6, 9.0, and 9.1 all serve the
        // page with byte-identical NAME/SYNOPSIS/DESCRIPTION text (only
        // the SEE ALSO cross-reference list grows in 9.0+, tracking the
        // newer list commands like `lseq`/`lremove`/`lpop`/`ledit` — not
        // a behavioural change to `lmap` itself).  `TCL86_PLUS` is
        // therefore exact, not merely a close fit.
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
            snippet: "In the simple form, varname takes on each value of list in turn and body runs once per value. In the general form, each varlist/list pair is handled independently: on every iteration, the variables of each varlist are assigned consecutive values from its corresponding list, as if by lindex. Iteration continues until every value from every list has been used exactly once — enough passes to exhaust the longest list — and a list too short for its varlist supplies empty strings for the missing elements on later passes. Unlike foreach, the result of each iteration that completes normally is appended, in order, to an accumulator list that lmap returns; break and continue behave exactly as they do in foreach, except that the iteration they interrupt contributes nothing to the accumulator — continue is therefore a common way to filter values out of the result.",
            source: "Tcl lmap(n)",
            examples: "lmap x {1 2 3} {expr {$x * 2}}\nlmap x {1 2 3 4 5 6} {\n    if {$x % 2} {continue}\n    set x\n}\nlmap a {1 2 3} b {x y z} {\n    list $a $b\n}",
            return_value: "The accumulator list: one element per iteration of body that completed normally (break/continue iterations contribute nothing).",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}

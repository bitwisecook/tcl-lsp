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
//! Present only in 9.1 — confirmed absent (404) on the 8.4, 8.5, 8.6, and 9.0
//! manual-page trees at <https://www.tcl-lang.org/man/>.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

// `lfilter` assigns to its loop variable(s) on every iteration it runs —
// Tcl lfilter(n): "lfilter assigns the contents of the element to varname"
// (simple form) / "the variables of each varlist are assigned consecutive
// values from the corresponding list" (general form). That write is real
// but not unconditional: if every value list is empty, `EachloopCmd`
// (`generic/tclCmdAH.c`) never enters the loop body at all, so nothing is
// assigned — `writes: true` is the sound over-approximation for a fact a
// static spec can't see per call site. No named target is known statically,
// hence `Unknown`.
//
// `reads: true` too, matching every other Eachloop-family/opaque-body
// control construct in this registry that declares an `Unknown` target:
// `foreach`/`lmap` (`generic/tclCmdAH.c`'s `TclNRForeachCmd`/
// `TclNRLmapCmd`/`TclNRLfilterCmd` all delegate to the very same
// `EachloopCmd`, differing only in the `TCL_EACH_*` mode constant they
// pass — `TCL_EACH_FILTER` here), plus `for`/`while`/`if`/`switch`/`try`/
// `catch`/`subst` all declare `(reads: true, writes: true)` for this exact
// situation; none in this codebase pairs `Unknown` with `writes: true,
// reads: false` for a control construct (the two counterexamples,
// `json::create`/`json::set`, are pure value constructors with no loop
// body, not the same shape). `lfilter` also, on every iteration, reads an
// element out of each value list — state of unknown static identity, the
// same reason `foreach`/`lmap` read. Dropping `reads` to `false` would
// mean `to_effect_regions()` (`tcl_compiler::side_effects::
// target_to_region`, whose wildcard arm maps `Unknown` to
// `EffectRegion::UNKNOWN_STATE`, not `NONE`) omits `UNKNOWN_STATE` from
// this call's read set entirely — an unsound under-approximation for the
// same reason `writes: false` was. The predicate/body's own effects are
// tracked separately by recursing into its `Body` arg role.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

// Both forms are documented side by side in `doc/lfilter.n`'s SYNOPSIS (and
// in that order): the single-variable/single-list shorthand, then the
// general `foreach`-style multi-list form. The single-variable form is
// structurally just the n=1 case of the general grammar (a bare varname is
// a one-element varlist), so both share the same `arity` below.
const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "lfilter varname list expression",
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "lfilter varlist1 list1 ?varlist2 list2 ...? body",
        ..FormSpec::DEFAULT
    },
];

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
            | Traits::LOOP_LIST_HEADER
            // `generic/tclBasic.c`'s command table wires `lfilter` to
            // `TclCompileLfilterCmd` (`generic/tclCompCmds.c`), a genuine
            // per-command inline-compile proc mirroring `foreach`/`lmap`'s
            // `CompileEachloopCmd` — a real bytecode-compiler special case,
            // not just an "otherwise load-bearing" builtin.
            | Traits::BYTE_COMPILED
            // `lfilter x {1 2 3} {puts $x}` matches the signature scanner's
            // `HEAD NAME BRACED BRACED` four-token factory-candidate shape
            // (`tcl_compiler::signature_scan::handlers::
            // maybe_record_factory_candidate`) exactly like `foreach`/`for`/
            // `if`, so it needs the same skip to avoid being misread as a
            // tcllib-style `proc`-factory wrapper call.
            | Traits::NOT_PROC_FACTORY,
        surface: Some(SpecSurface::TCL91),
        // `varList list ?varList list ...? command` — identical grammar to
        // `foreach`/`lmap`: n varList/list pairs (n >= 1) plus one command
        // body, so a valid argument count (after the command name) is odd
        // and >= 3; an even count is `wrong # args`. Confirmed against C Tcl
        // 9.1b0's shared `EachloopCmd` (`generic/tclCmdAH.c`): `if (objc < 4
        // || (objc%2 != 0)) { Tcl_WrongNumArgs(interp, 1, objv, "varList
        // list ?varList list ...? command"); ... }`, where `objc` counts the
        // command name too — and the same `numWords < 4 || (numWords%2 !=
        // 0)` minimum/parity condition (alongside an unrelated unsigned-
        // range guard) gates `TclCompileLfilterCmd`'s inline-compile path
        // in `generic/tclCompCmds.c`. Previously `at_least(3)`, which
        // missed this odd/even parity.
        arity: Arity::stepped(3, Arity::UNLIMITED, 2),
        arg_role_resolver: Some(lfilter_arg_roles),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Select elements from a list based on an expression.",
            synopsis: &[
                "lfilter varname list expression",
                "lfilter varlist1 list1 ?varlist2 list2 ...? body",
            ],
            snippet: "In the simple form, lfilter assigns each element of list to varname in turn (as if by lindex) and evaluates the boolean expression. Whenever the expression completes normally and is true — any of Tcl's boolean spellings (1/0, true/false, yes/no, on/off) — that element is appended to the result. In the general form, each varlist/list pair is handled independently: every iteration assigns the variables of each varlist consecutive values from its corresponding list, running enough iterations to exhaust the longest list and supplying empty strings once a shorter list runs out. Only the value assigned to the first variable of the first varlist is ever appended, regardless of how many lists take part — the rest exist purely to help compute the predicate. break and continue inside the body behave exactly as in for and foreach; either one, or a false result, simply skips that element.",
            source: "Tcl lfilter(n)",
            examples: "lfilter x {0 2 4 1 3 5 10 8 6 11 9 7} {\n    expr {($x % 2) == 1}\n}\n# Result: 1 3 5 11 9 7\n\nlfilter x [lseq 10] y [lseq 0 .. 1 by 0.1] {\n    expr {$y > 0.3 && $y < 0.7}\n}\n# Result: 4 5 6",
            return_value: "The elements of the first value list for which the predicate evaluated true (a possibly empty list).",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}

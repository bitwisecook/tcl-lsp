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

//! `lrepeat` — build a list by repeating elements.
//
// Manpage comparison across Tcl 8.4, 8.5, 8.6, 9.0, and 9.1
// (tcl-lang.org's TclCmd/lrepeat.html, `.htm` for the 8.6 tree): no
// lrepeat.n page exists for 8.4 on either extension (a genuine 404
// error page, not the broken-redirect gotcha), and the 8.4.20 source
// tree's generic/tclCmdIL.c has no Tcl_LrepeatObjCmd at all — the
// command was added in Tcl 8.5 (TIP 136), matching the
// `surface: Some(SpecSurface::TCL85_PLUS)` gate below.
//
// Tcl 8.5's own synopsis and C implementation (8.5.19
// generic/tclCmdIL.c) are stricter than every later release: the
// synopsis requires at least one `element` ("lrepeat number element1
// ?element2 element3 ...?", `objc < 3` is a wrong-#-args error) and
// `number` must be a *positive* integer — `elementCount < 1` errors
// "must have a count of at least 1", so both a bare `lrepeat 3` and
// `lrepeat 0 x` fail on 8.5. Tcl 8.6 through 9.1 loosen both
// restrictions together: the synopsis becomes "lrepeat count
// ?element ...?" (`objc < 2` is the only arity floor left, so
// `element` is fully optional and a bare `lrepeat 3` legally returns
// the empty list), and `count` need only be *non-negative* — a
// negative count still errors ("bad count \"N\": must be integer >=
// 0", error code TCL OPERATION LREPEAT NEGARG in the 8.6.16 and
// 9.0.4 sources) but 0 is accepted. Tcl 9.1's Tcl_LrepeatObjCmd keeps
// the same "count ?value ...?" arity floor and delegates the
// count/element work to a new public C API, Tcl_ListObjRepeat
// (tcl.decls stub 693) — a refactor exposing the operation to C
// extensions, with no evidence of a scripting-visible behaviour
// change from 9.0 (its error-message text was not independently
// located in the 9.1b0 source tree, since Tcl_ListObjRepeat's own
// definition lives outside generic/tclListObj.c and
// generic/tclCmdIL.c). This codebase's own interpreter
// (tcl-vm/src/cmd_list.rs::cmd_lrepeat + tcl-cmd-core/src/list.rs::
// lrepeat) already matches the 8.6+ shape — count alone is legal,
// negative errors, 0 is fine — consistent with
// `command_version_gates_fire_w123` in
// tcl-compiler/src/analyser/diagnostics/tests.rs, which asserts
// `lrepeat 3 x` is unknown (W123) under tcl8.4 and known under
// tcl8.5.
//
// Every release from 8.5 onward documents the same equivalence,
// "lrepeat 1 element ... is identical to list element ...", and the
// same worked examples (`lrepeat 3 a`, `lrepeat 3 a b c`, and the two
// nested-lrepeat examples); the 9.0/9.1 SEE ALSO list simply grows
// (lassign, ledit, lindex, linsert, llength, lmap, lpop, lrange,
// lremove, lreplace, lreverse, lsearch, lseq, lset, lsort) because
// those sibling commands did not exist yet in 8.5/8.6 — cross-
// reference churn, not a change to lrepeat itself.
//
// Not disabled or overridden in any modelled dialect: there is no
// disable list for it to appear in (a sandbox-banned command would
// instead carry a bare `ALL_TCL` group lacking the `IRULES` bit —
// `lrepeat` is version-gated, not banned), and no
// irules/iapps/tk/expect/eda*/itcl spec file names `lrepeat` (grepped
// every sibling dialect directory under commands/). iRules' own
// runtime is a genuine embedded Tcl 8.4.6
// (`DialectSet::expr_grammar_base_version`), which predates TIP 136,
// so `lrepeat` is correctly unreachable there simply because the
// `IRULES` bit is absent from `TCL85_PLUS` — no extra dialect
// restriction is needed on top of the version gate already in place.
// F5 iApps/tmsh and the Xilinx/Quartus/Mentor EDA shells host a real
// embedded Tcl 8.5 core, and Synopsys/Cadence/Expect host 8.6, so all
// of them already resolve `lrepeat` through the same `TCL85_PLUS`
// gate via their additive vendor-bit composition — no dialect adds or
// removes a form or option here.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "lrepeat count ?element ...?",
        surface: Some(SpecSurface::TCL86_PLUS),
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "lrepeat count element ?element ...?",
        surface: Some(SpecSurface::TCL85),
        ..FormSpec::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lrepeat",
        const_fold: Some(crate::const_fold::fold_lrepeat),
        // `lrepeat count {a b} {c d}` — a call with exactly two element
        // arguments, both braced — is a `HEAD NAME BRACED BRACED`
        // invocation shape, so `NOT_PROC_FACTORY` keeps the proc-factory
        // signature scanner from mistaking it for a `proc`-defining
        // wrapper call, the same reason `list`/`lindex`/`lappend`/
        // `lrange`/`ledit` carry it. The native VM builtin reads its
        // arguments and builds a list with no eval fallback, opens no
        // new call frame, and never invokes a user-supplied callback, so
        // `FRAMELESS_RUNTIME` holds; it is also a deterministic,
        // side-effect-free function of its arguments (`PURE`).
        traits: Traits::FRAMELESS_RUNTIME | Traits::PURE | Traits::NOT_PROC_FACTORY,
        // Added in Tcl 8.5 (TIP 136); absent from 8.4 and never
        // available in iRules (a genuine embedded Tcl 8.4.6) — see the
        // module comment above for the full version/dialect cross-check.
        surface: Some(SpecSurface::TCL85_PLUS),
        // The 8.6+ minimum (count alone, no elements). `Arity` has no
        // per-dialect axis, so this is a narrow gap for arity
        // diagnostics against code targeting Tcl 8.5 specifically,
        // where at least one element is required — see the two FORMS
        // entries below and the module comment.
        arity: Arity::at_least(1),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet {
            summary: "Build a list by repeating elements",
            synopsis: &[
                "lrepeat count ?element ...?",
                "lrepeat count element ?element ...?",
            ],
            snippet: "Creates a list of size count times the number of element arguments, by repeating the sequence element ... count times in order — lrepeat 1 element ... is identical to list element .... count must be an integer, and a negative count is always an error. From Tcl 8.6 onward, element is optional and count may be 0, so a bare lrepeat count with no elements at all is legal and returns the empty list. In Tcl 8.5, at least one element is required and count must be a positive integer (at least 1) — both a bare lrepeat count and lrepeat 0 element are errors there.",
            source: "Tcl man page lrepeat.n",
            examples: "lrepeat 3 a\nlrepeat 3 a b c\nlrepeat 3 [lrepeat 2 a] b c",
            return_value: "A list of size count times the number of element arguments — the element sequence repeated count times in order, or the empty list when count is 0 (Tcl 8.6 and later).",
        }),
        forms: FORMS,
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..CommandSpec::DEFAULT
    }
}

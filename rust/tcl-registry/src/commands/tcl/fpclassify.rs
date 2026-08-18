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

//! `fpclassify` — classify a floating-point value (Tcl 9.0+).
//
// Release span, verified against the shipped source trees rather than
// paraphrased: `doc/fpclassify.n` exists in tcl9.0.4 and tcl9.1b0 and is
// byte-identical between them (`diff` reports no difference), while
// tcl8.4.20, tcl8.5.19, and tcl8.6.16 ship no such page and no `fpclassify`
// entry in their command tables.  The 9.x page's own SYNOPSIS states
// `package require tcl 9.0`.  Confirmed at runtime: tclsh 9.0.4 and 9.1b0
// both answer `fpclassify 1e-320` -> `subnormal`, whereas tclsh 8.6.14
// raises `invalid command name "fpclassify"`.
//
// Registered in the C core's builtin table as an ordinary object command
// (`{"fpclassify", FloatClassifyObjCmd, NULL, NULL, CMD_IS_SAFE}` in
// `generic/tclBasic.c`), so it is safe-interpreter-visible and is *not* an
// `expr` math function — the classification is exposed only as a command
// (`mathfunc(n)` merely cross-references it).
//
// No modelled dialect composes a base Tcl core of 9.0 or later — iRules is an
// embedded Tcl 8.4.6, and the iApps / Tk / Expect / EDA / itcl profiles top
// out at 8.6 — so the plain `TCL90_PLUS` gate below already excludes every one
// of them and no extra dialect restriction is needed.
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "fpclassify value",
    ..FormSpec::DEFAULT
}];

/// Command spec for `fpclassify`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fpclassify",
        // Added in Tcl 9.0 (`package require tcl 9.0` in its own SYNOPSIS)
        // and unchanged in 9.1 — see the module comment.
        dialects: Some(DialectSet::TCL90_PLUS),
        // A deterministic classification of its single argument with no
        // interpreter state read or written, and no expression / script
        // fallback for the argument (`FloatClassifyObjCmd` takes the double
        // directly), so it is genuinely pure and safe to CSE. Not
        // byte-compiled: the C core registers it as a plain `ObjCmd` with no
        // compile proc. `FRAMELESS_RUNTIME` is deliberately absent — that
        // trait is an audited claim about this workspace's own VM dispatch
        // (`registry::tests::frameless_runtime_covers_the_audited_allow_list`),
        // and `tcl-vm` has no `fpclassify` implementation to make it about.
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        // `wrong # args: should be "fpclassify floatValue"` for both zero and
        // two arguments on tclsh 9.0.4 and 9.1b0 — exactly one word.
        arity: Arity::exact(1),
        return_type: Some(TclType::String),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Double),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        hover: Some(HoverSnippet {
            summary: "Classify a floating-point value.",
            synopsis: &["fpclassify value"],
            snippet: "Returns one of five strings describing value: `zero` for a floating-point zero, `subnormal` for the result of a gradual underflow (representable but with lost precision), `normal` for an ordinary floating-point number, `infinite` for an infinity, and `nan` for Not-a-Number. An integer is classified as the double it converts to, so `fpclassify 5` is `normal`. A value that is not a floating-point number and cannot be converted to one raises an error rather than returning a classification. This is the command form only — `fpclassify` is not an `expr` math function.",
            source: "Tcl fpclassify(n)",
            examples: "fpclassify 0.0       ;# zero\nfpclassify 1.0       ;# normal\nfpclassify 1e-320    ;# subnormal -- precision has been lost\nfpclassify Inf       ;# infinite\nfpclassify NaN       ;# nan\n\n# Guard a computed result against representational problems\nswitch [fpclassify $value] {\n    normal - zero { puts \"Result is $value\" }\n    subnormal     { puts \"Result is $value - WARNING! precision lost\" }\n    infinite      { puts \"Result is infinite\" }\n    nan           { puts \"Computation completely failed\" }\n}",
            return_value: "One of `zero`, `subnormal`, `normal`, `infinite`, or `nan`. Raises an error when value is not convertible to a floating-point number.",
        }),
        forms: FORMS,
        ..CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT
    }
}

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

//! `set` — read or write a variable.

// `set`'s SYNOPSIS, DESCRIPTION, EXAMPLES, SEE ALSO, and KEYWORDS are
// byte-for-byte identical across Tcl 8.4, 8.5, 8.6, 9.0, and 9.1 (fetched
// and diffed all five set.html/set.htm pages directly — 9.0's and 9.1's
// are content-identical; 8.6's is content-identical to 9.0/9.1 too, modulo
// the page-chrome breadcrumb. 8.4 differs from 8.5+ in two purely cosmetic
// spots, neither a `set` behaviour change: the EXAMPLES's `[expr rand()]`
// vs. the now-recommended braced `[expr {rand()}]`, and DESCRIPTION's
// "doesn't already exist" vs. "does not already exist" contraction
// expansion). No option, form, or argument was ever added, removed, or
// altered for this command; it has taken zero flags in any of these five
// releases.
//
// One real behavioural delta exists that the manpage documents only by
// reference (it defers to namespace(n)'s NAME RESOLUTION rules, which
// changed under TIP 278): outside a procedure — or with a namespace-
// qualified name — an unqualified varName at namespace scope falls back
// to a same-named variable in the global namespace when the current
// namespace has none, through Tcl 8.6; Tcl 9.0 removed that fallback
// (`tclVar.c`'s `TclLookupSimpleVar` now forces `TCL_NAMESPACE_ONLY`).
// See `DialectProfile::namespace_var_global_fallback` for the dialect-keyed
// version of this same fact, and the hover snippet below for the
// developer-facing wording.

use crate::hooks::LoweringHookId;
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

// `value` (not `newValue`) is the parameter name in every fetched
// set.html/set.htm SYNOPSIS, 8.4 through 9.1 alike. This deliberately
// diverges from `tcl-vm`'s own `cmd_set` wrong-#-args text
// (`"wrong # args: should be \"set varName ?newValue?\""`,
// `tcl-vm/src/command.rs`) and from this fact's write-up in
// `tcl-compiler/tests/analyser.rs`'s `too_few_args_set_fires_e002` — both
// reproduce a genuine, long-standing real-`tclsh` quirk where the C
// `Tcl_SetObjCmd` usage string says `newValue` even though `doc/set.n`
// has always said `value`; the two have simply never been reconciled
// upstream. This field documents the manpage wording (also surfaced by
// the compiler's E002/E003 "usage: …" hint via `primary_synopsis`), not
// the runtime error string; `tcl-vm`'s hardcoded message is unaffected by
// this file and still prints `newValue` verbatim, matching real tclsh.
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "set varName ?value?",
    ..FormSpec::DEFAULT
}];

/// Dynamic arg role resolver: getter (1 arg) vs setter (2 args).
fn set_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() >= 2 {
        vec![(0, ArgRole::VarWrite)]
    } else if args.len() == 1 {
        vec![(0, ArgRole::VarRead)]
    } else {
        Vec::new()
    }
}

/// Command spec for `set`.
///
/// Every documented aspect of `set` — SYNOPSIS, options (there are none),
/// DESCRIPTION, and EXAMPLES — is unchanged from Tcl 8.4 through 9.1; see
/// the module-level comment above for the full per-version cross-check
/// and the one real (if namespace-rules-deferred) behavioural delta.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set",
        // A core variable primitive with no filesystem/process/network access,
        // present unmodified in every dialect that hosts a real Tcl core
        // (irules, iapps, tmsh, the EDA shells, expect, tk, itcl) — its
        // surface carries an iRules row explicitly (`ALL_TCL.union(IRULES)`),
        // so it resolves under the iRules point, and no dialect grants it
        // extra options or an alternate form: every `"set"` hit under the
        // irules/, expect/, iapps/, tk/, itcl/, and eda_*/ command packs is an
        // unrelated subcommand of a different ensemble (`array set`, `dict
        // set`, a Tk widget's `pathName set`, iRules' `table set`), never a
        // redefinition of this command.
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::FIRST_ARG_VARNAME,
        arity: Arity::new(1, 2),
        arg_role_resolver: Some(set_arg_roles),
        assigns_variable_at: Some(0),
        // `set NAME [TYPE inst …]` — when the value word is a construction,
        // `NAME` ends up holding an object handle.  Registry data so the
        // object-handle scan resolves the layout for `::set` and a provable
        // static alias/rename of `set` exactly as for the bare spelling,
        // instead of matching the word `set` (issue #1185).
        binds_handle: Some(&crate::handle_binding::SET_BINDS_HANDLE),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Read the value of a variable, or set it to a new value.",
            synopsis: &["set varName ?value?"],
            snippet: "With one argument, returns varName's current value — an error if it has never been set. With two, assigns value to varName, creating the variable if it doesn't already exist, and returns that same value. varName names an array element when written as arrayName(index); the index may itself come from variable or command substitution. varName can also be produced by substitution itself, so `set out [set $vbl]` reads whichever variable vbl currently names — Tcl's own documentation notes arrays are usually the cleaner alternative to this kind of double-dereferencing. Scope resolution depends on context: inside a procedure, an unqualified varName is a local or parameter unless previously declared via global, variable, or upvar; outside a procedure, or when varName is namespace-qualified, it resolves as a namespace variable instead. At namespace scope, Tcl 8.4 through 8.6 fall back to a same-named variable in the global namespace (never an intermediate enclosing namespace) when the current namespace has none; Tcl 9.0 and later require an exact match in the current namespace, with no global fallback.",
            source: "Tcl set(n)",
            examples: "set r [expr {rand()}]\nset anAry(msg) \"Hello, World!\"\n\n# Array element addressed by a variable-held index\nset elemName \"msg\"\nset anAry($elemName) \"Hello, World!\"\n\n# Indirection: read the variable named by the current value of vbl\nset in0 \"small random\"\nset in1 \"large random\"\nset vbl in[expr {rand() >= 0.5}]\nset out [set $vbl]",
            return_value: "varName's new value when value is given, otherwise its current value.",
        }),
        lowering_hook: Some(LoweringHookId::Set),
        native_lowering: Some(NativeLowering::Structured(LoweringHookId::Set)),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Set),
        ..CommandSpec::DEFAULT
    }
}

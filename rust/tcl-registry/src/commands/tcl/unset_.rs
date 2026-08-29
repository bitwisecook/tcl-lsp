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

//! `unset` — delete variables.

// `unset`'s SYNOPSIS is byte-for-byte identical across Tcl 8.4, 8.5, 8.6,
// 9.0, and 9.1 (fetched and diffed all five unset.html/unset.htm pages
// directly): `unset ?-nocomplain? ?--? ?name name name ...?` in every one,
// including 8.4 — `-nocomplain` and `--` have never been version-gated for
// this command (this file previously claimed `-nocomplain` was added in
// 8.5; that was wrong). DESCRIPTION is identical in substance across all
// five, with two purely cosmetic wording differences that are not
// behaviour changes: 8.4/8.5 say "If an error occurs, any variables …"
// where 8.6/9.0/9.1 say "If an error occurs during variable deletion, any
// variables …", and 8.4 says "doesn't exist" where 8.5 through 9.1 say
// "does not exist". EXAMPLE, SEE ALSO, and KEYWORDS are byte-for-byte
// identical across all five. No option, form, or argument was ever added,
// removed, or altered for this command in any of these five releases.
//
// Two behavioural facts the manpage states only loosely, confirmed by
// direct testing against a real tclsh 8.6.14 (`Tcl_UnsetObjCmd`,
// generic/tclCmdMZ.c) and consistent with the SYNOPSIS's own grammar —
// `-nocomplain` and `--` each appear without the `...` repeat marker
// `name` carries, so the manpage itself implies "at most once, in a fixed
// position" rather than "anywhere, repeatably":
//   - `-nocomplain` and `--` are recognised *only* in a fixed leading
//     position: `-nocomplain` as argument 0, and `--` as argument 0 (with
//     no preceding `-nocomplain`) or argument 1 (immediately after
//     `-nocomplain`). Neither is recognised anywhere else, and neither
//     repeats — a second `-nocomplain`, or a `--` outside those two
//     positions, is a literal variable name that `unset` genuinely
//     attempts to delete.
//   - Without `-nocomplain`, a failing name aborts the command immediately:
//     every name still to come is left untouched (names already deleted
//     before the failure stay deleted). With `-nocomplain`, each error is
//     instead suppressed individually and processing continues through the
//     rest of the list.
//
// One further behavioural fact the manpage only implies (via its "refers
// to a variable in a non-existent namespace" error case): `unset` shares
// `set`'s TIP 278 namespace/global-variable lookup (`tclVar.c`'s
// `TclLookupSimpleVar`, also used by `incr` / `info exists` — see
// `DialectSet::namespace_var_global_fallback`). Outside a procedure, an
// unqualified name not found in the current namespace falls back to a
// same-named variable in the global namespace through Tcl 8.6; Tcl 9.0
// removed that fallback, requiring an exact match in the current
// namespace. See the hover snippet below for the developer-facing wording.

use crate::hooks::{CodegenHookId, LoweringHookId};
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "unset ?-nocomplain? ?--? ?name name name ...?",
    ..FormSpec::DEFAULT
}];

/// `unset ?-nocomplain? ?--? ?name name name ...?` — every trailing word is a
/// variable name, not just the first.  Resolve `VarWrite` dynamically (skipping
/// the leading options) so all names highlight as variables rather than only
/// the first (issue #774).
///
/// Mirrors `Tcl_UnsetObjCmd` (generic/tclCmdMZ.c) — *not* the broader,
/// currently-incorrect leading-option loop in `tcl-compiler`'s `lower_unset`
/// (`lowering_hooks.rs`), which still treats `-nocomplain` as skippable and
/// repeatable at every position. Real Tcl recognises at most one leading
/// `-nocomplain` (argument 0), and only then an optional `--` immediately
/// after it (argument 1); a bare `--` with no preceding `-nocomplain` is also
/// recognised, but only at argument 0. Neither word is recognised anywhere
/// else, and neither repeats — confirmed empirically against tclsh 8.6.14: a
/// second `-nocomplain`, or a `--` in any other position, is a real variable
/// name (e.g. one literally called `-nocomplain`) and keeps its `VarWrite`
/// role like any other. Any other leading word — including one that begins
/// with `-`, such as `unset -foo bar` — is a real variable name too, never an
/// option.
fn unset_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let i = match args {
        ["-nocomplain", "--", ..] => 2,
        ["-nocomplain" | "--", ..] => 1,
        _ => 0,
    };
    (i..args.len())
        .filter_map(|j| u8::try_from(j).ok().map(|j| (j, ArgRole::VarWrite)))
        .collect()
}

/// Command spec for `unset`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "unset",
        // A core variable primitive with no filesystem/process/network
        // access, present unmodified in every dialect that hosts a real Tcl
        // core (irules, iapps, tmsh, the EDA shells, expect, tk, itcl) —
        // iRules enables it, so it carries the `IRULES` bit explicitly
        // (`ALL_TCL.union(IRULES)`) and resolves under the bare `IRULES`
        // mask, and no dialect grants it extra options or an alternate
        // form: every `"unset"` hit under the irules/, expect/, iapps/,
        // tk/, itcl/, and eda_*/ command packs is an unrelated subcommand
        // of a different ensemble (`array unset`, `dict unset`) or a
        // `trace` operation name, never a redefinition of this command.
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        // `FIRE_AND_FORGET_TEARDOWN`: `Tcl_UnsetObjCmd` (tclCmdMZ.c) removes the
        // variable and errors ("can't unset …: no such variable") when the
        // target is already gone (absent `-nocomplain`) — the property the
        // W302 fire-and-forget suppression (`catch {unset x}`) keys off.
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::DESTROYS_VARIABLE
            | Traits::FIRST_ARG_VARNAME
            | Traits::FIRE_AND_FORGET_TEARDOWN,
        // Names are optional: `unset`, `unset -nocomplain`, and `unset --`
        // are all valid no-ops in every fetched manpage version, 8.4 through
        // 9.1 alike (see the module-level comment above) — `-nocomplain` has
        // never been version-gated for this command. The "consumed all args,
        // unset nothing" footgun is surfaced by W217, not an arity error.
        arity: Arity::at_least(0),
        arg_role_resolver: Some(unset_arg_roles),
        // NOTE: `unset` does NOT set `assigns_variable_at`. That field means
        // "arg N is assigned a value" (the `set`/`incr` shape); `unset`
        // *destroys* variables, which is modelled by the `DESTROYS_VARIABLE`
        // trait plus the `VarWrite` roles from `arg_role_resolver`. Declaring
        // `assigns_variable_at: Some(0)` both contradicts the arg-role resolver
        // (arg 0 may be `-nocomplain`) and pre-empted the destroy branch in the
        // side-effect classifier, modelling `unset x` as a *write* to `x` and
        // `unset -nocomplain x` as a write to `-nocomplain`.
        return_type: Some(TclType::String),
        options: const {
            &[
                OptionSpec {
                    name: "-nocomplain",
                    value: OptionValue::flag(),
                    detail: "Suppress every error unset would otherwise raise for each name that follows — a missing variable, an array-element reference against a scalar, or a nonexistent namespace — and keep processing the remaining names. Must be the literal first argument, spelled out in full; never abbreviated, so it can't be confused with a variable name that happens to start the same way.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "End option parsing, so a following word is always treated as a variable name even if it looks like an option. Recognised only as the first argument, or as the second when it immediately follows -nocomplain — anywhere else -- is itself just another name to delete.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Delete one or more variables.",
            synopsis: &["unset ?-nocomplain? ?--? ?name name name ...?"],
            snippet: "Deletes each name in turn, left to right. name may be a scalar variable, an array element written arrayName(index), or a bare array name with no index, which deletes the whole array; every name accepts the same forms set does, including a namespace-qualified name. Deleting one name can fail — it doesn't exist, it names an array element but the variable is actually a scalar, or its namespace doesn't exist — and by default that error stops the command immediately: the failing name and everything still to come is left alone, though names already deleted before the failure stay deleted. -nocomplain, given as the very first argument, suppresses every one of those errors instead and keeps going through the rest of the list; it must be spelled out in full, never abbreviated, so it can't be confused with a variable name that happens to start the same way. --, recognised only as the first argument (or as the second, immediately after -nocomplain), marks the end of option parsing — needed to delete a variable whose own name would otherwise look like an option, such as one literally called -nocomplain. A variable carrying an unset trace runs that trace immediately before removal; an error the trace itself raises is ignored. Outside a procedure, an unqualified name not found in the current namespace falls back to a same-named variable in the global namespace through Tcl 8.6; Tcl 9.0 and later require an exact match in the current namespace, with no such fallback.",
            source: "Tcl unset(n)",
            examples: "set x 1\nset y 2\nunset x y                   ;# delete two scalars at once\n\narray set opts {debug 1 verbose 0}\nunset opts(verbose)         ;# delete a single array element\nunset opts                  ;# delete the whole array\n\nunset -nocomplain maybeSet  ;# no error even if maybeSet was never set\nunset -- -flag              ;# delete a variable literally named \"-flag\"",
            return_value: "An empty string, always.",
        }),
        lowering_hook: Some(LoweringHookId::Unset),
        codegen_hook: Some(CodegenHookId::Unset),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

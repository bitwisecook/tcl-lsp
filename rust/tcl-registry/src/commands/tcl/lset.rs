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

//! `lset` — change an element in a list variable.
//
// Cross-checked against the tcl-lang.org lset(n) manpages for Tcl 8.4,
// 8.5, 8.6, 9.0, and 9.1 (independently re-fetched and re-verified,
// verifier pass, against raw manpage HTML rather than a prior summary).
// Forms and arity are identical across that range; two facts are
// version-gated:
//
// - index-equals-length: 8.4/8.5 raise "list index out of range" for an
//   index equal to the target sublist's length ("negative or greater
//   than or equal to" -> error, "strictly less than" required); 8.6+
//   appends `newValue` there instead ("negative or greater than" ->
//   error, "equal to" -> append, "less than or equal to" required).
//   Note: the tcl8.6 manpage's own EXAMPLES section still prints the
//   pre-8.6 error for `lset x {2 3} j` even though its DESCRIPTION
//   paragraph on the very same page states the new append rule — an
//   upstream doc staleness fixed in the 9.0/9.1 manpage's EXAMPLES
//   (which show `{2 3}` appending and add a new `{2 4}` example for the
//   real boundary). This file follows the DESCRIPTION prose plus the
//   corrected 9.0/9.1 example, not the stale 8.6 one — confirmed against
//   the raw HTML of all five pages, not just an AI summary of them.
// - index arithmetic: 8.4 accepts only a bare integer, `end`, or
//   `end-integer`; 8.5+ adds the fuller `string index` arithmetic (e.g.
//   `end-1+1`), per the manpage's own cross-reference to `string index`
//   (itself new to lset's SEE ALSO list starting in 8.5).
//
// See `hover.snippet` below for the exact wording surfaced to users.
use crate::forms::CommandForm;
use crate::hooks::CodegenHookId;
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "lset varName ?index ...? newValue",
    ..FormSpec::DEFAULT
}];

/// `lset varName newValue` — replace the entire list (no index). A
/// single empty-list index (`lset varName {} newValue`, 3 args at
/// runtime) is an equivalent spelling of this same whole-list
/// replacement, but arity-wise it matches `LSET_SINGLE_INDEX` below, not
/// this form.
const LSET_REPLACE: CommandForm = CommandForm {
    name: "replace",
    arity: Arity::exact(2),
    arg_roles: &[(0, ArgRole::VarWrite)],
    codegen_hook: Some(CodegenHookId::Lset),
    ..CommandForm::DEFAULT
};

/// `lset varName index newValue` — single-level update. `index` may equal
/// the list's length: Tcl 8.6+ appends `newValue` there, while 8.4/8.5
/// raise "list index out of range" (see the command's `hover.snippet`).
const LSET_SINGLE_INDEX: CommandForm = CommandForm {
    name: "single_index",
    arity: Arity::exact(3),
    arg_roles: &[(0, ArgRole::VarWrite)],
    codegen_hook: Some(CodegenHookId::Lset),
    ..CommandForm::DEFAULT
};

/// `lset varName index1 ?index2 ...? newValue` — multi-level path; each
/// index after the first addresses an element of the sublist the
/// previous index selected. The same length-equal append rule (Tcl
/// 8.6+; see `LSET_SINGLE_INDEX` above) applies to the final index in
/// the chain.
const LSET_FLAT_PATH: CommandForm = CommandForm {
    name: "flat_path",
    arity: Arity::at_least(4),
    arg_roles: &[(0, ArgRole::VarWrite)],
    codegen_hook: Some(CodegenHookId::Lset),
    ..CommandForm::DEFAULT
};

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lset",
        // `lset` reads the list's current value before rewriting one element,
        // and — like `set`/`append`/`lappend`/`incr` — its first argument is
        // a variable *name*, so it joins the name-first set the write-command
        // consumers (bounds checks, dead-store cancellation, minifier RMW
        // protection) query via `FIRST_ARG_VARNAME`. `BYTE_COMPILED`: the
        // compiler special-cases the common shapes with its own codegen
        // (`codegen_hook` below; see `tcl-vm/src/cmd_list.rs::cmd_lset`'s
        // doc comment for the compiled-vs-fallback split), so the minifier
        // must not rewrite this head to a `$var` alias. `NOT_PROC_FACTORY`:
        // the 3-arg `lset varName index newValue` form is exactly the
        // proc-factory scanner's four-token `HEAD NAME ARGS BODY` shape
        // (`tcl-compiler/src/signature_scan/handlers.rs::
        // maybe_record_factory_candidate`) whenever `newValue` is a braced
        // literal, e.g. `lset mylist 0 {hello there}` — without this trait
        // that ordinary call would be misread as a factory-wrapper
        // definition. `lset` deliberately stays off `FRAMELESS_RUNTIME`
        // (see the allow-list comment in `registry.rs`): its multi-level
        // descent has no frameless codegen path today.
        traits: Traits::FRAME_HASH_BUILTIN
            .union(Traits::READS_BEFORE_WRITE)
            .union(Traits::FIRST_ARG_VARNAME)
            .union(Traits::BYTE_COMPILED)
            .union(Traits::NOT_PROC_FACTORY),
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        arity: Arity::at_least(2),
        arg_roles: &[(0, ArgRole::VarWrite)],
        assigns_variable_at: Some(0),
        return_type: Some(TclType::List),
        representation_effect: Some(RepresentationEffect::copy_on_write_container(0, 3)),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet {
            summary: "Change an element (or nested element) in a list stored in a variable.",
            synopsis: &["lset varName ?index ...? newValue"],
            snippet: "varName must already name a variable holding a valid Tcl list — unlike set, lset cannot create the variable, so an undefined varName raises \"can't read ...\" even when no index is given. With no index, or a single empty-list index ({}), newValue replaces the variable's entire contents, the same as set varName newValue. With one index, lset addresses that 0-based element of the list. index accepts a non-negative or negative integer, end, end-integer, or — since Tcl 8.5 — the fuller index-arithmetic syntax of `string index`, such as end-1+1; Tcl 8.4 supports only a bare integer or the fixed end-integer form. Two or more indices descend into nested sublists, each addressing an element of the sublist the previous index selected — lset a 1 2 v is equivalent to lset a [list 1 2] v — and the indices may be given as separate words or grouped into a single list argument, e.g. lset x {2 1} v. Every index must be zero or greater; a negative index, or one greater than the target sublist's length, raises \"list index out of range\". An index equal to the target sublist's current length appends newValue as a new element — but only from Tcl 8.6 onward; Tcl 8.4 and 8.5 treat an equal-to-length index the same as one that is too large.",
            source: "Tcl lset(n)",
            examples: "set x {{a b c} {d e f} {g h i}}\nlset x 0 j             ;# j {d e f} {g h i}\nlset x end j           ;# {a b c} {d e f} j\nlset x 2 1 j           ;# {a b c} {d e f} {g j i}\nlset x {2 1} j         ;# {a b c} {d e f} {g j i} -- indices as one grouped list argument\n\nlset x {2 3} j         ;# {a b c} {d e f} {g h i j} -- Tcl 8.6+ appends when index equals the sublist length",
            return_value: "The new value of varName after the replacement — the same value that is stored back into the variable.",
        }),
        codegen_hook: Some(CodegenHookId::Lset),
        command_forms: &[LSET_REPLACE, LSET_SINGLE_INDEX, LSET_FLAT_PATH],
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..CommandSpec::DEFAULT
    }
}

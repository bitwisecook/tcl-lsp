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

//! Folds and inline hooks under whole-unit mutation: renamed, shadowed and deleted builtins keep their runtime binding.

use super::*;

// Command-binding trust gate.
//
// A fold or a specialised inline emission *is* a builtin's semantics, so it
// may only fire while the name still denotes that builtin. C Tcl reaches the
// same answer dynamically: it compiles the specialisation unconditionally but
// wraps every command in `INST_START_CMD`, which re-dispatches the slow way
// once `iPtr->compileEpoch` moves (`tclExecute.c`, `instStartCmdFailed`).
// Specialised bytecode carries the exact source-name/registry-identity
// binding it assumes. The VM validates those typed dependencies at explicit
// source-command boundaries and replays stale commands through ordinary
// dispatch. Transformations which cannot preserve that executable provenance
// still abstain through the static whole-unit `command_binding` scan.
//
// The runtime results below are pinned on tclsh 8.6.16 and 9.0.4 (byte
// identical): `rename dict {}` then `dict create a 1 b 2` raises
// `invalid command name "dict"` on both, and `proc format {args} {return z}`
// then `format "%s!" hi` yields `z` on both — neither is a folded value this
// codegen may bake in.

/// The literal pool of `source`'s top-level function.
fn top_lits(source: &str) -> Vec<String> {
    top_asm(source)
        .literals
        .entries()
        .iter()
        .map(|literal| literal.unicode().expect("Unicode test source").to_owned())
        .collect()
}

#[test]
fn list_fold_bails_when_the_unit_renames_list() {
    crate::install_surfaces();
    // Untouched: the whole substitution collapses to one literal.
    assert!(top_lits("puts [list a b c]\n").contains(&"a b c".to_owned()));

    // tclsh 8.6.16 / 9.0.4: `rename list mylist` then `list a b c` errors
    // through `unknown` — so the words must survive as a real call.
    let lits = top_lits("rename list mylist\nputs [list a b c]\n");
    assert!(
        !lits.contains(&"a b c".to_owned()),
        "the folded list must not survive a `rename list`: {lits:?}"
    );
    for word in ["a", "b", "c"] {
        assert!(
            lits.contains(&word.to_owned()),
            "argument {word:?} must reach the runtime call: {lits:?}"
        );
    }
}

#[test]
fn format_fold_bails_when_a_proc_shadows_format() {
    crate::install_surfaces();
    assert!(top_lits("puts [format \"%s!\" hi]\n").contains(&"hi!".to_owned()));

    // tclsh 8.6.16 / 9.0.4: the shadowing proc runs and the script prints `z`.
    let lits = top_lits("proc format {args} {return z}\nputs [format \"%s!\" hi]\n");
    assert!(
        !lits.contains(&"hi!".to_owned()),
        "the folded result must not survive a shadowing `proc format`: {lits:?}"
    );
    assert!(lits.contains(&"%s!".to_owned()) && lits.contains(&"hi".to_owned()));
}

#[test]
fn dict_create_fold_bails_when_the_unit_deletes_dict() {
    crate::install_surfaces();
    let ops = ops_of("puts [dict create a 1 b 2]\n");
    assert!(ops.contains(&Op::VERIFY_DICT), "baseline folds: {ops:?}");

    // tclsh 8.6.16 / 9.0.4: `invalid command name "dict"`.
    let src = "rename dict {}\nputs [dict create a 1 b 2]\n";
    let ops = opcodes(&top_asm(src));
    assert!(
        !ops.contains(&Op::VERIFY_DICT),
        "the folded dict must not survive a `rename dict {{}}`: {ops:?}"
    );
    assert!(top_lits(src).contains(&"create".to_owned()));
}

#[test]
fn value_position_hook_with_unit_rename_retains_runtime_binding() {
    crate::install_surfaces();
    // A unit rename in a module lowered with no interpreter: a live
    // interpreter runs the `rename` before it compiles the rest.
    let ops = opcodes(&asm_for("set s ab\nset x [string length $s]\n").top_level);
    assert!(ops.contains(&Op::STR_LEN), "baseline specialises: {ops:?}");

    let asm = asm_for("rename string mystring\nset s ab\nset x [string length $s]\n").top_level;
    let ops = opcodes(&asm);
    assert!(
        ops.contains(&Op::STR_LEN),
        "the exact runtime binding lets `string length` remain specialised: {ops:?}"
    );
    assert!(
        asm.command_bindings
            .iter()
            .any(|binding| binding.name == "string" && binding.identity == "string"),
        "the specialised opcode must retain its exact ensemble dependency: {:?}",
        asm.command_bindings,
    );
    assert!(
        asm.instructions.iter().any(|instruction| {
            instruction.source_command_boundary.is_start()
                && instruction.source_cmd_text == "set x [string length $s]"
        }),
        "the VM needs an explicit replay boundary before the stale nested opcode"
    );
}

#[test]
fn the_trust_gate_is_per_name_not_whole_module() {
    crate::install_surfaces();
    // Shadowing `format` must not cost `list` or `dict` their folds — the gate
    // asks about one name, not "did this unit touch any command table".
    let src = "proc format {args} {return z}\nputs [list a b c]\nputs [dict create k v]\n";
    let lits = top_lits(src);
    assert!(lits.contains(&"a b c".to_owned()), "{lits:?}");
    assert!(lits.contains(&"k v".to_owned()), "{lits:?}");
}

/// The gate holds through the *other* caller of the shared fold helper.
///
/// The tests above reach `try_emit_constant_fold` from the argument path; the
/// simplified value emitter (`emit_value_interpolated`) reaches it from the
/// hook emitters, and `lset`'s new-value word is one such route. Both callers
/// must share the exact same fold-gate implementation, and this test is what
/// keeps the second route honest.
#[test]
fn the_fold_gate_holds_through_the_simplified_value_emitter() {
    crate::install_surfaces();
    let lits = |src: &str| {
        proc_asm(src, "::p")
            .literals
            .entries()
            .iter()
            .map(|literal| literal.unicode().expect("Unicode test source").to_owned())
            .collect::<Vec<_>>()
    };

    let list_body = "proc p {} {set l {x}\nlset l 0 [list a b c]\nreturn $l}\n";
    assert!(lits(list_body).contains(&"a b c".to_owned()));
    let renamed = lits(&format!("{list_body}rename list mylist\np\n"));
    assert!(
        !renamed.contains(&"a b c".to_owned()) && renamed.contains(&"list".to_owned()),
        "`lset`'s value word must keep the real `list` call: {renamed:?}"
    );

    let fmt_body = "proc p {} {set l {x}\nlset l 0 [format \"%s!\" hi]\nreturn $l}\n";
    assert!(lits(fmt_body).contains(&"hi!".to_owned()));
    let shadowed = lits(&format!("{fmt_body}proc format {{args}} {{return z}}\np\n"));
    assert!(
        !shadowed.contains(&"hi!".to_owned()) && shadowed.contains(&"%s!".to_owned()),
        "`lset`'s value word must keep the real `format` call: {shadowed:?}"
    );

    let dict_body = "proc p {} {set l {x}\nlset l 0 [dict create a 1]\nreturn $l}\n";
    let base_ops = opcodes(&proc_asm(dict_body, "::p"));
    assert!(base_ops.contains(&Op::VERIFY_DICT), "{base_ops:?}");
    let deleted = opcodes(&proc_asm(
        &format!("{dict_body}rename dict {{}}\np\n"),
        "::p",
    ));
    assert!(
        !deleted.contains(&Op::VERIFY_DICT),
        "`lset`'s value word must keep the real `dict create` call: {deleted:?}"
    );
}

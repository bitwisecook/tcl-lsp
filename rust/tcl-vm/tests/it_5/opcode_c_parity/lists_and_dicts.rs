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

//! `dictGetDef`, `dictRecombineStk`, argument expansion, `listIndex` and `lindexMulti`.

use super::*;

// `dictGetDef`.

/// `dictGetDef` reads the key path like `dictGet`, but a key missing at any
/// depth yields the default instead of erroring (C `INST_DICT_GET_DEF`).
#[test]
fn dict_get_def_reads_or_defaults() {
    // Present single key.
    let mut a = Asm::new();
    a.push("a 1 b 2")
        .push("b")
        .push("dflt")
        .op(Op::DICT_GET_DEF, &[1]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "2");

    // Missing leaf key → the default.
    let mut a = Asm::new();
    a.push("a 1 b 2")
        .push("z")
        .push("dflt")
        .op(Op::DICT_GET_DEF, &[1]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "dflt");

    // Nested path, present and then missing at the first level.
    let mut a = Asm::new();
    a.push("x {a 1}")
        .push("x")
        .push("a")
        .push("dflt")
        .op(Op::DICT_GET_DEF, &[2]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "1");

    let mut a = Asm::new();
    a.push("x {a 1}")
        .push("y")
        .push("a")
        .push("dflt")
        .op(Op::DICT_GET_DEF, &[2]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "dflt");
}

/// A malformed dictionary still errors — the default only covers *absent*
/// keys (C distinguishes `DICT_PATH_NON_EXISTENT` from a failed conversion).
#[test]
fn dict_get_def_rejects_malformed_dict() {
    let mut a = Asm::new();
    a.push("a 1 b")
        .push("a")
        .push("dflt")
        .op(Op::DICT_GET_DEF, &[1]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "missing value to go with key");
}

// `dictRecombineStk`.

/// `dictRecombineStk` is `dictRecombineImm` with the dict variable's name on
/// the stack (`varName path state`): it writes each expanded key's local back
/// into the variable, dropping keys whose local was unset.
#[test]
fn dict_recombine_stk_writes_back() {
    let mut a = Asm::new();
    // d = {a 1 b 2}
    a.push("d").push("a 1 b 2").op(Op::STORE_SCALAR_STK, &[]);
    a.op(Op::POP, &[]);
    // Lay down the recombine's own operands (variable name, then key path),
    // then run the `dict with` prologue, which expands the dict's keys into
    // same-named locals and pushes the recombine state on top.
    a.push("d").push("");
    a.push("a 1 b 2").push("").op(Op::DICT_EXPAND, &[]);
    // Rewrite one expanded local (a net-zero stack effect: store then pop).
    a.push("a").push("99").op(Op::STORE_SCALAR_STK, &[]);
    a.op(Op::POP, &[]);
    a.op(Op::DICT_RECOMBINE_STK, &[]);
    a.push("d").op(Op::LOAD_SCALAR_STK, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "a 99 b 2");
}

/// A key whose expanded local was unset is removed from the dict, exactly as
/// the `Imm` form (and `dict with`) does.
#[test]
fn dict_recombine_stk_drops_unset_keys() {
    let mut a = Asm::new();
    a.push("d").push("a 1 b 2").op(Op::STORE_SCALAR_STK, &[]);
    a.op(Op::POP, &[]);
    a.push("d").push("");
    a.push("a 1 b 2").push("").op(Op::DICT_EXPAND, &[]);
    a.push("b").op(Op::UNSET_STK, &[0]);
    a.op(Op::DICT_RECOMBINE_STK, &[]);
    a.push("d").op(Op::LOAD_SCALAR_STK, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "a 1");
}

// -- argument expansion ---------------------------------------------

/// Expanding an empty command-head list leaves no argv. C Tcl treats that as
/// an empty successful command rather than looking up the empty command name.
#[test]
fn invoke_expanded_with_no_words_returns_empty_success() {
    let mut a = Asm::new();
    a.op(Op::EXPAND_START, &[])
        .push("")
        .op(Op::EXPAND_STKTOP, &[1])
        .op(Op::INVOKE_EXPANDED, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "");
}

/// `Tcl_ListObjGetElements` supplies the structured list error code. The
/// opcode boundary must preserve it when the expanded word is malformed.
#[test]
fn expand_stktop_preserves_list_error_code() {
    let mut a = Asm::new();
    a.op(Op::EXPAND_START, &[])
        .push("list {")
        .op(Op::EXPAND_STKTOP, &[1]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "unmatched open brace in list");
    assert_eq!(err_code(&c), "TCL VALUE LIST BRACE");
}

/// `expandDrop` abandons the innermost expansion, truncating the stack back to
/// the depth its `expandStart` recorded (C `INST_EXPAND_DROP`).
#[test]
fn expand_drop_restores_pre_expansion_depth() {
    let mut a = Asm::new();
    a.push("keep");
    a.op(Op::EXPAND_START, &[]);
    a.push("list").push("a b c").op(Op::EXPAND_STKTOP, &[]);
    a.op(Op::EXPAND_DROP, &[]);
    // Only `keep` is left, so the trailing `list 1` sees exactly it.
    a.op(Op::LIST, &[1]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "keep");
}

/// A dropped expansion leaves the marker stack clean, so a following
/// `expandStart`/`invokeExpanded` pair still invokes the right word count.
#[test]
fn expand_drop_leaves_marker_stack_clean() {
    let mut a = Asm::new();
    a.op(Op::EXPAND_START, &[]);
    a.push("string").push("cat x y").op(Op::EXPAND_STKTOP, &[]);
    a.op(Op::EXPAND_DROP, &[]);
    a.op(Op::EXPAND_START, &[]);
    a.push("string").push("cat a b").op(Op::EXPAND_STKTOP, &[]);
    a.op(Op::INVOKE_EXPANDED, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "ab");
}

// `listIndex` / `lindexMulti`.

/// C `INST_LIST_INDEX` (`tclExecute.c:4696-4768`) only *fast-paths* an integer
/// index; anything else falls through to `TclLindexList` (4754), so the index
/// word may be a list of indices (nested `lindex`) or empty (the whole list),
/// and a spec that is neither is `bad index …` (4758-4761).
#[test]
fn list_index_implements_tcl_lindex_list() {
    for (list, idx, want) in [
        ("{a b} {c d}", "1 0", "c"),        // nested drill
        ("{a b} {c d}", "", "{a b} {c d}"), // empty index list ⇒ whole list
        ("a b c", "1", "b"),                // plain integer
        ("a b c", "end-1", "b"),            // end-relative
        ("a b c", "9", ""),                 // out of range ⇒ empty
        ("a b c", "-1", ""),                // before the start ⇒ empty
        ("{a b} {c d}", "1 9", ""),         // out of range at depth
    ] {
        let mut a = Asm::new();
        a.push(list).push(idx).op(Op::LIST_INDEX, &[]);
        let (_vm, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "listIndex {list:?} {idx:?}");
    }
}

/// A syntactically bad index is an error, not the empty string.
#[test]
fn list_index_bad_index_errors() {
    for idx in ["foo", "1.5", "end-"] {
        let mut a = Asm::new();
        a.push("a b c").push(idx).op(Op::LIST_INDEX, &[]);
        let (_vm, c) = run_fresh(a);
        assert_eq!(
            err_str(&c),
            format!("bad index \"{idx}\": must be integer?[+-]integer? or end?[+-]integer?"),
            "listIndex bad index {idx:?}"
        );
    }
}

/// `lindexMulti` is C's `TclLindexFlat` (`tclExecute.c:4833-4858`): each operand
/// is *one* index (never re-split as an index list), an out-of-range index gives
/// the empty string, and a malformed one returns `NULL` → `goto gotError`.
#[test]
fn lindex_multi_validates_every_index() {
    let mut a = Asm::new();
    a.push("{a b} {c d}").push("1").push("0");
    a.op(Op::LINDEX_MULTI, &[3]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "c");

    let mut a = Asm::new();
    a.push("{a b} {c d}").push("0").push("9");
    a.op(Op::LINDEX_MULTI, &[3]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "");

    let mut a = Asm::new();
    a.push("{a b} {c d}").push("0").push("foo");
    a.op(Op::LINDEX_MULTI, &[3]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "bad index \"foo\": must be integer?[+-]integer? or end?[+-]integer?"
    );
}

/// The compiled forms agree with the `lindex` command, which already
/// implemented `TclLindexList`/`TclLindexFlat` faithfully.
#[test]
fn list_index_matches_lindex_command() {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(compiler::svc()));
    let cmd = vm
        .eval_source("lindex {{a b} {c d}} {1 0}")
        .expect("compiles")
        .result
        .to_str()
        .to_string();
    let mut a = Asm::new();
    a.push("{a b} {c d}").push("1 0").op(Op::LIST_INDEX, &[]);
    let c = run(&mut vm, a);
    assert_eq!(ok_str(&c), cmd);
}

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

//! Commands the release compiles to instructions: math functions, loop exits, `string length`, the list commands and variable access.

use super::*;

// Bytecoded commands

#[test]
fn a_math_function_call_is_invoked_not_folded_in_tcl_8_6() {
    crate::install_surfaces();
    // A math function is a command a script may redefine, so C never folds a
    // call of one. tclsh8.6 (8.5, 9.0 and 9.1 agree),
    // `tcl::unsupported::disassemble script {set x [expr {abs(-3) + 1}]}`:
    //   push1 "x"; startCommand; push1 "tcl::mathfunc::abs"; push1 "-3";
    //   invokeStk1 2; push1 "1"; add; storeStk; done
    let function = top_asm("set x [expr {abs(-3) + 1}]");
    let ops: Vec<Op> = function.instructions.iter().map(|i| i.op).collect();
    let call = ops
        .iter()
        .position(|op| *op == Op::INVOKE_STK1)
        .expect("the function is invoked");
    assert_eq!(ops[call + 2], Op::ADD, "{ops:?}");
    assert!(
        function
            .literals
            .entries()
            .iter()
            .any(|value| value == "tcl::mathfunc::abs")
    );
    assert!(
        function.literals.entries().iter().all(|value| value != "4"),
        "the sum is computed when the script runs"
    );
}

#[test]
fn a_math_function_behind_a_false_conjunct_is_still_compiled_in_tcl_8_6() {
    crate::install_surfaces();
    // tclsh8.6, `set x [expr {0 && missing(1)}]`:
    //   push1 "x"; startCommand; push1 "0"; jumpFalse1; push1
    //   "tcl::mathfunc::missing"; push1 "1"; invokeStk1 2; jumpFalse1;
    //   push1 "1"; jump1; push1 "0"; storeStk; done
    // The call is compiled and jumped over; nothing is folded.
    let function = top_asm("set x [expr {0 && missing(1)}]");
    let ops: Vec<Op> = function.instructions.iter().map(|i| i.op).collect();
    let selected: Vec<Op> = ops
        .iter()
        .copied()
        .filter(|op| matches!(op, Op::JUMP_FALSE1 | Op::INVOKE_STK1 | Op::JUMP1))
        .collect();
    assert_eq!(
        selected,
        [Op::JUMP_FALSE1, Op::INVOKE_STK1, Op::JUMP_FALSE1, Op::JUMP1],
        "{ops:?}"
    );
    assert!(
        function
            .literals
            .entries()
            .iter()
            .any(|value| value == "tcl::mathfunc::missing")
    );
}

#[test]
fn break_and_continue_in_a_compiled_loop_follow_the_release() {
    crate::install_surfaces();
    // `tcl::unsupported::disassemble proc spin` for
    //   proc spin {} { while {1} { break } }
    // tclsh8.5:  startCommand; break; pop; jump1; push1; done
    // tclsh8.6 and 9.0:  jump4 +8; pop; jump1 -6; push1; done
    // and for `proc skip {} { for {set i 0} {$i < 3} {incr i} { continue } }`
    // tclsh8.5 emits `continue`, tclsh8.6 and 9.0 `jump4` to the next clause.
    // Inside `catch` the catch range is innermost, so every release keeps
    // the instruction: `while {1} { catch {break} }` has `break` in 8.6 too.
    let spin = "proc spin {} { while {1} { break } }";
    let skip = "proc skip {} { for {set i 0} {$i < 3} {incr i} { continue } }";
    let caught = "proc caught {} { while {1} { catch {break}; return } }";
    for (release, jumps) in [("tcl8.5", false), ("tcl8.6", true), ("tcl9.0", true)] {
        let breaks = opcodes(&crate::vm_compiled::procedure(release, spin, "::spin"));
        assert_eq!(breaks.contains(&Op::JUMP4), jumps, "{release}: {breaks:?}");
        assert_eq!(breaks.contains(&Op::BREAK), !jumps, "{release}: {breaks:?}");
        let continues = opcodes(&crate::vm_compiled::procedure(release, skip, "::skip"));
        assert_eq!(
            continues.contains(&Op::JUMP4),
            jumps,
            "{release}: {continues:?}"
        );
        assert_eq!(
            continues.contains(&Op::CONTINUE),
            !jumps,
            "{release}: {continues:?}"
        );
        let in_catch = opcodes(&crate::vm_compiled::procedure(release, caught, "::caught"));
        assert!(in_catch.contains(&Op::BREAK), "{release}: {in_catch:?}");
    }
}

#[test]
fn string_length_of_a_known_word_is_its_folded_length_from_tcl_8_5() {
    crate::install_surfaces();
    // `TclCompileStringLenCmd` pushes the character length of a word known
    // at compile time. tclsh8.5, 8.6, 9.0 and 9.1,
    // `tcl::unsupported::disassemble script {string length "hello"}`:
    //   push1 "5"; done
    // and `set s abc; string length $s` keeps `loadStk; strlen`.
    for release in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let folded = crate::vm_compiled::top_level(release, "string length \"hello\"");
        let ops = opcodes(&folded);
        assert!(!ops.contains(&Op::STR_LEN), "{release}: {ops:?}");
        assert!(
            folded
                .literals
                .entries()
                .iter()
                .any(|literal| literal == "5"),
            "{release}: {:?}",
            folded.literals.entries()
        );
        let read = opcodes(&crate::vm_compiled::top_level(
            release,
            "set s abc; string length $s",
        ));
        assert!(read.contains(&Op::STR_LEN), "{release}: {read:?}");
    }
}

#[test]
fn top_level_commands_compile_to_the_instructions_tclsh_8_6_emits() {
    crate::install_surfaces();
    // tclsh8.6: tcl::unsupported::disassemble script {<source>}
    //   set x hello; append x world      -> push1 push1 storeStk pop push1 push1 appendStk done
    //   set x {}; lappend x foo          -> ... push1 push1 list 1 lappendListStk done
    //   lindex "a b c" 1                 -> push1 listIndexImm 1 done
    //   lreplace "a b c d" 1 2 X Y       -> push1 push1 push1 list 2 over 1 listRangeImm 0 0
    //                                       reverse 2 listConcat reverse 2 listRangeImm 3 end
    //                                       listConcat done
    //   string equal "a" "b"             -> push1 push1 streq done
    //   string compare "a" "b"           -> push1 push1 strcmp done
    //   set d [dict create a 1 b 2]; dict get $d a -> ... loadStk push1 dictGet 1 done
    //   set d [dict create a 1]; dict exists $d a  -> ... loadStk push1 dictExists 1 done
    for (source, compiled) in [
        ("set x hello; append x world", vec![Op::APPEND_STK]),
        (
            "set x {}; lappend x foo",
            vec![Op::LIST, Op::LAPPEND_LIST_STK],
        ),
        ("lindex \"a b c\" 1", vec![Op::LIST_INDEX_IMM]),
        (
            "lreplace \"a b c d\" 1 2 X Y",
            vec![
                Op::LIST,
                Op::OVER,
                Op::LIST_RANGE_IMM,
                Op::REVERSE,
                Op::LIST_CONCAT,
                Op::REVERSE,
                Op::LIST_RANGE_IMM,
                Op::LIST_CONCAT,
            ],
        ),
        ("string equal \"a\" \"b\"", vec![Op::STR_EQ]),
        ("string compare \"a\" \"b\"", vec![Op::STR_CMP]),
        (
            "set d [dict create a 1 b 2]; dict get $d a",
            vec![Op::DUP, Op::VERIFY_DICT, Op::LOAD_STK, Op::DICT_GET],
        ),
        (
            "set d [dict create a 1]; dict exists $d a",
            vec![Op::DUP, Op::VERIFY_DICT, Op::LOAD_STK, Op::DICT_EXISTS],
        ),
    ] {
        let ops = opcodes(&top_asm(source));
        let selected: Vec<Op> = ops
            .iter()
            .copied()
            .filter(|op| compiled.contains(op))
            .collect();
        assert_eq!(selected, compiled, "{source}: {ops:?}");
        assert!(
            !ops.iter()
                .any(|op| matches!(op, Op::INVOKE_STK1 | Op::INVOKE_STK4)),
            "{source}: no command is invoked: {ops:?}"
        );
    }
}

#[test]
fn append_proc_is_bytecoded() {
    crate::install_surfaces();
    // In a PROC body the Rust emitter DOES specialise append → appendScalar1.
    let fa = proc_asm("proc b {} { set s \"\"; append s \"x\"; return $s }", "::b");
    assert!(opcodes(&fa).contains(&Op::APPEND_SCALAR1));
}

#[test]
fn lappend_proc_is_bytecoded() {
    crate::install_surfaces();
    // In a PROC body Rust specialises lappend → lappendScalar1.
    let fa = proc_asm("proc b {} { set l {}; lappend l \"y\"; return $l }", "::b");
    assert!(opcodes(&fa).contains(&Op::LAPPEND_SCALAR1));
}

// Variable access patterns

#[test]
fn proc_array_ref() {
    crate::install_surfaces();
    // tclsh8.6: `return $arr(key1)` is `push1 "key1"; loadArray1 %v1`.
    let src =
        "proc array_op {key} { set arr(key1) \"val1\"; set arr($key) \"val2\"; return $arr(key1) }";
    let ops = opcodes(&proc_asm(src, "::array_op"));
    assert!(ops.contains(&Op::STORE_ARRAY1));
    assert!(ops.contains(&Op::LOAD_ARRAY1));
}

#[test]
fn top_level_array_ref() {
    crate::install_surfaces();
    let ops = ops_of("set arr(key) \"hello\"; set x $arr(key)");
    assert!(ops.contains(&Op::STORE_ARRAY_STK) || ops.contains(&Op::STORE_STK));
}

#[test]
fn qualified_var_in_proc() {
    crate::install_surfaces();
    let src = "proc ns_access {} { set ::global_var 42; return $::global_var }";
    let ops = opcodes(&proc_asm(src, "::ns_access"));
    // Qualified names always use stack-based ops, even in procs.
    assert!(ops.contains(&Op::STORE_STK));
    assert!(ops.contains(&Op::LOAD_STK));
}

#[test]
fn incr_in_proc_vs_top_level() {
    crate::install_surfaces();
    let proc_fa = proc_asm("proc inc {x} { incr x; return $x }", "::inc");
    let top = ops_of("set x 0; incr x");
    assert!(
        opcodes(&proc_fa).contains(&Op::INCR_SCALAR1_IMM),
        "proc uses INCR_SCALAR1_IMM"
    );
    assert!(
        top.contains(&Op::INCR_STK_IMM),
        "top-level uses INCR_STK_IMM"
    );
}

#[test]
fn incr_large_amount() {
    crate::install_surfaces();
    let src = "proc big_incr {} { set x 0; incr x 1000; return $x }";
    let ops = opcodes(&proc_asm(src, "::big_incr"));
    assert!(ops.contains(&Op::INCR_SCALAR1) || ops.contains(&Op::INVOKE_STK1));
}

// Bytecoded list commands
//
// Only the list commands the emitter actually specialises are asserted
// with their opcodes; `lindex` routes through a generic invoke (no
// LIST_INDEX_IMM specialisation) and is asserted accordingly.

#[test]
fn llength_bytecoded() {
    crate::install_surfaces();
    // tclsh: llength "a b c"  →  3   (8.6 + 9.0)
    assert!(ops_of("llength \"a b c\"").contains(&Op::LIST_LENGTH));
}

#[test]
fn lrange_bytecoded() {
    crate::install_surfaces();
    assert!(ops_of("lrange \"a b c d e\" 1 3").contains(&Op::LIST_RANGE_IMM));
}

#[test]
fn list_create_in_proc() {
    crate::install_surfaces();
    let fa = proc_asm("proc mk {a b c} { list $a $b $c }", "::mk");
    assert!(opcodes(&fa).contains(&Op::INVOKE_STK1) || opcodes(&fa).contains(&Op::LIST));
}

#[test]
fn linsert_bytecoded() {
    crate::install_surfaces();
    // `linsert` IS specialised to lreplace4.
    assert!(ops_of("linsert \"a b c\" 1 X").contains(&Op::LREPLACE4));
}

#[test]
fn lassign_bytecoded() {
    crate::install_surfaces();
    let ops = ops_of("lassign \"a b c\" x y z");
    assert!(ops.contains(&Op::LIST_INDEX_IMM));
    assert!(ops.contains(&Op::LIST_RANGE_IMM));
}

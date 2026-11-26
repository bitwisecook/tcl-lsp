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

//! List and dictionary commands and their compiled forms: `linsert`, `ledit`, `lset`, `lpop`, `lrange`, `lindex`, `lsort`, `lseq`, `lappend`, `dict`.

use super::*;

/// The compiled `linsert`/`lreplace` opcode rejects a non-integer index with
/// the same `bad index` error as the command, rather than silently treating it
/// as 0. Regression for linsert-2.2 / linsert-2.3.
#[test]
fn compiled_linsert_rejects_bad_index() {
    let (ok, result, _) = run("catch {linsert a b} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(
        result,
        "bad index \"b\": must be integer?[+-]integer? or end?[+-]integer?"
    );

    // A valid index still inserts (no false positive).
    let (ok, result, _) = run("linsert {a b c} end X");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "a b c X");
}

/// `ledit listVar first last ?element ...?` is the in-place `lreplace` (Tcl 9):
/// it edits the list held in the variable, stores the result back, and returns
/// it. A missing array element reports the array-specific read error. Regression
/// for lreplace.test's ledit battery.
#[test]
fn ledit_edits_in_place_and_reports_array_miss() {
    let (ok, result, _) = run("set l {1 2 3 4 5}\nledit l 1 1 a b\nset l");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "1 a b 3 4 5");

    let (ok, result, _) = run(concat!(
        "unset -nocomplain arr\nset arr(y) y\n",
        "catch {ledit arr(x) 0 0 x} m\nset m",
    ));
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "can't read \"arr(x)\": no such element in array");
}

/// The runtime `lset` builtin (the fallback for the dynamic / wrong-arg paths
/// the compiler does not inline) reports the usage error, replaces the whole
/// value with no index, and descends a flat index path. Regression for
/// lsetComp-1.1 (`lset` was only a codegen hook, so the fallback hit "invalid
/// command name").
#[test]
fn lset_runtime_builtin() {
    // Dispatched indirectly so it cannot be inlined → exercises the builtin.
    let (ok, result, _) = run("set c lset\ncatch {$c} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(
        result,
        "wrong # args: should be \"lset listVar ?index? ?index ...? value\""
    );

    let (ok, result, _) = run("set l {{a b} {c d}}\nset c lset\n$c l 0 1 X\nset l");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "{a X} {c d}");
}

/// `lrepeat` rejects a negative count with C's exact message (lrepeat-1.4).
#[test]
fn lrepeat_negative_count_message() {
    let (ok, result, _) = run("catch {lrepeat -3 1} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "bad count \"-3\": must be integer >= 0");
}

/// `lpop` removes and returns an element of the list in a variable (default the
/// last), descending into nested sublists with several indices, and stores the
/// trimmed list back. Out-of-range / non-integer indices error like C.
#[test]
fn lpop_removes_and_returns_element() {
    let (ok, result, _) = run("set l {a b c d}\nset got [lpop l]\nlist $got $l");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "d {a b c}");

    let (ok, result, _) = run("set l {{a b} {c d}}\nset got [lpop l 0 1]\nlist $got $l");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "b {a {c d}}");

    let (ok, result, _) = run("set l {a b c}\ncatch {lpop l 99} m\nset m");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "index \"99\" out of range");
}

/// The compiled `LIST_RANGE_IMM` path resolves `end+N` indices like the
/// uncompiled command: as the last index `end+N` clamps to the end (keeps
/// everything), as the first index it is past the end (empty). Regression for
/// lrange.test's lrange-5 "shared compiled" battery, where `end+N` encodes as
/// `INDEX_END + N` (above a `<= INDEX_END` detection) and would be misread as a
/// huge plain index.
#[test]
fn compiled_lrange_handles_end_plus_n() {
    // `end+1` as the last index includes through the end.
    let (ok, result, _) = run("join [lrange {a b c} 0 end+1] -");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "a-b-c");

    // `end+1` as the first index is past the end → empty.
    let (ok, result, _) = run("list [lrange {a b c} end+1 end]");
    assert!(ok, "script should complete: {result}");
    assert_eq!(result, "{}");
}

/// `dict with` maps a dict's keys to local variables, runs the body, and
/// reflects the variables back: a modified key is updated, an unset key is
/// removed, and a variable the body merely created is not added. A nested
/// key path updates the sub-dict in place.
#[test]
fn dict_with_maps_and_reflects() {
    // Modified `a`, removed `b` (unset), new `c` not added back.
    assert_eq!(
        run("set d {a 1 b 2}\ndict with d {set a 9; unset b; set c 3}\nset d").1,
        "a 9",
    );
    // Nested key path: the sub-dict at `x` is updated in place.
    assert_eq!(
        run("set d {x {a 1 b 2}}\ndict with d x {incr a 5}\nset d").1,
        "x {a 6 b 2}",
    );
}

/// `append`/`lappend` routed through the shared cores
/// (`tcl_cmd_core::var::{append_bytes, lappend_value}`). Pins the user-visible
/// behaviour against tclsh: concatenation/list building, the no-values read
/// form, and `append`/`lappend` of an unset variable with no values
/// errors (`can't read`) rather than silently creating an empty variable.
#[test]
fn append_lappend_shared_core() {
    assert_eq!(run("set x ab\nappend x cd ef").1, "abcdef");
    assert_eq!(run("lappend y a b\nlappend y c").1, "a b c");
    // no-values read returns the value (set) and errors (unset).
    assert_eq!(run("set s hi\nappend s").1, "hi");
    let (ok, msg, _) = run("append nope");
    assert!(!ok);
    assert_eq!(msg, "can't read \"nope\": no such variable");
    // lappend no-values validates the current value as a list.
    let (ok, msg, _) = run("set z \"{\"\nlappend z");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
}

/// `lsort` comparison modes routed through the shared `tcl_cmd_core::sort` core,
/// lifting the VM from its old crude subset: `-dictionary` (was a plain byte
/// compare), the `-integer`-vs-`-real` distinction (both were double), and
/// `-nocase`. Pinned against tclsh 9.0.
#[test]
fn lsort_shared_comparison() {
    assert_eq!(run("lsort -dictionary {x10 x9 x100}").1, "x9 x10 x100");
    assert_eq!(run("lsort -integer {10 9 100 2}").1, "2 9 10 100");
    assert_eq!(run("lsort -real {1.5 1.25 10.0}").1, "1.25 1.5 10.0");
    assert_eq!(run("lsort -nocase {B a C b}").1, "a B b C");
    // -unique removes elements equal under the mode (1 and 01 for -integer).
    assert_eq!(run("lsort -integer -unique {1 01 1 2}").1, "1 2");
    // plain ascii is unchanged (lexical, so x10 < x100 < x9).
    assert_eq!(run("lsort {x10 x9 x100}").1, "x10 x100 x9");
}

/// Native Tcl 9.0 numeric sequence argument decoding and bounded generation.
#[test]
fn lseq_shared() {
    fn run(src: &str) -> (bool, String, String) {
        let mut vm = tcl_vm::test_support::native_vm_for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            Box::new(std::io::stdout()),
        );
        vm.set_compiler(Box::new(BytecodeCompileService::default()));
        let completion = vm
            .try_eval_source(src)
            .expect("native numeric sequence fixture");
        (
            completion.code.is_ok(),
            String::from_utf8(completion.result.string_bytes().to_vec()).expect("Unicode result"),
            String::new(),
        )
    }
    // Integer forms (count-only, ranges, by-step, count keyword, n n).
    assert_eq!(run("lseq 5").1, "0 1 2 3 4");
    assert_eq!(run("lseq 0").1, "");
    assert_eq!(run("lseq -5").1, ""); // negative count → empty
    assert_eq!(run("lseq 1 .. 10").1, "1 2 3 4 5 6 7 8 9 10");
    assert_eq!(run("lseq 10 .. 1").1, "10 9 8 7 6 5 4 3 2 1");
    assert_eq!(run("lseq 1 to 10 by 2").1, "1 3 5 7 9");
    assert_eq!(run("lseq 1 to 10 by -2").1, ""); // wrong-sign step → empty
    assert_eq!(run("lseq 5 count 5 by -2").1, "5 3 1 -1 -3");
    assert_eq!(run("lseq 3 by 2").1, "0 2 4");
    assert_eq!(run("lseq 5 1").1, "5 4 3 2 1");
    // Double precision matches the inputs' fractional digits.
    assert_eq!(run("lseq 0 0.5 by 0.1").1, "0.0 0.1 0.2 0.3 0.4 0.5");
    assert_eq!(run("lseq 25. to 5. by -5").1, "25.0 20.0 15.0 10.0 5.0");
    // A double-valued count stays an integer sequence.
    assert_eq!(run("lseq 5 count 5.0").1, "5 6 7 8 9");
    // Native argument bytes are number-converted without expression evaluation.
    let (ok, message, _) = run("lseq 1+2 to 10");
    assert!(!ok);
    assert_eq!(message, "expected number but got \"1+2\"");
    let (ok, message, _) = run("set n 3; lseq $n*2");
    assert!(!ok);
    assert_eq!(message, "expected number but got \"3*2\"");
    // Errors.
    let (ok, msg, _) = run("lseq");
    assert!(!ok);
    assert_eq!(msg, "wrong # args: should be \"lseq n ??op? n ??by? n??\"");
}

/// The `list`-family commands served by the shared `tcl-cmd-core` (driven over
/// the VM's `ValueOps` list operations) behave identically end-to-end.
#[test]
fn list_core_helpers() {
    assert_eq!(run("puts [llength {a b c}]").2, "3\n");
    assert_eq!(run("puts [lindex {a b c} 1]").2, "b\n");
    assert_eq!(run("puts [lindex {a b c} end]").2, "c\n");
    assert_eq!(run("puts [lindex {{a b} {c d}} 1 0]").2, "c\n");
    assert_eq!(run("puts [lrange {a b c d} 1 2]").2, "b c\n");
    assert_eq!(run("puts [lreverse {a b c}]").2, "c b a\n");
    assert_eq!(run("puts [lrepeat 3 x]").2, "x x x\n");
    assert_eq!(run("puts [concat a {b c} d]").2, "a b c d\n");
    assert_eq!(run("puts [join {a b c} -]").2, "a-b-c\n");
    assert_eq!(run("puts [llength [split a,b,c ,]]").2, "3\n");
    assert_eq!(run("puts [list x {a b} y]").2, "x {a b} y\n");
}

/// A malformed list index errors faithfully (the shared core errors where the
/// VM's old lenient path silently returned empty).
#[test]
fn lindex_bad_index_errors() {
    let (ok, result, _out) = run("lindex {a b c} foo");
    assert!(!ok);
    assert!(result.contains("bad index"), "got: {result}");
}

/// The pure `dict` family, served by the shared core (over the default
/// list-backed `dict_pairs`/`new_dict` seam).
#[test]
fn dict_core_helpers() {
    assert_eq!(run("puts [dict get {a 1 b 2} b]").2, "2\n");
    assert_eq!(run("puts [dict size {a 1 b 2}]").2, "2\n");
    assert_eq!(run("puts [dict keys {a 1 b 2}]").2, "a b\n");
    assert_eq!(run("puts [dict exists {a 1} a]").2, "1\n");
    assert_eq!(run("puts [dict exists {a 1} z]").2, "0\n");
    assert_eq!(
        run("puts [dict merge {a 1 b 2} {b 3 c 4}]").2,
        "a 1 b 3 c 4\n"
    );
    // canonicalisation (last value wins) — a non-deduping path would be
    // wrong here; the shared core handles it correctly.
    assert_eq!(run("puts [dict get [dict create x 1 x 2] x]").2, "2\n");
}

/// A missing key errors faithfully.
#[test]
fn dict_get_missing_key_errors() {
    let (ok, result, _out) = run("dict get {a 1} z");
    assert!(!ok);
    assert!(result.contains("not known in dictionary"), "got: {result}");
}

/// `lset` (the `LSET_LIST` single-index / index-path form and the `LSET_FLAT`
/// multi-index form) end-to-end. Both the proc-local opcode form and the
/// top-level stack form route through these opcodes; verified against tclsh 9.0.
#[test]
fn lset_list_and_flat() {
    // Single index (LSET_LIST).
    assert_eq!(run("set l {a b c}; lset l 1 X; puts $l").2, "a X c\n");
    assert_eq!(run("set l {a b c}; lset l end 9; puts $l").2, "a b 9\n");
    // Appending one past the end.
    assert_eq!(run("set l {a b c}; lset l 3 D; puts $l").2, "a b c D\n");
    // Empty index path replaces the whole value.
    assert_eq!(run("set l {a b c}; lset l {} Z; puts $l").2, "Z\n");
    // An index *path* given as one list argument (LSET_LIST descends it).
    assert_eq!(
        run("set l {{a b} {c d}}; lset l {1 0} X; puts $l").2,
        "{a b} {X d}\n"
    );
    // Multiple flat index arguments (LSET_FLAT), top-level and in a proc.
    assert_eq!(
        run("set l {{a b} {c d}}; lset l 1 0 X; puts $l").2,
        "{a b} {X d}\n"
    );
    assert_eq!(
        run("proc f {} { set l {{a b} {c d}}; lset l 1 0 X; return $l }; puts [f]").2,
        "{a b} {X d}\n"
    );
    // Errors match tclsh 9.0.
    let (ok, msg, _) = run("set l {a b c}; lset l 5 X");
    assert!(!ok);
    assert_eq!(msg, "index \"5\" out of range");
    let (ok, msg, _) = run("set l {a b c}; lset l foo X");
    assert!(!ok);
    assert_eq!(
        msg,
        "bad index \"foo\": must be integer?[+-]integer? or end?[+-]integer?"
    );
}

/// `REVERSE` (reverse top N stack items), `LINDEX_MULTI` (nested `lindex`), and
/// `STR_REPLACE` (`string replace`) — driven directly through the VM's opcode
/// dispatch (these are emitted by the inline command-substitution emitter).
#[test]
fn opcode_reverse_lindex_multi_str_replace() {
    use tcl_bytecode::{Instruction, Op, Operand};
    // REVERSE 2 then concat: push x,y → reverse → y,x → "yx".
    let r = run_asm(
        &["x", "y"],
        vec![
            pushv(0),
            pushv(1),
            Instruction::new(Op::REVERSE, vec![Operand::Imm(2)]),
            Instruction::new(Op::STR_CONCAT1, vec![Operand::Imm(2)]),
            Instruction::new(Op::DONE, vec![]),
        ],
    );
    assert_eq!(r, "yx");
    // LINDEX_MULTI 3: lindex {{a b} {c d}} 1 0 → "c".
    let r = run_asm(
        &["{a b} {c d}", "1", "0"],
        vec![
            pushv(0),
            pushv(1),
            pushv(2),
            Instruction::new(Op::LINDEX_MULTI, vec![Operand::Imm(3)]),
            Instruction::new(Op::DONE, vec![]),
        ],
    );
    assert_eq!(r, "c");
    // STR_REPLACE: string replace "abcde" 1 3 "XY" → "aXYe".
    let r = run_asm(
        &["abcde", "1", "3", "XY"],
        vec![
            pushv(0),
            pushv(1),
            pushv(2),
            pushv(3),
            Instruction::new(Op::STR_REPLACE, vec![]),
            Instruction::new(Op::DONE, vec![]),
        ],
    );
    assert_eq!(r, "aXYe");
    // STR_REPLACE whole-string replace and end indices.
    let r = run_asm(
        &["abcde", "0", "end", "Z"],
        vec![
            pushv(0),
            pushv(1),
            pushv(2),
            pushv(3),
            Instruction::new(Op::STR_REPLACE, vec![]),
            Instruction::new(Op::DONE, vec![]),
        ],
    );
    assert_eq!(r, "Z");
}

/// `lindex` with a non-constant index compiles to `LIST_INDEX`, which is C's
/// full `TclLindexList` (`tclExecute.c:4696-4768` falling through to line 4754):
/// a list-valued index drills nested sublists, an empty index list is the whole
/// list, and a garbage spec errors. The old `unwrap_or(-1)` funnel made all
/// three the empty string.
#[test]
fn compiled_list_index_implements_tcl_lindex_list() {
    assert_eq!(run("set i {1 0}; set r [lindex {{a b} {c d}} $i]").1, "c");
    assert_eq!(
        run("set i {}; set r [lindex {{a b} {c d}} $i]").1,
        "{a b} {c d}"
    );
    assert_eq!(run("set i 9; set r [lindex {a b c} $i]").1, "");
    let (ok, msg, _) = run("set i foo; set r [lindex {a b c} $i]");
    assert!(!ok);
    assert_eq!(
        msg,
        "bad index \"foo\": must be integer?[+-]integer? or end?[+-]integer?"
    );
    // `LINDEX_MULTI` (≥ 2 indices) is C's `TclLindexFlat` and validates each.
    assert_eq!(run("set i 0; set r [lindex {{a b} {c d}} 1 $i]").1, "c");
    let (ok, msg, _) = run("set i foo; set r [lindex {{a b} {c d}} 0 $i]");
    assert!(!ok);
    assert!(msg.starts_with("bad index \"foo\""), "got: {msg}");
    // A malformed *list value* still propagates its own conversion error, as
    // C's `TclListObjGetElements` failure does — not `bad index`.
    let (ok, msg, _) = run("set l \"a \\{b\"; set i 0; set r [lindex $l $i]");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
    // The compiled path agrees with the uncompiled command.
    assert_eq!(
        run("set i {1 0}; set r [lindex {{a b} {c d}} $i]").1,
        run("puts -nonewline [lindex {{a b} {c d}} {1 0}]").2
    );
}

/// Top-level (non-proc) mutating `dict` subcommands compile to the ensemble
/// rewrite `INVOKE_REPLACE` form (`push dict <sub> … ::tcl::dict::<sub>;
/// invokeReplace`), matching tclsh 9.0, rather than a plain generic invoke.
#[test]
fn dict_toplevel_ensemble() {
    assert_eq!(run("dict set d k v; puts $d").2, "k v\n");
    assert_eq!(
        run("dict set d a 1; dict set d b 2; puts $d").2,
        "a 1 b 2\n"
    );
    assert_eq!(run("dict incr c n 3; puts $c").2, "n 3\n");
    assert_eq!(
        run("dict append e k ab; dict append e k cd; puts $e").2,
        "k abcd\n"
    );
    assert_eq!(run("dict lappend f k 1 2; puts $f").2, "k {1 2}\n");
    assert_eq!(run("set d {a 1 b 2}; dict unset d a; puts $d").2, "b 2\n");
}

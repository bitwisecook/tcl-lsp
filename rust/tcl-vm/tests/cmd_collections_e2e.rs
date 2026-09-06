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

//! End-to-end coverage of the collection commands (`array`, `dict`, and the
//! `list`/`l*` family) served by `tcl-vm`'s adapters over the shared
//! `tcl-cmd-core`. Each script is compiled as real Tcl via `tcl-compiler` and
//! run through `tcl-vm`, asserting observable behaviour against real tclsh
//! (tclsh8.6 + tclsh9.0 — the oracle; cited per test in `// tclsh:` comments).
//!
//! The harness (`run`, `Capture`, `CompilerSvc`) is copied verbatim from
//! `run_script.rs` so this file is self-contained.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::Vm;

/// A `Write` sink backed by a shared buffer the test can read afterwards.
#[derive(Clone)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Compile and run `src`; return `(ok, result-string, captured-stdout)`.
fn run(src: &str) -> (bool, String, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);

    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    let completion = vm.run_module(&asm);

    let out = String::from_utf8(buf.borrow().clone()).expect("utf-8 output");
    (
        completion.code.is_ok(),
        completion.result.to_str().to_string(),
        out,
    )
}

// array

/// `array set` materialises elements, including the empty-list form (which
/// still creates the array), and `array get`/`names`/`size`/`exists` read them
/// back. `array get` order is unspecified, so results are sorted before compare.
#[test]
fn array_set_get_names_size_exists() {
    // tclsh: `array set a {x 1 y 2}` then `lsort [array get a]` → `1 2 x y`.
    assert_eq!(
        run("array set a {x 1 y 2}\nlsort [array get a]").1,
        "1 2 x y"
    );
    // tclsh: `lsort [array names a]` → `x y`.
    assert_eq!(run("array set a {x 1 y 2}\nlsort [array names a]").1, "x y");
    // tclsh: `array size a` → `2`.
    assert_eq!(run("array set a {x 1 y 2}\narray size a").1, "2");
    // tclsh: `array exists a` → `1`; missing → `0`.
    assert_eq!(run("array set a {x 1 y 2}\narray exists a").1, "1");
    assert_eq!(run("array exists nope").1, "0");
    // tclsh: `array set a {}` still creates the array, so `array exists a` → `1`.
    assert_eq!(run("array set a {}\narray exists a").1, "1");
}

/// `array get`/`names` glob-filter on the key with `?pattern?`.
#[test]
fn array_get_names_pattern() {
    // tclsh: `lsort [array names a x*]` over {x,y,xz} → `x xz`.
    assert_eq!(
        run("array set a {x 1 y 2 xz 3}\nlsort [array names a x*]").1,
        "x xz"
    );
    // tclsh: `lsort [array get a x*]` → `1 3 x xz`.
    assert_eq!(
        run("array set a {x 1 y 2 xz 3}\nlsort [array get a x*]").1,
        "1 3 x xz"
    );
}

/// `array unset arrayName ?pattern?` — with a pattern removes the matching
/// elements; with no pattern removes the whole array (the fix the module
/// docstring calls out — it used to iterate-and-empty instead of deleting).
#[test]
fn array_unset_pattern_and_whole() {
    // tclsh: after `array unset a x` over {x,y} → names `y`.
    assert_eq!(
        run("set a(x) 1\nset a(y) 2\narray unset a x\nlsort [array names a]").1,
        "y"
    );
    // tclsh: `array unset a x*` over {x,y,xz} → names `y`.
    assert_eq!(
        run("set a(x) 1\nset a(y) 2\nset a(xz) 3\narray unset a x*\nlsort [array names a]").1,
        "y"
    );
    // tclsh: `array unset a` (no pattern) deletes the array → `array exists a` 0.
    assert_eq!(run("set a(x) 1\narray unset a\narray exists a").1, "0");
}

/// BUG: `array set` onto an existing scalar reports the wrong variable name in
/// its error. C raises the failure at the *element* write, so the message names
/// the element (`s(a)`); the VM pre-checks with `ensure_array`, which raises
/// before any element is written and so names only the bare scalar (`s`).
///   script:        set s 5; array set s {a 1}
///   tclsh (8.6+9.0): can't set "s(a)": variable isn't array
///   VM (wrong):      can't set "s": variable isn't array
/// This test asserts the correct tclsh behaviour, now fixed (guards regression).
#[test]
fn array_set_onto_scalar_errors_bug() {
    let (ok, msg, _) = run("set s 5\narray set s {a 1}");
    assert!(!ok);
    // tclsh names the element `s(a)`, not the bare scalar `s`.
    assert_eq!(msg, "can't set \"s(a)\": variable isn't array");
}

/// `array set` with an odd-length list is rejected before any write.
#[test]
fn array_set_odd_list_errors() {
    // tclsh: `array set a {x 1 y}` → list must have an even number of elements.
    let (ok, msg, _) = run("array set a {x 1 y}");
    assert!(!ok);
    assert_eq!(msg, "list must have an even number of elements");
}

/// `array set arrayName list` arity, and the bare `array` arity error.
#[test]
fn array_arity_errors() {
    // tclsh: `array set a` → wrong # args: should be "array set arrayName list".
    let (ok, msg, _) = run("array set a");
    assert!(!ok);
    assert_eq!(msg, "wrong # args: should be \"array set arrayName list\"");
    // tclsh: bare `array` → wrong # args: should be "array subcommand ?arg ...?".
    let (ok, msg, _) = run("array");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"array subcommand ?arg ...?\""
    );
}

/// An unknown `array` subcommand reports the VM's bad-subcommand message (the
/// VM's supported set is exists/for/get/names/set/size/unset — narrower than C's
/// full ensemble, so the message text is the VM's own).
///
/// Issue #1607: the sentence and the exact-then-unique-prefix scan are
/// `tcl_cmd_core::ensemble`'s, so `array e a` abbreviates as it does in tclsh.
///
/// tclsh 9.0.4 (the verdicts, not the shortened list):
///   array e a  -> 1     ;  array ex a -> 1
///   array s a  -> unknown or ambiguous subcommand "s": must be …
///                        (set/size both start with `s`)
///   array {} a -> unknown or ambiguous subcommand "": must be …
#[test]
fn array_bad_subcommand() {
    const MUST: &str = "must be exists, for, get, names, set, size, or unset";
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    assert_eq!(
        msg("array badcmd a"),
        format!("unknown or ambiguous subcommand \"badcmd\": {MUST}")
    );
    assert_eq!(
        msg("array s a"),
        format!("unknown or ambiguous subcommand \"s\": {MUST}")
    );
    assert_eq!(
        msg("array {} a"),
        format!("unknown or ambiguous subcommand \"\": {MUST}")
    );
    // Unique prefixes resolve.
    assert_eq!(run("set a(x) 1\narray e a").1, "1");
    assert_eq!(run("set a(x) 1\narray ex a").1, "1");
    assert_eq!(run("set a(x) 1\narray n a").1, "x");
}

/// `array for {k v} arrayName body` iterates the element pairs (Tcl 9.0). The
/// element set is snapshotted, so a stable walk visits every pair; results are
/// sorted because element order is unspecified.
#[test]
fn array_for_iterates() {
    // tclsh9.0: `array for {k v} a {...}` over {x 1 y 2} → pairs x=1 y=2.
    assert_eq!(
        run("array set a {x 1 y 2}\nset r {}\narray for {k v} a {lappend r $k=$v}\nlsort $r").1,
        "x=1 y=2"
    );
}

/// `array for` honours `break` and `continue` in the body.
#[test]
fn array_for_break_continue() {
    // continue: skip the `b` pair; collect the rest. (Deterministic: filter by key.)
    // tclsh9.0: collects every key except the skipped one.
    assert_eq!(
        run("array set a {a 1 b 2 c 3}\nset r {}\narray for {k v} a {if {$k eq \"b\"} continue; lappend r $k}\nlsort $r").1,
        "a c"
    );
    // break stops the walk after one element is recorded → exactly one key kept.
    assert_eq!(
        run("array set a {a 1 b 2 c 3}\nset n 0\narray for {k v} a {incr n; break}\nset n").1,
        "1"
    );
}

/// `array for` argument validation: the var list must hold exactly two names,
/// the target must be an array, and the arity is fixed (Tcl 9.0).
#[test]
fn array_for_errors() {
    // tclsh9.0: one var name → must have two variable names.
    let (ok, msg, _) = run("set a(x) 1\narray for {k} a {puts $k}");
    assert!(!ok);
    assert_eq!(msg, "must have two variable names");
    // tclsh9.0: non-array target → "nope" isn't an array.
    let (ok, msg, _) = run("array for {k v} nope {puts $k}");
    assert!(!ok);
    assert_eq!(msg, "\"nope\" isn't an array");
    // arity: missing body.
    let (ok, msg, _) = run("set a(x) 1\narray for {k v} a");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"array for {key value} arrayName script\""
    );
}

/// An error raised in an `array for` body propagates out of the command.
#[test]
fn array_for_body_error_propagates() {
    // tclsh9.0: the body error surfaces as the command's error.
    let (ok, msg, _) = run("set a(x) 1\narray for {k v} a {error boom}");
    assert!(!ok);
    assert_eq!(msg, "boom");
}

// dict — pure (read) side

/// The pure dict reads: `get`/`size`/`keys`/`values`/`exists`, including nested
/// `get`, glob-filtered `keys`/`values`, and `exists` false paths.
#[test]
fn dict_pure_reads() {
    // tclsh: basic reads.
    assert_eq!(run("dict get {a 1 b 2} b").1, "2");
    assert_eq!(run("dict size {a 1 b 2}").1, "2");
    assert_eq!(run("dict keys {a 1 b 2}").1, "a b");
    assert_eq!(run("dict values {a 1 b 2}").1, "1 2");
    // tclsh: nested get descends the key path.
    assert_eq!(run("dict get {a {x 1}} a x").1, "1");
    // tclsh: exists true/false (missing key, and non-dict short-circuit).
    assert_eq!(run("dict exists {a 1} a").1, "1");
    assert_eq!(run("dict exists {a 1} z").1, "0");
    // tclsh: glob-filtered keys/values.
    assert_eq!(
        run("dict keys {apple 1 avocado 2 banana 3} a*").1,
        "apple avocado"
    );
    assert_eq!(run("dict values {a 1 b 2 c 1} 1").1, "1 1");
}

/// `dict create`/`merge`/`remove`/`replace` build new dicts; `merge`/`create`
/// canonicalise (last value wins).
#[test]
fn dict_create_merge_remove_replace() {
    // tclsh: empty forms.
    assert_eq!(run("dict create").1, "");
    assert_eq!(run("dict merge").1, "");
    // tclsh: create with pairs.
    assert_eq!(run("dict create a 1 b 2").1, "a 1 b 2");
    // tclsh: merge, last value wins.
    assert_eq!(run("dict merge {a 1 b 2} {b 3 c 4}").1, "a 1 b 3 c 4");
    // tclsh: remove a key.
    assert_eq!(run("dict remove {a 1 b 2} a").1, "b 2");
    // tclsh: replace existing + add new.
    assert_eq!(
        run("dict replace {a 1 b 2 c 3} a 9 d 4").1,
        "a 9 b 2 c 3 d 4"
    );
}

/// `dict getdef`/`getwithdefault` return the default for a missing key, else the
/// found value (Tcl 9.0).
#[test]
fn dict_getdef() {
    // tclsh9.0: missing key → default.
    assert_eq!(run("dict getdef {a 1 b 2} c -1").1, "-1");
    // tclsh9.0: present key → its value.
    assert_eq!(run("dict getwithdefault {a 1 b 2} a -1").1, "1");
    // arity: needs dict + key + default.
    let (ok, msg, _) = run("dict getdef {a 1} a");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"dict getdef dictionary ?key ...? key default\""
    );
}

/// dict read errors: a missing key, a malformed (odd) dict *with* a key lookup,
/// and the `dict create` arity error.
#[test]
fn dict_read_errors() {
    // tclsh: missing key.
    let (ok, msg, _) = run("dict get {a 1} z");
    assert!(!ok);
    assert_eq!(msg, "key \"z\" not known in dictionary");
    // tclsh: a key lookup against an odd-length dict errors on the malformed dict.
    let (ok, msg, _) = run("dict get {a 1 b} a");
    assert!(!ok);
    assert_eq!(msg, "missing value to go with key");
    // tclsh: dict create odd args.
    let (ok, msg, _) = run("dict create a");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"dict create ?key value ...?\""
    );
}

/// BUG: `dict get` with NO keys does not validate that its argument is a
/// well-formed dict. tclsh always parses the dict first, so `dict get` of an
/// odd-length value errors; the VM's shared `get` core returns the value
/// unchanged when the key path is empty, so the malformed value slips through.
///   script:        dict get {a 1 b}
///   tclsh (8.6+9.0): missing value to go with key   (error)
///   VM (wrong):      a 1 b                           (ok, returns it verbatim)
/// This test asserts the correct tclsh behaviour, now fixed (guards regression).
#[test]
fn dict_get_no_keys_validates_dict_bug() {
    let (ok, msg, _) = run("dict get {a 1 b}");
    assert!(
        !ok,
        "malformed dict must error even with no keys, got ok: {msg}"
    );
    assert_eq!(msg, "missing value to go with key");
}

// dict — variable-mutating side

/// `dict set` writes (creating intermediate dicts for a key path) and stores the
/// dict back into the variable.
#[test]
fn dict_set() {
    // tclsh: append a new key.
    assert_eq!(
        run("set d {a 1 b 2}\ndict set d c 3\nset d").1,
        "a 1 b 2 c 3"
    );
    // tclsh: a key path creates the intermediate dict.
    assert_eq!(run("set d {a 1}\ndict set d x y z\nset d").1, "a 1 x {y z}");
    // arity.
    let (ok, msg, _) = run("dict set d");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"dict set dictVarName key ?key ...? value\""
    );
}

/// `dict unset` removes a key (a path descends), storing the dict back.
#[test]
fn dict_unset() {
    // tclsh: remove a key.
    assert_eq!(run("set d {a 1 b 2}\ndict unset d a\nset d").1, "b 2");
    // tclsh: removing a nested key keeps the sub-dict.
    assert_eq!(
        run("set d {p {a 1 b 2}}\ndict unset d p a\nset d").1,
        "p {b 2}"
    );
    // arity (no keys).
    let (ok, msg, _) = run("dict unset d");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"dict unset dictVarName key ?key ...?\""
    );
}

/// `dict incr` adds to (or creates) an integer value; a non-integer current
/// value errors.
#[test]
fn dict_incr() {
    // tclsh: increment existing.
    assert_eq!(run("set d {a 1 b 2}\ndict incr d a 5\nset d").1, "a 6 b 2");
    // tclsh: default increment 1, creating the key from 0.
    assert_eq!(run("set d {}\ndict incr d a\nset d").1, "a 1");
    // tclsh: non-integer value → coercion error.
    let (ok, msg, _) = run("set d {a x}\ndict incr d a");
    assert!(!ok);
    assert_eq!(msg, "expected integer but got \"x\"");
}

/// `dict incr` promotes through the same integer tower as `incr`/`expr`
/// (`value_ops::int_add`): a sum past `i64` becomes the exact bignum, never
/// the old "integer value too large to represent" error.
#[test]
fn dict_incr_promotes_past_wide() {
    // tclsh 8.6/9.0: the default-increment (compiled `DICT_INCR_IMM`) path
    // at i64::MAX promotes.
    assert_eq!(
        run("set d {k 9223372036854775807}\ndict incr d k\nset d").1,
        "k 9223372036854775808"
    );
    // tclsh: two i64::MAX increments accumulate exactly (the increment is
    // beyond the opcode's immediate, driving the command path).
    assert_eq!(
        run("set d {}\ndict incr d k 9223372036854775807\n\
             dict incr d k 9223372036854775807\ndict get $d k")
        .1,
        "18446744073709551614"
    );
    // tclsh: a bignum increment (past i64) is itself legal and exact.
    assert_eq!(
        run("set d {k 5}\ndict incr d k 18446744073709551616\ndict get $d k").1,
        "18446744073709551621"
    );
    // tclsh: a stored bignum value keeps promoting.
    assert_eq!(
        run("set d {k 18446744073709551616}\ndict incr d k\ndict get $d k").1,
        "18446744073709551617"
    );
}

/// `dict append` concatenates strings to the value (creating it empty);
/// `dict lappend` appends list elements.
#[test]
fn dict_append_lappend() {
    // tclsh: append strings.
    assert_eq!(run("set d {a 1}\ndict append d a xyz\nset d").1, "a 1xyz");
    // tclsh: append to a missing key starts empty.
    assert_eq!(run("set d {}\ndict append d k ab cd\nset d").1, "k abcd");
    // tclsh: lappend list elements.
    assert_eq!(
        run("set d {a 1}\ndict lappend d a 2 3\nset d").1,
        "a {1 2 3}"
    );
    // tclsh: lappend to a missing key starts an empty list.
    assert_eq!(run("set d {}\ndict lappend d k a b\nset d").1, "k {a b}");
}

/// `dict for {k v} dict body` iterates pairs in order; `break`/`continue` apply.
#[test]
fn dict_for() {
    // tclsh: iteration order is insertion order.
    assert_eq!(
        run("set d {a 1 b 2 c 3}\nset r {}\ndict for {k v} $d {lappend r $k$v}\nset r").1,
        "a1 b2 c3"
    );
    // arity / var-count.
    let (ok, msg, _) = run("dict for {k} {a 1} {}");
    assert!(!ok);
    assert_eq!(msg, "must have exactly two variable names");
}

/// BUG: the *compiled* form of `dict map` fails. The codegen rewrites
/// `dict map` to the qualified ensemble member `::tcl::dict::map` (its registry
/// spec carries `cfg_rewrite_name`), but `cmd_dict::register` only installs
/// `::tcl::dict::for` among the rewritten members — `::tcl::dict::map` is never
/// registered — so the compiled call hits an unresolved command. (The indirect
/// form via a dynamic command word, which is not rewritten, works fine — see
/// `dict_map_indirect_path_works`.)
///   script:        set d {a 1 b 2}; dict map {k v} $d {expr {$v*2}}
///   tclsh (8.6+9.0): a 2 b 4
///   VM (wrong):      invalid command name `::tcl::dict::map`
/// This test asserts the correct tclsh behaviour, now fixed (guards regression).
#[test]
fn dict_map_compiled_path_bug() {
    let (ok, result, _) = run("set d {a 1 b 2}\ndict map {k v} $d {expr {$v*2}}");
    assert!(ok, "dict map should run, got: {result}");
    assert_eq!(result, "a 2 b 4");
}

/// `dict map`'s loop semantics (the `cmd_dict_map` body — value mapping, key
/// re-keying, `continue` drop) exercised through the indirect dispatch path (a
/// dynamic command word, which the codegen does not rewrite to the missing
/// `::tcl::dict::map`). This covers the command's logic while the compiled entry
/// point is broken (see `dict_map_compiled_path_bug`). The `break` case is a
/// separate bug (see `dict_map_break_returns_empty_bug`).
#[test]
fn dict_map_indirect_path_works() {
    // tclsh: map values.
    assert_eq!(
        run("set c dict\nset d {a 1 b 2}\n$c map {k v} $d {expr {$v*2}}").1,
        "a 2 b 4"
    );
    // tclsh: continue drops a pair.
    assert_eq!(
        run("set c dict\nset d {a 1 b 2}\n$c map {k v} $d {if {$v==1} continue; expr {$v*2}}").1,
        "b 4"
    );
    // tclsh: a body that rewrites the key var re-keys the output.
    assert_eq!(
        run(
            "set c dict\nset d {a 1 b 2}\n$c map {k v} $d {set k [string toupper $k]; expr {$v*10}}"
        )
        .1,
        "A 10 B 20"
    );
}

/// BUG: `dict map` with `break` in the body must discard *all* accumulated pairs
/// and return the empty dict (C `DictMapNRCmd` drops the result on `TCL_BREAK`).
/// The VM's `cmd_dict_map` instead returns the pairs collected before the break.
/// (Exercised through the indirect dispatch path so it is not masked by the
/// separate compiled-`dict map` bug.)
///   script:        set d {a 1 b 2 c 3}; dict map {k v} $d {if {$v==2} break; set v}
///   tclsh (8.6+9.0): "" (empty)
///   VM (wrong):      a 1   (the pre-break accumulation)
/// This test asserts the correct tclsh behaviour, now fixed (guards regression).
#[test]
fn dict_map_break_returns_empty_bug() {
    let (ok, result, _) =
        run("set c dict\nset d {a 1 b 2 c 3}\n$c map {k v} $d {if {$v==2} break; set v}");
    assert!(ok, "should run: {result}");
    assert_eq!(result, "", "break must discard the whole result");
}

/// `dict filter` (the `script` filter type) keeps each pair whose body is true;
/// the `key`/`value` glob types route through the shared core.
#[test]
fn dict_filter() {
    // tclsh: script filter keeps v>1.
    assert_eq!(
        run("dict filter {a 1 b 2 c 3} script {k v} {expr {$v>1}}").1,
        "b 2 c 3"
    );
    // tclsh: key glob.
    assert_eq!(
        run("dict filter {apple 1 banana 2 avocado 3} key a*").1,
        "apple 1 avocado 3"
    );
    // tclsh: value glob.
    assert_eq!(run("dict filter {a 1 b 2 c 1} value 1").1, "a 1 c 1");
    // tclsh: bad filterType.
    let (ok, msg, _) = run("dict filter {a 1} bogus");
    assert!(!ok);
    assert_eq!(
        msg,
        "bad filterType \"bogus\": must be key, script, or value"
    );
}

/// `dict update dictVar key var ?...? body` binds keys to vars, runs the body,
/// then reflects the vars back (an unset var removes the key).
#[test]
fn dict_update() {
    // tclsh: modify through the bound var.
    assert_eq!(
        run("set d {a 1 b 2}\ndict update d a x {set x 99}\nset d").1,
        "a 99 b 2"
    );
    // tclsh: unset the bound var removes the key; a second var is updated.
    assert_eq!(
        run("set d {a 1 b 2}\ndict update d a x b y {unset x; set y 88}\nset d").1,
        "b 88"
    );
    // arity (no key/var pairs).
    let (ok, msg, _) = run("dict update d body");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"dict update dictVarName key varName ?key varName ...? script\""
    );
}

/// `dict with dictVar ?key ...? body` maps keys to locals, runs the body, and
/// reflects them back: modified keys update, unset keys are removed, a body-only
/// var is not added. A nested key path updates the sub-dict in place.
#[test]
fn dict_with() {
    // tclsh: modify a mapped local.
    assert_eq!(
        run("set d {a 1 b 2}\ndict with d {set a 100}\nset d").1,
        "a 100 b 2"
    );
    // tclsh: nested key path.
    assert_eq!(
        run("set d {p {a 1 b 2}}\ndict with d p {set a 9}\nset d").1,
        "p {a 9 b 2}"
    );
    // tclsh: a missing dict variable errors.
    let (ok, msg, _) = run("dict with nope {}");
    assert!(!ok);
    assert_eq!(msg, "can't read \"nope\": no such variable");
}

/// An unknown `dict` subcommand reports the VM's bad-subcommand message.
///
/// Issue #1607: `dict` is a `TclMakeEnsemble` command, so the whole sentence
/// — which this used to emit without any `must be` clause — and the
/// exact-then-unique-prefix scan come from `tcl_cmd_core::ensemble`. The list
/// is C's 9.0 table minus `info` (per-dict hash statistics), which this engine
/// does not model.
///
/// tclsh 9.0.4 (the verdicts, not the shortened list):
///   dict k {a 1}         -> a      (keys)
///   dict g {a 1} a       -> unknown or ambiguous subcommand "g": must be …
///                                  (get/getdef/getwithdefault)
///   dict {} {a 1}        -> unknown or ambiguous subcommand "": must be …
///   dict filter {a 1} k *  -> a 1
///   dict filter {a 1} x *  -> bad filterType "x": must be key, script, or value
///   dict filter {a 1} {} * -> ambiguous filterType "": must be key, script, or value
#[test]
fn dict_bad_subcommand() {
    const MUST: &str = "must be append, create, exists, filter, for, get, getdef, \
                        getwithdefault, incr, keys, lappend, map, merge, remove, replace, \
                        set, size, unset, update, values, or with";
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    assert_eq!(
        msg("dict no_such_sub x"),
        format!("unknown or ambiguous subcommand \"no_such_sub\": {MUST}")
    );
    assert_eq!(
        msg("dict g {a 1} a"),
        format!("unknown or ambiguous subcommand \"g\": {MUST}")
    );
    assert_eq!(
        msg("dict {} {a 1}"),
        format!("unknown or ambiguous subcommand \"\": {MUST}")
    );
    // Unique prefixes resolve.
    assert_eq!(run("dict k {a 1}").1, "a");
    assert_eq!(run("dict si {a 1}").1, "1");
    // `dict filter`'s type word is its own `Tcl_GetIndexFromObj` table.
    assert_eq!(run("dict filter {a 1} k *").1, "a 1");
    assert_eq!(
        msg("dict filter {a 1} x *"),
        "bad filterType \"x\": must be key, script, or value"
    );
    assert_eq!(
        msg("dict filter {a 1} {} *"),
        "ambiguous filterType \"\": must be key, script, or value"
    );
}

// list family

/// `list`/`llength`/`lindex` basics, including nested and `end`-relative index.
#[test]
fn list_llength_lindex() {
    // tclsh: list quoting.
    assert_eq!(run("list x {a b} y").1, "x {a b} y");
    assert_eq!(run("llength {a b c}").1, "3");
    assert_eq!(run("llength {}").1, "0");
    assert_eq!(run("lindex {a b c} 1").1, "b");
    assert_eq!(run("lindex {a b c} end").1, "c");
    assert_eq!(run("lindex {a b c} end-1").1, "b");
    // tclsh: arithmetic and radix index operands (shared `index` grammar).
    assert_eq!(run("lindex {a b c d e} 1+1").1, "c");
    assert_eq!(run("lindex {a b c d e} 0x2").1, "c");
    assert_eq!(run("lindex {a b c d e} end--1").1, ""); // end+1 → out of range
    // tclsh: nested lindex.
    assert_eq!(run("lindex {{a b} {c d}} 1 0").1, "c");
    // arity.
    let (ok, msg, _) = run("lindex");
    assert!(!ok);
    assert_eq!(msg, "wrong # args: should be \"lindex list ?index ...?\"");
    // bad index.
    let (ok, msg, _) = run("lindex {a b c} foo");
    assert!(!ok);
    assert!(msg.contains("bad index"), "got: {msg}");
}

/// `lrange` with plain and `end`-relative indices, plus its arity error.
#[test]
fn list_lrange() {
    // tclsh.
    assert_eq!(run("lrange {a b c d e} 1 3").1, "b c d");
    assert_eq!(run("lrange {a b c} end-1 end").1, "b c");
    let (ok, msg, _) = run("lrange a b");
    assert!(!ok);
    assert_eq!(msg, "wrong # args: should be \"lrange list first last\"");
}

/// `lappend` builds a list in a variable; the no-values form reads it back
/// (validating it as a list).
#[test]
fn list_lappend() {
    // tclsh.
    assert_eq!(run("lappend y a b\nlappend y c\nset y").1, "a b c");
    // no-values read of an existing var returns it unchanged.
    assert_eq!(run("set s {a b}\nlappend s").1, "a b");
    // arity.
    let (ok, msg, _) = run("lappend");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"lappend varName ?value ...?\""
    );
}

/// `lassign` binds list elements to variables and returns the unassigned tail.
#[test]
fn list_lassign() {
    // tclsh: `set r [lassign {1 2 3} x y]` → x=1 y=2, r="3".
    assert_eq!(run("lassign {1 2 3} x y\nset x").1, "1");
    assert_eq!(run("lassign {1 2 3} x y").1, "3");
    // Fewer elements than vars → the extra var is empty, tail empty.
    assert_eq!(run("lassign {1} x y\nlist [set x] [set y]").1, "1 {}");
    // arity.
    let (ok, msg, _) = run("lassign");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"lassign list ?varName ...?\""
    );
}

/// `lreverse`/`lrepeat`, with arity / count errors.
#[test]
fn list_lreverse_lrepeat() {
    // tclsh.
    assert_eq!(run("lreverse {1 2 3}").1, "3 2 1");
    assert_eq!(run("lrepeat 3 a b").1, "a b a b a b");
    let (ok, msg, _) = run("lreverse");
    assert!(!ok);
    assert_eq!(msg, "wrong # args: should be \"lreverse list\"");
    // tclsh: negative count.
    let (ok, msg, _) = run("lrepeat -3 1");
    assert!(!ok);
    assert_eq!(msg, "bad count \"-3\": must be integer >= 0");
}

/// `linsert` at plain, `end`, negative, and the per-arg insert positions.
#[test]
fn list_linsert() {
    // tclsh.
    assert_eq!(run("linsert {a b c} 1 X Y").1, "a X Y b c");
    assert_eq!(run("linsert {a b c} end Z").1, "a b c Z");
    // tclsh: a negative index clamps to the front.
    assert_eq!(run("linsert {a b c} -1 X").1, "X a b c");
    let (ok, msg, _) = run("linsert");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"linsert list index ?element ...?\""
    );
}

/// `lreplace` replaces a range, deletes (no replacement), inserts (first>last),
/// and clamps an over-long last index.
#[test]
fn list_lreplace() {
    // tclsh.
    assert_eq!(run("lreplace {a b c d} 1 2 X").1, "a X d");
    assert_eq!(run("lreplace {a b c} end end").1, "a b");
    // tclsh: first>last inserts before `first` without deleting.
    assert_eq!(run("lreplace {a b c} 1 0 X").1, "a X b c");
    // tclsh: an over-long last index clamps to the end.
    assert_eq!(run("lreplace {a b c} 1 10 X").1, "a X");
    let (ok, msg, _) = run("lreplace");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"lreplace list first last ?element ...?\""
    );
}

/// `concat`/`join`/`split` basics.
#[test]
fn list_concat_join_split() {
    // tclsh.
    assert_eq!(run("concat {a b} {c d}").1, "a b c d");
    assert_eq!(run("join {a b c} -").1, "a-b-c");
    assert_eq!(run("join {a b c}").1, "a b c"); // default sep is a space
    assert_eq!(run("split a,b,c ,").1, "a b c");
    assert_eq!(run("split abc {}").1, "a b c"); // empty splitChars → per char
    // arity.
    let (ok, msg, _) = run("join");
    assert!(!ok);
    assert_eq!(msg, "wrong # args: should be \"join list ?joinString?\"");
    let (ok, msg, _) = run("split");
    assert!(!ok);
    assert_eq!(msg, "wrong # args: should be \"split string ?splitChars?\"");
}

// lsearch

/// `lsearch` default (exact-ish glob over the whole element) and `-exact`,
/// returning the first match index or -1.
#[test]
fn lsearch_basic() {
    // tclsh.
    assert_eq!(run("lsearch {a b c b} b").1, "1");
    assert_eq!(run("lsearch {a b c} z").1, "-1");
    assert_eq!(run("lsearch -exact {a ab abc} ab").1, "1");
}

/// `lsearch` option matrix: `-all`, `-inline`, `-not`, `-glob`, `-regexp`,
/// `-start`, `-integer`, `-sorted`, `-index`.
#[test]
fn lsearch_options() {
    // tclsh.
    assert_eq!(run("lsearch -all {a b c b} b").1, "1 3");
    assert_eq!(run("lsearch -inline {x ya zb} y*").1, "ya");
    assert_eq!(run("lsearch -all -inline {ab ac bd} a*").1, "ab ac");
    assert_eq!(run("lsearch -not -all -inline {1 2 3} 2").1, "1 3");
    assert_eq!(run("lsearch -all -not {1 2 1 3} 1").1, "1 3");
    assert_eq!(run("lsearch -start 2 {a b a b} a").1, "2");
    assert_eq!(run("lsearch -integer {3 5 7} 5").1, "1");
    assert_eq!(run("lsearch -sorted {1 3 5 7 9} 7").1, "3");
    assert_eq!(run("lsearch -regexp {abc def ghi} {d.f}").1, "1");
    assert_eq!(run("lsearch -index 1 {{a 1} {b 2}} 2").1, "1");
}

// lsort

/// `lsort` comparison modes: default ascii, `-integer`, `-real`, `-dictionary`,
/// `-ascii`, `-nocase`, and the `-decreasing`/`-unique` modifiers.
#[test]
fn lsort_modes() {
    // tclsh.
    assert_eq!(run("lsort -integer {3 1 2}").1, "1 2 3");
    assert_eq!(run("lsort -decreasing -integer {3 1 2}").1, "3 2 1");
    assert_eq!(
        run("lsort -real -decreasing {1.5 2.5 0.5}").1,
        "2.5 1.5 0.5"
    );
    assert_eq!(run("lsort -dictionary {x10 x2 x1}").1, "x1 x2 x10");
    assert_eq!(run("lsort -ascii {B a C}").1, "B C a");
    assert_eq!(run("lsort -unique {a b a c b}").1, "a b c");
}

/// BUG: `lsort -unique` retains the WRONG member of an equal run. Tcl keeps the
/// *last* element of each duplicate group (in input order); the VM keeps the
/// *first*. Distinct elements under the mode are unaffected, so this only shows
/// when two inputs compare equal but render differently.
///   script:        lsort -nocase -unique {A a B b}
///   tclsh (8.6+9.0): a b      (keeps the later `a`/`b`)
///   VM (wrong):      A B      (keeps the earlier `A`/`B`)
/// Also: `lsort -integer -unique {01 1}` → tclsh `1`, VM `01`.
/// This test asserts the correct tclsh behaviour, now fixed (guards regression).
#[test]
fn lsort_unique_retains_last_duplicate_bug() {
    // tclsh8.6/9.0: a b
    assert_eq!(run("lsort -nocase -unique {A a B b}").1, "a b");
    // tclsh8.6/9.0: A   (input order a, A → keeps the later A)
    assert_eq!(run("lsort -nocase -unique {a A}").1, "A");
    // tclsh8.6/9.0: 1   (input order 01, 1 → keeps the later 1's rendering)
    assert_eq!(run("lsort -integer -unique {01 1}").1, "1");
}

/// `lsort` structural options: `-index`, `-stride`, `-indices`, and the
/// `-command` comparator (which evaluates Tcl through the VM dispatcher).
#[test]
fn lsort_index_stride_command() {
    // tclsh: sort sublists by an element index.
    assert_eq!(
        run("lsort -index 0 {{c 3} {a 1} {b 2}}").1,
        "{a 1} {b 2} {c 3}"
    );
    // tclsh: -stride groups, -index keys within the group.
    assert_eq!(
        run("lsort -stride 2 -index 0 {b 2 a 1 c 3}").1,
        "a 1 b 2 c 3"
    );
    // tclsh: -indices returns the permutation.
    assert_eq!(run("lsort -indices {c a b}").1, "1 2 0");
    // tclsh: -command runs the comparator.
    assert_eq!(
        run("lsort -command {apply {{a b} {expr {$a - $b}}}} {3 1 2}").1,
        "1 2 3"
    );
}

// lset / lpop / ledit (runtime builtins)

/// The runtime `lset` builtin (the fallback the compiler does not inline,
/// reached here via a dynamic command word). It descends a flat index path,
/// a single-index-list path, and replaces the whole value with an empty path.
#[test]
fn lset_runtime_builtin() {
    // tclsh: single flat index.
    assert_eq!(run("set l {a b c}\nset c lset\n$c l 1 X\nset l").1, "a X c");
    // tclsh: nested flat path.
    assert_eq!(
        run("set l {{a b} {c d}}\nset c lset\n$c l 0 1 X\nset l").1,
        "{a X} {c d}"
    );
    // tclsh: empty index path replaces the whole value.
    assert_eq!(run("set l {a b c}\nset c lset\n$c l {} Z\nset l").1, "Z");
    // tclsh: usage error (too few args) via the indirect path.
    let (ok, msg, _) = run("set c lset\n$c");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"lset listVar ?index? ?index ...? value\""
    );
}

/// `lpop listVar ?index ...?` removes and returns an element (default the last),
/// descending into nested sublists, and stores the trimmed list back (Tcl 9.0).
#[test]
fn lpop_runtime_builtin() {
    // tclsh9.0: default removes the last element.
    assert_eq!(
        run("set l {a b c d}\nset g [lpop l]\nlist $g $l").1,
        "d {a b c}"
    );
    // tclsh9.0: a nested index path removes the deepest element.
    assert_eq!(
        run("set l {{a b} {c d}}\nset g [lpop l 0 1]\nlist $g $l").1,
        "b {a {c d}}"
    );
    // tclsh9.0: out-of-range index.
    let (ok, msg, _) = run("set l {a b c}\nlpop l 99");
    assert!(!ok);
    assert_eq!(msg, "index \"99\" out of range");
    // tclsh9.0: non-integer index.
    let (ok, msg, _) = run("set l {a b c}\nlpop l foo");
    assert!(!ok);
    assert_eq!(
        msg,
        "bad index \"foo\": must be integer?[+-]integer? or end?[+-]integer?"
    );
    // tclsh9.0: missing variable.
    let (ok, msg, _) = run("lpop nope");
    assert!(!ok);
    assert_eq!(msg, "can't read \"nope\": no such variable");
    // arity.
    let (ok, msg, _) = run("lpop");
    assert!(!ok);
    assert_eq!(msg, "wrong # args: should be \"lpop listvar ?index?\"");
}

/// `ledit listVar first last ?element ...?` is the in-place `lreplace` (Tcl 9.0):
/// it edits the list in the variable and stores it back.
#[test]
fn ledit_runtime_builtin() {
    // tclsh9.0: replace the 1..1 range with two elements.
    assert_eq!(
        run("set l {1 2 3 4 5}\nledit l 1 1 a b\nset l").1,
        "1 a b 3 4 5"
    );
    // tclsh9.0: a missing variable errors.
    let (ok, msg, _) = run("ledit nope 0 0 x");
    assert!(!ok);
    assert_eq!(msg, "can't read \"nope\": no such variable");
    // arity.
    let (ok, msg, _) = run("ledit l 0");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"ledit listVar first last ?element ...?\""
    );
}

// lsort / lsearch error paths

/// `lsort` error/edge paths: bad option, missing list, a non-numeric element
/// under `-integer`, and the `-command` comparator's empty / non-integer
/// failures (the comparator runs Tcl through the VM dispatcher).
#[test]
fn lsort_error_paths() {
    // tclsh (8.6+9.0): bad option (the VM's option set matches tclsh9.0's list).
    let (ok, msg, _) = run("lsort -badopt {1 2}");
    assert!(!ok);
    assert_eq!(
        msg,
        "bad option \"-badopt\": must be -ascii, -command, -decreasing, -dictionary, -increasing, -index, -indices, -integer, -nocase, -real, -stride, or -unique"
    );
    // tclsh: arity.
    let (ok, msg, _) = run("lsort");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"lsort ?-option value ...? list\""
    );
    // tclsh: a non-integer element under -integer.
    let (ok, msg, _) = run("lsort -integer {a b}");
    assert!(!ok);
    assert_eq!(msg, "expected integer but got \"a\"");
    // An empty -command prefix errors before any comparison (VM-specific wording).
    let (ok, msg, _) = run("lsort -command {} {1 2}");
    assert!(!ok, "empty -command should error");
    assert!(msg.contains("comparison command is empty"), "got: {msg}");
    // tclsh: a -command that returns a non-integer errors. The VM reports its own
    // wording (tclsh says "-compare command returned non-integer result"); the
    // behaviour — erroring on a non-integer comparator result — matches.
    let (ok, msg, _) = run("lsort -command {apply {{a b} {return x}}} {1 2}");
    assert!(!ok, "non-integer comparator result should error");
    assert!(msg.contains("non-integer"), "got: {msg}");
}

/// `lsearch` error paths: bad option and the arity error.
#[test]
fn lsearch_error_paths() {
    // tclsh (the VM's option set matches tclsh9.0's list, which includes -stride).
    let (ok, msg, _) = run("lsearch -badopt {a b} x");
    assert!(!ok);
    assert_eq!(
        msg,
        "bad option \"-badopt\": must be -all, -ascii, -bisect, -decreasing, -dictionary, -exact, -glob, -increasing, -index, -inline, -integer, -nocase, -not, -real, -regexp, -sorted, -start, -stride, or -subindices"
    );
    // tclsh: arity.
    let (ok, msg, _) = run("lsearch");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"lsearch ?-option value ...? list pattern\""
    );
}

/// `lsort` option words abbreviate like tclsh's `Tcl_GetIndexFromObj` (flags
/// 0): an exact match wins, a unique prefix resolves, and a shared prefix —
/// including the empty word, which prefixes everything — is an *ambiguous*
/// option. The VM previously demanded exact names (S4.2; probed tclsh 8.6.14,
/// flags + table order verified in the 9.0.4 source).
#[test]
fn lsort_option_abbreviation() {
    // tclsh: `lsort -uniq {b a b}` → `a b`; `-u` is already unique.
    assert_eq!(run("lsort -uniq {b a b}").1, "a b");
    assert_eq!(run("lsort -u {b a b}").1, "a b");
    // tclsh: `lsort -decr {b a c}` → `c b a`.
    assert_eq!(run("lsort -decr {b a c}").1, "c b a");
    // tclsh: a value-taking option abbreviates too (`-inde` → `-index`), and
    // its missing-value error names the canonical option.
    assert_eq!(run("lsort -inde 1 {{a 3} {b 1}}").1, "{b 1} {a 3}");
    let (ok, msg, _) = run("lsort -inde {a b}");
    assert!(!ok);
    assert_eq!(msg, "\"-index\" option must be followed by list index");
    // tclsh: `-d` prefixes both -decreasing and -dictionary.
    let (ok, msg, _) = run("lsort -d {b a}");
    assert!(!ok);
    assert_eq!(
        msg,
        "ambiguous option \"-d\": must be -ascii, -command, -decreasing, -dictionary, -increasing, -index, -indices, -integer, -nocase, -real, -stride, or -unique"
    );
    // tclsh: the empty word is ambiguous, not bad.
    let (ok, msg, _) = run("lsort \"\" {b a}");
    assert!(!ok);
    assert!(
        msg.starts_with("ambiguous option \"\": must be"),
        "got: {msg}"
    );
}

/// The same abbreviation rule for `lsearch` (S4.2): `-exa`/`-inl`/`-gl`
/// resolve; `-in` and the empty word are ambiguous. The enumeration is
/// tclsh9.0's table (which includes `-stride`).
#[test]
fn lsearch_option_abbreviation() {
    // tclsh: `-exa` → -exact, `-inl` → -inline, `-gl` → -glob.
    assert_eq!(run("lsearch -exa {a b c} b").1, "1");
    assert_eq!(run("lsearch -inl {a b c} b").1, "b");
    assert_eq!(run("lsearch -gl {a b c} b*").1, "1");
    // tclsh: `-in` prefixes -increasing/-index/-inline/-integer.
    let (ok, msg, _) = run("lsearch -in {a b} a");
    assert!(!ok);
    assert_eq!(
        msg,
        "ambiguous option \"-in\": must be -all, -ascii, -bisect, -decreasing, -dictionary, -exact, -glob, -increasing, -index, -inline, -integer, -nocase, -not, -real, -regexp, -sorted, -start, -stride, or -subindices"
    );
    // tclsh: the empty word is ambiguous, not bad.
    let (ok, msg, _) = run("lsearch \"\" {a b} a");
    assert!(!ok);
    assert!(
        msg.starts_with("ambiguous option \"\": must be"),
        "got: {msg}"
    );
}

// dict — additional edge / error paths

/// `dict with` reflects an unset mapped variable as a key removal, and does not
/// add a variable the body merely created (the reflect-back logic in
/// `cmd_dict_with`).
#[test]
fn dict_with_reflects_unset_and_ignores_new() {
    // tclsh: modified `a`, removed `b` (unset), new `c` not added back.
    assert_eq!(
        run("set d {a 1 b 2}\ndict with d {set a 9; unset b; set c 3}\nset d").1,
        "a 9"
    );
}

/// `dict with` runs the write-back even when the body raises, then propagates
/// the error (the `outcome`/`Code::Error` arm of `cmd_dict_with`).
#[test]
fn dict_with_body_error_writes_back_then_propagates() {
    // tclsh: the variable update survives the error; the error propagates.
    let (ok, msg, _) =
        run("set d {a 1 b 2}\ncatch {dict with d {set a 9; error boom}}\ndict get $d a");
    assert!(ok, "script should complete: {msg}");
    assert_eq!(msg, "9");
    let (ok, msg, _) = run("set d {a 1}\ndict with d {error boom}");
    assert!(!ok);
    assert_eq!(msg, "boom");
}

/// `dict incr` rejects a non-integer *increment* argument up front.
#[test]
fn dict_incr_bad_increment() {
    // tclsh: `dict incr d a xyz` → expected integer but got "xyz".
    let (ok, msg, _) = run("set d {a 1}\ndict incr d a xyz");
    assert!(!ok);
    assert_eq!(msg, "expected integer but got \"xyz\"");
}

/// `dict lappend` validates the existing value as a list before appending.
#[test]
fn dict_lappend_bad_list() {
    // tclsh: an unbalanced-brace value → unmatched open brace in list.
    let (ok, msg, _) = run("set d [list a \"\\{\"]\ndict lappend d a x");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
}

/// `dict for`/`map`/`filter`/`update` var-list / arity validation (the early
/// error arms of each command).
#[test]
fn dict_iter_arity_errors() {
    // dict map: two var names required.
    let (ok, msg, _) = run("set c dict\n$c map {k} {a 1} {}");
    assert!(!ok);
    assert_eq!(msg, "must have exactly two variable names");
    // dict filter (script): two var names required.
    let (ok, msg, _) = run("dict filter {a 1} script {k} {}");
    assert!(!ok);
    assert_eq!(msg, "must have exactly two variable names");
    // dict update: needs at least one key/var pair.
    let (ok, msg, _) = run("dict update d k");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"dict update dictVarName key varName ?key varName ...? script\""
    );
    // dict with: arity (no body).
    let (ok, msg, _) = run("dict with");
    assert!(!ok);
    assert_eq!(
        msg,
        "wrong # args: should be \"dict with dictVarName ?key ...? script\""
    );
}

// list-family runtime builtins (indirect dispatch)

/// The `list`/`l*` commands reached through their *runtime builtins* (the
/// compiler inlines the common forms to opcodes; a dynamic command word forces
/// the builtin entry points in `cmd_list.rs`). Each is checked against tclsh.
#[test]
fn list_builtins_via_indirect_dispatch() {
    // tclsh: each returns the same as its compiled form.
    assert_eq!(run("set c list\n$c a b c").1, "a b c");
    assert_eq!(run("set c llength\n$c {a b c}").1, "3");
    assert_eq!(run("set c lindex\n$c {a b c} 1").1, "b");
    assert_eq!(run("set c lrange\n$c {a b c d} 1 2").1, "b c");
    assert_eq!(run("set c lappend\nset l {}\n$c l a b\nset l").1, "a b");
    assert_eq!(run("set c lassign\n$c {1 2 3} x y\nset x").1, "1");
    assert_eq!(run("set c lreverse\n$c {1 2 3}").1, "3 2 1");
    assert_eq!(run("set c lrepeat\n$c 3 x").1, "x x x");
    assert_eq!(run("set c linsert\n$c {a b} 1 X").1, "a X b");
    assert_eq!(run("set c lreplace\n$c {a b c} 1 1 X").1, "a X c");
    assert_eq!(run("set c concat\n$c a {b c}").1, "a b c");
    assert_eq!(run("set c join\n$c {a b c} -").1, "a-b-c");
    assert_eq!(run("set c split\n$c a,b ,").1, "a b");
}

/// `lappend` of an unset variable with no values creates it as the empty string
/// (the `None` branch of `cmd_lappend`'s no-values arm).
#[test]
fn lappend_unset_no_values_creates_empty() {
    // tclsh: `lappend fresh` (unset, no values) → fresh exists and is empty.
    assert_eq!(
        run("set c lappend\n$c fresh\nlist [info exists fresh] \"[set fresh]<\"").1,
        "1 <"
    );
}

/// `dict for`/`filter`/`update`/`map` loop-control and error propagation: `break`
/// stops, `continue` skips, and a body error aborts the command (after the
/// write-back, for `update`). These hit the `Code::Break`/`_ => return c`/`Err`
/// arms of each loop.
#[test]
fn dict_loop_control_and_errors() {
    // dict for: break stops after the first kept key.
    assert_eq!(
        run("set d {a 1 b 2 c 3}\nset r {}\ndict for {k v} $d {if {$v==2} break; lappend r $k}\nset r").1,
        "a"
    );
    // dict for: continue skips a pair.
    assert_eq!(
        run("set d {a 1 b 2 c 3}\nset r {}\ndict for {k v} $d {if {$v==2} continue; lappend r $k}\nset r").1,
        "a c"
    );
    // dict for: a body error propagates.
    let (ok, msg, _) = run("set d {a 1}\ndict for {k v} $d {error boom}");
    assert!(!ok);
    assert_eq!(msg, "boom");
    // dict filter (script): continue / break / error.
    assert_eq!(
        run("dict filter {a 1 b 2 c 3} script {k v} {if {$v==2} continue; expr 1}").1,
        "a 1 c 3"
    );
    assert_eq!(
        run("dict filter {a 1 b 2 c 3} script {k v} {if {$v==2} break; expr 1}").1,
        "a 1"
    );
    let (ok, msg, _) = run("dict filter {a 1} script {k v} {error boom}");
    assert!(!ok);
    assert_eq!(msg, "boom");
    // dict update: a body error writes back the binding, then propagates.
    let (ok, msg, _) =
        run("set d {a 1 b 2}\ncatch {dict update d a x {error boom}}\ndict get $d a");
    assert!(ok, "{msg}");
    assert_eq!(msg, "1");
    let (ok, msg, _) = run("set d {a 1 b 2}\ndict update d a x {error boom}");
    assert!(!ok);
    assert_eq!(msg, "boom");
    // dict update: a nested-dict edit through the bound var is reflected back.
    assert_eq!(
        run("set d {a {x 1}}\ndict update d a sub {dict set sub y 2}\nset d").1,
        "a {x 1 y 2}"
    );
    // dict map (indirect): a body error propagates.
    let (ok, msg, _) = run("set c dict\nset d {a 1 b 2}\n$c map {k v} $d {error boom}");
    assert!(!ok);
    assert_eq!(msg, "boom");
}

/// The list-builtin arity errors reached via the runtime path (the `_ =>` arms
/// the compiler's inlined forms never reach).
#[test]
fn list_builtins_arity_errors() {
    // tclsh.
    let (ok, msg, _) = run("set c llength\n$c");
    assert!(!ok);
    assert_eq!(msg, "wrong # args: should be \"llength list\"");
    let (ok, msg, _) = run("set c lrange\n$c a");
    assert!(!ok);
    assert_eq!(msg, "wrong # args: should be \"lrange list first last\"");
    let (ok, msg, _) = run("set c lreverse\n$c a b");
    assert!(!ok);
    assert_eq!(msg, "wrong # args: should be \"lreverse list\"");
}

// lmap — inline collecting loop

/// A straight-line `lmap` body lowers to the inline collecting loop: it strips
/// the body's trailing `POP`, appends each iteration's result via `LMAP_COLLECT`,
/// and `FOREACH_END` yields the mapped list. Values pinned against tclsh 9.0.4.
#[test]
fn lmap_collects_mapped_values() {
    // A single-variable map (last statement's result is the mapped value).
    assert_eq!(run("lmap x {1 2 3} {expr {$x * $x}}").1, "1 4 9");
    // A multi-word body: only the final result is collected.
    assert_eq!(
        run("lmap x {a b c} {set y hi; string cat $x $x}").1,
        "aa bb cc"
    );
}

/// Empty and degenerate lists: an empty value list yields the empty string, and a
/// result-less (empty) body still collects one empty string per iteration.
#[test]
fn lmap_empty_and_resultless_body() {
    assert_eq!(run("lmap x {} {expr {$x * 2}}").1, "");
    assert_eq!(run("lmap x {1 2 3} {}").1, "{} {} {}");
}

/// Multi-variable groups iterate in lock-step; a short final group pads with the
/// empty string (tclsh 9.0.4), and multiple lists advance together.
///
/// (A *nested* `lmap` — an inner loop directly inside the outer body — is
/// barriered to the runtime builtin on the bytecode path, so it is covered by the
/// `tcl-vm-cli` corpus rather than here: this helper lowers with the analysis
/// `lower_to_ir`, where that barrier is inactive.)
#[test]
fn lmap_multivar_and_multilist() {
    assert_eq!(run("lmap {a b} {1 2 3 4} {expr {$a + $b}}").1, "3 7");
    assert_eq!(run("lmap {a b} {1 2 3} {list $a $b}").1, "{1 2} {3 {}}");
    assert_eq!(
        run("lmap a {1 2 3} b {10 20 30} {expr {$a + $b}}").1,
        "11 22 33"
    );
}

/// Issue #1427 — a dict with a **duplicate key** is canonicalised the way
/// `SetDictFromAny` (tclDictObj.c(9.0.4):589) does it: the pair keeps its
/// first-occurrence position and the **last** value wins, because the
/// `Tcl_DictObjPut` behind it overwrites on a hash collision.
///
/// Three VM paths used to decode the list rep straight into `chunks_exact(2)`
/// pairs and then `find` the *first* match, so they read and rewrote the
/// stale value: the `Op::DICT_GET` opcode, `dict_set_path` (and the
/// `dictUpdateStart`/`dictExpand`/`dictRecombine` prologues that share it),
/// and `cmd_dict`'s command-side `pairs`. All of them now start from the one
/// `ValueOps::dict_pairs` owner.
///
/// Every expectation below is the byte-exact output of `tclsh9.0.4` (and of
/// `tclsh8.6.16` for the subcommands 8.6 has).
#[test]
fn duplicate_dict_keys_canonicalise_last_value_wins() {
    // `Op::DICT_GET` — the inline opcode shape (`set s [dict get …]` inside a
    // proc is what the codegen lowers to `dictGet`, unlike `return [dict get …]`
    // which stays a generic invoke). Both must agree, and both answer 2.
    assert_eq!(
        run("proc j {} {set d {a 1 a 2}; set s [dict get $d a]; return $s}\nj").1,
        "2"
    );
    assert_eq!(
        run("proc j {} {set d {a 1 a 2}; return [dict get $d a]}\nj").1,
        "2"
    );
    // `dict set` rewrites the canonical pair, it does not shadow it.
    assert_eq!(
        run("proc k {} {set d {a 1 a 2}; dict set d a 9; return $d}\nk").1,
        "a 9"
    );
    // `dict update` binds the *canonical* value (2), not the first (1).
    assert_eq!(
        run(
            "proc l {} {set d {a 1 a 2}; dict update d a v {set v [expr {$v*100+1}]}; return $v}\nl"
        )
        .1,
        "201"
    );
    // `dict with` expands the canonical value into the local.
    assert_eq!(
        run("proc m {} {set d {a 1 a 2}; dict with d {}; return \"$a $a\"}\nm").1,
        "2 2"
    );
    // `dict for` / `dict size` see one pair, not two.
    assert_eq!(
        run("proc n {} {set d {a 1 a 2}; set c 0; dict for {kk vv} $d {incr c}; return $c}\nn").1,
        "1"
    );
    assert_eq!(run("dict size {a 1 a 2}").1, "1");
    assert_eq!(run("dict keys {a 1 a 2}").1, "a");
    // `dict unset` removes the whole (single) pair.
    assert_eq!(
        run("proc q {} {set d {a 1 a 2}; dict unset d a; return \"[dict size $d]|$d\"}\nq").1,
        "0|"
    );
    // The read-modify-write trio all start from the canonical value.
    assert_eq!(
        run("proc r {} {set d {a 1 a 2}; dict incr d a 5; return $d}\nr").1,
        "a 7"
    );
    assert_eq!(
        run("proc s {} {set d {a 1 a 2}; dict append d a X; return $d}\ns").1,
        "a 2X"
    );
    assert_eq!(
        run("proc t {} {set d {a 1 a 2}; dict lappend d a X; return $d}\nt").1,
        "a {2 X}"
    );
    // Reads that never went through a mutating path agree with the rest.
    assert_eq!(run("dict getdef {a 1 a 2} a Z").1, "2");
    assert_eq!(run("dict exists {a 1 a 2} a").1, "1");
    // Position is first-occurrence, so a later duplicate does not move the key.
    assert_eq!(run("dict replace {a 1 b 2 a 3} c 4").1, "a 3 b 2 c 4");

    // `dict create` — the *compile-time fold*, not the runtime path. An
    // all-literal `[dict create …]` is constant-folded in codegen, and the fold
    // used to render its arguments as a plain list join, freezing the duplicate
    // into the literal. `dict size`/`dict get` still answered correctly (they
    // re-canonicalise on the way in), so only the **string representation**
    // showed it — which is exactly what a `puts` or `string length` sees.
    //
    // The dynamic spelling defeats the fold and always took the correct path;
    // the two must agree.
    assert_eq!(run("string length [dict create a 1 a 2]").1, "3");
    assert_eq!(run("dict create a 1 a 2").1, "a 2");
    assert_eq!(run("set n 2\ndict create a 1 a $n").1, "a 2");
    assert_eq!(run("dict create x 1 x 2 y 3").1, "x 2 y 3");
    assert_eq!(run("string length [dict create x 1 x 2 y 3]").1, "7");
    // First-occurrence position holds through the fold too.
    assert_eq!(run("dict create a 1 b 2 a 3").1, "a 3 b 2");
    // A non-duplicate fold is unchanged, and an odd arg count still errors.
    assert_eq!(run("dict create a 1 b 2").1, "a 1 b 2");
    assert!(!run("dict create a 1 b").0);
}

/// Issue #1573 — a dict *value-parse* failure is reported as a *dict*, not a
/// list.
///
/// C's `SetDictFromAny` hands `FindElement` the type strings `dict`/
/// `DICTIONARY`, so the same malformed input that `llength` calls a
/// `list element …` / `TCL VALUE LIST JUNK` problem, `dict size` calls a
/// `dict element …` / `TCL VALUE DICTIONARY JUNK` one. The VM decoded dicts
/// through the (list-worded) shared codec and passed its message straight out,
/// so every dict path used the list noun and the list `errorCode`.
///
/// Byte-checked against `tclsh9.0.4`.
#[test]
fn dict_parse_errors_use_the_dict_noun_and_error_code() {
    let check = |script: &str, msg: &str, code: &str| {
        let (ok, r, _) = run(script);
        assert!(!ok, "{script} should fail");
        assert_eq!(r, msg, "{script}");
        let (ok, _r, out) = run(&format!(
            "catch {{{script}}} m\nputs -nonewline $::errorCode"
        ));
        assert!(ok);
        assert_eq!(out, code, "{script} errorCode");
    };
    check(
        "dict size {a 1 {b}c d}",
        "dict element in braces followed by \"c\" instead of space",
        "TCL VALUE DICTIONARY JUNK",
    );
    check(
        "dict size {a 1 \"b\"c d}",
        "dict element in quotes followed by \"c\" instead of space",
        "TCL VALUE DICTIONARY JUNK",
    );
    // Quoted so the *script* parser hands `dict` the unbalanced text verbatim
    // rather than balancing it itself.
    check(
        "dict size \"a 1 \\{b\"",
        "unmatched open brace in dict",
        "TCL VALUE DICTIONARY BRACE",
    );
    check(
        "dict size {a 1 b}",
        "missing value to go with key",
        "TCL VALUE DICTIONARY",
    );
    // The *list* noun is untouched — the two must stay distinguishable.
    check(
        "llength {a 1 {b}c d}",
        "list element in braces followed by \"c\" instead of space",
        "TCL VALUE LIST JUNK",
    );
    // A non-parse dict error must NOT be reclassified as a dict-value error.
    let (ok, r, _) = run("dict get {a 1} zz");
    assert!(!ok);
    assert_eq!(r, "key \"zz\" not known in dictionary");
}

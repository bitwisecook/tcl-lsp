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

//! List, array and dictionary builtins and their compiled inline forms.

use super::*;

#[test]
fn list_basics() {
    out_eq("set l [list 1 2 3]\nputs [llength $l]\n", "3\n");
    out_eq("puts [lindex {a b c} 1]\n", "b\n");
    out_eq("puts [lindex {a b c} end]\n", "c\n");
    out_eq("puts [lrange {a b c d} 1 2]\n", "b c\n");
    out_eq("puts [lreverse {1 2 3}]\n", "3 2 1\n");
    out_eq("puts [concat a {b c} d]\n", "a b c d\n");
    out_eq("puts [join {a b c} -]\n", "a-b-c\n");
    out_eq("puts [llength [split a,b,c ,]]\n", "3\n");
    out_eq("puts [lsort -integer {3 1 2 10}]\n", "1 2 3 10\n");
    out_eq("puts [lsearch {a b c} b]\n", "1\n");
}

#[test]
fn lappend_and_lassign() {
    out_eq("set l {}\nlappend l a b\nlappend l c\nputs $l\n", "a b c\n");
    out_eq("lassign {1 2 3} x y\nputs \"$x $y\"\n", "1 2\n");
}

/// `lsearch` — the VM gained the full option set by sharing
/// `tcl_cmd_core::lsearch` (it had only a `-exact`/`-glob` stub). Pinned to tclsh.
#[test]
fn lsearch_shared_core() {
    assert_eq!(run("lsearch {a b c d} c").1, "2");
    assert_eq!(run("lsearch {a b c d} x").1, "-1");
    assert_eq!(run("lsearch -all {a b a c a} a").1, "0 2 4");
    assert_eq!(run("lsearch -inline {foo bar baz} ba*").1, "bar");
    assert_eq!(run("lsearch -all -inline {x1 y2 x3} x*").1, "x1 x3");
    assert_eq!(run("lsearch -not {a b a} a").1, "1");
    assert_eq!(run("lsearch -start 2 {a b a a} a").1, "2");
    assert_eq!(run("lsearch -nocase {AB cd EF} ef").1, "2");
    assert_eq!(run("lsearch -integer {3 1 4 1 5} 4").1, "2");
    assert_eq!(run("lsearch -sorted {1 3 5 7 9} 7").1, "3");
    assert_eq!(run("lsearch -bisect -integer {2 4 6 8} 5").1, "1");
    assert_eq!(run("lsearch -regexp {foo123 bar456} {[0-9]+}").1, "0");
    assert_eq!(run("lsearch -index 1 {{a 1} {b 2} {c 3}} 2").1, "1");
    assert_eq!(run("lsearch -all -index 1 {{a 1} {b 2} {c 1}} 1").1, "0 2");
    assert_eq!(run("lsearch -subindices -index 1 {{a 1} {b 2}} 2").1, "1 1");
    assert_eq!(run("lsearch -stride 2 -index 0 {a 1 b 2 c 3} b").1, "2");
    assert_eq!(run("lsearch -index end {{a b} {c d}} d").1, "1");
    // bad option error (full message now).
    let (ok, msg, _) = run("lsearch -bogus {a b} a");
    assert!(!ok);
    assert!(msg.starts_with("bad option \"-bogus\""), "got: {msg}");
}

/// `lsort` — the VM gained `-index`/`-stride`/`-indices`/`-command` by sharing
/// `tcl_cmd_core::lsort` (it had only flat comparison modes). Pinned to tclsh.
#[test]
fn lsort_shared_core() {
    assert_eq!(run("lsort {b a c}").1, "a b c");
    assert_eq!(
        run("lsort -decreasing -dictionary {x9 x10 x100}").1,
        "x100 x10 x9"
    );
    assert_eq!(run("lsort -integer -unique {1 01 1 2}").1, "1 2");
    assert_eq!(run("lsort -indices {c a b}").1, "1 2 0");
    assert_eq!(
        run("lsort -index 1 {{a 3} {b 1} {c 2}}").1,
        "{b 1} {c 2} {a 3}"
    );
    assert_eq!(
        run("lsort -stride 2 -index 1 {x 3 y 1 z 2}").1,
        "y 1 z 2 x 3"
    );
    assert_eq!(
        run("lsort -stride 2 -indices {c 3 a 1 b 2}").1,
        "2 3 4 5 0 1"
    );
    // -command (Family-B: the comparator evaluates Tcl via vm.dispatch).
    assert_eq!(
        run("lsort -command {apply {{a b} {expr {$a - $b}}}} {3 1 2}").1,
        "1 2 3"
    );
    assert_eq!(
        run("lsort -unique -command {apply {{a b} {expr {$a - $b}}}} {3 1 3 2 1}").1,
        "1 2 3"
    );
    // Errors.
    let (ok, msg, _) = run("lsort -index 5 {{a b} {c d}}");
    assert!(!ok);
    assert_eq!(msg, "element 5 missing from sublist \"a b\"");
}

#[test]
fn arrays() {
    out_eq("set a(x) 1\nset a(y) 2\nputs [array size a]\n", "2\n");
    out_eq(
        "set a(x) 1\nset a(y) 2\nputs [lsort [array names a]]\n",
        "x y\n",
    );
    out_eq("set a(x) 10\nputs $a(x)\n", "10\n");
    out_eq("array set a {p 1 q 2}\nputs [array get a p]\n", "p 1\n");
    out_eq("puts [array exists nope]\n", "0\n");
}

/// `array` exists/size/names/get/unset route through the shared
/// `tcl_cmd_core::array` core (over the VM's `VarStore`). Pinned to tclsh 9.0.
#[test]
fn array_shared_core() {
    assert_eq!(run("array set a {x 1 y 2 z 3}; array exists a").1, "1");
    assert_eq!(run("array set a {x 1 y 2 z 3}; array size a").1, "3");
    assert_eq!(
        run("array set a {x 1 y 2 z 3}; lsort [array names a]").1,
        "x y z"
    );
    assert_eq!(
        run("array set a {ax 1 ay 2 bz 3}; lsort [array names a a*]").1,
        "ax ay"
    );
    assert_eq!(
        run("array set a {x 1 y 2 z 3}; lsort [array get a]").1,
        "1 2 3 x y z"
    );
    assert_eq!(
        run("array set a {x 1 y 2 z 3}; array unset a y; lsort [array names a]").1,
        "x z"
    );
    // `array unset a` (no pattern) removes the *whole* array.
    assert_eq!(
        run("array set a {x 1 y 2 z 3}; array unset a; array exists a").1,
        "0"
    );
    // A scalar is not an array; a missing var is not an array.
    assert_eq!(run("set s scalar; array exists s").1, "0");
    assert_eq!(run("array exists nope").1, "0");
    assert_eq!(run("array size nope").1, "0");
}

#[test]
fn dict_ops() {
    out_eq("set d [dict create a 1 b 2]\nputs [dict get $d b]\n", "2\n");
    out_eq("puts [dict exists [dict create a 1] a]\n", "1\n");
    out_eq("puts [dict exists [dict create a 1] z]\n", "0\n");
    out_eq("puts [dict size [dict create a 1 b 2 c 3]]\n", "3\n");
    out_eq("puts [lsort [dict keys [dict create a 1 b 2]]]\n", "a b\n");
    out_eq("set d {}\ndict set d k v\nputs [dict get $d k]\n", "v\n");
    out_eq(
        "set sum 0\ndict for {k v} {a 1 b 2 c 3} { incr sum $v }\nputs $sum\n",
        "6\n",
    );
    // `replace` / `remove` (shared command core; the VM gained these via the
    // `dispatch_canon` seam).
    out_eq("puts [dict replace {a 1 b 2} b 3 c 4]\n", "a 1 b 3 c 4\n");
    out_eq("puts [dict remove {a 1 b 2 c 3} b d]\n", "a 1 c 3\n");
    out_eq("puts [dict getdef {a 1 b 2} a X]\n", "1\n");
    out_eq("puts [dict getdef {a 1 b 2} z X]\n", "X\n");
}

/// `dict filter` — `key`/`value` globs via the shared core, `script` via the
/// VM's Family-B adapter (the VM lacked `dict filter` entirely). Pinned to tclsh.
#[test]
fn dict_filter() {
    assert_eq!(run("dict filter {a 1 b 2 aa 3} key a*").1, "a 1 aa 3");
    assert_eq!(run("dict filter {a 1 b 2 aa 3} value 2").1, "b 2");
    assert_eq!(run("dict filter {a 1 b 2} key").1, ""); // no patterns → empty
    assert_eq!(run("dict filter {a 1 b 2 c 3} key a c").1, "a 1 c 3"); // any-of
    // script mode (Family-B): keep pairs whose body is true.
    assert_eq!(
        run("dict filter {a 1 b 2 c 3} script {k v} {expr {$v > 1}}").1,
        "b 2 c 3"
    );
    // filterType is validated before the dict is parsed (was a runtime bug).
    let (ok, msg, _) = run("dict filter {a b c} bogus");
    assert!(!ok);
    assert_eq!(
        msg,
        "bad filterType \"bogus\": must be key, script, or value"
    );
    let (ok, msg, _) = run("dict filter {a 1 b 2} script {k} {expr 1}");
    assert!(!ok);
    assert_eq!(msg, "must have exactly two variable names");
}

#[test]
fn linsert_lreplace_inline() {
    out_eq("puts [linsert {a c} 1 b]\n", "a b c\n");
    out_eq("puts [linsert {a b} end c]\n", "a b c\n");
    out_eq("puts [lreplace {a b c d} 1 2 X]\n", "a X d\n");
    out_eq("puts [lreplace {a b c} 1 1]\n", "a c\n");
    out_eq("puts [linsert {1 2 3} 0 0]\n", "0 1 2 3\n");
}

#[test]
fn dict_incr_append_lappend() {
    out_eq(
        "set d [dict create a 1]\ndict incr d a 5\nputs [dict get $d a]\n",
        "6\n",
    );
    out_eq(
        "set d {}\ndict append d k foo\ndict append d k bar\nputs [dict get $d k]\n",
        "foobar\n",
    );
    out_eq(
        "set d {}\ndict lappend d k 1 2\nputs [dict get $d k]\n",
        "1 2\n",
    );
}

#[test]
fn array_element_incr_append_lappend() {
    // incr/append/lappend must resolve `arr(key)` to the array element, both
    // at top level (incrArrayStkImm) and on proc locals (invokeStk builtins).
    out_eq("set a(x) 1\nincr a(x)\nputs $a(x)\n", "2\n");
    out_eq("set a(x) 5\nincr a(x) 3\nputs $a(x)\n", "8\n");
    out_eq(
        "proc f {} { set a(x) 1; incr a(x); return $a(x) }\nputs [f]\n",
        "2\n",
    );
    out_eq(
        "proc f {} { set a(x) hi; append a(x) bye; return $a(x) }\nputs [f]\n",
        "hibye\n",
    );
    out_eq(
        "proc f {} { set a(x) 1; lappend a(x) 2 3; return $a(x) }\nputs [f]\n",
        "1 2 3\n",
    );
}

#[test]
fn lappend_no_values_preserves_string_rep() {
    // `lappend var` with no values returns the variable unchanged — it does NOT
    // re-render the list, so a leading `#` element keeps its bare form instead
    // of being requoted to `{#}` (Tcl shimmer-validates but never reformats).
    out_eq("set lst \"# 1 2 3\"\nputs [lappend lst]\n", "# 1 2 3\n");
    out_eq(
        "set z \"  spaced   out  \"\nputs <[lappend z]>\n",
        "<  spaced   out  >\n",
    );
    // Appending values DOES canonicalise as usual.
    out_eq("set x {1 2 3}\nputs [lappend x 4]\n", "1 2 3 4\n");
    // An unset variable is created as the empty string.
    out_eq(
        "puts <[lappend brandnew]>\nputs [info exists brandnew]\n",
        "<>\n1\n",
    );
}

#[test]
fn list_element_quoting_balanced_braces() {
    // Balanced braces inside an element stay bare; `]`/`"` escape; `[`/`$` brace.
    out_eq("puts [list a{b}c b{} d]\n", "a{b}c b{} d\n");
    out_eq("set e {a]b}\nputs [list $e]\n", "a\\]b\n");
    out_eq("set f {a[b}\nputs [list $f]\n", "{a[b}\n");
}

#[test]
fn list_constant_fold_decodes_and_quotes() {
    // Constant-folding `[list …]` must decode quoted/bare argument values
    // (`\x00` → NUL, `\t` → tab) and treat braced ones verbatim, then requote.
    out_eq(
        "puts [string equal [list \"\" \"\\x00\" \"\\x00\\x00\"] \"{} \\x00 \\x00\\x00\"]\n",
        "1\n",
    );
    out_eq(
        "puts [string equal [list \"\\x00abc\" xyz] \"\\x00abc xyz\"]\n",
        "1\n",
    );
    // A single-element fold whose result is brace-quoted must not be mistaken
    // for a braced literal and stripped at runtime.
    out_eq("puts [string length [list \"a b\"]]\n", "5\n");
    out_eq("puts [string length [list \"a\\tb\"]]\n", "5\n");
}

/// Compiled proc-local `unset a(k)` (the `unsetArray` opcode) must delete the
/// array element, not no-op on a base-name lookup.
#[test]
fn compiled_unset_array_element_removes_it() {
    out_eq(
        "proc f {} { set a(k) v; unset a(k); return [info exists a(k)] }\nputs [f]\n",
        "0\n",
    );
    // The rest of the array is untouched.
    out_eq(
        "proc f {} { set a(k) v; set a(j) w; unset a(k); return [info exists a(j)] }\nputs [f]\n",
        "1\n",
    );
}

/// Compiled `unset` of an absent variable without `-nocomplain` must raise the
/// Tcl error, while `-nocomplain` stays silent.
#[test]
fn compiled_unset_missing_honours_complain_flag() {
    let (ok, result, _) = run("proc f {} { unset nope }\nf\n");
    assert!(!ok, "unset of a missing var should error");
    assert!(
        result.contains("can't unset \"nope\""),
        "unexpected error: {result}"
    );

    // -nocomplain succeeds silently.
    out_eq(
        "proc f {} { unset -nocomplain nope; return done }\nputs [f]\n",
        "done\n",
    );
}

#[test]
fn dict_for_compiled_inline_executes() {
    // dict for compiled inline (dictFirst/dictNext) iterates in order.
    out_eq(
        "proc p {} { set d {a 1 b 2 c 3}; set out {}; dict for {k v} $d { lappend out $k=$v }; return $out }\nputs [p]\n",
        "a=1 b=2 c=3\n",
    );
    // Empty dict: body never runs.
    out_eq(
        "proc p {} { set d {}; set n 0; dict for {k v} $d { incr n }; return $n }\nputs [p]\n",
        "0\n",
    );
    // Value + key both bound; sum the values.
    out_eq(
        "proc p {} { set d {x 10 y 20}; set s 0; dict for {k v} $d { incr s $v }; return $s }\nputs [p]\n",
        "30\n",
    );
}

#[test]
fn dict_map_compiled_inline_executes() {
    // dict map doubles each value, keyed by k.
    out_eq(
        "proc p {} { set d {a 1 b 2 c 3}; return [dict map {k v} $d { expr {$v * 2} }] }\nputs [p]\n",
        "a 2 b 4 c 6\n",
    );
    // Empty dict → empty result.
    out_eq(
        "proc p {} { set d {}; return [dict map {k v} $d { expr {$v + 1} }] }\nputs [len [p]]\n"
            .replace("len ", "llength ")
            .as_str(),
        "0\n",
    );
    // Body using both key and value.
    out_eq(
        "proc p {} { set d {x 1 y 2}; return [dict map {k v} $d { list $k $v }] }\nputs [p]\n",
        "x {x 1} y {y 2}\n",
    );
}

#[test]
fn dict_update_compiled_inline_executes() {
    // Two keys mutated in the body flow back into the dict.
    out_eq(
        "proc p {} { set d {a 1 b 2}; dict update d a x b y { set x [expr {$x*10}]; set y [expr {$y*10}] }; return $d }\nputs [p]\n",
        "a 10 b 20\n",
    );
    // The `dict update` value is the body's result.
    out_eq(
        "proc p {} { set d {a 1 b 2}; return [dict update d a x { set x 99 }] }\nputs [p]\n",
        "99\n",
    );
    // A key absent from the dict leaves its target unset; setting it adds the key.
    out_eq(
        "proc p {} { set d {a 1}; dict update d a x c z { set z 7 }; return $d }\nputs [p]\n",
        "a 1 c 7\n",
    );
}

#[test]
fn dict_with_compiled_inline_executes() {
    // Keys become locals; the body reads them, the dict is unchanged.
    out_eq(
        "proc p {} { set d {a 1 b 2}; set r [dict with d { expr {$a+$b} }]; return [list $d $r] }\nputs [p]\n",
        "{a 1 b 2} 3\n",
    );
    // Mutating a key-local flows back into the dict.
    out_eq(
        "proc p {} { set d {a 1 b 2}; dict with d { set a 100 }; return $d }\nputs [p]\n",
        "a 100 b 2\n",
    );
    // A brand-new local set in the body is folded in only if it names a key;
    // an unrelated local does not extend the dict.
    out_eq(
        "proc p {} { set d {a 1}; dict with d { set a 5; set other 9 }; return $d }\nputs [p]\n",
        "a 5\n",
    );
}

/// A control-flow body is not straight-line, so `dict update`/`dict with` fall
/// back to the runtime `dict` invoke — which must still execute correctly.
#[test]
fn dict_update_with_control_flow_body_falls_back_and_executes() {
    out_eq(
        "proc p {} { set d {a 1 b 2}; dict update d a x b y { if {$x > 0} { set x [expr {$x+$y}] } }; return $d }\nputs [p]\n",
        "a 3 b 2\n",
    );
    out_eq(
        "proc q {} { set d {a 1 b 2}; dict with d { foreach k {a b} { }; set a 9 }; return $d }\nputs [q]\n",
        "a 9 b 2\n",
    );
}

/// A `{k v}` variable word is a Tcl list, not whitespace-delimited: a
/// 1-element list like `{{a b}}` must reach the runtime `dict for` and error,
/// never be miscompiled into two inline loop vars. Regression for the
/// `split_whitespace` var-list parsing bug.
#[test]
fn dict_for_map_malformed_var_list_errors_via_fallback() {
    // `{{a b}}` is one element → "must have exactly two variable names".
    let (ok, result, _out) = run("proc p {} { set d {x 1}; dict for {{a b}} $d { puts hi } }\np\n");
    assert!(!ok, "malformed dict for var list must error");
    assert!(
        result.contains("exactly two variable names"),
        "got: {result}"
    );
    let (ok, result, _out) = run("proc p {} { set d {x 1}; dict map {{a b}} $d { set x 1 } }\np\n");
    assert!(!ok, "malformed dict map var list must error");
    assert!(
        result.contains("exactly two variable names"),
        "got: {result}"
    );
}

/// A straight-line body whose final statement is `return` leaves no trailing
/// `pop`, so the inline emitter must bail out *cleanly* (rolling back its
/// partial prologue and catch depth) and let the runtime `dict` invoke run —
/// where `return` keeps its proc-exit semantics. Regression guard for the
/// mid-emission fallback corruption.
#[test]
fn dict_inline_return_body_falls_back_cleanly() {
    // dict map over an empty dict never runs the body → empty result.
    out_eq(
        "proc a {} { dict map {k v} {} { return 5 } }\nputs \"[a]|\"\n",
        "|\n",
    );
    // dict update / dict with run the body once; `return` exits the proc.
    out_eq(
        "proc b {} { set d {x 1}; dict update d x q { return 9 } }\nputs \"[b]|\"\n",
        "9|\n",
    );
    out_eq(
        "proc c {} { set d {x 1}; dict with d { return 7 } }\nputs \"[c]|\"\n",
        "7|\n",
    );
}

#[test]
fn dictionary_original_fallback_preserves_order_and_rebinding() {
    out_eq(
        "set trace {}\nproc side {x} {global trace; lappend trace $x; return $x}\nset d {a 2}\ndict incr [side d] [side a] [side 5]\nputs [list $trace $d]\n",
        "{d a 5} {a 7}\n",
    );
    out_eq(
        "set trace {}\nproc side {x} {global trace; lappend trace $x; return $x}\nset code [catch {dict incr [side d] [side a] [error stop]} result]\nputs [list $code $result $trace]\n",
        "1 stop {d a}\n",
    );
    out_eq(
        "rename ::tcl::dict::incr ::tcl::dict::oldincr\nproc ::tcl::dict::incr args {return [list override {*}$args]}\nputs [dict incr d a 5]\n",
        "override d a 5\n",
    );
}

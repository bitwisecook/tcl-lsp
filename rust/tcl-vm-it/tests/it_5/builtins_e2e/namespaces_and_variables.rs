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

//! Namespaces, ensembles, `upvar` links, variable traces and namespace variables.

use super::*;

/// Ensemble-resolved body commands execute correctly through the compiled
/// bytecode. `namespace eval` compiles to `invokeReplace … ::tcl::namespace::
/// eval` and `array for` to `invokeStk ::tcl::array::for`; both resolved
/// implementations must be registered in the VM (regression guard for the
/// ensemble-resolution codegen).
#[test]
fn ensemble_resolved_body_commands_execute() {
    // namespace eval — the invokeReplace form.
    out_eq(
        "namespace eval ::ns { variable c 5 }\nputs [set ::ns::c]\n",
        "5\n",
    );
    // namespace eval with a nested proc + call.
    out_eq(
        "namespace eval ::m { proc greet {} { return hi } }\nputs [::m::greet]\n",
        "hi\n",
    );
    // array for — the resolved ::tcl::array::for invoke; body iterates entries.
    out_eq(
        "array set a {x 1 y 2}\nset out {}\narray for {k v} a { lappend out $v }\nputs [lsort $out]\n",
        "1 2\n",
    );
}

/// `namespace exists`/`parent`/`children` route through the shared core over
/// the `Namespaces` handle trait (the VM's String model honouring `NsId`),
/// which gives `children` its `?pattern?` filter and the missing-namespace
/// error. Pinned to tclsh 9.0.
#[test]
fn namespace_nav_shared() {
    let setup = "namespace eval a { namespace eval b {}; namespace eval c {} }; ";
    assert_eq!(run(&format!("{setup}namespace parent ::a")).1, "::");
    assert_eq!(run(&format!("{setup}namespace parent ::a::b")).1, "::a");
    assert_eq!(
        run(&format!("{setup}lsort [namespace children ::a]")).1,
        "::a::b ::a::c"
    );
    // `children` now honours the pattern (was ignored), qualified to the target.
    assert_eq!(
        run(&format!("{setup}lsort [namespace children ::a b*]")).1,
        "::a::b"
    );
    assert_eq!(run(&format!("{setup}namespace exists ::a")).1, "1");
    assert_eq!(run(&format!("{setup}namespace exists ::a::b")).1, "1");
    assert_eq!(run("namespace exists ::nope").1, "0");
    // A missing namespace now errors (was: a computed/empty result).
    let (ok, msg, _) = run("namespace parent ::nope");
    assert!(!ok);
    assert_eq!(msg, "namespace \"::nope\" not found");
    let (ok, msg, _) = run("namespace children ::nope");
    assert!(!ok);
    assert_eq!(msg, "namespace \"::nope\" not found");
}

#[test]
fn namespace_export_import() {
    out_eq(
        "namespace eval foo { namespace export greet\nproc greet {} { return hi }\nproc secret {} { return no } }\nnamespace import ::foo::*\nputs [greet]\n",
        "hi\n",
    );
    // Non-exported commands are not imported.
    out_eq(
        "namespace eval foo { namespace export greet\nproc greet {} { return hi }\nproc secret {} { return no } }\nnamespace import ::foo::*\nputs [catch secret]\n",
        "1\n",
    );
}

#[test]
fn namespace_eval_upvar_alias() {
    // tcltest idiom: a `namespace eval` body aliases an array element to a
    // namespace variable, which a proc then reads/writes via `variable`.
    out_eq(
        "namespace eval foo {\n  variable Opt\n  set Opt(d) 0\n  namespace eval ::foo {upvar 0 Opt(d) dbg}\n  proc r {} { variable dbg; return $dbg }\n}\nputs [foo::r]\n",
        "0\n",
    );
    out_eq(
        "namespace eval foo {\n  variable Opt\n  set Opt(d) 0\n  namespace eval ::foo {upvar 0 Opt(d) dbg}\n  proc setd {v} { variable dbg; set dbg $v }\n  proc r {} { variable Opt; return $Opt(d) }\n}\nfoo::setd 5\nputs [foo::r]\n",
        "5\n",
    );
}

#[test]
fn upvar_to_namespace_array_element() {
    // `upvar` to an array element whose base is a `variable`-linked namespace
    // array resolves through the link.
    out_eq(
        "namespace eval foo { variable Opt\nset Opt(x) hello\nproc np {pv} { upvar 1 $pv path; return $path }\nproc t {} { variable Opt; return [np Opt(x)] } }\nputs [foo::t]\n",
        "hello\n",
    );
}

#[test]
fn array_element_traces_are_distinct() {
    // A trace on `a(x)` fires only for `a(x)`, not `a(y)`.
    out_eq(
        "proc cb {n1 n2 op} { puts $n2 }\ntrace add variable a(x) write cb\nset a(x) 1\nset a(y) 2\n",
        "x\n",
    );
    // A whole-array trace fires for every element.
    out_eq(
        "proc cb {n1 n2 op} { puts \"$n1/$n2\" }\ntrace add variable a write cb\nset a(p) 1\nset a(q) 2\n",
        "a/p\na/q\n",
    );
}

#[test]
fn upvar_to_array_element() {
    // `upvar 0 arr(key) alias` aliases a scalar to an array element.
    out_eq(
        "proc f {} { set a(x) 1; upvar 0 a(x) al; set al 9; return $a(x) }\nputs [f]\n",
        "9\n",
    );
    out_eq(
        "proc f {} { set a(x) 5; upvar 0 a(x) al; return $al }\nputs [f]\n",
        "5\n",
    );
}

#[test]
fn variable_traces() {
    // Write trace fires after the write with `name1 name2 op`.
    out_eq(
        "proc cb {n1 n2 op} { puts \"$n1 $n2 $op\" }\ntrace add variable x write cb\nset x 5\n",
        "x  write\n",
    );
    // Read trace fires before the read and may initialise the variable.
    out_eq(
        "proc cb {n1 n2 op} { set ::x 99 }\ntrace add variable x read cb\nputs $x\n",
        "99\n",
    );
    // Array-element write trace reports name1=array name2=key.
    out_eq(
        "proc cb {n1 n2 op} { puts \"$n1 $n2 $op\" }\ntrace add variable a write cb\nset a(k) 5\n",
        "a k write\n",
    );
    out_eq(
        "trace add variable x write cb\nputs [trace info variable x]\n",
        "{write cb}\n",
    );
    // Op validation and its type error route through the shared catalogue, so
    // an unrecognised op word is rejected rather than accepted.
    let (ok, msg, _) = run("trace add variable v bogus {}");
    assert!(!ok);
    assert_eq!(
        msg,
        "bad operation \"bogus\": must be array, read, unset, or write"
    );
    let (ok, msg, _) = run("trace add variable v {} {}");
    assert!(!ok);
    assert_eq!(
        msg,
        "bad operation list \"\": must be one or more of array, read, unset, or write"
    );
    let (ok, msg, _) = run("trace add bogus n o c");
    assert!(!ok);
    assert_eq!(
        msg,
        "bad option \"bogus\": must be execution, command, or variable"
    );
    // The type word abbreviates (Tcl_GetIndexFromObj, flags 0): tclsh accepts
    // `trace add var …`, and the empty word prefixes all three types, so it
    // is *ambiguous* — not bad (probed tclsh 8.6.14).
    out_eq(
        "trace add var x write cb\nputs [trace info variable x]\n",
        "{write cb}\n",
    );
    let (ok, msg, _) = run("trace add {} n o c");
    assert!(!ok);
    assert_eq!(
        msg,
        "ambiguous option \"\": must be execution, command, or variable"
    );
    // An op word, by contrast, may NOT abbreviate (C resolves op-list elements
    // with TCL_EXACT): tclsh rejects `w` for `write`.
    let (ok, msg, _) = run("trace add variable v w {}");
    assert!(!ok);
    assert_eq!(
        msg,
        "bad operation \"w\": must be array, read, unset, or write"
    );
    // Trace operations are a canonical set, and duplicate registrations are
    // removed one at a time. A scalar registration materialises only an unset
    // cell, so `info exists` stays false until a real write.
    out_eq(
        "trace add variable fresh {write read write} list\ntrace add variable fresh {read write} list\ntrace remove variable fresh {write read} list\nputs \"[info exists fresh]:[trace info variable fresh]\"\ntrace remove variable absent read list\nputs [info exists absent]\n",
        "0:{{read write} list}\n0\n",
    );
    let (ok, msg, _) = run("trace remove command missing delete list");
    assert!(!ok);
    assert_eq!(msg, "unknown command \"missing\"");
    let (ok, msg, _) = run("set scalar x\ntrace add variable scalar(k) read list");
    assert!(!ok);
    assert_eq!(msg, "can't trace \"scalar(k)\": variable isn't array");
}

/// A write-trace error fails the command and wraps the message, but the value
/// stays stored — C swaps the new value in before calling the traces and never
/// puts the old one back (`TclPtrSetVarIdx`, `tclVar.c`); every
/// line below is byte-pinned against tclsh 8.6.16 and 9.0.4.
#[test]
fn write_trace_error_keeps_the_stored_value() {
    out_eq(
        "proc cb {n1 n2 op} { error nope }\nset x 1\ntrace add variable x write cb\ncatch {set x 2} m\nputs \"$m / $x\"\n",
        "can't set \"x\": nope / 2\n",
    );
    // A cell the failing write created survives too.
    out_eq(
        "proc cb {n1 n2 op} { error nope }\ntrace add variable fresh write cb\ncatch {set fresh new} m\nputs \"$m / [info exists fresh] / $fresh\"\n",
        "can't set \"fresh\": nope / 1 / new\n",
    );
    // Array elements take the same path, existing and fresh alike.
    out_eq(
        "proc cb {n1 n2 op} { error nope }\narray set a {k old}\ntrace add variable a(k) write cb\ncatch {set a(k) new} m\nputs \"$m / $a(k)\"\n",
        "can't set \"a(k)\": nope / new\n",
    );
    out_eq(
        "proc cb {n1 n2 op} { error nope }\narray set b {}\ntrace add variable b(j) write cb\ncatch {set b(j) new} m\nputs \"$m / [info exists b(j)] / $b(j)\"\n",
        "can't set \"b(j)\": nope / 1 / new\n",
    );
    // Every other writing command reaches the same chokepoint.
    out_eq(
        "proc cb {n1 n2 op} { error nope }\nset s abc\ntrace add variable s write cb\ncatch {append s def} m\nputs \"$m / $s\"\n",
        "can't set \"s\": nope / abcdef\n",
    );
    out_eq(
        "proc cb {n1 n2 op} { error nope }\nset n 5\ntrace add variable n write cb\ncatch {incr n} m\nputs \"$m / $n\"\n",
        "can't set \"n\": nope / 6\n",
    );
    out_eq(
        "proc cb {n1 n2 op} { error nope }\nset l {a b}\ntrace add variable l write cb\ncatch {lappend l c} m\nputs \"$m / $l\"\n",
        "can't set \"l\": nope / a b c\n",
    );
    // …including when they create the variable.
    out_eq(
        "proc cb {n1 n2 op} { error nope }\ntrace add variable f write cb\ncatch {incr f} m\nputs \"$m / [info exists f] / $f\"\n",
        "can't set \"f\": nope / 1 / 1\n",
    );
}

#[test]
fn namespace_eval_and_procs() {
    out_eq(
        "namespace eval foo { proc bar {} { return hi } }\nputs [foo::bar]\n",
        "hi\n",
    );
    out_eq(
        "namespace eval foo { proc bar {} { return hi } }\nputs [::foo::bar]\n",
        "hi\n",
    );
    // A proc resolves a sibling proc in its own namespace.
    out_eq(
        "namespace eval a { proc f {} { return [g] }\nproc g {} { return in } }\nputs [a::f]\n",
        "in\n",
    );
}

#[test]
fn namespace_introspection() {
    out_eq("puts [namespace current]\n", "::\n");
    out_eq(
        "namespace eval foo { puts [namespace current] }\n",
        "::foo\n",
    );
    out_eq("puts [namespace qualifiers ::a::b::c]\n", "::a::b\n");
    out_eq("puts [namespace tail ::a::b::c]\n", "c\n");
    out_eq(
        "namespace eval foo {}\nputs [namespace exists foo]\n",
        "1\n",
    );
    out_eq("puts [namespace exists nope]\n", "0\n");
}

#[test]
fn namespace_variables() {
    out_eq(
        "namespace eval foo { variable v 42 }\nputs $::foo::v\n",
        "42\n",
    );
    out_eq(
        "namespace eval foo { variable v 42\nproc get {} { variable v; return $v } }\nputs [foo::get]\n",
        "42\n",
    );
    // A namespace variable persists across calls.
    out_eq(
        "namespace eval foo { variable v 1\nproc inc {} { variable v; incr v; return $v } }\nputs [foo::inc][foo::inc]\n",
        "23\n",
    );
    out_eq(
        "namespace eval foo { variable v 5 }\nputs [info exists ::foo::v]\n",
        "1\n",
    );
    out_eq("puts [info exists ::foo::nope]\n", "0\n");
}

#[test]
fn namespace_array_variables() {
    out_eq(
        "namespace eval foo { variable a\nset a(x) 1\nset a(y) 2 }\nputs [lsort [array names ::foo::a]]\n",
        "x y\n",
    );
    // An array namespace variable mutated through a `variable`-linked local,
    // persisting across calls.
    out_eq(
        "namespace eval foo { variable cnt\nset cnt(n) 0\nproc bump {} { variable cnt; incr cnt(n); return $cnt(n) } }\nputs [foo::bump][foo::bump]\n",
        "12\n",
    );
}

#[test]
fn namespace_code_inscope() {
    out_eq(
        "namespace eval foo { variable v 7 }\nputs [namespace inscope ::foo { set v }]\n",
        "7\n",
    );
    out_eq(
        "namespace eval foo { proc cb {} { return [namespace code {set x 1}] } }\nputs [foo::cb]\n",
        "::namespace inscope ::foo {set x 1}\n",
    );
}

#[test]
fn rename_rejects_existing_destination() {
    // Renaming onto an existing command errors (both commands stay intact),
    // matching Tcl; a fresh destination renames normally.
    let (ok, _result, _out) = run("rename set puts\n");
    assert!(!ok, "rename onto an existing command should error");
    out_eq(
        "proc foo {} { return 1 }\nrename foo bar\nputs [bar]\n",
        "1\n",
    );
    out_eq(
        "proc demo {} {}\nrename demo {}\nputs [llength [info commands demo]]\n",
        "0\n",
    );
}

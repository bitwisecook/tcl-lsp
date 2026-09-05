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

//! End-to-end coverage of the builtin command library: list/string/dict/array,
//! namespace, `info`, regexp/regsub, expr, binary/scan, channels, traces, and
//! friends — each compiled as real Tcl via `tcl-compiler` and run through
//! `tcl-vm`, asserting observable behaviour.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

struct Svc(CommandRegistry);
impl CompileService for Svc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let ir = lower_to_ir(src, &self.0);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.0))
    }
}

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

struct SyntheticHost {
    clock: SyntheticClock,
    stdio: SyntheticStdIo,
    env: SyntheticEnv,
}

impl SyntheticHost {
    fn new(entries: &[(&str, &str)]) -> Self {
        Self {
            clock: SyntheticClock,
            stdio: SyntheticStdIo,
            env: SyntheticEnv(
                entries
                    .iter()
                    .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                    .collect(),
            ),
        }
    }
}

struct SyntheticClock;

impl tcl_platform::Clock for SyntheticClock {
    fn now_secs(&self) -> i64 {
        0
    }

    fn now_millis(&self) -> i128 {
        0
    }
}

struct SyntheticStdIo;

impl tcl_platform::StdIo for SyntheticStdIo {
    fn write_stdout(&self, _bytes: &[u8]) {}

    fn write_stderr(&self, _bytes: &[u8]) {}
}

struct SyntheticEnv(BTreeMap<String, String>);

impl tcl_platform::Env for SyntheticEnv {
    fn get(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }

    fn set(&self, _key: &str, _value: &str) {}

    fn vars(&self) -> Vec<(String, String)> {
        self.0
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    fn cwd(&self) -> Result<String, tcl_platform::HostError> {
        Ok("/synthetic".to_string())
    }

    fn chdir(&self, _path: &str) -> Result<(), tcl_platform::HostError> {
        Ok(())
    }
}

impl tcl_platform::Host for SyntheticHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        tcl_platform::Capabilities::empty()
    }

    fn clock(&self) -> &dyn tcl_platform::Clock {
        &self.clock
    }

    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        &self.stdio
    }

    fn env(&self) -> &dyn tcl_platform::Env {
        &self.env
    }
}

fn run(src: &str) -> (bool, String, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(Svc(CommandRegistry::build_default())));
    let c = vm.run_module(&asm);
    let out = String::from_utf8(buf.borrow().clone()).expect("utf-8");
    (c.code.is_ok(), c.result.to_str().to_string(), out)
}

/// Assert a script runs OK and prints `expected`.
fn out_eq(src: &str, expected: &str) {
    let (ok, result, out) = run(src);
    assert!(ok, "script errored: {result}");
    assert_eq!(out, expected, "for script:\n{src}");
}

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

/// The `exec` command, end-to-end through the bytecode pipeline, on both host
/// postures — the capability model proven at the command level (the helper-level
/// proof lives in `capability.rs`).
#[test]
fn exec_command_capability() {
    // Native host (the VM's default): `exec` runs a real subprocess.
    out_eq("puts [exec echo hello]\n", "hello\n");

    // Sandboxed host — subprocess capability off, the posture every WASM/WASI
    // host has. `exec` yields the faithful "unsupported" error, not a panic.
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(Svc(CommandRegistry::build_default())));
    vm.set_host(Rc::new(tcl_vm::host_native::NativeHost::sandboxed()));
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir("exec echo hello", &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);
    let c = vm.run_module(&asm);
    assert!(!c.code.is_ok(), "sandboxed exec should error, got ok");
    assert!(
        c.result.to_str().contains("no subprocess support"),
        "expected the faithful unsupported error, got: {}",
        c.result.to_str()
    );
}

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

/// `namespace exists`/`parent`/`children` now route through the shared core over
/// the `Namespaces` handle trait (the VM's String model honouring `NsId`).
/// Sharing gave `children` its `?pattern?` filter and the missing-namespace
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
fn string_ops() {
    out_eq("puts [string length hello]\n", "5\n");
    out_eq("puts [string index hello 1]\n", "e\n");
    out_eq("puts [string range hello 1 3]\n", "ell\n");
    out_eq("puts [string toupper aBc]\n", "ABC\n");
    out_eq("puts [string trim {  hi  }]\n", "hi\n");
    out_eq("puts [string map {a A o O} banana]\n", "bAnAnA\n");
    out_eq("puts [string match f* foo]\n", "1\n");
    out_eq("puts [string equal abc abc]\n", "1\n");
    out_eq("puts [string repeat ab 3]\n", "ababab\n");
    out_eq("puts [string is integer 42]\n", "1\n");
    out_eq("puts [string is integer 4x]\n", "0\n");
}

#[test]
fn append_builtin() {
    out_eq("set s foo\nappend s bar baz\nputs $s\n", "foobarbaz\n");
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

/// `array` exists/size/names/get/unset now route through the shared
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
    // The fixed bug: `array unset a` (no pattern) removes the *whole* array.
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
fn info_introspection() {
    out_eq("puts [info exists nope]\n", "0\n");
    out_eq("set x 1\nputs [info exists x]\n", "1\n");
    out_eq(
        "proc f {a {b 5}} { return $a }\nputs [info args f]\n",
        "a b\n",
    );
    out_eq(
        "proc f {a {b 5}} { return $a }\nputs [info body f]\n",
        " return $a \n",
    );
    // `info exists` checks variables, so a proc name is 0; the proc is listed
    // by `info procs` (lsearch finds it at index 0 as the only user proc).
    out_eq("proc f {} {}\nputs [info exists f]\n", "0\n");
    out_eq("proc f {} {}\nputs [lsearch [info procs] f]\n", "0\n");
    out_eq("puts [info level]\n", "0\n");
    out_eq("proc g {} { return [info level] }\nputs [g]\n", "1\n");
    out_eq("puts [info tclversion]\n", "9.0\n");
    // body/args/default on a non-proc (unknown or a builtin) → the shared
    // `"name" isn't a procedure` error (pinned against tclsh 9.0).
    let (ok, msg, _) = run("info body nosuch");
    assert!(!ok);
    assert_eq!(msg, "\"nosuch\" isn't a procedure");
    let (ok, msg, _) = run("info args nosuch");
    assert!(!ok);
    assert_eq!(msg, "\"nosuch\" isn't a procedure");
    let (ok, msg, _) = run("info body set");
    assert!(!ok);
    assert_eq!(msg, "\"set\" isn't a procedure");
}

#[test]
fn foreach_single() {
    out_eq("foreach x {a b c} { puts $x }\n", "a\nb\nc\n");
}

#[test]
fn foreach_multi_var() {
    out_eq("foreach {k v} {a 1 b 2} { puts \"$k=$v\" }\n", "a=1\nb=2\n");
}

#[test]
fn foreach_two_lists() {
    out_eq("foreach x {1 2} y {3 4} { puts \"$x$y\" }\n", "13\n24\n");
}

#[test]
fn foreach_uneven_pads_empty() {
    out_eq("foreach {a b} {1 2 3} { puts \"$a-$b\" }\n", "1-2\n3-\n");
}

#[test]
fn foreach_accumulate() {
    out_eq(
        "set s 0\nforeach n {1 2 3 4} { incr s $n }\nputs $s\n",
        "10\n",
    );
}

#[test]
fn foreach_empty_list() {
    out_eq("foreach x {} { puts no }\nputs done\n", "done\n");
}

#[test]
fn unset_command() {
    out_eq("set x 1\nunset x\nputs [info exists x]\n", "0\n");
    out_eq("unset -nocomplain nope\nputs ok\n", "ok\n");
    out_eq("set a(x) 1\nunset a(x)\nputs [info exists a(x)]\n", "0\n");
}

#[test]
fn expr_math_functions() {
    out_eq("puts [expr {abs(-5)}]\n", "5\n");
    out_eq("puts [expr {max(3,7,2)}]\n", "7\n");
    out_eq("puts [expr {min(3,7,2)}]\n", "2\n");
    out_eq("puts [expr {int(3.9)}]\n", "3\n");
    out_eq("puts [expr {sqrt(9.0)}]\n", "3.0\n");
    out_eq("puts [expr {pow(2,10)}]\n", "1024.0\n");
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
fn dynamic_proc_body() {
    // A proc whose name and/or body is built at runtime is compiled on demand.
    out_eq(
        "set n foo\nproc $n {x} { return [expr {$x*2}] }\nputs [foo 21]\n",
        "42\n",
    );
    out_eq("set b {return hi}\nproc dyn {} $b\nputs [dyn]\n", "hi\n");
}

#[test]
fn info_complete() {
    out_eq("puts [info complete {set x 1}]\n", "1\n");
    // `{[a}` is a braced word → value `[a` (unbalanced bracket) → incomplete.
    out_eq("puts [info complete {[a}]\n", "0\n");
    out_eq("puts [info complete {puts hello}]\n", "1\n");
}

#[test]
fn subst_command() {
    out_eq("set x 5\nputs [subst {x is $x}]\n", "x is 5\n");
    out_eq("set x 5\nputs [subst {sum [expr {$x+1}]}]\n", "sum 6\n");
    out_eq("set a(k) v\nputs [subst {got $a(k)}]\n", "got v\n");
    out_eq("puts [subst -novariables {keep $x}]\n", "keep $x\n");
    out_eq("puts [subst -nocommands {keep [cmd]}]\n", "keep [cmd]\n");
    // Per-bracket control flow (subst-8.x/10.x, matched against tclsh 9.0.4):
    // `break` finalises with the text so far, `continue` drops the bracket's
    // value, and `return` (any non-error code) substitutes its result. These
    // exercise the yieldable subst frame's rules via the non-coroutine path.
    out_eq("puts [subst {a[break]b}]\n", "a\n");
    out_eq("puts [subst {a[continue]b}]\n", "ab\n");
    out_eq("puts [subst {a[return -level 0 X]b}]\n", "aXb\n");
    // `-nobackslashes` leaves an escape verbatim; the default decodes it.
    out_eq("puts [subst -nobackslashes {a\\tb}]\n", "a\\tb\n");
    out_eq("puts [subst {x[expr 1][expr 2]y}]\n", "x12y\n");
}

/// Issue #1443 — `subst`'s option words resolve through the one shared
/// `tcl-cmd-core::prefix` matcher, so they word every miss exactly as
/// `Tcl_GetIndexFromObj` at flags `0` does (`TclSubstOptions`,
/// `tclCmdMZ.c:3341`). The **empty** word is the case that used to diverge: it
/// prefixes all three entries, so C calls it `ambiguous`, not `bad`.
#[test]
fn subst_option_words_resolve_like_tcl_get_index_from_obj() {
    const MUST: &str = "must be -nobackslashes, -nocommands, or -novariables";
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    // The empty option word abbreviates every entry ⇒ `ambiguous` (tclsh
    // 8.6.16 and 9.0.4 agree).
    assert_eq!(
        msg("subst {} abc\n"),
        format!("ambiguous option \"\": {MUST}")
    );
    // A word that prefixes more than one entry is likewise ambiguous.
    assert_eq!(
        msg("subst -no abc\n"),
        format!("ambiguous option \"-no\": {MUST}")
    );
    // A word that prefixes nothing is `bad`.
    assert_eq!(msg("subst -q abc\n"), format!("bad option \"-q\": {MUST}"));
    // A unique prefix still resolves (subst-7.7).
    out_eq("puts [subst -nov {$x}]\n", "$x\n");
    out_eq("puts [subst -nob {a\\tb}]\n", "a\\tb\n");
    out_eq("puts [subst -noc {[cmd]}]\n", "[cmd]\n");
}

/// Issue #1443's bug, found repeated verbatim in `interp limit`'s option
/// matcher by the centralisation audit: a hand-rolled `starts_with` filter can
/// only ever say `bad option`, so the empty word — a prefix of *every* entry —
/// reported `bad option ""` where C reports `ambiguous option ""`. Both
/// engines now route through `prefix::OptionTable::abbreviating`, which owns
/// the verdict and the `", or"` enumeration alike.
///
/// Byte-checked against tclsh 8.6.16 and 9.0.4, which agree on every row.
#[test]
fn interp_limit_option_words_resolve_like_tcl_get_index_from_obj() {
    const MUST: &str = "must be -command, -granularity, -milliseconds, or -seconds";
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    assert_eq!(
        msg("interp create i\ninterp limit i time {}\n"),
        format!("ambiguous option \"\": {MUST}")
    );
    assert_eq!(
        msg("interp create i\ninterp limit i time -\n"),
        format!("ambiguous option \"-\": {MUST}")
    );
    assert_eq!(
        msg("interp create i\ninterp limit i time -zz\n"),
        format!("bad option \"-zz\": {MUST}")
    );
    // Unique prefixes still resolve.
    let (ok, _r, _) = run("interp create i\ninterp limit i time -sec 5\n");
    assert!(ok, "a unique prefix must still resolve");
    let (ok, _r, _) = run("interp create i\ninterp limit i time -com {}\n");
    assert!(ok);
}

/// Issue #1607: `interp debug`'s option word is a `Tcl_GetIndexFromObj` table
/// whose noun is `debug option` (`debugTypes[]`, `tclInterp.c`), so `-f`/`-fr`
/// abbreviate and the one-entry table never says `ambiguous`. The arity check
/// runs first, as it does in C.
///
/// tclsh 8.6.16 / 9.0.4:
///   interp debug i {}      -> bad debug option "": must be -frame
///   interp debug i -x      -> bad debug option "-x": must be -frame
///   interp debug i -f      -> 0
///   interp debug i -fr 1   -> 1
///   i debug -f             -> 1   (after the latch)
///   interp debug i -x 1 2  -> wrong # args: should be "interp debug path ?-frame ?bool??"
///   i debug -x 1 2         -> wrong # args: should be "i debug ?-frame ?bool??"
#[test]
fn interp_debug_option_uses_c_noun_and_abbreviates() {
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    assert_eq!(
        msg("interp create i\ninterp debug i {}\n"),
        "bad debug option \"\": must be -frame"
    );
    assert_eq!(
        msg("interp create i\ninterp debug i -x\n"),
        "bad debug option \"-x\": must be -frame"
    );
    assert_eq!(run("interp create i\ninterp debug i -f\n").1, "0");
    assert_eq!(run("interp create i\ninterp debug i -fr 1\n").1, "1");
    // The child-as-command spelling reaches the same switch.
    assert_eq!(
        run("interp create i\ninterp debug i -fr 1\ni debug -f\n").1,
        "1"
    );
    assert_eq!(
        msg("interp create i\ninterp debug i -x 1 2\n"),
        "wrong # args: should be \"interp debug path ?-frame ?bool??\""
    );
    assert_eq!(
        msg("interp create i\ni debug -x 1 2\n"),
        "wrong # args: should be \"i debug ?-frame ?bool??\""
    );
}

/// Issue #1607: `binary`, `binary encode`/`decode`, `encoding` and `namespace`
/// are `TclMakeEnsemble` commands. `binary encode`/`decode` run with
/// **`-prefixes` off**, so nothing abbreviates there and the miss is worded
/// `unknown subcommand`, never `unknown or ambiguous`.
///
/// tclsh 8.6.16 / 9.0.4:
///   binary e hex a       -> 61     ;  binary en hex a -> 61
///   binary {}            -> unknown or ambiguous subcommand "": must be
///                           decode, encode, format, or scan
///   binary encode h a    -> unknown subcommand "h": must be base64, hex, or uuencode
///   binary encode {} a   -> unknown subcommand "": must be <same>
///   binary decode b YQ== -> unknown subcommand "b": must be <same>
///   encoding s           -> the system encoding
///   encoding c           -> unknown or ambiguous subcommand "c": must be …
///   namespace cu         -> ::
#[test]
fn ensemble_subcommand_words_resolve_like_tclsh() {
    const FORMATS: &str = "must be base64, hex, or uuencode";
    const ENC_MUST: &str = "must be convertfrom, convertto, dirs, names, or system";
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    // `binary` abbreviates; its encode/decode format tables do not.
    assert_eq!(run("binary e hex a").1, "61");
    assert_eq!(run("binary en hex a").1, "61");
    assert_eq!(
        msg("binary {}"),
        "unknown or ambiguous subcommand \"\": must be decode, encode, format, or scan"
    );
    assert_eq!(
        msg("binary encode h a"),
        format!("unknown subcommand \"h\": {FORMATS}")
    );
    assert_eq!(
        msg("binary encode {} a"),
        format!("unknown subcommand \"\": {FORMATS}")
    );
    assert_eq!(
        msg("binary decode b YQ=="),
        format!("unknown subcommand \"b\": {FORMATS}")
    );
    // `encoding`: this engine advertises only what it implements (9.0 also has
    // `profiles` and `user`), but the verdicts are tclsh's.
    assert_eq!(run("encoding s").1, "utf-8");
    assert_eq!(
        msg("encoding c"),
        format!("unknown or ambiguous subcommand \"c\": {ENC_MUST}")
    );
    assert_eq!(
        msg("encoding {}"),
        format!("unknown or ambiguous subcommand \"\": {ENC_MUST}")
    );
    // `namespace`'s miss sentence now comes from the same owner.
    assert_eq!(run("namespace cu").1, "::");
    assert_eq!(
        msg("namespace {}"),
        "unknown or ambiguous subcommand \"\": must be children, code, current, delete, \
         ensemble, eval, exists, export, forget, import, inscope, origin, parent, path, \
         qualifiers, tail, unknown, upvar, or which"
    );
}

/// Issue #1607: `package`'s subcommand word is a `Tcl_GetIndexFromObj(…,
/// "option", 0)` table (`pkgOptions[]`, `tclPkg.c`) — both engines said
/// `unknown or ambiguous subcommand "x"` with no list, which is the *ensemble*
/// wording; `package` is not an ensemble. `package prefer`'s word is a second
/// table with the noun `preference`.
///
/// tclsh 9.0.4 (the VM's default release):
///   package x  -> bad option "x": must be files, forget, ifneeded, names,
///                 prefer, present, provide, require, unknown, vcompare,
///                 versions, or vsatisfies       [TCL LOOKUP INDEX option x]
///   package {} -> ambiguous option "": must be <same>
///   package pr -> ambiguous option "pr": must be <same>   (prefer/present/provide)
///   package v  -> ambiguous option "v": must be <same>    (vcompare/versions/vsatisfies)
///   package n  -> the names list
///   package prefer {} -> ambiguous preference "": must be latest or stable
///   package prefer x  -> bad preference "x": must be latest or stable
///   package prefer s  -> stable
#[test]
fn package_option_words_resolve_like_tcl_get_index_from_obj() {
    const MUST: &str = "must be files, forget, ifneeded, names, prefer, present, provide, \
                        require, unknown, vcompare, versions, or vsatisfies";
    const PREFER_MUST: &str = "must be latest or stable";
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    assert_eq!(msg("package x\n"), format!("bad option \"x\": {MUST}"));
    assert_eq!(
        msg("package {}\n"),
        format!("ambiguous option \"\": {MUST}")
    );
    assert_eq!(
        msg("package pr\n"),
        format!("ambiguous option \"pr\": {MUST}")
    );
    assert_eq!(
        msg("package v\n"),
        format!("ambiguous option \"v\": {MUST}")
    );
    assert_eq!(
        run("package provide foo 1.0\nllength [lsearch -all -exact [package n] foo]\n").1,
        "1"
    );
    // C's lookup error code travels with the message.
    assert_eq!(
        run("catch {package x} e opts\nreturn [dict get $opts -errorcode]\n").1,
        "TCL LOOKUP INDEX option x"
    );
    // `package prefer`'s own table.
    assert_eq!(
        msg("package prefer {}\n"),
        format!("ambiguous preference \"\": {PREFER_MUST}")
    );
    assert_eq!(
        msg("package prefer x\n"),
        format!("bad preference \"x\": {PREFER_MUST}")
    );
    assert_eq!(run("package prefer s\n").1, "stable");
    assert_eq!(run("package prefer l\n").1, "latest");
    // 9.0's `files`: nothing here loads through a package loader, so the
    // answer is the empty list, as it is in tclsh for a script-provided
    // package (`package provide foo 1.0; package files foo` → {}).
    assert_eq!(run("package provide foo 1.0\npackage files foo\n").1, "");
}

/// Issue #1607: the `interp` ensemble and the child-as-command dispatch are
/// `Tcl_GetIndexFromObj(…, "option", 0)` tables (`options[]` in `Tcl_InterpObjCmd`
/// and `NRChildCmd`, `tclInterp.c`), so subcommands abbreviate and the empty
/// word — a prefix of every entry — is `ambiguous option ""`.
///
/// Both lists name only what this engine dispatches (`aliases`, `cancel`, and
/// `target` need infrastructure the VM has none of), so the enumeration is
/// shorter than tclsh's. tclsh 9.0.4, for contrast:
///   interp x  -> bad option "x": must be alias, aliases, bgerror, cancel,
///                children, create, debug, delete, eval, exists, expose, hide,
///                hidden, issafe, invokehidden, limit, marktrusted,
///                recursionlimit, share, target, or transfer
///   i x       -> bad option "x": must be alias, aliases, bgerror, debug, eval,
///                expose, hide, hidden, issafe, invokehidden, limit,
///                marktrusted, or recursionlimit
///
/// The abbreviation verdicts are tclsh's exactly (8.6.16 / 9.0.4 agree):
///   interp cr j        -> j
///   interp c j         -> ambiguous option "c"
///   interp ev {set x 1} -> 1   ;  interp e {set x 1} -> ambiguous option "e"
///   i ev {set x 1}     -> 1    ;  i h / i hi -> ambiguous option "h" / "hi"
#[test]
fn interp_subcommand_words_resolve_like_tcl_get_index_from_obj() {
    const MUST: &str = "must be alias, bgerror, children, create, debug, delete, eval, \
                        exists, expose, hide, hidden, issafe, invokehidden, limit, \
                        marktrusted, recursionlimit, share, or transfer";
    const CHILD_MUST: &str = "must be debug, eval, expose, hide, hidden, issafe, \
                              invokehidden, limit, marktrusted, or recursionlimit";
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    assert_eq!(msg("interp x\n"), format!("bad option \"x\": {MUST}"));
    assert_eq!(msg("interp {}\n"), format!("ambiguous option \"\": {MUST}"));
    assert_eq!(
        msg("interp c j\n"),
        format!("ambiguous option \"c\": {MUST}")
    );
    assert_eq!(run("interp cr j\n").1, "j");
    assert_eq!(
        msg("interp e {set x 1}\n"),
        format!("ambiguous option \"e\": {MUST}")
    );
    assert_eq!(run("interp ev {} {set x 1}\n").1, "1");
    // The 8.x-only `slaves` spelling still resolves, and still dispatches.
    assert_eq!(run("interp create i\nllength [interp sl]\n").1, "1");
    // The child-as-command table.
    assert_eq!(
        msg("interp create i\ni x\n"),
        format!("bad option \"x\": {CHILD_MUST}")
    );
    assert_eq!(
        msg("interp create i\ni {}\n"),
        format!("ambiguous option \"\": {CHILD_MUST}")
    );
    assert_eq!(
        msg("interp create i\ni e {set x 1}\n"),
        format!("ambiguous option \"e\": {CHILD_MUST}")
    );
    assert_eq!(run("interp create i\ni ev {set x 1}\n").1, "1");
    assert_eq!(
        msg("interp create i\ni h\n"),
        format!("ambiguous option \"h\": {CHILD_MUST}")
    );
    assert_eq!(
        msg("interp create i\ni hi\n"),
        format!("ambiguous option \"hi\": {CHILD_MUST}")
    );
}

/// Issue #1607: `interp create`'s and `interp invokehidden`'s leading options
/// are `Tcl_GetIndexFromObj(…, "option", 0)` tables (`createOptions[]` /
/// `hiddenOptions[]`, `tclInterp.c`), so they abbreviate and the lone `-` —
/// a prefix of every entry — is `ambiguous`, not `bad`.
///
/// tclsh 8.6.16 / 9.0.4:
///   interp create -x k          -> bad option "-x": must be -safe or --
///   interp create - k           -> ambiguous option "-": must be -safe or --
///   interp create -s k          -> k        ;  interp create -- k -> k
///   interp invokehidden i -x f  -> bad option "-x": must be -global, -namespace, or --
///   interp invokehidden i - f   -> ambiguous option "-": must be -global, -namespace, or --
///   i invokehidden -x f         -> bad option "-x": must be -global, -namespace, or --
#[test]
fn interp_create_and_invokehidden_options_resolve_like_tcl_get_index_from_obj() {
    const CREATE_MUST: &str = "must be -safe or --";
    const HIDDEN_MUST: &str = "must be -global, -namespace, or --";
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    assert_eq!(
        msg("interp create -x k\n"),
        format!("bad option \"-x\": {CREATE_MUST}")
    );
    assert_eq!(
        msg("interp create - k\n"),
        format!("ambiguous option \"-\": {CREATE_MUST}")
    );
    assert_eq!(run("interp create -s k\n").1, "k");
    assert_eq!(run("interp create -- k\n").1, "k");
    assert_eq!(
        msg("interp create i\ninterp invokehidden i -x foo\n"),
        format!("bad option \"-x\": {HIDDEN_MUST}")
    );
    assert_eq!(
        msg("interp create i\ninterp invokehidden i - foo\n"),
        format!("ambiguous option \"-\": {HIDDEN_MUST}")
    );
    assert_eq!(
        msg("interp create i\ni invokehidden -x foo\n"),
        format!("bad option \"-x\": {HIDDEN_MUST}")
    );
}

/// Issue #1607: `interp limit`'s type word is `Tcl_GetIndexFromObj(…,
/// "limit type", 0)` (`limitTypes[]`, `tclInterp.c`), so `c`/`t` abbreviate
/// and the empty word — a prefix of both entries — is `ambiguous`.
///
/// tclsh 8.6.16 / 9.0.4:
///   interp limit i {}  -> ambiguous limit type "": must be commands or time
///   interp limit i x   -> bad limit type "x": must be commands or time
///   interp limit i c   -> -command {} -granularity 1 -value {}
///   interp limit i t   -> -command {} -granularity 10 -milliseconds {} -seconds {}
///   i limit {}         -> ambiguous limit type "": must be commands or time
#[test]
fn interp_limit_type_word_resolves_like_tcl_get_index_from_obj() {
    const MUST: &str = "must be commands or time";
    let msg = |src: &str| {
        let (ok, result, _) = run(src);
        assert!(!ok, "expected an error for {src}, got ok");
        result
    };
    assert_eq!(
        msg("interp create i\ninterp limit i {}\n"),
        format!("ambiguous limit type \"\": {MUST}")
    );
    assert_eq!(
        msg("interp create i\ninterp limit i x\n"),
        format!("bad limit type \"x\": {MUST}")
    );
    assert_eq!(
        run("interp create i\ninterp limit i c\n").1,
        "-command {} -granularity 1 -value {}"
    );
    assert_eq!(
        run("interp create i\ninterp limit i t\n").1,
        "-command {} -granularity 10 -milliseconds {} -seconds {}"
    );
    // The child-as-command spelling shares the resolver.
    assert_eq!(
        msg("interp create i\ni limit {}\n"),
        format!("ambiguous limit type \"\": {MUST}")
    );
    assert_eq!(
        run("interp create i\ni limit c\n").1,
        "-command {} -granularity 1 -value {}"
    );
}

#[test]
fn regexp_regsub() {
    out_eq("puts [regexp {[0-9]+} abc123]\n", "1\n");
    out_eq("puts [regexp {xyz} abc123]\n", "0\n");
    out_eq(
        "regexp {([a-z]+)([0-9]+)} abc123 m a b\nputs \"$m|$a|$b\"\n",
        "abc123|abc|123\n",
    );
    out_eq("puts [regexp -all {[0-9]} a1b2c3]\n", "3\n");
    out_eq("puts [regexp -inline {[0-9]+} abc123def]\n", "123\n");
    out_eq("puts [regsub {[0-9]+} abc123 X]\n", "abcX\n");
    out_eq("puts [regsub -all {[0-9]} a1b2 _]\n", "a_b_\n");
    out_eq(
        "set n [regsub -all o foo 0 out]\nputs \"$n|$out\"\n",
        "2|f00\n",
    );
}

/// Features the VM gained by sharing `tcl_cmd_core::regex`'s plumbing (it had
/// only `-nocase`/`-all`/`-inline`/`-line` before). Pinned against tclsh 9.0.
#[test]
fn regexp_shared_features() {
    // `-indices` reports char-offset {start end} pairs.
    assert_eq!(run("regexp -indices {bc} abcd m; set m").1, "1 2");
    // `-start` resumes the search at a char offset.
    assert_eq!(run("regexp -start 3 {a} {a a a}").1, "1");
    assert_eq!(run("regsub -start 2 -all {a} aaaa X").1, "aaXX");
    assert_eq!(run("regexp -start 1+1 {a} aaaa").1, "1");
    assert_eq!(run("regsub -start 0x2 {a} aaaa X").1, "aaXa");
    let (ok, msg, _) = run("regexp -start bogus {a} aaaa");
    assert!(!ok);
    assert!(msg.starts_with("bad index \"bogus\""), "got: {msg}");
    let (ok, msg, _) = run("regsub -start {end - 2} {a} aaaa X");
    assert!(!ok);
    assert!(msg.starts_with("bad index \"end - 2\""), "got: {msg}");
    // `-inline -all` with submatches flattens whole+subs per match.
    assert_eq!(
        run("regexp -inline -all {(\\d)(\\d)} 1234").1,
        "12 1 2 34 3 4"
    );
    // A failed match leaves the match variables untouched (was: set to empty).
    assert_eq!(run("set m PRESET; regexp {z} abc m; set m").1, "PRESET");
    // The tclsh compile-error prefix (was: "couldn't compile…").
    let (ok, msg, _) = run("regexp {a(} b");
    assert!(!ok);
    assert!(
        msg.starts_with("cannot compile regular expression pattern"),
        "got: {msg}"
    );
    // The per-command bad-option message.
    let (ok, msg, _) = run("regexp -bogus {a} b");
    assert!(!ok);
    assert!(
        msg.starts_with("bad option \"-bogus\": must be -all, -about"),
        "got: {msg}"
    );
    // Options are exact-only (C's TCL_EXACT): even a unique prefix is a bad
    // option. tclsh: `regexp -no a A x` errors identically; only tclsh's
    // compiled no-match-var fast path (`regexp -no a A`) abbreviates
    // `-nocase`, and the VM implements the runtime semantics everywhere
    // (S4.2, TclCompileRegexpCmd).
    let (ok, msg, _) = run("regexp -no {a} A x");
    assert!(!ok);
    assert_eq!(
        msg,
        "bad option \"-no\": must be -all, -about, -indices, -inline, \
         -expanded, -line, -linestop, -lineanchor, -nocase, -start, or --"
    );
    // regsub is exact-only in every context (tclsh compiles only a literal
    // `-all`); the enumeration is tclsh9.0's table (which has `-command`).
    let (ok, msg, _) = run("regsub -no {a} A b");
    assert!(!ok);
    assert_eq!(
        msg,
        "bad option \"-no\": must be -all, -command, -expanded, -line, \
         -linestop, -lineanchor, -nocase, -start, or --"
    );
}

#[test]
fn file_path_ops() {
    out_eq("puts [file join /a b c]\n", "/a/b/c\n");
    out_eq("puts [file join a /b c]\n", "/b/c\n");
    out_eq("puts [file dirname /a/b/c]\n", "/a/b\n");
    out_eq("puts [file tail /a/b/c.txt]\n", "c.txt\n");
    out_eq("puts [file extension foo.tcl]\n", ".tcl\n");
    out_eq("puts [file rootname foo.tcl]\n", "foo\n");
    out_eq("puts [file exists /nonexistent/xyz]\n", "0\n");
}

#[test]
fn bootstrap_globals_present() {
    out_eq("puts $::tcl_platform(platform)\n", "unix\n");
    out_eq("puts [info exists ::env]\n", "1\n");
    out_eq("puts $::tcl_version\n", "9.0\n");
}

#[test]
fn host_rebind_replaces_bytecode_vm_bootstrap_globals() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(Svc(CommandRegistry::build_default())));
    vm.set_host(Rc::new(SyntheticHost::new(&[
        ("USER", "first-user"),
        ("TCL_LIBRARY", "/first/lib"),
        ("TCL_WASM_SPEC", "first-wasm"),
        ("FIRST_ONLY", "stale"),
    ])));
    let initial = vm
        .eval_source(
            "list $::tcl_platform(user) $::tcl_platform(wasm) \
             $::tcl_library $::env(FIRST_ONLY)",
        )
        .expect("compile initial bootstrap query");
    assert!(initial.code.is_ok());
    assert_eq!(
        initial.result.to_str().as_ref(),
        "first-user first-wasm /first/lib stale"
    );
    let mutated = vm
        .eval_source(
            "set ::env(EMBEDDER_STALE) old; \
             set ::auto_path /old/auto; \
             set ::tclDefaultLibrary /old/default; \
             set ::tcl_pkgPath /old/pkg",
        )
        .expect("compile stale-global setup");
    assert!(mutated.code.is_ok());

    vm.set_host(Rc::new(SyntheticHost::new(&[
        ("USER", "second-user"),
        ("TCL_LIBRARY", "/second/lib"),
        ("TCL_WASM_SPEC", "second-wasm"),
        ("SECOND_ONLY", "fresh"),
    ])));
    let rebound = vm
        .eval_source(
            "list $::tcl_platform(user) $::tcl_platform(wasm) \
             $::tcl_library $::env(SECOND_ONLY) \
             [info exists ::env(FIRST_ONLY)] \
             [info exists ::env(EMBEDDER_STALE)] \
             [info exists ::tclDefaultLibrary] \
             [info exists ::tcl_pkgPath] [llength $::auto_path]",
        )
        .expect("compile rebound bootstrap query");
    assert!(rebound.code.is_ok(), "{}", rebound.result.to_str());
    assert_eq!(
        rebound.result.to_str().as_ref(),
        "second-user second-wasm /second/lib fresh 0 0 0 0 0"
    );
}

#[test]
fn bootstrap_platform_schema_comes_from_the_shared_owner() {
    let mut keys = tcl_platform::bootstrap::entries()
        .iter()
        .map(|entry| entry.name())
        .collect::<Vec<_>>();
    keys.sort_unstable();
    out_eq(
        "puts [lsort [array names ::tcl_platform]]\n",
        &format!("{}\n", keys.join(" ")),
    );
    out_eq(
        "puts [list [info exists ::tcl_platform(machine)] \
         [info exists ::tcl_platform(user)] \
         [expr {$::tcl_platform(osVersion) eq {}}]]\n",
        "1 1 1\n",
    );
}

#[test]
fn cmd_subst_substitutes_proc_param() {
    // A bare `$param` arg to a *generic* command inside a command substitution
    // must load the variable, not push the literal `$param`.
    out_eq(
        "proc f {a b} { return [file join $a $b] }\nputs [f /x y]\n",
        "/x/y\n",
    );
}

#[test]
fn source_command() {
    let mut path = std::env::temp_dir();
    path.push(format!("tclvm_source_{}.tcl", std::process::id()));
    std::fs::write(
        &path,
        "proc greet {n} { return \"hi $n\" }\nset ::loaded 1\n",
    )
    .expect("write temp");
    let src = format!(
        "source {}\nputs [greet bob]\nputs $::loaded\n",
        path.display()
    );
    out_eq(&src, "hi bob\n1\n");
    std::fs::remove_file(&path).ok();
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
    // Op validation + the type error now route through the shared catalogue
    // (the VM previously accepted any op word, and used the wrong type error).
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
    // is *ambiguous* — not bad (probed tclsh 8.6.14; S4.2).
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
/// puts the old one back (`TclPtrSetVarIdx`, `tclVar.c`). Issue #1438; every
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
fn switch_shared_core() {
    // Option parsing + pattern selection route through the shared core. Exact
    // switches are codegen-inlined (the `JUMP_TABLE` path), so the core is
    // exercised via `-glob`/`-regexp` and the error/option cases. Pinned vs tclsh.
    out_eq(
        "puts [switch -glob ab { a {expr 1} ab {expr 2} default {expr 9} }]\n",
        "2\n",
    );
    out_eq("puts [switch -nocase -glob ABC { ab* {expr 7} }]\n", "7\n");
    // `-` fall-through.
    out_eq(
        "puts [switch -glob x { a - b {expr 3} x {expr 4} }]\n",
        "4\n",
    );
    // `-regexp` now matches through the engine (previously fell back to exact).
    out_eq("puts [switch -regexp aXb { {a(.)b} {expr 11} }]\n", "11\n");
    // TIP #75 -matchvar/-indexvar.
    out_eq(
        "switch -regexp -matchvar mv -indexvar iv aXb { {a(.)b} {} }\n\
         puts \"$mv|$iv\"\n",
        "aXb X|{0 2} {1 1}\n",
    );
    // `default` is only a wildcard as the *last* pattern; elsewhere it is a
    // literal pattern (so value "q" matches the `q` arm, returning 6).
    out_eq(
        "puts [switch -glob q { default {expr 5} q {expr 6} }]\n",
        "6\n",
    );
    // No match → empty string.
    out_eq("puts <[switch -glob zz { a {expr 1} }]>\n", "<>\n");
    // (Option-error cases route through the codegen's inline switch, not the
    // runtime `cmd_switch`, so they are pinned via the tree-walking runtime,
    // which always calls the shared core — see `runtime/rust` switch tests.)
}

#[test]
fn info_vars_locals_globals() {
    // Variable listing routes through the shared cores. Pinned against tclsh 9.0
    // (C `InfoVarsCmd`/`InfoLocalsCmd`/`InfoGlobalsCmd`).
    let setup = "namespace eval foo { variable a 1; variable b 2 }\n\
                 namespace eval foo::sub { variable deep 9 }\nset gx 10\n";
    // In a proc: `info locals` is the genuine locals only; `info vars` also lists
    // the linked namespace variable (by its local alias `a`).
    out_eq(
        &format!(
            "{setup}proc p {{}} {{ set loc 5; variable ::foo::a; \
             puts \"[lsort [info locals]]|[lsort [info vars]]\" }}\np\n"
        ),
        "loc|a loc\n",
    );
    // `info globals` lists the global namespace's vars only — not `foo::a`.
    out_eq(
        &format!("{setup}puts [lsearch -exact [info globals] foo::a]\n"),
        "-1\n",
    );
    out_eq(
        &format!("{setup}puts [expr {{[lsearch -exact [info globals] gx] >= 0}}]\n"),
        "1\n",
    );
    // A qualified `info vars` lists that namespace, re-qualified, direct members.
    out_eq(
        &format!("{setup}puts [lsort [info vars ::foo::*]]\n"),
        "::foo::a ::foo::b\n",
    );
    // At namespace scope, `info vars` lists the namespace's own variables.
    out_eq(
        &format!("{setup}namespace eval foo {{ puts [lsort [info vars]] }}\n"),
        "a b\n",
    );
}

#[test]
fn info_commands_procs_namespaced() {
    // `info commands`/`procs` route through the shared namespace-aware core. The
    // expectations are pinned against tclsh 9.0 (see C `InfoCommandsCmd`/
    // `InfoProcsCmd`): a qualified pattern lists that namespace re-qualified
    // absolute; `info commands` merges the global namespace, `info procs` never
    // does; and a global-scope listing excludes namespaced names.
    let setup = "namespace eval foo { proc bar {} {}; proc baz {} {} }\n\
                 namespace eval foo::sub { proc deep {} {} }\nproc gproc {} {}\n";
    // A namespaced proc is not in the global command/proc listing.
    out_eq(
        &format!("{setup}puts [lsearch -exact [info commands] foo::bar]\n"),
        "-1\n",
    );
    // A qualified glob lists that namespace's members, re-qualified absolute,
    // and only direct members (not `foo::sub::deep`).
    out_eq(
        &format!("{setup}puts [lsort [info commands ::foo::*]]\n"),
        "::foo::bar ::foo::baz\n",
    );
    // A *relative* qualifier re-qualifies to absolute too.
    out_eq(
        &format!("{setup}puts [lsort [info commands foo::*]]\n"),
        "::foo::bar ::foo::baz\n",
    );
    out_eq(
        &format!("{setup}puts [lsort [info procs ::foo::*]]\n"),
        "::foo::bar ::foo::baz\n",
    );
    // Inside a namespace: `info procs` lists only that namespace's procs (no
    // global merge), while `info commands` *does* see the global `gproc`.
    out_eq(
        &format!("{setup}namespace eval foo {{ puts [lsort [info procs]] }}\n"),
        "bar baz\n",
    );
    out_eq(
        &format!(
            "{setup}namespace eval foo {{ puts [expr {{[lsearch -exact [info commands] gproc] >= 0}}] }}\n"
        ),
        "1\n",
    );
    // A missing namespace qualifier yields the empty list, not an error.
    out_eq(&format!("{setup}puts [info commands ::nope::*]\n"), "\n");
}

#[test]
fn clock_shared_core() {
    // `clock` is net-new (neither runtime had it); shared over tcl-cmd-core::clock.
    // Pinned vs tclsh 9.0 (the civil math is deterministic; UTC via -gmt).
    out_eq(
        "puts [clock format 1700000000 -gmt 1]\n",
        "Tue Nov 14 22:13:20 GMT 2023\n",
    );
    out_eq(
        "puts [clock format 1700000000 -format {%Y-%m-%d %H:%M:%S} -gmt 1]\n",
        "2023-11-14 22:13:20\n",
    );
    out_eq(
        "puts [clock format 1700000000 -format {%a %A %b %B %p %j %u %z} -gmt 1]\n",
        "Tue Tuesday Nov November PM 318 2 +0000\n",
    );
    // Unknown specifier (%F) passes through verbatim, as in tclsh.
    out_eq(
        "puts [clock format 0 -format {%F %T} -gmt 1]\n",
        "%F 00:00:00\n",
    );
    // `clock add` — fixed + calendar units.
    out_eq(
        "puts [clock format [clock add 1700000000 2 days -gmt 1] -format {%Y-%m-%d} -gmt 1]\n",
        "2023-11-16\n",
    );
    out_eq(
        "puts [clock format [clock add 1700000000 3 months -gmt 1] -format {%Y-%m-%d} -gmt 1]\n",
        "2024-02-14\n",
    );
    // Timing subcommands return sane monotone-ish values.
    out_eq("puts [expr {[clock seconds] > 1700000000}]\n", "1\n");
    out_eq(
        "puts [expr {[clock milliseconds] > 1700000000000}]\n",
        "1\n",
    );
    // `clock scan -format` round-trips `clock format` (the inverse).
    out_eq(
        "puts [clock scan {2023-11-14 22:13:20} -format {%Y-%m-%d %H:%M:%S} -gmt 1]\n",
        "1700000000\n",
    );
    out_eq(
        "puts [clock scan {Nov 14 2023} -format {%b %d %Y} -gmt 1]\n",
        "1699920000\n",
    );
    let (ok, msg, _) = run("clock scan {2023-13-01} -format {%Y-%m-%d} -gmt 1");
    assert!(!ok);
    assert_eq!(msg, "unable to convert input string: invalid month");
    // Free-form scan (no -format) is the remaining piece.
    let (ok, msg, _) = run("clock scan tomorrow");
    assert!(!ok);
    assert_eq!(msg, "free-form clock scan is not yet supported");
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
        "namespace eval foo { variable v 7 }\nputs [namespace inscope ::foo { return $v }]\n",
        "7\n",
    );
    out_eq(
        "namespace eval foo { proc cb {} { return [namespace code {set x 1}] } }\nputs [foo::cb]\n",
        "::namespace inscope ::foo {set x 1}\n",
    );
}

#[test]
fn package_stubs() {
    out_eq("puts [package require Tcl 8.5-]\n", "9.0.4\n");
    out_eq("puts [package vsatisfies 9.0 9.0-]\n", "1\n");
    out_eq("puts [package vsatisfies 8.4 8.5-]\n", "0\n");
    out_eq(
        "package provide Foo 1.2\nputs [package require Foo]\n",
        "1.2\n",
    );
}

/// An already-provided incompatible version is terminal. Discovery must not
/// run a loader or the unknown callback in an attempt to replace it.
#[test]
fn package_require_rejects_a_provided_conflict_before_discovery() {
    out_eq(
        "set events {}\n\
         package provide P 2.0\n\
         package ifneeded P 1.0 {lappend ::events loader; package provide P 1.0}\n\
         proc discover {name args} {lappend ::events unknown}\n\
         package unknown discover\n\
         catch {package require -exact P 1.0} message options\n\
         puts [list $message [dict get $options -errorcode] $events]\n",
        "{version conflict for package \"P\": have 2.0, need exactly 1.0} {TCL PACKAGE VERSIONCONFLICT} {}\n",
    );
}

/// A known satisfying ifneeded entry is the normal fast path. Inverting these
/// two stages makes the deliberately-failing unknown callback win instead.
#[test]
fn package_require_uses_a_registered_loader_before_unknown() {
    out_eq(
        "set events {}\n\
         package ifneeded P 1.0 {lappend ::events loader; package provide P 1.0}\n\
         proc discover {name args} {lappend ::events unknown; error UNKNOWN-RAN}\n\
         package unknown discover\n\
         puts [list [package require P] $events]\n",
        "1.0 loader\n",
    );
}

/// Both package scripts run at level #0 even when the require originates in a
/// namespaced proc. The unqualified callback also resolves in the global
/// namespace, guarding the frame transition as well as variable visibility.
#[test]
fn package_unknown_and_loader_scripts_run_in_global_scope() {
    out_eq(
        "set events {}\n\
         proc discover {name args} {\n\
             lappend ::events unknown-global\n\
             set ::unknownScope [list [namespace current] [info level] [info exists localOnly]]\n\
             package ifneeded $name 1.0 {\n\
                 set ::loaderScope [list [namespace current] [info level] [info exists localOnly]]\n\
                 package provide P 1.0\n\
             }\n\
         }\n\
         namespace eval N {\n\
             proc discover {name args} {lappend ::events unknown-namespaced}\n\
             proc run {} {set localOnly yes; package require P}\n\
         }\n\
         package unknown discover\n\
         puts [list [N::run] $events $unknownScope $loaderScope]\n",
        "1.0 unknown-global {:: 1 0} {:: 0 0}\n",
    );
}

/// Tcl defaults to preferring a stable release over a newer prerelease. The
/// explicit stable request is a no-op and reports the resulting state.
#[test]
fn package_prefer_defaults_to_stable_selection() {
    out_eq(
        "set initial [package prefer]\n\
         set requested [package prefer stable]\n\
         package ifneeded P 1.2 {package provide P 1.2}\n\
         package ifneeded P 1.3b1 {package provide P 1.3b1}\n\
         puts [list $initial $requested [package require P]]\n",
        "stable stable 1.2\n",
    );
}

/// `latest` is a one-way interpreter latch: a later stable request succeeds
/// but does not lower the policy, and the prerelease is then selected.
#[test]
fn package_prefer_latest_is_stateful_and_sticky() {
    out_eq(
        "set raised [package prefer latest]\n\
         set lowered [package prefer stable]\n\
         package ifneeded P 1.2 {package provide P 1.2}\n\
         package ifneeded P 1.3b1 {package provide P 1.3b1}\n\
         puts [list $raised $lowered [package prefer] [package require P]]\n",
        "latest latest latest 1.3b1\n",
    );
}

/// The selected ifneeded version is the contract the loader must fulfil.
/// Numerically-equivalent spellings are accepted, while a genuinely different
/// provide receives Tcl's dedicated WRONGPROVIDE result.
#[test]
fn package_loader_must_provide_its_selected_version() {
    out_eq(
        "package ifneeded Equivalent 1.0 {package provide Equivalent 1.0.0}\n\
         set equivalent [package require Equivalent]\n\
         package ifneeded Wrong 1.0 {package provide Wrong 2.0}\n\
         catch {package require Wrong} message options\n\
         puts [list $equivalent $message [dict get $options -errorcode]]\n",
        "1.0.0 {attempt to provide package Wrong 1.0 failed: package Wrong 2.0 provided instead} {TCL PACKAGE WRONGPROVIDE}\n",
    );
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
fn format_command() {
    out_eq("puts [format %05d 42]\n", "00042\n");
    out_eq("puts [format \"%d-%s\" 5 hi]\n", "5-hi\n");
    out_eq("puts [format %x 255]\n", "ff\n");
    out_eq("puts [format %.2f 3.14159]\n", "3.14\n");
    out_eq("puts [format %-5d| 42]\n", "42   |\n");
    out_eq("set n 3\nputs [format \"n=%d\" $n]\n", "n=3\n");
}

#[test]
fn expr_ternary_string_branches() {
    // `tryCvtToNumeric` must pass non-numeric results through, not error.
    out_eq("puts [expr {1 ? \"a\" : \"b\"}]\n", "a\n");
    out_eq("puts [expr {0 ? \"a\" : \"b\"}]\n", "b\n");
    out_eq(
        "set x 5\nputs [expr {$x > 3 ? \"big\" : \"small\"}]\n",
        "big\n",
    );
    out_eq("puts [expr {1 ? 2 : 3}]\n", "2\n");
}

#[test]
fn expr_in_operator() {
    out_eq("puts [expr {3 in {1 2 3}}]\n", "1\n");
    out_eq("puts [expr {9 in {1 2 3}}]\n", "0\n");
    out_eq("puts [expr {5 ni {1 2 3}}]\n", "1\n");
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
fn switch_glob_and_default() {
    out_eq(
        "switch -glob aa { a* {puts hit} default {puts no} }\n",
        "hit\n",
    );
    out_eq("switch zz { a {puts a} default {puts def} }\n", "def\n");
    out_eq("switch b { a {puts A} b {puts B} }\n", "B\n");
}

#[test]
fn info_default() {
    // A parameter with a default: the var is set to it and the command returns 1.
    out_eq(
        "proc f {a {b 99}} {}\nputs [info default f b d]:$d\n",
        "1:99\n",
    );
    // A parameter with no default: the var is set to the empty string, returns 0.
    out_eq(
        "proc f {a {b 99}} {}\nputs [info default f a d]:<$d>\n",
        "0:<>\n",
    );
    // An unknown parameter is the shared catalogue error (pinned vs tclsh 9.0).
    let (ok, msg, _) = run("proc f {a {b 99}} {}\ninfo default f zz d");
    assert!(!ok);
    assert_eq!(msg, "procedure \"f\" doesn't have an argument \"zz\"");
    // A non-proc target.
    let (ok, msg, _) = run("info default nosuch a d");
    assert!(!ok);
    assert_eq!(msg, "\"nosuch\" isn't a procedure");
}

#[test]
fn proc_body_command_subst() {
    // A proc body is a braced literal: its `[...]` must NOT be substituted at
    // definition time — it runs per-call, against the proc's locals.
    out_eq(
        "proc f {s} { return [string length $s] }\nputs [f hello]\n",
        "5\n",
    );
    out_eq(
        "proc f {l} { return [lindex $l 1] }\nputs [f {a b c}]\n",
        "b\n",
    );
}

#[test]
fn braced_literal_not_substituted() {
    out_eq("set x {a [b] $c}\nputs $x\n", "a [b] $c\n");
}

#[test]
fn catch_body_suppressed_until_eval() {
    out_eq(
        "set rc [catch { error boom } msg]\nputs \"$rc $msg\"\n",
        "1 boom\n",
    );
}

#[test]
fn info_exists_local_scalar() {
    // `info exists` on a proc local compiles to `existScalar`/`existStk`.
    out_eq(
        "proc f {} { set x 1; return [info exists x] }\nputs [f]\n",
        "1\n",
    );
    out_eq("proc f {} { return [info exists nope] }\nputs [f]\n", "0\n");
    out_eq(
        "proc f {} { set x 1; unset x; return [info exists x] }\nputs [f]\n",
        "0\n",
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
fn info_exists_local_array_elem() {
    out_eq(
        "proc f {} { set a(x) 1; return [info exists a(x)] }\nputs [f]\n",
        "1\n",
    );
    out_eq(
        "proc f {} { set a(x) 1; return [info exists a(y)] }\nputs [f]\n",
        "0\n",
    );
}

#[test]
fn scan_command() {
    out_eq("puts [scan 4 %c]\n", "52\n");
    out_eq("puts [scan \"42 7\" \"%d %d\"]\n", "42 7\n");
    out_eq("scan \"0xff\" 0x%x n\nputs $n\n", "255\n");
    out_eq(
        "scan \"hello 99\" {%s %d} word num\nputs \"$word $num\"\n",
        "hello 99\n",
    );
    out_eq("puts [scan abc %d]\n", "{}\n");
}

#[test]
fn string_map_nocase() {
    out_eq(
        "puts [string map -nocase {ok 0 error 1} {OK ERROR ok}]\n",
        "0 1 0\n",
    );
    out_eq("puts [string map {a A b B} abcab]\n", "ABcAB\n");
}

#[test]
fn uplevel_command() {
    out_eq(
        "proc setit {} { uplevel 1 { set x 42 } }\nproc caller {} { setit; return $x }\nputs [caller]\n",
        "42\n",
    );
    out_eq(
        "proc run {script} { uplevel 1 $script }\nset y 0\nrun {set y 9}\nputs $y\n",
        "9\n",
    );
    out_eq(
        "set g 0\nproc p {} { uplevel #0 { set g 7 } }\np\nputs $g\n",
        "7\n",
    );
}

#[test]
fn channel_io() {
    out_eq(
        "set f [open /tmp/zz_tcltest_chan_test.txt w]\nputs $f \"line one\"\nputs $f \"line two\"\nclose $f\n\
         set r [open /tmp/zz_tcltest_chan_test.txt r]\nset a [gets $r]\nset b [gets $r]\nclose $r\n\
         file delete /tmp/zz_tcltest_chan_test.txt\nputs \"$a|$b\"\n",
        "line one|line two\n",
    );
}

/// Issue #1607: `seek`'s origin word is a `Tcl_GetIndexFromObj(…, "origin", 0)`
/// table (`originOptions[]`, `tclIOCmd.c`). This engine silently treated any
/// unknown origin as `start`; C rejects it, abbreviates `s`/`c`/`e`, and words
/// the empty origin — a prefix of all three — `ambiguous`.
///
/// tclsh 8.6.16 / 9.0.4:
///   seek $f 0 x  -> bad origin "x": must be start, current, or end
///   seek $f 0 {} -> ambiguous origin "": must be start, current, or end
///   seek $f 0 s / c / e -> {}
#[test]
fn seek_origin_resolves_like_tcl_get_index_from_obj() {
    const MUST: &str = "must be start, current, or end";
    const SETUP: &str = "set p /tmp/zz_tcltest_seek_test.txt\n\
                         set f [open $p w]\nputs $f abcdef\nclose $f\n\
                         set r [open $p r]\n";
    let msg = |tail: &str| {
        let (ok, result, _) = run(&format!("{SETUP}catch {{{tail}}} e\nclose $r\nset e\n"));
        assert!(ok, "expected the catch to succeed for {tail}");
        result
    };
    assert_eq!(msg("seek $r 0 x"), format!("bad origin \"x\": {MUST}"));
    assert_eq!(
        msg("seek $r 0 {}"),
        format!("ambiguous origin \"\": {MUST}")
    );
    // Abbreviations resolve, and `e` seeks to the end.
    assert_eq!(
        run(&format!(
            "{SETUP}seek $r 0 e\nset n [tell $r]\nclose $r\nfile delete $p\nset n\n"
        ))
        .1,
        "7"
    );
    assert_eq!(
        run(&format!(
            "{SETUP}seek $r 2 s\nset n [tell $r]\nclose $r\nset n\n"
        ))
        .1,
        "2"
    );
    assert_eq!(
        run(&format!(
            "{SETUP}seek $r 2 s\nseek $r 1 c\nset n [tell $r]\nclose $r\nfile delete $p\nset n\n"
        ))
        .1,
        "3"
    );
}

/// `-failindex` must be written. The inline `string is` codegen gates on arity
/// alone, so it used to accept `CLASS -failindex var value`, take the last word
/// as the value, and silently drop the option — the class answer was right and
/// the variable was never assigned. Pinned against tclsh 8.6.16 / 9.0.4, which
/// report index 1 here.
#[test]
fn string_is_failindex_is_written() {
    out_eq(
        "set fi UNSET
set r [string is integer -failindex fi 1.5]
puts \"$r $fi\"\n",
        "0 1\n",
    );
    // Through a variable, and inside a proc body (a different codegen path).
    out_eq(
        "proc p {v} {\n set fi UNSET\n set r [string is integer -failindex fi $v]\n return \"$r $fi\"\n}\nputs [p 12x]\n",
        "0 2\n",
    );
    // A member leaves the variable untouched.
    out_eq(
        "set fi UNSET
set r [string is integer -failindex fi 42]
puts \"$r $fi\"\n",
        "1 UNSET\n",
    );
    // `-strict` still specialises inline and stays correct.
    out_eq("puts [string is alpha -strict \"\"]\n", "0\n");
}

#[test]
fn string_is_classes() {
    out_eq("puts [string is print 4]\n", "1\n");
    out_eq("puts [string is print \"hello world\"]\n", "1\n");
    out_eq("puts [string is graph 4]\n", "1\n");
    out_eq("puts [string is graph \" \"]\n", "0\n"); // space is not graph
    // A real control char (tab/bell) is not printable; build it with `format`.
    out_eq("puts [string is print [format %c 9]]\n", "0\n");
    out_eq("puts [string is control [format %c 7]]\n", "1\n");
    out_eq("puts [string is wordchar foo_1]\n", "1\n");
    out_eq("puts [string is wordchar foo-1]\n", "0\n");
}

#[test]
fn backslash_substituted_command_arguments() {
    // A non-braced literal command argument with backslash escapes must be
    // backslash-substituted at call time, like real Tcl. Previously the raw
    // escapes reached the VM unchanged (e.g. `\{` arrived as the two chars
    // `\{`). Covers pure escapes and an *escaped* `$`/`[` (no real subst).
    out_eq(
        "proc p {a} { puts [string length $a]:$a }\n\
         p \\{\n\
         p a\\{b\n\
         p x\\$y\n\
         p f\\[g\n",
        "1:{\n3:a{b\n3:x$y\n3:f[g\n",
    );
    // A real `$var` substitution still resolves (escaped markers don't disable
    // interpolation of the rest of the word).
    out_eq("set y hi\nproc p {a} { puts $a }\np \\$=$y\n", "$=hi\n");
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

/// An escaped bracket in an **expr** operand: the codegen decodes `"\["` to a
/// bare `[`, so the compiled word reaches the VM with an unclosed bracket and
/// nothing left to close it. That is data, not C's `missing close-bracket` —
/// tclsh 8.6.16 and 9.0.4 both answer the comparison. Treating it as a parse
/// error broke every `if {$ch eq "\["}` in a guest script.
#[test]
fn an_escaped_bracket_in_an_expr_operand_is_data() {
    out_eq("set ch {[}\nputs [expr {$ch eq \"\\[\"}]\n", "1\n");
    out_eq("puts [expr {\"\\[\" eq \"\\[\"}]\n", "1\n");
    out_eq(
        "set ch {[}\nif {$ch eq \"\\[\"} { puts hit } else { puts miss }\n",
        "hit\n",
    );
    // The closing half still behaves, and a real substitution still runs.
    out_eq("set ch {]}\nputs [expr {$ch eq \"\\]\"}]\n", "1\n");
    out_eq("puts [expr {[string length ab] + 1}]\n", "3\n");
}

#[test]
fn escaped_brackets_not_double_substituted() {
    // A word carrying an escaped bracket (`\[` / `\]`) must be backslash-decoded
    // exactly once. Pre-decoding `\[` to a bare `[` made the runtime word-subst
    // re-read it as a command substitution and drop the backslash before the
    // matching `]` (`"x\[y z\\]"` → `x[y z]` instead of `x[y z\]`).
    out_eq("set a \"x\\[y z\\\\]\"\nputs $a\n", "x[y z\\]\n");
    out_eq("set b \"{a\\[} b\\\\]\"\nputs $b\n", "{a[} b\\]\n");
    // An escaped `\${…}` is the literal `${…}`, not a variable substitution.
    out_eq("set x 9\nputs \"\\${x}\"\n", "${x}\n");
    // Same word as a (non-braced) proc argument.
    out_eq(
        "proc plen {s} { return [string length $s]:$s }\nputs [plen a\\[b\\]c]\n",
        "5:a[b]c\n",
    );
    // And surviving a `[list …]` → `uplevel` round-trip: the escaped brackets
    // and trailing braces must reach the evaluated `list` intact (the list.test
    // `invalid command name "}"` abort).
    out_eq(
        "proc run {s} { return [uplevel 1 $s] }\nputs [run {list a\\[ b\\]}]\n",
        "{a[} b\\]\n",
    );
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

#[test]
fn bare_dollar_literal_decodes() {
    // A deferred literal carrying a *bare* `$` (`f\$}` — `$}` is not a variable)
    // must still have its backslash escapes decoded once, not left raw.
    out_eq(
        "set body \"list e\\\\n} f\\\\$} \"\nputs [string length $body]\n",
        "15\n",
    );
    out_eq("set x \"a\\\\nq$\"\nputs $x\n", "a\\nq$\n");
    // Real variables still interpolate.
    out_eq("set z hi\nputs \"a $z b\"\n", "a hi b\n");
}

#[test]
fn brace_wrapped_command_substitution_runs() {
    // A quoted `{…}`-wrapped value with a command substitution runs the command
    // and keeps the literal braces (rather than being stripped as a braced
    // literal). `string is list` / `string is dict` validate list-ness.
    out_eq("set s \"{[list a b]}\"\nputs $s\n", "{a b}\n");
    out_eq("puts [string is list {a b c}]\n", "1\n");
    out_eq("puts [string is list \"a \\{b\"]\n", "0\n");
    out_eq("puts [string is dict {a 1 b 2}]\n", "1\n");
    out_eq("puts [string is dict {a 1 b}]\n", "0\n");
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

#[test]
fn binary_format_rejects_invalid_integer() {
    // A malformed integer is an error, not a silent zero byte.
    let (ok1, _r1, _o1) = run("binary format c bad\n");
    assert!(!ok1, "binary format c bad should error");
    let (ok2, _r2, _o2) = run("binary format c* {1 bad 3}\n");
    assert!(!ok2, "binary format c* with a bad element should error");
    // Valid radix lists still work.
    out_eq(
        "puts [string length [binary format I* {0x50515253 0x52}]]\n",
        "8\n",
    );
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

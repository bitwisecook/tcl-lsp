//! M3 end-to-end: arrays, list/string/dict builtins, and `info`.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

struct Svc(CommandRegistry);
impl CompileService for Svc {
    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let ir = lower_to_ir(src, &self.0);
        let cfg = build_cfg(&ir, false);
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

fn run(src: &str) -> (bool, String, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg(&ir, false);
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
    out_eq("puts [package require Tcl 8.5-]\n", "9.0\n");
    out_eq("puts [package vsatisfies 9.0 9.0-]\n", "1\n");
    out_eq("puts [package vsatisfies 8.4 8.5-]\n", "0\n");
    out_eq(
        "package provide Foo 1.2\nputs [package require Foo]\n",
        "1.2\n",
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
    out_eq(
        "proc f {a {b 99}} {}\ninfo default f b d\nputs $d\n",
        "99\n",
    );
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

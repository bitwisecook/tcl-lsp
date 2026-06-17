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
}

#[test]
fn write_trace_rejects_assignment() {
    // A write-trace error aborts the write (rolling back) and wraps the message.
    out_eq(
        "proc cb {n1 n2 op} { error nope }\nset x 1\ntrace add variable x write cb\ncatch {set x 2} m\nputs \"$m / $x\"\n",
        "can't set \"x\": nope / 1\n",
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
    out_eq(
        "set y hi\nproc p {a} { puts $a }\np \\$=$y\n",
        "$=hi\n",
    );
}

#[test]
fn lappend_no_values_preserves_string_rep() {
    // `lappend var` with no values returns the variable unchanged — it does NOT
    // re-render the list, so a leading `#` element keeps its bare form instead
    // of being requoted to `{#}` (Tcl shimmer-validates but never reformats).
    out_eq("set lst \"# 1 2 3\"\nputs [lappend lst]\n", "# 1 2 3\n");
    out_eq("set z \"  spaced   out  \"\nputs <[lappend z]>\n", "<  spaced   out  >\n");
    // Appending values DOES canonicalise as usual.
    out_eq("set x {1 2 3}\nputs [lappend x 4]\n", "1 2 3 4\n");
    // An unset variable is created as the empty string.
    out_eq("puts <[lappend brandnew]>\nputs [info exists brandnew]\n", "<>\n1\n");
}

#[test]
fn list_element_quoting_balanced_braces() {
    // Balanced braces inside an element stay bare; `]`/`"` escape; `[`/`$` brace.
    out_eq("puts [list a{b}c b{} d]\n", "a{b}c b{} d\n");
    out_eq("set e {a]b}\nputs [list $e]\n", "a\\]b\n");
    out_eq("set f {a[b}\nputs [list $f]\n", "{a[b}\n");
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
    out_eq("puts [string equal [list \"\\x00abc\" xyz] \"\\x00abc xyz\"]\n", "1\n");
    // A single-element fold whose result is brace-quoted must not be mistaken
    // for a braced literal and stripped at runtime.
    out_eq("puts [string length [list \"a b\"]]\n", "5\n");
    out_eq("puts [string length [list \"a\\tb\"]]\n", "5\n");
}

#[test]
fn bare_dollar_literal_decodes() {
    // A deferred literal carrying a *bare* `$` (`f\$}` — `$}` is not a variable)
    // must still have its backslash escapes decoded once, not left raw.
    out_eq("set body \"list e\\\\n} f\\\\$} \"\nputs [string length $body]\n", "15\n");
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

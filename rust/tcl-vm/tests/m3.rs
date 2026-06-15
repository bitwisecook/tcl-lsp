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
fn info_default() {
    out_eq(
        "proc f {a {b 99}} {}\ninfo default f b d\nputs $d\n",
        "99\n",
    );
}

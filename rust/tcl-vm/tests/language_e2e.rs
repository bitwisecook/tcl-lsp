//! End-to-end language semantics: procs, control flow, catch/return/error,
//! switch, and scoping (`global`/`upvar`/local isolation).
//! Compiles real Tcl via `tcl-compiler` (dev-dep) and runs it through `tcl-vm`.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

struct CompilerSvc {
    registry: CommandRegistry,
}

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let ir = lower_to_ir(src, &self.registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
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

/// Compile + run `src`; return `(ok, result, stdout)`.
fn run(src: &str) -> (bool, String, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    }));
    let c = vm.run_module(&asm);
    let out = String::from_utf8(buf.borrow().clone()).expect("utf-8");
    (c.code.is_ok(), c.result.to_str().to_string(), out)
}

#[test]
fn proc_call() {
    let (ok, _r, out) = run("proc add {a b} { expr {$a + $b} }\nputs [add 3 4]\n");
    assert!(ok);
    assert_eq!(out, "7\n");
}

#[test]
fn proc_default_and_args() {
    let (ok, _r, out) = run(concat!(
        "proc greet {name {greeting hi}} { return \"$greeting $name\" }\n",
        "puts [greet bob]\n",
        "puts [greet bob yo]\n",
    ));
    assert!(ok);
    assert_eq!(out, "hi bob\nyo bob\n");
}

#[test]
fn while_loop_output() {
    let (ok, _r, out) = run("set i 0\nwhile {$i < 3} { puts $i\nincr i }\n");
    assert!(ok);
    assert_eq!(out, "0\n1\n2\n");
}

#[test]
fn for_loop_output() {
    let (ok, _r, out) = run("for {set i 0} {$i < 3} {incr i} { puts $i }\n");
    assert!(ok);
    assert_eq!(out, "0\n1\n2\n");
}

/// Regression: a `while` whose condition is a *bare command substitution*
/// (not an inlinable expression) falls back to the runtime `while` builtin.
/// The braced `{cond}` / `{body}` words must be pushed verbatim so the builtin
/// re-evaluates the condition each iteration; previously the condition's
/// command substitution was evaluated *once* at the call site, freezing the
/// loop into an infinite spin.
#[test]
fn while_command_subst_condition_reevaluates() {
    let (ok, _r, out) = run("set x abc\nset n 0\nwhile {[string length $x]} { incr n\nset x \"\" }\nputs $n\n");
    assert!(ok);
    assert_eq!(out, "1\n");
}

#[test]
fn while_expr_subst_condition_counts() {
    let (ok, _r, out) = run("set i 0\nwhile {[expr {$i < 3}]} { incr i }\nputs $i\n");
    assert!(ok);
    assert_eq!(out, "3\n");
}

#[test]
fn for_command_subst_condition_reevaluates() {
    let (ok, _r, out) =
        run("set x abcde\nfor {set i 0} {[string length $x]} {incr i} { set x \"\" }\nputs $i\n");
    assert!(ok);
    assert_eq!(out, "1\n");
}

/// The frozen fallback preserves each word's *source* form: a non-braced
/// `$body` word is still substituted (to the body script), not pushed verbatim.
#[test]
fn while_command_subst_condition_with_substituted_body() {
    let (ok, _r, out) =
        run("set x abc\nset body {set x \"\"}\nwhile {[string length $x]} $body\nputs done\n");
    assert!(ok);
    assert_eq!(out, "done\n");
}

/// Regression: an array element whose index embeds a substitution around
/// literal text (`$a(x$item)`, `$a(-$opt)`, `$a(${item}suf)`) must expand the
/// inner variable. Previously only a bare whole-index `$a($item)` worked; a
/// composite index was pushed as a raw literal so the lookup failed.
#[test]
fn composite_array_index_substitutes() {
    let (ok, _r, out) = run(concat!(
        "array set a {xfoo X -bar Y foosuf Z ab C}\n",
        "set item foo\nset opt bar\nset i a\nset j b\n",
        "puts \"$a(x$item) $a(-$opt) $a(${item}suf) $a($i$j)\"\n",
    ));
    assert!(ok);
    assert_eq!(out, "X Y Z C\n");
}

#[test]
fn composite_array_index_in_proc_and_store() {
    let (ok, _r, out) = run(concat!(
        "proc p {} {\n",
        "  set n 1\n  set arr(k1) V\n  set arr(k$n) W\n  return $arr(k$n)\n",
        "}\nputs [p]\n",
    ));
    assert!(ok);
    assert_eq!(out, "W\n");
}

/// Regression: `dict for`/`map` and a runtime-fallback `foreach` (qualified
/// loop var) run the body via the runtime builtin, so the braced body must be
/// pushed verbatim. Previously its command substitutions were evaluated once at
/// the call site — before the loop variables existed — so the body errored.
#[test]
fn dict_for_body_command_subst_per_iteration() {
    let (ok, _r, out) = run("dict for {k v} {aa 1 bb 2} { puts \"$k=$v len=[string length $k]\" }\n");
    assert!(ok);
    assert_eq!(out, "aa=1 len=2\nbb=2 len=2\n");
}

#[test]
fn qualified_foreach_body_command_subst_per_iteration() {
    let (ok, _r, out) = run("foreach ::x {aa bbb} { puts \"[string length $::x]\" }\n");
    assert!(ok);
    assert_eq!(out, "2\n3\n");
}

#[test]
fn if_else() {
    let (ok, _r, out) = run("set x 5\nif {$x > 0} { puts pos } else { puts nonpos }\n");
    assert!(ok);
    assert_eq!(out, "pos\n");
}

#[test]
fn break_in_loop() {
    let (ok, _r, out) = run("set i 0\nwhile {1} { if {$i >= 2} break\nputs $i\nincr i }\n");
    assert!(ok);
    assert_eq!(out, "0\n1\n");
}

#[test]
fn catch_error() {
    let (ok, result, _out) = run("catch { error boom } msg\n");
    assert!(ok);
    assert_eq!(result, "1"); // catch returns TCL_ERROR == 1
}

#[test]
fn catch_sets_message() {
    let (ok, _r, out) = run("catch { error boom } msg\nputs $msg\n");
    assert!(ok);
    assert_eq!(out, "boom\n");
}

#[test]
fn return_value_from_proc() {
    let (ok, _r, out) = run("proc f {} { return 42 }\nputs [f]\n");
    assert!(ok);
    assert_eq!(out, "42\n");
}

#[test]
fn switch_dispatch() {
    let (ok, _r, out) = run("set x b\nswitch $x { a {puts A} b {puts B} default {puts D} }\n");
    assert!(ok);
    assert_eq!(out, "B\n");
}

#[test]
fn global_scoping() {
    let (ok, _r, out) = run("set g 10\nproc bump {} { global g\nincr g }\nbump\nputs $g\n");
    assert!(ok);
    assert_eq!(out, "11\n");
}

#[test]
fn upvar_scoping() {
    let (ok, _r, out) = run(concat!(
        "proc setit {varname val} { upvar 1 $varname v\nset v $val }\n",
        "set x 0\nsetit x 99\nputs $x\n",
    ));
    assert!(ok);
    assert_eq!(out, "99\n");
}

#[test]
fn error_propagates_to_top() {
    let (ok, result, _out) = run("error kaboom\n");
    assert!(!ok);
    assert_eq!(result, "kaboom");
}

#[test]
fn local_does_not_leak_to_global() {
    // `y` set inside the proc must not be visible at the top level: reading it
    // back at global scope errors, so `catch` reports code 1.
    let (ok, _r, out) = run("proc f {} { set y inside }\nf\nputs [catch { set y }]\n");
    assert!(ok);
    assert_eq!(out, "1\n");
}

#[test]
fn same_named_procs_in_different_namespaces_keep_own_bodies() {
    // Regression: the pre-compiled proc-body cache was keyed by the *global*
    // name, so two procs sharing an unqualified name across namespaces
    // (`::a::p` vs `::b::p`) collided on one body. Keying by the
    // runtime-qualified name keeps them distinct.
    let (ok, _r, out) = run(concat!(
        "namespace eval ::a { proc p {} { return A } }\n",
        "namespace eval ::b { proc p {} { return B } }\n",
        "puts [::a::p][::b::p]\n",
    ));
    assert!(ok);
    assert_eq!(out, "AB\n");
}

#[test]
fn namespaced_proc_resolves_variable_in_its_own_namespace() {
    // A wrapper proc in one namespace calling a proc in another must run the
    // callee in *its* defining namespace, so `variable` links correctly.
    let (ok, _r, out) = run(concat!(
        "namespace eval ::a { variable v 1\n proc get {} { variable v\n return $v } }\n",
        "namespace eval ::b { proc call {} { return [::a::get] } }\n",
        "puts [::b::call]\n",
    ));
    assert!(ok);
    assert_eq!(out, "1\n");
}

#[test]
fn format_with_variable_arg_substitutes_at_runtime() {
    // Regression: `set x [format {…%s…} $var]` constant-folded `$var` to its
    // literal source text. The fold must bail when an argument is a variable.
    let (ok, _r, out) = run("set cmd dict\nset s [format {x %s} $cmd]\nputs $s\n");
    assert!(ok);
    assert_eq!(out, "x dict\n");
}

#[test]
fn dict_set_and_get_in_proc() {
    // The dict bytecode opcodes (DICT_SET / DICT_GET) only emit in a proc body.
    let (ok, _r, out) = run(concat!(
        "proc t {} {\n",
        "  set d [dict create]\n",
        "  dict set d a 1\n",
        "  dict set d nested x 10\n",
        "  dict incr d a 5\n",
        "  dict append d a !\n",
        "  return [dict get $d a]-[dict get $d nested x]-[dict exists $d nested y]\n",
        "}\n",
        "puts [t]\n",
    ));
    assert!(ok);
    assert_eq!(out, "6!-10-0\n");
}

#[test]
fn unknown_handler_dispatches_missing_command() {
    // An unresolved command name is handed to a user-defined `unknown` proc.
    let (ok, _r, out) = run(concat!(
        "proc unknown {cmd args} { return \"u:$cmd:$args\" }\n",
        "puts [nosuchcmd a b]\n",
    ));
    assert!(ok);
    assert_eq!(out, "u:nosuchcmd:a b\n");
}

#[test]
fn clock_ensemble_members_qualified() {
    // `::tcl::clock::seconds` (the ensemble's implementation member) resolves
    // the same as `clock seconds`.
    let (ok, _r, out) = run("puts [expr {[::tcl::clock::seconds] > 0}]\n");
    assert!(ok);
    assert_eq!(out, "1\n");
}

#[test]
fn regexp_word_edge_escapes() {
    // Tcl ARE `\m` (begin-of-word) maps to the crate's `\b{start}`.
    let (ok, _r, out) = run("puts [regexp {\\mcat} \"the cat\"][regexp {\\mcat} \"scatter\"]\n");
    assert!(ok);
    assert_eq!(out, "10\n");
}

#[test]
fn exit_does_not_terminate_the_process() {
    // `exit` must not call `std::process::exit` (that would kill this test
    // binary and any embedding host). It unwinds with a non-OK completion and
    // records the code on the VM instead — reaching this assertion at all
    // proves the process survived.
    let (ok, _r, out) = run("puts before\nexit 3\nputs after\n");
    assert!(!ok, "exit unwinds with a non-OK completion");
    assert_eq!(out, "before\n", "statements after exit do not run");
}

#[test]
fn exit_is_not_catchable() {
    // Like C Tcl's Tcl_Exit, `exit` is not catchable: `catch` re-propagates the
    // unwind rather than swallowing it, so the trailing `puts` never runs.
    let (_ok, _r, out) = run("catch {exit 5}\nputs after\n");
    assert_eq!(out, "", "catch does not swallow exit");
}

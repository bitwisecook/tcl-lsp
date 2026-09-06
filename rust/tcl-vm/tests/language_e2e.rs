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

//! End-to-end language semantics: procs, control flow, catch/return/error,
//! switch, and scoping (`global`/`upvar`/local isolation).
//! Compiles real Tcl via `tcl-compiler` (dev-dep) and runs it through `tcl-vm`.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::Vm;

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
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
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
    let (ok, _r, out) =
        run("set x abc\nset n 0\nwhile {[string length $x]} { incr n\nset x \"\" }\nputs $n\n");
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
    let (ok, _r, out) =
        run("dict for {k v} {aa 1 bb 2} { puts \"$k=$v len=[string length $k]\" }\n");
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

/// Issue #1458 — a **zero-length array name** is a legal array reference.
/// `TclObjLookupVarEx` (`tclVar.c(9.0.4):683-686`) treats a name as an array
/// element whenever it is longer than one byte, ends in `)`, and contains a
/// `(` — the `(` is allowed at offset 0, so `(x)` is element `x` of the array
/// named `""`. The VM's splitter required a non-empty base, so every such
/// reference was silently demoted to an ordinary scalar.
///
/// Every vector below is byte-checked against `tclsh8.6.16` and `tclsh9.0.4`
/// (`array get` output is `lsort`ed because Tcl does not promise hash order).
#[test]
fn zero_length_array_name_is_an_array_element() {
    // Literal element form: the array named "" gains the element.
    let (ok, _r, out) = run(concat!(
        "set (x) 5\n",
        "puts [lsort [array get {}]]\n",
        "puts [info exists (x)]\n",
        "puts [array names {}]\n",
    ));
    assert!(ok);
    assert_eq!(out, "5 x\n1\nx\n");

    // Dynamic-name path: the name only becomes `(q)` at run time, so the
    // compiler cannot pre-split it — this is the elem_ref-routed write.
    let (ok, _r, out) = run(concat!(
        "set nm \"(q)\"\n",
        "set $nm 1\n",
        "puts [lsort [array get {}]]\n",
        "puts [lsearch -exact [info vars] {(q)}]\n",
    ));
    assert!(ok);
    assert_eq!(
        out, "1 q\n-1\n",
        "a dynamic `(q)` is an element, not a scalar"
    );

    // A substituted base that is empty still extends the same array.
    let (ok, _r, out) = run(concat!(
        "set (x) 5\n",
        "set b \"\"\n",
        "set ${b}(z) 7\n",
        "puts [lsort [array get {}]]\n",
    ));
    assert!(ok);
    assert_eq!(out, "5 7 x z\n");

    // `array set` into the empty-named array, then `unset` of an element of it
    // inside a proc (the unset_one path, which re-derived the split inline).
    let (ok, _r, out) = run(concat!(
        "array set {} {k v}\n",
        "puts [lsort [array get {}]]\n",
        "proc p {} { set (w) 3; unset (w); return [info exists (w)] }\n",
        "puts [p]\n",
    ));
    assert!(ok);
    assert_eq!(out, "k v\n0\n");

    // The rule's own boundary: a name that is *only* `)` has no `(` and a name
    // of one byte is never an element, so both stay scalars.
    let (ok, _r, out) = run(concat!(
        "set {)} plain\n",
        "puts [info exists {)}]\n",
        "puts [array exists {}]\n",
    ));
    assert!(ok);
    assert_eq!(out, "1\n0\n");
}

/// Issue #1458's companion guard: the element-reference rule also decides what
/// the **link** commands may name.
///
/// A link (`upvar`, `global`, `variable`) always binds a *scalar* cell, so C
/// refuses a target that looks like an array element — `MakeUpvar` raises
/// `bad variable name "…": can't create a scalar variable that looks like an
/// array element`, and `variable` raises `can't define "…": name refers to an
/// element in an array`. This VM had neither check and silently created a
/// mislinked variable whose name merely *contained* parentheses.
///
/// Both messages, and which spelling each reports for a qualified name, are
/// byte-checked against `tclsh8.6.16` and `tclsh9.0.4` — they agree.
#[test]
fn link_commands_reject_element_looking_names() {
    const SCALAR: &str = "can't create a scalar variable that looks like an array element";

    // `upvar` guards the *local* name; the other-var side may legally be an
    // element.
    let (ok, r, _out) = run("proc p {} { set zz 1; upvar 0 zz (v) }\np\n");
    assert!(!ok);
    assert_eq!(r, format!("bad variable name \"(v)\": {SCALAR}"));
    let (ok, r, _out) = run("proc p {} { set zz 1; upvar 0 zz arr(k) }\np\n");
    assert!(!ok);
    assert_eq!(r, format!("bad variable name \"arr(k)\": {SCALAR}"));
    let (ok, _r, out) = run("proc p {} { upvar 0 arr(k) v; set v 9; puts $v }\np\n");
    assert!(ok, "an element is a legal upvar *target*");
    assert_eq!(out, "9\n");

    // `namespace upvar` has the same scalar-alias guard. Its target namespace
    // is already resolved, but alias installation still belongs to the shared
    // upvar link owner.
    let (ok, r, _out) = run("set zz 1\nnamespace upvar :: zz (v)\n");
    assert!(!ok);
    assert_eq!(r, format!("bad variable name \"(v)\": {SCALAR}"));

    // `global` guards each name, reporting the unqualified tail.
    let (ok, r, _out) = run("proc p {} { global (x) }\np\n");
    assert!(!ok);
    assert_eq!(r, format!("bad variable name \"(x)\": {SCALAR}"));
    let (ok, r, _out) = run("namespace eval x {}\nproc p {} { global ::x::a(b) }\np\n");
    assert!(!ok);
    assert_eq!(
        r,
        format!("bad variable name \"a(b)\": {SCALAR}"),
        "C reports the tail, not the qualified spelling"
    );

    // `variable` has its own wording, and reports the name *as given*.
    let (ok, r, _out) = run("namespace eval q {variable (y) 7}\n");
    assert!(!ok);
    assert_eq!(
        r,
        "can't define \"(y)\": name refers to an element in an array"
    );
    let (ok, r, _out) = run("namespace eval q {variable v(k) 7}\n");
    assert!(!ok);
    assert_eq!(
        r,
        "can't define \"v(k)\": name refers to an element in an array"
    );
    let (ok, r, _out) = run("namespace eval x {}\nnamespace eval q {variable ::x::v(k) 7}\n");
    assert!(!ok);
    assert_eq!(
        r, "can't define \"::x::v(k)\": name refers to an element in an array",
        "C reports the qualified spelling here, unlike `global`"
    );

    // Names that only *look* similar are still fine.
    let (ok, _r, out) = run("proc p {} { global {)} ; set {)} 1; puts ok }\np\n");
    assert!(ok, "a name with no `(` is not an element");
    assert_eq!(out, "ok\n");
}

/// `global`'s guard applies **only inside a procedure**; `variable` and
/// `upvar`'s apply everywhere.
///
/// `Tcl_GlobalObjCmd` returns `TCL_OK` before looking at its arguments when the
/// current frame is already the global frame, so outside a proc `global` is a
/// documented no-op and never reaches the element-name check — `global (x)` at
/// top level, and inside `namespace eval`, are both accepted. `variable` and
/// `upvar` place their checks in the link-creation path itself, which runs at
/// every scope, so those stay unconditional.
///
/// Verified on `tclsh8.6.16` and `tclsh9.0.4`, which agree on all eight rows.
#[test]
fn global_element_guard_applies_only_inside_a_proc() {
    const SCALAR: &str = "can't create a scalar variable that looks like an array element";
    const DEFINE: &str = "name refers to an element in an array";

    // `global` outside a proc: accepted, no-op.
    for script in [
        "global (x)\nputs ok\n",
        "global a(b)\nputs ok\n",
        "namespace eval nsx {global (x)}\nputs ok\n",
        "namespace eval nsx {global a(b)}\nputs ok\n",
    ] {
        let (ok, r, out) = run(script);
        assert!(ok, "{script:?} must be a no-op outside a proc, got {r}");
        assert_eq!(out, "ok\n");
    }
    // Inside a proc the same words are refused.
    let (ok, r, _) = run("proc p {} { global (x) }\np\n");
    assert!(!ok);
    assert_eq!(r, format!("bad variable name \"(x)\": {SCALAR}"));
    let (ok, r, _) = run("proc p {} { global a(b) }\np\n");
    assert!(!ok);
    assert_eq!(r, format!("bad variable name \"a(b)\": {SCALAR}"));

    // `variable` is unconditional — top level, namespace eval, and proc alike.
    for script in [
        "variable (y) 1\n",
        "namespace eval nsx {variable (y) 1}\n",
        "proc p {} { variable (y) }\np\n",
    ] {
        let (ok, r, _) = run(script);
        assert!(!ok, "{script:?} must be refused at every scope");
        assert_eq!(r, format!("can't define \"(y)\": {DEFINE}"));
    }

    // `upvar` is unconditional too.
    let (ok, r, _) = run("set zz 1\nupvar 0 zz (v)\n");
    assert!(!ok, "upvar's guard is not scoped to a proc frame");
    assert_eq!(r, format!("bad variable name \"(v)\": {SCALAR}"));
}

/// A qualified name's **parent namespace is resolved before** the element-name
/// guard, so a missing namespace is reported as such rather than as an element
/// error (closes a slice of #1588; the `upvar` surface of that issue remains).
///
/// C reports the message name **as written**, but `errorCode` is
/// `TCL LOOKUP VARNAME <base>` — array syntax is stripped from that detail.
/// It uses a different verb per command: `define` for `variable`, `access` for
/// `global`. The element split happens first and only when the name *ends* with
/// `)`, which is what keeps `global v(x::y)` (index data, not a path) accepted
/// while `variable v(a)::b` is a namespace error.
///
/// Every row byte-checked against `tclsh8.6.16` and `tclsh9.0.4`, which agree.
#[test]
fn parent_namespace_is_resolved_before_the_element_guard() {
    let err_of = |src: &str| {
        let (ok, r, _) = run(src);
        assert!(!ok, "expected an error for {src}");
        r
    };
    let code_of = |src: &str| {
        let (ok, _r, out) = run(&format!("catch {{{src}}} m\nputs -nonewline $::errorCode"));
        assert!(ok);
        out
    };

    // `variable` — missing namespace wins over the element guard.
    for name in ["::nosuch::v(k)", "::nosuch::v"] {
        let src = format!("variable {name} 1\n");
        assert_eq!(
            err_of(&src),
            format!("can't define \"{name}\": parent namespace doesn't exist")
        );
        assert_eq!(code_of(&src), "TCL LOOKUP VARNAME ::nosuch::v");
    }
    // A *relative* qualified name resolves against the current namespace.
    assert_eq!(
        err_of("namespace eval real {}\nnamespace eval real {variable nope::v(k) 1}\n"),
        "can't define \"nope::v(k)\": parent namespace doesn't exist"
    );
    // When the namespace exists, the element guard is what fires.
    assert_eq!(
        err_of("namespace eval real {}\nvariable ::real::v(k) 1\n"),
        "can't define \"::real::v(k)\": name refers to an element in an array"
    );
    assert_eq!(
        code_of("namespace eval real {}\nvariable ::real::v(k) 1"),
        "TCL UPVAR LOCAL_ELEMENT"
    );

    // `global` — same precedence, but the verb is `access`.
    for name in ["::nosuch::v(k)", "::nosuch::v"] {
        let src = format!("proc p {{}} {{ global {name} }}\np\n");
        assert_eq!(
            err_of(&src),
            format!("can't access \"{name}\": parent namespace doesn't exist")
        );
        assert_eq!(code_of(&src), "TCL LOOKUP VARNAME ::nosuch::v");
    }

    // The element split only applies when the name ENDS with `)`. That is what
    // separates these two: `v(x::y)` is array `v` (accepted), while `v(a)::b`
    // has no element to split off, so `v(a)` is read as a namespace.
    let (ok, _r, out) = run("proc p {} { global v(x::y); puts ok }\np\n");
    assert!(ok, "a `::` inside an index is data, not a namespace path");
    assert_eq!(out, "ok\n");
    assert_eq!(
        err_of("namespace eval real {variable v(a)::b 1}\n"),
        "can't define \"v(a)::b\": parent namespace doesn't exist"
    );
}

/// `variable` and `global` split a qualified name **differently**, and a `::`
/// inside an array index is what separates them.
///
/// C's `variable` stops qualifier scanning at the first `(`, so the whole word
/// stays one element reference: `variable v(x::y)` is refused. `global` splits
/// at the last `::` anywhere, landing on the non-element tail `y)`, so the
/// identical spelling is accepted. Sharing one tail rule between them made all
/// five `::`-in-index spellings silently succeed under `variable`.
///
/// Every row byte-checked against `tclsh8.6.16` and `tclsh9.0.4`, which agree.
#[test]
fn variable_stops_qualifier_scanning_at_the_first_paren() {
    const DEFINE: &str = "name refers to an element in an array";

    // `variable` — a `::` inside the index does not split the name.
    for name in ["v(x::y)", "v(::)", "v(a::b::c)", "(a::b)"] {
        let (ok, r, _) = run(&format!("namespace eval y {{variable {name} 1}}\n"));
        assert!(!ok, "variable {name} must be refused");
        assert_eq!(r, format!("can't define \"{name}\": {DEFINE}"));
    }
    // Qualified, with a `::` in the index too: the scan splits only on the
    // `::` *before* the `(`, leaving the element tail `v(a::b)`.
    let (ok, r, _) = run("namespace eval x {}\nvariable ::x::v(a::b) 1\n");
    assert!(!ok);
    assert_eq!(r, format!("can't define \"::x::v(a::b)\": {DEFINE}"));

    // `global` — the same spellings are ACCEPTED, because its scan splits at
    // the last `::` and the resulting tail is not an element.
    for name in ["v(x::y)", "v(::)", "v(a::b::c)", "(a::b)"] {
        let (ok, _r, out) = run(&format!("proc p {{}} {{ global {name}; puts ok }}\np\n"));
        assert!(ok, "global {name} must still be accepted");
        assert_eq!(out, "ok\n");
    }
    let (ok, _r, out) = run(concat!(
        "namespace eval x {}\n",
        "proc p {} { global ::x::v(a::b); puts ok }\np\n",
    ));
    assert!(ok, "global keeps the last-`::` split even inside an index");
    assert_eq!(out, "ok\n");

    // The plain (no-`::`-in-index) forms stay refused under both commands.
    let (ok, r, _) = run("namespace eval y {variable v(k) 1}\n");
    assert!(!ok);
    assert_eq!(r, format!("can't define \"v(k)\": {DEFINE}"));
    let (ok, r, _) = run("namespace eval x {}\nproc p {} { global ::x::a(b) }\np\n");
    assert!(!ok);
    assert_eq!(
        r,
        "bad variable name \"a(b)\": can't create a scalar variable that looks like an array element"
    );
}

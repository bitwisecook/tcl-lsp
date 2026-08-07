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

//! End-to-end `TclOO` tests for the bytecode VM.
//!
//! Every expectation is the output of the same script under a locally-built
//! **tclsh 9.0.4** (the truth oracle): classes, methods, inheritance (`next`),
//! `self`/`my`, constructors/destructors, `oo::define`/`oo::objdefine`, and the
//! `info object`/`info class` introspection subset.

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
        if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(src) {
            return Err(CompileError(msg));
        }
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

/// Compile and run `src`; return `(ok, result-string, captured-stdout)`.
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
    let completion = vm.run_module(&asm);

    let out = String::from_utf8(buf.borrow().clone()).expect("utf-8 output");
    (
        completion.code.is_ok(),
        completion.result.to_str().to_string(),
        out,
    )
}

/// The trimmed result string of a script expected to succeed.
fn result(src: &str) -> String {
    let (ok, res, _) = run(src);
    assert!(ok, "script errored: {res}");
    res
}

// ===========================================================================
// Classes, methods, construction
// ===========================================================================

#[test]
fn basic_class_and_method() {
    // tclsh 9.0.4: -> generic
    assert_eq!(
        result(
            "oo::class create Animal { method speak {} { return generic } }; [Animal new] speak"
        ),
        "generic"
    );
}

#[test]
fn tcl_oo_package_and_version_vars() {
    // TclOO is compiled in, so — like C Tcl's core — `package require tcl::oo`
    // resolves to the provided patchlevel and the `::oo` version variables are
    // seeded (oo.test's `package require tcl::oo 1.3.1` header + oo-0.9).
    assert_eq!(result("package require tcl::oo"), "1.3.1");
    assert_eq!(result("package require tcl::oo 1.0.3"), "1.3.1");
    assert_eq!(result("package present tcl::oo"), "1.3.1");
    assert_eq!(result("package versions tcl::oo"), "1.3.1");
    assert_eq!(result("set ::oo::version"), "1.3");
    assert_eq!(result("set ::oo::patchlevel"), "1.3.1");
    // The exact oo-0.9 shape: name present, and the patchlevel is a known version.
    assert_eq!(
        result(
            "list [lsearch -nocase -all -inline [package names] tcl::oo] \
             [package present tcl::oo] \
             [expr {$::oo::patchlevel in [package versions tcl::oo]}]"
        ),
        "tcl::oo 1.3.1 1"
    );
}

#[test]
fn tip558_core_property_slots() {
    // tclsh 9.0.4 ooProp-1.1: `-set` replaces; results are sorted + unique.
    assert_eq!(
        result(
            "oo::class create c {}\n\
             set r {}\n\
             lappend r [info class properties c] [info class properties c -writable]\n\
             oo::define c ::oo::configuresupport::readableproperties -set a b c\n\
             lappend r [info class properties c]\n\
             oo::define c ::oo::configuresupport::readableproperties -set f e d\n\
             lappend r [info class properties c]\n\
             oo::define c ::oo::configuresupport::readableproperties -set a a a\n\
             lappend r [info class properties c]\n\
             set r"
        ),
        "{} {} {a b c} {d e f} a"
    );
}

#[test]
fn tip558_property_slot_ops_and_writable() {
    // `-append` / `-remove` / `-clear`, plus the writable slot is independent.
    assert_eq!(
        result(
            "oo::class create c {}\n\
             oo::define c ::oo::configuresupport::readableproperties -set a b\n\
             oo::define c ::oo::configuresupport::readableproperties -append c\n\
             oo::define c ::oo::configuresupport::readableproperties -remove b\n\
             oo::define c ::oo::configuresupport::writableproperties -set w\n\
             list [info class properties c] [info class properties c -writable]"
        ),
        "{a c} w"
    );
}

#[test]
fn tip558_property_all_walks_superclasses() {
    // `-all` unions the readable properties up the class MRO.
    assert_eq!(
        result(
            "oo::class create base {}\n\
             oo::define base ::oo::configuresupport::readableproperties -set p q\n\
             oo::class create derived {superclass base}\n\
             oo::define derived ::oo::configuresupport::readableproperties -set r\n\
             list [info class properties derived] [info class properties derived -all]"
        ),
        "r {p q r}"
    );
}

// ===========================================================================
// TIP 558 configurable layer: oo::configurable, property, configure
// ===========================================================================

#[test]
fn tip558_configurable_configure_get_set_list() {
    // tclsh 9.0.4 ooProp-2.1 shape: configure lists readable props as `-name
    // value` (sorted), gets one, and sets writable ones.
    assert_eq!(
        result(
            "oo::configurable create Point {\n\
                 property x y\n\
                 constructor args { my configure -x 0 -y 0 {*}$args }\n\
             }\n\
             set p [Point new -x 3]\n\
             $p configure -y 4\n\
             list [$p configure -x] [$p configure] [info class properties Point -all]"
        ),
        "3 {-x 3 -y 4} {-x -y}"
    );
}

#[test]
fn tip558_configurable_inheritance_and_next() {
    // ooProp-2.2: a configurable subclass inherits its superclass's properties;
    // `next` chains constructors so all properties initialise.
    assert_eq!(
        result(
            "oo::configurable create Point {\n\
                 property x y\n\
                 constructor args { my configure -x 0 -y 0 {*}$args }\n\
             }\n\
             oo::configurable create 3DPoint {\n\
                 superclass Point\n\
                 property z\n\
                 constructor args { next -z 0 {*}$args }\n\
             }\n\
             set p [3DPoint new -x 3 -y 4 -z 5]\n\
             $p configure"
        ),
        "-x 3 -y 4 -z 5"
    );
}

#[test]
fn tip558_configurable_per_object_property() {
    // ooProp-2.3: `oo::objdefine … property` adds a per-instance property.
    assert_eq!(
        result(
            "oo::configurable create Point {\n\
                 property x y\n\
                 constructor args { my configure -x 0 -y 0 {*}$args }\n\
             }\n\
             set p [Point new -x 3 -y 4]\n\
             oo::objdefine $p property z\n\
             $p configure -z 5\n\
             $p configure"
        ),
        "-x 3 -y 4 -z 5"
    );
}

#[test]
fn tip558_configurable_kind_readable_and_writable() {
    // ooProp-2.7/2.8: `-kind writable` rejects reads; `-kind readable` rejects
    // writes, with C's exact directionality messages.
    let (ok, msg, _) = run(
        "oo::configurable create P { property x -kind writable; constructor {} {my configure -x 0} }\n\
         [P new] configure -x",
    );
    assert!(!ok);
    assert_eq!(msg, "property \"-x\" is write only");
    let (ok, msg, _) = run(
        "oo::configurable create P { property x -kind readable; constructor {} {variable x 1} }\n\
         [P new] configure -x 9",
    );
    assert!(!ok);
    assert_eq!(msg, "property \"-x\" is read only");
}

#[test]
fn tip558_configurable_bad_property() {
    // ooProp-2.4/2.5: an unknown property lists the valid ones (Oxford comma).
    let (ok, msg, _) = run("oo::configurable create Point { property x y }\n\
         [Point new] configure gorp");
    assert!(!ok);
    assert_eq!(msg, "bad property \"gorp\": must be -x or -y");
    let (ok, msg, _) = run("oo::configurable create Point { property x y z }\n\
         [Point new] configure gorp");
    assert!(!ok);
    assert_eq!(msg, "bad property \"gorp\": must be -x, -y, or -z");
}

#[test]
fn tip558_configurable_custom_get_set() {
    // ooProp-3.1: custom `-get`/`-set` bodies run as accessor methods over the
    // class's declared variable.
    assert_eq!(
        result(
            "oo::configurable create Point {\n\
                 variable xyz\n\
                 property x -get { return [lrepeat 3 $xyz] } -set { set xyz [expr {$value * 3}] }\n\
             }\n\
             set pt [Point new]\n\
             $pt configure -x 5\n\
             $pt configure -x"
        ),
        "15 15 15"
    );
}

#[test]
fn tip558_configurable_property_name_validation() {
    // ooProp-3.3..3.7: property names must be simple, dash-free, unqualified.
    for (spec, want) in [
        ("-x", "bad property name \"-x\": must not begin with -"),
        ("{x y}", "bad property name \"x y\": must be a simple word"),
        (
            "::x",
            "bad property name \"::x\": must not contain namespace separators",
        ),
        (
            "x(",
            "bad property name \"x(\": must not contain parentheses",
        ),
    ] {
        let (ok, msg, _) = run(&format!(
            "oo::configurable create P {{ superclass oo::object }}\n\
             oo::define P {{ property {spec} }}"
        ));
        assert!(!ok, "{spec} should error");
        assert_eq!(msg, want, "for property {spec}");
    }
}

#[test]
fn tip558_configurable_option_abbreviation() {
    // ooProp-3.14: `-k reada -g {…}` — option and kind abbreviation both resolve.
    assert_eq!(
        result(
            "oo::configurable create Point {\n\
                 superclass oo::object\n\
                 property x -k reada -g {return ok}\n\
             }\n\
             [Point new] configure -x"
        ),
        "ok"
    );
}

#[test]
fn tip558_configurable_not_on_plain_class() {
    // ooProp T12/T28: `property` is not a command in a non-configurable define
    // body, but a plain subclass's *instances* still inherit `configure`.
    let (ok, msg, _) = run("oo::class create NC {}\noo::define NC { property x }");
    assert!(!ok);
    assert_eq!(msg, "invalid command name \"property\"");
    assert_eq!(
        result(
            "oo::configurable create P { property x; constructor {} {my configure -x 7} }\n\
             oo::class create Q { superclass P }\n\
             [Q new] configure -x"
        ),
        "7"
    );
}

#[test]
fn tip558_configurable_define_errorinfo_frame() {
    // ooProp-4.1: an error in a configurable define body carries the
    // definition-script context frame and the `TCL OO PROPERTY_FORMAT` code.
    assert_eq!(
        result(
            "oo::configurable create Point {superclass oo::object}\n\
             list [catch {oo::define Point {property -x}} msg opt] \
                  [dict get $opt -errorinfo] [dict get $opt -errorcode]"
        ),
        "1 {bad property name \"-x\": must not begin with -\n    \
            while executing\n\"property -x\"\n    \
            (in definition script for class \"::Point\" line 1)\n    \
            invoked from within\n\"oo::define Point {property -x}\"} \
         {TCL OO PROPERTY_FORMAT}"
    );
}

#[test]
fn destroying_a_class_cascades_to_instances_and_subclasses() {
    // tclsh 9.0.4: destroying a class destroys its instances and subclasses too,
    // so the names are free to reuse (the pattern every oo* -cleanup relies on).
    assert_eq!(
        result(
            "oo::class create parent {}\n\
             oo::class create child {superclass parent}\n\
             set inst [parent new]\n\
             parent destroy\n\
             list [info commands ::child] [info commands $inst] [info commands ::parent]"
        ),
        "{} {} {}"
    );
    // And the freed names can be recreated (the cross-test cleanup case every
    // oo* test file's -setup/-cleanup relies on).
    assert_eq!(
        result(
            "oo::class create parent {}\n\
             oo::class create child {superclass parent}\n\
             parent destroy\n\
             oo::class create child {superclass oo::object}\n\
             info commands ::child"
        ),
        "::child"
    );
}

#[test]
fn constructor_and_instance_variables() {
    // tclsh 9.0.4: -> 3 4
    assert_eq!(
        result(
            "oo::class create Point { variable x y; constructor {ax ay} { set x $ax; set y $ay }; \
             method get {} { list $x $y } }; [Point new 3 4] get"
        ),
        "3 4"
    );
}

#[test]
fn create_with_explicit_name() {
    // tclsh 9.0.4: -> hi
    assert_eq!(
        result("oo::class create C { method m {} {return hi} }; C create obj; obj m"),
        "hi"
    );
}

#[test]
fn instances_have_independent_state() {
    // tclsh 9.0.4: -> 10 20
    assert_eq!(
        result(
            "oo::class create P { variable v; constructor {x} {set v $x}; method g {} {return $v} }; \
             P create a 10; P create b 20; list [a g] [b g]"
        ),
        "10 20"
    );
}

#[test]
fn empty_constructor_body_ignores_extra_args() {
    // tclsh 9.0.4: an empty constructor body installs no constructor, so extra
    // construction arguments are silently accepted.
    assert_eq!(
        result(
            "oo::class create C { constructor {a b} {} }; C create foo 1 2 3; info object isa object foo"
        ),
        "1"
    );
}

// ===========================================================================
// Inheritance, next, nextto
// ===========================================================================

#[test]
fn single_inheritance_next() {
    // tclsh 9.0.4: -> derived->base
    assert_eq!(
        result(
            "oo::class create Base { method greet {} { return base } }; \
             oo::class create Derived { superclass Base; method greet {} { return derived->[next] } }; \
             [Derived new] greet"
        ),
        "derived->base"
    );
}

#[test]
fn multilevel_next_chain() {
    // tclsh 9.0.4: -> d-m-base
    assert_eq!(
        result(
            "oo::class create B { method g {} {return base} }; \
             oo::class create M { superclass B; method g {} {return m-[next]} }; \
             oo::class create D { superclass M; method g {} {return d-[next]} }; \
             [D new] g"
        ),
        "d-m-base"
    );
}

#[test]
fn diamond_inheritance_linearisation() {
    // tclsh 9.0.4: keep-last linearisation defers the shared base A -> DBCA.
    assert_eq!(
        result(
            "oo::class create A { method m {} {return A} }; \
             oo::class create B { superclass A; method m {} {return B[next]} }; \
             oo::class create C { superclass A; method m {} {return C[next]} }; \
             oo::class create D { superclass B C; method m {} {return D[next]} }; \
             [D new] m"
        ),
        "DBCA"
    );
}

#[test]
fn nextto_resumes_at_named_class() {
    // tclsh 9.0.4: -> C-A
    assert_eq!(
        result(
            "oo::class create A { method m {} {return A} }; \
             oo::class create B { superclass A; method m {} {return B} }; \
             oo::class create C { superclass A B; method m {} {return C-[nextto A]} }; \
             [C new] m"
        ),
        "C-A"
    );
}

#[test]
fn next_past_end_errors() {
    // tclsh 9.0.4: -> no next method implementation
    let (ok, msg, _) = run("oo::class create C { method m {} { next } }; [C new] m");
    assert!(!ok);
    assert_eq!(msg, "no next method implementation");
}

#[test]
fn constructor_next_chains_to_superclass() {
    // tclsh 9.0.4: prints "B\nS", derived constructor runs after next.
    let (ok, _, out) = run("oo::class create Base { constructor {} {puts B} }; \
         oo::class create Sub { superclass Base; constructor {} {next; puts S} }; \
         Sub new");
    assert!(ok);
    assert_eq!(out, "B\nS\n");
}

// ===========================================================================
// self / my
// ===========================================================================

#[test]
fn self_returns_object_name() {
    // tclsh 9.0.4: -> ::obj
    assert_eq!(
        result("oo::class create C { method who {} { return [self] } }; C create obj; obj who"),
        "::obj"
    );
}

#[test]
fn self_class_and_method() {
    // tclsh 9.0.4: -> ::C minfo
    assert_eq!(
        result(
            "oo::class create C { method minfo {} { list [self class] [self method] } }; \
             C create obj; obj minfo"
        ),
        "::C minfo"
    );
}

#[test]
fn self_class_reports_declaring_class_via_inheritance() {
    // tclsh 9.0.4: the declaring class of an inherited method -> ::Base
    assert_eq!(
        result(
            "oo::class create Base { method whoami {} {return [self class]} }; \
             oo::class create Derived { superclass Base }; [Derived new] whoami"
        ),
        "::Base"
    );
}

#[test]
fn my_calls_unexported_method() {
    // tclsh 9.0.4: an uppercase-initial method is unexported; `my` reaches it.
    assert_eq!(
        result(
            "oo::class create C { method Helper {} {return secret}; method use {} {return [my Helper]} }; \
             [C new] use"
        ),
        "secret"
    );
    // But it is not publicly callable.
    let (ok, msg, _) = run("oo::class create C { method Helper {} {return h} }; [C new] Helper");
    assert!(!ok);
    assert_eq!(msg, "unknown method \"Helper\": must be destroy");
}

#[test]
fn my_variable_links_instance_state() {
    // tclsh 9.0.4: `my variable` links an instance variable into the method.
    assert_eq!(
        result(
            "oo::class create C { constructor {} {my variable v; set v 42}; \
             method g {} {my variable v; return $v} }; [C new] g"
        ),
        "42"
    );
}

// ===========================================================================
// Export / unexport, unknown-method errors
// ===========================================================================

#[test]
fn export_default_is_lowercase_initial() {
    // tclsh 9.0.4: alpha (public) listed, _priv / Cap not -> lists exported set.
    let (ok, msg, _) = run(
        "oo::class create C { method beta {} {}; method alpha {} {}; method _priv {} {} }; \
         [C new] zzz",
    );
    assert!(!ok);
    assert_eq!(
        msg,
        "unknown method \"zzz\": must be alpha, beta or destroy"
    );
}

#[test]
fn unexport_hides_from_public_dispatch() {
    // tclsh 9.0.4: unexport removes public reachability -> catch returns 1.
    assert_eq!(
        result("oo::class create C { method m {} {return e}; unexport m }; catch {[C new] m}"),
        "1"
    );
}

#[test]
fn class_factory_names_in_unknown_error() {
    // tclsh 9.0.4: a class exposes create/new/destroy; oo::class hides new.
    let (_, msg, _) = run("oo::class new");
    assert_eq!(msg, "unknown method \"new\": must be create or destroy");
}

// ===========================================================================
// Destruction
// ===========================================================================

#[test]
fn destroy_runs_destructor_and_removes_command() {
    // tclsh 9.0.4: destructor prints, then the command is gone.
    let (ok, _, out) = run("oo::class create C { destructor {puts bye} }; C create o; o destroy");
    assert!(ok);
    assert_eq!(out, "bye\n");
}

#[test]
fn destructor_next_chains() {
    // tclsh 9.0.4: -> "B-d\nD-d" wait: derived destructor calls next first.
    let (ok, _, out) = run("oo::class create B { destructor {puts B-d} }; \
         oo::class create D { superclass B; destructor {next; puts D-d} }; \
         D create o; o destroy");
    assert!(ok);
    assert_eq!(out, "B-d\nD-d\n");
}

// ===========================================================================
// oo::define / oo::objdefine
// ===========================================================================

#[test]
fn oo_define_adds_method_later() {
    // tclsh 9.0.4: -> added
    assert_eq!(
        result("oo::class create C {}; oo::define C method m {} { return added }; [C new] m"),
        "added"
    );
}

#[test]
fn oo_objdefine_per_instance_method() {
    // tclsh 9.0.4: a per-object method on one instance only -> only
    assert_eq!(
        result(
            "oo::class create C {}; set o [C new]; oo::objdefine $o method solo {} { return only }; $o solo"
        ),
        "only"
    );
}

#[test]
fn oo_define_change_superclass() {
    // tclsh 9.0.4: -> a
    assert_eq!(
        result(
            "oo::class create A { method m {} {return a} }; oo::class create B {}; oo::define B superclass A; [B new] m"
        ),
        "a"
    );
}

// ===========================================================================
// Method-context commands outside a method
// ===========================================================================

#[test]
fn context_commands_outside_method_are_invalid() {
    // tclsh 9.0.4: self/my/next/nextto are only defined inside a method body.
    for cmd in ["self", "my", "next", "nextto"] {
        let (ok, msg, _) = run(&format!("catch {{{cmd}}} e; set e"));
        assert!(ok);
        assert_eq!(
            msg,
            format!("invalid command name \"{cmd}\""),
            "cmd = {cmd}"
        );
    }
}

// ===========================================================================
// info object / info class
// ===========================================================================

#[test]
fn info_object_class_and_isa() {
    // tclsh 9.0.4.
    assert_eq!(
        result(
            "oo::class create B {}; oo::class create S { superclass B }; S create o; info object class o"
        ),
        "::S"
    );
    assert_eq!(
        result(
            "oo::class create B {}; oo::class create S { superclass B }; S create o; info object isa typeof o B"
        ),
        "1"
    );
    assert_eq!(
        result("oo::class create C {}; info object isa object [C new]"),
        "1"
    );
}

#[test]
fn info_class_introspection() {
    // tclsh 9.0.4.
    assert_eq!(
        result(
            "oo::class create B {}; oo::class create D { superclass B; method x {} {} }; info class superclasses D"
        ),
        "::B"
    );
    assert_eq!(
        result("oo::class create C {}; C create a; C create b; lsort [info class instances C]"),
        "::a ::b"
    );
    assert_eq!(
        result(
            "oo::class create C { variable x y; constructor {a b} {set x $a; set y $b} }; info class constructor C"
        ),
        "{a b} {set x $a; set y $b}"
    );
}

#[test]
fn info_class_methods_lists_only_exported() {
    // tclsh 9.0.4: `info class methods` (no -private) lists exported only.
    assert_eq!(
        result("oo::class create C { method a {} {}; method B {} {} }; info class methods C"),
        "a"
    );
}

// ===========================================================================
// forward, mixin
// ===========================================================================

#[test]
fn forward_delegates_to_prefix() {
    // tclsh 9.0.4: -> 5
    assert_eq!(
        result("oo::class create C { forward len string length }; [C new] len hello"),
        "5"
    );
}

#[test]
fn mixin_adds_methods() {
    // tclsh 9.0.4: -> mixed
    assert_eq!(
        result(
            "oo::class create Mix { method extra {} {return mixed} }; \
             oo::class create Base {}; oo::define Base mixin Mix; [Base new] extra"
        ),
        "mixed"
    );
}

// ===========================================================================
// A realistic object: a stack
// ===========================================================================

#[test]
fn realistic_stack_object() {
    // tclsh 9.0.4: -> 2 b 1
    assert_eq!(
        result(
            "oo::class create Stack { \
               variable items; \
               constructor {} {set items {}}; \
               method push {x} {lappend items $x}; \
               method pop {} {set r [lindex $items end]; set items [lrange $items 0 end-1]; return $r}; \
               method size {} {llength $items} \
             }; \
             set s [Stack new]; $s push a; $s push b; list [$s size] [$s pop] [$s size]"
        ),
        "2 b 1"
    );
}

// ===========================================================================
// Recursive method dispatch — issue #996 (native-stack safety).
// ===========================================================================

/// A recursive method call (or `next`/mixin chain) is a genuine native Rust
/// call per level (`TclOO` method dispatch bypasses the proc trampoline that
/// makes ordinary recursive `proc` calls native-stack-free) — empirically,
/// unguarded recursion overflowed the native stack (SIGABRT) between depth
/// 45 and 48 on a 2 MiB thread (`cargo test`'s per-test default).
/// `OO_DISPATCH_DEPTH_LIMIT` (20) now turns that into a catchable error
/// well before the crash floor.
#[test]
fn deeply_recursive_method_errors_instead_of_crashing() {
    let (ok, res, _) = run("oo::class create C { \
           method go {n} { if {$n <= 0} { return done }; return [my go [expr {$n-1}]] } \
         }; \
         C create obj; obj go 1000");
    assert!(!ok, "expected a catchable error, got {res:?}");
    assert!(
        res.contains("too many nested evaluations"),
        "unexpected error: {res:?}"
    );
}

/// A moderately recursive method (well under `OO_DISPATCH_DEPTH_LIMIT`)
/// still runs normally — the safety net must not fire on realistic
/// recursion depths.
#[test]
fn moderately_recursive_method_still_runs() {
    assert_eq!(
        result(
            "oo::class create C { \
               method go {n} { if {$n <= 0} { return done }; return [my go [expr {$n-1}]] } \
             }; \
             C create obj; obj go 10"
        ),
        "done"
    );
}

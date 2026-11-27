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

//! `TclOO` object and class destruction, destructors, child interpreters and method recompilation, compared with tclsh 9.0.4.

use crate::prelude::*;
use std::cell::Cell;

type Destructor = fn(&mut Vm, &[Value]) -> Completion<Value>;

thread_local! {
    static CALLS: Cell<usize> = const { Cell::new(0) };
}

fn provided() -> Vm {
    let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl9.1"));
    text(
        &mut vm,
        "oo::configurable create C {property a -get {return A}}; C create obj",
    );
    vm
}

fn text(vm: &mut Vm, script: &str) -> String {
    let completion = crate::fixture::eval(vm, script);
    assert_eq!(
        completion.code,
        Code::Ok,
        "{script}: {}",
        completion.result.to_str()
    );
    completion.result.to_str().to_string()
}

fn guest(_: &mut Vm, _: &[Value]) -> Completion<Value> {
    CALLS.with(|calls| calls.set(calls.get() + 1));
    Completion::new(
        Code::Error,
        Value::string("DESTRUCTOR_GUEST"),
        Value::empty(),
    )
}

fn quiet(_: &mut Vm, _: &[Value]) -> Completion<Value> {
    CALLS.with(|calls| calls.set(calls.get() + 1));
    Completion::new(Code::Ok, Value::empty(), Value::empty())
}

fn host(vm: &mut Vm, _: &[Value]) -> Completion<Value> {
    CALLS.with(|calls| calls.set(calls.get() + 1));
    vm.refuse_host_command("FIRST_DESTRUCTOR".into())
}

fn install_destructor(vm: &mut Vm, destructor: Destructor) {
    CALLS.with(|calls| calls.set(0));
    vm.register("lifecycle_destructor", destructor);
    text(vm, "oo::define C {destructor lifecycle_destructor}");
}

fn calls() -> usize {
    CALLS.with(Cell::get)
}

#[test]
fn original_oo_class_cleanup_retires_actual_names_properties_and_whole_records() {
    // tclsh 9.0.4: `OBJECT A`, then `{} {} 0 0`.
    let mut vm = provided();
    assert_eq!(
        text(
            &mut vm,
            "oo::define C {method m {} {return CLASS}}; oo::objdefine obj {method m {} {return OBJECT}}; oo::define C {variable stored_declaration}; set ns [info object namespace obj]; set cns [info object namespace C]; list [obj m] [obj configure -a]",
        ),
        "OBJECT A"
    );
    assert_eq!(
        text(
            &mut vm,
            "C destroy; list [info commands obj] [info commands C] [namespace exists $ns] [namespace exists $cns]",
        ),
        "{} {} 0 0"
    );
}

#[test]
fn original_oo_destructor_guest_host_and_restored_context_preserve_cleanup_and_first_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // tclsh 9.0.4 reports a guest destructor error as `1 DESTRUCTOR_GUEST`
    // and still deletes the object and its namespace.
    for (destructor, expected) in [
        (quiet as Destructor, Some((Code::Ok, ""))),
        (guest, Some((Code::Error, "DESTRUCTOR_GUEST"))),
        (host, None),
    ] {
        let mut vm = provided();
        install_destructor(&mut vm, destructor);
        text(&mut vm, "set ns [info object namespace obj]");
        let reached = vm.try_invoke_command("obj", &[Value::string("destroy")]);
        assert_eq!(calls(), 1);
        match expected {
            Some((code, result)) => {
                let reached = reached.unwrap();
                assert_eq!(reached.code, code);
                assert_eq!(reached.result.to_str().as_ref(), result);
            }
            None => assert!(format!("{:?}", reached.unwrap_err()).contains("FIRST_DESTRUCTOR")),
        }
        assert_eq!(
            text(
                &mut vm,
                "list [info commands obj] [namespace exists $ns] [info object isa class C]"
            ),
            "{} 0 1"
        );
    }
}

#[test]
fn original_oo_cleanup_uses_parked_original_interpreter_without_retiring_child_records() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // tclsh 9.0.4: `{} 1`.
    let mut vm = provided();
    assert_eq!(
        text(
            &mut vm,
            "interp create cleanup_child; cleanup_child eval {oo::class create K; K create k}; obj destroy; list [info commands obj] [cleanup_child eval {info object isa object k}] [info object isa class C]",
        ),
        "{} 1 1"
    );
}

#[test]
fn original_oo_cascade_keeps_original_records_and_stops_later_destructors_after_host_refusal() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = provided();
    text(&mut vm, "C create later");
    install_destructor(&mut vm, host);
    let refusal = vm
        .try_invoke_command("C", &[Value::string("destroy")])
        .expect_err("the first destructor refuses");
    assert!(format!("{refusal:?}").contains("FIRST_DESTRUCTOR"));
    assert_eq!(calls(), 1);
    assert_eq!(
        text(
            &mut vm,
            "list [info commands obj] [info commands later] [info commands C]"
        ),
        "{} {} {}"
    );
}

/// A method body is reused until something C would recompile it for;
/// tclsh 9.0.4 prints `33`, `R`, `3`, `4` for the calls below.
#[test]
fn a_method_body_compiles_once_until_its_definition_or_command_world_changes() {
    let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl9.0"));
    text(
        &mut vm,
        "oo::class create C {method m {} {expr {1+2}}}; C create o",
    );
    assert_eq!(text(&mut vm, "o m"), "3");
    assert_eq!(text(&mut vm, "o m"), "3");
    text(&mut vm, "rename expr _e; proc expr args {return R}");
    assert_eq!(text(&mut vm, "o m"), "R");
    text(&mut vm, "rename expr {}; rename _e expr");
    assert_eq!(text(&mut vm, "o m"), "3");
    assert_eq!(text(&mut vm, "o m"), "3");
    text(&mut vm, "oo::define C method m {} {expr {2+2}}");
    assert_eq!(text(&mut vm, "o m"), "4");
}

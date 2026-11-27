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

//! Host command deletion: delete traces, import retirement and the native retire callback, per release.

use crate::prelude::*;
use std::cell::{Cell, RefCell};
use tcl_vm::NativeCommand;

const RELEASES: [&str; 5] = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"];

fn native(environment: &str) -> Vm {
    crate::fixture::interpreter(tcl_vm::test_support::profile(environment))
}

fn ok(value: Value) -> Completion<Value> {
    Completion::new(Code::Ok, value, Value::empty())
}

fn eval(vm: &mut Vm, script: &str) -> String {
    let completion = crate::fixture::eval(vm, script);
    assert_eq!(
        completion.code,
        Code::Ok,
        "{script}: {:?}",
        completion.result
    );
    completion.result.to_str().to_string()
}

struct Log {
    events: Rc<RefCell<Vec<&'static str>>>,
    tag: &'static str,
}
impl NativeCommand for Log {
    fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.events.borrow_mut().push(self.tag);
        ok(Value::empty())
    }
}

struct Retire {
    events: Rc<RefCell<Vec<&'static str>>>,
    calls: Rc<Cell<usize>>,
}
impl NativeCommand for Retire {
    fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
        ok(Value::int(1))
    }
    fn retire(&self, _: &mut Vm) -> Completion<Value> {
        self.calls.set(self.calls.get() + 1);
        self.events.borrow_mut().push("implementation");
        ok(Value::empty())
    }
}

struct Returns(i64);
impl NativeCommand for Returns {
    fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
        ok(Value::int(self.0))
    }
}

type InstalledCommandCleanup = (Rc<RefCell<Vec<&'static str>>>, Rc<Cell<usize>>);

/// `::src::victim`, a host command with a retire callback, imported into
/// `::dst`, with a delete trace on each name.
fn installed(vm: &mut Vm) -> InstalledCommandCleanup {
    eval(
        vm,
        "namespace eval src {namespace export *}; namespace eval dst {}",
    );
    let events = Rc::new(RefCell::new(Vec::new()));
    let calls = Rc::new(Cell::new(0));
    vm.register_native_command(
        "::src::victim",
        Rc::new(Retire {
            events: Rc::clone(&events),
            calls: Rc::clone(&calls),
        }),
    );
    for (name, tag) in [("source_trace", "trace"), ("import_trace", "imports")] {
        vm.register_native_command(
            name,
            Rc::new(Log {
                events: Rc::clone(&events),
                tag,
            }),
        );
    }
    eval(
        vm,
        "namespace eval dst {namespace import ::src::victim}
        trace add command ::src::victim delete source_trace
        trace add command ::dst::victim delete import_trace",
    );
    (events, calls)
}

#[test]
fn original_true_command_deletion_keeps_release_order_and_visible_source_imports() {
    for environment in RELEASES {
        let mut vm = native(environment);
        let (events, calls) = installed(&mut vm);
        assert!(vm.remove_command("::src::victim"));
        assert_eq!(calls.get(), 1);
        assert_eq!(
            &*events.borrow(),
            if matches!(environment, "tcl8.4" | "tcl8.5") {
                &["trace", "implementation", "imports"]
            } else {
                &["trace", "imports", "implementation"]
            },
            "{environment}"
        );
        assert_eq!(
            eval(
                &mut vm,
                "list [info commands ::src::victim] [info commands ::dst::victim]"
            ),
            "{} {}"
        );
    }
}

struct Nested {
    calls: Rc<Cell<usize>>,
    recreate: bool,
}
impl NativeCommand for Nested {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.calls.set(self.calls.get() + 1);
        if self.recreate {
            vm.register_native_command("::src::victim", Rc::new(Returns(99)));
        } else {
            vm.remove_command("::src::victim");
        }
        ok(Value::empty())
    }
}

#[test]
fn original_recursive_delete_unlinks_once_and_outer_owner_retires_captured_implementation() {
    for environment in RELEASES {
        for recreate in [false, true] {
            let mut vm = native(environment);
            let (events, retired) = installed(&mut vm);
            let calls = Rc::new(Cell::new(0));
            vm.register_native_command(
                "nested_delete",
                Rc::new(Nested {
                    calls: Rc::clone(&calls),
                    recreate,
                }),
            );
            eval(
                &mut vm,
                "trace add command ::src::victim delete nested_delete",
            );
            assert!(vm.remove_command("::src::victim"));
            assert_eq!(calls.get(), 1);
            assert_eq!(retired.get(), 1);
            if recreate {
                // The recreation adopts the dying command's imports.
                assert!(!events.borrow().contains(&"imports"), "{environment}");
                assert_eq!(
                    eval(
                        &mut vm,
                        "list [namespace origin ::dst::victim] [::dst::victim]"
                    ),
                    "::src::victim 99",
                    "{environment}"
                );
                assert!(vm.remove_command("::src::victim"));
                assert_eq!(events.borrow().last(), Some(&"imports"), "{environment}");
                assert_eq!(calls.get(), 1);
            }
            assert_eq!(
                eval(
                    &mut vm,
                    "list [info commands ::src::victim] [info commands ::dst::victim]"
                ),
                "{} {}",
                "{environment}"
            );
        }
    }
}

#[test]
fn delete_trace_recreation_keeps_imports_on_the_new_command() {
    // tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 all print the expected list.
    let script = r"namespace eval src {namespace export *; proc victim {} {return 1}}
        namespace eval dst {namespace import ::src::victim}
        set log {}
        proc recreate args {lappend ::log src; proc ::src::victim {} {return 2}}
        trace add command ::src::victim delete recreate
        proc imported args {lappend ::log imp}
        trace add command ::dst::victim delete imported
        rename ::src::victim {}
        set result [list $log [info commands ::dst::victim] [namespace origin ::dst::victim] [::dst::victim]]
        set log {}
        rename ::src::victim {}
        lappend result $log [info commands ::dst::victim] [info commands ::src::victim]";
    for environment in RELEASES {
        let mut vm = native(environment);
        assert_eq!(
            eval(&mut vm, script),
            "src ::dst::victim ::src::victim 2 imp {} {}",
            "{environment}"
        );
    }
}

struct MoveSource {
    calls: Rc<Cell<usize>>,
}
impl NativeCommand for MoveSource {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.calls.set(self.calls.get() + 1);
        crate::fixture::eval(vm, "rename ::src::victim ::src::moved")
    }
}

#[test]
fn original_command_retirement_follows_actual_rename_without_repeating_delete_trace() {
    for environment in RELEASES {
        let mut vm = native(environment);
        let (events, retired) = installed(&mut vm);
        let calls = Rc::new(Cell::new(0));
        vm.register_native_command(
            "move_source",
            Rc::new(MoveSource {
                calls: Rc::clone(&calls),
            }),
        );
        eval(
            &mut vm,
            "trace add command ::src::victim delete move_source",
        );
        assert!(vm.remove_command("::src::victim"));
        assert_eq!(calls.get(), 1);
        assert_eq!(retired.get(), 1);
        assert_eq!(
            events
                .borrow()
                .iter()
                .filter(|tag| **tag == "trace")
                .count(),
            1
        );
        assert_eq!(
            eval(
                &mut vm,
                "list [info commands ::src::victim] [info commands ::src::moved] [info commands ::dst::victim]"
            ),
            "{} {} {}",
            "{environment}"
        );
    }
}

struct RecreateCoroutine;
impl NativeCommand for RecreateCoroutine {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        crate::fixture::eval(
            vm,
            "coroutine paused apply {{} {yield NEW; return NEW_DONE}}",
        )
    }
}

#[test]
fn original_coroutine_retirement_leaves_a_genuine_same_key_recreated_state() {
    let mut vm = native("tcl9.1");
    assert_eq!(
        eval(
            &mut vm,
            "coroutine paused apply {{} {yield OLD; return OLD_DONE}}"
        ),
        "OLD"
    );
    vm.register_native_command("recreate_coroutine", Rc::new(RecreateCoroutine));
    eval(
        &mut vm,
        "trace add command paused delete recreate_coroutine",
    );
    assert!(vm.remove_command("paused"));
    assert_eq!(eval(&mut vm, "info commands paused"), "paused");
    let resumed = crate::fixture::call(&mut vm, "paused", &[]);
    assert_eq!(resumed.code, Code::Ok);
    assert_eq!(resumed.result.string_bytes().as_ref(), b"NEW_DONE");
}

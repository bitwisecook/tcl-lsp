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

//! The embedder surface (`tcl_vm::embed`), which is issue #1373 closed in the
//! VM: a public call-a-command path, invoke-by-handle with no per-call
//! bytecode clone, an enforced `commands` limit, embedder-registered stateful
//! commands, and a whitelist-reduced command table.
//!
//! These are the primitives the `SpecTcl` hook host is built from, so each test
//! asserts the property the host depends on rather than a Tcl surface
//! behaviour.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_vm::{Code, CompileService, Completion, NativeCommand, Value, Vm};

fn vm() -> Vm {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    vm
}

/// An embedder command carrying its own state: it records every argument list
/// it was called with.
struct Recorder {
    calls: RefCell<Vec<Vec<String>>>,
}

struct RelativeHostProcDefiner;

impl NativeCommand for Recorder {
    fn invoke(&self, _vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        self.calls
            .borrow_mut()
            .push(args.iter().map(|arg| arg.to_str().to_string()).collect());
        Completion::new(Code::Ok, Value::string(""), Value::string(""))
    }
}

impl NativeCommand for RelativeHostProcDefiner {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        match vm.define_procedure("host_relative", &[], "namespace current") {
            Ok(()) => Completion::new(Code::Ok, Value::empty(), Value::empty()),
            Err(error) => {
                Completion::new(Code::Error, Value::string(error.message), Value::empty())
            }
        }
    }
}

#[test]
fn invoke_command_is_callable_without_a_driver_script() {
    let mut vm = vm();
    let result = vm.invoke_command("string", &[Value::string("length"), Value::string("abcde")]);
    assert!(result.code.is_ok());
    assert_eq!(result.result.to_str().to_string(), "5");
}

#[test]
fn invoke_command_catch_applies_a_fatal_tail_after_its_complete_prefix() {
    let mut vm = vm();
    vm.set_var("side", Value::int(0)).expect("side is settable");

    let caught = vm.invoke_command(
        "catch",
        &[
            Value::string("incr side; set x \""),
            Value::string("result"),
        ],
    );
    assert_eq!(caught.code, Code::Ok);
    assert_eq!(caught.result.to_str().as_ref(), "1");
    assert_eq!(
        vm.get_var("side").expect("side is set").to_str().as_ref(),
        "1"
    );
    assert_eq!(
        vm.get_var("result")
            .expect("result is set")
            .to_str()
            .as_ref(),
        "missing \""
    );

    let early = vm.invoke_command(
        "catch",
        &[
            Value::string("return EARLY; set x \""),
            Value::string("result"),
        ],
    );
    assert_eq!(early.code, Code::Ok);
    assert_eq!(early.result.to_str().as_ref(), "2");
    assert_eq!(
        vm.get_var("result")
            .expect("result is reset")
            .to_str()
            .as_ref(),
        "EARLY"
    );
}

#[test]
fn a_compiled_handle_runs_repeatedly_without_recompiling() {
    let mut vm = vm();
    let handle = vm
        .compile_function("set n [expr {$n + 1}]")
        .expect("the body compiles");
    vm.set_var("n", Value::int(0)).expect("n is settable");
    for _ in 0..5 {
        let completion = vm.invoke_function(&handle);
        assert!(completion.code.is_ok());
    }
    assert_eq!(vm.get_var("n").expect("n is set").to_str().to_string(), "5");
}

#[test]
fn host_defined_procedure_compiles_parameters_as_proc_locals() {
    let mut vm = vm();
    vm.define_procedure(
        "join",
        &["value", "suffix"],
        "append value $suffix; return $value",
    )
    .expect("host procedure compiles");
    let completion = vm.invoke_command("join", &[Value::string("A"), Value::string("X")]);
    assert!(completion.code.is_ok(), "{}", completion.result.to_str());
    assert_eq!(completion.result.to_str().as_ref(), "AX");
}

#[test]
fn host_defined_procedure_uses_canonical_command_and_namespace_keys() {
    let mut vm = vm();
    vm.eval_source("namespace eval outer {}; namespace eval : {}")
        .expect("namespace setup compiles");

    vm.define_procedure(":::rooted", &[], "namespace current")
        .expect("odd rooted spelling defines globally");
    vm.define_procedure("outer:::qualified", &[], "namespace current")
        .expect("qualified separator run defines in outer");
    vm.register_native_command("host_define", Rc::new(RelativeHostProcDefiner));
    let completion = vm
        .eval_source("namespace eval outer {namespace eval : {host_define; host_relative}}")
        .expect("host definition in literal-colon namespace compiles");
    assert!(completion.code.is_ok(), "{}", completion.result.to_str());
    assert_eq!(completion.result.to_str().as_ref(), "::outer:::");

    for (name, expected) in [("rooted", "::"), ("outer::qualified", "::outer")] {
        let completion = vm.invoke_command(name, &[]);
        assert!(
            completion.code.is_ok(),
            "{name}: {}",
            completion.result.to_str()
        );
        assert_eq!(completion.result.to_str().as_ref(), expected, "{name}");
    }
}

#[test]
fn raw_function_execution_is_fallback_only_and_cannot_bypass_module_profile() {
    let v85 = tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile();
    let service = BytecodeCompileService::for_profile(v85);
    let named = service
        .compile_for_profile("lassign {a b} x; set x", v85)
        .expect("named-profile module compiles");

    let mut v84_vm = vm();
    v84_vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    );
    let rejected = v84_vm.run_function(&named.top_level);
    assert_eq!(rejected.code, Code::Error);
    assert_eq!(
        rejected.result.to_str().as_ref(),
        "profile-less bytecode cannot run under dialect profile tcl8.4"
    );

    // The low-level opcode/embedder contract remains available deliberately
    // on the permissive fallback VM, where no named-profile claim is made.
    let fallback = service
        .compile("set fallback_ok yes; set fallback_ok")
        .expect("fallback module compiles");
    let mut fallback_vm = vm();
    let completion = fallback_vm.run_function(&fallback.top_level);
    assert!(completion.code.is_ok(), "{}", completion.result.to_str());
    assert_eq!(completion.result.to_str().as_ref(), "yes");

    // The supported named-profile AOT entry remains the profile-bearing
    // module API, including same-profile execution.
    let mut v85_vm = vm();
    v85_vm.set_dialect_profile(v85);
    let completion = v85_vm.run_module(&named);
    assert!(completion.code.is_ok(), "{}", completion.result.to_str());
    assert_eq!(completion.result.to_str().as_ref(), "a");
}

#[test]
fn an_embedder_command_carries_its_own_state() {
    let mut vm = vm();
    let recorder = Rc::new(Recorder {
        calls: RefCell::new(Vec::new()),
    });
    vm.register_native_command("emit", recorder.clone());
    let handle = vm
        .compile_function("emit role 0 varwrite\nemit role 1 body\n")
        .expect("the body compiles");
    let completion = vm.invoke_function(&handle);
    assert!(completion.code.is_ok(), "{}", completion.result.to_str());
    assert_eq!(
        *recorder.calls.borrow(),
        vec![
            vec!["role".to_string(), "0".to_string(), "varwrite".to_string()],
            vec!["role".to_string(), "1".to_string(), "body".to_string()],
        ],
    );
}

#[test]
fn native_mathfunc_identity_survives_rename_and_hide_expose_across_profiles() {
    let mut vm = vm();
    let recorder = Rc::new(Recorder {
        calls: RefCell::new(Vec::new()),
    });
    vm.register_native_command("tcl::mathfunc::isfinite", recorder);
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    assert!(
        vm.eval_source("rename ::tcl::mathfunc::isfinite finite")
            .unwrap()
            .code
            .is_ok()
    );
    assert!(vm.invoke_command("finite", &[]).code.is_ok());
    assert!(
        vm.eval_source("interp hide {} finite held; interp expose {} held finite2")
            .unwrap()
            .code
            .is_ok()
    );
    assert!(vm.invoke_command("finite2", &[]).code.is_ok());
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert!(!vm.invoke_command("finite2", &[]).code.is_ok());
}

#[test]
fn embedder_can_remove_a_builtin_hidden_by_the_current_release() {
    let mut vm = vm();
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    );
    assert!(
        !vm.invoke_command("lassign", &[]).code.is_ok(),
        "lassign is outside Tcl 8.4's public surface"
    );

    assert!(
        vm.remove_command("lassign"),
        "embedder teardown must see the raw registered command"
    );
    assert!(!vm.command_names().iter().any(|name| name == "lassign"));

    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
    );
    assert!(
        !vm.invoke_command("lassign", &[]).code.is_ok(),
        "changing release must not revive a command removed by the embedder"
    );
}

/// A loop whose body really dispatches — `[$cmd length abc]` resolves a
/// computed command name, so each iteration is a command the limit charges.
const DISPATCHING_LOOP: &str =
    "set cmd string\nset i 0\nwhile {$i < 100000} { set x [$cmd length abc]\n incr i }\nset i";

#[test]
fn the_command_limit_is_enforced() {
    let mut vm = vm();
    vm.set_command_limit(Some(50));
    let handle = vm
        .compile_function(DISPATCHING_LOOP)
        .expect("the body compiles");
    let completion = vm.invoke_function(&handle);
    assert!(!completion.code.is_ok(), "the budget must stop the loop");
    assert_eq!(
        completion.result.to_str().to_string(),
        "command count limit exceeded"
    );
    assert!(
        vm.commands_run() >= 50,
        "the counter records the spend: {}",
        vm.commands_run()
    );
}

/// **What the limit does not see.** The counter charges *dispatched* commands
/// — which is what C Tcl's `interp limit commands` charges too. A loop the
/// compiler inlines into bytecode dispatches nothing, so it runs to completion
/// under a budget of one. An embedder that needs containment against *any*
/// runaway pairs this with [`Vm::set_wall_clock_budget`], which the trampoline
/// polls per tick; this test pins the gap so it is a documented property
/// rather than a surprise.
#[test]
fn an_inlined_loop_dispatches_nothing_and_so_is_not_charged() {
    let mut vm = vm();
    vm.set_command_limit(Some(1));
    let handle = vm
        .compile_function("set i 0\nwhile {$i < 500} { incr i }\nset i")
        .expect("the body compiles");
    let completion = vm.invoke_function(&handle);
    assert!(
        completion.code.is_ok(),
        "an inlined loop is not charged: {}",
        completion.result.to_str()
    );
    assert_eq!(completion.result.to_str().to_string(), "500");
}

#[test]
fn the_command_counter_refills_per_invocation() {
    let mut vm = vm();
    // Enough for ten dispatching iterations, nowhere near the hundred thousand
    // the loop would run unbudgeted: without the refill the second invocation
    // would start already spent.
    vm.set_command_limit(Some(200));
    let handle = vm
        .compile_function(
            "set cmd string\nset i 0\nwhile {$i < 10} { set x [$cmd length abc]\n incr i }\nset i",
        )
        .expect("the body compiles");
    for _ in 0..5 {
        vm.reset_command_count();
        let completion = vm.invoke_function(&handle);
        assert!(completion.code.is_ok(), "{}", completion.result.to_str());
        assert_eq!(completion.result.to_str().to_string(), "10");
    }
}

#[test]
fn an_unlimited_vm_still_counts_commands() {
    let mut vm = vm();
    assert_eq!(vm.command_limit(), None);
    let handle = vm
        .compile_function("set cmd string\n$cmd length abc\n$cmd length de\n")
        .expect("compiles");
    vm.reset_command_count();
    let completion = vm.invoke_function(&handle);
    assert!(completion.code.is_ok());
    assert!(
        vm.commands_run() >= 2,
        "the counter runs free even with no limit armed: {}",
        vm.commands_run()
    );
}

#[test]
fn retain_commands_reduces_the_table_to_a_whitelist() {
    let mut vm = vm();
    let before = vm.command_names().len();
    let allowed = ["set", "expr", "if", "string", "list", "lindex", "return"];
    let removed = vm.retain_commands(&|name| allowed.contains(&name));
    assert!(removed > 0 && removed < before);
    assert!(vm.command_names().iter().all(|n| allowed.contains(&&**n)));

    // A whitelisted command still works …
    let ok = vm.invoke_command("string", &[Value::string("length"), Value::string("abc")]);
    assert!(ok.code.is_ok());
    assert_eq!(ok.result.to_str().to_string(), "3");
    // … and a removed one is gone, not merely hidden.
    let denied = vm.invoke_command("open", &[Value::string("/etc/passwd")]);
    assert!(!denied.code.is_ok());
    assert!(
        denied.result.to_str().contains("invalid command name"),
        "{}",
        denied.result.to_str()
    );
}

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

//! Procedure, function-handle and `TclOO` method bodies: compiled once and reused, recompiled against a replaced builtin and again after its restoration, and under step traces.

use super::*;

#[test]
fn generated_procedure_body_keeps_proc_locals_and_entered_append_token() {
    let (mut vm, output) = vm();
    eval_ok(
        &mut vm,
        concat!(
            "proc mutate {} {",
            "rename append saved_append; ",
            "proc append args {return NEW}; ",
            "return X}\n",
            "set generated {",
            "set x A; ",
            "set first [append x [mutate]]; ",
            "list $first $x [append x B]}\n",
            "proc generated_proc {} $generated\n",
            "puts [generated_proc]\n",
        ),
    );
    assert_eq!(
        String::from_utf8(output.0.borrow().clone()).unwrap(),
        "AX AX NEW\n"
    );
}

#[test]
fn apply_and_tcloo_bodies_compile_with_parameter_lvt_context() {
    let (mut vm, output) = vm();
    eval_ok(
        &mut vm,
        concat!(
            "puts [apply {{x suffix} {append x $suffix; return $x}} A X]\n",
            "oo::class create C {method join {x suffix} {append x $suffix; return $x}}\n",
            "puts [[C new] join A X]\n",
        ),
    );
    assert_eq!(
        String::from_utf8(output.0.borrow().clone()).unwrap(),
        "AX\nAX\n"
    );
}

#[test]
fn precompiled_proc_cache_requires_exact_body_and_parameters() {
    let (mut vm, output) = vm();
    eval_ok(
        &mut vm,
        concat!(
            "proc p {x} {return FIRST:$x}\n",
            "rename p old_p\n",
            "proc p {} {return SECOND}\n",
            "puts [list [old_p A] [p]]\n",
        ),
    );
    assert_eq!(
        String::from_utf8(output.0.borrow().clone()).unwrap(),
        "FIRST:A SECOND\n"
    );
}

#[test]
fn a_procedure_body_compiled_at_its_first_call_is_reused() {
    let fast_calls = Rc::new(Cell::new(0));
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(CountingCompilerSvc {
        inner: BytecodeCompileService::default(),
        fast_calls: Rc::clone(&fast_calls),
        plain_calls: Rc::new(Cell::new(0)),
    }));
    eval_ok(&mut vm, "proc p {x} {return $x}");
    assert_eq!(fast_calls.get(), 1, "the outer module compiled once");

    let completion = vm.invoke_command("p", &[Value::string("kept")]);
    assert!(completion.code.is_ok(), "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "kept");
    // Defining `p` moved the command world on, so the first call compiles the
    // body against it, as C compiles a body at its first call; later calls
    // reuse that body.
    let compiled = fast_calls.get();
    let completion = vm.invoke_command("p", &[Value::string("again")]);
    assert_eq!(completion.result.to_str().as_ref(), "again");
    assert_eq!(fast_calls.get(), compiled, "the compiled body is reused");
}

/// What answers for a command that is not the shipped one.
struct HostAnswer;

impl NativeCommand for HostAnswer {
    fn invoke(&self, _vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        Completion::new(Code::Ok, Value::string("HOST"), Value::empty())
    }
}

fn host_builtin(_vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
    Completion::new(Code::Ok, Value::string("HOST"), Value::empty())
}

/// A handler an embedder registers through the guarded-builtin door is the
/// embedder's command, whatever name it is registered at: a unit specialised
/// for the shipped `llength` is refused over it — as it is over a native
/// command or a procedure — and the handler answers. The shipped command is
/// the control, and restoring it admits the unit again.
#[test]
fn a_host_registered_builtin_at_a_registry_name_is_not_the_shipped_command() {
    let module = BytecodeCompileService::default()
        .compile("set l {a b c}\nllength $l")
        .expect("compiles");
    let run = |vm: &mut Vm| {
        let completion = vm.run_module(&module);
        assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
        completion.result.to_str().to_string()
    };

    let (mut shipped, _output) = vm();
    assert_eq!(run(&mut shipped), "3", "the shipped command, as compiled");

    let (mut native, _output) = vm();
    native.register_native_command("llength", Rc::new(HostAnswer));
    assert_eq!(run(&mut native), "HOST", "a native command");

    let (mut procedure, _output) = vm();
    eval_ok(&mut procedure, "proc llength args {return HOST}");
    assert_eq!(run(&mut procedure), "HOST", "a procedure");

    let (mut guarded, _output) = vm();
    guarded.register_guarded_builtin(
        "llength",
        host_builtin,
        tcl_runtime_api::guard::GuardIdentity::new(7, 1),
    );
    assert_eq!(run(&mut guarded), "HOST", "a guarded host builtin");

    // The shipped token, renamed away and back, is the shipped command again.
    let (mut restored, _output) = vm();
    eval_ok(&mut restored, "rename llength held_llength");
    restored.register_guarded_builtin(
        "llength",
        host_builtin,
        tcl_runtime_api::guard::GuardIdentity::new(7, 1),
    );
    assert_eq!(run(&mut restored), "HOST");
    eval_ok(
        &mut restored,
        "rename llength {}; rename held_llength llength",
    );
    assert_eq!(run(&mut restored), "3", "the shipped token is back");
}

/// A procedure whose body compiled `expr` inline is recompiled against the
/// live commands once `expr` is replaced, so the replacement runs (tclsh
/// 9.0.4 and 8.6.18: `3`, then `OVERRIDE`, then `3` again). The recompiled
/// body is cached: a second call compiles nothing.
#[test]
fn proc_recompiles_against_a_replaced_builtin_and_its_restoration() {
    let (mut vm, _output) = vm();
    eval_ok(&mut vm, "proc p {} {expr {1+2}}");
    assert_eq!(eval_ok(&mut vm, "p"), "3");
    eval_ok(
        &mut vm,
        "rename expr held_expr; proc expr args {return OVERRIDE}",
    );

    let fast_calls = Rc::new(Cell::new(0));
    let plain_calls = Rc::new(Cell::new(0));
    vm.set_compiler(Box::new(CountingCompilerSvc {
        inner: BytecodeCompileService::default(),
        fast_calls: Rc::clone(&fast_calls),
        plain_calls: Rc::clone(&plain_calls),
    }));
    assert_eq!(eval_ok(&mut vm, "p"), "OVERRIDE");
    assert!(
        fast_calls.get() + plain_calls.get() > 0,
        "the replacement must recompile the proc"
    );

    eval_ok(&mut vm, "rename expr {}; rename held_expr expr");
    assert_eq!(eval_ok(&mut vm, "p"), "3");
    assert_eq!(plain_calls.get(), 0, "no plain dispatch is needed");
    fast_calls.set(0);
    assert_eq!(vm.dispatch("p", &[]).unwrap().result.to_str().as_ref(), "3");
    assert_eq!(fast_calls.get(), 0, "the recompiled body is cached");
}

/// [`proc_recompiles_against_a_replaced_builtin_and_its_restoration`] for a
/// reusable function handle.
#[test]
fn function_handle_recompiles_against_a_replaced_builtin_and_its_restoration() {
    let (mut vm, _output) = vm();
    let handle = vm
        .compile_function("expr {1+2}")
        .expect("function compiles");
    assert_eq!(vm.invoke_function(&handle).result.to_str().as_ref(), "3");
    eval_ok(
        &mut vm,
        "rename expr held_expr; proc expr args {return OVERRIDE}",
    );

    let fast_calls = Rc::new(Cell::new(0));
    let plain_calls = Rc::new(Cell::new(0));
    vm.set_compiler(Box::new(CountingCompilerSvc {
        inner: BytecodeCompileService::default(),
        fast_calls: Rc::clone(&fast_calls),
        plain_calls: Rc::clone(&plain_calls),
    }));
    assert_eq!(
        vm.invoke_function(&handle).result.to_str().as_ref(),
        "OVERRIDE"
    );
    assert!(
        fast_calls.get() + plain_calls.get() > 0,
        "the replacement must recompile the handle"
    );

    eval_ok(&mut vm, "rename expr {}; rename held_expr expr");
    assert_eq!(vm.invoke_function(&handle).result.to_str().as_ref(), "3");
    assert_eq!(plain_calls.get(), 0, "no plain dispatch is needed");
    fast_calls.set(0);
    assert_eq!(vm.invoke_function(&handle).result.to_str().as_ref(), "3");
    assert_eq!(fast_calls.get(), 0, "the recompiled handle is cached");
}

#[test]
fn proc_reoptimises_after_running_plain_under_a_removed_step_trace() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc p {} {expr {1+2}}; proc step_callback args {}",
    );
    assert_eq!(vm.dispatch("p", &[]).unwrap().result.to_str().as_ref(), "3");
    eval_ok(&mut vm, "trace add execution p enterstep step_callback");

    let fast_calls = Rc::new(Cell::new(0));
    let plain_calls = Rc::new(Cell::new(0));
    vm.set_compiler(Box::new(CountingCompilerSvc {
        inner: BytecodeCompileService::default(),
        fast_calls: Rc::clone(&fast_calls),
        plain_calls: Rc::clone(&plain_calls),
    }));
    assert_eq!(vm.dispatch("p", &[]).unwrap().result.to_str().as_ref(), "3");
    assert!(
        plain_calls.get() > 0,
        "the active step trace must deopt the proc"
    );

    eval_ok(&mut vm, "trace remove execution p enterstep step_callback");
    fast_calls.set(0);
    plain_calls.set(0);
    assert_eq!(vm.dispatch("p", &[]).unwrap().result.to_str().as_ref(), "3");
    assert_eq!(fast_calls.get(), 1, "trace removal must restore fast code");
    assert_eq!(plain_calls.get(), 0);
}

/// [`proc_recompiles_against_a_replaced_builtin_and_its_restoration`] for a
/// cached `TclOO` method body.
#[test]
fn tcloo_method_recompiles_against_a_replaced_builtin_and_its_restoration() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "oo::class create C {method m {} {expr {1+2}}}; C create o",
    );
    let call = |vm: &mut Vm| {
        vm.dispatch("o", &[Value::string("m")])
            .unwrap()
            .result
            .to_str()
            .to_string()
    };
    assert_eq!(call(&mut vm), "3");
    eval_ok(
        &mut vm,
        "rename expr held_expr; proc expr args {return METHOD_OVERRIDE}",
    );

    let fast_calls = Rc::new(Cell::new(0));
    let plain_calls = Rc::new(Cell::new(0));
    vm.set_compiler(Box::new(CountingCompilerSvc {
        inner: BytecodeCompileService::default(),
        fast_calls: Rc::clone(&fast_calls),
        plain_calls: Rc::clone(&plain_calls),
    }));
    assert_eq!(eval_ok(&mut vm, "o m"), "METHOD_OVERRIDE");
    assert!(
        fast_calls.get() + plain_calls.get() > 0,
        "the replacement must recompile the cached method"
    );

    eval_ok(&mut vm, "rename expr {}; rename held_expr expr");
    assert_eq!(call(&mut vm), "3");
    assert_eq!(plain_calls.get(), 0, "no plain dispatch is needed");
    fast_calls.set(0);
    assert_eq!(call(&mut vm), "3");
    assert_eq!(fast_calls.get(), 0, "the recompiled method is cached");
}

/// An `enterstep` trace observes only the commands its traced command runs
/// (C's `TclCheckExecutionTraces`), so one on an unrelated procedure leaves a
/// cached method on its compiled body.
#[test]
fn tcloo_method_stays_compiled_under_an_unrelated_step_trace() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "oo::class create C {method m {} {expr {1+2}}}; \
         C create o; proc traced {} {}; proc step_callback args {}",
    );
    let call = |vm: &mut Vm| {
        vm.dispatch("o", &[Value::string("m")])
            .unwrap()
            .result
            .to_str()
            .to_string()
    };
    assert_eq!(call(&mut vm), "3");
    eval_ok(
        &mut vm,
        "trace add execution traced enterstep step_callback",
    );

    let plain_calls = Rc::new(Cell::new(0));
    vm.set_compiler(Box::new(CountingCompilerSvc {
        inner: BytecodeCompileService::default(),
        fast_calls: Rc::new(Cell::new(0)),
        plain_calls: Rc::clone(&plain_calls),
    }));
    assert_eq!(call(&mut vm), "3");
    assert_eq!(
        plain_calls.get(),
        0,
        "an unrelated step trace does not deopt the method"
    );
}

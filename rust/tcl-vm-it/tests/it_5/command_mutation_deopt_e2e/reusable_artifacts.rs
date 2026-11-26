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

//! Reusable modules and function handles are revalidated after command mutation, on native re-entry, under execution traces and across hide and expose.

use super::*;

struct CompileOrInvokeHandle(RefCell<Option<FunctionHandle>>);

struct RunCompiledModule(Rc<tcl_bytecode::ModuleAsm>);

struct RunCompiledFunction(Rc<tcl_bytecode::FunctionAsm>);

impl NativeCommand for CompileOrInvokeHandle {
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        let operation = args.first().map(|arg| arg.to_str().to_string());
        match operation.as_deref() {
            Some("compile") => match vm.compile_function("expr {1+2}") {
                Ok(handle) => {
                    *self.0.borrow_mut() = Some(handle);
                    Completion::new(Code::Ok, Value::empty(), Value::empty())
                }
                Err(error) => Completion::new(
                    Code::Error,
                    error.into_value().expect("guest fixture compilation error"),
                    Value::empty(),
                ),
            },
            Some("invoke") => {
                let handle = self.0.borrow().clone().expect("handle compiled first");
                vm.invoke_function(&handle)
            }
            _ => Completion::new(
                Code::Error,
                Value::string("expected compile or invoke"),
                Value::empty(),
            ),
        }
    }
}

impl NativeCommand for RunCompiledModule {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        vm.run_module(&self.0)
    }
}

impl NativeCommand for RunCompiledFunction {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        vm.run_function(&self.0)
    }
}

#[test]
fn reusable_module_is_revalidated_after_command_mutation() {
    let (mut vm, _output) = vm();
    let module = BytecodeCompileService::default()
        .compile("expr {1+2}")
        .expect("module compiles");
    assert_eq!(vm.run_module(&module).result.to_str().as_ref(), "3");
    eval_ok(
        &mut vm,
        "rename expr real_expr; proc expr args {return MODULE_OVERRIDE}",
    );

    let completion = vm.run_module(&module);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "MODULE_OVERRIDE");
}

#[test]
fn reusable_function_handle_is_revalidated_after_command_mutation() {
    let (mut vm, _output) = vm();
    let handle = vm
        .compile_function("expr {1+2}")
        .expect("function compiles");
    assert_eq!(vm.invoke_function(&handle).result.to_str().as_ref(), "3");
    eval_ok(
        &mut vm,
        "rename expr real_expr; proc expr args {return HANDLE_OVERRIDE}",
    );

    let completion = vm.invoke_function(&handle);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "HANDLE_OVERRIDE");
}

#[test]
fn native_reentry_refreshes_function_handle_in_the_invoking_namespace() {
    let (mut vm, _output) = vm();
    vm.register_native_command(
        "host_handle",
        Rc::new(CompileOrInvokeHandle(RefCell::new(None))),
    );
    eval_ok(
        &mut vm,
        "namespace eval ::a {}; namespace eval ::b {proc expr args {return B}}",
    );

    // The command epoch is unchanged between these two native re-entries. The
    // reusable artifact must still refresh because its unqualified `expr` was
    // specialised in ::a and is now being invoked under ::b's local shadow.
    assert_eq!(
        eval_ok(
            &mut vm,
            "namespace eval ::a {host_handle compile}; \
             namespace eval ::b {host_handle invoke}",
        ),
        "B",
    );
}

#[test]
fn native_reentry_recompiles_module_in_the_running_namespace() {
    let profile = DialectProfile::plain_tcl();
    let module = BytecodeCompileService::default()
        .compile_script_for_profile(
            ScriptCompileTarget {
                source: "expr {1+2}",
                namespace: "a",
                evaluation: tcl_runtime_api::CommandEvaluation::Compiled,
            },
            profile,
        )
        .expect("namespaced module compiles");
    assert_eq!(
        module.source_namespace,
        tcl_runtime_api::ByteNamespacePath::from_segments(["a"])
    );

    let (mut vm, _output) = vm();
    vm.register_native_command("host_run", Rc::new(RunCompiledModule(Rc::new(module))));
    eval_ok(
        &mut vm,
        "namespace eval ::a {}; namespace eval ::b {proc expr args {return B}}",
    );

    assert_eq!(
        eval_ok(&mut vm, "namespace eval ::b {host_run}"),
        "B",
        "run_module must not execute ::a-specialised bytecode under ::b",
    );
}

#[test]
fn native_reentry_rejects_a_bare_function_from_another_namespace() {
    let profile = DialectProfile::plain_tcl();
    let function = BytecodeCompileService::default()
        .compile_script_for_profile(
            ScriptCompileTarget {
                source: "expr {1+2}",
                namespace: "a",
                evaluation: tcl_runtime_api::CommandEvaluation::Compiled,
            },
            profile,
        )
        .expect("namespaced function compiles")
        .top_level;

    let (mut vm, _output) = vm();
    vm.register_native_command(
        "host_run_function",
        Rc::new(RunCompiledFunction(Rc::new(function))),
    );
    eval_ok(
        &mut vm,
        "namespace eval ::a {}; namespace eval ::b {proc expr args {return B}}",
    );

    assert_eq!(
        eval_ok(
            &mut vm,
            "namespace eval ::b {catch {host_run_function} message; set message}",
        ),
        "stale profile-less bytecode has no source for plain dispatch",
        "source-less FunctionAsm must fail closed outside its recorded resolution namespace",
    );
}

#[test]
fn reusable_consumers_revalidate_command_execution_traces() {
    fn install_trace(vm: &mut Vm) {
        eval_ok(
            vm,
            "set ::traced 0; proc trace_expr {command op} {\
             if {$command eq {expr 1+2}} {incr ::traced}}; \
             trace add execution expr enter trace_expr",
        );
    }

    let module = BytecodeCompileService::default()
        .compile("expr {1+2}")
        .expect("module compiles");
    let (mut module_vm, _output) = vm();
    assert_eq!(module_vm.run_module(&module).result.to_str().as_ref(), "3");
    install_trace(&mut module_vm);
    assert_eq!(module_vm.run_module(&module).result.to_str().as_ref(), "3");
    assert_eq!(eval_ok(&mut module_vm, "set ::traced"), "1");

    let (mut handle_vm, _output) = vm();
    let handle = handle_vm
        .compile_function("expr {1+2}")
        .expect("function compiles");
    assert_eq!(
        handle_vm.invoke_function(&handle).result.to_str().as_ref(),
        "3"
    );
    install_trace(&mut handle_vm);
    assert_eq!(
        handle_vm.invoke_function(&handle).result.to_str().as_ref(),
        "3"
    );
    assert_eq!(eval_ok(&mut handle_vm, "set ::traced"), "1");

    let (mut eval_vm, _output) = vm();
    assert_eq!(eval_ok(&mut eval_vm, "expr {1+2}"), "3");
    install_trace(&mut eval_vm);
    assert_eq!(eval_ok(&mut eval_vm, "expr {1+2}"), "3");
    assert_eq!(eval_ok(&mut eval_vm, "set ::traced"), "1");
}

#[test]
fn hide_and_expose_lifecycle_mutations_revalidate_artifacts() {
    let (mut vm, _output) = vm();
    let handle = vm
        .compile_function("expr {1+2}")
        .expect("function compiles");
    eval_ok(
        &mut vm,
        "interp hide {} expr held; proc expr args {return HIDDEN_OVERRIDE}",
    );
    assert_eq!(
        vm.invoke_function(&handle).result.to_str().as_ref(),
        "HIDDEN_OVERRIDE"
    );

    eval_ok(&mut vm, "rename expr {}; interp expose {} held expr");
    assert_eq!(vm.invoke_function(&handle).result.to_str().as_ref(), "3");
}

#[test]
fn unrelated_mutation_does_not_deoptimise_a_reusable_artifact() {
    let (mut vm, _output) = vm();
    let handle = vm
        .compile_function("expr {1+2}")
        .expect("function compiles");
    eval_ok(&mut vm, "proc unrelated {} {return X}");
    vm.set_compiler(Box::new(OptimisedOnlyCompilerSvc(
        BytecodeCompileService::default(),
    )));

    let completion = vm.invoke_function(&handle);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "3");
}

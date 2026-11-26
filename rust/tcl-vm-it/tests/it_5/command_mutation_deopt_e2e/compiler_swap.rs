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

//! Replacing a builtin, the compile service or the profile while code is cached or running: cached evals, foreign modules, suspended coroutines, tailcalls and runtime loops fail closed or recompile.

use super::*;

struct CountCalls(Rc<Cell<usize>>);

#[cfg(debug_assertions)]
struct SwitchProfileAndContinue(&'static DialectProfile);

#[cfg(debug_assertions)]
struct SwitchProfileAndOk(&'static DialectProfile);

struct SwitchCompilerToCustom;

struct SwitchCompilerToDefault;

struct SwitchCompilerAndError;

struct SwitchCompilerAndReturn;

impl NativeCommand for CountCalls {
    fn invoke(&self, _vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        self.0.set(self.0.get() + 1);
        Completion::new(Code::Ok, Value::empty(), Value::empty())
    }
}

#[cfg(debug_assertions)]
impl NativeCommand for SwitchProfileAndContinue {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        vm.set_dialect_profile(self.0);
        Completion::new(Code::Continue, Value::empty(), Value::empty())
    }
}

#[cfg(debug_assertions)]
impl NativeCommand for SwitchProfileAndOk {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        vm.set_dialect_profile(self.0);
        Completion::new(Code::Ok, Value::string("switched"), Value::empty())
    }
}

impl NativeCommand for SwitchCompilerToCustom {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        vm.set_compiler(Box::new(custom_compile_service()));
        Completion::new(Code::Ok, Value::string("switched"), Value::empty())
    }
}

impl NativeCommand for SwitchCompilerToDefault {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        vm.set_compiler(Box::new(BytecodeCompileService::default()));
        Completion::new(Code::Ok, Value::string("switched"), Value::empty())
    }
}

impl NativeCommand for SwitchCompilerAndError {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        vm.set_compiler(Box::new(custom_compile_service()));
        Completion::new(
            Code::Error,
            Value::string("native tailcall error"),
            Value::empty(),
        )
    }
}

impl NativeCommand for SwitchCompilerAndReturn {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        vm.set_compiler(Box::new(custom_compile_service()));
        Completion::new(
            Code::Return,
            Value::string("native tailcall return"),
            Value::empty(),
        )
    }
}

fn custom_compile_service() -> BytecodeCompileService {
    let mut registry = tcl_registry::CommandRegistry::build_default();
    let mut list = registry.get("list").expect("list spec").clone();
    // This owned surface deliberately redefines list's compiler semantics as
    // Expr. Keeping the builtin list fold would make the registry internally
    // contradictory and let a value-position fold mask the custom hooks.
    list.const_fold = None;
    list.lowering_hook = Some(tcl_registry::hooks::LoweringHookId::Expr);
    list.inline_codegen_hook = Some(tcl_registry::hooks::InlineCodegenHookId::Expr);
    registry.insert(list);
    BytecodeCompileService::new(registry)
}

/// Cluster P test correction (every interpreter is a native core).
///
/// A native core's entered operation captures the pinned profile in its
/// currency receipt (`interp/native_operation_currency.rs`, unit test
/// `original_vm_context_receipt_refuses_host_profile_and_pack_change_restore`).
/// Debug builds re-prove it, so switching the profile while bytecode of that
/// operation is still entered or parked retires the receipt, and the first
/// refusal is this host refusal: it reaches the embedder as `TclError::Host`,
/// outside every guest `catch`/`try`. tclsh has no mid-run release switch, so
/// no C behaviour is at stake; the run still fails closed. A release build
/// stops one tick later with the guest error `cannot continue bytecode after
/// dialect profile changed`. A compile-service switch is left to that
/// freshness check in every build (`assert_stale_compile_service`).
#[cfg(debug_assertions)]
const MID_RUN_SWITCH_REFUSAL: &str = "entered VM original execution context";

#[cfg(debug_assertions)]
fn is_mid_run_switch_refusal(error: &tcl_runtime_api::NativeExecutionError) -> bool {
    matches!(
        error,
        tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(reason)
        ) if *reason == MID_RUN_SWITCH_REFUSAL
    )
}

#[cfg(debug_assertions)]
fn assert_mid_run_switch_refused(result: Result<Completion<Value>, tcl_vm::TclError>, label: &str) {
    match result {
        Err(tcl_vm::TclError::Host(tcl_vm::TclHostFailure::Execution(error)))
            if is_mid_run_switch_refusal(&error) => {}
        Err(error) => panic!("{label}: expected the mid-run switch refusal, got {error:?}"),
        Ok(completion) => {
            panic!("{label}: a mid-run switch must be refused at the host boundary: {completion:?}")
        }
    }
}

/// A compile-service switch under running bytecode stops that bytecode with
/// a guest error at its next step.
fn assert_stale_compile_service(
    result: Result<Completion<Value>, tcl_vm::TclError>,
    label: &str,
) -> Completion<Value> {
    let completion = result.unwrap_or_else(|error| panic!("{label}: {error:?}"));
    assert_eq!(completion.code, Code::Error, "{label}: {completion:?}");
    assert_eq!(
        completion.result.to_str().as_ref(),
        "cannot continue bytecode after compile service changed",
        "{label}"
    );
    completion
}

#[test]
fn builtin_replacement_crosses_proc_and_eval_unit_boundaries() {
    let (mut vm, output) = vm();
    eval_ok(
        &mut vm,
        "rename expr real_expr; proc expr args {return OVERRIDE}",
    );
    eval_ok(
        &mut vm,
        "proc p {} {set x [expr {1+2}]; return $x}; \
         puts [list proc [p]]; \
         puts [list eval [eval {set x [expr {1+2}]; set x}]]",
    );

    // Tcl 9.0.4: `proc OVERRIDE\neval OVERRIDE`.
    assert_eq!(
        String::from_utf8(output.0.borrow().clone()).expect("UTF-8 output"),
        "proc OVERRIDE\neval OVERRIDE\n"
    );
}

#[test]
fn cached_eval_is_not_reused_after_builtin_replacement() {
    let (mut vm, _output) = vm();
    assert_eq!(eval_ok(&mut vm, "expr {1+2}"), "3");
    eval_ok(
        &mut vm,
        "rename expr real_expr; proc expr args {return OVERRIDE}",
    );
    assert_eq!(eval_ok(&mut vm, "expr {1+2}"), "OVERRIDE");
}

#[test]
fn compiler_swap_invalidates_cached_eval_in_both_directions() {
    let (mut vm, _output) = vm();
    let source = "list {1 + 2}";
    assert_eq!(eval_ok(&mut vm, source), "{1 + 2}");

    vm.set_compiler(Box::new(custom_compile_service()));
    assert_eq!(eval_ok(&mut vm, source), "3");

    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    assert_eq!(eval_ok(&mut vm, source), "{1 + 2}");
}

#[test]
fn owned_registry_controls_value_position_constant_folds() {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(custom_compile_service()));

    // `set`'s value is emitted through the shared value-position fold and
    // inline-hook path. The owned registry maps list to Expr and removes
    // list's const-fold, so the nested call must follow the registry's
    // expression hook rather than being frozen with builtin list semantics.
    assert_eq!(
        eval_ok(&mut vm, "set result [list {1 + 2}]; set result"),
        "3"
    );
}

#[test]
fn foreign_module_procs_are_recompiled_by_the_current_service_in_both_orders() {
    let module = BytecodeCompileService::default()
        .compile("proc foreign_p {} {list {1 + 2}}; list {1 + 2}")
        .expect("default module compiles");

    for install_custom_first in [false, true] {
        let (mut vm, _output) = vm();
        if install_custom_first {
            vm.set_compiler(Box::new(custom_compile_service()));
        }

        // Public run_module preserves the supplied top-level bytecode's
        // semantics even when a different same-profile service is installed.
        let top = vm.run_module(&module);
        assert_eq!(top.code, Code::Ok);
        assert_eq!(top.result.to_str().as_ref(), "{1 + 2}");

        if !install_custom_first {
            vm.set_compiler(Box::new(custom_compile_service()));
        }
        // The source-bearing proc is foreign, not falsely stamped as custom;
        // first entry recompiles it through the current list->Expr service.
        assert_eq!(eval_ok(&mut vm, "foreign_p"), "3");
    }
}

/// A VM with no compile service cannot compile a foreign module's procedure
/// bodies, so calling one is a host refusal, never a guest error. Once a
/// service is installed the procedure the module defined compiles through it.
#[test]
fn source_less_vm_refuses_a_foreign_modules_procedure_bodies() {
    let module = BytecodeCompileService::default()
        .compile("proc foreign_self {} {list {1 + 2}}; foreign_self")
        .expect("default module compiles");
    let mut vm = tcl_vm::test_support::native_vm();

    let refusal = vm
        .try_run_module(&module)
        .expect_err("a source-less VM compiles no procedure body");
    assert!(
        matches!(
            &refusal,
            tcl_runtime_api::NativeExecutionError::CompileServiceRefusal(refusal)
                if refusal.reason == "procedure compilation requires a byte CompileService"
        ),
        "{refusal:?}"
    );

    vm.set_compiler(Box::new(custom_compile_service()));
    assert_eq!(eval_ok(&mut vm, "foreign_self"), "3");
}

#[test]
fn foreign_proc_provenance_name_must_match_its_module_entry() {
    let service = BytecodeCompileService::default();
    let mut malformed = service
        .compile("proc wrong {} {return POISON}")
        .expect("malformed source module starts valid");
    assert_eq!(malformed.procedures.len(), 1, "one original procedure");
    let id = *malformed
        .procedures
        .keys()
        .next()
        .expect("compiler-issued body");
    let provenance = malformed
        .procedure_provenance
        .get_mut(&id)
        .expect("original procedure provenance exists");
    assert_eq!(provenance.name, "::wrong");
    provenance.name = "::target".to_owned();
    provenance.body = "return CLAIMED".to_owned();
    let valid = service
        .compile("proc target {} {return CLAIMED}; target")
        .expect("valid module compiles");
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));

    // Admitting the malformed module defines only its own procedure; its
    // forged provenance must not poison another name.
    assert_eq!(vm.run_module(&malformed).code, Code::Ok);
    let completion = vm.run_module(&valid);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "CLAIMED");
    assert_eq!(eval_ok(&mut vm, "wrong"), "POISON");
}

#[test]
fn compiler_swap_refreshes_function_proc_and_tcloo_bodies() {
    let (mut vm, _output) = vm();
    let handle = vm
        .compile_function("list {1 + 2}")
        .expect("function compiles");
    eval_ok(
        &mut vm,
        "proc p {} {list {1 + 2}}; \
         oo::class create C {method m {} {list {1 + 2}}}; \
         set o [C new]",
    );

    vm.set_compiler(Box::new(custom_compile_service()));
    assert_eq!(vm.invoke_function(&handle).result.to_str().as_ref(), "3");
    assert_eq!(eval_ok(&mut vm, "p"), "3");
    assert_eq!(eval_ok(&mut vm, "$o m"), "3");

    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    assert_eq!(
        vm.invoke_function(&handle).result.to_str().as_ref(),
        "{1 + 2}"
    );
    assert_eq!(eval_ok(&mut vm, "p"), "{1 + 2}");
    assert_eq!(eval_ok(&mut vm, "$o m"), "{1 + 2}");
}

#[test]
fn suspended_coroutine_fails_closed_after_compiler_swap() {
    let (mut vm, _output) = vm();
    assert_eq!(
        eval_ok(
            &mut vm,
            "coroutine c apply {{} {yield ready; list {1 + 2}}}",
        ),
        "ready"
    );

    // The parked coroutine's bytecode was compiled by the old service.
    vm.set_compiler(Box::new(custom_compile_service()));
    assert_stale_compile_service(vm.eval_source("c"), "resume after swap to custom");

    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(custom_compile_service()));
    assert_eq!(
        eval_ok(
            &mut vm,
            "coroutine c apply {{} {yield ready; list {1 + 2}}}",
        ),
        "ready"
    );
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    assert_stale_compile_service(vm.eval_source("c"), "resume after swap to default");
}

#[test]
fn stale_coroutine_teardown_does_not_run_queued_injection() {
    let (mut vm, _output) = vm();
    let injections = Rc::new(Cell::new(0));
    let unsets = Rc::new(Cell::new(0));
    let execution_leaves = Rc::new(Cell::new(0));
    let coroutine_deletes = Rc::new(Cell::new(0));
    vm.register_native_command(
        "count_injection",
        Rc::new(CountCalls(Rc::clone(&injections))),
    );
    vm.register_native_command("count_unset", Rc::new(CountCalls(Rc::clone(&unsets))));
    vm.register_native_command(
        "count_execution_leave",
        Rc::new(CountCalls(Rc::clone(&execution_leaves))),
    );
    vm.register_native_command(
        "count_coroutine_delete",
        Rc::new(CountCalls(Rc::clone(&coroutine_deletes))),
    );
    eval_ok(
        &mut vm,
        "proc worker {} {set local 1; \
             trace add variable local unset count_unset; \
             yield ready; return done}; \
         trace add execution worker leave count_execution_leave; \
         coroutine c apply {{} {worker}}; \
         coroinject c count_injection; \
         trace add command c delete count_coroutine_delete",
    );

    vm.set_compiler(Box::new(custom_compile_service()));
    let completion = vm.eval_source("c resumed").expect("resume compiles");
    assert_eq!(completion.code, Code::Error);
    assert_eq!(
        completion.result.to_str().as_ref(),
        "cannot continue bytecode after compile service changed"
    );
    assert_eq!(
        injections.get(),
        0,
        "stale resume must not execute coroinject"
    );
    assert_eq!(unsets.get(), 1, "normal unwind must fire local unset trace");
    assert_eq!(
        execution_leaves.get(),
        1,
        "normal unwind must settle the suspended worker's execution leave once"
    );
    assert_eq!(
        coroutine_deletes.get(),
        1,
        "stale teardown must delete the coroutine command exactly once"
    );
    assert_eq!(eval_ok(&mut vm, "info commands c"), "");
}

#[test]
fn tailcall_non_ok_after_compiler_swap_rejects_the_stale_parent() {
    for (name, command) in [
        (
            "switch_compiler_and_error",
            Rc::new(SwitchCompilerAndError) as Rc<dyn NativeCommand>,
        ),
        (
            "switch_compiler_and_return",
            Rc::new(SwitchCompilerAndReturn) as Rc<dyn NativeCommand>,
        ),
    ] {
        let (mut vm, _output) = vm();
        vm.register_native_command(name, command);
        eval_ok(
            &mut vm,
            &format!(
                "proc tailcaller {{}} {{tailcall {name}}}; \
                 proc parent {{}} {{tailcaller; return unreachable}}"
            ),
        );

        // The stale parent never continues past the tailcall, and `return
        // unreachable` cannot become the result.
        assert_stale_compile_service(vm.eval_source("parent"), name);
    }
}

#[test]
fn final_native_compiler_swap_cannot_settle_old_deferred_bytecode() {
    for source in [
        "catch {switch_compiler}",
        "set head catch; $head {switch_compiler}",
        "try {switch_compiler}",
        "set head try; $head {switch_compiler}",
    ] {
        let (mut vm, _output) = vm();
        vm.register_native_command("switch_compiler", Rc::new(SwitchCompilerToCustom));
        let completion = vm.eval_source(source).expect("source compiles");
        assert_eq!(completion.code, Code::Error, "{source}: {completion:?}");
        assert_eq!(
            completion.result.to_str().as_ref(),
            "cannot continue bytecode after compile service changed",
            "{source}"
        );

        let mut vm = tcl_vm::test_support::native_vm();
        vm.set_compiler(Box::new(custom_compile_service()));
        vm.register_native_command("switch_compiler", Rc::new(SwitchCompilerToDefault));
        let completion = vm.eval_source(source).expect("source compiles");
        assert_eq!(completion.code, Code::Error, "{source}: {completion:?}");
        assert_eq!(
            completion.result.to_str().as_ref(),
            "cannot continue bytecode after compile service changed",
            "{source}"
        );
    }
}

#[test]
fn stale_coroutine_ancestor_cannot_leak_from_fresh_try_handler_or_finally() {
    for source in [
        "coroutine c apply {{} {set h try; $h {switch_compiler} on error {m o} {yield leaked}}}",
        "coroutine c apply {{} {set h try; $h {switch_compiler} finally {yield leaked}}}",
    ] {
        let (mut vm, _output) = vm();
        vm.register_native_command("switch_compiler", Rc::new(SwitchCompilerToCustom));
        // The fresh handler or finally clause never yields `leaked`.
        assert_stale_compile_service(vm.eval_source(source), source);
        assert_eq!(eval_ok(&mut vm, "info commands c"), "");
    }
}

#[test]
fn fresh_try_return_cannot_pop_a_stale_directly_invoked_proc() {
    for body in [
        "set h try; $h {switch_compiler} on error {m o} {return leaked}",
        "set h try; $h {switch_compiler} finally {return leaked}",
    ] {
        let (mut vm, _output) = vm();
        vm.register_native_command("switch_compiler", Rc::new(SwitchCompilerToCustom));
        eval_ok(&mut vm, &format!("proc p {{}} {{{body}}}"));

        // The fresh handler or finally clause never returns `leaked`.
        assert_stale_compile_service(
            vm.try_invoke_command("p", &[])
                .map_err(tcl_vm::TclError::from_execution_failure),
            body,
        );
    }
}

#[cfg(debug_assertions)]
#[test]
fn terminal_profile_flips_fail_closed_in_catch_try_and_trace() {
    let v90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let v84 = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    for source in [
        "catch {switch_profile}",
        "set head catch; $head {switch_profile}",
        "try {switch_profile}",
        "set head try; $head {switch_profile}",
        "set x 0; trace add variable x write switch_profile; set x 1",
    ] {
        let mut vm = tcl_vm::test_support::native_vm_for_profile(v90, Box::new(std::io::stdout()));
        vm.set_compiler(Box::new(BytecodeCompileService::for_profile(v90)));
        vm.register_native_command("switch_profile", Rc::new(SwitchProfileAndOk(v84)));

        // Cluster P correction: the switch is refused at the host boundary
        // (see `assert_mid_run_switch_refused`), not as a guest error a
        // `catch`/`try` or trace could absorb.
        assert_mid_run_switch_refused(vm.eval_source(source), source);
        // The refusal is retired at the outer boundary; the interpreter keeps
        // running under the profile the host selected.
        assert_eq!(eval_ok(&mut vm, "set after 1"), "1", "{source}");
    }
}

#[test]
fn runtime_foreach_revalidates_its_stored_body_between_iterations() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc mutate_expr {} {\
             rename expr real_expr; \
             proc expr args {return FOREACH_OVERRIDE}\
         }",
    );

    let result = eval_ok(
        &mut vm,
        "set loop foreach; \
         set seen {}; \
         $loop i {0 1} {\
             if {$i == 0} {mutate_expr; continue}; \
             lappend seen [expr {1+2}]\
         }; \
         set seen",
    );

    // Tcl 9.0.4: iteration one replaces `expr`; the runtime fallback's stored
    // body must observe that replacement when it is activated for iteration two.
    assert_eq!(result, "FOREACH_OVERRIDE");
}

#[test]
fn runtime_lmap_revalidates_its_stored_body_between_iterations() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc mutate_expr {} {\
             rename expr real_expr; \
             proc expr args {return LMAP_OVERRIDE}\
         }",
    );

    let result = eval_ok(
        &mut vm,
        "set loop lmap; \
         $loop i {0 1} {\
             if {$i == 0} {mutate_expr; continue}; \
             expr {1+2}\
         }",
    );

    // Tcl 9.0.4: `continue` omits iteration one and the replacement command's
    // result is the sole value collected from iteration two.
    assert_eq!(result, "LMAP_OVERRIDE");
}

#[test]
fn runtime_foreach_revalidates_after_a_loop_variable_write_trace() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc mutate_expr_trace args {\
             rename expr real_expr; \
             proc expr args {return TRACE_OVERRIDE}\
         }",
    );

    let result = eval_ok(
        &mut vm,
        "trace add variable i write mutate_expr_trace; \
         set seen {}; \
         set loop foreach; \
         $loop i {0} {lappend seen [expr {1+2}]}; \
         set seen",
    );

    // The Rust-side loop-variable assignment runs its write trace after the
    // body was compiled but before its first deferred activation is pushed.
    assert_eq!(result, "TRACE_OVERRIDE");
}

#[cfg(debug_assertions)]
#[test]
fn runtime_foreach_does_not_tag_old_profile_assembly_as_fresh() {
    let v90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let v84 = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(v90, Box::new(std::io::stdout()));
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(v90)));

    let body_calls = Rc::new(Cell::new(0));
    vm.register_native_command("count_body", Rc::new(CountCalls(Rc::clone(&body_calls))));
    vm.register_native_command("switch_profile", Rc::new(SwitchProfileAndContinue(v84)));

    let result = vm.eval_source(
        "set loop foreach; \
         $loop i {0 1} {\
             count_body; \
             if {$i == 0} {switch_profile}; \
             lassign {a b} x; \
             set x\
         }",
    );

    // The host command ends iteration one with `continue` as it flips profile.
    // The stored Tcl 9.0 body must fail closed before entering iteration two,
    // rather than executing its old `lassign` lowering under Tcl 8.4. Cluster
    // P correction: on a native core the first refusal is the host-boundary
    // currency refusal (`assert_mid_run_switch_refused`).
    assert_mid_run_switch_refused(result, "foreach profile flip");
    assert_eq!(body_calls.get(), 1, "stale body ran a second time");
}

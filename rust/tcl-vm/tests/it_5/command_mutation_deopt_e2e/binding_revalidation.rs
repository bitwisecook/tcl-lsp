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

//! A compiled command binding is revalidated after a shadow, a forgotten import or a namespace teardown, and an active frame redispatches after a nested command mutation.

use super::*;

#[test]
fn namespaced_builtin_shadow_invalidates_future_proc_compilation() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "namespace eval n {proc expr args {return NAMESPACE}}",
    );
    eval_ok(
        &mut vm,
        "namespace eval n {proc p {} {set x [expr {1+2}]; return $x}}",
    );
    assert_eq!(eval_ok(&mut vm, "n::p"), "NAMESPACE");
}

#[test]
fn forgotten_import_revalidates_a_compiled_proc_binding() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "namespace eval :: {namespace export expr}; \
         namespace eval N {namespace import ::expr; proc p {} {expr {1+2}}}",
    );
    assert_eq!(eval_ok(&mut vm, "N::p"), "3");
    eval_ok(
        &mut vm,
        "rename expr real_expr; proc expr args {return OVERRIDE}; \
         namespace eval N {namespace forget expr}",
    );

    // Tcl 9.0.4: the imported command follows the original token through its
    // rename, then forgetting that alias makes the proc's unqualified lookup
    // reach the replacement global command.
    assert_eq!(eval_ok(&mut vm, "N::p"), "OVERRIDE");
}

#[test]
fn namespace_teardown_revalidates_a_cached_eval_binding() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "namespace eval N {proc expr args {return OVERRIDE}}",
    );
    assert_eq!(
        eval_ok(&mut vm, "namespace eval N {expr {1+2}}"),
        "OVERRIDE"
    );
    eval_ok(&mut vm, "namespace delete N; namespace eval N {}");

    // Tcl 9.0.4: recreating the namespace does not preserve the deleted
    // shadow command, so the same eval body now resolves the global builtin.
    assert_eq!(eval_ok(&mut vm, "namespace eval N {expr {1+2}}"), "3");
}

#[test]
fn untouched_builtin_keeps_normal_result() {
    let (mut vm, _output) = vm();
    eval_ok(&mut vm, "proc p {} {set x [expr {1+2}]; return $x}");
    assert_eq!(eval_ok(&mut vm, "p"), "3");
}

#[test]
fn active_proc_frame_redispatches_after_nested_command_mutation() {
    let (mut vm, _output) = vm();
    let compiled = BytecodeCompileService::default()
        .compile("proc p {} {mutate; set x [expr {1+2}]; return $x}")
        .expect("module compiles");
    assert!(
        sole_compiled_procedure(&compiled)
            .command_bindings
            .iter()
            .any(|binding| binding.name == "expr" && binding.identity == "expr"),
        "p must begin as an optimised active frame"
    );
    eval_ok(&mut vm, "proc p {} {mutate; set x [expr {1+2}]; return $x}");
    eval_ok(
        &mut vm,
        "proc mutate {} {eval {rename expr real_expr; proc expr args {return OVERRIDE}}}",
    );

    // Tcl 9.0.4: OVERRIDE. The mutation occurs through nested `eval` inside
    // `mutate`, after p's optimised frame is already active; p's next source
    // command must redispatch rather than run its stale expr opcode.
    assert_eq!(eval_ok(&mut vm, "p"), "OVERRIDE");
}

#[test]
fn fused_expr_keeps_its_entered_token_when_an_operand_mutates_expr() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc mutate {} {\
             rename expr saved_expr; \
             proc expr args {return REPLACED}; \
             return 2}; \
         proc p {} {\
             set first [expr {[mutate]+1}]; \
             list $first [expr {2+2}]}",
    );

    // Tcl 9.0.4: `3 REPLACED`. The first expr command was entered before its
    // operand replaced the live spelling, so it finishes with the original
    // token; the next source boundary observes the replacement.
    assert_eq!(eval_ok(&mut vm, "p"), "3 REPLACED");
}

#[test]
fn active_return_expr_redispatches_after_nested_command_mutation() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc mutate {} {rename expr real_expr; proc expr args {return RETURN_OVERRIDE}}; \
         proc p {} {mutate; return [expr {1+2}]}",
    );

    // Tcl 9.0.4: RETURN_OVERRIDE. `return` consumes the nested expr command
    // into typed IR, but the live frame must still depend on expr's identity.
    assert_eq!(eval_ok(&mut vm, "p"), "RETURN_OVERRIDE");
}

#[test]
fn active_catch_body_redispatches_after_nested_command_mutation() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc mutate {} {rename expr real_expr; proc expr args {return CATCH_OVERRIDE}}; \
         proc p {} {mutate; set rc [catch {expr {1+2}} value]; list $rc $value}",
    );

    // Tcl 9.0.4: `0 CATCH_OVERRIDE`. The catch emitter consumes both catch
    // and its body command, so both bindings participate in revalidation.
    assert_eq!(eval_ok(&mut vm, "p"), "0 CATCH_OVERRIDE");
}

#[test]
fn active_nested_try_body_redispatches_after_nested_command_mutation() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc mutate {} {rename expr real_expr; proc expr args {return TRY_OVERRIDE}}; \
         proc p {} {mutate; set rc [catch {try {expr {1+2}} on error {m} {set m handled}} value]; list $rc $value}",
    );

    // Tcl 9.0.4: `0 TRY_OVERRIDE`. The nested try body is another inline
    // compiler context, not a reason to lose expr's live binding identity.
    assert_eq!(eval_ok(&mut vm, "p"), "0 TRY_OVERRIDE");
}

#[test]
fn active_proc_frame_revalidates_a_cfg_consumed_while_head() {
    let (mut vm, _output) = vm();
    let compiled = BytecodeCompileService::default()
        .compile("proc p {} {mutate; while {0} {}}")
        .expect("module compiles");
    assert!(
        sole_compiled_procedure(&compiled)
            .command_bindings
            .iter()
            .any(|binding| binding.name == "while" && binding.identity == "while")
    );
    assert!(
        sole_compiled_procedure(&compiled)
            .instructions
            .iter()
            .any(|instruction| {
                instruction.op == tcl_bytecode::Op::START_CMD
                    && instruction.source_cmd_text == "while {0} {}"
            })
    );

    eval_ok(&mut vm, "proc p {} {mutate; while {0} {}}");
    eval_ok(
        &mut vm,
        "proc mutate {} {rename while real_while; proc while args {return WHILE_OVERRIDE}}",
    );
    let fast_calls = Rc::new(Cell::new(0));
    let plain_calls = Rc::new(Cell::new(0));
    vm.set_compiler(Box::new(CountingCompilerSvc {
        inner: BytecodeCompileService::default(),
        fast_calls: Rc::clone(&fast_calls),
        plain_calls: Rc::clone(&plain_calls),
    }));
    let result = eval_ok(&mut vm, "p");
    assert!(plain_calls.get() > 0, "the live while frame must deopt");
    assert_eq!(result, "WHILE_OVERRIDE");
}

#[test]
fn active_proc_frame_revalidates_a_cfg_consumed_eval_head() {
    let (mut vm, _output) = vm();
    let source = "proc p {} {mutate; eval {set ::body_ran 1}}";
    let compiled = BytecodeCompileService::default()
        .compile(source)
        .expect("module compiles");
    assert!(
        sole_compiled_procedure(&compiled)
            .command_bindings
            .iter()
            .any(|binding| binding.name == "eval" && binding.identity == "eval")
    );
    assert!(
        sole_compiled_procedure(&compiled)
            .instructions
            .iter()
            .any(|instruction| {
                instruction.op == tcl_bytecode::Op::START_CMD
                    && instruction.source_cmd_text == "eval {set ::body_ran 1}"
            })
    );

    eval_ok(
        &mut vm,
        "set ::body_ran 0; proc p {} {mutate; eval {set ::body_ran 1}}",
    );
    eval_ok(
        &mut vm,
        "proc mutate {} {rename eval real_eval; proc eval args {return EVAL_OVERRIDE}}",
    );
    let fast_calls = Rc::new(Cell::new(0));
    let plain_calls = Rc::new(Cell::new(0));
    vm.set_compiler(Box::new(CountingCompilerSvc {
        inner: BytecodeCompileService::default(),
        fast_calls: Rc::clone(&fast_calls),
        plain_calls: Rc::clone(&plain_calls),
    }));

    assert_eq!(eval_ok(&mut vm, "p"), "EVAL_OVERRIDE");
    assert_eq!(eval_ok(&mut vm, "set ::body_ran"), "0");
    assert!(plain_calls.get() > 0, "the live eval frame must deopt");
}

#[test]
fn stale_constant_if_boundary_replays_only_the_owning_command() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "set ::mutations 0; set ::body_ran 0; \
         proc mutate {} {incr ::mutations; rename if real_if; proc if args {return IF_OVERRIDE}}; \
         proc p {} {mutate; if {1} {set ::body_ran 1}}",
    );
    assert_eq!(eval_ok(&mut vm, "p"), "IF_OVERRIDE");
    assert_eq!(eval_ok(&mut vm, "list $::mutations $::body_ran"), "1 0");
}

#[test]
fn replacement_delete_trace_reentry_cannot_acknowledge_the_preinstall_epoch() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc generator {} {yield ready; set first [expr {1+1}]; yield $first; expr {2+2}}; \
         coroutine c generator; \
         proc resume_on_delete args {set ::during_delete [c]}; \
         trace add command expr delete resume_on_delete; \
         proc expr args {return EXPR_OVERRIDE}",
    );

    assert_eq!(
        eval_ok(&mut vm, "list $::during_delete [c]"),
        "2 EXPR_OVERRIDE"
    );
}

#[test]
fn active_proc_frame_redispatches_after_namespace_path_mutation() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "namespace eval Override {proc expr args {return PATH_OVERRIDE}}; \
         namespace eval N { \
             proc mutate {} {namespace path ::Override}; \
             proc p {} {mutate; expr {1+2}} \
         }",
    );

    // Tcl 9.0.4: PATH_OVERRIDE. `p` entered with unqualified `expr`
    // resolving to the global builtin, but its preceding command changes N's
    // resolution path while the optimised frame is live. The next source
    // command must revalidate before executing its stale expression opcodes.
    assert_eq!(eval_ok(&mut vm, "N::p"), "PATH_OVERRIDE");
}

#[test]
fn fused_expr_alias_uses_the_alias_binding_identity() {
    let (mut vm, _output) = vm();
    let compiled = BytecodeCompileService::default()
        .compile("interp alias {} e {} expr; proc p {} {set x [e {1+2}]; return $x}")
        .expect("module compiles");
    assert!(
        sole_compiled_procedure(&compiled)
            .command_bindings
            .iter()
            .any(|binding| binding.name == "e" && binding.identity == "expr"),
        "the fused alias must retain its source and target identities"
    );
    eval_ok(
        &mut vm,
        "interp alias {} e {} expr; \
         proc p {} {set x [e {1+2}]; return $x}",
    );
    assert_eq!(eval_ok(&mut vm, "p"), "3");
    eval_ok(
        &mut vm,
        "interp alias {} e {} {}; proc e args {return ALIAS_OVERRIDE}",
    );

    // Tcl 9.0.4: ALIAS_OVERRIDE. The optimised AssignExpr assumption belongs
    // to `e -> expr`, not to the unrelated live spelling `expr`.
    assert_eq!(eval_ok(&mut vm, "p"), "ALIAS_OVERRIDE");
}

#[test]
fn fused_expr_alias_tracks_its_source_implementation() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "interp alias {} e {} expr; \
         proc p {} {set x [e {1+2}]; return $x}",
    );
    assert_eq!(eval_ok(&mut vm, "p"), "3");
    eval_ok(
        &mut vm,
        "rename expr real_expr; proc expr args {return TARGET_OVERRIDE}",
    );

    // The source spelling `e` is unchanged, but it no longer reaches the
    // registry `expr` implementation. Its target identity is part of the same
    // dependency stamp, so the cached proc redispatches through the alias.
    assert_eq!(eval_ok(&mut vm, "p"), "TARGET_OVERRIDE");
}

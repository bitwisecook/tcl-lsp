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

//! The embedder surface (`tcl_vm::embed`), closed in the
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
            Err(error) => Completion::new(
                Code::Error,
                error.into_value().expect("guest fixture compilation error"),
                Value::empty(),
            ),
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
    assert!(
        !vm.command_names()
            .expect("Unicode command names")
            .iter()
            .any(|name| name == "lassign")
    );

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
    let before = vm.command_names().expect("Unicode command names").len();
    let allowed = ["set", "expr", "if", "string", "list", "lindex", "return"];
    let removed = vm.retain_commands(&|name| allowed.contains(&name));
    assert!(removed > 0 && removed < before);
    assert!(
        vm.command_names()
            .expect("Unicode command names")
            .iter()
            .all(|n| allowed.contains(&&**n))
    );

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

/// Run `script` on the VM and answer its result, or its error as `error: …`.
fn run(vm: &mut Vm, script: &str) -> String {
    match vm.eval_source(script) {
        Ok(completion) if completion.code.is_ok() => completion.result.to_str().to_string(),
        Ok(completion) => format!("error: {}", completion.result.to_str()),
        Err(error) => format!("error: {}", error.message),
    }
}

#[test]
fn a_package_the_host_provides_satisfies_a_require() {
    let mut vm = vm();
    assert_eq!(
        run(&mut vm, "package require hostpkg"),
        "error: can't find package hostpkg",
        "nothing provides it yet"
    );
    vm.package_provide("hostpkg", "1.2")
        .expect("a version is provided");
    assert_eq!(run(&mut vm, "package require hostpkg"), "1.2");
    assert_eq!(run(&mut vm, "package provide hostpkg"), "1.2");
    assert_eq!(
        run(&mut vm, "package require hostpkg 1.1"),
        "1.2",
        "a requirement the version meets"
    );
    assert!(
        run(&mut vm, "package require hostpkg 2.0").starts_with("error: "),
        "and one it does not"
    );
}

#[test]
fn a_package_the_host_provides_is_refused_as_package_provide_refuses_it() {
    let mut vm = vm();
    vm.package_provide("hostpkg", "1.0")
        .expect("a version is provided");
    vm.package_provide("hostpkg", "1.0")
        .expect("the same version again is a no-op");

    let conflict = vm
        .package_provide("hostpkg", "2.0")
        .expect_err("a different version conflicts");
    assert_eq!(
        run(&mut vm, "package provide hostpkg 2.0"),
        format!("error: {}", conflict.message),
        "the message is the script command's"
    );
    assert_eq!(
        conflict.message,
        "conflicting versions provided for package \"hostpkg\": 1.0, then 2.0"
    );
    assert_eq!(
        conflict.error_code.as_deref(),
        Some("TCL PACKAGE VERSIONCONFLICT")
    );

    let invalid = vm
        .package_provide("other", "1..2")
        .expect_err("a malformed version is refused");
    assert_eq!(
        run(&mut vm, "package provide other 1..2"),
        format!("error: {}", invalid.message),
        "as the script command refuses it"
    );
    assert_eq!(
        run(&mut vm, "package provide other"),
        "",
        "and nothing is provided"
    );
}

#[test]
fn info_loaded_lists_the_libraries_a_host_recorded() {
    let mut vm = vm();
    assert_eq!(run(&mut vm, "info loaded"), "", "nothing is loaded");
    assert_eq!(run(&mut vm, "info loaded {}"), "");

    vm.library_loaded("", "Pkga");
    vm.library_loaded("/opt/pkgb/libpkgb.so", "Pkgb");
    assert_eq!(
        run(&mut vm, "info loaded"),
        "{{} Pkga} {/opt/pkgb/libpkgb.so Pkgb}"
    );
    assert_eq!(
        run(&mut vm, "info loaded {}"),
        "{{} Pkga} {/opt/pkgb/libpkgb.so Pkgb}",
        "the current interpreter's, as the empty interpreter name asks"
    );
    assert_eq!(run(&mut vm, "info loaded {} Pkgb"), "/opt/pkgb/libpkgb.so");
    assert_eq!(
        run(&mut vm, "info loaded {} Pkga"),
        "",
        "a static library's file is empty"
    );
    assert_eq!(
        run(&mut vm, "info loaded {} Nosuch"),
        "",
        "an unloaded prefix"
    );
    assert_eq!(
        run(&mut vm, "info loaded child"),
        "error: could not find interpreter \"child\"",
        "another interpreter is not reachable"
    );

    vm.library_loaded("/elsewhere/libpkgb.so", "Pkgb");
    assert_eq!(
        run(&mut vm, "info loaded {} Pkgb"),
        "/opt/pkgb/libpkgb.so",
        "a prefix is listed under the file it was first loaded from"
    );
    assert_eq!(
        run(&mut vm, "info loaded"),
        "{{} Pkga} {/opt/pkgb/libpkgb.so Pkgb}",
        "and once"
    );
}

fn failed(result: Result<Value, tcl_vm::TclError>) -> String {
    let completion = result
        .expect_err("an error")
        .into_completion()
        .expect("original guest completion");
    assert_eq!(completion.code, Code::Error);
    completion.result.to_str().to_string()
}

#[test]
fn a_variable_is_read_written_and_unset_as_set_and_unset_do() {
    let mut vm = vm();
    vm.write_variable("x", Value::string("1"))
        .expect("a scalar is set");
    assert_eq!(run(&mut vm, "set x"), "1");
    assert_eq!(vm.read_variable("x").expect("reads").to_str().as_ref(), "1");

    vm.write_variable("a(k)", Value::string("v"))
        .expect("an element is set");
    assert_eq!(run(&mut vm, "set a(k)"), "v", "as the array element it is");
    assert_eq!(
        vm.read_variable("a(k)").expect("reads").to_str().as_ref(),
        "v"
    );

    assert_eq!(
        failed(vm.read_variable("nosuch")),
        "can't read \"nosuch\": no such variable"
    );
    assert_eq!(
        failed(vm.read_variable("a")),
        "can't read \"a\": variable is array"
    );
    assert_eq!(
        failed(vm.read_variable("a(no)")),
        "can't read \"a(no)\": no such element in array"
    );
    assert_eq!(
        failed(vm.read_variable("x(k)")),
        "can't read \"x(k)\": variable isn't array"
    );
    assert_eq!(
        vm.write_variable("a", Value::string("2"))
            .expect_err("an array is not a scalar")
            .into_completion()
            .expect("original guest completion")
            .result
            .to_str()
            .as_ref(),
        "can't set \"a\": variable is array"
    );

    vm.unset_variable("a(k)").expect("an element is unset");
    assert_eq!(run(&mut vm, "info exists a(k)"), "0");
    vm.unset_variable("x").expect("a scalar is unset");
    assert_eq!(run(&mut vm, "info exists x"), "0");
    assert_eq!(
        vm.unset_variable("nosuch")
            .expect_err("nothing to unset")
            .into_completion()
            .expect("original guest completion")
            .result
            .to_str()
            .as_ref(),
        "can't unset \"nosuch\": no such variable"
    );
}

#[test]
fn the_variable_forms_fire_the_traces_a_script_would() {
    let mut vm = vm();
    assert_eq!(
        run(
            &mut vm,
            "set log {}; set x 0; \
             trace add variable x {read write unset} {apply {{n1 n2 op} {lappend ::log $op}}}; \
             set log {}"
        ),
        ""
    );
    vm.write_variable("x", Value::string("1")).expect("writes");
    vm.read_variable("x").expect("reads");
    vm.unset_variable("x").expect("unsets");
    assert_eq!(
        run(&mut vm, "set log"),
        "write read unset",
        "each form fires the trace of its operation"
    );
}

struct VariableTraceHostRefusal;

impl NativeCommand for VariableTraceHostRefusal {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        vm.set_var("::reached", Value::string("BEFORE"))
            .expect("actual trace entry marker");
        vm.refuse_host_command("original variable trace refusal".into())
    }
}

#[test]
fn variable_embedding_returns_original_trace_host_cause_and_preserves_prior_effects() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for version in tcl_dialect::TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        for operation in ["r", "w", "u"] {
            let mut vm = Vm::new();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
            vm.register_native_command("trace_refusal", Rc::new(VariableTraceHostRefusal));
            vm.try_eval_source(&format!(
                "set target ORIGINAL; set prior BEFORE; trace variable target {operation} trace_refusal",
            )).unwrap();
            let error = match operation {
                "r" => vm.read_variable("target").map(|_| ()),
                "w" => vm.write_variable("target", Value::string("REPLACEMENT")),
                "u" => vm.unset_variable("target"),
                _ => unreachable!(),
            }
            .expect_err("actual reached trace host refusal");
            let Err(tcl_vm::TclHostFailure::Execution(
                tcl_runtime_api::NativeExecutionError::HostCommandRefusal(original),
            )) = error.into_completion()
            else {
                panic!("a host trace failure cannot supply a guest completion");
            };
            assert_eq!(original.reason, "original variable trace refusal");
            assert_eq!(original.source_profile, profile.cache_key());
            assert_eq!(original.native_profile, profile.cache_key());
            assert_eq!(original.namespace.as_ref(), "::");
            assert_eq!(original.frame, 0);
            assert_eq!(
                vm.get_var("prior").unwrap().string_bytes().as_ref(),
                b"BEFORE"
            );
            assert_eq!(
                vm.get_var("reached").unwrap().string_bytes().as_ref(),
                b"BEFORE"
            );
            vm.write_variable("next", Value::string("NEXT")).unwrap();
            assert_eq!(
                vm.read_variable("next").unwrap().string_bytes().as_ref(),
                b"NEXT"
            );
        }
    }
}

#[test]
fn embedding_entry_refuses_an_earlier_original_cause_before_new_variable_or_package_effects() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let original = tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
        "original embedding operation",
    );
    for operation in ["read", "write", "unset", "package"] {
        let mut vm = vm();
        vm.write_variable("kept", Value::string("BEFORE")).unwrap();
        let _ = vm.refuse_tcl_host_failure(tcl_vm::TclHostFailure::ValueAccess(original));
        let error = match operation {
            "read" => vm.read_variable("kept").map(|_| ()),
            "write" => vm.write_variable("kept", Value::string("AFTER")),
            "unset" => vm.unset_variable("kept"),
            "package" => vm.package_provide("unentered", "1.0"),
            _ => unreachable!(),
        }
        .expect_err("earlier original cause wins before another operation");
        assert_eq!(
            error.into_completion().unwrap_err(),
            tcl_vm::TclHostFailure::Execution(
                tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(original)
            ),
        );
        assert_eq!(
            vm.get_var("kept").unwrap().string_bytes().as_ref(),
            b"BEFORE"
        );
        vm.package_provide("unentered", "2.0").unwrap();
    }
}

fn failure(message: &str, options: Value) -> Completion<Value> {
    Completion::new(Code::Error, Value::string(message), options)
}

#[test]
fn an_error_a_host_took_as_its_own_leaves_error_code_and_error_info() {
    // Software metadata contract: naming.diagnostics.original-error-code-metadata-not-message
    // docs/design/analysis/name-resolution-proofs/diagnostics-original-error-code-metadata-not-message.md
    let mut vm = vm();
    assert_eq!(
        run(&mut vm, "info exists errorCode"),
        "0",
        "nothing has failed"
    );
    vm.publish_caught_error(&failure(
        "boom",
        Value::list(vec![Value::string("-errorcode"), Value::string("MY CODE")]),
    ))
    .unwrap();
    assert_eq!(run(&mut vm, "set errorCode"), "MY CODE");
    assert_eq!(run(&mut vm, "set errorInfo"), "boom");

    vm.publish_caught_error(&failure("plain", Value::empty()))
        .unwrap();
    assert_eq!(
        run(&mut vm, "set errorCode"),
        "NONE",
        "an error that carries no code is NONE, as a catch publishes it"
    );
    assert_eq!(run(&mut vm, "set errorInfo"), "plain");

    vm.publish_caught_error(&failure("wrong # args: should be \"x\"", Value::empty()))
        .unwrap();
    assert_eq!(
        run(&mut vm, "set errorCode"),
        "NONE",
        "arbitrary guest text does not supply structured wrong-arguments metadata"
    );
    vm.publish_caught_error(&failure(
        "explicit usage failure",
        Value::list(vec![
            Value::string("-errorcode"),
            Value::string("TCL WRONGARGS"),
        ]),
    ))
    .unwrap();
    assert_eq!(run(&mut vm, "set errorCode"), "TCL WRONGARGS");
}

#[test]
fn a_completion_that_is_not_an_error_publishes_nothing() {
    let mut vm = vm();
    for code in [Code::Ok, Code::Return, Code::Break, Code::Continue] {
        vm.publish_caught_error(&Completion::new(code, Value::string("x"), Value::empty()))
            .unwrap();
    }
    assert_eq!(run(&mut vm, "info exists errorCode"), "0");
    assert_eq!(run(&mut vm, "info exists errorInfo"), "0");
}

#[test]
fn confined_stores_keep_a_taken_error_out_of_the_globals() {
    let mut vm = vm();
    vm.set_stores_confined(true);
    vm.publish_caught_error(&failure("boom", Value::empty()))
        .unwrap();
    vm.set_stores_confined(false);
    assert_eq!(
        run(&mut vm, "info exists errorCode"),
        "0",
        "a body confined to its own frame leaves no global behind"
    );
}

#[test]
fn caught_guest_publication_preserves_opaque_message_info_and_explicit_code_bytes() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = vm();
    let original = Completion::new_error_metadata(
        Code::Error,
        Value::from_string_bytes(b"GUEST \xff\0tail".as_slice()),
        Value::list(vec![
            Value::string("-errorinfo"),
            Value::from_string_bytes(b"INFO \xfe\0exact".as_slice()),
            Value::string("-errorcode"),
            Value::from_string_bytes(b"RAW \xfd\0CODE".as_slice()),
        ]),
    );
    vm.publish_caught_error(&original).unwrap();
    assert_eq!(
        vm.read_variable("::errorInfo")
            .unwrap()
            .resident_string_bytes()
            .unwrap()
            .as_ref(),
        b"INFO \xfe\0exact"
    );
    assert_eq!(
        vm.read_variable("::errorCode")
            .unwrap()
            .resident_string_bytes()
            .unwrap()
            .as_ref(),
        b"RAW \xfd\0CODE"
    );
    assert_eq!(
        original.result.resident_string_bytes().unwrap().as_ref(),
        b"GUEST \xff\0tail"
    );
}

#[test]
fn caught_publication_preserves_first_typed_host_cause_before_any_guest_access() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = vm();
    vm.set_var("kept", Value::string("BEFORE")).unwrap();
    let original = tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
        "original caught-value ingress",
    );
    let _ = vm.refuse_tcl_host_failure(tcl_vm::TclHostFailure::ValueAccess(original));
    let error = vm
        .publish_caught_error(&failure("unentered", Value::empty()))
        .unwrap_err();
    assert_eq!(
        error.into_completion().unwrap_err(),
        tcl_vm::TclHostFailure::Execution(
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(original)
        )
    );
    assert_eq!(
        vm.get_var("kept")
            .unwrap()
            .resident_string_bytes()
            .unwrap()
            .as_ref(),
        b"BEFORE"
    );
    assert!(vm.get_var("errorInfo").is_none());
    assert!(vm.get_var("errorCode").is_none());
}

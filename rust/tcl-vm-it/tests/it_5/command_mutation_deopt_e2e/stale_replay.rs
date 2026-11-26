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

//! What replays once a binding is stale: lowering hooks, escaped heads, constant folds, and the structured-command replay cases.

use super::*;

#[test]
fn non_expr_lowering_hook_redispatches_after_replacement() {
    let (mut vm, _output) = vm();
    let compiled = BytecodeCompileService::default()
        .compile("proc p {} {mutate; set x OLD}")
        .expect("module compiles");
    assert!(
        sole_compiled_procedure(&compiled)
            .command_bindings
            .iter()
            .any(|binding| binding.name == "set" && binding.identity == "set"),
        "typed lowering must retain the registry command it specialised"
    );
    eval_ok(&mut vm, "proc p {} {mutate; set x OLD}");
    eval_ok(
        &mut vm,
        "proc mutate {} {rename set real_set; proc set args {return LOWERING_OVERRIDE}}",
    );

    // Tcl 9.0.4: LOWERING_OVERRIDE. Typed lowering is a registry-command
    // assumption just like a codegen hook; an active frame must not execute
    // the stale AssignConst after the nested mutation.
    assert_eq!(eval_ok(&mut vm, "p"), "LOWERING_OVERRIDE");
}

#[test]
fn escaped_head_sees_the_replacement_at_its_start() {
    let (mut vm, _output) = vm();
    let compiled = BytecodeCompileService::default()
        .compile(r"proc p {} {mutate; se\x74 x OLD}")
        .expect("module compiles");
    // Tcl 8.6.18, 9.0.4 and 9.1.0 compile the escaped head as `set`
    // (`tcl::unsupported::disassemble proc p` shows `storeScalar1 %v0`);
    // 8.5.19 invokes it. The default engine is 9.0, so the specialised store
    // rests on the `set` it compiled.
    assert!(
        sole_compiled_procedure(&compiled)
            .command_bindings
            .iter()
            .any(|binding| binding.name == "set" && binding.identity == "set"),
        "the escaped head is compiled as the `set` it spells",
    );
    eval_ok(&mut vm, r"proc p {} {mutate; se\x74 x OLD}");
    eval_ok(
        &mut vm,
        "proc mutate {} {rename set real_set; proc set args {return ESCAPED_OVERRIDE}}",
    );

    // Tcl 8.4.20 to 9.1.0 and Jim 0.84: ESCAPED_OVERRIDE. The replacement is
    // seen at the command's start, as for any compiled `set`.
    assert_eq!(eval_ok(&mut vm, "p"), "ESCAPED_OVERRIDE");
}

#[test]
fn constant_fold_dependency_redispatches_after_replacement() {
    let (mut vm, _output) = vm();
    let compiled = BytecodeCompileService::default()
        .compile("set x [list a b]; set x")
        .expect("module compiles");
    assert!(
        compiled
            .top_level
            .command_bindings
            .iter()
            .any(|binding| binding.name == "list" && binding.identity == "list"),
        "constant-folded commands must carry the same binding dependency as hooks"
    );
    let handle = vm
        .compile_function("set x [list a b]; set x")
        .expect("function compiles");
    assert_eq!(vm.invoke_function(&handle).result.to_str().as_ref(), "a b");
    eval_ok(
        &mut vm,
        "rename list real_list; proc list args {return FOLD_OVERRIDE}",
    );

    let completion = vm.invoke_function(&handle);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "FOLD_OVERRIDE");
}

#[test]
fn active_constant_fold_replays_after_an_earlier_argument_mutates_its_binding() {
    let (mut vm, _output) = vm();

    // Compile the mutator and consumer in separate units. The consumer cannot
    // see through the already-installed `mutate` proc while deciding whether
    // its later constant command substitution is foldable.
    eval_ok(
        &mut vm,
        "proc mutate {} {\
             rename list saved_list; \
             proc list args {return REPLACED}; \
             return X\
         }",
    );
    eval_ok(&mut vm, "proc p {} {concat [mutate] [list a b]}");

    // Tcl 9.0.4: the first substitution replaces `list`; the folded second
    // substitution must redispatch at its own boundary inside the same active
    // outer command.
    assert_eq!(eval_ok(&mut vm, "p"), "X REPLACED");
}

#[test]
fn top_level_fold_replays_after_a_specialised_write_trace_mutates_its_binding() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        r"proc replace_list {name1 name2 op} {
             rename list saved_list
             proc list args {return TRACE_REPLACED}
         }
         set trigger 0
         trace add variable trigger write replace_list",
    );
    let handle = vm
        .compile_function("set a([incr trigger]) [list x y]; set a(1)")
        .expect("function compiles");

    // No generic invoke exists in this top-level unit. The specialised INCR
    // fires a write trace and advances the command epoch before the folded
    // list executes, so its nested replay marker must survive peephole passes.
    let completion = vm.invoke_function(&handle);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "TRACE_REPLACED");
}

#[test]
fn top_level_dict_fold_replay_skips_stale_verification_and_payload() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        r"proc replace_dict {name1 name2 op} {
             rename dict saved_dict
             proc dict args {return NOT_A_DICT}
         }
         set trigger 0
         trace add variable trigger write replace_dict",
    );
    let handle = vm
        .compile_function("set a([incr trigger]) [dict create k v]; set a(1)")
        .expect("function compiles");

    // Replaying the replacement resumes after the entire stale folded range,
    // including VERIFY_DICT; the replacement's non-dict result is legitimate.
    let completion = vm.invoke_function(&handle);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "NOT_A_DICT");
}

#[test]
fn restored_builtin_identity_does_not_leave_the_vm_permanently_untrusted() {
    let (mut vm, _output) = vm();
    let handle = vm
        .compile_function("expr {1+2}")
        .expect("function compiles");
    eval_ok(
        &mut vm,
        "rename expr held_expr; proc expr args {return OVERRIDE}; \
         rename expr {}; rename held_expr expr",
    );
    vm.set_compiler(Box::new(OptimisedOnlyCompilerSvc(
        BytecodeCompileService::default(),
    )));

    // Tcl 9.0.4 returns 3 after the original token is renamed back. The live
    // binding once again satisfies the handle's dependency, so invocation
    // must not demand a plain compiler merely because an epoch changed.
    let completion = vm.invoke_function(&handle);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "3");
}

/// A stale binding replays the next command from source and resumes after
/// it, so every structured command the compiler inlines must open its own
/// boundary: one without lets the resume land inside its body.
#[test]
fn replay_after_a_stale_binding_resumes_after_each_structured_command() {
    let tclsh =
        tcl_test_support::locate_tclsh(TclVersion::V9_0).expect("the Tcl 9.0 oracle is valid");
    for (source, expected) in &stale_replay_cases() {
        let (mut vm, output) = vm();
        eval_ok(&mut vm, source);
        let printed = String::from_utf8(output.0.borrow().clone()).expect("UTF-8 output");
        assert_eq!(printed.trim_end(), *expected, "{source}");
        if let Some(tclsh) = &tclsh {
            let outcome = tcl_test_support::run_script(&tclsh.path, source.as_bytes())
                .expect("Tcl 9.0 oracle runs");
            assert_eq!(
                outcome.strict_text().expect("Tcl 9.0 succeeds"),
                *expected,
                "Tcl 9.0: {source}",
            );
        }
    }
}

/// Scripts and their Tcl 9.0 output for
/// [`replay_after_a_stale_binding_resumes_after_each_structured_command`].
fn stale_replay_cases() -> [(String, &'static str); 17] {
    let stale = "set n 0; proc try args {}; try a b\n";
    [
        (
            format!("{stale}if {{$n}} {{\n    puts WRONG\n}}\nputs done"),
            "done",
        ),
        (
            format!("{stale}if {{$n}} {{puts WRONG}} else {{puts RIGHT}}"),
            "RIGHT",
        ),
        (
            format!("{stale}switch $n {{1 {{puts W}} default {{puts D}}}}"),
            "D",
        ),
        (
            format!("{stale}for {{set i 0}} {{$i < 2}} {{incr i}} {{puts $i; if {{$n}} {{puts W}}}}"),
            "0\n1",
        ),
        (
            format!("{stale}for {{}} {{$n}} {{}} {{puts W}}\nputs done"),
            "done",
        ),
        (
            format!("{stale}while {{$n < 2}} {{incr n; if {{$n == 1}} {{puts one}} else {{puts two}}}}"),
            "one\ntwo",
        ),
        (
            // tclsh reads stdin through `if`, so restore it before the line ends.
            "set n 1; puts pre; rename if _if; proc if args {puts redefined}; \
             if {$n} {puts W}; rename if {}; rename _if if"
                .to_owned(),
            "pre\nredefined",
        ),
        (
            "set n 1; puts pre; rename switch _s; proc switch args {puts redefined}\nswitch $n {1 {puts W}}"
                .to_owned(),
            "pre\nredefined",
        ),
        (
            "puts pre; rename for _f; proc for args {puts redefined}\nfor {set i 0} {$i < 1} {incr i} {puts W}"
                .to_owned(),
            "pre\nredefined",
        ),
        (
            "set n 0; puts pre; rename for _f; proc for args {puts redefined}\nfor {} {$n} {} {puts W}"
                .to_owned(),
            "pre\nredefined",
        ),
        // A constant-false `if` still owns a boundary.
        (
            "proc r {} {rename if _if; proc if args {puts redefined}}; \
             foreach x {a} {r; if {0} {puts W}}; rename if {}; rename _if if"
                .to_owned(),
            "redefined",
        ),
        // Every arm leaves, so only a replay reaches the code after the `if`.
        (
            "for {set i 0} {$i < 2} {incr i} {proc try args {}; if {1} {break}}\n\
             try a b; puts end"
                .to_owned(),
            "end",
        ),
        (
            "proc p {n} {proc try args {}; if {$n} {return x} else {return y}; try a b}; puts [p 1]"
                .to_owned(),
            "x",
        ),
        (
            "proc r {} {rename if _if; proc if args {puts redefined}}; \
             proc p {} {foreach x {1} {r; if {1} {break}; puts after}}; p; \
             rename if {}; rename _if if"
                .to_owned(),
            "redefined\nafter",
        ),
        (
            "proc r {} {rename if _if; proc if args {puts redefined}}; \
             foreach x {1} {r; if {1} {continue}; puts after}; rename if {}; rename _if if"
                .to_owned(),
            "redefined\nafter",
        ),
        (
            "proc r {} {rename if _if; proc if args {puts redefined}}; \
             proc p {n} {foreach x {1} {r; if {$n} {break} else {continue}; puts after}}; p 0; \
             rename if {}; rename _if if"
                .to_owned(),
            "redefined\nafter",
        ),
        (
            "proc r {} {rename switch _s; proc switch args {puts redefined}}; \
             proc p {n} {foreach x {1} {r; switch $n {0 {break} default {continue}}; puts after}}; \
             p 0; rename switch {}; rename _s switch"
                .to_owned(),
            "redefined\nafter",
        ),
    ]
}

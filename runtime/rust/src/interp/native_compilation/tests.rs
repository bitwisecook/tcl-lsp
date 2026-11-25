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

use super::*;
use crate::interp::new_string;

#[test]
fn host_refusal_preserves_an_unmaterialized_original_jim_namespace() {
    // Software refusal control: no original Jim command or frame parity claim.
    let mut interp = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    let context = interp.native_jim_object_context().unwrap();
    let original = obj::Owned::fresh(obj::new_wide_int_obj(42));
    let pointer = original.as_ptr();
    crate::native_source::bind_context(pointer, &context).unwrap();
    let before = obj::native_object_snapshot(pointer).unwrap();
    let original_type = obj::obj_type_ptr(pointer);
    assert!(before.resident.is_none());
    let namespace = interp
        .namespaces_mut()
        .retain_unmaterialized_jim_namespace(original);
    let caller = interp.current_ns();
    interp.enter_namespace_activation(namespace);
    interp.frames.borrow_mut().push(namespace);
    interp.set_current_ns(namespace);
    let references = unsafe { (*pointer).ref_count };
    assert_eq!(
        interp.refuse_host_command("first host boundary"),
        Code::Error
    );
    let first = interp.native_host_command_refusal().unwrap();
    assert_eq!(first.reason, "first host boundary");
    assert_eq!(first.namespace.as_ref(), "<unavailable namespace>");
    assert_eq!(first.namespace_token, Some(namespace as u64));
    assert_eq!(first.interpreter, interp.native_command_interpreter);
    assert_eq!(first.frame, interp.frames.borrow().current_level());
    assert!(interp.namespaces().native_context_path(namespace).is_none());
    assert!(interp.namespaces().jim_namespace_bytes(namespace).is_none());
    let after = obj::native_object_snapshot(pointer).unwrap();
    assert!(after.resident.is_none());
    assert_eq!(obj::obj_type_ptr(pointer), original_type);
    assert_eq!(unsafe { (*pointer).ref_count }, references);
    assert_eq!(interp.refuse_host_command("later cleanup"), Code::Error);
    assert_eq!(interp.native_host_command_refusal().unwrap(), first);
    let popped = interp.pop_native_call_frame();
    interp.set_current_ns(caller);
    interp.leave_namespace_activation(popped);
}

#[cfg(have_tommath)]
#[test]
fn safe_index_missing_fixed_table_stays_an_outer_refusal() {
    let mut interp = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jim"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert!(interp.native_math_function_table().is_some());
    interp.native_compilation.borrow_mut().fixed_math = None;
    let error = tcl_cmd_core::index::resolve_for_ops(&mut interp, "0 && sqrt(1)", 2).unwrap_err();
    assert!(error.native_access_refusal().is_some());
    assert_eq!(interp.report_cmd_error(error), Code::Error);
    assert!(interp.native_access_refusal().is_some());

    let mut caught = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jim"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    caught.native_compilation.borrow_mut().fixed_math = None;
    assert_eq!(
        caught.eval_str(b"catch {lindex {A B} {0 && sqrt(1)}} captured"),
        Code::Error,
    );
    assert!(caught.native_access_refusal().is_some());
}

fn interpreter(dialect: &str) -> Interp {
    let profile = tcl_registry::model::resolve_environment(dialect).unit_profile();
    let mut interpreter = Interp::new();
    interpreter.set_dialect_profile(profile);
    interpreter
}

#[test]
fn variable_output_addresses_follow_the_selected_native_write_order() {
    for reference in tcl_test_support::available_tclshs() {
        for source in
            tcl_test_support::variable_outputs::variable_output_lookup_scripts(reference.version)
        {
            let oracle = tcl_test_support::run_script(
                &reference.path,
                format!("puts [{source}]\n").as_bytes(),
            )
            .unwrap();
            assert!(oracle.success(), "{source}");
            assert!(oracle.stderr.is_empty(), "{source}: {:?}", oracle.stderr);
            let mut interpreter = interpreter(reference.version.dialect_profile_name());
            assert_eq!(
                interpreter.eval_str(source.as_bytes()),
                Code::Ok,
                "{source}"
            );
            let mut actual = interpreter.result_bytes();
            actual.push(b'\n');
            assert_eq!(actual, oracle.stdout, "{}: {source}", reference.patchlevel);
        }
    }
}

#[test]
fn compiled_native_selection_precedes_argv_in_procedures_but_direct_eval_remains_late() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut interpreter = interpreter(dialect);
        let script = b"proc p {} {set x [proc set args {return CUSTOM}]; info exists x}; p";
        assert_eq!(interpreter.eval_str(script), Code::Ok, "{dialect}");
        assert_eq!(
            interpreter.result_bytes(),
            if dialect == "jim" { b"0" } else { b"1" },
            "{dialect}"
        );
        let mut interpreter = self::interpreter(dialect);
        assert_eq!(
            interpreter.eval_str(b"eval {set x [proc set args {return CUSTOM}]}; info exists x"),
            Code::Ok
        );
        let direct = matches!(dialect, "tcl8.4" | "tcl8.5" | "jim");
        assert_eq!(
            interpreter.result_bytes(),
            if direct { b"0" } else { b"1" },
            "{dialect}"
        );
    }
}

#[test]
fn legacy_compilation_rejects_the_entire_chunk_before_arity_and_prunes_only_literal_booleans() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for (body, legacy_rejects) in [
            ("set ::before 1; set x extra bad", true),
            ("set ::before 1; if {0} {set x extra bad}", false),
            ("set ::before 1; if {0+0} {set x extra bad}", true),
            ("set ::before 1; while {0} {set x extra bad}", false),
            ("set ::before 1; for {} {0} {} {set x extra bad}", true),
            ("set ::before 1; foreach x {} {set x extra bad}", true),
            ("set ::before 1; catch {set x extra bad}", false),
        ] {
            let mut interpreter = interpreter(dialect);
            let source = format!(
                    "set before 0; proc p {{required}} {{{body}}}; catch {{p}} message; list $before $message"
                );
            assert_eq!(
                interpreter.eval_str(source.as_bytes()),
                Code::Ok,
                "{dialect}: {body}"
            );
            let result = interpreter.result_bytes();
            let result = String::from_utf8(result).unwrap();
            assert!(result.starts_with("0 "), "{dialect}: {body}: {result}");
            let usage = if dialect == "tcl8.4" && legacy_rejects {
                "set varName"
            } else {
                "p required"
            };
            assert!(result.contains(usage), "{dialect}: {body}: {result}");
        }
    }
}

#[test]
fn native_runtime_matches_all_six_compiler_traversal_oracles() {
    fn capture(interpreter: &mut Interp, arguments: &[*mut TclObj]) -> Code {
        let mut output = interpreter
            .var_get(b"::observations")
            .map_or_else(Vec::new, obj_bytes);
        output.extend_from_slice(&obj_bytes(arguments[1]));
        output.push(b'\n');
        let value = new_string(&output);
        if interpreter.var_set(b"::observations", value).is_err() {
            drop_fresh(value);
            return Code::Error;
        }
        interpreter.set_result_bytes(b"");
        Code::Ok
    }
    let fixture = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../rust/tcl-syntax/tests/data/body_execution/native-compiler-traversal.tcl"
    ));
    for (dialect, expected) in [
            (
                "tcl8.4",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../rust/tcl-syntax/tests/data/body_execution/native-compiler-traversal-8.4.txt"
                )),
            ),
            (
                "tcl8.5",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../rust/tcl-syntax/tests/data/body_execution/native-compiler-traversal-8.5.txt"
                )),
            ),
            (
                "tcl8.6",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../rust/tcl-syntax/tests/data/body_execution/native-compiler-traversal-8.6.txt"
                )),
            ),
            (
                "tcl9.0",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../rust/tcl-syntax/tests/data/body_execution/native-compiler-traversal-9.0.txt"
                )),
            ),
            (
                "tcl9.1",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../rust/tcl-syntax/tests/data/body_execution/native-compiler-traversal-9.1.txt"
                )),
            ),
            (
                "jim",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../rust/tcl-syntax/tests/data/body_execution/native-compiler-traversal-jim.txt"
                )),
            ),
        ] {
            let mut interpreter = interpreter(dialect);
            interpreter.register_builtin(b"puts", capture);
            assert_eq!(
                interpreter.eval_str(fixture.as_bytes()),
                Code::Ok,
                "{dialect}: {:?}",
                interpreter.result_bytes()
            );
            let observed = interpreter
                .var_get(b"::observations")
                .map(obj_bytes)
                .unwrap();
            assert_eq!(String::from_utf8(observed).unwrap(), expected, "{dialect}");
        }
}

#[test]
fn legacy_compile_failure_precedes_procedure_effects() {
    // tclsh8.4 compiles the body first: `**` is not an 8.4 operator.
    let mut interpreter = interpreter("tcl8.4");
    assert_eq!(
            interpreter.eval_str(
                b"set before 0; proc p {} {set ::before 1; expr {1 ** 2}}; list [catch p msg] $msg $before"
            ),
            Code::Ok,
        );
    assert_eq!(
        interpreter.result_bytes(),
        b"1 {syntax error in expression \"1 ** 2\": unexpected operator *} 0"
    );
    assert_eq!(interpreter.native_compilation_admission_error(), None);
}

#[test]
fn legacy_error_contexts_preserve_inline_annotations_and_source_word_delimiters() {
    let mut interpreter = interpreter("tcl8.4");
    assert_eq!(
        interpreter.eval_str(
            b"proc p {} {\nset before 1\nif 1 {\nset x extra bad\n}\n}; catch p; set ::errorInfo"
        ),
        Code::Ok
    );
    assert_eq!(interpreter.result_bytes(), b"wrong # args: should be \"set varName ?newValue?\"\n    while compiling\n\"set x extra bad\"\n    (\"if\" then script line 2)\n    while compiling\n\"if 1 {\nset x extra bad\n}\"\n    (compiling body of proc \"p\", line 3)\n    invoked from within\n\"p\"");
    for (body, expected) in [
        (
            "if {1}",
            "wrong # args: no script following \"{1}\" argument",
        ),
        (
            "if \"1\"",
            "wrong # args: no script following \"\"1\"\" argument",
        ),
        ("{if}", "wrong # args: no expression after \"if\" argument"),
    ] {
        let source = format!("proc p {{}} {{{body}}}; catch p message; set message");
        assert_eq!(interpreter.eval_str(source.as_bytes()), Code::Ok);
        assert_eq!(interpreter.result_bytes(), expected.as_bytes(), "{body}");
    }
}

#[test]
fn selected_error_preserves_native_handler_across_argument_mutation() {
    for (dialect, expected) in [
        ("tcl8.4", b"0 CUSTOM".as_slice()),
        ("tcl8.5", b"0 CUSTOM".as_slice()),
        ("tcl8.6", b"1 MESSAGE".as_slice()),
        ("tcl9.0", b"1 MESSAGE".as_slice()),
        ("tcl9.1", b"1 MESSAGE".as_slice()),
        ("jim", b"0 CUSTOM".as_slice()),
    ] {
        for body in [
            "error [rename error native_error;proc error args {return CUSTOM};list MESSAGE]",
            "error MESSAGE [rename error native_error;proc error args {return CUSTOM};list INFO]",
        ] {
            let mut interpreter = interpreter(dialect);
            let source =
                format!("proc p {{}} {{{body}}};set code [catch p result];list $code $result");
            assert_eq!(
                interpreter.eval_str(source.as_bytes()),
                Code::Ok,
                "{dialect}: {body}"
            );
            assert_eq!(interpreter.result_bytes(), expected, "{dialect}: {body}");
        }
    }
}

#[test]
fn execution_traces_decline_native_selection_and_custom_host_handlers_never_gain_stock_proof() {
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.1"] {
        let mut interpreter = interpreter(dialect);
        assert_eq!(interpreter.eval_str(b"proc observe args {}; trace add execution set enter observe; proc p {} {set x [proc set args {return CUSTOM}]; info exists x}; p"), Code::Ok);
        assert_eq!(interpreter.result_bytes(), b"0", "{dialect}");
    }
    fn custom(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
        interpreter.set_result_bytes(b"HOST");
        Code::Ok
    }
    let mut interpreter = interpreter("tcl8.4");
    interpreter.register_builtin(b"set", custom);
    assert_eq!(
        interpreter.eval_str(b"proc p {} {set x extra bad}; p"),
        Code::Ok
    );
    assert_eq!(interpreter.result_bytes(), b"HOST");
}

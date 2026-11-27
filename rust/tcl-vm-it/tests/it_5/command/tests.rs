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

//! Core command behaviour: `time`, `proc`, `expr`, `eval` and Jim's `catch`, through scripts and object-word calls.

use crate::prelude::*;

#[test]
fn time_keeps_original_counted_script_and_skips_zero_count_materialization() {
    // Source proof: naming.time.original-script-object-evaluation
    // docs/design/analysis/name-resolution-proofs/time-original-script-object-evaluation.md
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = tcl_vm::test_support::profile(engine);
        let mut vm = crate::fixture::interpreter(profile);
        let invalid = Value::new_native_string_bytes(b"invalid script \xff".as_slice());
        let skipped = crate::fixture::call(&mut vm, "time", &[invalid, Value::int(0)]);
        assert_eq!(skipped.code, tcl_runtime_api::Code::Ok, "{engine}");
        assert_eq!(
            skipped.result.string_bytes().as_ref(),
            b"0 microseconds per iteration"
        );
        let body = Value::new_native_string_bytes(b"set {time\xff} OK".as_slice());
        let completion = crate::fixture::call(&mut vm, "time", &[body, Value::int(2)]);
        assert_eq!(
            completion.code,
            tcl_runtime_api::Code::Ok,
            "{engine}: {:?}",
            completion.result.string_bytes()
        );
        assert_eq!(
            vm.get_var_bytes(b"time\xff")
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"OK",
            "{engine}"
        );
    }
}

/// Invoke `name` with `arguments` as a host does; a name that is not UTF-8
/// is invoked through `eval` of the whole command as a list.
fn invoke(vm: &mut Vm, name: &Value, arguments: Vec<Value>) -> Completion<Value> {
    let bytes = crate::fixture::bytes(vm, name);
    if let Ok(name) = std::str::from_utf8(&bytes) {
        return crate::fixture::call(vm, name, &arguments);
    }
    let mut command = vec![name.clone()];
    command.extend(arguments);
    crate::fixture::call(vm, "eval", &[Value::list(command)])
}

#[test]
fn c84_procedure_parse_failures_match_7_native_original_object_completions() {
    // Native proof: naming.procedure-error.original-name-context
    // docs/design/analysis/name-resolution-proofs/procedure-error.original-name-context.md
    // Native proof: naming.procedure-error.counted-zero-name-context
    // docs/design/analysis/name-resolution-proofs/procedure-error.counted-zero-name-context.md
    // Native proof: naming.procedure-error.long-name-and-argv-context
    // docs/design/analysis/name-resolution-proofs/procedure-error.long-name-and-argv-context.md
    // Native proof: naming.procedure-error.body-line-context
    // docs/design/analysis/name-resolution-proofs/procedure-error.body-line-context.md
    let decode = crate::fixture::unhex;
    let profile = tcl_vm::test_support::profile("tcl8.4");
    let mut compared = 0;
    for row in
        include_str!("../../../../tcl-vm/tests/data/native_procedure_parse_context/8.4.20.tsv")
            .lines()
    {
        let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
        assert_eq!(fields.len(), 8);
        let mut vm = crate::fixture::interpreter(profile);
        let name = Value::new_native_string_bytes(decode(fields[3]));
        let definition = crate::fixture::call(
            &mut vm,
            "proc",
            &[
                name.clone(),
                Value::new_native_string_bytes(&b"x"[..]),
                Value::new_native_string_bytes(decode(fields[4])),
            ],
        );
        assert_eq!(definition.code.as_int().to_string(), fields[1], "{row}");
        let arguments = if fields[0] == "5" {
            vec![Value::new_native_string_bytes(&b"OK"[..])]
        } else {
            Vec::new()
        };
        let completion = invoke(&mut vm, &name, arguments);
        assert_eq!(completion.code.as_int().to_string(), fields[2], "{row}");
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            decode(fields[5]),
            "{row}"
        );
        let info = crate::fixture::call(&mut vm, "set", &[Value::string("::errorInfo")]).result;
        // tclsh 8.4.20 keeps `invoked from within "p"` above an `eval`
        // frame; the VM omits it, so a name invoked through `eval` checks the
        // frames both report.
        let mut expected = decode(fields[6]);
        if std::str::from_utf8(&decode(fields[3])).is_err() {
            let frame = b"\n    invoked from within\n";
            let last = expected
                .windows(frame.len())
                .rposition(|part| part == frame)
                .unwrap();
            expected.truncate(last);
        }
        assert!(
            info.string_bytes().starts_with(&expected),
            "{row}: {}",
            String::from_utf8_lossy(&info.string_bytes())
        );
        assert_eq!(
            crate::fixture::option(&completion.options, "-errorcode")
                .unwrap()
                .string_bytes()
                .as_ref(),
            decode(fields[7]),
            "{row}"
        );
        assert!(
            vm.get_var_bytes(b"side").is_none(),
            "C84 must reject before the valid prefix: {row}"
        );
        compared += 1;
    }
    assert_eq!(compared, 7);
}

#[test]
fn procedure_definition_and_activation_match_42_native_observations() {
    // Native proof: naming.procedure.original-malformed-body-arity-order
    // docs/design/analysis/name-resolution-proofs/procedure-original-malformed-body-arity-order.md
    // Native proof: naming.procedure.original-wrong-arity-body-withdrawal
    // docs/design/analysis/name-resolution-proofs/procedure-original-wrong-arity-body-withdrawal.md
    let engines = [
        (
            "tcl8.4",
            include_str!("../../../../tcl-vm/tests/data/native_procedure_activation/8.4.txt"),
        ),
        (
            "tcl8.5",
            include_str!("../../../../tcl-vm/tests/data/native_procedure_activation/8.5.txt"),
        ),
        (
            "tcl8.6",
            include_str!("../../../../tcl-vm/tests/data/native_procedure_activation/8.6.txt"),
        ),
        (
            "tcl9.0",
            include_str!("../../../../tcl-vm/tests/data/native_procedure_activation/9.0.txt"),
        ),
        (
            "tcl9.1",
            include_str!("../../../../tcl-vm/tests/data/native_procedure_activation/9.1.txt"),
        ),
        (
            "jim",
            include_str!("../../../../tcl-vm/tests/data/native_procedure_activation/Jim.txt"),
        ),
    ];
    let mut compared = 0;
    for (engine, fixture) in engines {
        let profile = tcl_vm::test_support::profile(engine);
        let mut vm = crate::fixture::interpreter(profile);
        let protocol = crate::fixture::strings(&vm);
        let rows = fixture
            .lines()
            .map(|line| {
                tcl_syntax::list::split_native_list_bytes(line.as_bytes(), protocol)
                    .unwrap()
                    .into_iter()
                    .map(std::borrow::Cow::into_owned)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 7);
        // The interpreter result is reset by the next command, so each
        // completion is compared as the call reports it.
        let mut rows = rows.iter();
        let mut check = |vm: &mut Vm, completion: Completion<Value>| {
            let row = rows.next().unwrap();
            assert_eq!(
                completion.code.as_int().to_string().as_bytes(),
                row[1],
                "{engine}/{row:?}"
            );
            assert_eq!(
                crate::fixture::bytes(vm, &completion.result),
                row[2],
                "{engine}/{row:?}"
            );
            compared += 1;
        };
        for body in ["set value \"", "error BODY"] {
            let definition = crate::fixture::call(
                &mut vm,
                "proc",
                &[Value::string("p"), Value::string("x"), Value::string(body)],
            );
            check(&mut vm, definition);
            let missing = crate::fixture::call(&mut vm, "p", &[]);
            check(&mut vm, missing);
            let supplied = crate::fixture::call(&mut vm, "p", &[Value::string("OK")]);
            check(&mut vm, supplied);
        }
        let definition = crate::fixture::call(
            &mut vm,
            "proc",
            &[
                Value::string("q"),
                Value::string("x"),
                Value::string("set side BODY;return OK"),
            ],
        );
        assert_eq!(definition.code, tcl_runtime_api::Code::Ok);
        vm.set_var_bytes(b"side", Value::string("BEFORE")).unwrap();
        let arity = crate::fixture::call(&mut vm, "q", &[]);
        check(&mut vm, arity);
        assert_eq!(
            vm.get_var_bytes(b"side").unwrap().string_bytes().as_ref(),
            b"BEFORE"
        );
    }
    assert_eq!(compared, 42);
}

#[test]
fn expression_source_objects_preserve_native_shared_representation_timing() {
    let cases = [
        ("set x [list 3]; expr $x; concat $x [list]", "3", "3 "),
        (
            "set x [list nope]; catch {expr $x}; concat $x [list]",
            "nope",
            "nope ",
        ),
        (
            "set x [list 1/0]; catch {expr $x}; concat $x [list]",
            "1/0",
            "1/0 ",
        ),
        (
            "set x [list 3]; set y $x; expr $y; concat $x [list]",
            "3",
            "3 ",
        ),
        (
            "set x [list 3]; expr {0 ? $x+0 : 1}; concat $x [list]",
            "3",
            "3",
        ),
        ("set x [list 3]; expr {$x+0}; concat $x [list]", "3", "3 "),
    ];
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = tcl_vm::test_support::profile(dialect);
        for (source, c_result, jim_result) in cases {
            let mut vm = crate::fixture::interpreter(profile);
            let completion = vm
                .try_eval_source(source)
                .unwrap_or_else(|error| panic!("{dialect}: {source}: {error:?}"));
            assert_eq!(
                completion.code,
                tcl_runtime_api::Code::Ok,
                "{dialect}: {source}"
            );
            let expected = if dialect == "jim" {
                jim_result
            } else {
                c_result
            };
            assert_eq!(
                &*completion.result.to_str(),
                expected,
                "{dialect}: {source}"
            );
        }
    }
}

#[test]
fn expression_argv_validation_precedes_shared_source_coercion() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = tcl_vm::test_support::profile(dialect);
        let mut vm = crate::fixture::interpreter(profile);
        let result = vm.try_eval_source(
                    "set x [list 3]; set code [catch {expr $x + 1} result]; list $code $result [concat $x [list]]",
                )
                .unwrap_or_else(|error| panic!("{dialect}: expression argv: {error:?}"));
        assert_eq!(result.code, tcl_runtime_api::Code::Ok);
        let expected = if dialect == "jim" {
            "1 {wrong # args: should be \"expr expression\"} 3"
        } else {
            "0 4 3"
        };
        assert_eq!(&*result.result.to_str(), expected, "{dialect}");
    }
}

fn vm_with_supported_source_and_unsupported_expression_window()
-> (Vm, &'static tcl_dialect::DialectProfile) {
    let profile = tcl_vm::test_support::profile("tcl9.0");
    (crate::fixture::interpreter(profile), profile)
}

fn unsupported_expression_source() -> String {
    format!("{}1{}", "(".repeat(9000), ")".repeat(9000))
}

#[test]
fn unsupported_expression_is_neutral_and_preserves_only_prior_effects() {
    use tcl_runtime_api::{NativeExecutionError, NativeExpressionRefusal};
    let cases = [
        "catch {expr $source; set ::late 1} caught options; set ::handler 1",
        "try {expr $source; set ::late 1} on error {} {set ::handler 1} finally {set ::cleanup 1}",
        "set ::target [expr $source]; set ::late 1",
        "proc p {} {set local 1; trace add variable local unset observe; expr $::source; set ::late 1}; p",
    ];
    for body in cases {
        let (mut vm, profile) = vm_with_supported_source_and_unsupported_expression_window();
        let unsupported = unsupported_expression_source();
        let setup = format!(
            "set early 0; set late 0; set handler 0; set cleanup 0; set observer 0; set caught ORIGINAL; set options ORIGINAL; set target ORIGINAL; set errorInfo ORIGINAL; set errorCode ORIGINAL; set source {{{unsupported}}}; proc observe args {{set ::observer 1}}; trace add execution expr leave observe"
        );
        let result = vm.try_eval_source(&format!("{setup}; set ::early 1; {body}"));
        let refusal = result.unwrap_err();
        let NativeExecutionError::ExpressionRefusal(failure) = refusal else {
            panic!("{body}: expected expression refusal, got {refusal:?}");
        };
        assert_eq!(failure.reason, NativeExpressionRefusal::UnsupportedGrammar);
        assert_eq!(failure.source.bytes(), unsupported.as_bytes());
        assert_eq!(failure.source_profile, profile.cache_key());
        assert_eq!(failure.native_profile, profile.cache_key());
        assert_eq!(failure.interpreter.interpreter, 0);
        assert_eq!(failure.frame, usize::from(body.starts_with("proc p")));
        assert_eq!(failure.namespace.as_ref(), "");
        assert_eq!(failure.namespace_token, Some(0));
        for (name, expected) in [
            ("early", "1"),
            ("late", "0"),
            ("handler", "0"),
            ("cleanup", "0"),
            ("observer", "0"),
            ("caught", "ORIGINAL"),
            ("options", "ORIGINAL"),
            ("target", "ORIGINAL"),
            ("errorInfo", "ORIGINAL"),
            ("errorCode", "ORIGINAL"),
        ] {
            assert_eq!(
                vm.get_var(name).unwrap().to_str().as_ref(),
                expected,
                "{body}: {name}"
            );
        }
        assert_eq!(
            crate::fixture::run(&mut vm, "info", &["level"]).1,
            b"0",
            "host refusal retires proc storage"
        );
        assert_eq!(
            vm.try_eval_source("set resumed YES")
                .unwrap()
                .result
                .to_str()
                .as_ref(),
            "YES",
            "a new independent host entry can run after receiving the typed refusal"
        );
    }
}

#[test]
fn unsupported_expression_refusal_survives_child_interpreter_switches() {
    use tcl_runtime_api::{NativeExecutionError, NativeExpressionRefusal};
    let (mut vm, _) = vm_with_supported_source_and_unsupported_expression_window();
    let unsupported = unsupported_expression_source();
    let result = vm.try_eval_source(&format!(
            "set parentLate 0; set child [interp create]; interp eval $child {{set early 1; set late 0; set source {{{unsupported}}}; catch {{expr $source}} result; set late 1}}; set parentLate 1",
        ));
    let error = result.unwrap_err();
    let NativeExecutionError::ExpressionRefusal(failure) = error else {
        panic!("child expression needs a provider: {error:?}");
    };
    assert_eq!(failure.reason, NativeExpressionRefusal::UnsupportedGrammar);
    assert_eq!(failure.source.bytes(), unsupported.as_bytes());
    assert_ne!(failure.interpreter.interpreter, 0);
    assert_eq!(failure.frame, 0);
    assert_eq!(vm.get_var("parentLate").unwrap().to_str().as_ref(), "0");
    let resumed = vm
        .try_eval_source("interp eval $child {list $early $late [info exists result]}")
        .unwrap();
    assert_eq!(resumed.result.to_str().as_ref(), "1 0 0");
}

#[test]
fn original_absent_f5_expression_and_eval_providers_do_not_borrow_native_core_receipts() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let source = tcl_dialect::DialectProfile::irules();
    let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("tcl9.0"));
    vm.set_dialect_profile(source);
    assert_eq!(vm.dialect_profile().cache_key(), source.cache_key());
    assert!(vm.try_eval_source("set original_f5_unentered 1").is_err());
    assert!(vm.get_var("original_f5_unentered").is_none());
}

#[test]
fn jim_static_storage_matches_the_fixed_native_fixture() {
    let profile = tcl_vm::test_support::profile("jim");
    let source =
        include_str!("../../../../tcl-syntax/tests/data/body_execution/native-static-storage.tcl");
    let expected = include_str!(
        "../../../../tcl-syntax/tests/data/body_execution/native-static-storage-jim.txt"
    );
    for (line, expected) in source.lines().skip(1).zip(expected.lines()) {
        let words = tcl_syntax::list::split_list_jim(line);
        let wanted = tcl_syntax::list::split_list_jim(expected);
        assert_eq!(words.len(), 3);
        assert_eq!(wanted.len(), 3);
        assert_eq!(words[1], wanted[0]);
        let mut vm = crate::fixture::distribution(profile);
        let completion = crate::fixture::eval(&mut vm, &words[2]);
        assert_eq!(
            completion.code.as_int().to_string(),
            &*wanted[1],
            "{}",
            words[1]
        );
        assert_eq!(&*completion.result.to_str(), &*wanted[2], "{}", words[1]);
    }
}

#[test]
fn jim_captured_aliases_follow_the_selected_logical_level() {
    let cases = [
        (
            "proc maker {} {set a KEEP; upvar 0 a x; proc p {} {&x} {set x}}; maker; list [catch p r] $r",
            "1 {can't read \"x\": no such variable}",
        ),
        (
            "proc maker {} {set a KEEP; upvar 0 a x; proc p {} {&x} {set x}}; maker; proc unrelated {} {set a OTHER; list [catch p r] $r}; unrelated",
            "0 OTHER",
        ),
        (
            "proc maker {} {set a OLD; upvar 0 a x; proc p {} {&x} {set x MODIFIED}; proc reader {} {&x} {set x}}; maker; list [p] [catch reader r] $r",
            "MODIFIED 1 {can't read \"x\": no such variable}",
        ),
    ];
    let profile = tcl_vm::test_support::profile("jim");
    for (source, wanted) in cases {
        let mut vm = crate::fixture::distribution(profile);
        let completion = crate::fixture::eval(&mut vm, source);
        assert_eq!(
            completion.code,
            tcl_runtime_api::Code::Ok,
            "{}",
            completion.result.to_str()
        );
        assert_eq!(completion.result.to_str().as_ref(), wanted, "{source}");
    }
}

#[test]
fn procedure_definition_result_follows_the_native_protocol() {
    let mut profiles: Vec<_> = tcl_dialect::TclVersion::ALL
        .iter()
        .map(|version| tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap())
        .collect();
    profiles.push(tcl_vm::test_support::profile("jim"));
    for profile in profiles {
        let mut vm = crate::fixture::interpreter(profile);
        let completion = crate::fixture::eval(&mut vm, "proc written {} {}");
        assert_eq!(completion.code, tcl_runtime_api::Code::Ok);
        let wanted = match profile.parameter_grammar().unwrap() {
            tcl_dialect::ParameterGrammar::Tcl => "",
            tcl_dialect::ParameterGrammar::Jim => "written",
        };
        assert_eq!(&*completion.result.to_str(), wanted, "{}", profile.name);
    }
}

#[test]
fn formal_activation_obeys_the_selected_native_plan() {
    let cases = [
        ("proc p {x x} {set x}; p FIRST SECOND", "FIRST", "SECOND"),
        (
            "proc p {args x} {list $args $x}; set c [catch {p A B C} r]; list $c $r",
            "1 {wrong # args: should be \"p args x\"}",
            "0 {{A B} C}",
        ),
        (
            "proc p {{x DEF} y} {list $x $y}; set c [catch {p VAL} r]; list $c $r",
            "1 {wrong # args: should be \"p ?x? y\"}",
            "0 {DEF VAL}",
        ),
        (
            "set v OLD; proc p {&x} {set x NEW}; p v; set v",
            "OLD",
            "NEW",
        ),
        ("proc p {{&x DEFAULT}} {set {&x}}; p", "DEFAULT", "DEFAULT"),
        (
            "proc p {&x} {}; set c [catch {p MISSING} r]; list $c $r",
            "0 {}",
            "0 {}",
        ),
        (
            "set c [catch {proc p {a(k)} {set a(k)}} r]; if {$c} {set c} else {list $c [p ARG]}",
            "1",
            "0 ARG",
        ),
        (
            "namespace eval param {}; set c [catch {proc p {::param::x} {set ::param::x}} r]; if {$c} {set c} else {list $c [p ARG]}",
            "1",
            "0 ARG",
        ),
        (
            "proc p {{args rest} x} {list $rest $x}; set c [catch {p A B C} r]; list $c $r",
            "1 {wrong # args: should be \"p ?args? x\"}",
            "0 {{A B} C}",
        ),
    ];
    let mut profiles: Vec<_> = tcl_dialect::TclVersion::ALL
        .iter()
        .map(|version| tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap())
        .collect();
    profiles.push(tcl_vm::test_support::profile("jim"));
    for profile in profiles {
        for (source, tcl, jim) in cases {
            let mut vm = crate::fixture::interpreter(profile);
            let result = crate::fixture::eval(&mut vm, source);
            assert_eq!(
                result.code,
                tcl_runtime_api::Code::Ok,
                "{}: {source}: {}",
                profile.name,
                result.result.to_str()
            );
            let expected =
                if profile.parameter_grammar() == Some(tcl_dialect::ParameterGrammar::Jim) {
                    jim
                } else {
                    tcl
                };
            assert_eq!(
                &*result.result.to_str(),
                expected,
                "{}: {source}",
                profile.name
            );
        }
    }
}

#[test]
fn original_list_eval_retains_member_identity_without_materialising_source() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = tcl_vm::test_support::profile(dialect);
        let mut vm = if dialect == "jim" {
            crate::fixture::distribution(profile)
        } else {
            crate::fixture::interpreter(profile)
        };
        let result = Value::new_native_string_bytes(&b"\xff\0TAIL"[..]);
        let script = Value::list(vec![Value::string("return"), result]);
        let completion = crate::fixture::call(&mut vm, "eval", &[script]);
        assert_eq!(completion.code, Code::Return, "{dialect}");
        assert_eq!(completion.result.string_bytes().as_ref(), b"\xff\0TAIL");
    }
}

#[test]
fn jim_native_catch_preserves_its_flag_and_exit_protocol() {
    let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile("jim"));
    let words = |args: &[&str]| {
        args.iter()
            .map(|word| Value::string(*word))
            .collect::<Vec<_>>()
    };
    crate::fixture::call(&mut vm, "set", &words(&["r", "ORIGINAL"]));
    crate::fixture::call(&mut vm, "set", &words(&["o", "OPTIONS"]));
    let completion = crate::fixture::call(
        &mut vm,
        "catch",
        &words(&["-nook", "set x VALUE", "r", "o"]),
    );
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(&*completion.result.to_str(), "VALUE");
    assert_eq!(
        &*crate::fixture::call(&mut vm, "set", &words(&["r"]))
            .result
            .to_str(),
        "ORIGINAL"
    );
    let completion =
        crate::fixture::call(&mut vm, "catch", &words(&["-ok", "set x VALUE", "r", "o"]));
    assert_eq!(&*completion.result.to_str(), "0");
    assert_eq!(
        &*crate::fixture::call(&mut vm, "set", &words(&["r"]))
            .result
            .to_str(),
        "VALUE"
    );
    let completion =
        crate::fixture::call(&mut vm, "catch", &words(&["-exit", "exit 12", "r", "o"]));
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(&*completion.result.to_str(), "6");
    assert_eq!(
        &*crate::fixture::call(&mut vm, "set", &words(&["r"]))
            .result
            .to_str(),
        "12"
    );
    assert!(vm.take_exit().is_none());
    let completion = crate::fixture::call(
        &mut vm,
        "catch",
        &words(&["-noerror", "error BAD", "r", "o"]),
    );
    assert_eq!(completion.code, Code::Error);
    assert_eq!(&*completion.result.to_str(), "BAD");
    let completion = crate::fixture::call(&mut vm, "catch", &words(&["exit 12", "r", "o"]));
    assert_eq!(completion.code, Code::from_int(6));
    assert_eq!(vm.take_exit(), Some(12));
}

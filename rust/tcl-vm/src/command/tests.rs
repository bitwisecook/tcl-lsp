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

use super::parse_params;
use tcl_syntax::value::ValueOps;

/// `incr` creates a missing variable from 8.5; 8.4 reads it first and
/// raises "no such variable".
#[test]
fn incr_of_a_missing_variable_follows_the_selected_release() {
    for version in tcl_dialect::TclVersion::ALL {
        let profile =
            tcl_registry::model::ingress::resolve_environment(version.dialect_profile_name())
                .unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        let completion = vm
            .try_invoke_command("incr", &[super::Value::string("n")])
            .expect("host execution");
        if version == tcl_dialect::TclVersion::V8_4 {
            assert_eq!(completion.code, super::Code::Error, "{version:?}");
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                b"can't read \"n\": no such variable",
                "{version:?}"
            );
        } else {
            assert_eq!(completion.code, super::Code::Ok, "{version:?}");
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                b"1",
                "{version:?}"
            );
        }
    }
}

#[test]
fn hidden_options_follow_original_release_selected_index_tables() {
    for version in tcl_dialect::TclVersion::ALL {
        let profile =
            tcl_registry::model::ingress::resolve_environment(version.dialect_profile_name())
                .unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        let words = ["-namespace", "chosen", "hiddenSet"]
            .map(|word| super::Value::new_native_string_bytes(word.as_bytes()));
        let selected = super::skip_hidden_options(&mut vm, &words);
        if version == tcl_dialect::TclVersion::V8_4 {
            let failure = selected.unwrap_err();
            assert_eq!(failure.code, super::Code::Error);
            assert_eq!(
                failure.result.string_bytes().as_ref(),
                b"bad option \"-namespace\": must be -global or --",
            );
        } else {
            assert_eq!(selected.unwrap(), 2);
        }
        let words =
            ["--", "-namespace"].map(|word| super::Value::new_native_string_bytes(word.as_bytes()));
        assert_eq!(super::skip_hidden_options(&mut vm, &words).unwrap(), 1);
    }
}

#[test]
fn c84_primitive_failure_keeps_direct_code_and_eval_completion_supplies_none() {
    // naming.numeric.original-capi-scalar-publication-width
    // docs/design/analysis/name-resolution-proofs/numeric-original-capi-scalar-publication-width.md
    // naming.numeric.seeded-wide-frontier
    // docs/design/analysis/name-resolution-proofs/numeric-seeded-wide-frontier.md
    // Actual VM command-completion projection only; the neutral original
    // getter and failure renderer retain their independent direct purpose.
    use tcl_syntax::scalar_getter::NativeScalarGetterKind;
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.4").unit_profile();
    let mut vm = crate::native_fixture::interpreter(profile);
    let dialect = vm.native_invocation_dialect();
    for kind in [
        NativeScalarGetterKind::Int,
        NativeScalarGetterKind::Long,
        NativeScalarGetterKind::Wide,
        NativeScalarGetterKind::Double,
        NativeScalarGetterKind::Boolean,
    ] {
        vm.restore_guest_error_code(super::Value::new_native_string_bytes(
            b"SEEDED CODE".as_slice(),
        ));
        let original = super::Value::new_native_string_bytes(b"bad".as_slice());
        let failure = original
            .native_scalar_probe_with_environment(dialect, kind, vm.host().numeric_environment())
            .unwrap()
            .unwrap_err();
        let record = original
            .native_scalar_failure_presentation(dialect, kind, failure)
            .unwrap();
        assert_eq!(
            record.error_code_update(),
            &tcl_syntax::scalar_getter::NativeScalarGetterErrorCode::Unchanged
        );
        let code = vm.native_return_error_code().unwrap().clone();
        assert_eq!(
            vm.native_string_bytes(&code).unwrap().as_ref(),
            b"SEEDED CODE"
        );
        let error = tcl_syntax::value::ValueError::NativeScalarGetter(Box::new(record));
        let completion = super::completion_from_cmd_error(&mut vm, error.into());
        assert_eq!(completion.code, super::Code::Error);
        let code = super::opt_get_checked(&mut vm, &completion.options, b"-errorcode")
            .unwrap()
            .unwrap();
        assert_eq!(vm.native_string_bytes(&code).unwrap().as_ref(), b"NONE");
        assert!(vm.finish_host_execution(completion).is_ok());
    }
}

#[test]
fn original_command_core_refusal_keeps_typed_first_host_cause() {
    // naming.interpreter.original-child-host-refusal-transport
    // docs/design/analysis/name-resolution-proofs/interpreter-original-child-host-refusal-transport.md
    // Central CmdCore adapter control only; this does not enter a child or
    // supply native invocation, command or value-operation availability.
    use tcl_syntax::raw_string::NativeValueAccessRefusal;
    let original = NativeValueAccessRefusal::CommandProtocolUnavailable("original command query");
    let later = NativeValueAccessRefusal::CharacterModelUnavailable;
    let mut vm = crate::Vm::new();
    let _ = super::completion_from_cmd_error(&mut vm, original.into());
    let completion = super::completion_from_cmd_error(&mut vm, later.into());
    assert!(matches!(
        vm.finish_host_execution(completion),
        Err(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(actual)) if actual == original
    ));
}

#[test]
fn jim_core_alias_query_retains_original_prefix_header_and_members() {
    // naming.alias.jim-original-core-prefix-object-storage
    // docs/design/analysis/name-resolution-proofs/alias-jim-original-core-prefix-object-storage.md
    // Native273 independently measures name/member identity and absent
    // resident member strings. Repeated query header identity is a current
    // implementation owner control, not an additional native API observation.
    let profile = tcl_registry::model::resolve_environment("jim").unit_profile();
    let mut vm = crate::native_fixture::core(profile);
    let head = super::Value::new_native_string_bytes(b"alias".as_slice());
    let name = super::Value::new_native_string_bytes(b"::original_prefix_alias".as_slice());
    let target = super::Value::new_native_string_bytes(b"list".as_slice());
    let member = super::Value::int(17);
    assert!(member.resident_string_bytes().is_none());
    let definition = vm
        .invoke_host_original_object_vector(&head, &[name.clone(), target.clone(), member.clone()]);
    assert_eq!(definition.code, super::Code::Ok);
    assert!(definition.result.is_same_object(&name));
    assert!(member.resident_string_bytes().is_none());
    let info = super::Value::new_native_string_bytes(b"info".as_slice());
    let selector = super::Value::new_native_string_bytes(b"alias".as_slice());
    let query = vm.invoke_host_original_object_vector(&info, &[selector.clone(), name.clone()]);
    assert_eq!(query.code, super::Code::Ok);
    let members = vm.list_elements(&query.result).unwrap();
    assert_eq!(members.len(), 2);
    assert!(members[0].is_same_object(&target));
    assert!(members[1].is_same_object(&member));
    assert!(member.resident_string_bytes().is_none());
    let repeated = vm.invoke_host_original_object_vector(&info, &[selector, name.clone()]);
    assert_eq!(repeated.code, super::Code::Ok);
    assert!(repeated.result.is_same_object(&query.result));
    assert!(member.resident_string_bytes().is_none());
    let invocation = vm.invoke_host_original_object_vector(&name, &[]);
    assert_eq!(invocation.code, super::Code::Ok);
    let members = vm.list_elements(&invocation.result).unwrap();
    assert_eq!(members.len(), 1);
    assert!(members[0].is_same_object(&member));
    assert!(member.resident_string_bytes().is_none());
}

#[test]
fn native_wrong_arguments_completion_preserves_the_selected_string_producer() {
    // naming.variable.original-upvar-and-exists-completion-and-name-windows
    // docs/design/analysis/name-resolution-proofs/variable.original-upvar-and-exists-completion-and-name-windows.md
    // naming.list.original-assign-objects-and-instructions
    // docs/design/analysis/name-resolution-proofs/list.original-assign-objects-and-instructions.md
    // Shared presenter binding, separately from the original native rows.
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        let error = tcl_cmd_core::CmdError::wrong_args_bytes(b"upvar ?level? otherVar localVar");
        let message = error.message_bytes().to_vec();
        let completion = super::completion_from_cmd_error(&mut vm, error);
        assert_eq!(completion.code, super::Code::Error, "{engine}");
        assert!(
            matches!(
                completion.result.native_object_snapshot().cache,
                Cache::String { .. }
            ),
            "{engine}"
        );
        assert_eq!(
            completion.result.resident_string_bytes().unwrap().as_ref(),
            message,
            "{engine}"
        );
        let unrelated =
            super::completion_from_cmd_error(&mut vm, tcl_cmd_core::CmdError::new_bytes(message));
        assert!(
            matches!(unrelated.result.native_object_snapshot().cache, Cache::None),
            "{engine}: unrelated message"
        );
    }
}

#[test]
fn return_level_replacement_preserves_opaque_option_objects() {
    let key = super::Value::new_native_string_bytes(b"-opaque\xff\0TAIL".as_slice());
    let value = super::Value::new_native_string_bytes(b"VALUE\xff\0TAIL".as_slice());
    let options = super::Value::list(vec![
        key.clone(),
        value.clone(),
        super::Value::string("-level"),
        super::Value::int(2),
    ]);
    let mut vm = crate::native_fixture::interpreter(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile(),
    );
    let replaced = super::with_return_level(&mut vm, &options, 1).unwrap();
    let pairs = replaced.as_list().unwrap();
    assert!(pairs[0].is_same_object(&key));
    assert!(pairs[1].is_same_object(&value));
    assert_eq!(pairs[0].string_bytes().as_ref(), b"-opaque\xff\0TAIL");
    assert_eq!(pairs[3].as_int().unwrap(), 1);
}

#[test]
fn time_keeps_original_counted_script_and_skips_zero_count_materialization() {
    // Source proof: naming.time.original-script-object-evaluation
    // docs/design/analysis/name-resolution-proofs/time-original-script-object-evaluation.md
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        let invalid = super::Value::new_native_string_bytes(b"invalid script \xff".as_slice());
        let skipped = super::cmd_time(&mut vm, &[invalid, super::Value::int(0)]);
        assert_eq!(skipped.code, tcl_runtime_api::Code::Ok, "{engine}");
        assert_eq!(
            skipped.result.string_bytes().as_ref(),
            b"0 microseconds per iteration"
        );
        let body = super::Value::new_native_string_bytes(b"set {time\xff} OK".as_slice());
        let completion = super::cmd_time(&mut vm, &[body, super::Value::int(2)]);
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

#[test]
fn physical_procedure_capture_and_header_remain_separate_from_authored_names() {
    // Native proof: naming.procedure.counted-body-header-selection
    // docs/design/analysis/name-resolution-proofs/procedure-counted-body-header-selection.md
    use super::{Value, Vm};
    use std::rc::Rc;
    use tcl_dialect::NativeProcedureHeaderCompilation as Header;
    let host = tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile();
    let authored = tcl_dialect::DialectProfile::irules();
    let mut vm = Vm::with_native_core(
        Box::new(Vec::<u8>::new()),
        Rc::new(crate::host_native::NativeHost::new()),
        host,
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    vm.set_dialect_profile(authored);
    assert!(vm.set_command_surface_profile(host));
    assert!(
        vm.set_logical_name_provider(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
            tcl_dialect::TclVersion::V8_4,
        ))
    );
    let parameters = Value::new_native_string_bytes(b"args".as_slice());
    let original = Value::new_native_string_bytes(b"\0".as_slice());
    let alias = original.clone();
    let captured = vm
        .capture_procedure_declaration_body(&parameters, &original)
        .unwrap();
    assert_ne!(
        captured.native_object_identity(),
        original.native_object_identity()
    );
    assert_eq!(captured.string_bytes().as_ref(), b"\0");
    assert_eq!(
        alias.native_object_identity(),
        original.native_object_identity()
    );
    // These exact original headers are independently recorded in the
    // native_procedure_headers corpus (case 7 of each selected release).
    let native = include_str!("../../../tcl-syntax/tests/data/native_procedure_headers/9.0.4.txt");
    assert!(native.lines().nth(7).unwrap().ends_with("header=0"));
    assert_eq!(
        super::procedure_header_compilation(&mut vm, &parameters, &captured).unwrap(),
        Header::Absent
    );
    assert_eq!(
        tcl_registry::native_procedure::procedure_header_compilation_bytes(
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4),
            Some(b"args"),
            Some(b"\0"),
            Some(false),
        ),
        Header::NoOp,
    );
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
    use super::Value;
    let decode = |text: &str| {
        assert_eq!(text.len() % 2, 0);
        text.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect::<Vec<_>>()
    };
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.4").unit_profile();
    let mut compared = 0;
    for row in include_str!("../../tests/data/native_procedure_parse_context/8.4.20.tsv").lines() {
        let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
        assert_eq!(fields.len(), 8);
        let mut vm = crate::native_fixture::interpreter(profile);
        let name = Value::new_native_string_bytes(decode(fields[3]));
        let definition = super::cmd_proc(
            &mut vm,
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
        let completion = vm.invoke_host_original_object_vector(&name, &arguments);
        assert_eq!(completion.code.as_int().to_string(), fields[2], "{row}");
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            decode(fields[5]),
            "{row}"
        );
        assert_eq!(vm.error_info_value().unwrap(), decode(fields[6]), "{row}");
        assert_eq!(
            super::opt_get(&completion.options, "-errorcode")
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

fn procedure_definition_object_state(value: &super::Value) -> (&'static str, usize) {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot;
    let snapshot = value.native_object_snapshot();
    let kind = match snapshot.cache {
        NativeObjectCacheSnapshot::None => "none",
        NativeObjectCacheSnapshot::List { .. } => "list",
        NativeObjectCacheSnapshot::ByteArray { .. } => "bytearray",
        other => panic!("unexpected definition cache {other:?}"),
    };
    (kind, usize::from(snapshot.resident.is_some()))
}

#[test]
fn procedure_body_capture_matches_58_native_definition_windows() {
    // Native proof: naming.procedure.original-string-body-capture
    // docs/design/analysis/name-resolution-proofs/procedure-original-string-body-capture.md
    // Native proof: naming.procedure.original-list-bytearray-body-capture
    // docs/design/analysis/name-resolution-proofs/procedure-original-list-bytearray-body-capture.md
    use super::{Command, Value};
    let engines = [
        (
            "tcl8.4",
            include_str!("../../tests/data/native_procedure_body_capture/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../tests/data/native_procedure_body_capture/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../tests/data/native_procedure_body_capture/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../tests/data/native_procedure_body_capture/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_procedure_body_capture/9.1.0.tsv"),
        ),
        (
            "jim",
            include_str!("../../tests/data/native_procedure_body_capture/Jim.tsv"),
        ),
    ];
    let mut compared = 0;
    for (engine, fixture) in engines {
        for row in fixture.lines() {
            let expected = tcl_test_support::fixture::tsv_fields(row)
                .map(str::to_owned)
                .collect::<Vec<_>>();
            assert_eq!(expected.len(), 12);
            let kind = expected[0].parse::<usize>().unwrap();
            let refs = expected[1].parse::<usize>().unwrap();
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = crate::native_fixture::interpreter(profile);
            let dialect = vm.native_invocation_dialect();
            let protocol = dialect.native_string_protocol().unwrap();
            let body = match kind {
                0 => Value::new_native_string_bytes(&b"return OK"[..]),
                1 => Value::native_list_constructor(
                    vec![
                        Value::new_native_string_bytes(&b"return"[..]),
                        Value::new_native_string_bytes(&b"OK"[..]),
                    ],
                    protocol,
                ),
                2 => Value::from_native_byte_array(std::rc::Rc::from(&b"return OK"[..]), dialect)
                    .unwrap(),
                3 => Value::new_native_string_bytes(&b"set value \""[..]),
                4 => Value::new_native_string_bytes(&b"return OK\0error BAD"[..18]),
                _ => panic!("unknown fixture kind"),
            };
            let argv = [
                Value::new_native_string_bytes(&b"p"[..]),
                Value::new_native_string_bytes(&b""[..]),
                body,
            ];
            let _alias = (refs == 2).then(|| argv[2].clone());
            assert_eq!(argv[2].native_object_reference_count(), refs);
            let before = procedure_definition_object_state(&argv[2]);
            let completion = super::cmd_proc(&mut vm, &argv);
            assert_eq!(
                completion.code,
                tcl_runtime_api::Code::Ok,
                "{engine}: {row}"
            );
            let original_refs = argv[2].native_object_reference_count();
            let after = procedure_definition_object_state(&argv[2]);
            let Some(Command::Proc(proc)) = vm.lookup_command("p") else {
                panic!("missing native declaration")
            };
            assert!(proc.body.is_none());
            let stored = procedure_definition_object_state(&proc.body_src);
            let actual = vec![
                kind.to_string(),
                refs.to_string(),
                "0".into(),
                usize::from(
                    proc.body_src.native_object_identity() == argv[2].native_object_identity(),
                )
                .to_string(),
                original_refs.to_string(),
                before.0.into(),
                before.1.to_string(),
                after.0.into(),
                after.1.to_string(),
                stored.0.into(),
                stored.1.to_string(),
                proc.body_src.native_object_reference_count().to_string(),
            ];
            assert_eq!(actual, expected, "{engine}: {row}");
            compared += 1;
        }
    }
    assert_eq!(compared, 58);
}

#[test]
fn procedure_definition_and_activation_match_42_native_observations() {
    // Native proof: naming.procedure.original-malformed-body-arity-order
    // docs/design/analysis/name-resolution-proofs/procedure-original-malformed-body-arity-order.md
    // Native proof: naming.procedure.original-wrong-arity-body-withdrawal
    // docs/design/analysis/name-resolution-proofs/procedure-original-wrong-arity-body-withdrawal.md
    use super::{Command, Value};
    let engines = [
        (
            "tcl8.4",
            include_str!("../../tests/data/native_procedure_activation/8.4.txt"),
        ),
        (
            "tcl8.5",
            include_str!("../../tests/data/native_procedure_activation/8.5.txt"),
        ),
        (
            "tcl8.6",
            include_str!("../../tests/data/native_procedure_activation/8.6.txt"),
        ),
        (
            "tcl9.0",
            include_str!("../../tests/data/native_procedure_activation/9.0.txt"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_procedure_activation/9.1.txt"),
        ),
        (
            "jim",
            include_str!("../../tests/data/native_procedure_activation/Jim.txt"),
        ),
    ];
    let mut compared = 0;
    for (engine, fixture) in engines {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        let protocol = vm
            .native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
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
        let mut check = |vm: &mut crate::Vm, completion: crate::Completion<Value>| {
            let row = rows.next().unwrap();
            assert_eq!(
                completion.code.as_int().to_string().as_bytes(),
                row[1],
                "{engine}/{row:?}"
            );
            assert_eq!(
                vm.native_string_bytes(&completion.result).unwrap().as_ref(),
                row[2],
                "{engine}/{row:?}"
            );
            compared += 1;
        };
        for body in ["set value \"", "error BODY"] {
            let definition = vm.invoke_command(
                "proc",
                &[Value::string("p"), Value::string("x"), Value::string(body)],
            );
            check(&mut vm, definition);
            assert!(
                matches!(vm.lookup_command("p"), Some(Command::Proc(proc)) if proc.body.is_none()),
                "{engine}"
            );
            let missing = vm.invoke_command("p", &[]);
            check(&mut vm, missing);
            let supplied = vm.invoke_command("p", &[Value::string("OK")]);
            check(&mut vm, supplied);
        }
        let definition = vm.invoke_command(
            "proc",
            &[
                Value::string("q"),
                Value::string("x"),
                Value::string("set side BODY;return OK"),
            ],
        );
        assert_eq!(definition.code, tcl_runtime_api::Code::Ok);
        vm.set_var_bytes(b"side", Value::string("BEFORE")).unwrap();
        let arity = vm.invoke_command("q", &[]);
        check(&mut vm, arity);
        assert_eq!(
            vm.get_var_bytes(b"side").unwrap().string_bytes().as_ref(),
            b"BEFORE"
        );
    }
    assert_eq!(compared, 42);
}

// Native proof: naming.variable.original-level-object-conversion-and-selection
// docs/design/analysis/name-resolution-proofs/variable.original-level-object-conversion-and-selection.md
#[test]
fn original_frame_selectors_match_native_cache_and_failure_order() {
    let fixtures = [
        (
            tcl_dialect::TclVersion::V8_4,
            include_str!("../../../tcl-syntax/tests/data/native_frame_levels/8.4.20.jsonl"),
        ),
        (
            tcl_dialect::TclVersion::V8_5,
            include_str!("../../../tcl-syntax/tests/data/native_frame_levels/8.5.19.jsonl"),
        ),
        (
            tcl_dialect::TclVersion::V8_6,
            include_str!("../../../tcl-syntax/tests/data/native_frame_levels/8.6.18.jsonl"),
        ),
        (
            tcl_dialect::TclVersion::V9_0,
            include_str!("../../../tcl-syntax/tests/data/native_frame_levels/9.0.4.jsonl"),
        ),
        (
            tcl_dialect::TclVersion::V9_1,
            include_str!("../../../tcl-syntax/tests/data/native_frame_levels/9.1.0.jsonl"),
        ),
    ];
    let inputs: [&[u8]; 13] = [
        b"1",
        b"1\0X",
        b"#1",
        b"#1\0X",
        b"-1",
        b"+1",
        b"01",
        b"1.0",
        b"NaN",
        b"2147483648",
        b"4294967295",
        b"0",
        b"bad",
    ];
    let mut count = 0;
    let jim = tcl_registry::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
    );
    let fixtures = fixtures
        .into_iter()
        .map(|(version, fixture)| {
            (
                tcl_registry::InvocationDialect::for_version(version),
                fixture,
            )
        })
        .chain(std::iter::once((
            jim,
            include_str!("../../../tcl-syntax/tests/data/native_frame_levels/jim0.84.jsonl"),
        )));
    for (dialect, fixture) in fixtures {
        let protocol = dialect.native_frame_level_protocol().unwrap();
        let jim_inputs: [&[u8]; 18] = [
            b"1",
            b"1\0X",
            b"#1",
            b"#1\0X",
            b"-1",
            b"+1",
            b"01",
            b"1.0",
            b"NaN",
            b"2147483648",
            b"4294967295",
            b"0",
            b"bad",
            b"#1 ",
            b"#9223372036854775808",
            b"#-0x8000000000000000",
            b"#-0",
            b"#0o1",
        ];
        let inputs = if protocol.is_jim084() {
            &jim_inputs[..]
        } else {
            &inputs[..]
        };
        for (case, line) in fixture.lines().enumerate() {
            assert_original_frame_selector(dialect, inputs, case, line);
            count += 1;
        }
    }
    assert_eq!(count, 101);
}

struct NoFrameNumeric(std::rc::Rc<dyn tcl_platform::Host>);
impl tcl_platform::Host for NoFrameNumeric {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.0.capabilities()
    }
    fn clock(&self) -> &dyn tcl_platform::Clock {
        self.0.clock()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self.0.stdio()
    }
    fn env(&self) -> &dyn tcl_platform::Env {
        self.0.env()
    }
    fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
        self.0.native_integer_formatter()
    }
}

#[test]
fn original_frame_value_adapter_requires_selected_environment_and_live_header() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // VM Value/Host operation control; Runtime interpreter currency remains independent.
    use tcl_registry::frame_effect::NativeFrameLevelObject;
    for environment in ["tcl8.4", "tcl8.5"] {
        let host = std::rc::Rc::new(NoFrameNumeric(std::rc::Rc::new(
            crate::host_native::NativeHost::new(),
        )));
        let vm = crate::Vm::with_native_core(
            Box::new(Vec::<u8>::new()),
            host,
            tcl_registry::model::resolve_environment(environment).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let original = crate::Value::new_native_string_bytes(b"#0".as_slice());
        let dialect = vm.native_scalar_carrier_dialect();
        let protocol = dialect.native_frame_level_protocol().unwrap();
        let lease = original.native_lifetime_lease();
        let mut operand = super::OriginalFrameLevel::selected(&vm, lease.value()).unwrap();
        if environment == "tcl8.4" {
            assert!(matches!(
                protocol.resolve_object(0, &mut operand),
                Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)
            ));
            assert!(original.native_frame_level_cache().is_none());
        } else {
            original
                .install_native_frame_level_cache(
                    tcl_registry::NativeFrameLevelCache::Absolute(0),
                    dialect,
                )
                .unwrap();
            assert_eq!(
                protocol
                    .resolve_object(0, &mut operand)
                    .unwrap()
                    .unwrap()
                    .target,
                0
            );
            drop(original);
            assert!(!lease.value().native_object_is_live());
            assert!(operand.native_frame_is_integer().is_err());
            assert!(operand.native_frame_is_machine_integer().is_err());
            assert!(operand.native_frame_cache().is_err());
        }
    }
}

fn native_frame_field<'a>(line: &'a str, name: &str) -> &'a str {
    let marker = format!("\"{name}\":\"");
    line.split_once(&marker)
        .unwrap()
        .1
        .split('"')
        .next()
        .unwrap()
}

fn decode_native_frame_hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn assert_original_frame_selector(
    dialect: tcl_registry::InvocationDialect,
    inputs: &[&[u8]],
    case: usize,
    line: &str,
) {
    use super::OriginalFrameLevel;
    use tcl_registry::frame_effect::{NativeFrameLevelFailure, NativeFrameLevelResolution};
    use tcl_syntax::number::Number;
    use tcl_syntax::scalar_getter::NativeScalarCache;
    let version = dialect;
    let protocol = dialect.native_frame_level_protocol().unwrap();
    let original = match case.cmp(&inputs.len()) {
        std::cmp::Ordering::Less => crate::Value::from_string_bytes(inputs[case]),
        std::cmp::Ordering::Equal => crate::Value::int(1),
        std::cmp::Ordering::Greater => crate::Value::native_double(
            if case == inputs.len() + 1 {
                1.0
            } else {
                f64::NAN
            },
            dialect,
        ),
    };
    let alias = original.clone();
    let environment = if protocol.is_jim084() {
        "jimtcl"
    } else {
        match protocol.tcl_version().unwrap() {
            tcl_dialect::TclVersion::V8_4 => "tcl8.4",
            tcl_dialect::TclVersion::V8_5 => "tcl8.5",
            tcl_dialect::TclVersion::V8_6 => "tcl8.6",
            tcl_dialect::TclVersion::V9_0 => "tcl9.0",
            tcl_dialect::TclVersion::V9_1 => "tcl9.1",
        }
    };
    let vm = crate::Vm::with_native_core(
        Box::new(Vec::<u8>::new()),
        std::rc::Rc::new(crate::host_native::NativeHost::new()),
        tcl_registry::model::resolve_environment(environment).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(vm.native_scalar_carrier_dialect(), dialect);
    let mut operand = OriginalFrameLevel::selected(&vm, &original).unwrap();
    let outcome = protocol.resolve_leading_object(2, &mut operand).unwrap();
    let expected = decode_native_frame_hex(native_frame_field(line, "result"));
    match outcome {
        Ok(NativeFrameLevelResolution {
            explicit: true,
            target,
        }) => {
            assert!(line.contains("\"code\":0"), "{version:?} {case}");
            assert_eq!(
                expected,
                target.to_string().as_bytes(),
                "{version:?} {case}"
            );
        }
        Ok(NativeFrameLevelResolution {
            explicit: false, ..
        }) => {
            assert!(line.contains("\"code\":1"), "{version:?} {case}");
            assert!(
                expected.starts_with(b"invalid command name"),
                "{version:?} {case}"
            );
        }
        Err(NativeFrameLevelFailure::Primitive(record)) => {
            assert_eq!(expected, record.eval_message_bytes(), "{version:?} {case}");
        }
        Err(NativeFrameLevelFailure::BadLevel { name, .. }) => {
            let mut actual = b"bad level \"".to_vec();
            actual.extend_from_slice(&name);
            actual.push(b'"');
            assert_eq!(expected, actual, "{version:?} {case}");
        }
    }
    let actual_cache = if alias.native_frame_level_cache().is_some() {
        "levelReference"
    } else {
        match alias.native_scalar_cache() {
            Some(NativeScalarCache::Number(Number::Int(_))) => "integer",
            Some(NativeScalarCache::Number(Number::Double(_) | Number::Nan { .. })) => "double",
            None => "string",
            other => panic!("unexpected frame cache {other:?}"),
        }
    };
    let expected_cache = match native_frame_field(line, "after") {
        "int" | "wideInt" => "integer",
        other => other,
    };
    assert_eq!(actual_cache, expected_cache, "{version:?} {case}");
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
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).unit_profile();
        for (source, c_result, jim_result) in cases {
            let mut vm = crate::native_fixture::interpreter(profile);
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
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        let result = vm
                .try_eval_source(
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

#[test]
fn expression_source_cache_uses_the_actual_native_grammar() {
    let mut modern = crate::Vm::new();
    modern.set_dialect_profile(tcl_dialect::DialectProfile::find("tcl9.0").unwrap());
    let source = crate::Value::string("0d10");
    assert!(modern.prepare_expression_value(&source).is_ok());
    assert!(
        modern.prepare_expression_value(&source).is_ok(),
        "warm cache"
    );
    let mut legacy = crate::Vm::new();
    legacy.set_dialect_profile(tcl_dialect::DialectProfile::find("tcl8.4").unwrap());
    assert!(
        legacy.prepare_expression_value(&source).is_err(),
        "an expression object cannot retain another engine's numeral grammar"
    );
    assert!(
        modern.prepare_expression_value(&source).is_ok(),
        "a failed conversion must not corrupt the original source bytes"
    );
}

#[test]
fn checked_expression_cache_retains_operator_base_independently_of_lexing() {
    use tcl_runtime_api::{NativeExecutionError, NativeExpressionRefusal};
    let modern = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let mut restricted = modern.clone();
    restricted.expr_grammar_base = Some(tcl_dialect::TclVersion::V8_4);
    let restricted = restricted.intern();
    assert_eq!(modern.name, restricted.name);
    assert_eq!(modern.grammar, restricted.grammar);
    let source = crate::Value::string("2 ** 3");
    let mut vm = crate::Vm::new();
    vm.set_dialect_profile(modern);
    assert_eq!(
        {
            let invocation_argument = source.clone();
            vm.try_invoke_command("expr", std::slice::from_ref(&invocation_argument))
        }
        .unwrap()
        .result
        .to_str()
        .as_ref(),
        "8"
    );
    vm.set_dialect_profile(restricted);
    let NativeExecutionError::ExpressionRefusal(failure) = {
        let invocation_argument = source.clone();
        vm.try_invoke_command("expr", std::slice::from_ref(&invocation_argument))
    }
    .unwrap_err() else {
        panic!("operator syntax needs an expression provider");
    };
    assert_eq!(
        failure.reason,
        NativeExpressionRefusal::UnpresentedSyntaxFailure
    );
    assert_eq!(failure.source.bytes(), b"2 ** 3");
    assert_eq!(failure.source_profile, restricted.cache_key());
    assert_eq!(
        failure.source_profile.profile().expr_grammar_base,
        Some(tcl_dialect::TclVersion::V8_4)
    );
    assert_eq!(
        failure.native_profile.profile().runtime_version(),
        Some(tcl_dialect::TclVersion::V9_0)
    );
    vm.set_dialect_profile(modern);
    assert_eq!(
        vm.try_invoke_command("expr", &[source])
            .unwrap()
            .result
            .to_str()
            .as_ref(),
        "8"
    );
}

#[test]
fn legacy_bareword_errors_use_actual_fixed_function_name_resolution() {
    for (dialect, source, expected) in [
        (
            "tcl8.4",
            "nope",
            "syntax error in expression \"nope\": variable references require preceding $",
        ),
        (
            "tcl8.4",
            "abs",
            "syntax error in expression \"abs\": expected parenthesis enclosing function arguments",
        ),
        ("jim", "nope", "syntax error in expression: \"nope\""),
        (
            "jim",
            "abs",
            "syntax error in expression: \"abs\": function requires parentheses",
        ),
    ] {
        let profile = crate::environment::profile_for_dialect(dialect);
        let mut vm = crate::native_fixture::core(profile);
        let completion = vm.try_eval_expr(source).unwrap();
        assert_eq!(completion.code, tcl_runtime_api::Code::Error);
        assert_eq!(completion.result.to_str().as_ref(), expected);
        // Jim reports `-errorcode` only from a global `errorCode`, which
        // its core never creates (native_bootstrap/core-roots.tsv).
        let error_code = super::opt_get(&completion.options, "-errorcode");
        assert_eq!(
            error_code.map(|code| code.to_str().to_string()).as_deref(),
            (dialect != "jim").then_some("NONE"),
            "{dialect}: {source}"
        );
    }
}

// The actual C9 source/Catch surface is supported. An expression nested
// beyond the checked parser's depth is outside its window, though C
// evaluates it (tclsh9.0 gives 1 for 9000 nested parentheses); no absent
// F5 parser or eval-object provider is used to manufacture that refusal.
fn vm_with_supported_source_and_unsupported_expression_window()
-> (crate::Vm, &'static tcl_dialect::DialectProfile) {
    let profile = crate::environment::profile_for_dialect("tcl9.0");
    (crate::native_fixture::interpreter(profile), profile)
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
        assert_eq!(failure.interpreter.owner, vm.owner_nonce);
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
        assert_eq!(vm.current_level(), 0, "host refusal retires proc storage");
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
    assert_eq!(failure.interpreter.owner, vm.owner_nonce);
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
    let actual = crate::environment::profile_for_dialect("tcl9.0");
    let source = tcl_dialect::DialectProfile::irules();
    let mut vm = crate::native_fixture::interpreter(actual);
    vm.set_dialect_profile(source);
    assert_eq!(
        vm.actual_native_execution_profile().cache_key(),
        actual.cache_key()
    );
    assert_eq!(vm.source_profile().cache_key(), source.cache_key());
    assert!(vm.expression_evaluation_policy().is_none());
    assert!(vm.eval_object_protocol().is_none());
    assert!(vm.try_eval_source("set original_f5_unentered 1").is_err());
    assert!(vm.get_var("original_f5_unentered").is_none());
}

#[test]
fn jim_static_storage_matches_the_fixed_native_fixture() {
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    let source =
        include_str!("../../../tcl-syntax/tests/data/body_execution/native-static-storage.tcl");
    let expected =
        include_str!("../../../tcl-syntax/tests/data/body_execution/native-static-storage-jim.txt");
    for (line, expected) in source.lines().skip(1).zip(expected.lines()) {
        let words = tcl_syntax::list::split_list_jim(line);
        let wanted = tcl_syntax::list::split_list_jim(expected);
        assert_eq!(words.len(), 3);
        assert_eq!(wanted.len(), 3);
        assert_eq!(words[1], wanted[0]);
        let mut vm = crate::native_fixture::interpreter(profile);
        crate::native_fixture::install_distribution(&mut vm);
        let completion = vm.eval_source(&words[2]).unwrap();
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
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    for (source, wanted) in cases {
        let mut vm = crate::native_fixture::interpreter(profile);
        crate::native_fixture::install_distribution(&mut vm);
        let completion = vm.eval_source(source).unwrap();
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
    profiles.push(tcl_registry::model::ingress::resolve_environment("jim").unit_profile());
    for profile in profiles {
        let mut vm = crate::Vm::new();
        vm.set_dialect_profile(profile);
        vm.set_compiler(Box::new(
            tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        let completion = vm.eval_source("proc written {} {}").unwrap();
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
    profiles.push(tcl_registry::model::ingress::resolve_environment("jim").unit_profile());
    for profile in profiles {
        for (source, tcl, jim) in cases {
            let mut vm = crate::native_fixture::interpreter(profile);
            let result = vm.eval_source(source).unwrap();
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
fn script_object_materialization_keeps_opaque_counted_source() {
    use crate::{Value, Vm};
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = Vm::new();
        vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
        );
        let original = Value::new_native_string_bytes(&b"set opaque {\xff\0TAIL}; set opaque"[..]);
        let source = vm
            .native_script_source_image(&original)
            .unwrap_or_else(|_| panic!("physical script issuer {engine}"));
        assert_eq!(source.bytes(), b"set opaque {\xff\0TAIL}; set opaque");
        assert_eq!(
            original.resident_string_bytes().unwrap().as_ref(),
            source.bytes()
        );
    }
}

#[test]
fn original_list_eval_retains_member_identity_without_materialising_source() {
    use crate::Value;
    use tcl_runtime_api::Code;
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        if dialect == "jim" {
            crate::native_fixture::install_distribution(&mut vm);
        }
        let result = Value::new_native_string_bytes(&b"\xff\0TAIL"[..]);
        let script = Value::list(vec![Value::string("return"), result.clone()]);
        let completion = {
            let invocation_argument = script.clone();
            vm.try_invoke_command("eval", std::slice::from_ref(&invocation_argument))
        }
        .unwrap();
        assert_eq!(completion.code, Code::Return, "{dialect}");
        assert_eq!(
            completion.result.native_object_identity(),
            result.native_object_identity(),
            "{dialect}"
        );
        assert_eq!(completion.result.string_bytes().as_ref(), b"\xff\0TAIL");
        assert!(script.resident_string_bytes().is_none(), "{dialect}");
    }
}

#[test]
fn jim_native_catch_preserves_its_flag_and_exit_protocol() {
    use crate::Value;
    use tcl_runtime_api::Code;
    let mut vm = crate::native_fixture::interpreter(crate::environment::profile_for_dialect("jim"));
    let words = |args: &[&str]| {
        args.iter()
            .map(|word| Value::string(*word))
            .collect::<Vec<_>>()
    };
    vm.invoke_command("set", &words(&["r", "ORIGINAL"]));
    vm.invoke_command("set", &words(&["o", "OPTIONS"]));
    let completion = vm.invoke_command("catch", &words(&["-nook", "set x VALUE", "r", "o"]));
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(&*completion.result.to_str(), "VALUE");
    assert_eq!(
        &*vm.invoke_command("set", &words(&["r"])).result.to_str(),
        "ORIGINAL"
    );
    let completion = vm.invoke_command("catch", &words(&["-ok", "set x VALUE", "r", "o"]));
    assert_eq!(&*completion.result.to_str(), "0");
    assert_eq!(
        &*vm.invoke_command("set", &words(&["r"])).result.to_str(),
        "VALUE"
    );
    let completion = vm.invoke_command("catch", &words(&["-exit", "exit 12", "r", "o"]));
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(&*completion.result.to_str(), "6");
    assert_eq!(
        &*vm.invoke_command("set", &words(&["r"])).result.to_str(),
        "12"
    );
    assert!(vm.take_exit().is_none());
    let completion = vm.invoke_command("catch", &words(&["-noerror", "error BAD", "r", "o"]));
    assert_eq!(completion.code, Code::Error);
    assert_eq!(&*completion.result.to_str(), "BAD");
    let completion = vm.invoke_command("catch", &words(&["exit 12", "r", "o"]));
    assert_eq!(completion.code, Code::from_int(6));
    assert_eq!(vm.take_exit(), Some(12));
}

#[test]
fn shared_proc_parameter_parser_adapts_to_vm_values() {
    let (params, has_args) = parse_params("a {b {hello world}} args").unwrap();
    assert!(has_args);
    assert_eq!(params[1].name, "b");
    let default = params[1].default.as_ref().unwrap().to_str();
    assert_eq!(&*default, "hello world");
    assert_eq!(
        parse_params("{{} default}").err().unwrap(),
        "argument with no name"
    );
    assert_eq!(
        parse_params("{ns::name}").err().unwrap(),
        "formal parameter \"ns::name\" is not a simple name"
    );
}

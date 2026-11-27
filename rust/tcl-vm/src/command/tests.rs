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
        let mut vm = crate::native_fixture::core(profile);
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
        let mut vm = crate::native_fixture::core(profile);
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
    let mut vm = crate::native_fixture::core(profile);
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
        let mut vm = crate::native_fixture::core(profile);
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
    let mut vm = crate::native_fixture::core(
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
            let mut vm = crate::native_fixture::core(profile);
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

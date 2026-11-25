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

use super::super::{obj_bytes, ExceptionState};
use super::*;
use crate::interp::default_host;
use crate::{counters, environment::profile_for_dialect};
use tcl_registry::special_vars::NativeBootstrapInputs;

thread_local! {
    static C84_LOG_WRITES: std::cell::RefCell<Vec<Vec<u8>>> = const { std::cell::RefCell::new(Vec::new()) };
    static LOG_HEADERS: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn observe_c84_log(interp: &mut Interp, _arguments: &[*mut obj::TclObj]) -> super::super::Code {
    let original = interp.var_get_at(b"::errorInfo", 0).unwrap();
    C84_LOG_WRITES.with(|writes| writes.borrow_mut().push(obj_bytes(original)));
    LOG_HEADERS.with(|headers| headers.borrow_mut().push(original as usize));
    super::super::Code::Ok
}

fn check_repeated_error_episode(
    interp: &mut Interp,
    head: &obj::Owned,
    original: &obj::Owned,
    fields: &[&str],
    profile: &str,
) {
    use super::super::Code;
    assert_eq!(
        interp.eval_original_object_vector(&[head.as_ptr()]),
        Code::from_int(fields[8].parse().unwrap()),
        "{profile}"
    );
    assert!(
        !interp.host_refusal_pending(),
        "{profile}: {:?}",
        interp.native_access_refusal()
    );
    let current = interp.read_named_variable(b"::errorInfo").unwrap();
    assert_eq!(current != original.as_ptr(), fields[9] == "1", "{profile}");
    assert_eq!(
        interp.result_bytes() == b"X",
        fields[10] == "1",
        "{profile}"
    );
    assert_eq!(
        !obj_bytes(current)
            .windows(b"FIRST_ERROR_LONG".len())
            .any(|bytes| bytes == b"FIRST_ERROR_LONG"),
        fields[11] == "1",
        "{profile}"
    );
    assert_eq!(
        interp.eval_original_object_vector(&[head.as_ptr()]),
        Code::from_int(fields[12].parse().unwrap()),
        "{profile}"
    );
    assert!(
        !interp.host_refusal_pending(),
        "{profile}: {:?}",
        interp.native_access_refusal()
    );
    let current = interp.read_named_variable(b"::errorInfo").unwrap();
    assert_eq!(
        !obj_bytes(current)
            .windows(b"FIRST_ERROR_LONG".len())
            .any(|bytes| bytes == b"FIRST_ERROR_LONG"),
        fields[13] == "1",
        "{profile}"
    );
}

// Native proof: naming.variable.error-reset-distinct-episodes
// docs/design/analysis/name-resolution-proofs/variable.error-reset-distinct-episodes.md
#[test]
fn public_error_resets_preserve_global_headers_and_start_fresh_episodes() {
    use super::super::Code;
    let observations = include_str!(
        "../../../../../rust/tcl-registry/tests/data/native_error_variables/reset-episodes.tsv"
    );
    for (version, profile) in [
        ("8.4.20", "tcl8.4"),
        ("8.5.19", "tcl8.5"),
        ("8.6.18", "tcl8.6"),
        ("9.0.4", "tcl9.0"),
        ("9.1.0", "tcl9.1"),
    ] {
        let row = observations
            .lines()
            .find(|row| row.starts_with(version))
            .unwrap();
        let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
        counters::reset();
        {
            let mut interp = Interp::with_native_core(
                default_host(),
                profile_for_dialect(profile),
                NativeBootstrapInputs::default(),
            )
            .unwrap();
            assert_eq!(
                interp.eval_str(b"proc p {} {error FIRST_ERROR_LONG}"),
                Code::from_int(fields[1].parse().unwrap()),
                "{profile}"
            );
            let head = obj::Owned::fresh(new_string(b"p"));
            assert_eq!(
                interp.eval_original_object_vector(&[head.as_ptr()]),
                Code::from_int(fields[2].parse().unwrap()),
                "{profile}"
            );
            assert!(
                !interp.host_refusal_pending(),
                "{profile}: {:?}",
                interp.native_access_refusal()
            );
            let original = obj::Owned::retain(interp.read_named_variable(b"::errorInfo").unwrap());
            if interp.uses_c84_global_error_info() {
                // C8.4 TclEvalObjvInternal resets even the empty vector.
                assert_eq!(interp.eval_original_object_vector(&[]), Code::Ok);
                assert_eq!(
                    interp.read_named_variable(b"::errorInfo").unwrap() == original.as_ptr(),
                    fields[3] == "1"
                );
                assert_eq!(
                    unsafe { (*original.as_ptr()).ref_count },
                    fields[4].parse::<isize>().unwrap()
                );
                assert!(!interp.exc.borrow().native.legacy_copy);
                assert_eq!(interp.exc.borrow().native.info_len, 0);
            }
            assert_eq!(
                interp.eval_str(b"proc p {} {error X}"),
                Code::from_int(fields[5].parse().unwrap()),
                "{profile}"
            );
            assert_eq!(
                interp.read_named_variable(b"::errorInfo").unwrap() == original.as_ptr(),
                fields[6] == "1",
                "{profile}"
            );
            assert_eq!(
                unsafe { (*original.as_ptr()).ref_count },
                fields[7].parse::<isize>().unwrap(),
                "{profile}"
            );
            check_repeated_error_episode(&mut interp, &head, &original, &fields, profile);
        }
        assert_eq!(counters::finalize(), 0, "{profile}");
    }
}

thread_local! {
    static C84_CODE_EVENTS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    static C84_CODE_HEADERS: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn observe_c84_code(interp: &mut Interp, argv: &[*mut obj::TclObj]) -> super::super::Code {
    let name = obj_bytes(argv[1]);
    let operation = obj_bytes(argv[3]);
    let rooted = if name.starts_with(b"::") {
        name.clone()
    } else {
        [b"::".as_slice(), name.as_slice()].concat()
    };
    let value = interp.var_get(&rooted).expect("actual traced global cell");
    let bytes = obj_bytes(value);
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    C84_CODE_EVENTS.with(|events| {
        events.borrow_mut().push(format!(
            "{}|{}|{hex}",
            String::from_utf8(name).unwrap(),
            String::from_utf8(operation).unwrap(),
        ));
    });
    if rooted == b"::errorCode" {
        C84_CODE_HEADERS.with(|headers| headers.borrow_mut().push(value as usize));
    }
    super::super::Code::Ok
}

fn c84_code_interpreter() -> Interp {
    let mut interp = Interp::with_native_core(
        default_host(),
        profile_for_dialect("tcl8.4"),
        NativeBootstrapInputs::default(),
    )
    .unwrap();
    interp.register_builtin(b"observe", observe_c84_code);
    assert_eq!(
        interp.eval_str(
            b"trace variable ::errorInfo rw observe;trace variable ::errorCode rw observe"
        ),
        super::super::Code::Ok
    );
    C84_CODE_EVENTS.with(|events| events.borrow_mut().clear());
    C84_CODE_HEADERS.with(|headers| headers.borrow_mut().clear());
    interp
}

#[test]
fn c84_global_error_code_publication_matches_five_original_callback_controls() {
    let controls = include_str!("../../../tests/data/native_c84_error_code/controls.tsv");
    let sources: &[(&str, &[u8])] = &[
        (
            "conflict",
            b"package provide probe 1;catch {package provide probe 2} m;list $m $::errorCode",
        ),
        ("no-code", b"catch {error FAIL} m;list $m $::errorCode"),
        ("NONE", b"catch {error FAIL {} NONE} m;list $m $::errorCode"),
        (
            "structured",
            b"catch {error FAIL {} {CUSTOM DETAIL}} m;list $m $::errorCode",
        ),
        (
            "structured-getter",
            b"catch {lindex {} BAD} m;list $m $::errorCode",
        ),
    ];
    for row in controls.lines() {
        let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
        let source = sources
            .iter()
            .find(|(name, _)| *name == fields[0])
            .unwrap()
            .1;
        let mut interp = c84_code_interpreter();
        assert_eq!(
            interp.eval_str(source).as_int().to_string(),
            fields[1],
            "{row}"
        );
        let result: String = interp
            .result_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(result, fields[2], "{row}");
        C84_CODE_EVENTS.with(|events| assert_eq!(events.borrow().join(";"), fields[3], "{row}"));
        assert!(!interp.host_refusal_pending(), "{row}");
    }
}

#[test]
fn c84_nonempty_error_info_publication_matches_original_callback_controls() {
    let controls = include_str!("../../../tests/data/native_c84_error_code/nonempty/controls.tsv");
    let sources: &[(&str, &[u8])] = &[
        (
            "info-no-code",
            b"catch {error FAIL INFO} m;list $m $::errorInfo $::errorCode",
        ),
        (
            "info-NONE",
            b"catch {error FAIL INFO NONE} m;list $m $::errorInfo $::errorCode",
        ),
        (
            "info-structured",
            b"catch {error FAIL INFO {CUSTOM DETAIL}} m;list $m $::errorInfo $::errorCode",
        ),
    ];
    for row in controls.lines() {
        let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(row).collect();
        let source = sources
            .iter()
            .find(|(name, _)| *name == fields[0])
            .unwrap()
            .1;
        let mut interp = c84_code_interpreter();
        assert_eq!(
            interp.eval_str(source).as_int().to_string(),
            fields[1],
            "{row}"
        );
        let result: String = interp
            .result_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(result, fields[2], "{row}");
        C84_CODE_EVENTS.with(|events| assert_eq!(events.borrow().join(";"), fields[3], "{row}"));
        assert!(!interp.host_refusal_pending(), "{row}");
    }
}

#[test]
fn c84_explicit_error_code_setter_retains_the_same_original_header() {
    let mut interp = c84_code_interpreter();
    let head = obj::Owned::fresh(new_string(b"error"));
    let message = obj::Owned::fresh(new_string(b"FAIL"));
    let info = obj::Owned::fresh(new_string(b""));
    let code = obj::Owned::fresh(new_string(b"CUSTOM DETAIL"));
    assert_eq!(
        interp.eval_original_object_vector(&[
            head.as_ptr(),
            message.as_ptr(),
            info.as_ptr(),
            code.as_ptr(),
        ]),
        super::super::Code::Error
    );
    C84_CODE_HEADERS.with(|headers| {
        assert_eq!(
            headers.borrow().first().copied(),
            Some(code.as_ptr() as usize)
        )
    });
    assert_eq!(interp.var_get(b"::errorCode"), Some(code.as_ptr()));
    assert!(!interp.host_refusal_pending());
}

#[test]
fn c85_error_options_rearm_publication_of_the_restored_original_header() {
    counters::reset();
    {
        let mut interp = Interp::with_native_core(
            default_host(),
            profile_for_dialect("tcl8.5"),
            NativeBootstrapInputs::default(),
        )
        .unwrap();
        interp.register_builtin(b"observe_log", observe_c84_log);
        assert_eq!(
            interp.eval_str(b"trace variable ::errorInfo w observe_log"),
            super::super::Code::Ok
        );
        C84_LOG_WRITES.with(|writes| writes.borrow_mut().clear());
        LOG_HEADERS.with(|headers| headers.borrow_mut().clear());
        interp.set_result_bytes(b"SEED");
        interp.log_command_bytes(1, b"failing command");
        let original = interp.exc.borrow().native.info.as_ref().unwrap().as_ptr();
        assert!(!interp.exc.borrow().native.legacy_copy);
        let options = obj::Owned::fresh(
            crate::cmd_error::completion_options(&mut interp, super::super::Code::Error).unwrap(),
        );
        assert!(interp.exc.borrow().native.legacy_copy);
        assert_eq!(
            interp.exc.borrow().native.info.as_ref().unwrap().as_ptr(),
            original
        );
        interp.publish_and_reset_error();
        LOG_HEADERS.with(|headers| assert_eq!(headers.borrow().as_slice(), [original as usize; 2]));
        C84_LOG_WRITES.with(|writes| {
            assert_eq!(
                writes.borrow().as_slice(),
                [
                    b"SEED\n    while executing\n\"failing command\"".to_vec(),
                    b"SEED\n    while executing\n\"failing command\"".to_vec(),
                ]
            )
        });
        assert!(interp.exc.borrow().native.info.is_none());
        drop(options);
    }
    assert_eq!(counters::finalize(), 0);
}

#[test]
fn c84_command_logging_seeds_then_appends_the_original_global_receiver() {
    counters::reset();
    {
        let mut interp = Interp::with_native_core(
            default_host(),
            profile_for_dialect("tcl8.4"),
            NativeBootstrapInputs::default(),
        )
        .unwrap();
        interp.register_builtin(b"observe_c84_log", observe_c84_log);
        assert_eq!(
            interp.eval_str(b"trace variable ::errorInfo w observe_c84_log"),
            super::super::Code::Ok
        );
        C84_LOG_WRITES.with(|writes| writes.borrow_mut().clear());
        let original = obj::Owned::fresh(new_string(b"SEED"));
        interp.set_result(original.as_ptr());
        interp.log_command_bytes(1, b"failing command");
        assert!(!interp.host_refusal_pending());
        C84_LOG_WRITES.with(|writes| {
            assert_eq!(
                writes.borrow().as_slice(),
                [
                    b"SEED".to_vec(),
                    b"SEED\n    while executing\n\"failing command\"".to_vec()
                ]
            );
        });
        assert_eq!(interp.result.get(), original.as_ptr());
        assert_eq!(obj_bytes(original.as_ptr()), b"SEED");
        assert!(interp.exc.borrow().native.info.is_none());
        // Direct Tcl_LogCommandInfo clears ERR_ALREADY_LOGGED in C8.4;
        // the evaluating command, rather than this logger, owns the bit.
        assert!(!interp.exc.borrow().already_logged);
        interp.publish_and_reset_error();
        C84_LOG_WRITES.with(|writes| assert_eq!(writes.borrow().len(), 2));
    }
    assert_eq!(counters::finalize(), 0);
}

#[test]
fn command_logging_publishes_same_private_header_only_to_user_traced_cells() {
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        counters::reset();
        {
            let mut interp = Interp::with_native_core(
                default_host(),
                profile_for_dialect(profile),
                NativeBootstrapInputs::default(),
            )
            .unwrap();
            let public = obj::Owned::fresh(new_string(b"previous public info"));
            interp.var_set(b"::errorInfo", public.as_ptr()).unwrap();
            interp.set_result_bytes(b"first error");
            interp.log_command_bytes(1, b"first command");
            assert_eq!(interp.var_get_at(b"::errorInfo", 0), Some(public.as_ptr()));

            *interp.exc.borrow_mut() = ExceptionState::default();
            assert_eq!(
                interp.eval_str(b"trace add variable ::errorInfo write list"),
                super::super::Code::Ok,
                "{profile}"
            );
            interp.set_result_bytes(b"second error");
            interp.log_command_bytes(1, b"second command");
            assert!(!interp.host_refusal_pending(), "{profile}");
            let private = interp
                .exc
                .borrow()
                .native
                .info
                .as_ref()
                .expect("restored private error info")
                .as_ptr();
            assert_eq!(interp.var_get_at(b"::errorInfo", 0), Some(private));
            assert_eq!(
                obj_bytes(private),
                b"second error\n    while executing\n\"second command\""
            );
            assert!(interp.exc.borrow().already_logged, "{profile}");
        }
        assert_eq!(counters::finalize(), 0, "{profile}");
    }
}

#[test]
fn outermost_native_error_retains_private_objects_until_the_next_evaluation() {
    use super::super::Code;
    for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        counters::reset();
        {
            let mut interp = Interp::with_native_core(
                default_host(),
                profile_for_dialect(profile),
                NativeBootstrapInputs::default(),
            )
            .unwrap();
            assert_eq!(
                interp.eval_str(b"::oo::define::method outside {} {}"),
                Code::Error,
                "{profile}"
            );
            assert_eq!(interp.error_code(), b"TCL OO MONKEY_BUSINESS", "{profile}");
            let original = interp.native_private_error_object(false).unwrap();
            assert_eq!(
                interp.native_private_error_object(false).unwrap().as_ptr(),
                original.as_ptr()
            );
            // Tcl_GetReturnOptions(TCL_ERROR) observes the private error,
            // while a subsequent public Eval resets it before dispatch.
            assert_eq!(interp.eval_str(b"namespace current"), Code::Ok);
            assert_eq!(interp.error_code(), b"NONE", "{profile}");
            assert!(interp.native_private_error_object(false).is_none());
            assert_eq!(
                interp.eval_str(b"catch {::oo::define::method outside {} {}}"),
                Code::Ok
            );
            assert_eq!(interp.error_code(), b"NONE", "{profile}");
            assert!(interp.native_private_error_object(false).is_none());
        }
        assert_eq!(counters::finalize(), 0, "{profile}");
    }
}

// Native proof: naming.variable.error-globals-bootstrap-and-legacy-copy
// docs/design/analysis/name-resolution-proofs/variable.error-globals-bootstrap-and-legacy-copy.md
#[test]
fn hidden_error_reads_and_unsets_match_original_native_objects() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        counters::reset();
        {
            let mut interp = Interp::with_native_core(
                default_host(),
                profile_for_dialect(profile),
                NativeBootstrapInputs::default(),
            )
            .unwrap();
            let enabled = interp
                .native_invocation_dialect()
                .native_error_variable_protocol()
                .is_some();
            let original = obj::Owned::fresh(new_string(b"PRIVATE INFO"));
            let code = obj::Owned::fresh(new_string(b"PRIVATE CODE"));
            let public_info = obj::Owned::fresh(new_string(b"GUEST INFO"));
            interp
                .var_set(b"::errorInfo", public_info.as_ptr())
                .unwrap();
            drop(public_info);
            let public_code = obj::Owned::fresh(new_string(b"GUEST CODE"));
            interp
                .var_set(b"::errorCode", public_code.as_ptr())
                .unwrap();
            drop(public_code);
            interp.exc.borrow_mut().native = NativeErrorObjects {
                info: Some(original.clone()),
                info_len: b"PRIVATE INFO".len(),
                code: Some(code.clone()),
                legacy_copy: false,
                global_code_set: false,
                ..NativeErrorObjects::default()
            };
            assert_eq!(
                obj_bytes(interp.read_named_variable(b"::errorInfo").unwrap()),
                b"GUEST INFO"
            );
            interp.exc.borrow_mut().native.legacy_copy = true;
            let read = interp.read_named_variable(b"::errorInfo").unwrap();
            if enabled {
                assert_eq!(read, original.as_ptr());
            } else {
                assert_eq!(obj_bytes(read), b"GUEST INFO");
            }
            assert!(interp.var_unset(b"::errorInfo"));
            if enabled {
                let home = interp.trace_identity(b"::errorInfo");
                assert!(interp.native_error_variable_at(&home).is_some());
                assert_eq!(
                    interp.read_named_variable(b"::errorInfo").ok(),
                    Some(original.as_ptr())
                );
                assert!(interp.var_unset(b"::errorCode"));
                assert!(!interp.var_unset(b"::errorCode"));
                interp.exc.borrow_mut().native.code = None;
                assert_eq!(
                    obj_bytes(interp.read_named_variable(b"::errorCode").unwrap()),
                    b""
                );
                assert!(interp.exc.borrow().native.code.is_none());
                interp.exc.borrow_mut().native.code = Some(code.clone());
                interp.publish_native_error_objects();
                assert_eq!(interp.var_get_at(b"::errorCode", 0), Some(code.as_ptr()));
            } else {
                assert!(interp.var_get(b"::errorInfo").is_none());
            }
            *interp.exc.borrow_mut() = ExceptionState::default();
        }
        assert_eq!(counters::finalize(), 0, "{profile}");
    }
}
thread_local! {
    static RESET_CALLBACKS: std::cell::RefCell<Vec<Vec<u8>>> = const { std::cell::RefCell::new(Vec::new()) };
}
fn observe_reset(interp: &mut Interp, argv: &[*mut obj::TclObj]) -> super::super::Code {
    assert!(interp.exc.borrow().native.info.is_none());
    assert!(interp.exc.borrow().native.code.is_none());
    RESET_CALLBACKS.with(|seen| seen.borrow_mut().push(obj_bytes(argv[1])));
    super::super::Code::Ok
}

fn producer_reference_counts(profile: &str) -> (usize, usize) {
    let fixture = match profile {
            "tcl8.5" => include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_error_variables/8.5.19-producers.tsv"
            ),
            "tcl8.6" => include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_error_variables/8.6.18-producers.tsv"
            ),
            "tcl9.0" => include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_error_variables/9.0.4-producers.tsv"
            ),
            "tcl9.1" => include_str!(
                "../../../../../rust/tcl-registry/tests/data/native_error_variables/9.1.0-producers.tsv"
            ),
            _ => panic!("native producer fixture"),
        };
    let count = |label: &str| {
        let line = fixture
            .lines()
            .find(|line| line.starts_with(label))
            .unwrap();
        tcl_test_support::fixture::tsv_fields(line)
            .find_map(|field| field.strip_prefix("info_refs="))
            .unwrap()
            .parse::<usize>()
            .unwrap()
    };
    (count("explicit-error\t"), count("after-reset\t"))
}

// Native proof: naming.variable.error-producer-reset-callback-order
// docs/design/analysis/name-resolution-proofs/variable.error-producer-reset-callback-order.md
// Native proof: naming.variable.error-producer-script-trace-reset-order
// docs/design/analysis/name-resolution-proofs/variable.error-producer-script-trace-reset-order.md
#[test]
fn original_private_error_primaries_and_script_reset_match_native_producers() {
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let (before_reset, after_reset) = producer_reference_counts(profile);
        counters::reset();
        {
            let mut interp = Interp::with_native_core(
                default_host(),
                profile_for_dialect(profile),
                NativeBootstrapInputs::default(),
            )
            .unwrap();
            interp.replace_native_error_code(b"NONE");
            assert!(std::ptr::eq(
                obj::obj_type_ptr(interp.exc.borrow().native.code.as_ref().unwrap().as_ptr()),
                &crate::list::TCL_LIST_TYPE
            ));
            interp.exc.borrow_mut().native = NativeErrorObjects::default();
            interp.register_builtin(b"observe_native_reset", observe_reset);
            assert_eq!(interp.eval_str(b"trace add variable ::errorCode write observe_native_reset; trace add variable ::errorInfo write observe_native_reset"), super::super::Code::Ok);
            RESET_CALLBACKS.with(|seen| seen.borrow_mut().clear());
            let recipe = interp
                .native_invocation_dialect()
                .byte_array_string_recipe(None)
                .unwrap();
            let info =
                obj::Owned::fresh(crate::bytearray::new_byte_array(&[0xff, 0, b'a'], recipe));
            let code =
                obj::Owned::fresh(crate::bytearray::new_byte_array(&[0xff, 0, b'a'], recipe));
            let head = obj::Owned::fresh(new_string(b"error"));
            let message = obj::Owned::fresh(new_string(b"message"));
            let super::super::Command::Builtin(handler) =
                interp.resolve_dispatchable(GLOBAL, b"error").unwrap()
            else {
                panic!("native error handler");
            };
            assert_eq!(
                handler(
                    &mut interp,
                    &[
                        head.as_ptr(),
                        message.as_ptr(),
                        info.as_ptr(),
                        code.as_ptr()
                    ]
                ),
                super::super::Code::Error
            );
            assert_eq!(
                interp.exc.borrow().native.info.as_ref().unwrap().as_ptr(),
                info.as_ptr()
            );
            assert_eq!(
                interp.exc.borrow().native.code.as_ref().unwrap().as_ptr(),
                code.as_ptr()
            );
            assert!(std::ptr::eq(
                obj::obj_type_ptr(info.as_ptr()),
                &crate::bytearray::TCL_BYTE_ARRAY_TYPE
            ));
            assert!(std::ptr::eq(
                obj::obj_type_ptr(code.as_ptr()),
                &crate::list::TCL_LIST_TYPE
            ));
            assert_eq!(
                unsafe { (*info.as_ptr()).ref_count },
                before_reset as obj::TclSize,
                "{profile}"
            );
            assert_eq!(
                unsafe { (*code.as_ptr()).ref_count },
                before_reset as obj::TclSize,
                "{profile}"
            );
            interp.publish_native_error_objects();
            RESET_CALLBACKS.with(|seen| {
                assert_eq!(
                    *seen.borrow(),
                    [b"::errorInfo".to_vec(), b"::errorCode".to_vec()],
                    "{profile}"
                )
            });
            assert!(interp.exc.borrow().native.info.is_none());
            assert!(interp.exc.borrow().native.code.is_none());
            assert_eq!(interp.var_get_at(b"::errorInfo", 0), Some(info.as_ptr()));
            assert_eq!(interp.var_get_at(b"::errorCode", 0), Some(code.as_ptr()));
            assert_eq!(
                unsafe { (*info.as_ptr()).ref_count },
                after_reset as obj::TclSize,
                "{profile}"
            );
            assert_eq!(
                unsafe { (*code.as_ptr()).ref_count },
                after_reset as obj::TclSize,
                "{profile}"
            );
        }
        assert_eq!(counters::finalize(), 0, "{profile}");
    }
}

#[test]
fn hidden_trace_survives_script_removal_array_unset_and_root_teardown() {
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        counters::reset();
        {
            let mut interp = Interp::with_native_core(
                default_host(),
                profile_for_dialect(profile),
                NativeBootstrapInputs::default(),
            )
            .unwrap();
            assert_eq!(interp.eval_str(b"proc hook {args} {}; trace add variable ::errorInfo read hook; trace remove variable ::errorInfo read hook"), super::super::Code::Ok);
            assert!(interp
                .native_error_variable_at(&interp.trace_identity(b"::errorInfo"))
                .is_some());
            assert_eq!(
                interp.eval_str(b"array set ::errorInfo {k value}; unset ::errorInfo"),
                super::super::Code::Ok
            );
            assert!(interp
                .native_error_variable_at(&interp.trace_identity(b"::errorInfo"))
                .is_some());
            assert!(!interp.var_unset(b"::errorInfo"));
            interp.delete_namespace_by_id(GLOBAL);
            assert!(interp
                .native_error_variable_at(&interp.trace_identity(b"::errorInfo"))
                .is_some());
            assert!(interp
                .native_error_variable_at(&interp.trace_identity(b"::errorCode"))
                .is_some());
        }
        assert_eq!(counters::finalize(), 0, "{profile}");
    }
}

// Native proof: naming.variable.error-info-append-original-object
// docs/design/analysis/name-resolution-proofs/variable.error-info-append-original-object.md
#[test]
fn error_info_byte_append_preserves_seed_primary_until_nonempty_mutation() {
    counters::reset();
    {
        let mut interp = Interp::with_native_core(
            default_host(),
            profile_for_dialect("tcl9.0"),
            NativeBootstrapInputs::default(),
        )
        .unwrap();
        let recipe = interp
            .native_invocation_dialect()
            .byte_array_string_recipe(None)
            .unwrap();
        let original = obj::Owned::fresh(crate::bytearray::new_byte_array(b"seed", recipe));
        unsafe {
            interp.set_obj_result(original.as_ptr());
        }
        interp.exc.borrow_mut().info = Some(b"seed".to_vec());
        interp.exc.borrow_mut().native.info = Some(original.clone());
        interp.exc.borrow_mut().native.info_len = 4;
        interp.update_native_error_info();
        assert_eq!(
            interp.exc.borrow().native.info.as_ref().unwrap().as_ptr(),
            original.as_ptr()
        );
        assert!(std::ptr::eq(
            obj::obj_type_ptr(original.as_ptr()),
            &crate::bytearray::TCL_BYTE_ARRAY_TYPE
        ));
        interp.append_error_info_context(b"context");
        let pointer = interp.exc.borrow().native.info.as_ref().unwrap().as_ptr();
        assert_ne!(pointer, original.as_ptr());
        assert!(matches!(
            obj::native_object_snapshot(pointer).unwrap().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
        ));
        assert_eq!(obj_bytes(pointer), b"seed\n    context");
        assert!(std::ptr::eq(
            obj::obj_type_ptr(original.as_ptr()),
            &crate::bytearray::TCL_BYTE_ARRAY_TYPE
        ));
    }
    assert_eq!(counters::finalize(), 0);
}

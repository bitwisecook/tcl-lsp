// SPDX-License-Identifier: AGPL-3.0-or-later
//! Real compiled Next List conversion and original body cleanup controls.
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use std::cell::Cell;
use tcl_dialect::TclVersion;

#[test]
fn original_next_concat_retains_first_cause_through_reached_operand_cleanup() {

    for mode in [0, 1, 3] {
        let mut interpreter = fixture::native("tcl9.1");
        let _current = fixture::install(&mut interpreter, mode);
        interpreter
            .frames
            .borrow_mut()
            .push(crate::namespace::GLOBAL);
        let operation = BodyObjectOperation::capture(&interpreter).unwrap();
        let target = fixture::string(b"next");
        let source = fixture::original(b"VALUE");
        let source_pointer = source.as_ptr();
        let source_allocation = obj::NativeObjectLifetime::retain(source_pointer);
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let reached = interpreter.body_next_concatenate(
            &operation,
            target,
            source,
            NativeStringProtocol::C(TclVersion::V9_1),
        );
        assert_eq!(fixture::FREES.with(Cell::get), 1);
        assert_eq!(fixture::REFS.with(Cell::get), 1);
        assert!(obj::check_native_liveness(source_pointer).is_err());
        if mode == 0 {
            let combined = reached.unwrap();
            let members = operation
                .members(
                    &interpreter,
                    combined.as_ptr(),
                    NativeStringProtocol::C(TclVersion::V9_1),
                )
                .unwrap();
            assert_eq!(members.len(), 2);
            assert_eq!(
                operation.bytes(&interpreter, members[0]).unwrap().as_ref(),
                b"next"
            );
            assert_eq!(
                operation.bytes(&interpreter, members[1]).unwrap().as_ref(),
                b"VALUE"
            );
            drop(combined);
            operation.check(&interpreter).unwrap();
        } else {
            let first = match reached {
                Ok(_) => panic!("withdrawn original concat"),
                Err(first) => first,
            };
            assert_eq!(
                first.native_execution_refusal(),
                interpreter.native_execution_refusal().as_ref()
            );
            if mode == 1 {
                fixture::host(first.native_execution_refusal().unwrap());
            } else {
                assert_eq!(
                    first,
                    tcl_cmd_core::CmdError::from(ValueError::CommandProtocolUnavailable(
                        "original compiled List frame or temporary",
                    ))
                );
            }
            assert!(operation.check(&interpreter).is_err());
        }
        assert_eq!(interpreter.get_obj_result(), prior);
        drop(source_allocation);
    }
}
#[test]
fn original_next_operand_error_finishes_earliest_body_cleanup_without_later_getter() {

    let mut interpreter = fixture::native("tcl9.1");
    let _current = fixture::install(&mut interpreter, 1);
    let operation = BodyObjectOperation::capture(&interpreter).unwrap();
    let target = fixture::string(b"next");
    let source = fixture::original(b"VALUE");
    let later = fixture::later();
    let reached = interpreter.body_next_concatenate(
        &operation,
        target,
        source,
        NativeStringProtocol::C(TclVersion::V9_1),
    );
    let first = match reached {
        Ok(_) => panic!("original concat Host refusal"),
        Err(first) => first,
    };
    assert!(operation.bytes(&interpreter, later.as_ptr()).is_err());
    fixture::host(first.native_execution_refusal().unwrap());
    assert_eq!(fixture::GETTERS.with(Cell::get), 0);
    assert_eq!(fixture::FREES.with(Cell::get), 1);
    assert_eq!(fixture::REFS.with(Cell::get), 1);
    assert!(operation.check(&interpreter).is_err());
}
#[test]
fn original_compiled_next_and_nextto_keep_selected_list_and_public_results() {

    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interpreter = fixture::native(dialect);
        let scope =
            crate::interp::native_operation_currency::NativeOperationScope::enter(&interpreter)
                .unwrap();
        assert_eq!(
            interpreter.eval_str(
                br"oo::class create Base {method m {args} {return $args}; method to {args} {return $args}}
            oo::class create Leaf {superclass Base; method m {args} {next {*}$args};
                method to {args} {nextto Base {*}$args}}
            Leaf create subject
            list [subject m A B] [subject to C D]"
            ),
            Code::Ok,
            "{dialect}: {:?}",
            interpreter.result_bytes()
        );
        assert_eq!(interpreter.result_bytes(), b"{A B} {C D}");
        scope.currency().ensure_current_or_refuse().unwrap();
    }
}

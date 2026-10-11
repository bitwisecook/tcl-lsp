// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual ArrayFor ListLength conversion before name/search selection.
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use std::cell::Cell;
#[test]
fn original_array_for_length_retirement_refuses_before_array_and_script_getters() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The original first converter is Length, not the later ListCopy worker.
    for mode in [1, 2] {
        let mut interpreter = fixture::native("tcl9.0");
        let _current = fixture::install(&mut interpreter, mode);
        let command = fixture::string(b"array");
        let subcommand = fixture::string(b"for");
        let names = fixture::original(b"key value");
        let array = fixture::later();
        let body = fixture::later();
        let argv = [
            command.as_ptr(),
            subcommand.as_ptr(),
            names.as_ptr(),
            array.as_ptr(),
            body.as_ptr(),
        ];
        let arguments = OriginalArguments::enter(&interpreter, &argv).unwrap();
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let before = crate::counters::live_objs();
        assert_eq!(array_cmd(&mut interpreter, &argv), Code::Error);
        let first = interpreter.native_execution_refusal().unwrap();
        if mode == 1 {
            fixture::host(&first);
        } else {
            assert!(matches!(
                first,
                tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "stale entered native operation"
                    )
                )
            ));
        }
        assert!(arguments.check_for(&interpreter).is_err());
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(fixture::GETTERS.with(Cell::get), 0);
        assert_eq!(fixture::FREES.with(Cell::get), 1);
        assert_eq!(fixture::REFS.with(Cell::get), 1);
        assert_eq!(unsafe { (*names.as_ptr()).ref_count }, 1);
        assert!(!core::ptr::eq(
            obj::obj_type_ptr(names.as_ptr()),
            &crate::list::TCL_LIST_TYPE
        ));
        assert_eq!(crate::counters::live_objs(), before);
    }
}

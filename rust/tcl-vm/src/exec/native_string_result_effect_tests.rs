// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual selected Native source/header correspondence for the effect receipt.

use tcl_syntax::value::ValueOps;

#[test]
fn actual_native_template_append_preserves_cow_and_original_header_alternatives() {
    for engine in ["tcl8.6", "tcl9.0"] {
        let mut vm =
            crate::native_fixture::interpreter(tcl_dialect::DialectProfile::find(engine).unwrap());
        let completion = vm.try_eval_source(r#"set name world; set greeting "hello, $name"; set alias $greeting; append greeting "!"; string length $greeting"#).unwrap();
        assert_eq!(completion.code, tcl_runtime_api::Code::Ok, "{engine}");
        assert!(
            vm.execution_refusal.is_none(),
            "{engine}: {:?}",
            vm.execution_refusal
        );
        assert_eq!(
            vm.native_string_bytes(&completion.result).unwrap().as_ref(),
            b"13"
        );
        let greeting = vm.get_var("greeting").unwrap();
        let alias = vm.get_var("alias").unwrap();
        assert!(!greeting.is_same_object(&alias), "actual COW {engine}");
        assert_eq!(
            vm.native_string_bytes(&greeting).unwrap().as_ref(),
            b"hello, world!"
        );
        assert_eq!(
            vm.native_string_bytes(&alias).unwrap().as_ref(),
            b"hello, world"
        );
        assert!(matches!(
            greeting.native_object_type_name(),
            "none" | "string"
        ));
        let completion = vm
            .try_eval_source(r#"set donor original; set retained "$donor"; append retained """#)
            .unwrap();
        assert_eq!(completion.code, tcl_runtime_api::Code::Ok);
        assert!(vm.execution_refusal.is_none());
        let donor = vm.get_var("donor").unwrap();
        let retained = vm.get_var("retained").unwrap();
        assert_eq!(
            vm.native_string_bytes(&donor).unwrap().as_ref(),
            b"original"
        );
        assert_eq!(
            vm.native_string_bytes(&retained).unwrap().as_ref(),
            b"original"
        );
        // Empty append may preserve or duplicate its receiver under actual COW;
        // both alternatives must remain ordinary, without a fresh-header claim.
        assert!(matches!(
            retained.native_object_type_name(),
            "none" | "string"
        ));
    }
}

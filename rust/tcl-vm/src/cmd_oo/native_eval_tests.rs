// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original-object evaluation by the selected stock object eval adapter.

use super::*;

#[test]
fn object_eval_retains_original_script_and_concat_children() {
    // Source proof: naming.tcloo.object-eval-original-script
    // docs/design/analysis/name-resolution-proofs/object-eval-original-script.md
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for concatenate in [false, true] {
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            let mut vm = crate::native_fixture::core(profile);
            assert_eq!(
                vm.invoke_command(
                    "::oo::object",
                    &[Value::string("create"), Value::string("O")]
                )
                .code,
                Code::Ok
            );
            let object = object_key(&mut vm, &Value::string("O")).unwrap();
            let value = Value::new_native_string_bytes(b"VALUE\xff\0TAIL".as_slice());
            let arguments = if concatenate {
                vec![
                    Value::list(vec![Value::string("return")]),
                    Value::list(vec![value.clone()]),
                ]
            } else {
                vec![Value::list(vec![Value::string("return"), value.clone()])]
            };
            let completion = builtin_method(
                &mut vm,
                object,
                "eval",
                &arguments,
                &Value::string("O"),
                &Value::string("eval"),
                false,
            );
            assert_eq!(completion.code, Code::Return, "{engine}/{concatenate}");
            assert!(
                completion.result.is_same_object(&value),
                "{engine}/{concatenate}"
            );
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                b"VALUE\xff\0TAIL"
            );
            assert!(
                arguments
                    .iter()
                    .all(|argument| argument.resident_string_bytes().is_none()),
                "{engine}/{concatenate}"
            );
        }
    }
}

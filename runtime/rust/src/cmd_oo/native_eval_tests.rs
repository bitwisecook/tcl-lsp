// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original-object evaluation by the selected stock object eval adapter.

use super::*;

#[test]
fn object_eval_retains_original_script_and_concat_children() {
    // Source proof: naming.tcloo.object-eval-original-script
    // docs/design/analysis/name-resolution-proofs/object-eval-original-script.md
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for concatenate in [false, true] {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            assert_eq!(interp.eval_str(b"::oo::object create O"), Code::Ok);
            let object = interp.oo_resolve_object(b"O");
            let command = obj::Owned::fresh(obj::new_string_bytes(b"return"));
            let value = obj::Owned::fresh(obj::new_string_bytes(b"VALUE\xff\0TAIL"));
            let arguments = if concatenate {
                vec![
                    obj::Owned::fresh(crate::list::new_list_obj(&[command.as_ptr()])),
                    obj::Owned::fresh(crate::list::new_list_obj(&[value.as_ptr()])),
                ]
            } else {
                vec![obj::Owned::fresh(crate::list::new_list_obj(&[
                    command.as_ptr(),
                    value.as_ptr(),
                ]))]
            };
            let pointers = arguments.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
            assert_eq!(
                interp.oo_builtin_method(object, b"eval", &pointers, false, None),
                Some(Code::Return),
                "{engine}/{concatenate}"
            );
            assert_eq!(
                interp.result_obj(),
                value.as_ptr(),
                "{engine}/{concatenate}"
            );
            assert_eq!(interp.result_bytes(), b"VALUE\xff\0TAIL");
            assert!(
                arguments
                    .iter()
                    .all(|argument| !obj::has_string_rep(argument.as_ptr())),
                "{engine}/{concatenate}"
            );
        }
    }
}

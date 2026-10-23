// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original Jim namespace getter boundaries through actual object-vector apply.

use super::*;

fn unhex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn original_type(value: *mut TclObj) -> Vec<u8> {
    let descriptor = obj::obj_type_ptr(value);
    assert!(!descriptor.is_null());
    // SAFETY: each actual original integer has its live static descriptor here.
    unsafe {
        std::ffi::CStr::from_ptr((*descriptor).name)
            .to_bytes()
            .to_vec()
    }
}

#[test]
fn original_first_substitutions_get_namespace_only_when_command_lookup_is_reached() {
    let rows = include_str!(
        "../../../../../rust/tcl-registry/tests/data/native_jim_first_substitution/jim/stdout.tsv"
    );
    for (label, body, formal, argument) in [
        ("MISSING_LOCAL_HEAD", "$missing", "", None),
        ("MISSING_ROOT_HEAD", "${::missing}", "", None),
        ("MISSING_LOCAL_ARGUMENT", "return $missing", "", None),
        ("MISSING_ROOT_ARGUMENT", "::return ${::missing}", "", None),
        ("LOCAL_ARGUMENT", "return $arg", "arg", Some("VALUE")),
        ("ROOT_ARGUMENT", "::return ${::global}", "", None),
        ("COMPUTED_LOCAL_HEAD", "$arg VALUE", "arg", Some("::return")),
        ("BRACKET_MISSING_ARGUMENT", "return [set missing]", "", None),
        ("COMMENT_CONTROL", "# only a comment", "", None),
        ("INVALID_CONTROL", "{", "", None),
    ] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jimtcl"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(interp.eval_str(b"set ::global VALUE"), Code::Ok);
        let context = interp.native_jim_object_context().unwrap();
        let namespace = Owned::fresh(obj::new_wide_int_obj(42));
        crate::native_source::bind_context(namespace.as_ptr(), &context).unwrap();
        let parameters = Owned::fresh(obj::new_string_bytes(formal.as_bytes()));
        let body = Owned::fresh(obj::new_string_bytes(body.as_bytes()));
        let protocol = interp
            .native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        let lambda = Owned::fresh(crate::list::new_list_obj_native(
            &[parameters.as_ptr(), body.as_ptr(), namespace.as_ptr()],
            protocol,
        ));
        let head = Owned::fresh(obj::new_string_bytes(b"apply"));
        let argument = argument.map(|bytes| Owned::fresh(obj::new_string_bytes(bytes.as_bytes())));
        let mut argv = vec![head.as_ptr(), lambda.as_ptr()];
        if let Some(argument) = &argument {
            argv.push(argument.as_ptr());
        }
        let expected = |kind: &str| {
            let selected: Vec<_> = rows
                .lines()
                .filter(|line| line.starts_with(&format!("{kind}|{label}|")))
                .collect();
            assert_eq!(selected.len(), 1);
            selected[0].split('|').collect::<Vec<_>>()
        };
        assert_eq!(
            obj::has_string_rep(namespace.as_ptr()),
            expected("BEFORE")[2] == "1"
        );
        let namespace_type = obj::obj_type_ptr(namespace.as_ptr());
        assert_eq!(
            original_type(namespace.as_ptr()),
            expected("BEFORE")[3].as_bytes()
        );
        let code = interp.dispatch(&argv);
        assert_eq!(obj::obj_type_ptr(namespace.as_ptr()), namespace_type);
        assert_eq!(
            original_type(namespace.as_ptr()),
            expected("AFTER")[3].as_bytes()
        );
        assert_eq!(
            obj::has_string_rep(namespace.as_ptr()),
            expected("AFTER")[2] == "1",
            "{label}/namespace getter"
        );
        assert_eq!(
            code.as_int(),
            expected("RESULT")[2].parse::<i64>().unwrap(),
            "{label}/code"
        );
        assert_eq!(
            interp.result_bytes(),
            unhex(expected("RESULT")[3]),
            "{label}/result"
        );
        assert_eq!(
            interp
                .native_object_string_bytes(namespace.as_ptr())
                .unwrap()
                .as_ref(),
            unhex(expected("GETTER")[3]),
            "{label}/later getter"
        );
        assert!(
            interp.native_execution_refusal().is_none(),
            "{label}/Host refusal"
        );
        assert_eq!(interp.current_ns(), crate::namespace::GLOBAL);
    }
}

#[test]
fn original_frame_arguments_stay_stringless_and_keep_their_actual_list_identity() {
    let rows = include_str!(
        "../../../../../rust/tcl-registry/tests/data/native_jim_apply_frame_argv/jim/stdout.tsv"
    );
    for (label, formal, body, level) in [
        ("COMMENT", "arg", "# only a comment", false),
        ("INVALID", "arg", "{", false),
        ("RETURN", "arg", "return $arg", false),
        ("LEVEL", "arg", "info level 0", true),
        ("MISSING", "arg", "return $missing", false),
        ("WRONG_ARITY", "", "# only a comment", false),
        ("EMPTY", "arg", "", false),
    ] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jimtcl"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let context = interp.native_jim_object_context().unwrap();
        let namespace = Owned::fresh(obj::new_wide_int_obj(42));
        let argument = Owned::fresh(obj::new_wide_int_obj(84));
        crate::native_source::bind_context(namespace.as_ptr(), &context).unwrap();
        crate::native_source::bind_context(argument.as_ptr(), &context).unwrap();
        let parameters = Owned::fresh(obj::new_string_bytes(formal.as_bytes()));
        let body = Owned::fresh(obj::new_string_bytes(body.as_bytes()));
        let protocol = interp
            .native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        let lambda = Owned::fresh(crate::list::new_list_obj_native(
            &[parameters.as_ptr(), body.as_ptr(), namespace.as_ptr()],
            protocol,
        ));
        let head = Owned::fresh(obj::new_string_bytes(b"apply"));
        let expected = |kind: &str| {
            let selected: Vec<_> = rows
                .lines()
                .filter(|line| line.starts_with(&format!("{kind}|{label}|")))
                .collect();
            assert_eq!(selected.len(), 1);
            selected[0].split('|').collect::<Vec<_>>()
        };
        assert_eq!(
            obj::has_string_rep(namespace.as_ptr()),
            expected("NAMESPACE_BEFORE")[2] == "1"
        );
        assert_eq!(
            obj::has_string_rep(argument.as_ptr()),
            expected("ARGUMENT_BEFORE")[2] == "1"
        );
        let namespace_type = obj::obj_type_ptr(namespace.as_ptr());
        let argument_type = obj::obj_type_ptr(argument.as_ptr());
        assert_eq!(
            original_type(namespace.as_ptr()),
            expected("NAMESPACE_BEFORE")[3].as_bytes()
        );
        assert_eq!(
            original_type(argument.as_ptr()),
            expected("ARGUMENT_BEFORE")[3].as_bytes()
        );
        let code = interp.dispatch(&[head.as_ptr(), lambda.as_ptr(), argument.as_ptr()]);
        assert_eq!(obj::obj_type_ptr(namespace.as_ptr()), namespace_type);
        assert_eq!(obj::obj_type_ptr(argument.as_ptr()), argument_type);
        assert_eq!(
            original_type(namespace.as_ptr()),
            expected("NAMESPACE_AFTER")[3].as_bytes()
        );
        assert_eq!(
            original_type(argument.as_ptr()),
            expected("ARGUMENT_AFTER")[3].as_bytes()
        );
        assert_eq!(
            obj::has_string_rep(namespace.as_ptr()),
            expected("NAMESPACE_AFTER")[2] == "1",
            "{label}/namespace getter"
        );
        assert_eq!(
            obj::has_string_rep(argument.as_ptr()),
            expected("ARGUMENT_AFTER")[2] == "1",
            "{label}/argument getter"
        );
        let result = interp.get_obj_result();
        let selected = if level && code == Code::Ok {
            ValueOps::list_elements(&mut interp, &result).unwrap()[1]
        } else {
            result
        };
        assert_eq!(
            selected == argument.as_ptr(),
            unhex(expected("RESULT_IDENTITY")[3]) == b"1",
            "{label}/original argument identity"
        );
        assert_eq!(
            code.as_int(),
            expected("RESULT")[2].parse::<i64>().unwrap(),
            "{label}/code"
        );
        assert_eq!(
            interp.result_bytes(),
            unhex(expected("RESULT")[3]),
            "{label}/result"
        );
        assert_eq!(
            interp
                .native_object_string_bytes(namespace.as_ptr())
                .unwrap()
                .as_ref(),
            unhex(expected("NAMESPACE_GETTER")[3]),
            "{label}/namespace later getter"
        );
        assert_eq!(
            interp
                .native_object_string_bytes(argument.as_ptr())
                .unwrap()
                .as_ref(),
            unhex(expected("ARGUMENT_GETTER")[3]),
            "{label}/argument later getter"
        );
        assert!(
            interp.native_execution_refusal().is_none(),
            "{label}/Host refusal"
        );
        assert_eq!(interp.current_ns(), crate::namespace::GLOBAL);
    }
}

// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

fn actual(engine: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(engine),
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .unwrap()
}

#[test]
fn original_procedure_colon_names_match_all_18_native_home_and_error_controls() {
    let rows =
        include_str!("../../../../rust/tcl-syntax/tests/data/native_procedure_name/rows.txt");
    let decode = |text: &str| {
        text.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect::<Vec<_>>()
    };
    let mut count = 0;
    for row in rows.lines() {
        let fields: Vec<_> = row.split('\t').collect();
        let mut interp = actual(fields[0]);
        let code = interp.eval_str(&decode(fields[4]));
        assert_eq!(
            code.as_int(),
            fields[2].parse::<i64>().unwrap(),
            "{}/{}",
            fields[0],
            fields[1]
        );
        assert_eq!(
            interp.result_bytes(),
            decode(fields[3]),
            "{}/{}",
            fields[0],
            fields[1]
        );
        count += 1;
    }
    assert_eq!(count, 18);
}

#[test]
fn legacy_name_rejection_precedes_original_formal_default_and_body_access() {
    for engine in ["tcl8.4", "tcl8.5"] {
        let mut interp = actual(engine);
        let namespace = interp
            .namespaces_mut()
            .ensure_namespace(crate::namespace::GLOBAL, b":");
        interp.set_current_ns(namespace);
        let member = Owned::fresh(crate::obj::new_string_bytes(b"untouched"));
        let default = Owned::fresh(interp.new_list_object(&[member.as_ptr()]));
        let name = Owned::fresh(crate::obj::new_string_bytes(b"x"));
        let formal = Owned::fresh(interp.new_list_object(&[name.as_ptr(), default.as_ptr()]));
        let parameters = Owned::fresh(interp.new_list_object(&[formal.as_ptr()]));
        let body = Owned::fresh(interp.new_list_object(&[member.as_ptr()]));
        let head = Owned::fresh(crate::obj::new_string_bytes(b"proc"));
        let name = Owned::fresh(crate::obj::new_string_bytes(b":"));
        for original in [&parameters, &default, &body] {
            assert!(std::ptr::eq(
                crate::obj::obj_type_ptr(original.as_ptr()),
                &crate::list::TCL_LIST_TYPE
            ));
            assert!(!crate::obj::has_string_rep(original.as_ptr()));
        }
        assert_eq!(
            proc_cmd(
                &mut interp,
                &[
                    head.as_ptr(),
                    name.as_ptr(),
                    parameters.as_ptr(),
                    body.as_ptr()
                ]
            ),
            Code::Error
        );
        for original in [&parameters, &default, &body] {
            assert!(
                std::ptr::eq(
                    crate::obj::obj_type_ptr(original.as_ptr()),
                    &crate::list::TCL_LIST_TYPE
                ),
                "{engine}"
            );
            assert!(!crate::obj::has_string_rep(original.as_ptr()), "{engine}");
        }
    }
}

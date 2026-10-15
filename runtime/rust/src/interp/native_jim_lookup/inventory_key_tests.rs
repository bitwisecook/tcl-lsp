// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;

const ORIGINAL: &str = include_str!(
    "../../../tests/data/native_jim_command_inventory/original-key-objects-20261015.tsv"
);

fn references(original: *mut TclObj) -> i64 {
    // SAFETY: each observer owns an actual native reference to this header.
    unsafe { (*original).ref_count as i64 }
}

fn primary(original: *mut TclObj) -> Vec<u8> {
    let descriptor = obj::obj_type_ptr(original);
    if descriptor.is_null() {
        b"none".to_vec()
    } else {
        // SAFETY: every actual live type descriptor retains its static C name.
        unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }
            .to_bytes()
            .to_vec()
    }
}

#[test]
fn original_jim_command_inventory_keeps_key_identity_lazy_headers_and_native_references() {
    assert_eq!(ORIGINAL.lines().count(), 24);
    for row in ORIGINAL.lines() {
        let fields = row.split('|').collect::<Vec<_>>();
        assert_eq!(fields.len(), 14);
        let kind = fields[1];
        let all = fields[2] == "1";
        let mode = fields[3].parse::<usize>().unwrap();
        let mut interp = Interp::with_native_core(
            super::super::default_host(),
            crate::environment::profile_for_dialect("jimtcl"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(
            interp.eval_str(b"proc 123 {} {}; proc 456 {} {}; alias 789 123"),
            Code::Ok
        );
        let name = if kind == "aliases" { b"789" } else { b"123" };
        let candidates = interp
            .namespaces()
            .original_jim_command_inventory_keys(
                tcl_runtime_api::NativeJimCommandInventoryKind::Commands,
                &|_, _| true,
            )
            .unwrap();
        let key = candidates.iter().find(|key| key.report == name).unwrap();
        let original = Owned::retain(key.original().unwrap());
        assert_eq!(
            crate::typed_value::native_wide_int(
                original.as_ptr(),
                interp.native_invocation_dialect()
            ),
            Ok(if kind == "aliases" { 789 } else { 123 })
        );
        obj::invalidate_string(original.as_ptr());
        assert_eq!(
            u8::from(obj::has_string_rep(original.as_ptr())).to_string(),
            fields[5],
            "{row}"
        );
        assert_eq!(
            references(original.as_ptr()).to_string(),
            fields[7],
            "{row}"
        );
        assert_eq!(primary(original.as_ptr()), fields[6].as_bytes(), "{row}");
        let mut words = vec![
            Owned::fresh(obj::new_string_bytes(b"info")),
            Owned::fresh(obj::new_string_bytes(b"-nons")),
            Owned::fresh(obj::new_string_bytes(kind.as_bytes())),
        ];
        if all {
            words.push(Owned::fresh(obj::new_string_bytes(b"-all")));
        }
        if mode != 0 {
            let pattern = match mode {
                1 => name.as_slice(),
                2 => b"456",
                _ if kind == "aliases" => b"7*",
                _ => b"1*",
            };
            words.push(Owned::fresh(obj::new_string_bytes(pattern)));
        }
        let argv = words.iter().map(Owned::as_ptr).collect::<Vec<_>>();
        let code = interp.dispatch(&argv);
        assert_eq!(
            code.as_int().to_string(),
            fields[4],
            "{row}: {:?}",
            interp.native_execution_refusal()
        );
        assert_eq!(
            u8::from(obj::has_string_rep(original.as_ptr())).to_string(),
            fields[8],
            "{row}"
        );
        assert_eq!(
            references(original.as_ptr()).to_string(),
            fields[10],
            "{row}"
        );
        assert_eq!(primary(original.as_ptr()), fields[9].as_bytes(), "{row}");
        let backing = crate::list::native_list_backing(interp.result_obj()).unwrap();
        let found = backing
            .elements()
            .unwrap()
            .iter()
            .filter(|&&member| member == original.as_ptr())
            .count();
        assert_eq!(found.to_string(), fields[11], "{row}");
        // The original SDK's complete registered roster is a separate premise.
        // These cells compare the selected key's actual ownership and getters.
        drop(backing);
        interp.set_result_bytes(b"");
        assert_eq!(
            references(original.as_ptr()).to_string(),
            fields[13],
            "{row}"
        );
        assert!(interp.native_execution_refusal().is_none(), "{row}");
    }
}

// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::obj;
const SETUP: &[u8] =
    include_bytes!("../../../tests/data/native_jim_namespace_info_projection/setup.tcl");
const ORIGINAL: &str = include_str!(
    "../../../tests/data/native_jim_namespace_info_projection/observations-20261015.tsv"
);
fn jim() -> Interp {
    Interp::with_native_core(
        super::super::default_host(),
        crate::environment::profile_for_dialect("jimtcl"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn primary(original: *mut TclObj) -> Vec<u8> {
    let ty = obj::obj_type_ptr(original);
    if ty.is_null() {
        b"none".to_vec()
    } else {
        // SAFETY: the entered result List owns this live original header.
        unsafe { std::ffi::CStr::from_ptr((*ty).name).to_bytes().to_vec() }
    }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
#[test]
fn actual_namespace_info_relay_preserves_original_global_projection_and_empty_headers() {
    for (label, input) in [
        (
            "GENUINE",
            b"{namespace info} commands ::RelayInfoSource::*".as_slice(),
        ),
        (
            "EMPTY",
            b"{namespace info} commands ::MissingInfoSource::*".as_slice(),
        ),
    ] {
        let mut actual = jim();
        assert_eq!(
            actual.eval_str(SETUP),
            Code::Ok,
            "{:?}",
            actual.native_execution_refusal()
        );
        let code = actual.eval_str(input);
        let result = actual.result_obj();
        // SAFETY: the interpreter's result role owns this exact live header.
        let refs = unsafe { (*result).ref_count };
        let header = format!(
            "HEADER|{label}|{}|{}|{}|{refs}",
            code.as_int(),
            String::from_utf8(primary(result)).unwrap(),
            u8::from(obj::has_string_rep(result))
        );
        assert!(
            ORIGINAL.lines().any(|line| line == header),
            "{header}; refusal={:?}",
            actual.native_execution_refusal()
        );
        let backing = crate::list::native_list_backing(result).unwrap();
        let members = backing.elements().unwrap();
        assert!(ORIGINAL
            .lines()
            .any(|line| line == format!("LIST|{label}|{}", members.len())));
        for (index, &member) in members.iter().enumerate() {
            let kind = String::from_utf8(primary(member)).unwrap();
            let resident = u8::from(obj::has_string_rep(member));
            // SAFETY: the actual retained result List owns this member.
            let refs = unsafe { (*member).ref_count };
            let bytes = actual.native_object_string_bytes(member).unwrap();
            let row = format!(
                "MEMBER|{label}|{index}|{kind}|{resident}|{refs}|{}",
                hex(&bytes)
            );
            assert!(ORIGINAL.lines().any(|line| line == row), "{row}");
        }
        drop(members);
        drop(backing);
        let result = format!("RESULT|{label}|{}", hex(&actual.result_bytes()));
        assert!(ORIGINAL.lines().any(|line| line == result), "{result}");
        assert!(actual.native_execution_refusal().is_none());
    }
}

#[test]
fn mutable_lmap_and_cat_cannot_donate_missing_projection_completion() {
    for (label, replacement, reason) in [
        ("RETURN_INPUT_LMAP", b"proc lmap {var values body} {return $values}".as_slice(), "original NamespaceInfo completed transformation"),
        ("FORGED_CAT", b"rename string actual_string; proc string args {if {[lindex $args 0] eq {cat}} {return FORGED}; tailcall actual_string {*}$args}".as_slice(), "original NamespaceInfo native cat completion"),
    ] {
        assert!(ORIGINAL.lines().any(|line| line.starts_with(&format!("HEADER|{label}|0|"))));
        let mut actual = jim();
        assert_eq!(actual.eval_str(SETUP), Code::Ok);
        assert_eq!(actual.eval_str(replacement), Code::Ok);
        assert_eq!(actual.eval_str(b"{namespace info} commands ::RelayInfoSource::*"), Code::Error);
        assert_eq!(actual.native_access_refusal(), Some(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(reason)));
    }
}

fn references(original: *mut TclObj) -> obj::TclSize {
    // SAFETY: these observations run while the actual result or dictionary owns the header.
    unsafe { (*original).ref_count }
}

fn assert_local_observation(
    actual: &mut Interp,
    label: &str,
    rows: &str,
    original: Option<*mut TclObj>,
) {
    let result = actual.result_obj();
    let header = format!(
        "HEADER|{label}|0|{}|{}|{}",
        String::from_utf8(primary(result)).unwrap(),
        u8::from(obj::has_string_rep(result)),
        references(result)
    );
    assert!(
        rows.lines().any(|line| line == header),
        "{header}; refusal={:?}",
        actual.native_execution_refusal()
    );
    let backing = crate::list::native_list_backing(result).unwrap();
    let members = backing.elements().unwrap();
    assert!(rows
        .lines()
        .any(|line| line == format!("LIST|{label}|{}", members.len())));
    for (index, &member) in members.iter().enumerate() {
        let kind = String::from_utf8(primary(member)).unwrap();
        let resident = u8::from(obj::has_string_rep(member));
        let refs = references(member);
        let same = original.map_or(-1, |original| i32::from(original == member));
        let bytes = actual.native_object_string_bytes(member).unwrap();
        let row = format!(
            "MEMBER|{label}|{index}|{kind}|{resident}|{refs}|{same}|{}",
            hex(&bytes)
        );
        assert!(rows.lines().any(|line| line == row), "{row}");
    }
    drop(members);
    drop(backing);
    let row = format!("RESULT|{label}|{}", hex(&actual.result_bytes()));
    assert!(rows.lines().any(|line| line == row), "{row}");
    assert!(actual.native_execution_refusal().is_none());
}

#[test]
fn original_local_namespace_projection_preserves_first_keys_ascii_construction_and_empty_results() {
    for (setup, rows) in [
        (include_bytes!("../../../tests/data/native_jim_namespace_info_local_dictionary/setup-01.tcl").as_slice(), include_str!("../../../tests/data/native_jim_namespace_info_local_dictionary/observations-20261015-01.tsv")),
        (include_bytes!("../../../tests/data/native_jim_namespace_info_local_dictionary/setup-02.tcl").as_slice(), include_str!("../../../tests/data/native_jim_namespace_info_local_dictionary/observations-20261015-02.tsv")),
    ] {
        for (label, input) in [
            ("LOCAL_DUPLICATE", b"namespace eval LocalReceiptScope {{namespace info} commands LocalReceiptP*}".as_slice()),
            ("LOCAL_EMPTY", b"namespace eval LocalReceiptScope {{namespace info} commands MissingReceiptP*}".as_slice()),
        ] {
            let mut actual = jim();
            assert_eq!(actual.eval_str(setup), Code::Ok);
            assert_eq!(actual.eval_str(input), Code::Ok, "{:?}", actual.native_execution_refusal());
            assert_local_observation(&mut actual, label, rows, None);
        }
    }
}

#[test]
fn original_dictionary_keys_skip_unreached_key_getters_and_retain_actual_headers() {
    let rows = include_str!("../../../tests/data/native_jim_namespace_info_local_dictionary/observations-20261015-02.tsv");
    for (label, input) in [
        ("KEYS_NO_PATTERN", b"dict keys $d".as_slice()),
        ("KEYS_PATTERN", b"dict keys $d *".as_slice()),
    ] {
        let mut actual = jim();
        let key = actual.new_list_object(&[obj::new_wide_int_obj(123)]);
        let dict = crate::dict::new_dict_obj_native(
            &[(key, obj::new_wide_int_obj(1))],
            None,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )
        .unwrap();
        let name = obj::Owned::fresh(obj::new_string_bytes(b"d"));
        assert_eq!(
            actual.assign_original_named_variable(name.as_ptr(), dict),
            Ok(())
        );
        obj::invalidate_string(key);
        let row = format!(
            "KEY_BEFORE|{label}|{}|{}|{}",
            String::from_utf8(primary(key)).unwrap(),
            u8::from(obj::has_string_rep(key)),
            references(key)
        );
        assert!(rows.lines().any(|line| line == row), "{row}");
        assert_eq!(actual.eval_str(input), Code::Ok);
        assert_local_observation(&mut actual, label, rows, Some(key));
    }
}

#[test]
fn dictionary_mutation_observation_rejects_same_value_and_revert_writes() {
    let key = obj::Owned::fresh(obj::new_string_bytes(b"key"));
    let one = obj::Owned::fresh(obj::new_wide_int_obj(1));
    let two = obj::Owned::fresh(obj::new_wide_int_obj(2));
    let dict = obj::Owned::fresh(
        crate::dict::new_dict_obj_native(
            &[(key.as_ptr(), one.as_ptr())],
            None,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )
        .unwrap(),
    );
    let before = crate::dict::OriginalDictionaryMutation::capture(dict.as_ptr()).unwrap();
    before.check(dict.as_ptr()).unwrap();
    crate::dict::dict_set(dict.as_ptr(), key.as_ptr(), one.as_ptr()).unwrap();
    assert!(before.check(dict.as_ptr()).is_err());
    let next = before.completed_mutation(dict.as_ptr()).unwrap();
    crate::dict::dict_set(dict.as_ptr(), key.as_ptr(), two.as_ptr()).unwrap();
    crate::dict::dict_set(dict.as_ptr(), key.as_ptr(), one.as_ptr()).unwrap();
    assert!(next.check(dict.as_ptr()).is_err());
    assert!(next.completed_mutation(dict.as_ptr()).is_err());
}

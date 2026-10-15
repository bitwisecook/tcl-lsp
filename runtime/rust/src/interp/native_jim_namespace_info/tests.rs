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

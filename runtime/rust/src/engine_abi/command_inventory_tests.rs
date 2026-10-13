// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::{capi, interp::Code, obj};
use std::ffi::{c_void, CString};
use std::rc::Rc;
use tcl_dialect::TclVersion;
use tcl_host_native::NativeHost;

const VERSIONS: [TclVersion; 5] = [
    TclVersion::V8_4,
    TclVersion::V8_5,
    TclVersion::V8_6,
    TclVersion::V9_0,
    TclVersion::V9_1,
];

unsafe extern "C" fn original_command(
    _: *mut c_void,
    interp: *mut Interp,
    _: core::ffi::c_int,
    _: *const *mut TclObj,
) -> core::ffi::c_int {
    // SAFETY: the actual installed C command receives its live interpreter.
    unsafe { &mut *interp }.set_result_bytes(b"ORIGINAL");
    0
}

unsafe extern "C" fn install_in_current(
    _: *mut c_void,
    interp: *mut Interp,
    _: core::ffi::c_int,
    _: *const *mut TclObj,
) -> core::ffi::c_int {
    // SAFETY: the real callback retains the interpreter's entered namespace.
    install(unsafe { &mut *interp }, b"child::y");
    0
}

fn install(interp: &mut Interp, original: &[u8]) {
    let name = CString::new(original).expect("original terminated C name");
    // SAFETY: this interpreter/name/callback remain valid through publication.
    assert!(!unsafe {
        capi::Tcl_CreateObjCommand(
            interp,
            name.as_ptr(),
            Some(original_command),
            core::ptr::null_mut(),
            None,
        )
    }
    .is_null());
}

fn capture(interp: &mut Interp) -> EngineCommandInventory {
    // SAFETY: actual live interpreter; the returned owned object is released below.
    let pointer = unsafe { tcl_engine_visible_command_inventory(interp) };
    assert!(!pointer.is_null());
    let inventory = EngineCommandInventory::decode(&obj::bytes_of(pointer))
        .expect("counted original inventory");
    unsafe { obj::decr_ref_count(pointer) };
    inventory
}

fn row<'a>(
    inventory: &'a EngineCommandInventory,
    original: &[u8],
) -> &'a EngineCommandInventoryRow {
    inventory
        .rows
        .iter()
        .find(|row| row.root_operand.as_deref() == Some(original))
        .expect("actual callable root operand")
}

#[test]
fn actual_inventory_preserves_root_colons_and_bracket_namespace_tail() {
    for version in VERSIONS {
        let mut interp = Interp::with_native_core(
            Rc::new(NativeHost::new()),
            tcl_registry::model::resolve_environment(version.dialect_profile_name()).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("actual selected C Native core");
        for name in [b":lost".as_slice(), b"lost", b"::N[0]::p*", b"::N0::pZ"] {
            install(&mut interp, name);
        }
        let inventory = capture(&mut interp);
        assert_eq!(inventory.version, version);
        let colon = row(&inventory, b":lost");
        let plain = row(&inventory, b"lost");
        assert_ne!(
            (colon.namespace, colon.generation, &colon.slot),
            (plain.namespace, plain.generation, &plain.slot)
        );
        assert_eq!(colon.slot.simple.as_ref(), b":lost");
        let bracket = row(&inventory, b"N[0]::p*");
        assert_eq!(bracket.slot.namespace.as_segments()[0].as_ref(), b"N[0]");
        assert_eq!(bracket.slot.simple.as_ref(), b"p*");
        for original in [b":lost".as_slice(), b"lost", b"N[0]::p*", b"N0::pZ"] {
            assert_eq!(
                interp.eval_str(
                    tcl_syntax::list::list_element(std::str::from_utf8(original).unwrap())
                        .as_bytes()
                ),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), b"ORIGINAL");
        }
    }
}

#[test]
fn actual_inventory_retains_visible_alias_and_import_binding_generations() {
    for version in VERSIONS {
        let mut interp = Interp::with_native_core(
            Rc::new(NativeHost::new()),
            tcl_registry::model::resolve_environment(version.dialect_profile_name()).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("actual selected C Native core");
        install(&mut interp, b"::inventorySource::target");
        assert_eq!(interp.eval_str(b"namespace eval ::inventorySource {namespace export target}; namespace eval ::inventoryDest {namespace import ::inventorySource::target}; interp alias {} inventoryAlias {} ::inventorySource::target"), Code::Ok);
        let before = capture(&mut interp);
        for name in [
            b"inventorySource::target".as_slice(),
            b"inventoryDest::target",
            b"inventoryAlias",
        ] {
            let selected = row(&before, name);
            assert!(interp
                .resolve_dispatchable_with_generation(GLOBAL, name)
                .is_some_and(|(_, generation)| generation == Some(selected.generation)));
            assert_eq!(
                interp.eval_str(
                    tcl_syntax::list::list_element(std::str::from_utf8(name).unwrap()).as_bytes()
                ),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), b"ORIGINAL");
        }
        let first = row(&before, b"inventorySource::target").generation;
        install(&mut interp, b"::inventorySource::target");
        let after = capture(&mut interp);
        assert_ne!(row(&after, b"inventorySource::target").generation, first);
    }
}

#[test]
fn actual_inventory_keeps_unaddressable_slot_instead_of_dropping_it() {
    for version in VERSIONS {
        let mut interp = Interp::with_native_core(
            Rc::new(NativeHost::new()),
            tcl_registry::model::resolve_environment(version.dialect_profile_name()).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("actual selected C Native core");
        assert_eq!(interp.eval_str(b"namespace eval ::N: {}"), Code::Ok);
        let ns = interp
            .namespaces()
            .find_namespace(GLOBAL, b"::N:")
            .expect("actual published namespace");
        let name = c"inventoryInstallOriginal";
        // SAFETY: the real installed callback publishes while entered in N:.
        assert!(!unsafe {
            capi::Tcl_CreateObjCommand(
                &mut interp,
                name.as_ptr(),
                Some(install_in_current),
                core::ptr::null_mut(),
                None,
            )
        }
        .is_null());
        assert_eq!(
            interp.eval_str(b"namespace eval ::N: {::inventoryInstallOriginal}"),
            Code::Ok
        );
        let inventory = capture(&mut interp);
        let actual = inventory
            .rows
            .iter()
            .find(|row| {
                row.slot.namespace.as_segments().len() == 2
                    && row.slot.namespace.as_segments()[0].as_ref() == b"N:"
                    && row.slot.namespace.as_segments()[1].as_ref() == b"child"
                    && row.slot.simple.as_ref() == b"y"
            })
            .expect("actual ambiguous namespace slot retained");
        assert_ne!(actual.namespace, u64::try_from(ns).unwrap());
        assert!(actual.root_operand.is_none());
        assert_eq!(actual.slot.namespace.as_segments()[0].as_ref(), b"N:");
    }
}

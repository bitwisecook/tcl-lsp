// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original Jim cat worker source contracts, separate from public native rows.

use crate::{Value, Vm};
use tcl_dialect::TclVersion;
use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
use tcl_syntax::native_string::{NativeStringProtocol, NativeStringStorageIdentity as Storage};
use tcl_syntax::raw_string::NativeValueAccessRefusal;

fn jim_vm() -> Vm {
    crate::native_fixture::core(
        tcl_registry::model::ingress::resolve_known_environment("jim")
            .expect("the retained Jim provider has an explicit ingress recipe")
            .unit_profile(),
    )
}

#[test]
fn jim_cat_zero_and_single_operand_keep_their_distinct_original_source_contracts() {
    // naming.source.jim-original-concatenation-ownership
    // docs/design/analysis/name-resolution-proofs/jim-original-concatenation-ownership.md
    // Pinned Jim OPT_CAT source/API contract; no new public zero/unary observation.
    let mut vm = jim_vm();
    let empty = tcl_cmd_core::string::cat(&mut vm, &[]).unwrap();
    assert_eq!(empty.native_object_type_name(), "none");
    assert_eq!(empty.resident_string_bytes().unwrap().as_ref(), b"");
    let member = Value::int(17);
    let original =
        Value::native_list_constructor(vec![member.clone()], NativeStringProtocol::Jim084);
    let references = original.native_object_reference_count();
    let only = tcl_cmd_core::string::cat(&mut vm, std::slice::from_ref(&original)).unwrap();
    assert!(only.is_same_object(&original));
    assert_eq!(original.native_object_type_name(), "list");
    assert!(original.resident_string_bytes().is_none());
    assert!(member.resident_string_bytes().is_none());
    assert_eq!(original.native_object_reference_count(), references + 1);
    drop(only);
    assert_eq!(original.native_object_reference_count(), references);
}

#[test]
fn jim_cat_multiple_original_operands_create_unknown_count_string_backing() {
    // naming.source.jim-original-concatenation-ownership
    // docs/design/analysis/name-resolution-proofs/jim-original-concatenation-ownership.md
    // Native Info186 independently observes the two-operand name result's String.
    let mut vm = jim_vm();
    let prefix = Value::new_native_string_bytes(b"::".as_slice());
    let name = Value::new_native_string_bytes(b"::alpha".as_slice());
    let result = tcl_cmd_core::string::cat(&mut vm, &[prefix.clone(), name.clone()]).unwrap();
    assert!(!result.is_same_object(&prefix));
    assert!(!result.is_same_object(&name));
    assert_eq!(result.native_object_type_name(), "string");
    assert_eq!(
        result.resident_string_bytes().unwrap().as_ref(),
        b"::::alpha"
    );
    let snapshot = result.native_object_snapshot();
    assert!(matches!(
        snapshot.cache,
        Cache::JimString { num_chars: None }
    ));
    assert_eq!(snapshot.storage, Some(Storage::Allocated));
    assert_eq!(prefix.native_object_type_name(), "none");
    assert_eq!(name.native_object_type_name(), "none");
    let empty = Value::new_native_string_bytes(b"".as_slice());
    let allocated = tcl_cmd_core::string::cat(&mut vm, &[empty.clone(), empty]).unwrap();
    assert_eq!(allocated.native_object_type_name(), "string");
    assert_eq!(
        allocated.native_object_snapshot().storage,
        Some(Storage::Allocated)
    );
}

#[test]
fn jim_cat_refuses_retired_and_foreign_original_getters_without_rebuilding() {
    // naming.source.jim-original-concatenation-ownership
    // docs/design/analysis/name-resolution-proofs/jim-original-concatenation-ownership.md
    let mut vm = jim_vm();
    let original = Value::new_native_string_bytes(b"retired".as_slice());
    let lease = original.native_lifetime_lease();
    drop(original);
    let error =
        tcl_cmd_core::string::cat(&mut vm, std::slice::from_ref(lease.value())).unwrap_err();
    assert_eq!(
        error.native_access_refusal(),
        Some(NativeValueAccessRefusal::CommandProtocolUnavailable(
            "retired native object header"
        )),
    );
    let prefix = Value::new_native_string_bytes(b"::".as_slice());
    let member = Value::int(23);
    let foreign = Value::native_list_constructor(
        vec![member.clone()],
        NativeStringProtocol::C(TclVersion::V9_0),
    );
    assert!(tcl_cmd_core::string::cat(&mut vm, &[prefix.clone(), foreign.clone()]).is_err());
    assert_eq!(foreign.native_object_type_name(), "list");
    assert!(foreign.resident_string_bytes().is_none());
    assert!(member.resident_string_bytes().is_none());
    assert_eq!(prefix.native_object_type_name(), "none");
}

#[test]
fn jim_cat_selected_unavailable_logical_purpose_does_not_reach_physical_engine() {
    // naming.source.jim-original-concatenation-ownership
    // docs/design/analysis/name-resolution-proofs/jim-original-concatenation-ownership.md
    let mut vm = jim_vm();
    vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
    assert!(
        vm.set_logical_name_provider(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
            TclVersion::V8_4
        ),)
    );
    let unknown = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
        "jim",
        &[],
        "Jim",
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
    )));
    vm.set_dialect_profile(unknown);
    let original = Value::int(17);
    assert!(tcl_cmd_core::string::cat(&mut vm, std::slice::from_ref(&original)).is_err());
    assert!(tcl_cmd_core::string::cat(&mut vm, &[]).is_err());
    assert!(original.resident_string_bytes().is_none());
    assert_eq!(original.native_object_type_name(), "int");
}

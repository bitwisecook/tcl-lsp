// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
#[cfg(debug_assertions)]
use std::cell::RefCell;
use tcl_test_support::encoding_names::{EncodingHost, EncodingStages};

fn supplied(engine: &str, script: &[u8]) -> (Vm, Rc<EncodingStages>) {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect(engine));
    let stages = Rc::new(EncodingStages::default());
    let host = EncodingHost::new(
        vm.host_rc(),
        Rc::clone(&stages),
        SystemEncoding::Identity,
        script.to_vec(),
    );
    vm.set_host(Rc::new(host));
    stages.events.borrow_mut().clear();
    (vm, stages)
}

#[test]
fn original_system_names_keep_resident_bytes_zero_queries_and_captured_default() {
    // naming.encoding.original-byte-name-lookup-and-source-order
    // docs/design/analysis/name-resolution-proofs/encoding-original-byte-name-lookup-and-source-order.md
    let (mut vm, stages) = supplied("tcl8.6", b"return YES");
    for (name, expected) in [
        (b"utf-8\0TAIL".as_slice(), SystemEncoding::Utf8),
        (b"\0TAIL".as_slice(), SystemEncoding::Identity),
    ] {
        let completion = set_system(&mut vm, &Value::new_native_string_bytes(name));
        assert_eq!(completion.code, tcl_runtime_api::Code::Ok);
        assert_eq!(vm.system_encoding(), expected);
        assert!(stages.events.borrow().is_empty());
    }
    let completion = set_system(&mut vm, &Value::new_native_string_bytes(b"\xff".as_slice()));
    assert_eq!(completion.code, tcl_runtime_api::Code::Error);
    assert_eq!(
        completion.result.resident_string_bytes().unwrap().as_ref(),
        b"unknown encoding \"\xff\""
    );
    assert_eq!(stages.events.borrow().as_slice(), ["catalogue", "lookup"]);
    assert_eq!(vm.system_encoding(), SystemEncoding::Identity);
    assert!(vm.execution_refusal.is_none());
}

#[cfg(debug_assertions)]
#[test]
fn original_catalogue_callback_header_withdrawal_preserves_first_host_before_guest() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Supplied software catalogue callback, not an external Native callback chronology.
    let (mut vm, stages) = supplied("tcl8.6", b"return YES");
    let owner = Rc::new(RefCell::new(Some(Value::new_native_string_bytes(
        b"external-codec".as_slice(),
    ))));
    let lease = owner.borrow().as_ref().unwrap().native_lifetime_lease();
    let release = Rc::clone(&owner);
    *stages.after_lookup.borrow_mut() = Some(Box::new(move || {
        drop(release.borrow_mut().take());
    }));
    let scope =
        crate::interp::native_operation_currency::NativeOperationScope::enter(&mut vm).unwrap();
    let completion = set_system(&mut vm, lease.value());
    assert_eq!(completion.code, tcl_runtime_api::Code::Error);
    let first = vm
        .execution_refusal
        .clone()
        .expect("actual original-header withdrawal");
    assert_eq!(scope.check_native(&mut vm), Err(first.clone()));
    assert_eq!(vm.system_encoding(), SystemEncoding::Identity);
    assert_eq!(stages.events.borrow().as_slice(), ["catalogue", "lookup"]);
    assert!(owner.borrow().is_none());
    let later = set_system(
        &mut vm,
        &Value::new_native_string_bytes(b"ascii".as_slice()),
    );
    assert_eq!(later.code, tcl_runtime_api::Code::Error);
    assert_eq!(vm.execution_refusal, Some(first));
    assert_eq!(stages.events.borrow().as_slice(), ["catalogue", "lookup"]);
}

#[test]
fn original_absent_catalogue_refuses_without_unknown_guest_or_system_mutation() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, stages) = supplied("tcl8.6", b"return YES");
    stages.catalogue_available.set(false);
    let scope =
        crate::interp::native_operation_currency::NativeOperationScope::enter(&mut vm).unwrap();
    let completion = set_system(
        &mut vm,
        &Value::new_native_string_bytes(b"external-codec".as_slice()),
    );
    assert_eq!(completion.code, tcl_runtime_api::Code::Error);
    let first = vm
        .execution_refusal
        .clone()
        .expect("unavailable original catalogue");
    assert_eq!(scope.check_native(&mut vm), Err(first));
    assert_eq!(vm.system_encoding(), SystemEncoding::Identity);
    assert_eq!(stages.events.borrow().as_slice(), ["catalogue"]);
    assert!(
        !completion
            .result
            .resident_string_bytes()
            .unwrap()
            .starts_with(b"unknown encoding")
    );
}

#[test]
fn original_opened_file_configures_before_read_and_closes_on_failure() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Supplied open/read/close stages, independent of external native read-error chronology.
    let (mut vm, stages) = supplied("tcl8.6", b"return YES");
    *stages.read_error.borrow_mut() = Some(HostError::Permission);
    let args = [
        Value::new_native_string_bytes(b"-encoding".as_slice()),
        Value::new_native_string_bytes(b"external-codec".as_slice()),
        Value::new_native_string_bytes(b"/source.tcl".as_slice()),
    ];
    let completion = source(&mut vm, &args);
    assert_eq!(completion.code, tcl_runtime_api::Code::Error);
    assert!(
        completion
            .result
            .resident_string_bytes()
            .unwrap()
            .starts_with(b"unknown encoding")
    );
    assert_eq!(
        stages.events.borrow().as_slice(),
        ["filesystem", "open", "catalogue", "lookup", "close"]
    );
    assert!(vm.execution_refusal.is_none());
    stages.events.borrow_mut().clear();
    let args = [
        Value::new_native_string_bytes(b"-encoding".as_slice()),
        Value::new_native_string_bytes(b"utf-8".as_slice()),
        Value::new_native_string_bytes(b"/source.tcl".as_slice()),
    ];
    let completion = source(&mut vm, &args);
    assert_eq!(completion.code, tcl_runtime_api::Code::Error);
    assert!(
        completion
            .result
            .resident_string_bytes()
            .unwrap()
            .starts_with(b"couldn't read file")
    );
    assert_eq!(
        stages.events.borrow().as_slice(),
        ["filesystem", "open", "read", "close"]
    );
    assert!(vm.execution_refusal.is_none());
}

#[cfg(debug_assertions)]
#[test]
fn original_filename_converter_retains_header_refusal_before_open() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Genuine supplied converter callback; no external Native callback chronology.
    let (mut vm, stages) = supplied("tcl8.6", b"set ::SHOULD_NOT_RUN YES");
    assert_eq!(
        set_system(
            &mut vm,
            &Value::new_native_string_bytes(b"ascii".as_slice())
        )
        .code,
        tcl_runtime_api::Code::Ok
    );
    *stages.filename_conversion.borrow_mut() = Some(Ok(b"/source.tcl".to_vec()));
    let owner = Rc::new(RefCell::new(Some(Value::new_native_string_bytes(
        b"/source.tcl".as_slice(),
    ))));
    let lease = owner.borrow().as_ref().unwrap().native_lifetime_lease();
    let release = Rc::clone(&owner);
    *stages.after_filename.borrow_mut() = Some(Box::new(move || drop(release.borrow_mut().take())));
    let scope =
        crate::interp::native_operation_currency::NativeOperationScope::enter(&mut vm).unwrap();
    let completion = source(&mut vm, std::slice::from_ref(lease.value()));
    assert_eq!(completion.code, tcl_runtime_api::Code::Error);
    let first = vm
        .execution_refusal
        .clone()
        .expect("actual filename header withdrawal");
    assert_eq!(scope.check_native(&mut vm), Err(first));
    assert!(owner.borrow().is_none());
    assert!(vm.get_var("::SHOULD_NOT_RUN").is_none());
    assert_eq!(
        stages.events.borrow().as_slice(),
        ["filesystem", "filename-conversion"]
    );
}

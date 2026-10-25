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
use crate::interp::native_operation_currency::NativeOperationScope;
use tcl_syntax::raw_string::{NativeValueAccessRefusal, UnicodeAccessError};

#[test]
fn original_raw_operand_projection_observes_none_without_native_or_unicode_grants() {
    // naming.embedding.original-host-publication-and-fact-transport

    let mut vm = Vm::new();
    let value = Value::new_native_string_bytes(b"A\xc0\x80B".as_slice());
    let count = value.native_object_reference_count();
    let original = vm.capture_original_operand_projection().unwrap();
    assert!(
        NativeOperationScope::current_entered(&mut vm)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        vm.original_operand_raw_bytes(&original, &value)
            .unwrap()
            .as_ref(),
        b"A\xc0\x80B"
    );
    let expected = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::Unicode(UnicodeAccessError {
            valid_up_to: 1,
            error_len: Some(1),
        }),
    );
    let error = vm.original_operand_unicode(&original, &value).map_or_else(
        core::convert::identity,
        |value| {
            drop(value);
            panic!("strict UTF-8 must refuse native modified-NUL bytes")
        },
    );
    assert!(
        matches!(error, TclError::Host(crate::TclHostFailure::Execution(ref actual)) if actual == &expected)
    );
    assert_eq!(value.native_object_reference_count(), count);
    assert_eq!(
        value.resident_string_bytes().unwrap().as_ref(),
        b"A\xc0\x80B"
    );
    assert!(vm.finish_original_operand_projection(original).is_err());
    let next = vm.capture_original_operand_projection().unwrap();
    assert_eq!(
        vm.original_operand_raw_bytes(&next, &Value::int(i64::MIN))
            .unwrap()
            .as_ref(),
        i64::MIN.to_string().as_bytes()
    );
    assert!(
        vm.original_operand_string_bytes(&next, &Value::int(23))
            .is_err()
    );
    assert!(vm.finish_original_operand_projection(next).is_err());
    assert!(
        NativeOperationScope::current_entered(&mut vm)
            .unwrap()
            .is_none()
    );
}

#[test]
fn original_operand_projection_refuses_real_header_retirement_before_legacy_cache() {
    // naming.embedding.original-host-publication-and-fact-transport

    let mut vm = Vm::new();
    let owner = Value::int(i64::MIN);
    let borrowed = owner.native_lifetime_lease();
    let original = vm.capture_original_operand_projection().unwrap();
    drop(owner);
    assert_eq!(borrowed.value().native_object_reference_count(), 0);
    let expected = borrowed
        .value()
        .original_native_header_currentness()
        .unwrap_err();
    assert!(borrowed.value().resident_string_bytes().is_none());
    let error = vm
        .original_operand_raw_bytes(&original, borrowed.value())
        .map_or_else(core::convert::identity, |value| {
            drop(value);
            panic!("retired actual header")
        });
    assert!(
        matches!(error, TclError::Host(crate::TclHostFailure::Execution(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(ref actual))) if actual == &expected)
    );
    assert!(borrowed.value().resident_string_bytes().is_none());
    assert!(vm.finish_original_operand_projection(original).is_err());
    assert!(
        NativeOperationScope::current_entered(&mut vm)
            .unwrap()
            .is_none()
    );
}

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
use crate::interp::{
    native_operation_currency::NativeOperationScope,
    original_authored_operands::OriginalAuthoredOperandScope,
};
use tcl_registry::f5::{BigIpExecutionContext, evidence::BigIpBuild, naming::BigIpNameEvent};

fn inputs() -> AuthoredObservedFrameStorageContext {
    AuthoredObservedFrameStorageContext {
        policy: tcl_registry::f5::naming::observed_name_policy(
            BigIpBuild::MEASURED_21_1_0_1,
            BigIpExecutionContext::TmmIRule,
            BigIpNameEvent::HttpRequest,
        )
        .unwrap(),
        tmm: 7,
        domain: 11,
    }
}
fn fixture() -> Vm {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
    Frames::push(&mut vm, tcl_runtime_api::ROOT_NS);
    vm
}

#[test]
fn original_event_input_transition_preserves_actual_scope_and_restores_selected_fields() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software issuer/actual frame storage; no physical BIG-IP chronology claim.
    let mut vm = fixture();
    let before = vm.execution_name_policy();
    let entered = NativeOperationScope::enter(&mut vm).unwrap();
    let original = OriginalAuthoredOperandScope::capture(&mut vm).unwrap();
    let result = vm
        .with_original_observed_event_inputs_using(
            &original,
            BigIpBuild::MEASURED_21_1_0_1,
            BigIpExecutionContext::TmmIRule,
            BigIpNameEvent::HttpRequest,
            inputs(),
            |vm| {
                assert_eq!(
                    vm.execution_name_policy(),
                    Some(ExecutionNamePolicy::ObservedBigIp(inputs().policy))
                );
                assert_eq!(
                    vm.observed_frame_storage.as_ref().unwrap().context,
                    inputs()
                );
                let frame = Frames::current(vm);
                tcl_runtime_api::VarStore::set_bytes(
                    vm,
                    frame,
                    b"event\0key",
                    crate::Value::string("REACHED"),
                )
                .unwrap();
                Ok(crate::Completion::new(
                    tcl_runtime_api::Code::Ok,
                    crate::Value::string("RETURNED"),
                    crate::Value::empty(),
                ))
            },
        )
        .unwrap();
    assert_eq!(
        result.result.resident_string_bytes().unwrap().as_ref(),
        b"RETURNED"
    );
    assert_eq!(vm.execution_name_policy(), before);
    assert!(vm.observed_frame_storage.is_none());
    assert!(vm.execution_refusal.is_none());
    entered.check_native(&mut vm).unwrap();
    original.finish(&mut vm).unwrap();
    drop(entered);
    Frames::pop(&mut vm);
}

#[test]
fn original_event_input_transition_keeps_callback_aba_and_prior_host_terminal_after_restoration() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for prior_host in [false, true] {
        let mut vm = fixture();
        let before = vm.execution_name_policy();
        let entered = NativeOperationScope::enter(&mut vm).unwrap();
        let original = OriginalAuthoredOperandScope::capture(&mut vm).unwrap();
        let reached = vm.with_original_observed_event_inputs_using(
            &original,
            BigIpBuild::MEASURED_21_1_0_1,
            BigIpExecutionContext::TmmIRule,
            BigIpNameEvent::HttpRequest,
            inputs(),
            |vm| {
                if prior_host {
                    let _ = vm.refuse_host_command("original event callback Host".into());
                }
                let old = vm.host_rc();
                vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
                vm.set_host(old);
                Ok(crate::Completion::new(
                    tcl_runtime_api::Code::Ok,
                    crate::Value::string("UNPUBLISHED"),
                    crate::Value::empty(),
                ))
            },
        );
        let error = reached.map_or_else(core::convert::identity, |value| {
            drop(value);
            panic!("callback ABA remains terminal")
        });
        let first = vm.execution_refusal.clone().unwrap();
        assert_eq!(error.native_execution_refusal(), Some(&first));
        if prior_host {
            assert!(
                matches!(&first, tcl_runtime_api::NativeExecutionError::HostCommandRefusal(actual) if actual.reason == "original event callback Host")
            );
        }
        assert_eq!(vm.execution_name_policy(), before);
        assert!(vm.observed_frame_storage.is_none());
        assert_eq!(entered.check_native(&mut vm).err(), Some(first.clone()));
        assert!(original.finish(&mut vm).is_err());
        assert_eq!(vm.execution_refusal, Some(first));
        drop(entered);
        Frames::pop(&mut vm);
    }
}

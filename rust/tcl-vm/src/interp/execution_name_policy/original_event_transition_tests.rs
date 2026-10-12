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
        let error = reached.err().expect("callback ABA remains terminal");
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

#[test]
fn original_observed_cache_retirement_keeps_child_artifacts_on_actual_child_selection() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software stage selection after actual first cache clear; no Native
    // destructor scheduling/physical callback chronology is claimed.
    let mut vm = fixture();
    let parent = vm.cur_interp();
    let child_name = vm.create_child(Some("eventCacheChild".into()), false);
    let child = vm.child_id(&child_name).unwrap();
    vm.in_interp(child, |vm| {
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(
                crate::environment::profile_for_dialect("tcl9.0"),
            ),
        ));
        let source = tcl_lexer::SourceImage::native(b"return CHILD".as_slice());
        let namespace = vm.source_namespace_path();
        let normal = vm.compile_module_bytes(&source, &namespace, false).unwrap();
        let plain = vm.compile_module_bytes(&source, &namespace, true).unwrap();
        drop((normal, plain));
        assert!(!vm.eval_cache.is_empty());
        assert!(!vm.eval_cache_plain.is_empty());
    });
    let child_counts = vm.in_interp(child, |vm| {
        (
            vm.eval_cache.len(),
            vm.eval_cache_plain.len(),
            vm.module_procs.len(),
        )
    });
    let entered = NativeOperationScope::enter(&mut vm).unwrap();
    let observed =
        super::super::native_operation_currency::OriginalOperationObservation::capture(&mut vm)
            .unwrap();
    let operands = OriginalAuthoredOperandScope::capture(&mut vm).unwrap();
    let names = ObservedNameSelection {
        policy: inputs().policy,
        build: BigIpBuild::MEASURED_21_1_0_1,
        context: BigIpExecutionContext::TmmIRule,
        event: BigIpNameEvent::HttpRequest,
        profile: vm.dialect_profile(),
        engine: vm.actual_engine_profile,
    };
    let mut checks = 0;
    let reached =
        observed.replace_observed_inputs_using(&mut vm, Some(names), None, false, &mut |vm| {
            checks += 1;
            if checks == 2 {
                super::super::oo_cleanup::select_cleanup_fixture_interpreter(vm, child);
            }
            operands.check(vm)
        });
    assert!(reached.is_err());
    assert!(vm.interp_is_current(child));
    assert_eq!(
        (
            vm.eval_cache.len(),
            vm.eval_cache_plain.len(),
            vm.module_procs.len()
        ),
        child_counts
    );
    let first = vm.execution_refusal.clone().unwrap();
    vm.in_interp(parent, |vm| {
        assert!(vm.eval_cache.is_empty());
        assert!(vm.eval_cache_plain.is_empty());
        assert!(vm.module_procs.is_empty());
        assert!(
            observed
                .replace_observed_inputs_using(vm, None, None, true, &mut |_| Ok(()))
                .is_err()
        );
    });
    assert!(vm.interp_is_current(child));
    assert_eq!(vm.execution_refusal, Some(first));
    drop(entered);
}

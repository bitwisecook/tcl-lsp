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

//! Source dispatch inside a measured BIG-IP event frame, with its counted
//! variable names.

use crate::prelude::*;
use tcl_registry::f5::{BigIpExecutionContext, evidence::BigIpBuild, naming::BigIpNameEvent};
use tcl_runtime_api::authored_tmm::AuthoredObservedFrameStorageContext;
use tcl_syntax::naming::ExecutionNamePolicy;

fn select(vm: &mut Vm) -> AuthoredObservedFrameStorageContext {
    assert!(vm.set_observed_bigip_name_policy(
        BigIpBuild::MEASURED_21_1_0_1,
        BigIpExecutionContext::TmmIRule,
        BigIpNameEvent::HttpRequest
    ));
    let Some(ExecutionNamePolicy::ObservedBigIp(policy)) = vm.execution_name_policy() else {
        panic!("exact measured issuer");
    };
    AuthoredObservedFrameStorageContext {
        policy,
        tmm: 2,
        domain: 7,
    }
}

#[test]
fn observed_counted_set_dispatch_uses_independent_command_and_source_issuers() {
    let profile = tcl_vm::test_support::profile("tcl9.0");
    let mut vm = crate::fixture::interpreter(profile);
    let context = select(&mut vm);
    vm.with_observed_event_frame(context, |vm| {
        let result = vm.try_eval_source("set {__tcl_lsp_2286_r2286m_nul_A\0B} counted; set {__tcl_lsp_2286_r2286m_nul_A\0B}").unwrap();
        assert_eq!(result.code, tcl_runtime_api::Code::Ok);
        assert_eq!(result.result.string_bytes().as_ref(), b"counted");
        let result = vm.try_eval_source("set {__tcl_lsp_2286_r2286m_arrayindex_root(A\0B)} element; set {__tcl_lsp_2286_r2286m_arrayindex_root(A\0B)}").unwrap();
        assert_eq!(result.code, tcl_runtime_api::Code::Ok);
        assert_eq!(result.result.string_bytes().as_ref(), b"element");
    }).unwrap();
}

#[test]
fn counted_array_root_and_scalar_queries_use_normal_source_dispatch() {
    let profile = tcl_vm::test_support::profile("tcl9.0");
    let mut vm = crate::fixture::interpreter(profile);
    let context = select(&mut vm);
    vm.with_observed_event_frame(context, |vm| {
        let result = vm.try_eval_source("set {R\0T(k)} element; list [array exists {R\0T}] [array size {R\0T}] [array names {R\0T}] [info exists {R\0T(k)}] [info exists {R\0T(missing)}]").unwrap();
        assert_eq!(result.code, tcl_runtime_api::Code::Ok);
        assert_eq!(result.result.string_bytes().as_ref(), b"1 1 k 1 0");
        let result = vm.try_eval_source("set {S\0T} scalar; set copied ${S\0T}; unset {S\0T}; list $copied [info exists {S\0T}]").unwrap();
        assert_eq!(result.code, tcl_runtime_api::Code::Ok);
        assert_eq!(result.result.string_bytes().as_ref(), b"scalar 0");
    }).unwrap();
}

#[test]
fn observed_trace_and_upvar_purposes_are_unavailable_in_source_dispatch() {
    for source in [
        "trace add variable A write {set callback}",
        "upvar 0 A alias",
    ] {
        let profile = tcl_vm::test_support::profile("tcl9.0");
        let mut vm = crate::fixture::interpreter(profile);
        let context = select(&mut vm);
        vm.with_observed_event_frame(context, |vm| {
            assert!(vm.try_eval_source(source).is_err(), "{source}");
        })
        .unwrap();
    }
}

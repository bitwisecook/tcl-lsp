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

//! The `clock` command — wall-clock readout + civil-date math, over the shared
//! [`tcl_cmd_core::clock`] core. The VM reads the current time from its host's
//! [`Clock`](tcl_platform::Clock) and passes it (plus a local-offset callback)
//! into the host-free core.

use tcl_cmd_core::clock as core_clock;
use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    register_clock(vm);
    vm.register_stock_builtin(
        "::tcl::unsupported::clock::configure",
        |_vm, args| match args {
            [option] if &*option.to_str() == "-init-complete" => ok(Value::empty()),
            _ => err("unsupported clock configuration option"),
        },
    );
}

/// The `::tcl::clock` ensemble where the release bootstraps one, so compiled
/// readings find the members C compiles; elsewhere the plain command. Library
/// and framework code call the fully-qualified members directly either way.
fn register_clock(vm: &mut Vm) {
    if let Some(namespace) = vm
        .actual_native_invocation_dialect()
        .ensemble_implementation_namespace(tcl_registry::EnsembleImplementationFamily::Clock)
    {
        let subs: Vec<&str> = CLOCK_MEMBERS.iter().map(|&(member, _)| member).collect();
        vm.register_stock_namespace_ensemble("clock", namespace, CLOCK_MEMBERS, &subs);
        return;
    }
    vm.register_stock_builtin("clock", cmd_clock);
    for &(member, handler) in CLOCK_MEMBERS {
        vm.register_stock_builtin(&format!("::tcl::clock::{member}"), handler);
    }
}

/// Repinning retains user replacements and selects the release's surface.
pub(crate) fn refresh_profile(vm: &mut Vm) {
    if vm.stock_native_identity("clock").as_deref() != Some("clock") {
        return;
    }
    for &(member, _) in CLOCK_MEMBERS {
        let target = format!("::tcl::clock::{member}");
        if vm.stock_native_identity(&target).as_deref()
            == tcl_syntax::naming::unroot_rooted_key(&target)
        {
            vm.remove_registered_command(
                tcl_syntax::naming::unroot_rooted_key(&target).unwrap_or(&target),
            );
        }
    }
    register_clock(vm);
    if vm
        .actual_native_invocation_dialect()
        .ensemble_implementation_namespace(tcl_registry::EnsembleImplementationFamily::Clock)
        .is_none()
    {
        vm.retire_unused_stock_ensemble_namespace("clock");
    }
}

const CLOCK_MEMBERS: &[(&str, crate::command::BuiltinFn)] = &[
    ("add", |vm, args| member(vm, "add", args)),
    ("clicks", |vm, args| member(vm, "clicks", args)),
    ("format", |vm, args| member(vm, "format", args)),
    ("microseconds", |vm, args| member(vm, "microseconds", args)),
    ("milliseconds", |vm, args| member(vm, "milliseconds", args)),
    ("scan", |vm, args| member(vm, "scan", args)),
    ("seconds", |vm, args| member(vm, "seconds", args)),
];

/// Dispatch an `::tcl::clock::<sub>` ensemble member by prepending `sub`.
fn member(vm: &mut Vm, sub: &str, a: &[Value]) -> Completion<Value> {
    let mut args = Vec::with_capacity(a.len() + 1);
    args.push(Value::string(sub));
    args.extend_from_slice(a);
    cmd_clock(vm, &args)
}

fn cmd_clock(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let host = vm.host_rc();
    let now = core_clock::Now {
        secs: host.clock().now_secs(),
        millis: i64::try_from(host.clock().now_millis()).unwrap_or(i64::MAX),
        micros: i64::try_from(host.clock().now_micros()).unwrap_or(i64::MAX),
    };
    let offset = move |ts: i64| host.clock().local_offset_secs(ts);
    let release = vm.runtime_version();
    match core_clock::dispatch(vm, args, &now, &offset, release) {
        Ok(v) => ok(v),
        Err(e) => vm.cmd_error(e),
    }
}

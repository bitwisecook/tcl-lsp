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
    vm.register("clock", cmd_clock);
    // The ensemble's implementation members — `clock clicks` compiles/dispatches
    // to `::tcl::clock::clicks`, and library/framework code calls these
    // fully-qualified forms directly. Each prepends its subcommand and reuses
    // the same dispatch.
    vm.register("::tcl::clock::seconds", |vm, a| member(vm, "seconds", a));
    vm.register("::tcl::clock::milliseconds", |vm, a| {
        member(vm, "milliseconds", a)
    });
    vm.register("::tcl::clock::microseconds", |vm, a| {
        member(vm, "microseconds", a)
    });
    vm.register("::tcl::clock::clicks", |vm, a| member(vm, "clicks", a));
    vm.register("::tcl::clock::format", |vm, a| member(vm, "format", a));
    vm.register("::tcl::clock::add", |vm, a| member(vm, "add", a));
    vm.register("::tcl::clock::scan", |vm, a| member(vm, "scan", a));
    vm.register(
        "::tcl::unsupported::clock::configure",
        |_vm, args| match args {
            [option] if &*option.to_str() == "-init-complete" => ok(Value::empty()),
            _ => err("unsupported clock configuration option"),
        },
    );
}

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
    match core_clock::dispatch(vm, args, &now, &offset) {
        Ok(v) => ok(v),
        Err(e) => err(e.into_message()),
    }
}

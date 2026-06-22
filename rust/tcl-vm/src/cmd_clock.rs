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

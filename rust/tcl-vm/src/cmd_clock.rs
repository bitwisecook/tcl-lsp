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

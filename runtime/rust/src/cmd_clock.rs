//! The `clock` command — wall-clock readout + civil-date math, over the shared
//! [`tcl_cmd_core::clock`] core. The runtime reads the current time from its
//! host's [`Clock`](tcl_platform::Clock) and passes it (plus a local-offset
//! callback) into the host-free core; the fresh result object is retained by
//! `set_result`.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use tcl_cmd_core::clock as core_clock;

use crate::interp::{Code, Interp};
use crate::obj::TclObj;

/// Register `clock`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"clock", clock_cmd);
}

fn clock_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let host = interp.host();
    let now = core_clock::Now {
        secs: host.clock().now_secs(),
        millis: i64::try_from(host.clock().now_millis()).unwrap_or(i64::MAX),
        micros: i64::try_from(host.clock().now_micros()).unwrap_or(i64::MAX),
    };
    let offset = move |ts: i64| host.clock().local_offset_secs(ts);
    match core_clock::dispatch(interp, &argv[1..], &now, &offset) {
        Ok(v) => {
            interp.set_result(v);
            Code::Ok
        }
        Err(e) => interp.set_error(e.message().as_bytes()),
    }
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
        }
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
    }

    fn run(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?}",
            String::from_utf8_lossy(src)
        );
        i.result_bytes()
    }

    #[test]
    fn clock_format_add_and_timing() {
        leak_free(|i| {
            // Civil-date formatting (UTC), pinned vs tclsh 9.0.
            assert_eq!(
                run(i, b"clock format 1700000000 -gmt 1"),
                b"Tue Nov 14 22:13:20 GMT 2023"
            );
            assert_eq!(
                run(
                    i,
                    b"clock format 1700000000 -format {%Y-%m-%d %H:%M:%S %j %u} -gmt 1"
                ),
                b"2023-11-14 22:13:20 318 2"
            );
            // Unknown specifier passes through (no %F in Tcl).
            assert_eq!(
                run(i, b"clock format 0 -format {%F %T} -gmt 1"),
                b"%F 00:00:00"
            );
            // clock add: fixed + calendar units.
            assert_eq!(
                run(i, b"clock format [clock add 1700000000 3 months -gmt 1] -format {%Y-%m-%d} -gmt 1"),
                b"2024-02-14"
            );
            // Timing.
            assert_eq!(run(i, b"expr {[clock seconds] > 1700000000}"), b"1");
            assert_eq!(
                run(i, b"expr {[clock microseconds] > 1700000000000000}"),
                b"1"
            );
            // scan not yet supported.
            assert_eq!(i.eval_str(b"clock scan now"), Code::Error);
            assert_eq!(i.result_bytes(), b"clock scan is not yet supported");
        });
    }
}

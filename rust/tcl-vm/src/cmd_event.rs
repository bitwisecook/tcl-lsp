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

//! The Tcl event loop for the bytecode VM: `after`, `vwait`, `update`.
//!
//! A minimal but faithful single-threaded event loop, ported from the
//! tree-walking runtime (`runtime/rust/src/cmd_event.rs`). `after` schedules
//! timer and idle events; `vwait` runs the loop until a named variable is
//! written; `update` drains the events that are ready now. Timer events fire in
//! deadline order (then scheduling order); an idle event fires only when no
//! timer is due.
//!
//! This is the scheduler half of the coroutine subsystem ([`crate::cmd_coro`]):
//! `after 0 $coro` schedules a coroutine resume and `vwait`/`update` drives it,
//! the idiomatic way to run a coroutine to completion off the event loop.
//!
//! Event scripts are evaluated at the global level via [`Vm::eval_source`]; a
//! script error is routed to the background-error handler (C's event handlers
//! send errors to `bgerror`) rather than propagating out of `vwait`/`update`.
//!
//! C refs: `tclEvent.c` (`Tcl_DoOneEvent`, `Tcl_AfterObjCmd`, `vwait`),
//! `tclTimer.c`.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use tcl_runtime_api::{Code, Completion};

use crate::interp::{Vm, err, ok};
use crate::value::Value;

/// A scheduled timer event (`after ms script`); `after 0` has a now-deadline.
struct TimerEv {
    id: u64,
    deadline: Instant,
    seq: u64,
    script: String,
}

/// The pending event set: due-ordered timers + a FIFO idle queue.
#[derive(Default)]
pub(crate) struct EventQueue {
    timers: Vec<TimerEv>,
    idle: VecDeque<(u64, String)>,
    next_id: u64,
    seq: u64,
}

impl EventQueue {
    fn fresh_id(&mut self) -> u64 {
        // C's `after` ids are 0-based (the first scheduled event is `after#0`).
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Schedule a timer event `ms` from now; returns its `after#<id>` number.
    fn push_timer(&mut self, ms: u64, script: String) -> u64 {
        let id = self.fresh_id();
        self.seq += 1;
        self.timers.push(TimerEv {
            id,
            deadline: Instant::now() + Duration::from_millis(ms),
            seq: self.seq,
            script,
        });
        id
    }

    /// Schedule an idle event; returns its id.
    fn push_idle(&mut self, script: String) -> u64 {
        let id = self.fresh_id();
        self.idle.push_back((id, script));
        id
    }

    /// Cancel by id (`after#<id>`); returns the removed script, if any.
    fn cancel_id(&mut self, id: u64) -> Option<String> {
        if let Some(p) = self.timers.iter().position(|t| t.id == id) {
            return Some(self.timers.remove(p).script);
        }
        if let Some(p) = self.idle.iter().position(|(i, _)| *i == id) {
            return self.idle.remove(p).map(|(_, s)| s);
        }
        None
    }

    /// Cancel by exact script text; returns the removed script, if any.
    fn cancel_script(&mut self, script: &str) -> Option<String> {
        if let Some(p) = self.timers.iter().position(|t| t.script == script) {
            return Some(self.timers.remove(p).script);
        }
        if let Some(p) = self.idle.iter().position(|(_, s)| s == script) {
            return self.idle.remove(p).map(|(_, s)| s);
        }
        None
    }

    fn is_empty(&self) -> bool {
        self.timers.is_empty() && self.idle.is_empty()
    }

    /// The earliest timer deadline (to know how long `vwait` may sleep).
    fn earliest_deadline(&self) -> Option<Instant> {
        self.timers.iter().map(|t| t.deadline).min()
    }

    /// Pop the next event whose time has come: a due timer (earliest deadline,
    /// then scheduling order), else an idle event. `None` if nothing is ready
    /// yet (a future timer remains) or the queue is empty.
    fn pop_ready(&mut self, now: Instant) -> Option<String> {
        let due = self
            .timers
            .iter()
            .enumerate()
            .filter(|(_, t)| t.deadline <= now)
            .min_by_key(|(_, t)| (t.deadline, t.seq))
            .map(|(i, _)| i);
        if let Some(i) = due {
            return Some(self.timers.remove(i).script);
        }
        // No due timer: an idle event runs only when no timer is *due*.
        self.idle.pop_front().map(|(_, s)| s)
    }

    /// Pop the next idle event only (never a timer) — `update idletasks` drains
    /// idle handlers but must not run due timer events.
    fn pop_idle(&mut self) -> Option<String> {
        self.idle.pop_front().map(|(_, s)| s)
    }
}

pub(crate) fn register(vm: &mut Vm) {
    vm.register("after", cmd_after);
    vm.register("vwait", cmd_vwait);
    vm.register("update", cmd_update);
}

/// `after ms ?script ...?` / `after idle ?script ...?` /
/// `after cancel id|script` / `after info ?id?`. A bare `after ms` (no script)
/// processes events until `ms` has elapsed (a blocking delay). Multiple script
/// args are joined with spaces as a command prefix (matching C).
fn cmd_after(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(first) = args.first() else {
        return err("wrong # args: should be \"after option ?arg ...?\"");
    };
    match &*first.to_str() {
        "idle" => {
            if args.len() < 2 {
                return err("wrong # args: should be \"after idle script\"");
            }
            let script = join_args(&args[1..]);
            let id = vm.events.push_idle(script);
            ok(after_id(id))
        }
        "cancel" => {
            if args.len() < 2 {
                return err("wrong # args: should be \"after cancel id|command\"");
            }
            let arg = args[1].to_str();
            if let Some(id) = parse_after_id(&arg) {
                vm.events.cancel_id(id);
            } else {
                let script = join_args(&args[1..]);
                vm.events.cancel_script(&script);
            }
            ok(Value::empty())
        }
        // Minimal `after info`: C lists pending ids / a given id's handler; we
        // report empty (nothing depends on the detail, and it never lies about
        // scheduling — a cancelled/fired event is simply absent).
        "info" => ok(Value::empty()),
        other => {
            let Some(ms) = parse_ms(other) else {
                return err(format!(
                    "bad argument \"{other}\": must be cancel, idle, info, or an integer"
                ));
            };
            if args.len() == 1 {
                // Bare delay: block, servicing events, until `ms` elapses.
                return delay(vm, ms);
            }
            let script = join_args(&args[1..]);
            let id = vm.events.push_timer(ms, script);
            ok(after_id(id))
        }
    }
}

/// `vwait varName` — run the event loop until `varName` is written, returning
/// when its value changes. Returns immediately (after draining) if no events
/// remain and the variable never changes, so a missing event source cannot hang.
fn cmd_vwait(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [name] = args else {
        return err("wrong # args: should be \"vwait name\"");
    };
    let name = name.to_str();
    let before = var_snapshot(vm, &name);
    loop {
        if var_snapshot(vm, &name) != before {
            break;
        }
        if vm.events.is_empty() {
            // Nothing left to service and the variable is unchanged: C reports
            // "can't wait … would wait forever"; we return so a missing event
            // source does not hang.
            break;
        }
        if let Some(c) = process_one(vm) {
            return c;
        }
    }
    ok(Value::empty())
}

/// `update` / `update idletasks` — service the events ready now, then return.
/// `idletasks` services *only* idle events (C's `TCL_IDLE_EVENTS` excludes
/// timers).
fn cmd_update(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let idletasks = match args {
        [] => false,
        [a] if &*a.to_str() == "idletasks" => true,
        [a] => {
            return err(format!("bad option \"{}\": must be idletasks", a.to_str()));
        }
        _ => return err("wrong # args: should be \"update ?idletasks?\""),
    };
    let now = Instant::now();
    loop {
        let script = if idletasks {
            vm.events.pop_idle()
        } else {
            vm.events.pop_ready(now)
        };
        let Some(script) = script else { break };
        run_event(vm, &script);
    }
    ok(Value::empty())
}

/// Process exactly one event, sleeping until the earliest timer is due if none
/// is ready yet. Returns `Some(error)` only if an event handler's error should
/// propagate (it never does today — handler errors go to `bgerror`), else `None`.
fn process_one(vm: &mut Vm) -> Option<Completion<Value>> {
    let now = Instant::now();
    if let Some(script) = vm.events.pop_ready(now) {
        run_event(vm, &script);
        return None;
    }
    // Nothing ready: wait until the earliest timer deadline, then retry.
    if let Some(d) = vm.events.earliest_deadline() {
        let wait = d.saturating_duration_since(Instant::now());
        if !wait.is_zero() {
            std::thread::sleep(wait.min(Duration::from_millis(50)));
        }
        let now = Instant::now();
        if let Some(script) = vm.events.pop_ready(now) {
            run_event(vm, &script);
        }
    }
    None
}

/// Bare `after ms`: block, servicing events, until `ms` has elapsed.
fn delay(vm: &mut Vm, ms: u64) -> Completion<Value> {
    let end = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < end {
        if vm.events.is_empty() {
            let remaining = end.saturating_duration_since(Instant::now());
            std::thread::sleep(remaining.min(Duration::from_millis(50)));
        } else {
            process_one(vm);
        }
    }
    ok(Value::empty())
}

/// Evaluate one event script at the global level; a script error is reported as
/// a background error (C routes event-handler errors to `bgerror`), never
/// propagated out of the loop.
fn run_event(vm: &mut Vm, script: &str) {
    let failed = match vm.eval_source(script) {
        Ok(c) if c.code == Code::Error => Some(c.result.to_str().to_string()),
        Ok(_) => None,
        Err(e) => Some(e.message),
    };
    if let Some(message) = failed {
        report_bg_error(vm, &message);
    }
}

/// Report a background error the way C's `::tcl::Bgerror` does: hand it to the
/// `bgerror` handler if one is callable; otherwise print the error's `errorInfo`
/// to stderr (with the `("after" script)` frame the event raised it under). Any
/// error from the handler itself is swallowed — it must not escape the loop.
fn report_bg_error(vm: &mut Vm, message: &str) {
    let handler = vm.bgerror_handler_prefix();
    if !handler.is_empty() {
        let script = format!("{handler} {}", tcl_syntax::list::list_element(message));
        let _ = vm.eval_source(&script);
        return;
    }
    let info = vm.take_error_info().unwrap_or_else(|| message.to_string());
    eprintln!("{info}\n    (\"after\" script)");
}

// -- helpers ---------------------------------------------------------------

/// Join the script words with single spaces (a command *prefix*, exactly as C's
/// `after`/`Tcl_ConcatObj` does — NOT list-quoted, so `after 0 {set x 1}` runs
/// the body `set x 1`, and `after 0 puts hi` runs `puts hi`).
fn join_args(args: &[Value]) -> String {
    args.iter()
        .map(|v| v.to_str().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_ms(s: &str) -> Option<u64> {
    // A negative delay clamps to 0 (C treats `after -5` as `after 0`).
    s.trim()
        .parse::<i64>()
        .ok()
        .map(|n| u64::try_from(n).unwrap_or(0))
}

/// The `after#<id>` token C returns for a scheduled event.
fn after_id(id: u64) -> Value {
    Value::string(format!("after#{id}"))
}

fn parse_after_id(s: &str) -> Option<u64> {
    s.strip_prefix("after#")?.parse::<u64>().ok()
}

/// A variable's current value for `vwait` change detection (`None` = unset).
fn var_snapshot(vm: &Vm, name: &str) -> Option<String> {
    vm.get_var(name).map(|v| v.to_str().to_string())
}

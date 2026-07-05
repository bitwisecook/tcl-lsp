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

//! The Tcl event loop: `after`, `vwait`, and `update` (T-event).
//!
//! A minimal but faithful single-threaded event loop. `after` schedules timer
//! and idle events; `vwait` runs the loop until a named variable is written;
//! `update` drains the currently-ready events once. Timer events fire in
//! deadline order (then scheduling order); idle events fire after all due
//! timers. This is the scheduler half of the coroutine subsystem (`cmd_coro`):
//! `after 0 $coro` schedules a coroutine resume, and `vwait`/`update` drives it.
//!
//! C refs: `tclEvent.c` (`Tcl_DoOneEvent`, `Tcl_AfterObjCmd`, `vwait`),
//! `tclTimer.c`.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// A scheduled timer event (`after ms script`); `after 0` has a now-deadline.
struct TimerEv {
    id: u64,
    deadline: Instant,
    seq: u64,
    script: Vec<u8>,
}

/// The pending event set: due-ordered timers + a FIFO idle queue.
#[derive(Default)]
pub struct EventQueue {
    timers: Vec<TimerEv>,
    idle: VecDeque<(u64, Vec<u8>)>,
    next_id: u64,
    seq: u64,
}

impl EventQueue {
    fn fresh_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    /// Schedule a timer event `ms` from now; returns its `after#<id>` number.
    fn push_timer(&mut self, ms: u64, script: Vec<u8>) -> u64 {
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
    fn push_idle(&mut self, script: Vec<u8>) -> u64 {
        let id = self.fresh_id();
        self.idle.push_back((id, script));
        id
    }

    /// Cancel by id (`after#<id>`); returns the removed script, if any.
    fn cancel_id(&mut self, id: u64) -> Option<Vec<u8>> {
        if let Some(p) = self.timers.iter().position(|t| t.id == id) {
            return Some(self.timers.remove(p).script);
        }
        if let Some(p) = self.idle.iter().position(|(i, _)| *i == id) {
            return self.idle.remove(p).map(|(_, s)| s);
        }
        None
    }

    /// Cancel by exact script text; returns the removed script, if any.
    fn cancel_script(&mut self, script: &[u8]) -> Option<Vec<u8>> {
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
    fn pop_ready(&mut self, now: Instant) -> Option<Vec<u8>> {
        // The earliest-deadline due timer.
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
    fn pop_idle(&mut self) -> Option<Vec<u8>> {
        self.idle.pop_front().map(|(_, s)| s)
    }
}

/// Register `after`, `vwait`, and (replacing the stub) `update`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"after", after_cmd);
    interp.register_builtin(b"vwait", vwait_cmd);
    interp.register_builtin(b"update", update_cmd);
}

fn err(interp: &mut Interp, m: &[u8]) -> Code {
    interp.set_error(m)
}

/// `after ms ?script ...?` / `after idle ?script ...?` / `after cancel id|script`
/// / `after info ?id?`. With a bare `after ms` (no script) it processes events
/// until `ms` has elapsed (a delay). Multiple script args are concatenated as a
/// command prefix (C joins them with spaces).
fn after_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return err(
            interp,
            b"wrong # args: should be \"after option ?arg ...?\"",
        );
    }
    let first = obj_bytes(argv[1]);
    match first.as_slice() {
        b"idle" => {
            if argv.len() < 3 {
                return err(interp, b"wrong # args: should be \"after idle script\"");
            }
            let script = join_args(&argv[2..]);
            let id = interp.events_mut().push_idle(script);
            set_after_id(interp, id);
            Code::Ok
        }
        b"cancel" => {
            if argv.len() < 3 {
                return err(
                    interp,
                    b"wrong # args: should be \"after cancel id|command\"",
                );
            }
            // `after cancel <id>` or `after cancel <script...>`.
            let arg = obj_bytes(argv[2]);
            let by_id = parse_after_id(&arg);
            let removed = match by_id {
                Some(id) => interp.events_mut().cancel_id(id),
                None => {
                    let script = join_args(&argv[2..]);
                    interp.events_mut().cancel_script(&script)
                }
            };
            let _ = removed;
            interp.set_result_bytes(b"");
            Code::Ok
        }
        b"info" => {
            // Minimal: list pending ids, or for a given id its script.
            interp.set_result_bytes(b"");
            Code::Ok
        }
        _ => {
            // `after ms ?script?`: a non-negative integer delay.
            let ms = match parse_ms(&first) {
                Some(ms) => ms,
                None => {
                    let mut m = b"bad argument \"".to_vec();
                    m.extend_from_slice(&first);
                    m.extend_from_slice(b"\": must be cancel, idle, info, or an integer");
                    return err(interp, &m);
                }
            };
            if argv.len() == 2 {
                // Bare delay: process events until `ms` elapses (Tcl blocks the
                // event loop for the delay, still servicing other events).
                return delay(interp, ms);
            }
            let script = join_args(&argv[2..]);
            let id = interp.events_mut().push_timer(ms, script);
            set_after_id(interp, id);
            Code::Ok
        }
    }
}

/// `vwait varName` — run the event loop until `varName` is written (set/unset),
/// returning when its value changes. Returns immediately (after draining) if no
/// events remain and the variable never changes (C reports nothing to wait on).
fn vwait_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return err(interp, b"wrong # args: should be \"vwait name\"");
    }
    let var = obj_bytes(argv[1]);
    let before = read_var_snapshot(interp, &var);
    loop {
        // Stop as soon as the watched variable has changed.
        let now = read_var_snapshot(interp, &var);
        if now != before {
            break;
        }
        if interp.events_mut().is_empty() {
            // Nothing left to service and the variable hasn't changed: C would
            // report "can't wait … would wait forever"; we return so a missing
            // event source doesn't hang the test runner.
            break;
        }
        if process_one(interp) == Code::Error {
            return Code::Error;
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `update` / `update idletasks` — service the events that are ready now, then
/// return. `idletasks` services *only* idle events — it must not run timer
/// events (C's `Tcl_UpdateObjCmd`: `TCL_IDLE_EVENTS` excludes timers).
fn update_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 2 || (argv.len() == 2 && obj_bytes(argv[1]) != b"idletasks") {
        return err(interp, b"wrong # args: should be \"update ?idletasks?\"");
    }
    let idletasks = argv.len() == 2;
    interp.process_bg_errors();
    // Drain everything ready *now* (does not wait for future timers); for
    // `idletasks`, only idle handlers.
    let now = Instant::now();
    loop {
        let script = if idletasks {
            interp.events_mut().pop_idle()
        } else {
            interp.events_mut().pop_ready(now)
        };
        let Some(script) = script else { break };
        if run_event(interp, &script) == Code::Error {
            return Code::Error;
        }
    }
    interp.process_bg_errors();
    interp.set_result_bytes(b"");
    Code::Ok
}

/// Process exactly one event, sleeping until the earliest timer is due if no
/// event is ready yet. Returns `Ok` even when nothing ran.
fn process_one(interp: &mut Interp) -> Code {
    let now = Instant::now();
    let script = interp.events_mut().pop_ready(now);
    if let Some(script) = script {
        return run_event(interp, &script);
    }
    // Nothing ready: wait until the earliest timer deadline, then retry.
    let deadline = interp.events_mut().earliest_deadline();
    if let Some(d) = deadline {
        let wait = d.saturating_duration_since(Instant::now());
        if !wait.is_zero() {
            std::thread::sleep(wait.min(Duration::from_millis(50)));
        }
        let now = Instant::now();
        let script = interp.events_mut().pop_ready(now);
        if let Some(script) = script {
            return run_event(interp, &script);
        }
    }
    Code::Ok
}

/// Evaluate one event script at the global level; a script error is reported as
/// a background error (C's event handlers route errors to `bgerror`).
fn run_event(interp: &mut Interp, script: &[u8]) -> Code {
    let code = interp.eval_str(script);
    if code == Code::Error {
        let msg = interp.result_bytes();
        // Simple-word options list (no special chars → space-join is valid Tcl).
        interp.report_bg_error(&msg, b"-code 1 -level 0");
        interp.process_bg_errors();
    }
    Code::Ok
}

/// Bare `after ms`: block, servicing events, until `ms` has elapsed.
fn delay(interp: &mut Interp, ms: u64) -> Code {
    let end = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < end {
        if interp.events_mut().is_empty() {
            let remaining = end.saturating_duration_since(Instant::now());
            std::thread::sleep(remaining.min(Duration::from_millis(50)));
        } else if process_one(interp) == Code::Error {
            return Code::Error;
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

// -- helpers ---------------------------------------------------------------

fn join_args(args: &[*mut TclObj]) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, &a) in args.iter().enumerate() {
        if i > 0 {
            out.push(b' ');
        }
        out.extend_from_slice(&obj_bytes(a));
    }
    out
}

fn parse_ms(s: &[u8]) -> Option<u64> {
    core::str::from_utf8(s)
        .ok()?
        .trim()
        .parse::<i64>()
        .ok()
        .map(|n| n.max(0) as u64)
}

/// Set the result to the `after#<id>` token C returns for a scheduled event.
fn set_after_id(interp: &mut Interp, id: u64) {
    let mut t = b"after#".to_vec();
    t.extend_from_slice(id.to_string().as_bytes());
    interp.set_result_bytes(&t);
}

fn parse_after_id(s: &[u8]) -> Option<u64> {
    let rest = s.strip_prefix(b"after#")?;
    core::str::from_utf8(rest).ok()?.parse::<u64>().ok()
}

/// Read a variable's current `(exists, value)` snapshot for `vwait` change
/// detection (a missing variable and an empty one are distinguished).
fn read_var_snapshot(interp: &mut Interp, name: &[u8]) -> Option<Vec<u8>> {
    interp.var_get(name).map(obj_bytes)
}

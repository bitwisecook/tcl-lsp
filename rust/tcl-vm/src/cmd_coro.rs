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

//! Coroutines for the bytecode VM (`RUST_ISSUE_008`).
//!
//! The tree-walking runtime backs `coroutine`/`yield` with **one OS thread per
//! coroutine** (a parked native stack is the continuation) because it has no
//! explicit call stack. The VM already runs an explicit-stack NRE trampoline
//! ([`Vm::drive`](crate::interp::Vm)), so a coroutine's continuation is just its
//! **frozen activation stack (`Vec<Frame>`) plus a saved per-flow context**
//! ([`ParkedFlow`]) — pure data, no OS threads, no `unsafe`.
//!
//! - `yield` is a builtin that records a [`YieldReq`] in [`CoroSystem::pending`];
//!   the trampoline's `dispatch_words` turns it into a `Tick::Suspend` that
//!   freezes the stack (mirroring how `tailcall` becomes `Tick::Tailcall`).
//! - [`resume`] swaps the coroutine's flow in ([`Vm::swap_flow`]), pushes the
//!   resume value where `yield`'s result belongs, drives until the next suspend
//!   or completion, then swaps the resumer's flow back.
//! - A `yield` can only cross the *explicit* stack. Reaching one across a host
//!   re-entry (`catch`/`uplevel`/`eval`/`lsort -command`/an OO method) is
//!   rejected with C Tcl's `cannot yield: C stack busy`, detected by comparing
//!   [`Vm::activation_depth`](crate::interp::Vm) against the driver's base depth.

use std::collections::HashMap;

use tcl_runtime_api::Completion;

use crate::command::Command;
use crate::exec::{Frame, RunExit, YieldReq};
use crate::interp::{ParkedFlow, Vm, err, ok};
use crate::value::Value;

/// The coroutine subsystem held by the [`Vm`].
#[derive(Default)]
pub(crate) struct CoroSystem {
    /// Live coroutines, keyed by fully-qualified name.
    live: HashMap<String, CoroState>,
    /// Active drivers (innermost last) — one entry per in-flight `resume`. Drives
    /// `[info coroutine]` and the yield-boundary check.
    stack: Vec<CoroHandle>,
    /// A `yield`/`yieldto` request set by the builtin, drained into a
    /// `Tick::Suspend` by `dispatch_words`.
    pub(crate) pending: Option<YieldReq>,
}

/// One suspended (or in-flight) coroutine: its frozen activation stack and the
/// per-flow context to reinstate on resume.
struct CoroState {
    /// The frozen bytecode-activation stack — the continuation.
    acts: Vec<Frame>,
    /// The saved per-flow execution context (call/ns tails, error/script state).
    parked: ParkedFlow,
    status: CoroStatus,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CoroStatus {
    /// Created but not yet run (the first `resume` starts the body at pc 0).
    Fresh,
    /// Parked at a `yield` (the next `resume` delivers a value there).
    Suspended,
    /// Currently being driven (its `acts`/`parked` are moved out) — a nested
    /// re-entrant `resume` of the same coroutine is an error.
    Running,
}

/// A driver frame for an in-flight `resume`: which coroutine, and the
/// `activation_depth` its `drive` started at (for the yield-boundary check).
struct CoroHandle {
    name: String,
    base_depth: usize,
}

pub(crate) fn register(vm: &mut Vm) {
    vm.register("coroutine", cmd_coroutine);
    vm.register("yield", cmd_yield);
    vm.register("yieldto", cmd_yieldto);
}

/// `[info coroutine]` — the fully-qualified name of the innermost running
/// coroutine (display form), or `""` at top level.
pub(crate) fn current_coroutine(vm: &Vm) -> Value {
    match vm.coro.stack.last() {
        Some(h) => Value::string(format!("::{}", h.name)),
        None => Value::empty(),
    }
}

/// Whether `fqn` (canonical) names a live coroutine — used by `rename`/deletion
/// to tear one down.
pub(crate) fn is_coroutine(vm: &Vm, fqn: &str) -> bool {
    vm.coro.live.contains_key(fqn)
}

/// Drop a coroutine's state on command deletion (`rename $coro {}`). The
/// continuation is pure data, so teardown is a plain remove — no `finally`
/// blocks run, matching C Tcl. The command itself is removed by the caller.
pub(crate) fn on_command_deleted(vm: &mut Vm, fqn: &str) {
    vm.coro.live.remove(fqn);
}

/// Move a coroutine's state to a new key on `rename $coro $new`.
pub(crate) fn on_command_renamed(vm: &mut Vm, old_fqn: &str, new_fqn: &str) {
    if let Some(state) = vm.coro.live.remove(old_fqn) {
        vm.coro.live.insert(new_fqn.to_string(), state);
    }
}

/// `coroutine name command ?arg ...?` — create a coroutine that runs
/// `command arg…`, register `name` as its resume command, and run it to the
/// first `yield` (or completion), returning that value.
fn cmd_coroutine(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.len() < 2 {
        return err("wrong # args: should be \"coroutine name command ?arg ...?\"");
    }
    let name = args[0].to_str();
    let fqn = vm.qualify_name(&name);
    if vm.lookup_command(&fqn).is_some() {
        return err(format!(
            "can't create procedure \"{name}\": command already exists"
        ));
    }
    // The body is `command arg…` reconstructed as a one-line script (list
    // quoting preserves the words exactly), dispatched through the compiled
    // `INVOKE` path so a proc call stays on the coroutine's explicit stack.
    let body_src = Value::list(args[1..].to_vec()).to_str();
    let Some(body) = vm.compile_dynamic_body(&body_src) else {
        return err(format!("coroutine \"{name}\": could not compile body"));
    };
    vm.coro.live.insert(
        fqn.clone(),
        CoroState {
            acts: vec![Frame::new(body, false)],
            parked: ParkedFlow::default(),
            status: CoroStatus::Fresh,
        },
    );
    vm.register_command(&fqn, Command::Builtin(coro_resume));
    resume(vm, &fqn, None)
}

/// The builtin the coroutine command name resolves to: `$coro ?value?` resumes
/// it, delivering `value` as the result of the parked `yield`.
fn coro_resume(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    // Read the invocation name before `resume` swaps context (which overwrites
    // `invoked_name` with the coroutine's saved value).
    let Some(invoked) = vm.invoked_name().map(str::to_owned) else {
        return err("invalid command name");
    };
    let fqn = vm.qualify_name(&invoked);
    if args.len() > 1 {
        return err(format!("wrong # args: should be \"{invoked} ?arg?\""));
    }
    resume(vm, &fqn, args.first().cloned())
}

/// Resume the coroutine `fqn`, delivering `value` (the parked `yield`'s result;
/// `None` on the first run). Returns the yielded value, or — on completion — the
/// body's result with its code.
fn resume(vm: &mut Vm, fqn: &str, value: Option<Value>) -> Completion<Value> {
    // Borrow choreography: mark the entry Running and move its `acts`/`parked`
    // out (a `Running` sentinel stays in the map) so no `coro.live` borrow is
    // held across the `&mut self` drive.
    let (mut acts, mut parked, was_fresh) = {
        let Some(state) = vm.coro.live.get_mut(fqn) else {
            return err(format!("invalid command name \"{fqn}\""));
        };
        if state.status == CoroStatus::Running {
            return err(format!("coroutine \"{fqn}\" is already running"));
        }
        let was_fresh = state.status == CoroStatus::Fresh;
        state.status = CoroStatus::Running;
        (
            std::mem::take(&mut state.acts),
            std::mem::take(&mut state.parked),
            was_fresh,
        )
    };

    // Install the coroutine's flow; `parked` now holds the resumer's flow.
    vm.swap_flow(&mut parked);
    vm.coro.stack.push(CoroHandle {
        name: fqn.to_string(),
        base_depth: vm.activation_depth + 1,
    });
    // Deliver the resume value where the parked `yield`'s result belongs. A Fresh
    // coroutine starts at pc 0, so it takes no delivered value.
    if !was_fresh
        && let Some(top) = acts.last_mut()
    {
        top.push_operand(value.unwrap_or_else(Value::empty));
    }

    let exit = vm.drive_coro(&mut acts);

    vm.coro.stack.pop();
    // Swap the resumer's flow back in; `parked` again holds the coroutine's flow.
    vm.swap_flow(&mut parked);

    match exit {
        RunExit::Yielded(req) => {
            // Park the coroutine (its frozen stack + flow) for the next resume.
            if let Some(state) = vm.coro.live.get_mut(fqn) {
                state.acts = acts;
                state.parked = parked;
                state.status = CoroStatus::Suspended;
            }
            match req {
                YieldReq::Yield(v) => ok(v),
                // `yieldto cmd args`: the coroutine is now parked; run `cmd args`
                // in the (restored) resumer context and return its result.
                YieldReq::YieldTo(words) => {
                    let name = words[0].to_str().to_string();
                    vm.invoke_command(&name, &words[1..])
                }
            }
        }
        RunExit::Done(c) => {
            // The body finished: remove the command + state (unless `exit` is
            // propagating, in which case leave teardown to the unwinding caller).
            if !vm.exit_pending() {
                vm.take_command(fqn);
            }
            vm.coro.live.remove(fqn);
            c
        }
    }
}

/// `yield ?value?` — suspend the current coroutine.
fn cmd_yield(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.len() > 1 {
        return err("wrong # args: should be \"yield ?value?\"");
    }
    if let Err(e) = check_yieldable(vm, "yield") {
        return e;
    }
    let value = args.first().cloned().unwrap_or_else(Value::empty);
    vm.coro.pending = Some(YieldReq::Yield(value));
    ok(Value::empty())
}

/// `yieldto command ?arg ...?` — suspend, then resume runs `command args` in the
/// resumer's context.
fn cmd_yieldto(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.is_empty() {
        return err("wrong # args: should be \"yieldto command ?arg ...?\"");
    }
    if let Err(e) = check_yieldable(vm, "yieldto") {
        return e;
    }
    vm.coro.pending = Some(YieldReq::YieldTo(args.to_vec()));
    ok(Value::empty())
}

/// Reject a `yield`/`yieldto` that is not in a coroutine, or that sits across a
/// host re-entry (`catch`/`uplevel`/`eval`/`lsort -command`/OO method) — the
/// non-yieldable NRE boundary C Tcl reports as `cannot yield: C stack busy`.
fn check_yieldable(vm: &Vm, verb: &str) -> Result<(), Completion<Value>> {
    match vm.coro.stack.last() {
        None => Err(err(format!("{verb} can only be called in a coroutine"))),
        Some(h) if vm.activation_depth != h.base_depth => Err(err("cannot yield: C stack busy")),
        Some(_) => Ok(()),
    }
}

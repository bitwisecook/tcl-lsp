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

//! Coroutines for the bytecode VM.
//!
//! The tree-walking runtime backs `coroutine`/`yield` with **one OS thread per
//! coroutine** (a parked native stack is the continuation) because it has no
//! explicit call stack. The VM already runs an explicit-stack NRE trampoline
//! ([`Vm::drive`](crate::interp::Vm)), so a coroutine's continuation is just its
//! **frozen activation stack (`Vec<Frame>`) plus a saved per-flow context**
//! ([`ParkedFlow`]) — pure data, no OS threads, no `unsafe`.
//!
//! - `yield` is a builtin that records a [`YieldReq`] in [`CoroSystem::pending`]
//!   (via [`request_yield`]); the trampoline's `dispatch_words` turns it into a
//!   `Tick::Suspend` that freezes the stack (mirroring how `tailcall` becomes
//!   `Tick::Tailcall`). The compiled `yield`/`yieldToInvoke` opcodes record the
//!   same request through the same core and drain it themselves, so the
//!   boundary checks and the suspend machinery cannot drift between the two.
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
use crate::interp::{CommandSidecarHandle, CommandSidecarKey, ParkedFlow, Vm, err, ok};
use crate::value::Value;

/// The coroutine subsystem held by the [`Vm`].
#[derive(Default)]
pub(crate) struct CoroSystem {
    /// Live coroutines, keyed by fully-qualified name.
    live: HashMap<CommandSidecarKey, CoroState>,
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
    /// How the coroutine last suspended — `yield` vs `yieldto`. Reported by
    /// `::tcl::unsupported::corotype` when the coroutine is [`CoroStatus::Suspended`].
    last_suspend: SuspendKind,
    /// Pending one-shot `coroinject` commands. At the next resume each runs in the
    /// coroutine's context as `cmd… <kind> <resumeValue>`, and its result becomes
    /// the value the parked `yield` returns (FIFO).
    injections: Vec<Vec<Value>>,
    /// For a `coroutine … apply {lambda} …`, the internal proc the lambda was
    /// bound to (so its body runs on this coroutine's explicit stack). Deleted
    /// with the coroutine — its lifetime is the coroutine's, not one call's.
    temp_proc: Option<String>,
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

/// Which suspend primitive last parked a coroutine (for `corotype`).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum SuspendKind {
    #[default]
    Yield,
    YieldTo,
}

/// A driver frame for an in-flight `resume`: which coroutine, and the
/// `activation_depth` its `drive` started at (for the yield-boundary check).
struct CoroHandle {
    key: CommandSidecarHandle,
    base_depth: usize,
}

pub(crate) fn register(vm: &mut Vm) {
    vm.register("coroutine", cmd_coroutine);
    vm.register("yield", cmd_yield);
    vm.register("yieldto", cmd_yieldto);
    vm.register("coroprobe", cmd_coroprobe);
    vm.register("coroinject", cmd_coroinject);
    vm.register("::tcl::unsupported::corotype", cmd_corotype);
}

/// `[info coroutine]` — the fully-qualified name of the innermost running
/// coroutine (display form), or `""` at top level.
pub(crate) fn current_coroutine(vm: &Vm) -> Value {
    match vm.coro.stack.last() {
        // A coroutine that deleted its own command (`rename [info coroutine] {}`)
        // is no longer live even though its driver is still on the stack; C Tcl
        // then reports `[info coroutine]` as empty (coroutine-3.5).
        Some(h) => match h.key.key() {
            Some(key) if vm.coro.live.contains_key(&key) => {
                Value::string(format!("::{}", key.name()))
            }
            _ => Value::empty(),
        },
        _ => Value::empty(),
    }
}

/// Whether `fqn` (canonical) names a live coroutine — used by `rename`/deletion
/// to tear one down.
pub(crate) fn is_coroutine(vm: &Vm, fqn: &str) -> bool {
    vm.coro.live.contains_key(&CommandSidecarKey::visible(fqn))
}

/// Drop a coroutine's state on command deletion (`rename $coro {}`). The
/// continuation is pure data, so teardown is a plain remove — no `finally`
/// blocks run, matching C Tcl. The command itself is removed by the caller; an
/// `apply` coroutine's bound lambda proc is removed here (its lifetime is the
/// coroutine's).
pub(crate) fn on_command_deleted(vm: &mut Vm, fqn: &str) {
    let Some(mut state) = vm.coro.live.remove(&CommandSidecarKey::visible(fqn)) else {
        return;
    };
    // A suspended coroutine's locals are about to disappear with its frozen
    // frames; fire their `unset` traces first (C unsets a deleted coroutine's
    // variables). A running coroutine's frames are on the live stack instead, so
    // their traces fire the normal way as those frames pop.
    if state.status == CoroStatus::Suspended {
        vm.fire_parked_unset_traces(&mut state.parked);
    }
    if let Some(p) = state.temp_proc {
        vm.take_command_unchecked(&p);
    }
}

/// Move a coroutine's state to a new key on `rename $coro $new`.
pub(crate) fn on_command_renamed(vm: &mut Vm, old_fqn: &str, new_fqn: &str) {
    if let Some(state) = vm.coro.live.remove(&CommandSidecarKey::visible(old_fqn)) {
        vm.coro
            .live
            .insert(CommandSidecarKey::visible(new_fqn), state);
    }
}

pub(crate) fn on_command_hidden(vm: &mut Vm, old_fqn: &str, token: &str) {
    if let Some(state) = vm.coro.live.remove(&CommandSidecarKey::visible(old_fqn)) {
        vm.coro.live.insert(CommandSidecarKey::hidden(token), state);
    }
}

pub(crate) fn on_command_exposed(vm: &mut Vm, token: &str, new_fqn: &str) {
    if let Some(state) = vm.coro.live.remove(&CommandSidecarKey::hidden(token)) {
        vm.coro
            .live
            .insert(CommandSidecarKey::visible(new_fqn), state);
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
    // C's `coroutine` (re)creates the command, *replacing* whatever already
    // exists under that name (a proc, or a leftover coroutine) — it does not
    // error. A live coroutine at that name has its captured state torn down
    // first so it does not leak; the command entry itself is overwritten by
    // `register_command` below.
    if is_coroutine(vm, &fqn) {
        vm.detach_active_sidecars(&CommandSidecarKey::visible(&fqn));
        on_command_deleted(vm, &fqn);
    }
    // `coroutine NAME apply {lambda} arg…` — bind the lambda to an internal proc
    // and run *that* (`lambdaProc arg…`), so the lambda body executes on this
    // coroutine's explicit activation stack and a `yield` inside it is yieldable.
    // The generic `apply` builtin would run the body through a host-stack
    // re-entry, where a `yield` cannot cross. The proc is torn down with the
    // coroutine (`temp_proc`). Everything else runs `command arg…` verbatim.
    // Both spellings of the command word are accepted — bare `apply` and the
    // fully-qualified `::apply` (the form the C `coroutine.test` idiom uses).
    let is_apply = matches!(&*args[1].to_str(), "apply" | "::apply");
    let mut temp_proc = None;
    let words: Vec<Value> = if args.len() >= 3 && is_apply {
        match crate::command::build_lambda_proc(vm, &args[2]) {
            Ok(lambda_name) => {
                let mut w = Vec::with_capacity(args.len() - 2);
                w.push(Value::string(lambda_name.as_str()));
                w.extend_from_slice(&args[3..]);
                temp_proc = Some(lambda_name);
                w
            }
            Err(c) => return c,
        }
    } else {
        let mut w = args[1..].to_vec();
        // C resolves a coroutine's initial command at creation, in the current
        // namespace. When that namespace is not global, resolve the command word
        // there and run its absolute name — the wrapper itself executes at the
        // global level, so a bare name would otherwise miss a namespace-local
        // command (coroutine-4.4).
        let cxt = vm.current_ns().to_string();
        if !cxt.is_empty()
            && let Some(fqn) = vm.resolve_command_fqn(&cxt, &args[1].to_str())
        {
            w[0] = Value::string(format!("::{fqn}"));
        }
        w
    };
    // The body is `command arg…` reconstructed as a one-line script (list
    // quoting preserves the words exactly), dispatched through the compiled
    // `INVOKE` path so a proc call stays on the coroutine's explicit stack.
    let body_src = Value::list(words).to_str();
    let Some(body) = vm.compile_dynamic_body(&body_src) else {
        if let Some(p) = &temp_proc {
            vm.take_command_unchecked(p);
        }
        return err(format!("coroutine \"{name}\": could not compile body"));
    };
    vm.coro.live.insert(
        CommandSidecarKey::visible(&fqn),
        CoroState {
            acts: vec![Frame::new(body, false)],
            parked: ParkedFlow::default(),
            status: CoroStatus::Fresh,
            last_suspend: SuspendKind::Yield,
            injections: Vec::new(),
            temp_proc,
        },
    );
    vm.register_command(&fqn, Command::Builtin(coro_resume));
    resume(vm, &CommandSidecarKey::visible(&fqn), &[], &name)
}

/// The builtin the coroutine command name resolves to: `$coro ?value ...?`
/// resumes it, delivering the value(s) as the result of the parked
/// `yield`/`yieldto`.
fn coro_resume(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    // Read the invocation name before `resume` swaps context (which overwrites
    // `invoked_name` with the coroutine's saved value).
    let Some(invoked) = vm.invoked_name().map(str::to_owned) else {
        return err("invalid command name");
    };
    let key = vm
        .take_invoked_sidecar()
        .unwrap_or_else(|| CommandSidecarKey::visible(vm.qualify_name(&invoked)));
    resume(vm, &key, args, &invoked)
}

/// Resume the coroutine `fqn`, delivering `args` as the result of the parked
/// suspend point. A `yield`-suspended coroutine takes at most one value; a
/// `yieldto`-suspended one accepts any number, delivered as a list. `args` is
/// empty on the first run; `display_name` names the resume command for the
/// arity error. Returns the yielded value, or — on completion — the body's
/// result with its code.
fn resume(
    vm: &mut Vm,
    key: &CommandSidecarKey,
    args: &[Value],
    display_name: &str,
) -> Completion<Value> {
    // Validate the resume arity against how the coroutine last suspended, before
    // the borrow choreography, so an error leaves the coroutine untouched.
    match vm.coro.live.get(key) {
        None => return err(format!("invalid command name \"{}\"", key.name())),
        Some(s) if s.status == CoroStatus::Running => {
            return err(format!("coroutine \"{}\" is already running", key.name()));
        }
        Some(s)
            if s.status == CoroStatus::Suspended
                && s.last_suspend == SuspendKind::Yield
                && args.len() > 1 =>
        {
            return err(format!("wrong # args: should be \"{display_name} ?arg?\""));
        }
        Some(_) => {}
    }
    // Borrow choreography: mark the entry Running and move its `acts`/`parked`
    // out (a `Running` sentinel stays in the map) so no `coro.live` borrow is
    // held across the `&mut self` drive.
    let (mut acts, mut parked, was_fresh, injections, kind) = {
        let state = vm.coro.live.get_mut(key).expect("presence checked above");
        let was_fresh = state.status == CoroStatus::Fresh;
        state.status = CoroStatus::Running;
        (
            std::mem::take(&mut state.acts),
            std::mem::take(&mut state.parked),
            was_fresh,
            std::mem::take(&mut state.injections),
            state.last_suspend,
        )
    };

    // Install the coroutine's flow; `parked` now holds the resumer's flow.
    vm.swap_flow(&mut parked);
    // `activation_depth` lives on the engine (`Vm`) and `coro` on the current
    // interp state (reached through `Deref`), so read the depth into a local
    // before the push to avoid overlapping the whole-`Vm` deref borrow.
    let base_depth = vm.activation_depth + 1;
    let active_key = vm.active_sidecar(key.clone());
    vm.coro.stack.push(CoroHandle {
        key: active_key.clone(),
        base_depth,
    });
    // Deliver the resume value where the parked `yield`'s result belongs. A Fresh
    // coroutine starts at pc 0, so it takes no delivered value. Any `coroinject`
    // commands run first, in the coroutine's context, each transforming the
    // delivered value (called with the suspend kind + current value appended).
    if !was_fresh {
        // A `yield` returns the single resume value (or `""`); a `yieldto`
        // returns the whole resume-argument list.
        let initial = match kind {
            SuspendKind::Yield => args.first().cloned().unwrap_or_else(Value::empty),
            SuspendKind::YieldTo => Value::list(args.to_vec()),
        };
        match run_injections(vm, injections, kind, initial) {
            Ok(delivered) => {
                if let Some(top) = acts.last_mut() {
                    top.push_operand(delivered);
                }
            }
            Err(r) => {
                // An injected command failed: the coroutine is torn down (C
                // unwinds it, so its command becomes invalid), the caller's flow
                // is restored, and the error propagates.
                vm.coro.stack.pop();
                vm.swap_flow(&mut parked);
                if let Some(key) = active_key.key() {
                    teardown_coro(vm, &key);
                }
                return r;
            }
        }
    }

    let exit = vm.drive_coro(&mut acts);

    vm.coro.stack.pop();
    // Swap the resumer's flow back in; `parked` again holds the coroutine's flow.
    vm.swap_flow(&mut parked);
    let key = active_key.key();

    match exit {
        RunExit::Yielded(req) => {
            let kind = match &req {
                YieldReq::Yield(_) => SuspendKind::Yield,
                YieldReq::YieldTo(_) => SuspendKind::YieldTo,
            };
            // Park the coroutine (its frozen stack + flow) for the next resume.
            if let Some(key) = key
                && let Some(state) = vm.coro.live.get_mut(&key)
            {
                state.acts = acts;
                state.parked = parked;
                state.status = CoroStatus::Suspended;
                state.last_suspend = kind;
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
            if let Some(key) = key {
                teardown_coro(vm, &key);
            }
            c
        }
    }
}

/// Run the pending [`coroinject`](cmd_coroinject) commands in the (already
/// swapped-in) coroutine context, FIFO. Each command prefix is invoked as
/// `cmd args… <kind> <delivered>` — the suspend kind (`yield`/`yieldto`) and the
/// value the parked `yield` is about to return — and its result becomes the new
/// delivered value threaded into the next injection. A non-`ok` completion
/// aborts the chain; [`resume`] then unwinds the coroutine.
fn run_injections(
    vm: &mut Vm,
    injections: Vec<Vec<Value>>,
    kind: SuspendKind,
    mut delivered: Value,
) -> Result<Value, Completion<Value>> {
    let kind_str = match kind {
        SuspendKind::Yield => "yield",
        SuspendKind::YieldTo => "yieldto",
    };
    for prefix in injections {
        // `prefix` is `cmd arg…` (never empty — `coroinject` requires a command);
        // append the suspend kind and the current delivered value.
        let mut call = prefix[1..].to_vec();
        call.push(Value::string(kind_str));
        call.push(delivered.clone());
        let name = prefix[0].to_str().to_string();
        let c = vm.invoke_command(&name, &call);
        if c.code.is_ok() {
            delivered = c.result;
        } else {
            return Err(c);
        }
    }
    Ok(delivered)
}

/// Drop a finished or unwound coroutine: remove its command and, for an `apply`
/// coroutine, its bound lambda proc — unless `exit` is propagating, when the
/// unwinding caller tears everything down instead.
fn teardown_coro(vm: &mut Vm, key: &CommandSidecarKey) {
    if !vm.exit_pending() {
        vm.retire_command_lifecycle_key(key);
    }
    if let Some(state) = vm.coro.live.remove(key)
        && let Some(p) = state.temp_proc
    {
        vm.take_command_unchecked(&p);
    }
}

/// Record a `yield` request — the core the `yield` builtin and the `YIELD`
/// opcode share. Checks the yield boundary first, so a rejected yield leaves
/// [`CoroSystem::pending`] untouched; on success the caller (the builtin's
/// dispatch, or the opcode arm itself) drains the request into a
/// `Tick::Suspend`.
pub(crate) fn request_yield(vm: &mut Vm, value: Value) -> Result<(), Completion<Value>> {
    check_yieldable(vm, "yield")?;
    vm.coro.pending = Some(YieldReq::Yield(value));
    Ok(())
}

/// Record a `yieldto` request (`words` is `command arg…`) — the core the
/// `yieldto` builtin and the `YIELD_TO_INVOKE` opcode share. See
/// [`request_yield`].
pub(crate) fn request_yieldto(vm: &mut Vm, words: &[Value]) -> Result<(), Completion<Value>> {
    if words.is_empty() {
        return Err(err("wrong # args: should be \"yieldto command ?arg ...?\""));
    }
    check_yieldable(vm, "yieldto")?;
    vm.coro.pending = Some(YieldReq::YieldTo(words.to_vec()));
    Ok(())
}

/// `yield ?value?` — suspend the current coroutine.
fn cmd_yield(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.len() > 1 {
        return err("wrong # args: should be \"yield ?value?\"");
    }
    let value = args.first().cloned().unwrap_or_else(Value::empty);
    match request_yield(vm, value) {
        Ok(()) => ok(Value::empty()),
        Err(e) => e,
    }
}

/// `yieldto command ?arg ...?` — suspend, then resume runs `command args` in the
/// resumer's context.
fn cmd_yieldto(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match request_yieldto(vm, args) {
        Ok(()) => ok(Value::empty()),
        Err(e) => e,
    }
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

/// `coroprobe coroName cmd ?arg ...?` — evaluate `cmd args` in the **suspended**
/// coroutine's own context (its call frame + namespace) *without* resuming it,
/// and return the result. C Tcl's synchronous coroutine-introspection primitive.
fn cmd_coroprobe(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [name, cmd, rest @ ..] = args else {
        return err("wrong # args: should be \"coroprobe coroName cmd ?arg1 arg2 ...?\"");
    };
    let fqn = vm.qualify_name(&name.to_str());
    // Take the coroutine's parked flow out; it must be suspended.
    let mut parked = {
        let Some(state) = vm.coro.live.get_mut(&CommandSidecarKey::visible(&fqn)) else {
            return err("can only inject a probe command into a coroutine");
        };
        if state.status != CoroStatus::Suspended {
            return err("can only inject a probe command into a coroutine");
        }
        state.status = CoroStatus::Running;
        std::mem::take(&mut state.parked)
    };
    // Install the coroutine's flow, run the probe in it (so `[info coroutine]`
    // and its locals resolve), then restore the caller's flow.
    vm.swap_flow(&mut parked);
    let base_depth = vm.activation_depth + 1;
    let active_key = vm.active_sidecar(CommandSidecarKey::visible(&fqn));
    vm.coro.stack.push(CoroHandle {
        key: active_key.clone(),
        base_depth,
    });
    let result = vm.invoke_command(&cmd.to_str(), rest);
    vm.coro.stack.pop();
    vm.swap_flow(&mut parked);
    if let Some(key) = active_key.key()
        && let Some(state) = vm.coro.live.get_mut(&key)
    {
        state.parked = parked;
        state.status = CoroStatus::Suspended;
    }
    result
}

/// `coroinject coroName cmd ?arg ...?` — schedule `cmd args` to run in the
/// coroutine's context at its next resume, *before* the parked `yield` returns.
/// The injected command is called with two extra trailing arguments — the
/// suspend kind (`yield`/`yieldto`) and the resume value — and its result
/// becomes what that `yield` returns. One-shot, FIFO (see [`resume`]).
fn cmd_coroinject(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [name, _cmd, _rest @ ..] = args else {
        return err("wrong # args: should be \"coroinject coroName cmd ?arg1 arg2 ...?\"");
    };
    let fqn = vm.qualify_name(&name.to_str());
    match vm.coro.live.get_mut(&CommandSidecarKey::visible(&fqn)) {
        Some(state) if state.status == CoroStatus::Suspended => {
            state.injections.push(args[1..].to_vec());
            ok(Value::empty())
        }
        _ => err("can only inject a command into a coroutine"),
    }
}

/// `::tcl::unsupported::corotype coroName` — the coroutine's current type:
/// `active` while it is running, otherwise how it last suspended (`yield` /
/// `yieldto`).
fn cmd_corotype(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [name] = args else {
        return err("wrong # args: should be \"::tcl::unsupported::corotype coroName\"");
    };
    let fqn = vm.qualify_name(&name.to_str());
    match vm.coro.live.get(&CommandSidecarKey::visible(&fqn)) {
        None => err("can only get coroutine type of a coroutine"),
        Some(s) if s.status == CoroStatus::Running => ok(Value::string("active")),
        Some(s) => ok(Value::string(match s.last_suspend {
            SuspendKind::Yield => "yield",
            SuspendKind::YieldTo => "yieldto",
        })),
    }
}

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

use std::{collections::HashMap, rc::Rc};

use tcl_runtime_api::Completion;

use crate::command::{Command, NativeCommand};
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

impl CoroSystem {
    #[cfg(test)]
    pub(crate) fn parked_lambda_registrations(&self) -> Vec<CommandSidecarHandle> {
        self.live
            .values()
            .flat_map(|state| &state.acts)
            .filter_map(Frame::lambda_registration)
            .cloned()
            .collect()
    }
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
    /// the value the parked `yield` returns (newest callback first).
    injections: Vec<Vec<Value>>,
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

/// Script-created coroutine resume commands are native handlers but not
/// registry builtin implementations. Keeping that distinction in the command
/// type makes compiled binding validation reject a coroutine that replaces a
/// specialised command name before its first activation.
struct CoroResumeCommand;

impl NativeCommand for CoroResumeCommand {
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        coro_resume(vm, args)
    }
}

pub(crate) fn register(vm: &mut Vm) {
    vm.register_stock_builtin("coroutine", cmd_coroutine);
    vm.register_stock_builtin("yield", cmd_yield);
    vm.register_stock_builtin("yieldto", cmd_yieldto);
    vm.register_stock_builtin("coroprobe", cmd_coroprobe);
    vm.register_stock_builtin("coroinject", cmd_coroinject);
    vm.register_stock_builtin("::tcl::unsupported::corotype", cmd_corotype);
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
                Value::from_string_bytes(vm.rooted_command_sidecar_display_bytes(&key))
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

/// Drop a coroutine's state on command deletion (`rename $coro {}`).
/// No finally blocks run. The actual parked activation owners retire at the
/// shared teardown boundary; command publication is removed by the caller.
pub(crate) fn on_command_deleted(vm: &mut Vm, key: &CommandSidecarKey) {
    let Some(mut state) = vm.coro.live.remove(key) else {
        return;
    };
    // A suspended coroutine's locals are about to disappear with its frozen
    // frames; fire their `unset` traces first (C unsets a deleted coroutine's
    // variables). A running coroutine's frames are on the live stack instead, so
    // their traces fire the normal way as those frames pop.
    if state.status == CoroStatus::Suspended {
        vm.fire_parked_unset_traces(&mut state.parked, &mut state.acts);
    }
    vm.retire_parked_lambda_registrations(&mut state.acts);
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
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"coroutine name command ?arg ...?\"",
        );
    }
    let slot = match vm.native_coroutine_publication_slot(&args[0]) {
        Ok(slot) => slot,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    // C's `coroutine` (re)creates the command, *replacing* whatever already
    // exists under that name (a proc, or a leftover coroutine) — it does not
    // error. `register_command` below is the single lifecycle owner: it tears
    // down a replaced coroutine's sidecar and saved state before publishing the
    // new resume command.
    let words = args[1..].to_vec();
    let namespace = match tcl_cmd_core::namespace::current_original(vm) {
        Ok(namespace) => namespace,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let Some(protocol) = vm
        .actual_native_invocation_dialect()
        .native_string_protocol()
    else {
        return vm.refuse_host_command("original coroutine invocation List issuer".into());
    };
    let mut original = Vec::with_capacity(words.len() + 1);
    original.push(namespace);
    original.extend(words);
    let original = Value::native_list_constructor(original, protocol);
    let asm = Rc::new(tcl_bytecode::FunctionAsm {
        instructions: vec![tcl_bytecode::Instruction::new(
            tcl_bytecode::Op::DONE,
            vec![],
        )],
        ..tcl_bytecode::FunctionAsm::default()
    });
    // The new flow starts on the actual shared root frame. Its original List
    // separately retains the caller namespace for the first command lookup.
    let source_namespace = vm.namespace_path_for_token(tcl_core_types::ROOT_NS);
    let driver = vm.admitted_foreign_unit(asm, source_namespace);
    let frame = Frame::new_original_invocation(driver, original);
    // Publish the resume command before attaching its fresh coroutine state.
    // `register_command` retires any command it replaces, including an old
    // coroutine at this name; attaching first would let that replacement
    // teardown mistake the new state for the old command's state and remove
    // it. Once the replacement is complete, install the state atomically from
    // the coroutine subsystem's perspective and start it below.
    let fqn = vm.register_command_in_slot(slot, Command::Native(Rc::new(CoroResumeCommand)));
    vm.coro.live.insert(
        CommandSidecarKey::visible(&fqn),
        CoroState {
            acts: vec![frame],
            parked: ParkedFlow::default(),
            status: CoroStatus::Fresh,
            last_suspend: SuspendKind::Yield,
            injections: Vec::new(),
        },
    );
    resume(vm, &CommandSidecarKey::visible(&fqn), &[], &args[0])
}

/// The builtin the coroutine command name resolves to: `$coro ?value ...?`
/// resumes it, delivering the value(s) as the result of the parked
/// `yield`/`yieldto`.
fn coro_resume(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    // Lookup has already selected the command token. The original argv[0]
    // supplies diagnostics; a text representation cannot identify that token.
    let Some(invoked) = vm.invoked_name_value() else {
        return vm.refuse_host_command("original coroutine invocation operand".into());
    };
    let Some(key) = vm.take_invoked_sidecar() else {
        return vm.refuse_host_command("selected coroutine command token".into());
    };
    resume(vm, &key, args, &invoked)
}

fn validate_resume_arity(
    vm: &mut Vm,
    key: &CommandSidecarKey,
    args: &[Value],
    original_name: &Value,
) -> Result<(), Completion<Value>> {
    match vm.coro.live.get(key) {
        None => return Err(err(format!("invalid command name \"{}\"", key.name()))),
        Some(s) if s.status == CoroStatus::Running => {
            return Err(err(format!(
                "coroutine \"{}\" is already running",
                key.name()
            )));
        }
        Some(s)
            if s.status == CoroStatus::Suspended
                && s.last_suspend == SuspendKind::Yield
                && args.len() > 1 =>
        {
            let name = vm
                .native_name_operand_bytes(original_name)
                .map_err(|error| vm.refuse_host_command(error.to_string()))?;
            let mut message = b"wrong # args: should be \"".to_vec();
            message.extend_from_slice(&name);
            message.extend_from_slice(b" ?arg?\"");
            return Err(crate::command::native_wrong_arguments_message(vm, message));
        }
        Some(_) => {}
    }
    Ok(())
}

/// Resume the coroutine `fqn`, delivering `args` as the result of the parked
/// suspend point. A `yield`-suspended coroutine takes at most one value; a
/// `yieldto`-suspended one accepts any number, delivered as a list. `args` is
/// empty on the first run; `original_name` retains the resume operand for the
/// arity error. Returns the yielded value, or — on completion — the body's
/// result with its code.
fn resume(
    vm: &mut Vm,
    key: &CommandSidecarKey,
    args: &[Value],
    original_name: &Value,
) -> Completion<Value> {
    // Validate the resume arity against how the coroutine last suspended, before
    // the borrow choreography, so an error leaves the coroutine untouched.
    if let Err(error) = validate_resume_arity(vm, key, args, original_name) {
        return error;
    }
    // Snapshot whole-stack freshness before moving the state. Rejection below
    // happens only after installing the frozen flow, so ordinary unwind fires
    // unset/leave traces and frame/temp-proc cleanup; queued injections remain
    // deliberately unexecuted.
    let stale_message = vm.coro.live.get(key).and_then(|state| {
        vm.activation_stack_stale_message(
            &state.acts,
            &vm.namespace_path_for_token(state.parked.current_namespace_token()),
        )
    });
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
    if !was_fresh && stale_message.is_none() {
        // A `yield` returns the single resume value (or `""`); a `yieldto`
        // returns the whole resume-argument list.
        let initial = match kind {
            SuspendKind::Yield => args.first().cloned().unwrap_or_else(Value::empty),
            SuspendKind::YieldTo => {
                let Some(protocol) = vm
                    .actual_native_invocation_dialect()
                    .native_string_protocol()
                else {
                    vm.coro.stack.pop();
                    vm.swap_flow(&mut parked);
                    teardown_coro(vm, key);
                    return vm.refuse_host_command("original coroutine resume List issuer".into());
                };
                Value::native_list_constructor(args.to_vec(), protocol)
            }
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

    let exit = match stale_message {
        Some(message) => vm.unwind_stale_coroutine(&mut acts, message),
        None => vm.drive_coro(&mut acts),
    };

    if matches!(exit, RunExit::Done(_)) {
        vm.finish_coroutine_flow();
    }

    vm.coro.stack.pop();
    // Swap the resumer's flow back in; `parked` again holds the coroutine's flow.
    vm.swap_flow(&mut parked);
    let key = active_key.key();

    match exit {
        RunExit::Yielded(req) => {
            let kind = match &req {
                YieldReq::Yield(_) => SuspendKind::Yield,
                YieldReq::YieldTo { .. } => SuspendKind::YieldTo,
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
                // Relay lookup uses the captured coroutine namespace; the
                // target executes in the restored resumer variable frame.
                YieldReq::YieldTo { original } => invoke_original_relay(vm, &original),
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
/// swapped-in) coroutine context, newest callback first. Each command prefix is invoked as
/// `cmd args… <kind> <delivered>` — the suspend kind (`yield`/`yieldto`) and the
/// value the parked `yield` is about to return — and its result becomes the new
/// delivered value threaded into the next injection. Earlier callbacks still run
/// after a non-`ok` completion and replace it with their own completion.
fn run_injections(
    vm: &mut Vm,
    mut injections: Vec<Vec<Value>>,
    kind: SuspendKind,
    delivered: Value,
) -> Result<Value, Completion<Value>> {
    use tcl_registry::native_coroutine_compilation::{
        NativeCoroutineInjectionProtocol, NativeCoroutineResumeArity,
    };
    if injections.is_empty() {
        return Ok(delivered);
    }
    let Some(protocol) =
        NativeCoroutineInjectionProtocol::select(vm.actual_native_invocation_dialect())
    else {
        return Err(vm.refuse_host_command("original coroutine injection protocol".into()));
    };
    let arity = match kind {
        SuspendKind::Yield => NativeCoroutineResumeArity::SingleOptional,
        SuspendKind::YieldTo => NativeCoroutineResumeArity::Arbitrary,
    };
    let completion = protocol.run_pending(&mut injections, ok(delivered), |prefix, prior| {
        let mut call = prefix[1..].to_vec();
        call.push(Value::new_native_string_bytes(protocol.kind(arity)));
        // Tcl's callback argv retains the previous result while command entry
        // resets the interpreter result. A lifetime view alone permits reuse.
        call.push(prior.result.into_native_reference());
        vm.invoke_host_original_object_vector(&prefix[0], &call)
    });
    if completion.code.is_ok() {
        Ok(completion.result)
    } else {
        Err(completion)
    }
}

/// Drop the finished coroutine command. Actual body-owned lambda registrations
/// are retired by the shared activation teardown, independently of its spelling.
fn teardown_coro(vm: &mut Vm, key: &CommandSidecarKey) {
    if !vm.exit_pending() {
        vm.retire_command_lifecycle_key(key);
    }
    vm.coro.live.remove(key);
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
        return Err(crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"yieldto command ?arg ...?\"",
        ));
    }
    check_relay_boundary(vm)?;
    let namespace = tcl_cmd_core::namespace::current_original(vm)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    let protocol = vm
        .actual_native_invocation_dialect()
        .native_string_protocol()
        .ok_or_else(|| vm.refuse_host_command("original yieldto List issuer".into()))?;
    let mut members = Vec::with_capacity(words.len() + 1);
    members.push(namespace);
    members.extend_from_slice(words);
    vm.coro.pending = Some(YieldReq::YieldTo {
        original: Value::native_list_constructor(members, protocol),
    });
    Ok(())
}

fn check_relay_boundary(vm: &mut Vm) -> Result<(), Completion<Value>> {
    check_yieldable(vm, "yieldto")?;
    if vm.namespace_token_is_dying(vm.current_ns_id()) {
        return Err(crate::command::err_with_code(
            "yieldto called in deleted namespace",
            "TCL COROUTINE YIELDTO_IN_DELETED",
        ));
    }
    Ok(())
}

/// Keep the actual compiled namespace-prefixed List through suspension.
pub(crate) fn request_yieldto_original(
    vm: &mut Vm,
    original: Value,
) -> Result<(), Completion<Value>> {
    check_relay_boundary(vm)?;
    let Some(protocol) = vm
        .actual_native_invocation_dialect()
        .native_string_protocol()
    else {
        return Err(vm.refuse_host_command("original yieldto List issuer".into()));
    };
    let members = vm
        .native_object_list_elements_in(&original, protocol)
        .map_err(|error| crate::command::completion_from_tcl_error(vm, error.into()))?;
    if members.len() < 2 {
        return Err(crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"yieldto command ?arg ...?\"",
        ));
    }
    vm.coro.pending = Some(YieldReq::YieldTo { original });
    Ok(())
}

fn invoke_original_relay(vm: &mut Vm, original: &Value) -> Completion<Value> {
    let Some(protocol) = vm
        .actual_native_invocation_dialect()
        .native_string_protocol()
    else {
        return vm.refuse_host_command("original yieldto List issuer".into());
    };
    let members = match vm.native_object_list_elements_in(original, protocol) {
        Ok(members) => members,
        Err(error) => return crate::command::completion_from_tcl_error(vm, error.into()),
    };
    let Some((namespace, words)) = members.split_first() else {
        return vm.refuse_host_command("original yieldto namespace operand".into());
    };
    let Some((head, args)) = words.split_first() else {
        return ok(Value::empty());
    };
    let namespace = match vm.namespace_object_lookup(namespace) {
        Ok(Some(namespace)) => namespace,
        Ok(None) => {
            return match vm.native_name_operand_bytes(namespace) {
                Ok(bytes) => vm.namespace_lookup_error_bytes(&bytes),
                Err(error) => vm.refuse_host_command(error.to_string()),
            };
        }
        Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
    };
    vm.invoke_command_value_at(
        namespace,
        head,
        args,
        &[],
        tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
    )
}

/// `yield ?value?` — suspend the current coroutine.
fn cmd_yield(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.len() > 1 {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"yield ?value?\"",
        );
    }
    let value = args.first().cloned().unwrap_or_else(Value::empty);
    match request_yield(vm, value) {
        Ok(()) => ok(Value::empty()),
        Err(e) => e,
    }
}

/// `yieldto command ?arg ...?` — capture relay lookup at handler entry, then
/// execute its target in the resumer's variable frame.
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
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"coroprobe coroName cmd ?arg1 arg2 ...?\"",
        );
    };
    let fqn = match vm.resolve_original_command_key_at(vm.current_ns_id(), name) {
        Ok(Some(key)) => key,
        Ok(None) => return err("can only inject a probe command into a coroutine"),
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
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
    let result = vm.invoke_host_original_object_vector(cmd, rest);
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
/// becomes what that `yield` returns. One-shot, newest first (see [`resume`]).
fn cmd_coroinject(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [name, _cmd, _rest @ ..] = args else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"coroinject coroName cmd ?arg1 arg2 ...?\"",
        );
    };
    let fqn = match vm.resolve_original_command_key_at(vm.current_ns_id(), name) {
        Ok(Some(key)) => key,
        Ok(None) => return err("can only inject a command into a coroutine"),
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
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
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"::tcl::unsupported::corotype coroName\"",
        );
    };
    let fqn = match vm.resolve_original_command_key_at(vm.current_ns_id(), name) {
        Ok(Some(key)) => key,
        Ok(None) => return err("can only get coroutine type of a coroutine"),
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    match vm.coro.live.get(&CommandSidecarKey::visible(&fqn)) {
        None => err("can only get coroutine type of a coroutine"),
        Some(s) if s.status == CoroStatus::Running => ok(Value::string("active")),
        Some(s) => ok(Value::string(match s.last_suspend {
            SuspendKind::Yield => "yield",
            SuspendKind::YieldTo => "yieldto",
        })),
    }
}

/// Actual active coroutine identity, including its relocation/retirement cell.
pub(crate) fn current_activation_handle(vm: &Vm) -> Option<CommandSidecarHandle> {
    vm.coro.stack.last().map(|handle| handle.key.clone())
}

/// Enter only the coroutine selected by an authentic retained sidecar handle.
/// The exact physical frame must still be present after installing its flow.
pub(crate) fn eval_retained_activation(
    vm: &mut Vm,
    handle: &CommandSidecarHandle,
    frame: &Rc<crate::frame::ActivationIdentity>,
    source: &Value,
) -> tcl_runtime_api::retained_activation::ScheduledCallbackOutcome<Value> {
    use tcl_runtime_api::retained_activation::{
        RetainedActivationRefusal, ScheduledCallbackOutcome,
    };
    let Some(key) = handle.key() else {
        return ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::RetiredFrame);
    };
    let mut parked = {
        let Some(state) = vm.coro.live.get_mut(&key) else {
            return ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::RetiredFrame);
        };
        if state.status != CoroStatus::Suspended {
            return ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::BusyActivation);
        }
        state.status = CoroStatus::Running;
        std::mem::take(&mut state.parked)
    };
    vm.swap_flow(&mut parked);
    let base_depth = vm.activation_depth + 1;
    vm.coro.stack.push(CoroHandle {
        key: handle.clone(),
        base_depth,
    });
    let outcome = match vm.retained_frame_level(frame) {
        Some(level) => {
            crate::retained_activation::execution_outcome(vm.eval_retained_frame(level, source))
        }
        None => ScheduledCallbackOutcome::Activation(RetainedActivationRefusal::RetiredFrame),
    };
    vm.coro.stack.pop();
    vm.swap_flow(&mut parked);
    if let Some(key) = handle.key()
        && let Some(state) = vm.coro.live.get_mut(&key)
    {
        state.parked = parked;
        state.status = CoroStatus::Suspended;
    } else {
        // Deletion while entered detached the original sidecar. Retire that
        // original flow, including unset traces, rather than adopting a new name.
        vm.fire_parked_unset_traces(&mut parked, &mut Vec::new());
    }
    outcome
}

#[cfg(test)]
mod retirement_tests {
    use tcl_syntax::value::ValueOps;

    #[test]
    fn yieldto_retains_deleted_entered_namespace_when_its_spelling_is_recreated() {
        for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for relay in ["yieldto target", "$relay target"] {
                let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
                let mut vm = crate::native_fixture::interpreter(profile);
                let source = format!(
                    "namespace eval N {{proc generator {{relay}} {{\
                     namespace delete ::N; namespace eval ::N {{proc target {{}} {{return NEW}}}}; \
                     {relay}}}}}; \
                     set status [catch {{coroutine c N::generator yieldto}} message]; \
                     list $status $message $::errorCode"
                );
                let completion = vm.try_eval_source(&source).unwrap();
                assert_eq!(completion.code, crate::Code::Ok, "{engine}/{relay}");
                assert_eq!(
                    vm.native_string_bytes(&completion.result).unwrap().as_ref(),
                    b"1 {yieldto called in deleted namespace} {TCL COROUTINE YIELDTO_IN_DELETED}",
                    "{engine}/{relay}"
                );
                assert!(vm.refused_completion().is_none(), "{engine}/{relay}");
            }
        }
    }
}

#[cfg(test)]
mod original_operand_tests {
    use super::*;
    use tcl_syntax::value::ValueOps;

    struct ReturnFirst;
    impl NativeCommand for ReturnFirst {
        fn invoke(&self, _: &mut Vm, args: &[Value]) -> Completion<Value> {
            ok(args.first().cloned().unwrap_or_else(Value::empty))
        }
    }

    fn native() -> Vm {
        crate::native_fixture::interpreter(tcl_dialect::DialectProfile::find("tcl9.1").unwrap())
    }

    #[test]
    fn coroutine_probe_and_injection_keep_opaque_names_and_original_arguments() {
        let mut vm = native();
        let ready = vm
            .eval_source("coroutine c apply {{} {return [yield READY]}}")
            .unwrap();
        assert!(ready.code.is_ok());
        let name = Value::new_native_string_bytes(b"c\xff".as_slice());
        let renamed = vm.invoke_host_original_object_vector(
            &Value::new_native_string_bytes(b"rename".as_slice()),
            &[
                Value::new_native_string_bytes(b"c".as_slice()),
                name.clone(),
            ],
        );
        assert!(renamed.code.is_ok());
        let head = Value::new_native_string_bytes(b"probe\xff".as_slice());
        vm.register_command_in_slot(
            tcl_runtime_api::CommandSlot {
                namespace: vm.current_ns_id(),
                simple: b"probe\xff".as_slice().into(),
            },
            Command::Native(Rc::new(ReturnFirst)),
        );
        let argument = Value::new_native_string_bytes(b"A\0B\xff".as_slice());
        let kind = cmd_corotype(&mut vm, std::slice::from_ref(&name));
        assert!(kind.code.is_ok());
        assert_eq!(
            vm.native_string_bytes(&kind.result).unwrap().as_ref(),
            b"yield"
        );
        let probe = cmd_coroprobe(&mut vm, &[name.clone(), head.clone(), argument.clone()]);
        assert!(probe.code.is_ok());
        assert!(probe.result.is_same_object(&argument));
        let queued = cmd_coroinject(&mut vm, &[name.clone(), head, argument.clone()]);
        assert!(queued.code.is_ok());
        let resumed = vm.invoke_host_original_object_vector(&name, &[]);
        assert!(resumed.code.is_ok());
        assert!(resumed.result.is_same_object(&argument));
        assert!(vm.refused_completion().is_none());
    }

    #[test]
    fn coroutine_original_name_lookup_keeps_missing_and_unavailable_distinct() {
        let missing = Value::new_native_string_bytes(b"absent\xff".as_slice());
        let mut vm = native();
        let error = cmd_corotype(&mut vm, std::slice::from_ref(&missing));
        assert_eq!(error.code, crate::Code::Error);
        assert!(vm.refused_completion().is_none());
        let mut unavailable = Vm::new();
        unavailable.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        let _ = cmd_corotype(&mut unavailable, &[missing]);
        assert!(unavailable.refused_completion().is_some());
    }
}

#[cfg(test)]
mod native_injection_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod native_original_holder_tests;

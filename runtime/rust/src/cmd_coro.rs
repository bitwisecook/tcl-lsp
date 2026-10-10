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

//! Coroutines: `coroutine`, `yield`, `yieldto`, and `info coroutine`.
//!
//! ## Design — cooperative OS-thread coroutines
//!
//! `yield` must suspend execution mid-evaluation (arbitrarily deep in the
//! recursive tree-walking evaluator) and resume it later. Rather than rewrite
//! the evaluator into an explicit-stack engine, each coroutine runs the body on
//! its **own OS thread**; the native Rust call stack of a parked thread *is* the
//! suspended continuation. Control is strictly **cooperative ping-pong** — only
//! one thread is ever runnable at a time, handed off over rendezvous channels —
//! so although the interpreter state ([`Interp`] = `Rc<InterpState>`, `!Send`)
//! is shared by raw `Rc` clone across the threads, the accesses never overlap
//! and the `RefCell`s never alias. The one `unsafe` is asserting `Send` on the
//! handle that carries the `Rc` to the worker; it is sound precisely because of
//! the serialised handoff (see [`SendPtr`]).
//!
//! The interpreter's *per-flow execution context* (call frames, the `info frame`
//! stack, current namespace, the TclOO call/define stacks, …) is swapped in/out
//! on every handoff via [`Interp::swap_coro_ctx`], so each coroutine has its own
//! frames while sharing the global namespaces / commands / classes / channels.
//!
//! Native only: a single-threaded wasm reactor has no OS threads, so there the
//! commands report that coroutines need the (future) explicit-stack evaluator.
//!
//! C refs: `tclBasic.c` (`TclNRCoroutineObjCmd`, `TclNRYieldObjCmd`,
//! `CoroTypeObjCmd`).

use crate::interp::{Code, CoroContext, Interp};
use crate::obj::{self, TclObj};

/// Register `coroutine`, `yield`, `yieldto`, and the `info coroutine` hook.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"coroutine", coroutine_cmd);
    interp.register_builtin(b"yield", yield_cmd);
    interp.register_builtin(b"yieldto", yieldto_cmd);
    interp.register_builtin(b"coroprobe", coroprobe_cmd);
    interp.register_builtin(b"coroinject", coroinject_cmd);
    interp.register_builtin(b"::tcl::unsupported::corotype", corotype_cmd);
}

// the cross-thread plumbing

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::*;
    use std::cell::RefCell;
    use std::panic::AssertUnwindSafe;
    use std::sync::mpsc::{Receiver, Sender};
    use std::sync::{Arc, Mutex, Once};
    use std::thread::JoinHandle;

    /// The panic payload used to unwind a parked worker thread's native stack
    /// when its coroutine is deleted/renamed while suspended. Caught in
    /// `worker_main`; a panic hook keeps it silent.
    struct CoroTerminate;

    static HOOK: Once = Once::new();

    /// Install a panic hook that swallows the `CoroTerminate` unwind sentinel
    /// (chaining to the previous hook for real panics).
    fn ensure_quiet_terminate_hook() {
        HOOK.call_once(|| {
            let prev = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                if info.payload().downcast_ref::<CoroTerminate>().is_some() {
                    return;
                }
                prev(info);
            }));
        });
    }

    /// One original owning reference transferred only while the other interpreter
    /// thread is blocked at its rendezvous. No object access occurs concurrently.
    pub(super) struct OriginalHandoff(pub(super) obj::Owned);
    // SAFETY: only the serialised coroutine channels transport this owner; the
    // sender cannot access it after send and interpreter access remains parked.
    unsafe impl Send for OriginalHandoff {}

    pub(super) enum ResumeValue {
        Wire(Vec<u8>),
        Original(OriginalHandoff),
    }

    impl ResumeValue {
        fn publish(self, interp: &mut Interp) {
            match self {
                Self::Wire(bytes) => interp.set_result_bytes(&bytes),
                Self::Original(original) => interp.set_result(original.0.as_ptr()),
            }
        }
    }

    /// Main → worker: resume with a value, tear the coroutine down, run a probe
    /// command in the suspended context (`coroprobe`), or queue a command to run
    /// on the next resume (`coroinject`).
    pub(super) enum ToCoro {
        Resume(ResumeValue),
        Terminate,
        Probe(OriginalHandoff),
        Inject(OriginalHandoff),
    }

    /// Worker → main: a `yield` (value), the body finished (code + result), a
    /// probe's result (code + result, with its original error objects retained
    /// in the serialised interpreter owner), or an acknowledgement that an inject was queued.
    pub(super) enum FromCoro {
        Yield(ResumeValue),
        YieldTo(OriginalHandoff),
        Done(Code, OriginalHandoff),
        ProbeDone(Code, OriginalHandoff),
        InjectAck,
    }

    /// A live coroutine: its saved execution context (swapped on handoff) plus
    /// the main-side channel ends and worker join handle.
    pub struct CoroEntry {
        pub(crate) context: CoroContext,
        yielded_to: bool,
        /// Shared with the worker thread, so [`rename`] moves the name both
        /// sides read (see [`CoroName`]).
        name: CoroName,
        to_coro: Sender<ToCoro>,
        from_coro: Option<Receiver<FromCoro>>,
        join: Option<JoinHandle<()>>,
    }

    /// A raw pointer carried to the worker thread. `Send` is asserted: the
    /// coroutine protocol guarantees the main and worker threads never run
    /// concurrently (strict channel ping-pong), so the `!Send` `Rc` interior is
    /// only ever touched by one thread at a time.
    struct SendPtr(Interp);
    // SAFETY: serialised cooperative handoff — see the module docs.
    unsafe impl Send for SendPtr {}

    /// A coroutine's current command name, shared between the main flow and its
    /// worker thread so a `rename` is visible on both sides: C ties the
    /// coroutine to the command *token*, and `[info coroutine]` reports
    /// whatever that token is called now.
    pub(crate) struct CoroIdentity {
        generation: u64,
        report: Mutex<Vec<u8>>,
    }
    pub(crate) type CoroName = Arc<CoroIdentity>;

    /// Read the shared name. Poisoning is ignored — the value is a plain name
    /// that is never observed half-written, because the lock is only ever held
    /// across a clone or an assignment, never across anything that can unwind
    /// (the `CoroTerminate` sentinel included).
    pub(super) fn read_name(name: &CoroName) -> Vec<u8> {
        name.report
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Point the shared name at `value` (a `rename` of the coroutine command).
    fn write_name(name: &CoroName, value: &[u8]) {
        value.clone_into(&mut name.report.lock().unwrap_or_else(|e| e.into_inner()));
    }

    /// The worker thread's own handles + identity (its end of the channels).
    struct CoroTls {
        name: CoroName,
        from_coro: Sender<FromCoro>,
        to_coro: Receiver<ToCoro>,
    }

    thread_local! {
        static TLS: RefCell<Option<CoroTls>> = const { RefCell::new(None) };
    }

    /// `info coroutine` / `[info coroutine]` — the running coroutine's command
    /// name, or empty on the main flow (or any non-coroutine thread).
    pub(super) fn current_name() -> Vec<u8> {
        TLS.with(|t| {
            t.borrow()
                .as_ref()
                .map(|c| read_name(&c.name))
                .unwrap_or_default()
        })
    }

    pub(super) fn current_generation() -> Option<u64> {
        TLS.with(|t| t.borrow().as_ref().map(|c| c.name.generation))
    }

    pub(super) fn in_coroutine() -> bool {
        TLS.with(|t| t.borrow().is_some())
    }

    /// `coroutine name cmd ?arg ...?` — create a coroutine running `cmd args`,
    /// run it until its first `yield` (or completion), and return that value.
    pub(super) fn create(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
        if argv.len() < 3 {
            return interp.wrong_arguments_message(
                b"wrong # args: should be \"coroutine name cmd ?arg ...?\"",
            );
        }
        let slot = match interp.native_coroutine_publication_slot(argv[1]) {
            Ok(slot) => slot,
            Err(error) => return interp.report_cmd_error(error),
        };
        let name = interp.namespaces().command_fqn_at(slot.0, &slot.1);
        let Some(protocol) = interp.native_invocation_dialect().native_string_protocol() else {
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original coroutine invocation List",
                )
                .into(),
            );
        };
        let original_command = OriginalHandoff(obj::Owned::fresh(
            crate::list::new_list_obj_native(&argv[2..], protocol),
        ));

        // Retire the old binding before attaching the fresh coroutine state.
        let generation = interp.register_coroutine_command_in_slot(slot);
        ensure_quiet_terminate_hook();
        let (to_tx, to_rx) = std::sync::mpsc::channel::<ToCoro>();
        let (from_tx, from_rx) = std::sync::mpsc::channel::<FromCoro>();

        // Fresh execution context in the creating namespace.
        let context = CoroContext::fresh(interp.current_ns());

        // Hand a clone of the interp to the worker (sound under serialisation).
        let send_interp = SendPtr(interp.clone_handle());
        let shared_name: CoroName = Arc::new(CoroIdentity {
            generation,
            report: Mutex::new(name),
        });
        let worker_name = Arc::clone(&shared_name);
        let join = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(move || worker_main(send_interp, original_command, worker_name, from_tx, to_rx))
            .expect("spawn coroutine thread");

        interp.coros_mut().insert(
            generation,
            CoroEntry {
                context,
                yielded_to: false,
                name: shared_name,
                to_coro: to_tx,
                from_coro: Some(from_rx),
                join: Some(join),
            },
        );
        // Run to the first suspension point.
        resume_value(interp, generation, ResumeValue::Wire(Vec::new()))
    }

    /// The worker thread entry: wait for the first resume, run the body in the
    /// (already swapped-in) coroutine context, then hand the result back.
    fn worker_main(
        si: SendPtr,
        original_command: OriginalHandoff,
        name: CoroName,
        from_tx: Sender<FromCoro>,
        to_rx: Receiver<ToCoro>,
    ) {
        // Install this thread's identity + channel ends.
        TLS.with(|t| {
            *t.borrow_mut() = Some(CoroTls {
                name: Arc::clone(&name),
                from_coro: from_tx.clone(),
                to_coro: to_rx,
            });
        });
        let interp = si.0;
        // Run the body inside `catch_unwind`: a `CoroTerminate` panic (raised by
        // `recv_resume`/`do_yield` when the coroutine is deleted while
        // suspended) unwinds this thread's native stack — which only ever
        // happens while the main thread is blocked awaiting us, so no concurrent
        // interpreter access occurs.
        let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
            // Block for the first resume (the creator's swap installed our ctx).
            recv_resume();
            let mut ip = interp.clone_handle();
            let code = match crate::list::list_elements(original_command.0.as_ptr()) {
                Ok(argv) => ip.dispatch(&argv),
                Err(error) => ip.report_cmd_error(error.into()),
            };
            let result = OriginalHandoff(obj::Owned::retain(ip.result_obj()));
            (code, result)
        }));
        // Restore the caller's context, then acknowledge (Done on normal
        // completion, or in response to a terminate). The main thread is blocked
        // in `resume`/`terminate` recv, so this is race-free.
        let ip = interp;
        // The same actual command generation owns the context across rename.
        ip.coro_swap_generation(name.generation);
        let msg = match outcome {
            Ok((code, result)) => FromCoro::Done(code, result),
            Err(_) => FromCoro::Done(
                Code::Error,
                OriginalHandoff(obj::Owned::fresh(obj::new_string_bytes(b""))),
            ),
        };
        let _ = from_tx.send(msg);
        // TLS drops with the thread.
    }

    /// On the worker: block until the next resume. A terminate (or a closed
    /// channel) unwinds the thread via the `CoroTerminate` panic sentinel.
    fn recv_resume() -> ResumeValue {
        let msg = TLS.with(|t| {
            let b = t.borrow();
            let tls = b.as_ref().expect("coroutine TLS");
            tls.to_coro.recv()
        });
        match msg {
            Ok(ToCoro::Resume(v)) => v,
            _ => std::panic::panic_any(CoroTerminate),
        }
    }

    /// Dispatch the retained original prefix inside the parked worker context.
    /// Pin the result before releasing the prefix's owning reference.
    fn eval_words(interp: &mut Interp, words: &obj::Owned) -> (Code, OriginalHandoff) {
        let code = match crate::list::list_elements(words.as_ptr()) {
            Ok(argv) => interp.dispatch(&argv),
            Err(error) => interp.report_cmd_error(error.into()),
        };
        (
            code,
            OriginalHandoff(obj::Owned::retain(interp.result_obj())),
        )
    }

    /// `yield ?value?` — suspend the current coroutine, returning `value` to the
    /// resumer; the result is whatever value resumes it. While suspended the
    /// worker also services `coroprobe` (run a command here, stay suspended) and
    /// `coroinject` (queue a command for the next resume) requests.
    pub(super) fn do_yield_original(
        interp: &mut Interp,
        value: obj::Owned,
        invocation: bool,
    ) -> Code {
        if invocation {
            if !in_coroutine() {
                return interp.error_with_code(
                    b"yieldto can only be called in a coroutine",
                    b"TCL COROUTINE ILLEGAL_YIELD",
                );
            }
            if !interp.namespaces().namespace_is_live(interp.current_ns()) {
                return interp.error_with_code(
                    b"yieldto called in deleted namespace",
                    b"TCL COROUTINE YIELDTO_IN_DELETED",
                );
            }
            let Some(protocol) = interp.native_invocation_dialect().native_string_protocol() else {
                return interp.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original yieldto List issuer",
                    )
                    .into(),
                );
            };
            let members = match crate::list::list_elements_native_checked(value.as_ptr(), protocol)
            {
                Ok(members) => members,
                Err(error) => return interp.report_cmd_error(error.into()),
            };
            if members.len() < 2 {
                return interp.wrong_arguments_message(
                    b"wrong # args: should be \"yieldto command ?arg ...?\"",
                );
            }
        }
        let outgoing = if invocation {
            FromCoro::YieldTo(OriginalHandoff(value))
        } else {
            FromCoro::Yield(ResumeValue::Original(OriginalHandoff(value)))
        };
        yield_message(interp, outgoing)
    }

    fn yield_message(interp: &mut Interp, mut outgoing: FromCoro) -> Code {
        let Some(name) = current_generation() else {
            return interp.error_with_code(
                b"yield can only be called in a coroutine",
                b"TCL COROUTINE ILLEGAL_YIELD",
            );
        };
        // Restore the caller's context, hand back the yield value, then loop
        // servicing requests until an actual resume (or teardown) arrives.
        interp.coro_swap_generation(name);
        use tcl_registry::native_coroutine_compilation::{
            NativeCoroutineInjectionProtocol, NativeCoroutineResumeArity,
        };
        let arity = if matches!(&outgoing, FromCoro::YieldTo(_)) {
            NativeCoroutineResumeArity::Arbitrary
        } else {
            NativeCoroutineResumeArity::SingleOptional
        };
        let mut pending_inject: Vec<OriginalHandoff> = Vec::new();
        loop {
            let to_send = outgoing;
            let msg = TLS.with(|t| {
                let b = t.borrow();
                let tls = b.as_ref().expect("coroutine TLS");
                if tls.from_coro.send(to_send).is_err() {
                    return None;
                }
                tls.to_coro.recv().ok()
            });
            match msg {
                Some(ToCoro::Resume(v)) => {
                    // A queued `coroinject` runs first, in this (now swapped-in)
                    // context, with the yield command + resume value appended; its
                    // result becomes what `yield` returns.
                    if !pending_inject.is_empty() {
                        let Some(protocol) = NativeCoroutineInjectionProtocol::select(
                            interp.native_invocation_dialect(),
                        ) else {
                            return interp.report_cmd_error(
                                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                    "original coroutine injection protocol",
                                )
                                .into(),
                            );
                        };
                        v.publish(interp);
                        return protocol.run_pending(
                            &mut pending_inject,
                            Code::Ok,
                            |words, _prior_code| {
                                let mut argv = match crate::list::list_elements(words.0.as_ptr()) {
                                    Ok(argv) => argv,
                                    Err(error) => return interp.report_cmd_error(error.into()),
                                };
                                let kind =
                                    obj::Owned::fresh(obj::new_string_bytes(protocol.kind(arity)));
                                let delivered = obj::Owned::retain(interp.result_obj());
                                argv.push(kind.as_ptr());
                                argv.push(delivered.as_ptr());
                                interp.dispatch(&argv)
                            },
                        );
                    }
                    v.publish(interp);
                    return Code::Ok;
                }
                // `coroprobe`: the caller has swapped our context in, so run the
                // command here, capture its error trace for transplanting, and
                // stay suspended.
                Some(ToCoro::Probe(words)) => {
                    let (code, result) = eval_words(interp, &words.0);
                    let snap = interp.snapshot_error();
                    interp.publish_coro_probe_error(snap);
                    outgoing = FromCoro::ProbeDone(code, result);
                }
                // `coroinject`: remember the command; it runs on the next resume.
                Some(ToCoro::Inject(words)) => {
                    pending_inject.push(words);
                    outgoing = FromCoro::InjectAck;
                }
                // Deleted while suspended (Terminate / closed channel): unwind
                // this worker without touching the interpreter further (the main
                // thread is blocked awaiting us).
                _ => std::panic::panic_any(CoroTerminate),
            }
        }
    }

    /// Resume coroutine `name` with `value`; returns its next yield value, or
    /// its final result (and tears it down) if the body completed.
    pub(super) fn resume_original(interp: &mut Interp, name: u64, value: obj::Owned) -> Code {
        resume_value(interp, name, ResumeValue::Original(OriginalHandoff(value)))
    }

    fn resume_value(interp: &mut Interp, name: u64, value: ResumeValue) -> Code {
        // Swap the coroutine's context into the interpreter, then hand control
        // to its worker and block until it yields or completes.
        let exists = interp.coros_mut().contains_key(&name);
        if !exists {
            return interp.set_error(b"coroutine is dead");
        }
        let chans = {
            let mut reg = interp.coros_mut();
            let entry = reg.get_mut(&name).expect("checked above");
            // (the actual context swap touches other RefCells; do it after)
            entry
                .from_coro
                .take()
                .map(|rx| (entry.to_coro.clone(), rx, Arc::clone(&entry.name)))
        };
        let Some((to_coro, from_coro, shared_name)) = chans else {
            // Already running (re-entrant resume) — C: "coroutine ... is already
            // running".
            let mut m = b"coroutine \"".to_vec();
            let report = interp
                .coros_mut()
                .get(&name)
                .map(|entry| read_name(&entry.name))
                .unwrap_or_default();
            m.extend_from_slice(&report);
            m.extend_from_slice(b"\" is already running");
            return interp.set_error(&m);
        };
        interp.coro_swap_generation(name);
        if to_coro.send(ToCoro::Resume(value)).is_err() {
            return interp.set_error(b"coroutine is dead");
        }
        let msg = from_coro.recv();
        // The retained actual command generation owns the sidecar across rename.
        let name = shared_name.generation;
        // Put the receiver back for the next resume.
        if let Some(entry) = interp.coros_mut().get_mut(&name) {
            entry.from_coro = Some(from_coro);
        }
        match msg {
            Ok(FromCoro::Yield(v)) => {
                if let Some(entry) = interp.coros_mut().get_mut(&name) {
                    entry.yielded_to = false;
                }
                v.publish(interp);
                Code::Ok
            }
            Ok(FromCoro::YieldTo(original)) => {
                if let Some(entry) = interp.coros_mut().get_mut(&name) {
                    entry.yielded_to = true;
                }
                interp.invoke_original_namespace_list(original.0.as_ptr())
            }
            Ok(FromCoro::Done(code, result)) => {
                finish(interp, name);
                interp.set_result(result.0.as_ptr());
                code
            }
            // A resume only ever draws `Yield`/`Done`; a probe/inject reply or a
            // closed channel here means the worker is gone.
            _ => {
                finish(interp, name);
                interp.set_error(b"coroutine is dead")
            }
        }
    }

    pub(super) fn yielded_to(interp: &Interp, name: u64) -> bool {
        interp
            .coros_mut()
            .get(&name)
            .is_some_and(|entry| entry.yielded_to)
    }

    /// Rename updates only the public name of the same actual coroutine generation.
    /// The worker, sidecar and context retain that immutable registration identity.
    pub(super) fn rename(interp: &mut Interp, generation: u64, new_fqn: &[u8]) {
        if let Some(entry) = interp.coros_mut().get(&generation) {
            write_name(&entry.name, new_fqn);
        }
    }

    /// Tear down a completed/dead coroutine: remove the registry entry (before
    /// deleting the command, so the command-delete hook sees no coroutine), join
    /// its thread, and delete the command.
    fn finish(interp: &mut Interp, name: u64) {
        let entry = interp.coros_mut().remove(&name);
        if let Some(mut e) = entry {
            if let Some(j) = e.join.take() {
                let _ = j.join();
            }
        }
        interp.delete_command_generation(name);
    }

    /// `coroprobe coroName cmd ?arg ...?` — run `cmd args` in the *suspended*
    /// coroutine `name`'s context (its frames/variables) and return the result;
    /// the coroutine stays suspended. Its error trace (if any) is transplanted
    /// into the caller once the context is swapped back out.
    pub(super) fn probe(interp: &mut Interp, name: u64, words: OriginalHandoff) -> Code {
        if !interp.coros_mut().contains_key(&name) {
            return interp.set_error(b"can only inject a probe command into a coroutine");
        }
        let chans = {
            let mut reg = interp.coros_mut();
            let entry = reg.get_mut(&name).expect("checked above");
            entry
                .from_coro
                .take()
                .map(|rx| (entry.to_coro.clone(), rx, Arc::clone(&entry.name)))
        };
        let Some((to_coro, from_coro, shared_name)) = chans else {
            return interp.set_error(b"can only inject a probe command into a suspended coroutine");
        };
        // Swap the coroutine's context in so the probe runs in its frames, hand
        // off, and block for the result — then swap back to the caller's context.
        // The swap is a symmetric toggle, so it is self-correcting regardless of
        // whether the caller is the main flow or another coroutine.
        interp.coro_swap_generation(name);
        let sent = to_coro.send(ToCoro::Probe(words)).is_ok();
        let msg = if sent { from_coro.recv().ok() } else { None };
        // Probe callbacks preserve the same actual sidecar generation across rename.
        let name = shared_name.generation;
        interp.coro_swap_generation(name);
        if let Some(entry) = interp.coros_mut().get_mut(&name) {
            entry.from_coro = Some(from_coro);
        }
        let snapshot = interp.take_coro_probe_error();
        match (msg, snapshot) {
            (Some(FromCoro::ProbeDone(code, result)), Some(snap)) => {
                interp.set_result(result.0.as_ptr());
                if code == Code::Error {
                    interp.restore_error(snap);
                    interp.append_frame_noline(b"injected coroutine probe command");
                }
                code
            }
            _ => {
                finish(interp, name);
                interp.set_error(b"coroutine is dead")
            }
        }
    }

    /// `coroinject coroName cmd ?arg ...?` — queue `cmd args` to run inside the
    /// *suspended* coroutine `name` the next time it is resumed, before it
    /// continues; the yield command and resume value are appended, and the
    /// injected command's result becomes what `yield` returns. Returns empty.
    pub(super) fn inject(interp: &mut Interp, name: u64, words: OriginalHandoff) -> Code {
        if !interp.coros_mut().contains_key(&name) {
            return interp.set_error(b"can only inject a command into a coroutine");
        }
        let chans = {
            let mut reg = interp.coros_mut();
            let entry = reg.get_mut(&name).expect("checked above");
            entry
                .from_coro
                .take()
                .map(|rx| (entry.to_coro.clone(), rx, Arc::clone(&entry.name)))
        };
        let Some((to_coro, from_coro, shared_name)) = chans else {
            return interp.set_error(b"can only inject a command into a suspended coroutine");
        };
        // No context swap: the worker only records the command (it runs later, on
        // resume). Send + await the acknowledgement to keep the channel in step.
        let sent = to_coro.send(ToCoro::Inject(words)).is_ok();
        let msg = if sent { from_coro.recv().ok() } else { None };
        let name = shared_name.generation;
        if let Some(entry) = interp.coros_mut().get_mut(&name) {
            entry.from_coro = Some(from_coro);
        }
        match msg {
            Some(FromCoro::InjectAck) => {
                interp.set_result_bytes(b"");
                Code::Ok
            }
            _ => {
                finish(interp, name);
                interp.set_error(b"coroutine is dead")
            }
        }
    }

    /// Terminate a *suspended* coroutine `name` (e.g. `rename $coro {}` deletes
    /// its command). Swaps the coroutine's context in, sends `Terminate` so its
    /// worker unwinds, waits for the acknowledgement (so the unwind happens with
    /// no concurrent interpreter access), then joins. Removes the registry entry
    /// but does **not** delete the command (the caller is doing that).
    pub(super) fn terminate(interp: &mut Interp, name: u64) {
        // Don't try to terminate from within a coroutine worker (avoid a thread
        // joining itself); only the main flow tears coroutines down.
        if let Some(entry) = interp.coros_mut().get(&name) {
            write_name(&entry.name, b"");
        }
        if in_coroutine() {
            return;
        }
        let chans = {
            let mut reg = interp.coros_mut();
            match reg.get_mut(&name) {
                Some(e) => e
                    .from_coro
                    .take()
                    .map(|rx| (e.to_coro.clone(), rx, Arc::clone(&e.name))),
                None => return,
            }
        };
        let mut name = name;
        if let Some((to_coro, from_coro, shared_name)) = chans {
            // Install the coroutine's context so its worker unwinds in its own
            // frames, then signal + await the acknowledgement.
            interp.coro_swap_generation(name);
            if to_coro.send(ToCoro::Terminate).is_ok() {
                let _ = from_coro.recv();
            }
            name = shared_name.generation;
        }
        let entry = interp.coros_mut().remove(&name);
        if let Some(mut e) = entry {
            if let Some(j) = e.join.take() {
                let _ = j.join();
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use imp::CoroEntry;

#[cfg(not(target_arch = "wasm32"))]
fn coroutine_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    imp::create(interp, argv)
}

#[cfg(not(target_arch = "wasm32"))]
fn yield_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 2 {
        return interp.wrong_arguments_message(b"wrong # args: should be \"yield ?value?\"");
    }
    let value = argv.get(1).map_or_else(
        || obj::Owned::fresh(obj::new_string_bytes(b"")),
        |&original| obj::Owned::retain(original),
    );
    yield_original(interp, value)
}

#[cfg(not(target_arch = "wasm32"))]
fn yieldto_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args_for_prefix(argv, 1, b"command ?arg ...?");
    }
    if !imp::in_coroutine() {
        return interp.error_with_code(
            b"yieldto can only be called in a coroutine",
            b"TCL COROUTINE ILLEGAL_YIELD",
        );
    }
    if !interp.namespaces().namespace_is_live(interp.current_ns()) {
        return interp.error_with_code(
            b"yieldto called in deleted namespace",
            b"TCL COROUTINE YIELDTO_IN_DELETED",
        );
    }
    let Some(protocol) = interp.native_invocation_dialect().native_string_protocol() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original yieldto invocation List",
            )
            .into(),
        );
    };
    let namespace = match tcl_cmd_core::namespace::current_original(interp) {
        Ok(original) => obj::Owned::fresh(original),
        Err(error) => return interp.report_cmd_error(error),
    };
    let mut pointers = vec![namespace.as_ptr()];
    pointers.extend_from_slice(&argv[1..]);
    let original = obj::Owned::fresh(crate::list::new_list_obj_native(&pointers, protocol));
    yieldto_original(interp, original)
}

pub(crate) fn yield_original(interp: &mut Interp, value: obj::Owned) -> Code {
    #[cfg(not(target_arch = "wasm32"))]
    {
        imp::do_yield_original(interp, value, false)
    }
    #[cfg(target_arch = "wasm32")]
    {
        drop(value);
        interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native coroutine suspension",
            )
            .into(),
        )
    }
}

pub(crate) fn yieldto_original(interp: &mut Interp, value: obj::Owned) -> Code {
    #[cfg(not(target_arch = "wasm32"))]
    {
        imp::do_yield_original(interp, value, true)
    }
    #[cfg(target_arch = "wasm32")]
    {
        drop(value);
        interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native coroutine suspension",
            )
            .into(),
        )
    }
}

/// Retain the actual original prefix objects for the serialised worker handoff.
#[cfg(not(target_arch = "wasm32"))]
fn original_prefix(
    interp: &mut Interp,
    argv: &[*mut TclObj],
) -> Result<imp::OriginalHandoff, tcl_syntax::value::ValueError> {
    let protocol = interp
        .native_invocation_dialect()
        .native_string_protocol()
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original coroutine prefix List issuer",
        ))?;
    Ok(imp::OriginalHandoff(obj::Owned::fresh(
        crate::list::new_list_obj_native(argv, protocol),
    )))
}

#[cfg(not(target_arch = "wasm32"))]
fn coroprobe_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_arguments_message(
            b"wrong # args: should be \"coroprobe coroName cmd ?arg1 arg2 ...?\"",
        );
    }
    let name = match interp.resolve_original_command_generation(argv[1]) {
        Ok(Some(key)) => key,
        Ok(None) => return interp.set_error(b"can only inject a probe command into a coroutine"),
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let words = match original_prefix(interp, &argv[2..]) {
        Ok(words) => words,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    imp::probe(interp, name, words)
}

#[cfg(not(target_arch = "wasm32"))]
fn coroinject_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_arguments_message(
            b"wrong # args: should be \"coroinject coroName cmd ?arg1 arg2 ...?\"",
        );
    }
    let name = match interp.resolve_original_command_generation(argv[1]) {
        Ok(Some(key)) => key,
        Ok(None) => return interp.set_error(b"can only inject a command into a coroutine"),
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let words = match original_prefix(interp, &argv[2..]) {
        Ok(words) => words,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    imp::inject(interp, name, words)
}

/// `::tcl::unsupported::corotype coroName` — the coroutine's current type: the
/// currently-running coroutine (e.g. `corotype [info coroutine]`) is `active`;
/// other live coroutines report the actual suspended yield kind.
#[cfg(not(target_arch = "wasm32"))]
fn corotype_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_arguments_message(
            b"wrong # args: should be \"::tcl::unsupported::corotype coroName\"",
        );
    }
    let name = match interp.resolve_original_command_generation(argv[1]) {
        Ok(Some(key)) => key,
        Ok(None) => return interp.set_error(b"can only get coroutine type of a coroutine"),
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    if imp::current_generation() == Some(name) {
        interp.set_result_bytes(b"active");
        return Code::Ok;
    }
    if interp.coros_mut().contains_key(&name) {
        interp.set_result_bytes(if imp::yielded_to(interp, name) {
            b"yieldto"
        } else {
            b"yield"
        });
        return Code::Ok;
    }
    interp.set_error(b"can only get coroutine type of a coroutine")
}

/// Resume the coroutine named by `argv[0]` (its command invocation).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn coro_resume_command(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let Some(name) = interp.active_builtin_command_generation() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original coroutine selected registration",
            )
            .into(),
        );
    };
    let value = if imp::yielded_to(interp, name) {
        let Some(protocol) = interp.native_invocation_dialect().native_string_protocol() else {
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original coroutine resume List",
                )
                .into(),
            );
        };
        obj::Owned::fresh(crate::list::new_list_obj_native(&argv[1..], protocol))
    } else {
        if argv.len() > 2 {
            return interp.wrong_args_for_prefix(argv, 1, b"?value?");
        }
        argv.get(1).map_or_else(
            || obj::Owned::fresh(obj::new_string_bytes(b"")),
            |&original| obj::Owned::retain(original),
        )
    };
    imp::resume_original(interp, name, value)
}

/// Retire the suspended worker attached to an actual deleted command generation.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn on_command_deleted(interp: &mut Interp, generation: u64) {
    imp::terminate(interp, generation);
}

/// Hook for `rename $coro $new`: move a live coroutine's state to the new name.
/// C keeps the coroutine on the command *token*, so a rename is transparent to
/// it — including to `[info coroutine]`, which reports the new name from inside
/// the coroutine itself (tclsh 8.6/9.0-pinned).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn on_command_renamed(interp: &mut Interp, generation: u64, new_fqn: &[u8]) {
    imp::rename(interp, generation, new_fqn);
}

/// `info coroutine` — the current coroutine's command name (empty on the main
/// flow). Used by `cmd_info`.
#[cfg(not(target_arch = "wasm32"))]
pub fn current_coroutine() -> Vec<u8> {
    imp::current_name()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn in_coroutine() -> bool {
    imp::in_coroutine()
}

// wasm: no OS threads → coroutines need the explicit-stack evaluator

#[cfg(target_arch = "wasm32")]
fn coroutine_cmd(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    interp.set_error(b"coroutines are not supported in the single-threaded wasm build")
}

#[cfg(target_arch = "wasm32")]
fn yield_cmd(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    interp.set_error(b"yield can only be called in a coroutine")
}

#[cfg(target_arch = "wasm32")]
fn yieldto_cmd(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    interp.set_error(b"yieldto is not supported in the single-threaded wasm build")
}

#[cfg(target_arch = "wasm32")]
fn coroprobe_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_arguments_message(
            b"wrong # args: should be \"coroprobe coroName cmd ?arg1 arg2 ...?\"",
        );
    }
    // No coroutines exist on the single-threaded wasm build, so no name is one.
    interp.set_error(b"can only inject a probe command into a coroutine")
}

#[cfg(target_arch = "wasm32")]
fn coroinject_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_arguments_message(
            b"wrong # args: should be \"coroinject coroName cmd ?arg1 arg2 ...?\"",
        );
    }
    interp.set_error(b"can only inject a command into a coroutine")
}

#[cfg(target_arch = "wasm32")]
fn corotype_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_arguments_message(
            b"wrong # args: should be \"::tcl::unsupported::corotype coroName\"",
        );
    }
    // No coroutines exist on the single-threaded wasm build, so no name is one.
    interp.set_error(b"can only get coroutine type of a coroutine")
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn coro_resume_command(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    interp.set_error(b"coroutines are not supported in the single-threaded wasm build")
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn on_command_deleted(_interp: &mut Interp, _generation: u64) {}

#[cfg(target_arch = "wasm32")]
pub(crate) fn on_command_renamed(_interp: &mut Interp, _generation: u64, _new_fqn: &[u8]) {}

#[cfg(target_arch = "wasm32")]
pub fn current_coroutine() -> Vec<u8> {
    Vec::new()
}

#[cfg(target_arch = "wasm32")]
pub fn in_coroutine() -> bool {
    false
}

/// The wasm32 stand-in for a registered coroutine: the type must exist so the
/// shared `Interp`'s `coros` table + `coro_swap_generation` compile, but it is never
/// populated (coroutine creation errors in the single-threaded wasm build).
#[cfg(target_arch = "wasm32")]
pub struct CoroEntry {
    pub(crate) context: crate::interp::CoroContext,
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use crate::interp::Interp;

    /// Functional coroutine tests. They are *not* leak-checked: a coroutine runs
    /// on a worker thread, and the leak counters are thread-local, so cross-
    /// thread alloc/free is expected to skew them (the production build does not
    /// leak-check, and the tcltest sweep covers behaviour end-to-end).
    fn run(i: &mut Interp, s: &[u8]) -> Vec<u8> {
        let code = i.eval_str(s);
        assert!(
            !i.host_refusal_pending(),
            "admission {:?}, native access {:?}",
            i.native_compilation_admission_error(),
            i.native_access_refusal()
        );
        assert_eq!(code, crate::interp::Code::Ok, "eval failed: {:?}", s);
        i.result_bytes()
    }

    // Needs the numeric tower: the generator body loops via `for`.
    #[cfg(have_tommath)]
    #[test]
    fn coroutine_yield_resume_and_info() {
        let mut i = Interp::new();
        run(
            &mut i,
            b"proc gen {n} { for {set k 0} {$k < $n} {incr k} { yield $k }; return done }",
        );
        // Creation runs to the first yield; each call resumes.
        assert_eq!(run(&mut i, b"coroutine c gen 3"), b"0");
        assert_eq!(run(&mut i, b"c"), b"1");
        assert_eq!(run(&mut i, b"c"), b"2");
        // The loop ends and the body returns; the command then disappears.
        assert_eq!(run(&mut i, b"c"), b"done");
        assert_eq!(run(&mut i, b"llength [info commands c]"), b"0");
        // `info coroutine` is empty on the main flow but the running name inside.
        assert_eq!(run(&mut i, b"info coroutine"), b"");
        run(&mut i, b"proc who {} { yield [info coroutine] }");
        assert_eq!(run(&mut i, b"coroutine w who"), b"::w");
    }

    /// A coroutine frame is an activation like any other (C's
    /// `Tcl_PushCallFrame` counts every frame), so a namespace deleted while a
    /// suspended coroutine holds it is retained, and torn down when the
    /// coroutine's frames unwind. Measured on tclsh 9.0.4 and 8.6.16.
    #[test]
    fn a_suspended_coroutine_holds_the_namespace_it_parked_in() {
        let mut i = Interp::new();
        run(&mut i, b"set log {}");
        run(
            &mut i,
            b"proc rec {old new op} {lappend ::log [list $old $op]}",
        );
        run(&mut i, b"namespace eval N {proc q {} {return Q}}");
        run(&mut i, b"trace add command ::N::q delete rec");
        run(
            &mut i,
            b"coroutine ::co apply {{} {namespace eval ::N \
               {yield ready; list [q] [namespace exists ::N] [namespace current]}}}",
        );
        run(&mut i, b"namespace delete ::N");
        // Unpublished at once, but nothing has fired yet.
        assert_eq!(
            run(&mut i, b"list [namespace exists ::N] [llength $::log]"),
            b"0 0"
        );
        // Resuming still resolves `q` through the parked frame's token.
        assert_eq!(run(&mut i, b"co"), b"Q 0 ::N");
        assert_eq!(run(&mut i, b"set log"), b"{::N::q delete}");
    }

    /// Deleting the coroutine instead frees its frames without popping them, so
    /// C never runs the deferred teardown at all — the retained namespace is
    /// simply abandoned (`tclNamesp.c` deletes only from `Tcl_PopCallFrame`).
    #[test]
    fn deleting_a_suspended_coroutine_abandons_its_retained_namespace() {
        let mut i = Interp::new();
        run(&mut i, b"set log {}");
        run(
            &mut i,
            b"proc rec {old new op} {lappend ::log [list $old $op]}",
        );
        run(&mut i, b"namespace eval N {proc q {} {return Q}}");
        run(&mut i, b"trace add command ::N::q delete rec");
        run(
            &mut i,
            b"coroutine ::co apply {{} {namespace eval ::N {yield ready; list [q]}}}",
        );
        run(&mut i, b"namespace delete ::N");
        run(&mut i, b"rename ::co {}");
        assert_eq!(run(&mut i, b"list $log [namespace exists ::N]"), b"{} 0");
    }

    /// A `rename` carries a live coroutine with its command, as C carries it
    /// with the command *token*: the new name resumes it, `[info coroutine]`
    /// reports the new name from inside the body, `corotype` still finds it,
    /// and the delete side still tears it down — across namespaces too.
    /// Keeping the registry keyed by the vacated name instead would leave
    /// the coroutine undispatchable under either name.
    ///
    /// tclsh 8.6.16 and 9.0.4 both produce this sequence.
    #[test]
    fn a_rename_carries_a_live_coroutine_to_its_new_name() {
        let mut i = Interp::new();
        run(
            &mut i,
            b"proc body {} { yield; yield [info coroutine]; return B }",
        );
        // Creation runs to the first (valueless) yield.
        assert_eq!(run(&mut i, b"coroutine co body"), b"");
        run(&mut i, b"rename co c2");
        assert_eq!(run(&mut i, b"llength [info commands co]"), b"0");
        assert_eq!(run(&mut i, b"llength [info commands c2]"), b"1");
        assert_eq!(run(&mut i, b"::tcl::unsupported::corotype c2"), b"yield");
        // The resume reaches the coroutine, and the body reports the new name.
        assert_eq!(run(&mut i, b"c2"), b"::c2");
        assert_eq!(run(&mut i, b"c2"), b"B");
        assert_eq!(run(&mut i, b"llength [info commands c2]"), b"0");
        // The same across namespaces, and the delete side still terminates it.
        run(&mut i, b"namespace eval n {}");
        run(&mut i, b"coroutine k body");
        run(&mut i, b"rename k ::n::k");
        assert_eq!(run(&mut i, b"n::k"), b"::n::k");
        run(&mut i, b"rename ::n::k {}");
        assert_eq!(run(&mut i, b"llength [info commands ::n::k]"), b"0");
    }

    /// The same rule seen from the inside: a coroutine that renames *itself*
    /// while running keeps going, and its resumer's bookkeeping follows it —
    /// the next resume must reach it under the name it chose.
    #[test]
    fn a_coroutine_can_rename_itself_and_keep_running() {
        let mut i = Interp::new();
        run(
            &mut i,
            b"proc body {} { yield; rename [info coroutine] self2; \
              yield [info coroutine]; return B }",
        );
        run(&mut i, b"coroutine co body");
        assert_eq!(run(&mut i, b"co"), b"::self2");
        assert_eq!(run(&mut i, b"self2"), b"B");
        assert_eq!(run(&mut i, b"llength [info commands self2]"), b"0");
        assert_eq!(run(&mut i, b"llength [info commands co]"), b"0");
    }

    #[test]
    fn corotype_reports_active_and_yield() {
        use crate::interp::Code;
        let mut i = Interp::new();
        run(&mut i, b"proc gen {} { yield; yield }");
        run(&mut i, b"coroutine c gen");
        // A suspended coroutine is parked at a `yield`.
        assert_eq!(run(&mut i, b"::tcl::unsupported::corotype c"), b"yield");
        // A running coroutine sees itself as active.
        run(
            &mut i,
            b"proc who {} { yield [::tcl::unsupported::corotype [info coroutine]] }",
        );
        assert_eq!(run(&mut i, b"coroutine w who"), b"active");
        // A non-coroutine name errors.
        assert_eq!(
            i.eval_str(b"::tcl::unsupported::corotype nope"),
            Code::Error
        );
        assert_eq!(
            i.result_bytes(),
            b"can only get coroutine type of a coroutine"
        );
        assert_eq!(i.eval_str(b"::tcl::unsupported::corotype"), Code::Error);
    }

    #[test]
    fn original_injection_public_completions_match_both_c9_releases() {
        // naming.coroutine.original-injection-public-completions
        // docs/design/analysis/name-resolution-proofs/coroutine-original-injection-public-completions.md
        macro_rules! original_case {
            ($directory:literal, $case:literal) => {
                (include_str!(concat!("../../../rust/tcl-registry/tests/data/native_coroutine_public_injection255/", $directory, "/", $case, ".tcl")), [
                    include_str!(concat!("../../../rust/tcl-registry/tests/data/native_coroutine_public_injection255/", $directory, "/9.0.4/", $case, "/stdout")),
                    include_str!(concat!("../../../rust/tcl-registry/tests/data/native_coroutine_public_injection255/", $directory, "/9.1.0/", $case, "/stdout")),
                ])
            };
        }
        let cases = [
            original_case!("original", "yield-ok"),
            original_case!("original", "yield-error"),
            original_case!("original", "yieldto-ok"),
            original_case!("original", "yieldto-error"),
            original_case!("errors", "yield-terminal-error"),
            original_case!("errors", "yield-replaced-error"),
            original_case!("errors", "yieldto-terminal-error"),
            original_case!("errors", "yieldto-replaced-error"),
        ];
        for (provider, version) in [tcl_dialect::TclVersion::V9_0, tcl_dialect::TclVersion::V9_1]
            .into_iter()
            .enumerate()
        {
            for (source, rows) in &cases {
                let mut interp = Interp::new();
                interp.set_runtime_version(version);
                let row = rows[provider]
                    .lines()
                    .find(|row| row.starts_with("ORIGINAL|"))
                    .unwrap();
                let columns: Vec<_> = row.split('|').collect();
                assert_eq!(columns[1], "0", "fixed original C API completion");
                let expected: Vec<_> = columns[2]
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect();
                assert_eq!(
                    interp.eval_str(source.as_bytes()),
                    crate::interp::Code::Ok,
                    "{version:?}: {source}; admission={:?}; access={:?}",
                    interp.native_compilation_admission_error(),
                    interp.native_access_refusal()
                );
                assert!(!interp.host_refusal_pending(), "{version:?}: {source}");
                assert_eq!(interp.result_bytes(), expected, "{version:?}: {source}");
            }
        }
    }

    // Needs the numeric tower: the coroutine body parks in `while`.
    #[cfg(have_tommath)]
    #[test]
    fn deleting_a_suspended_coroutine_is_clean() {
        let mut i = Interp::new();
        run(&mut i, b"proc forever {} { while {1} { yield } }");
        run(&mut i, b"coroutine c forever");
        // Renaming a suspended coroutine to {} tears its worker down cleanly.
        run(&mut i, b"rename c {}");
        assert_eq!(run(&mut i, b"llength [info commands c]"), b"0");
        // The interpreter keeps working afterward (no frame-stack corruption).
        run(&mut i, b"proc p {a b} { expr {$a + $b} }");
        assert_eq!(run(&mut i, b"p 2 3"), b"5");
    }

    #[test]
    fn probe_handoff_preserves_the_original_jim_trace_object() {
        use crate::obj;
        let interp = Interp::new();
        let trace = obj::Owned::fresh(obj::new_string_bytes(b"TRACE\xff\0tail"));
        let original = trace.as_ptr();
        interp.adopt_jim_stacktrace(trace);
        interp.publish_coro_probe_error(interp.snapshot_error());
        interp.adopt_jim_stacktrace(obj::Owned::fresh(obj::new_string_bytes(b"OTHER")));
        let snapshot = interp
            .take_coro_probe_error()
            .expect("published probe receipt");
        assert!(interp.take_coro_probe_error().is_none());
        interp.restore_error(snapshot);
        assert_eq!(interp.jim_stacktrace_object().as_ptr(), original);
        assert_eq!(interp.jim_stacktrace(), b"TRACE\xff\0tail");
    }

    // Needs the numeric tower: the coroutine body parks in `while`.
    #[cfg(have_tommath)]
    #[test]
    fn coroprobe_reads_and_mutates_suspended_context() {
        use crate::interp::Code;
        let mut i = Interp::new();
        run(
            &mut i,
            b"coroutine c apply {{} { set local 42; while 1 { set got [yield ready] } }}",
        );
        // A probe reads the coroutine's frame variable; the coro stays suspended.
        assert_eq!(run(&mut i, b"coroprobe c set local"), b"42");
        assert_eq!(run(&mut i, b"coroprobe c set local"), b"42");
        assert_eq!(run(&mut i, b"llength [info commands c]"), b"1");
        // A probe mutation persists across the context swap-out.
        run(&mut i, b"coroprobe c set local 99");
        assert_eq!(run(&mut i, b"coroprobe c set local"), b"99");
        // A normal resume still works after probing.
        assert_eq!(run(&mut i, b"c hello"), b"ready");
        // Errors: not a coroutine, arity, probe-command failure.
        assert_eq!(i.eval_str(b"coroprobe nosuch set x"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can only inject a probe command into a coroutine"
        );
        assert_eq!(i.eval_str(b"coroprobe c"), Code::Error);
        assert_eq!(i.eval_str(b"coroprobe c set nope"), Code::Error);
        assert_eq!(i.result_bytes(), b"can't read \"nope\": no such variable");
    }

    // Needs the numeric tower: the coroutine body parks in `while`.
    #[cfg(have_tommath)]
    #[test]
    fn coroinject_runs_on_next_resume() {
        use crate::interp::Code;
        let mut i = Interp::new();
        run(
            &mut i,
            b"coroutine d apply {{} { while 1 { set ::last [yield ready] } }}",
        );
        // Inject returns empty and defers to the next resume.
        assert_eq!(
            run(&mut i, b"coroinject d apply {{args} {return INJECTED}}"),
            b""
        );
        // The next resume runs the injected command first; its result is what the
        // body's `yield` returns, and the resume itself returns the next yield.
        assert_eq!(run(&mut i, b"d hello"), b"ready");
        assert_eq!(run(&mut i, b"set ::last"), b"INJECTED");
        // A resume with no pending injection behaves normally.
        assert_eq!(run(&mut i, b"d world"), b"ready");
        assert_eq!(run(&mut i, b"set ::last"), b"world");
        // The injected command receives the yield command + resume value appended.
        run(
            &mut i,
            b"coroinject d apply {{args} {set ::iargs $args; return X}}",
        );
        run(&mut i, b"d payload");
        assert_eq!(run(&mut i, b"set ::iargs"), b"yield payload");
        // Not a coroutine errors.
        assert_eq!(i.eval_str(b"coroinject nosuch set x"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can only inject a command into a coroutine"
        );
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod original_operand_tests {
    use super::*;

    fn first(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
        interp.set_result(argv[1]);
        Code::Ok
    }

    fn native() -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            tcl_dialect::DialectProfile::find("tcl9.1").unwrap(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    fn dispatch(interp: &mut Interp, head: &[u8], args: &[*mut TclObj]) -> Code {
        let head = obj::Owned::fresh(obj::new_string_bytes(head));
        let mut argv = vec![head.as_ptr()];
        argv.extend_from_slice(args);
        interp.dispatch(&argv)
    }

    #[test]
    fn coroutine_probe_and_injection_keep_opaque_names_and_original_objects() {
        // Source proof: naming.coroutine.original-probe-and-injection-object-transport
        // docs/design/analysis/name-resolution-proofs/coroutine-original-probe-and-injection-object-transport.md
        let mut interp = native();
        assert_eq!(
            interp.eval_str(b"coroutine c apply {{} {return [yield READY]}}"),
            Code::Ok
        );
        let old = obj::Owned::fresh(obj::new_string_bytes(b"c"));
        let name = obj::Owned::fresh(obj::new_string_bytes(b"c\xff"));
        assert_eq!(
            dispatch(&mut interp, b"rename", &[old.as_ptr(), name.as_ptr()]),
            Code::Ok
        );
        interp.register_builtin(b"probe\xff", first);
        let head = obj::Owned::fresh(obj::new_string_bytes(b"probe\xff"));
        let argument = obj::Owned::fresh(obj::new_string_bytes(b"A\0B\xff"));
        assert_eq!(
            dispatch(
                &mut interp,
                b"::tcl::unsupported::corotype",
                &[name.as_ptr()]
            ),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"yield");
        assert_eq!(
            dispatch(
                &mut interp,
                b"coroprobe",
                &[name.as_ptr(), head.as_ptr(), argument.as_ptr()]
            ),
            Code::Ok
        );
        assert_eq!(interp.result_obj(), argument.as_ptr());
        assert_eq!(
            dispatch(
                &mut interp,
                b"coroinject",
                &[name.as_ptr(), head.as_ptr(), argument.as_ptr()]
            ),
            Code::Ok
        );
        assert_eq!(interp.dispatch(&[name.as_ptr()]), Code::Ok);
        assert_eq!(interp.result_obj(), argument.as_ptr());
        assert!(!interp.host_refusal_pending());
    }

    #[test]
    fn coroutine_name_lookup_preserves_native_missing_and_unavailable_boundaries() {
        let missing = obj::Owned::fresh(obj::new_string_bytes(b"missing\xff"));
        let mut interp = native();
        assert_eq!(
            dispatch(
                &mut interp,
                b"::tcl::unsupported::corotype",
                &[missing.as_ptr()]
            ),
            Code::Error
        );
        assert!(!interp.host_refusal_pending());
        // The default engine has a known C command-name issuer even when its
        // assistance profile is unpinned. Missing is distinct from unavailable.
        let mut default = Interp::new();
        assert_eq!(
            default.resolve_original_command_generation(missing.as_ptr()),
            Ok(None)
        );
        let unavailable = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        )));
        default.set_dialect_profile(unavailable);
        // Assistance configuration does not replace the actual physical C issuer.
        assert_eq!(
            default.resolve_original_command_generation(missing.as_ptr()),
            Ok(None)
        );
        // An unsupported actual bootstrap purpose cannot manufacture an issuer.
        assert!(
            Interp::with_native_core(
                crate::interp::default_host(),
                unavailable,
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .is_none()
        );
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod native_injection_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod native_original_holder_tests;

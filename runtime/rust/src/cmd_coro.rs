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
//! the serialized handoff (see [`SendPtr`]).
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

use crate::interp::{obj_bytes, Code, CoroContext, Interp};
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

// -- the cross-thread plumbing --------------------------------------------

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

    /// Main → worker: resume with a value, tear the coroutine down, run a probe
    /// command in the suspended context (`coroprobe`), or queue a command to run
    /// on the next resume (`coroinject`).
    pub(super) enum ToCoro {
        Resume(Vec<u8>),
        Terminate,
        Probe(Vec<Vec<u8>>),
        Inject(Vec<Vec<u8>>),
    }

    /// Worker → main: a `yield` (value), the body finished (code + result), a
    /// probe's result (code + result + its error trace, to transplant into the
    /// caller), or an acknowledgement that an inject was queued.
    pub(super) enum FromCoro {
        Yield(Vec<u8>),
        Done(Code, Vec<u8>),
        ProbeDone(Code, Vec<u8>, crate::interp::ErrorSnapshot),
        InjectAck,
    }

    /// A live coroutine: its saved execution context (swapped on handoff) plus
    /// the main-side channel ends and worker join handle.
    pub struct CoroEntry {
        pub(crate) context: CoroContext,
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
    // SAFETY: serialized cooperative handoff — see the module docs.
    unsafe impl Send for SendPtr {}

    /// A coroutine's current command name, shared between the main flow and its
    /// worker thread so a `rename` is visible on both sides: C ties the
    /// coroutine to the command *token*, and `[info coroutine]` reports
    /// whatever that token is called now.
    pub(crate) type CoroName = Arc<Mutex<Vec<u8>>>;

    /// Read the shared name. Poisoning is ignored — the value is a plain name
    /// that is never observed half-written, because the lock is only ever held
    /// across a clone or an assignment, never across anything that can unwind
    /// (the `CoroTerminate` sentinel included).
    pub(super) fn read_name(name: &CoroName) -> Vec<u8> {
        name.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Point the shared name at `value` (a `rename` of the coroutine command).
    fn write_name(name: &CoroName, value: &[u8]) {
        value.clone_into(&mut name.lock().unwrap_or_else(|e| e.into_inner()));
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

    pub(super) fn in_coroutine() -> bool {
        TLS.with(|t| t.borrow().is_some())
    }

    /// `coroutine name cmd ?arg ...?` — create a coroutine running `cmd args`,
    /// run it until its first `yield` (or completion), and return that value.
    pub(super) fn create(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
        if argv.len() < 3 {
            return interp.set_error(b"wrong # args: should be \"coroutine name cmd ?arg ...?\"");
        }
        let raw = obj_bytes(argv[1]);
        let name = interp.fqn_for(&raw);
        if interp.command_exists(&name) {
            let mut m = b"can't create procedure \"".to_vec();
            m.extend_from_slice(&raw);
            m.extend_from_slice(b"\": command already exists");
            return interp.set_error(&m);
        }
        // A leaked coroutine may still be registered under this name with no
        // command (e.g. a coroutine that renamed *itself* away while running).
        // Tear its worker down cleanly before reusing the name, so overwriting
        // the registry entry can't drop a still-parked worker (which would wake
        // on the closed channel and run concurrently with us).
        terminate(interp, &name);
        // The body command, captured as bytes to rebuild on the worker thread.
        let cmd_bytes: Vec<Vec<u8>> = argv[2..].iter().map(|&a| obj_bytes(a)).collect();

        ensure_quiet_terminate_hook();
        let (to_tx, to_rx) = std::sync::mpsc::channel::<ToCoro>();
        let (from_tx, from_rx) = std::sync::mpsc::channel::<FromCoro>();

        // Fresh execution context in the creating namespace.
        let context = CoroContext::fresh(interp.current_ns());

        // Hand a clone of the interp to the worker (sound under serialization).
        let send_interp = SendPtr(interp.clone_handle());
        let shared_name: CoroName = Arc::new(Mutex::new(name.clone()));
        let worker_name = Arc::clone(&shared_name);
        let join = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(move || worker_main(send_interp, cmd_bytes, worker_name, from_tx, to_rx))
            .expect("spawn coroutine thread");

        interp.coros_mut().insert(
            name.clone(),
            CoroEntry {
                context,
                name: shared_name,
                to_coro: to_tx,
                from_coro: Some(from_rx),
                join: Some(join),
            },
        );
        // The coroutine is invoked by name to resume it.
        interp.register_coroutine_command(&name);
        // Run to the first suspension point.
        resume(interp, &name, Vec::new())
    }

    /// The worker thread entry: wait for the first resume, run the body in the
    /// (already swapped-in) coroutine context, then hand the result back.
    fn worker_main(
        si: SendPtr,
        cmd_bytes: Vec<Vec<u8>>,
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
            let argv: Vec<*mut TclObj> =
                cmd_bytes.iter().map(|b| obj::new_string_bytes(b)).collect();
            for &a in &argv {
                unsafe { obj::incr_ref_count(a) };
            }
            let code = ip.dispatch(&argv);
            let result = ip.result_bytes();
            for &a in &argv {
                unsafe { obj::decr_ref_count(a) };
            }
            (code, result)
        }));
        // Restore the caller's context, then acknowledge (Done on normal
        // completion, or in response to a terminate). The main thread is blocked
        // in `resume`/`terminate` recv, so this is race-free.
        let ip = interp;
        // Read the name now, not at spawn: a `rename` while this coroutine was
        // suspended moved the registry key this swap has to address.
        ip.coro_swap_named(&read_name(&name));
        let msg = match outcome {
            Ok((code, result)) => FromCoro::Done(code, result),
            Err(_) => FromCoro::Done(Code::Error, Vec::new()),
        };
        let _ = from_tx.send(msg);
        // TLS drops with the thread.
    }

    /// On the worker: block until the next resume. A terminate (or a closed
    /// channel) unwinds the thread via the `CoroTerminate` panic sentinel.
    fn recv_resume() -> Vec<u8> {
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

    /// Evaluate the command `words` in the current (worker) context — used to run
    /// `coroprobe`/`coroinject` scripts inside a suspended coroutine's frames.
    /// Returns the completion code and the result bytes (mirroring the body
    /// dispatch in `worker_main`).
    fn eval_words(interp: &mut Interp, words: &[Vec<u8>]) -> (Code, Vec<u8>) {
        let argv: Vec<*mut TclObj> = words.iter().map(|b| obj::new_string_bytes(b)).collect();
        for &a in &argv {
            unsafe { obj::incr_ref_count(a) };
        }
        let code = interp.dispatch(&argv);
        let result = interp.result_bytes();
        for &a in &argv {
            unsafe { obj::decr_ref_count(a) };
        }
        (code, result)
    }

    /// `yield ?value?` — suspend the current coroutine, returning `value` to the
    /// resumer; the result is whatever value resumes it. While suspended the
    /// worker also services `coroprobe` (run a command here, stay suspended) and
    /// `coroinject` (queue a command for the next resume) requests.
    pub(super) fn do_yield(interp: &mut Interp, value: Vec<u8>) -> Code {
        let name = current_name();
        if name.is_empty() {
            return interp.set_error(b"yield can only be called in a coroutine");
        }
        // Restore the caller's context, hand back the yield value, then loop
        // servicing requests until an actual resume (or teardown) arrives.
        interp.coro_swap_named(&name);
        let mut outgoing = FromCoro::Yield(value);
        let mut pending_inject: Option<Vec<Vec<u8>>> = None;
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
                    if let Some(mut words) = pending_inject.take() {
                        words.push(b"yield".to_vec());
                        words.push(v);
                        let (code, _result) = eval_words(interp, &words);
                        return code;
                    }
                    interp.set_result_bytes(&v);
                    return Code::Ok;
                }
                // `coroprobe`: the caller has swapped our context in, so run the
                // command here, capture its error trace for transplanting, and
                // stay suspended.
                Some(ToCoro::Probe(words)) => {
                    let (code, result) = eval_words(interp, &words);
                    let snap = interp.snapshot_error();
                    outgoing = FromCoro::ProbeDone(code, result, snap);
                }
                // `coroinject`: remember the command; it runs on the next resume.
                Some(ToCoro::Inject(words)) => {
                    pending_inject = Some(words);
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
    pub(super) fn resume(interp: &mut Interp, name: &[u8], value: Vec<u8>) -> Code {
        // Swap the coroutine's context into the interpreter, then hand control
        // to its worker and block until it yields or completes.
        let exists = interp.coros_mut().contains_key(name);
        if !exists {
            return interp.invalid_command(name);
        }
        let chans = {
            let mut reg = interp.coros_mut();
            let entry = reg.get_mut(name).expect("checked above");
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
            m.extend_from_slice(name);
            m.extend_from_slice(b"\" is already running");
            return interp.set_error(&m);
        };
        interp.coro_swap_named(name);
        if to_coro.send(ToCoro::Resume(value)).is_err() {
            return interp.set_error(b"coroutine is dead");
        }
        let msg = from_coro.recv();
        // The body may have renamed the coroutine — including itself, `rename
        // [info coroutine] new` — so address the registry by where it is *now*,
        // not by the name this resume was called through.
        let name = &read_name(&shared_name);
        // Put the receiver back for the next resume.
        if let Some(entry) = interp.coros_mut().get_mut(name) {
            entry.from_coro = Some(from_coro);
        }
        match msg {
            Ok(FromCoro::Yield(v)) => {
                interp.set_result_bytes(&v);
                Code::Ok
            }
            Ok(FromCoro::Done(code, result)) => {
                finish(interp, name);
                interp.set_result_bytes(&result);
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

    /// `rename $coro $new` moved a live coroutine's command: move its registry
    /// entry to the new key and point the shared name at it, so the worker's
    /// own context swaps and `[info coroutine]` address the coroutine where it
    /// now lives. C needs no equivalent — the coroutine hangs off the command
    /// token, which a rename moves wholesale.
    pub(super) fn rename(interp: &mut Interp, old_fqn: &[u8], new_fqn: &[u8]) {
        let entry = interp.coros_mut().remove(old_fqn);
        if let Some(entry) = entry {
            write_name(&entry.name, new_fqn);
            interp.coros_mut().insert(new_fqn.to_vec(), entry);
        }
    }

    /// Tear down a completed/dead coroutine: remove the registry entry (before
    /// deleting the command, so the command-delete hook sees no coroutine), join
    /// its thread, and delete the command.
    fn finish(interp: &mut Interp, name: &[u8]) {
        let entry = interp.coros_mut().remove(name);
        if let Some(mut e) = entry {
            if let Some(j) = e.join.take() {
                let _ = j.join();
            }
        }
        interp.delete_command(name);
    }

    /// `coroprobe coroName cmd ?arg ...?` — run `cmd args` in the *suspended*
    /// coroutine `name`'s context (its frames/variables) and return the result;
    /// the coroutine stays suspended. Its error trace (if any) is transplanted
    /// into the caller once the context is swapped back out.
    pub(super) fn probe(interp: &mut Interp, name: &[u8], words: Vec<Vec<u8>>) -> Code {
        if !interp.coros_mut().contains_key(name) {
            return interp.set_error(b"can only inject a probe command into a coroutine");
        }
        let chans = {
            let mut reg = interp.coros_mut();
            let entry = reg.get_mut(name).expect("checked above");
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
        interp.coro_swap_named(name);
        let sent = to_coro.send(ToCoro::Probe(words)).is_ok();
        let msg = if sent { from_coro.recv().ok() } else { None };
        // A probe command can rename the coroutine, so swap back and put the
        // receiver away under the name it answers to now (see `resume`).
        let name = &read_name(&shared_name);
        interp.coro_swap_named(name);
        if let Some(entry) = interp.coros_mut().get_mut(name) {
            entry.from_coro = Some(from_coro);
        }
        match msg {
            Some(FromCoro::ProbeDone(code, result, snap)) => {
                interp.set_result_bytes(&result);
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
    pub(super) fn inject(interp: &mut Interp, name: &[u8], words: Vec<Vec<u8>>) -> Code {
        if !interp.coros_mut().contains_key(name) {
            return interp.set_error(b"can only inject a command into a coroutine");
        }
        let chans = {
            let mut reg = interp.coros_mut();
            let entry = reg.get_mut(name).expect("checked above");
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
        let name = &read_name(&shared_name);
        if let Some(entry) = interp.coros_mut().get_mut(name) {
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
    pub(super) fn terminate(interp: &mut Interp, name: &[u8]) {
        // Don't try to terminate from within a coroutine worker (avoid a thread
        // joining itself); only the main flow tears coroutines down.
        if in_coroutine() {
            return;
        }
        let chans = {
            let mut reg = interp.coros_mut();
            match reg.get_mut(name) {
                Some(e) => e
                    .from_coro
                    .take()
                    .map(|rx| (e.to_coro.clone(), rx, Arc::clone(&e.name))),
                None => return,
            }
        };
        let mut name = name.to_vec();
        if let Some((to_coro, from_coro, shared_name)) = chans {
            // Install the coroutine's context so its worker unwinds in its own
            // frames, then signal + await the acknowledgement.
            interp.coro_swap_named(&name);
            if to_coro.send(ToCoro::Terminate).is_ok() {
                let _ = from_coro.recv();
            }
            name = read_name(&shared_name);
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
        return interp.set_error(b"wrong # args: should be \"yield ?value?\"");
    }
    let value = argv.get(1).map(|&a| obj_bytes(a)).unwrap_or_default();
    imp::do_yield(interp, value)
}

#[cfg(not(target_arch = "wasm32"))]
fn yieldto_cmd(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    // `yieldto cmd args` yields and arranges for `cmd args` to be invoked on the
    // next resume. Not yet needed by the OO suites; report rather than misbehave.
    interp.set_error(b"yieldto is not yet implemented")
}

/// The registry key a written coroutine name addresses: its fully-qualified
/// name, followed through an open `rename` window. Inside a rename's callbacks
/// the vacating name still resolves but *is* the destination command (C's two
/// hash entries reference the one `Command`, and the coroutine hangs off that),
/// so resuming or probing through either name reaches the one coroutine.
#[cfg(not(target_arch = "wasm32"))]
fn coro_key(interp: &Interp, written: &[u8]) -> Vec<u8> {
    let fqn = interp.fqn_for(written);
    interp.renamed_cmd_key(&fqn).unwrap_or(fqn)
}

#[cfg(not(target_arch = "wasm32"))]
fn coroprobe_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp
            .set_error(b"wrong # args: should be \"coroprobe coroName cmd ?arg1 arg2 ...?\"");
    }
    let name = coro_key(interp, &obj_bytes(argv[1]));
    let words: Vec<Vec<u8>> = argv[2..].iter().map(|&a| obj_bytes(a)).collect();
    imp::probe(interp, &name, words)
}

#[cfg(not(target_arch = "wasm32"))]
fn coroinject_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp
            .set_error(b"wrong # args: should be \"coroinject coroName cmd ?arg1 arg2 ...?\"");
    }
    let name = coro_key(interp, &obj_bytes(argv[1]));
    let words: Vec<Vec<u8>> = argv[2..].iter().map(|&a| obj_bytes(a)).collect();
    imp::inject(interp, &name, words)
}

/// `::tcl::unsupported::corotype coroName` — the coroutine's current type: the
/// currently-running coroutine (e.g. `corotype [info coroutine]`) is `active`;
/// any other live coroutine is suspended at a `yield` (this runtime has no
/// `yieldto`, so never reports `yieldto`); anything else is not a coroutine.
#[cfg(not(target_arch = "wasm32"))]
fn corotype_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp
            .set_error(b"wrong # args: should be \"::tcl::unsupported::corotype coroName\"");
    }
    let name = coro_key(interp, &obj_bytes(argv[1]));
    if current_coroutine() == name {
        interp.set_result_bytes(b"active");
        return Code::Ok;
    }
    if interp.coros_mut().contains_key(&name) {
        interp.set_result_bytes(b"yield");
        return Code::Ok;
    }
    interp.set_error(b"can only get coroutine type of a coroutine")
}

/// Resume the coroutine named by `argv[0]` (its command invocation).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn coro_resume_command(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let name = coro_key(interp, &obj_bytes(argv[0]));
    let value = argv.get(1).map(|&a| obj_bytes(a)).unwrap_or_default();
    imp::resume(interp, &name, value)
}

/// Hook for command deletion (`rename $coro {}`, `delete_command`): if `name`
/// is a *suspended* coroutine, terminate its worker cleanly first.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn on_command_deleted(interp: &mut Interp, name: &[u8]) {
    let fqn = coro_key(interp, name);
    imp::terminate(interp, &fqn);
}

/// Hook for `rename $coro $new`: move a live coroutine's state to the new name.
/// C keeps the coroutine on the command *token*, so a rename is transparent to
/// it — including to `[info coroutine]`, which reports the new name from inside
/// the coroutine itself (tclsh 8.6/9.0-pinned).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn on_command_renamed(interp: &mut Interp, old_fqn: &[u8], new_fqn: &[u8]) {
    imp::rename(interp, old_fqn, new_fqn);
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

// -- wasm: no OS threads → coroutines need the explicit-stack evaluator -----

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
        return interp
            .set_error(b"wrong # args: should be \"coroprobe coroName cmd ?arg1 arg2 ...?\"");
    }
    // No coroutines exist on the single-threaded wasm build, so no name is one.
    interp.set_error(b"can only inject a probe command into a coroutine")
}

#[cfg(target_arch = "wasm32")]
fn coroinject_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp
            .set_error(b"wrong # args: should be \"coroinject coroName cmd ?arg1 arg2 ...?\"");
    }
    interp.set_error(b"can only inject a command into a coroutine")
}

#[cfg(target_arch = "wasm32")]
fn corotype_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp
            .set_error(b"wrong # args: should be \"::tcl::unsupported::corotype coroName\"");
    }
    // No coroutines exist on the single-threaded wasm build, so no name is one.
    interp.set_error(b"can only get coroutine type of a coroutine")
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn coro_resume_command(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    interp.set_error(b"coroutines are not supported in the single-threaded wasm build")
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn on_command_deleted(_interp: &mut Interp, _name: &[u8]) {}

#[cfg(target_arch = "wasm32")]
pub(crate) fn on_command_renamed(_interp: &mut Interp, _old_fqn: &[u8], _new_fqn: &[u8]) {}

#[cfg(target_arch = "wasm32")]
pub fn current_coroutine() -> Vec<u8> {
    Vec::new()
}

#[cfg(target_arch = "wasm32")]
pub fn in_coroutine() -> bool {
    false
}

/// The wasm32 stand-in for a registered coroutine: the type must exist so the
/// shared `Interp`'s `coros` table + `coro_swap_named` compile, but it is never
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
        assert_eq!(
            i.eval_str(s),
            crate::interp::Code::Ok,
            "eval failed: {:?}",
            s
        );
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
    /// and the delete side still tears it down — across namespaces too. The
    /// runtime used to leave the coroutine undispatchable under either name
    /// (its registry stayed keyed by the vacated name).
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

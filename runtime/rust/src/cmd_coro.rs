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
    interp.register_builtin(b"::tcl::unsupported::corotype", corotype_cmd);
}

// -- the cross-thread plumbing --------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::*;
    use std::cell::RefCell;
    use std::panic::AssertUnwindSafe;
    use std::sync::mpsc::{Receiver, Sender};
    use std::sync::Once;
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

    /// Main → worker: resume with a value, or tear the coroutine down.
    pub(super) enum ToCoro {
        Resume(Vec<u8>),
        Terminate,
    }

    /// Worker → main: a `yield` (value), or the body finished (code + result).
    pub(super) enum FromCoro {
        Yield(Vec<u8>),
        Done(Code, Vec<u8>),
    }

    /// A live coroutine: its saved execution context (swapped on handoff) plus
    /// the main-side channel ends and worker join handle.
    pub struct CoroEntry {
        pub(crate) context: CoroContext,
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

    /// The worker thread's own handles + identity (its end of the channels).
    struct CoroTls {
        name: Vec<u8>,
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
                .map(|c| c.name.clone())
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
        let worker_name = name.clone();
        let join = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(move || worker_main(send_interp, cmd_bytes, worker_name, from_tx, to_rx))
            .expect("spawn coroutine thread");

        interp.coros_mut().insert(
            name.clone(),
            CoroEntry {
                context,
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
        name: Vec<u8>,
        from_tx: Sender<FromCoro>,
        to_rx: Receiver<ToCoro>,
    ) {
        // Install this thread's identity + channel ends.
        TLS.with(|t| {
            *t.borrow_mut() = Some(CoroTls {
                name: name.clone(),
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
        ip.coro_swap_named(&name);
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

    /// `yield ?value?` — suspend the current coroutine, returning `value` to the
    /// resumer; the result is whatever value resumes it.
    pub(super) fn do_yield(interp: &mut Interp, value: Vec<u8>) -> Code {
        let name = current_name();
        if name.is_empty() {
            return interp.set_error(b"yield can only be called in a coroutine");
        }
        // Restore the caller's context, hand back the yield value, and block.
        interp.coro_swap_named(&name);
        let resumed = TLS.with(|t| {
            let b = t.borrow();
            let tls = b.as_ref().expect("coroutine TLS");
            if tls.from_coro.send(FromCoro::Yield(value)).is_err() {
                return None;
            }
            match tls.to_coro.recv() {
                Ok(ToCoro::Resume(v)) => Some(v),
                _ => None,
            }
        });
        match resumed {
            Some(v) => {
                interp.set_result_bytes(&v);
                Code::Ok
            }
            // Deleted while suspended: unwind this worker without touching the
            // interpreter further (the main thread is blocked awaiting us).
            None => std::panic::panic_any(CoroTerminate),
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
            entry.from_coro.take().map(|rx| (entry.to_coro.clone(), rx))
        };
        let Some((to_coro, from_coro)) = chans else {
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
            Err(_) => {
                finish(interp, name);
                interp.set_error(b"coroutine is dead")
            }
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
                Some(e) => e.from_coro.take().map(|rx| (e.to_coro.clone(), rx)),
                None => return,
            }
        };
        if let Some((to_coro, from_coro)) = chans {
            // Install the coroutine's context so its worker unwinds in its own
            // frames, then signal + await the acknowledgement.
            interp.coro_swap_named(name);
            if to_coro.send(ToCoro::Terminate).is_ok() {
                let _ = from_coro.recv();
            }
        }
        let entry = interp.coros_mut().remove(name);
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
    let name = interp.fqn_for(&obj_bytes(argv[1]));
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
    let name = interp.fqn_for(&obj_bytes(argv[0]));
    let value = argv.get(1).map(|&a| obj_bytes(a)).unwrap_or_default();
    imp::resume(interp, &name, value)
}

/// Hook for command deletion (`rename $coro {}`, `delete_command`): if `name`
/// is a *suspended* coroutine, terminate its worker cleanly first.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn on_command_deleted(interp: &mut Interp, name: &[u8]) {
    let fqn = interp.fqn_for(name);
    imp::terminate(interp, &fqn);
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
}

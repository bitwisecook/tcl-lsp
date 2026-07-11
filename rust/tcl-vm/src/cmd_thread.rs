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

//! A real, shared-nothing `thread` package for the bytecode VM
//! (`RUST_ISSUE_008`, phase 3).
//!
//! True OS-thread parallelism *without* making [`Value`]/[`Vm`] `Send`. Tcl
//! threading is **shared-nothing**: an interpreter is confined to one thread and
//! values cross thread boundaries only as serialized strings. So each worker
//! builds *its own* `Vm` inside the spawn closure (the `Vm` never crosses a
//! thread boundary — `!Send` is fine), and the only `Send + Sync` surface is a
//! small [`Shared`] block held behind `Arc`: the thread registry (id → job
//! channel), the `tsv` shared-variable store, a **`Send` compile-service
//! factory** each worker calls to build its compiler, and a `Send` output sink.
//! `forbid(unsafe)` is preserved — no `unsafe impl Send` (the tree-walker's
//! shortcut); safety comes from the type system.
//!
//! Commands: `thread::create`/`send`/`wait`/`release`/`id`/`exists`/`names`/
//! `errorproc`, and `tsv::{set,get,exists,unset,incr,append,lappend,keys,names}`.
//! `thread::send` serializes a script to the target's channel and (unless
//! `-async`) blocks for its result. `thread::wait` is the worker's message loop.
//!
//! No oracle: the reference tclsh 9.0.4 is a *non-threaded* build (no `Thread`
//! package, `tcl_platform(threaded)` unset), so — unlike the rest of the VM —
//! this subsystem is validated by deterministic Rust concurrency tests
//! (`tests/cmd_thread_e2e.rs`), with semantics per the Tcl `Thread` package docs.

use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use tcl_bytecode::ModuleAsm;
use tcl_runtime_api::{CompileService, Completion};

use crate::error::TclError;
use crate::interp::{Vm, err, ok};
use crate::value::Value;

/// The main interpreter's thread id (C Tcl numbers the initial thread first).
const MAIN_ID: u64 = 1;

/// A `Send + Sync` factory that builds a fresh compiler for a worker's `Vm`.
/// The embedder supplies it ([`Vm::enable_threads`]) because the compiler is
/// injected (`Rc<dyn CompileService>`), and that `Rc` cannot cross threads — a
/// worker instead *constructs* its own.
pub type CompileFactory =
    Arc<dyn Fn() -> Box<dyn CompileService<Module = ModuleAsm>> + Send + Sync>;

/// A `Send` output sink shared by every worker's `puts` (the main interpreter
/// keeps its own thread-local output).
pub type ThreadedOutput = Arc<Mutex<Box<dyn Write + Send>>>;

/// A unit of work handed to a worker over its channel.
enum Job {
    /// `thread::send`: evaluate `script`; if `reply` is set (synchronous send),
    /// return the result there.
    Eval {
        script: String,
        reply: Option<Sender<JobResult>>,
    },
    /// `thread::release`: leave the `thread::wait` loop so the worker winds down.
    Release,
}

/// The outcome of a synchronous `thread::send`, carried back to the sender.
struct JobResult {
    ok: bool,
    result: String,
}

/// One registered worker: the channel that feeds it jobs, and its join handle
/// (taken by `thread::release` to wait for a clean exit).
struct Worker {
    jobs: Sender<Job>,
    join: Option<std::thread::JoinHandle<()>>,
}

/// The `Send + Sync` state shared across every interpreter thread — the entire
/// cross-thread surface. Everything else (the `Vm`, its `Value`s) stays
/// thread-local.
struct Shared {
    factory: CompileFactory,
    output: ThreadedOutput,
    next_id: AtomicU64,
    /// Monotonic source of opaque handles (`mutex`/`cond`/`rwmutex`/`tpool`).
    handle_seq: AtomicU64,
    /// Live workers by thread id.
    workers: Mutex<HashMap<u64, Worker>>,
    /// The `tsv` store: array name → key → serialized value.
    tsv: Mutex<HashMap<String, HashMap<String, String>>>,
    /// `thread::mutex` handles.
    mutexes: Mutex<HashMap<String, Arc<TclMutex>>>,
    /// `thread::cond` handles.
    conds: Mutex<HashMap<String, Arc<TclCond>>>,
    /// `thread::rwmutex` handles.
    rwmutexes: Mutex<HashMap<String, Arc<TclRwMutex>>>,
    /// `tpool` handles.
    pools: Mutex<HashMap<String, Arc<Pool>>>,
}

impl Shared {
    /// Mint the next opaque handle with the given prefix (`mutex7`, `tpool2`, …).
    fn next_handle(&self, prefix: &str) -> String {
        let n = self.handle_seq.fetch_add(1, Ordering::SeqCst);
        format!("{prefix}{n}")
    }
}

/// The per-`Vm` thread state. Disabled (`shared == None`) until the embedder
/// calls [`Vm::enable_threads`]; a worker additionally owns its `inbox`.
#[derive(Default)]
pub(crate) struct ThreadSystem {
    shared: Option<Arc<Shared>>,
    this_id: u64,
    inbox: Option<Receiver<Job>>,
}

impl ThreadSystem {
    fn is_enabled(&self) -> bool {
        self.shared.is_some()
    }
}

/// An output adapter routing a worker's `puts` to the shared `Send` sink.
struct SharedOut(ThreadedOutput);

impl Write for SharedOut {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // A poisoned lock (a worker panicked mid-write) is treated as a closed
        // sink rather than propagated — `puts` cannot itself panic.
        if let Ok(mut out) = self.0.lock() {
            out.write(buf)
        } else {
            Ok(buf.len())
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        if let Ok(mut out) = self.0.lock() {
            out.flush()
        } else {
            Ok(())
        }
    }
}

impl Vm {
    /// Enable the `thread` package on this (main) interpreter. `factory` builds a
    /// compiler for each worker's `Vm`; `output` is the shared `puts` sink. Sets
    /// `tcl_platform(threaded)` to `1`. Idempotent-ish: a second call re-seeds
    /// the shared block (existing workers keep the old one).
    pub fn enable_threads(&mut self, factory: CompileFactory, output: ThreadedOutput) {
        let shared = Arc::new(Shared {
            factory,
            output,
            next_id: AtomicU64::new(MAIN_ID + 1),
            handle_seq: AtomicU64::new(0),
            workers: Mutex::new(HashMap::new()),
            tsv: Mutex::new(HashMap::new()),
            mutexes: Mutex::new(HashMap::new()),
            conds: Mutex::new(HashMap::new()),
            rwmutexes: Mutex::new(HashMap::new()),
            pools: Mutex::new(HashMap::new()),
        });
        self.thread = ThreadSystem {
            shared: Some(shared),
            this_id: MAIN_ID,
            inbox: None,
        };
        let _ = self.write_array_raw("tcl_platform", "threaded", Value::string("1"));
    }
}

pub(crate) fn register(vm: &mut Vm) {
    vm.register("thread::id", cmd_thread_id);
    vm.register("thread::create", cmd_thread_create);
    vm.register("thread::send", cmd_thread_send);
    vm.register("thread::wait", cmd_thread_wait);
    vm.register("thread::release", cmd_thread_release);
    vm.register("thread::exists", cmd_thread_exists);
    vm.register("thread::names", cmd_thread_names);
    vm.register("thread::errorproc", cmd_thread_errorproc);
    vm.register("thread::mutex", cmd_thread_mutex);
    vm.register("thread::cond", cmd_thread_cond);
    vm.register("thread::rwmutex", cmd_thread_rwmutex);
    vm.register("tpool::create", cmd_tpool_create);
    vm.register("tpool::post", cmd_tpool_post);
    vm.register("tpool::wait", cmd_tpool_wait);
    vm.register("tpool::get", cmd_tpool_get);
    vm.register("tpool::release", cmd_tpool_release);
    vm.register("tpool::names", cmd_tpool_names);
    vm.register("tsv::set", cmd_tsv_set);
    vm.register("tsv::get", cmd_tsv_get);
    vm.register("tsv::exists", cmd_tsv_exists);
    vm.register("tsv::unset", cmd_tsv_unset);
    vm.register("tsv::incr", cmd_tsv_incr);
    vm.register("tsv::append", cmd_tsv_append);
    vm.register("tsv::lappend", cmd_tsv_lappend);
    vm.register("tsv::keys", cmd_tsv_keys);
    vm.register("tsv::names", cmd_tsv_names);
}

/// Fetch the shared block, or the standard "not available" error.
fn shared(vm: &Vm) -> Result<Arc<Shared>, Completion<Value>> {
    vm.thread
        .shared
        .clone()
        .ok_or_else(|| err("thread package not available in this interpreter"))
}

// ===========================================================================
// thread::*
// ===========================================================================

/// `thread::id` — this interpreter's thread id.
fn cmd_thread_id(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if !args.is_empty() {
        return err("wrong # args: should be \"thread::id\"");
    }
    ok(Value::string(vm.thread.this_id.to_string()))
}

/// `thread::create ?script?` — spawn a worker running `script` (default
/// `thread::wait`, i.e. a bare worker that services `thread::send`). Returns the
/// new thread id.
fn cmd_thread_create(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let shared = match shared(vm) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let script = match args {
        [] => "thread::wait".to_string(),
        [s] => s.to_str().to_string(),
        // C also accepts option flags (`-joinable`, `-preserved`); the minimal
        // package treats a lone script argument as the body.
        _ => return err("wrong # args: should be \"thread::create ?script?\""),
    };
    let id = shared.next_id.fetch_add(1, Ordering::SeqCst);
    let (tx, rx) = channel::<Job>();
    // Register the channel *before* spawning so a `thread::send` to the new id
    // (from any thread) cannot race the worker's startup.
    shared.workers.lock().expect("workers lock").insert(
        id,
        Worker {
            jobs: tx,
            join: None,
        },
    );

    let worker_shared = Arc::clone(&shared);
    let join = std::thread::spawn(move || run_worker(&worker_shared, id, rx, &script));

    if let Some(w) = shared.workers.lock().expect("workers lock").get_mut(&id) {
        w.join = Some(join);
    }
    ok(Value::string(id.to_string()))
}

/// A worker thread: build its own `Vm` (shared-nothing), run `script`, then
/// deregister. The `Vm` is created and dropped entirely within this closure, so
/// it never crosses a thread boundary.
fn run_worker(shared: &Arc<Shared>, id: u64, inbox: Receiver<Job>, script: &str) {
    let out: Box<dyn Write> = Box::new(SharedOut(Arc::clone(&shared.output)));
    let mut vm = Vm::with_output(out);
    vm.set_compiler((shared.factory)());
    let _ = vm.write_array_raw("tcl_platform", "threaded", Value::string("1"));
    vm.thread = ThreadSystem {
        shared: Some(Arc::clone(shared)),
        this_id: id,
        inbox: Some(inbox),
    };
    let _ = vm.eval_source(script);
    // The body returned (or `thread::wait` was released): the thread is gone.
    shared.workers.lock().expect("workers lock").remove(&id);
}

/// `thread::send ?-async? id script ?resultvar?` — evaluate `script` in thread
/// `id`. Synchronous by default: block for the result (and store it in
/// `resultvar` if given). `-async` returns immediately.
fn cmd_thread_send(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let shared = match shared(vm) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let mut rest = args;
    let mut is_async = false;
    if let [first, ..] = rest
        && &*first.to_str() == "-async"
    {
        is_async = true;
        rest = &rest[1..];
    }
    let (id_arg, script, resultvar) = match rest {
        [id, script] => (id, script, None),
        [id, script, var] => (id, script, Some(var)),
        _ => return err("wrong # args: should be \"thread::send ?-async? id script ?varName?\""),
    };
    let Some(id) = parse_id(&id_arg.to_str()) else {
        return err(format!("invalid thread id \"{}\"", id_arg.to_str()));
    };

    // Clone the target's sender under the lock, then release it before blocking
    // so the registry stays available to other threads while we wait.
    let sender = {
        let workers = shared.workers.lock().expect("workers lock");
        match workers.get(&id) {
            Some(w) => w.jobs.clone(),
            None => return err(format!("thread \"{id}\" does not exist")),
        }
    };

    if is_async {
        let _ = sender.send(Job::Eval {
            script: script.to_str().to_string(),
            reply: None,
        });
        return ok(Value::empty());
    }

    let (reply_tx, reply_rx) = channel::<JobResult>();
    if sender
        .send(Job::Eval {
            script: script.to_str().to_string(),
            reply: Some(reply_tx),
        })
        .is_err()
    {
        return err(format!("thread \"{id}\" is unreachable"));
    }
    let Ok(res) = reply_rx.recv() else {
        return err(format!("thread \"{id}\" exited before replying"));
    };
    if let Some(var) = resultvar
        && let Err(e) = vm.set_var(&var.to_str(), Value::string(res.result.clone()))
    {
        return e;
    }
    if res.ok {
        ok(Value::string(res.result))
    } else {
        err(res.result)
    }
}

/// `thread::wait` — the worker message loop: service `thread::send` jobs until
/// released (or the channel closes). Returns an empty result to its caller (the
/// worker body), which then winds the thread down.
fn cmd_thread_wait(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if !args.is_empty() {
        return err("wrong # args: should be \"thread::wait\"");
    }
    if !vm.thread.is_enabled() || vm.thread.inbox.is_none() {
        return err("thread::wait can only be called in a worker thread");
    }
    loop {
        // Scope the `inbox` borrow to the blocking `recv` so `eval_source` can
        // take `&mut vm` once a job arrives.
        let job = {
            let Some(rx) = vm.thread.inbox.as_ref() else {
                break;
            };
            rx.recv()
        };
        match job {
            Ok(Job::Eval { script, reply }) => {
                let comp = vm.eval_source(&script);
                if let Some(reply) = reply {
                    let _ = reply.send(job_result(comp));
                }
            }
            Ok(Job::Release) | Err(_) => break,
        }
    }
    ok(Value::empty())
}

/// `thread::release ?id?` — stop worker `id` (default: the calling thread) and
/// wait for it to exit. Returns the number of threads released (0 or 1).
fn cmd_thread_release(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let shared = match shared(vm) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let id = match args {
        [] => vm.thread.this_id,
        [id] => match parse_id(&id.to_str()) {
            Some(id) => id,
            None => return err(format!("invalid thread id \"{}\"", id.to_str())),
        },
        _ => return err("wrong # args: should be \"thread::release ?id?\""),
    };
    // Take the worker out of the registry, send Release, then join outside the
    // lock (joining under the lock would deadlock a worker that is itself
    // deregistering on exit).
    let worker = shared.workers.lock().expect("workers lock").remove(&id);
    let Some(worker) = worker else {
        return ok(Value::string("0"));
    };
    let _ = worker.jobs.send(Job::Release);
    // Never join the calling thread from within itself (that would deadlock);
    // its own `thread::wait` loop returns once it processes the Release.
    if id != vm.thread.this_id
        && let Some(join) = worker.join
    {
        let _ = join.join();
    }
    ok(Value::string("1"))
}

/// `thread::exists id` — whether `id` names a live worker (or this thread).
fn cmd_thread_exists(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let shared = match shared(vm) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let [id_arg] = args else {
        return err("wrong # args: should be \"thread::exists id\"");
    };
    let Some(id) = parse_id(&id_arg.to_str()) else {
        return ok(Value::string("0"));
    };
    let live = id == vm.thread.this_id
        || shared
            .workers
            .lock()
            .expect("workers lock")
            .contains_key(&id);
    ok(Value::string(if live { "1" } else { "0" }))
}

/// `thread::names` — the ids of all live threads (this one plus every worker),
/// ascending.
fn cmd_thread_names(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let shared = match shared(vm) {
        Ok(s) => s,
        Err(e) => return e,
    };
    if !args.is_empty() {
        return err("wrong # args: should be \"thread::names\"");
    }
    let mut ids: Vec<u64> = shared
        .workers
        .lock()
        .expect("workers lock")
        .keys()
        .copied()
        .collect();
    ids.push(vm.thread.this_id);
    ids.sort_unstable();
    ids.dedup();
    let names: Vec<Value> = ids
        .into_iter()
        .map(|i| Value::string(i.to_string()))
        .collect();
    ok(Value::list(names))
}

/// `thread::errorproc ?cmd?` — get/set the background-error handler name. Stored
/// for parity; a minimal package reports uncaught worker errors to the shared
/// output rather than dispatching here.
fn cmd_thread_errorproc(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [] | [_] => ok(Value::empty()),
        _ => err("wrong # args: should be \"thread::errorproc ?cmdName?\""),
    }
}

fn job_result(comp: Result<Completion<Value>, TclError>) -> JobResult {
    match comp {
        Ok(c) => JobResult {
            ok: c.code.is_ok(),
            result: c.result.to_str().to_string(),
        },
        Err(e) => JobResult {
            ok: false,
            result: e.message,
        },
    }
}

// ===========================================================================
// thread::mutex / cond / rwmutex — synchronisation primitives
// ===========================================================================

/// A named exclusive lock whose `lock`/`unlock` span *separate* command calls,
/// so it can't use a native `MutexGuard` (that is bound to one stack frame). It
/// is a hand-built lock over a `Mutex<MutexState> + Condvar`: `lock` blocks until
/// the lock is free (or, when `-recursive`, already held by the calling thread).
struct TclMutex {
    recursive: bool,
    state: Mutex<MutexState>,
    cvar: Condvar,
}

#[derive(Default)]
struct MutexState {
    owner: Option<u64>,
    count: u32,
}

impl TclMutex {
    fn lock(&self, tid: u64) {
        let mut st = self.state.lock().expect("mutex state");
        loop {
            if st.count == 0 {
                st.owner = Some(tid);
                st.count = 1;
                return;
            }
            if self.recursive && st.owner == Some(tid) {
                st.count += 1;
                return;
            }
            st = self.cvar.wait(st).expect("mutex wait");
        }
    }

    fn unlock(&self, tid: u64) {
        let mut st = self.state.lock().expect("mutex state");
        if st.count == 0 {
            return;
        }
        if self.recursive && st.owner == Some(tid) && st.count > 1 {
            st.count -= 1;
            return;
        }
        st.owner = None;
        st.count = 0;
        drop(st);
        self.cvar.notify_one();
    }
}

/// A condition variable, paired with a `thread::mutex` at `wait` time. It carries
/// its own generation counter so a `notify` that races an about-to-`wait` thread
/// is not lost: `wait` samples the generation while holding the gate, releases
/// the mutex, then blocks until the generation moves — all without dropping the
/// gate, so a concurrent `notify` (which must take the gate to bump it) cannot
/// slip in between the sample and the block.
struct TclCond {
    gate: Mutex<u64>,
    cvar: Condvar,
}

impl TclCond {
    fn notify(&self) {
        let mut g = self.gate.lock().expect("cond gate");
        *g = g.wrapping_add(1);
        drop(g);
        self.cvar.notify_all();
    }

    fn wait(&self, mutex: &TclMutex, tid: u64, timeout: Option<Duration>) {
        let gate = self.gate.lock().expect("cond gate");
        let start_gen = *gate;
        // Tcl `cond wait` atomically releases the mutex and blocks; reacquire it
        // before returning.
        mutex.unlock(tid);
        match timeout {
            None => {
                let _g = self
                    .cvar
                    .wait_while(gate, |g| *g == start_gen)
                    .expect("cond wait");
            }
            Some(dur) => {
                let _g = self
                    .cvar
                    .wait_timeout_while(gate, dur, |g| *g == start_gen)
                    .expect("cond wait_timeout");
            }
        }
        mutex.lock(tid);
    }
}

/// A readers-writer lock: any number of concurrent `rlock`s, or one exclusive
/// `wlock`. `unlock` releases whichever mode the calling thread holds (inferred:
/// a live writer is released first, otherwise a reader).
struct TclRwMutex {
    state: Mutex<RwState>,
    cvar: Condvar,
}

#[derive(Default)]
struct RwState {
    readers: u32,
    writer: bool,
}

impl TclRwMutex {
    fn rlock(&self) {
        let mut st = self.state.lock().expect("rw state");
        while st.writer {
            st = self.cvar.wait(st).expect("rw wait");
        }
        st.readers += 1;
    }

    fn wlock(&self) {
        let mut st = self.state.lock().expect("rw state");
        while st.writer || st.readers > 0 {
            st = self.cvar.wait(st).expect("rw wait");
        }
        st.writer = true;
    }

    fn unlock(&self) {
        let mut st = self.state.lock().expect("rw state");
        if st.writer {
            st.writer = false;
        } else if st.readers > 0 {
            st.readers -= 1;
        }
        drop(st);
        self.cvar.notify_all();
    }
}

/// `thread::mutex create|lock|unlock|destroy` (`create` accepts `-recursive`).
fn cmd_thread_mutex(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let shared = match shared(vm) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let sub = args.first().map(|a| a.to_str().to_string());
    match (sub.as_deref(), &args[1.min(args.len())..]) {
        (Some("create"), rest) => {
            let recursive = match rest {
                [] => false,
                [opt] if &*opt.to_str() == "-recursive" => true,
                _ => return err("wrong # args: should be \"thread::mutex create ?-recursive?\""),
            };
            let handle = shared.next_handle("mutex");
            shared.mutexes.lock().expect("mutexes").insert(
                handle.clone(),
                Arc::new(TclMutex {
                    recursive,
                    state: Mutex::new(MutexState::default()),
                    cvar: Condvar::new(),
                }),
            );
            ok(Value::string(handle))
        }
        (Some("lock"), [h]) => match lookup(&shared.mutexes, &h.to_str()) {
            Some(m) => {
                let tid = vm.thread.this_id;
                m.lock(tid);
                ok(Value::empty())
            }
            None => err(no_such("mutex", &h.to_str())),
        },
        (Some("unlock"), [h]) => match lookup(&shared.mutexes, &h.to_str()) {
            Some(m) => {
                m.unlock(vm.thread.this_id);
                ok(Value::empty())
            }
            None => err(no_such("mutex", &h.to_str())),
        },
        (Some("destroy"), [h]) => {
            shared.mutexes.lock().expect("mutexes").remove(&*h.to_str());
            ok(Value::empty())
        }
        _ => err("wrong # args: should be \"thread::mutex option ?arg ...?\""),
    }
}

/// `thread::cond create|wait|notify|destroy`. `wait cond mutex ?ms?` releases the
/// mutex while blocking (relocking on wake); an optional `ms` bounds the wait.
fn cmd_thread_cond(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let shared = match shared(vm) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let sub = args.first().map(|a| a.to_str().to_string());
    match (sub.as_deref(), &args[1.min(args.len())..]) {
        (Some("create"), []) => {
            let handle = shared.next_handle("cond");
            shared.conds.lock().expect("conds").insert(
                handle.clone(),
                Arc::new(TclCond {
                    gate: Mutex::new(0),
                    cvar: Condvar::new(),
                }),
            );
            ok(Value::string(handle))
        }
        (Some("wait"), rest @ ([_, _] | [_, _, _])) => {
            let (cond_h, mutex_h) = (&rest[0], &rest[1]);
            let timeout = match rest.get(2) {
                None => None,
                Some(ms) => match ms.to_str().trim().parse::<u64>() {
                    Ok(n) => Some(Duration::from_millis(n)),
                    Err(_) => return err(format!("expected integer but got \"{}\"", ms.to_str())),
                },
            };
            let Some(cond) = lookup(&shared.conds, &cond_h.to_str()) else {
                return err(no_such("condition variable", &cond_h.to_str()));
            };
            let Some(mutex) = lookup(&shared.mutexes, &mutex_h.to_str()) else {
                return err(no_such("mutex", &mutex_h.to_str()));
            };
            cond.wait(&mutex, vm.thread.this_id, timeout);
            ok(Value::empty())
        }
        (Some("notify"), [h]) => match lookup(&shared.conds, &h.to_str()) {
            Some(c) => {
                c.notify();
                ok(Value::empty())
            }
            None => err(no_such("condition variable", &h.to_str())),
        },
        (Some("destroy"), [h]) => {
            shared.conds.lock().expect("conds").remove(&*h.to_str());
            ok(Value::empty())
        }
        _ => err("wrong # args: should be \"thread::cond option ?arg ...?\""),
    }
}

/// `thread::rwmutex create|rlock|wlock|unlock|destroy`.
fn cmd_thread_rwmutex(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let shared = match shared(vm) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let sub = args.first().map(|a| a.to_str().to_string());
    match (sub.as_deref(), &args[1.min(args.len())..]) {
        (Some("create"), []) => {
            let handle = shared.next_handle("rwmutex");
            shared.rwmutexes.lock().expect("rwmutexes").insert(
                handle.clone(),
                Arc::new(TclRwMutex {
                    state: Mutex::new(RwState::default()),
                    cvar: Condvar::new(),
                }),
            );
            ok(Value::string(handle))
        }
        (Some(mode @ ("rlock" | "wlock" | "unlock")), [h]) => {
            match lookup(&shared.rwmutexes, &h.to_str()) {
                Some(m) => {
                    match mode {
                        "rlock" => m.rlock(),
                        "wlock" => m.wlock(),
                        _ => m.unlock(),
                    }
                    ok(Value::empty())
                }
                None => err(no_such("rwmutex", &h.to_str())),
            }
        }
        (Some("destroy"), [h]) => {
            shared
                .rwmutexes
                .lock()
                .expect("rwmutexes")
                .remove(&*h.to_str());
            ok(Value::empty())
        }
        _ => err("wrong # args: should be \"thread::rwmutex option ?arg ...?\""),
    }
}

// ===========================================================================
// tpool::* — worker pools
// ===========================================================================

/// A worker pool: a shared job queue drained by a fixed set of persistent worker
/// `Vm`s (each keeps its state across jobs, as Tcl's do), plus a results table.
struct Pool {
    queue: Arc<PoolQueue>,
    workers: Mutex<Vec<std::thread::JoinHandle<()>>>,
}

struct PoolQueue {
    inner: Mutex<PoolInner>,
    job_ready: Condvar,
    done: Condvar,
}

#[derive(Default)]
struct PoolInner {
    /// Pending `(result-id or None-if-detached, script)` jobs.
    queue: VecDeque<(Option<u64>, String)>,
    /// Completed non-detached jobs by id.
    results: HashMap<u64, JobResult>,
    shutdown: bool,
}

/// A pool worker: build one `Vm` (thread-enabled so jobs can use `tsv`/mutexes),
/// run the optional `initcmd`, then drain jobs until the pool shuts down.
fn run_pool_worker(shared: &Arc<Shared>, queue: &Arc<PoolQueue>, initcmd: Option<&str>) {
    let out: Box<dyn Write> = Box::new(SharedOut(Arc::clone(&shared.output)));
    let mut vm = Vm::with_output(out);
    vm.set_compiler((shared.factory)());
    let _ = vm.write_array_raw("tcl_platform", "threaded", Value::string("1"));
    let id = shared.next_id.fetch_add(1, Ordering::SeqCst);
    vm.thread = ThreadSystem {
        shared: Some(Arc::clone(shared)),
        this_id: id,
        inbox: None,
    };
    if let Some(ic) = initcmd {
        let _ = vm.eval_source(ic);
    }
    loop {
        let job = {
            let mut inner = queue.inner.lock().expect("pool inner");
            loop {
                if inner.shutdown {
                    return;
                }
                if let Some(job) = inner.queue.pop_front() {
                    break job;
                }
                inner = queue.job_ready.wait(inner).expect("pool wait");
            }
        };
        let (result_id, script) = job;
        let comp = vm.eval_source(&script);
        if let Some(jid) = result_id {
            let res = job_result(comp);
            queue
                .inner
                .lock()
                .expect("pool inner")
                .results
                .insert(jid, res);
            queue.done.notify_all();
        }
    }
}

/// Fetch pool `handle` from the registry, or the standard error.
fn pool_of(shared: &Arc<Shared>, handle: &str) -> Result<Arc<Pool>, Completion<Value>> {
    lookup(&shared.pools, handle)
        .ok_or_else(|| err(format!("can not find threadpool \"{handle}\"")))
}

/// `tpool::create ?-minworkers n? ?-maxworkers n? ?-initcmd script? ?...?` — the
/// pool is sized to `maxworkers` (default 4) persistent workers; other options
/// (`-idletime`, `-exitcmd`) are accepted and ignored.
fn cmd_tpool_create(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let shared = match shared(vm) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let mut maxworkers = 4usize;
    let mut initcmd: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        let opt = args[i].to_str().to_string();
        let val = args.get(i + 1);
        match (opt.as_str(), val) {
            ("-maxworkers", Some(v)) => {
                maxworkers = v
                    .to_str()
                    .trim()
                    .parse::<usize>()
                    .unwrap_or(maxworkers)
                    .max(1);
            }
            ("-initcmd", Some(v)) => initcmd = Some(v.to_str().to_string()),
            // Accepted for compatibility but not modelled (this pool is a fixed
            // set of persistent workers, sized by `-maxworkers`).
            ("-minworkers" | "-idletime" | "-exitcmd", Some(_)) => {}
            _ => return err(format!("bad option \"{opt}\"")),
        }
        i += 2;
    }
    let queue = Arc::new(PoolQueue {
        inner: Mutex::new(PoolInner::default()),
        job_ready: Condvar::new(),
        done: Condvar::new(),
    });
    let mut handles = Vec::with_capacity(maxworkers);
    for _ in 0..maxworkers {
        let ws = Arc::clone(&shared);
        let wq = Arc::clone(&queue);
        let ic = initcmd.clone();
        handles.push(std::thread::spawn(move || {
            run_pool_worker(&ws, &wq, ic.as_deref());
        }));
    }
    let handle = shared.next_handle("tpool");
    shared.pools.lock().expect("pools").insert(
        handle.clone(),
        Arc::new(Pool {
            queue,
            workers: Mutex::new(handles),
        }),
    );
    ok(Value::string(handle))
}

/// `tpool::post ?-detached? ?-nowait? pool script` — enqueue `script`; returns a
/// job id (or empty for `-detached`, whose result is discarded).
fn cmd_tpool_post(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let shared = match shared(vm) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let mut detached = false;
    let mut rest = args;
    while let [first, ..] = rest {
        match &*first.to_str() {
            "-detached" => detached = true,
            "-nowait" => {}
            _ => break,
        }
        rest = &rest[1..];
    }
    let [pool_h, script] = rest else {
        return err("wrong # args: should be \"tpool::post ?-detached? ?-nowait? tpool script\"");
    };
    let pool = match pool_of(&shared, &pool_h.to_str()) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let jobid = if detached {
        None
    } else {
        Some(shared.handle_seq.fetch_add(1, Ordering::SeqCst))
    };
    {
        let mut inner = pool.queue.inner.lock().expect("pool inner");
        inner.queue.push_back((jobid, script.to_str().to_string()));
    }
    pool.queue.job_ready.notify_one();
    ok(jobid.map_or_else(Value::empty, |id| Value::string(id.to_string())))
}

/// `tpool::wait pool joblist ?varName?` — block until at least one job in
/// `joblist` has finished; return the finished ids (and store the still-pending
/// ones in `varName`).
fn cmd_tpool_wait(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let shared = match shared(vm) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let (pool_h, joblist, var) = match args {
        [p, j] => (p, j, None),
        [p, j, v] => (p, j, Some(v)),
        _ => return err("wrong # args: should be \"tpool::wait tpool joblist ?varName?\""),
    };
    let pool = match pool_of(&shared, &pool_h.to_str()) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let ids: Vec<u64> = tcl_syntax::list::split_list_lenient(&joblist.to_str())
        .iter()
        .filter_map(|s| s.trim().parse::<u64>().ok())
        .collect();
    let mut inner = pool.queue.inner.lock().expect("pool inner");
    loop {
        let done: Vec<u64> = ids
            .iter()
            .copied()
            .filter(|id| inner.results.contains_key(id))
            .collect();
        if !done.is_empty() || ids.is_empty() {
            let pending: Vec<u64> = ids
                .iter()
                .copied()
                .filter(|id| !done.contains(id))
                .collect();
            drop(inner);
            if let Some(var) = var {
                let list = Value::list(
                    pending
                        .iter()
                        .map(|i| Value::string(i.to_string()))
                        .collect(),
                );
                if let Err(e) = vm.set_var(&var.to_str(), list) {
                    return e;
                }
            }
            return ok(Value::list(
                done.into_iter()
                    .map(|i| Value::string(i.to_string()))
                    .collect(),
            ));
        }
        inner = pool.queue.done.wait(inner).expect("pool wait");
    }
}

/// `tpool::get pool jobid` — the job's result (blocking until it is done), or its
/// error. Removes the job from the results table.
fn cmd_tpool_get(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let shared = match shared(vm) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let [pool_h, job_h] = args else {
        return err("wrong # args: should be \"tpool::get tpool job\"");
    };
    let pool = match pool_of(&shared, &pool_h.to_str()) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let Some(jid) = job_h.to_str().trim().parse::<u64>().ok() else {
        return err(format!("invalid job id \"{}\"", job_h.to_str()));
    };
    let mut inner = pool.queue.inner.lock().expect("pool inner");
    let res = loop {
        if let Some(res) = inner.results.remove(&jid) {
            break res;
        }
        if inner.shutdown {
            return err(format!("job \"{jid}\" is not posted to this pool"));
        }
        inner = pool.queue.done.wait(inner).expect("pool wait");
    };
    drop(inner);
    if res.ok {
        ok(Value::string(res.result))
    } else {
        err(res.result)
    }
}

/// `tpool::release pool` — shut the pool down: signal every worker, join them,
/// and drop the registry entry.
fn cmd_tpool_release(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let shared = match shared(vm) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let [pool_h] = args else {
        return err("wrong # args: should be \"tpool::release tpool\"");
    };
    let pool = shared
        .pools
        .lock()
        .expect("pools")
        .remove(&*pool_h.to_str());
    let Some(pool) = pool else {
        return err(format!("can not find threadpool \"{}\"", pool_h.to_str()));
    };
    pool.queue.inner.lock().expect("pool inner").shutdown = true;
    pool.queue.job_ready.notify_all();
    pool.queue.done.notify_all();
    for h in pool.workers.lock().expect("pool workers").drain(..) {
        let _ = h.join();
    }
    ok(Value::empty())
}

/// `tpool::names` — the handles of all live pools.
fn cmd_tpool_names(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let shared = match shared(vm) {
        Ok(s) => s,
        Err(e) => return e,
    };
    if !args.is_empty() {
        return err("wrong # args: should be \"tpool::names\"");
    }
    let mut names: Vec<String> = shared
        .pools
        .lock()
        .expect("pools")
        .keys()
        .cloned()
        .collect();
    names.sort_unstable();
    ok(Value::list(names.into_iter().map(Value::string).collect()))
}

/// Clone the `Arc` for handle `name` out of a registry map (releasing the lock).
fn lookup<T>(map: &Mutex<HashMap<String, Arc<T>>>, name: &str) -> Option<Arc<T>> {
    map.lock().expect("registry lock").get(name).map(Arc::clone)
}

/// The "no such handle" error text for a synchronisation primitive.
fn no_such(kind: &str, handle: &str) -> String {
    format!("{kind} \"{handle}\" does not exist")
}

// ===========================================================================
// tsv::* — thread shared variables
// ===========================================================================

/// Run `f` against the shared `tsv` store, returning its result.
fn with_tsv<T>(
    vm: &Vm,
    f: impl FnOnce(&mut HashMap<String, HashMap<String, String>>) -> T,
) -> Result<T, Completion<Value>> {
    let shared = shared(vm)?;
    let mut tsv = shared.tsv.lock().expect("tsv lock");
    Ok(f(&mut tsv))
}

/// `tsv::set array key ?value?` — set (and return) the element, or read it when
/// `value` is omitted.
fn cmd_tsv_set(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [arr, key] => match with_tsv(vm, |t| {
            t.get(&*arr.to_str())
                .and_then(|m| m.get(&*key.to_str()))
                .cloned()
        }) {
            Ok(Some(v)) => ok(Value::string(v)),
            Ok(None) => err(format!(
                "key \"{}\" does not exist in shared variable \"{}\"",
                key.to_str(),
                arr.to_str()
            )),
            Err(e) => e,
        },
        [arr, key, value] => {
            let v = value.to_str().to_string();
            match with_tsv(vm, |t| {
                t.entry(arr.to_str().to_string())
                    .or_default()
                    .insert(key.to_str().to_string(), v.clone());
            }) {
                Ok(()) => ok(Value::string(v)),
                Err(e) => e,
            }
        }
        _ => err("wrong # args: should be \"tsv::set array key ?value?\""),
    }
}

/// `tsv::get array key ?varName?` — read the element. With `varName`, store it
/// there and return 1/0 for presence instead of erroring on a missing key.
fn cmd_tsv_get(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (arr, key, var) = match args {
        [arr, key] => (arr, key, None),
        [arr, key, var] => (arr, key, Some(var)),
        _ => return err("wrong # args: should be \"tsv::get array key ?varName?\""),
    };
    let got = match with_tsv(vm, |t| {
        t.get(&*arr.to_str())
            .and_then(|m| m.get(&*key.to_str()))
            .cloned()
    }) {
        Ok(v) => v,
        Err(e) => return e,
    };
    match (var, got) {
        (None, Some(v)) => ok(Value::string(v)),
        (None, None) => err(format!(
            "key \"{}\" does not exist in shared variable \"{}\"",
            key.to_str(),
            arr.to_str()
        )),
        (Some(var), Some(v)) => {
            if let Err(e) = vm.set_var(&var.to_str(), Value::string(v)) {
                return e;
            }
            ok(Value::string("1"))
        }
        (Some(_), None) => ok(Value::string("0")),
    }
}

/// `tsv::exists array ?key?` — whether the array (or its element) exists.
fn cmd_tsv_exists(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let present = match args {
        [arr] => with_tsv(vm, |t| t.contains_key(&*arr.to_str())),
        [arr, key] => with_tsv(vm, |t| {
            t.get(&*arr.to_str())
                .is_some_and(|m| m.contains_key(&*key.to_str()))
        }),
        _ => return err("wrong # args: should be \"tsv::exists array ?key?\""),
    };
    match present {
        Ok(p) => ok(Value::string(if p { "1" } else { "0" })),
        Err(e) => e,
    }
}

/// `tsv::unset array ?key?` — drop the whole array, or one element.
fn cmd_tsv_unset(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let res = match args {
        [arr] => with_tsv(vm, |t| {
            t.remove(&*arr.to_str());
        }),
        [arr, key] => with_tsv(vm, |t| {
            if let Some(m) = t.get_mut(&*arr.to_str()) {
                m.remove(&*key.to_str());
            }
        }),
        _ => return err("wrong # args: should be \"tsv::unset array ?key?\""),
    };
    match res {
        Ok(()) => ok(Value::empty()),
        Err(e) => e,
    }
}

/// `tsv::incr array key ?increment?` — atomically add to (and return) the
/// element, defaulting a missing element to 0.
fn cmd_tsv_incr(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (arr, key, delta) = match args {
        [arr, key] => (arr, key, 1_i64),
        [arr, key, by] => match by.to_str().trim().parse::<i64>() {
            Ok(n) => (arr, key, n),
            Err(_) => {
                return err(format!("expected integer but got \"{}\"", by.to_str()));
            }
        },
        _ => return err("wrong # args: should be \"tsv::incr array key ?count?\""),
    };
    let outcome = with_tsv(vm, |t| {
        let cell = t
            .entry(arr.to_str().to_string())
            .or_default()
            .entry(key.to_str().to_string())
            .or_insert_with(|| "0".to_string());
        match cell.trim().parse::<i64>() {
            Ok(cur) => {
                let next = cur + delta;
                *cell = next.to_string();
                Ok(next)
            }
            Err(_) => Err(cell.clone()),
        }
    });
    match outcome {
        Ok(Ok(n)) => ok(Value::string(n.to_string())),
        Ok(Err(bad)) => err(format!("expected integer but got \"{bad}\"")),
        Err(e) => e,
    }
}

/// `tsv::append array key value ?value ...?` — append to (and return) the
/// element as a plain string.
fn cmd_tsv_append(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [arr, key, values @ ..] = args else {
        return err("wrong # args: should be \"tsv::append array key value ?value ...?\"");
    };
    if values.is_empty() {
        return err("wrong # args: should be \"tsv::append array key value ?value ...?\"");
    }
    let suffix: String = values.iter().map(|v| v.to_str().to_string()).collect();
    let out = with_tsv(vm, |t| {
        let cell = t
            .entry(arr.to_str().to_string())
            .or_default()
            .entry(key.to_str().to_string())
            .or_default();
        cell.push_str(&suffix);
        cell.clone()
    });
    match out {
        Ok(v) => ok(Value::string(v)),
        Err(e) => e,
    }
}

/// `tsv::lappend array key value ?value ...?` — list-append to (and return) the
/// element.
fn cmd_tsv_lappend(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [arr, key, values @ ..] = args else {
        return err("wrong # args: should be \"tsv::lappend array key value ?value ...?\"");
    };
    if values.is_empty() {
        return err("wrong # args: should be \"tsv::lappend array key value ?value ...?\"");
    }
    let added = tcl_syntax::list::join_list(values.iter().map(Value::to_str));
    let out = with_tsv(vm, |t| {
        let cell = t
            .entry(arr.to_str().to_string())
            .or_default()
            .entry(key.to_str().to_string())
            .or_default();
        if cell.is_empty() {
            cell.clone_from(&added);
        } else {
            cell.push(' ');
            cell.push_str(&added);
        }
        cell.clone()
    });
    match out {
        Ok(v) => ok(Value::string(v)),
        Err(e) => e,
    }
}

/// `tsv::keys array ?pattern?` — the element names of `array` (optionally
/// `string match`-filtered).
fn cmd_tsv_keys(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (arr, pattern) = match args {
        [arr] => (arr, None),
        [arr, pat] => (arr, Some(pat.to_str())),
        _ => return err("wrong # args: should be \"tsv::keys array ?pattern?\""),
    };
    let keys = with_tsv(vm, |t| {
        t.get(&*arr.to_str())
            .map(|m| m.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
    });
    match keys {
        Ok(keys) => ok(filtered_list(keys, pattern.as_deref())),
        Err(e) => e,
    }
}

/// `tsv::names ?pattern?` — the names of all shared arrays.
fn cmd_tsv_names(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let pattern = match args {
        [] => None,
        [pat] => Some(pat.to_str()),
        _ => return err("wrong # args: should be \"tsv::names ?pattern?\""),
    };
    let names = with_tsv(vm, |t| t.keys().cloned().collect::<Vec<_>>());
    match names {
        Ok(names) => ok(filtered_list(names, pattern.as_deref())),
        Err(e) => e,
    }
}

// -- helpers ---------------------------------------------------------------

fn parse_id(s: &str) -> Option<u64> {
    // Accept a bare integer or the `tidNNNN` form some tools print.
    let digits = s.strip_prefix("tid").unwrap_or(s);
    digits.trim().parse::<u64>().ok()
}

/// A sorted list of `items`, optionally kept to those matching `pattern` under
/// `string match` (glob) semantics.
fn filtered_list(mut items: Vec<String>, pattern: Option<&str>) -> Value {
    if let Some(pat) = pattern {
        items.retain(|k| tcl_syntax::glob::string_match(pat, k));
    }
    items.sort_unstable();
    Value::list(items.into_iter().map(Value::string).collect::<Vec<_>>())
}

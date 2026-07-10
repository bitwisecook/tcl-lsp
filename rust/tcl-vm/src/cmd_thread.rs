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

use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

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
    /// Live workers by thread id.
    workers: Mutex<HashMap<u64, Worker>>,
    /// The `tsv` store: array name → key → serialized value.
    tsv: Mutex<HashMap<String, HashMap<String, String>>>,
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
            workers: Mutex::new(HashMap::new()),
            tsv: Mutex::new(HashMap::new()),
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

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

//! Shared end-to-end harness for the native `tcl-lsp-server` binary.
//!
//! Each test spawns the real `tcl-lsp-server` binary via nextest's archived
//! runtime path, with `CARGO_BIN_EXE_tcl-lsp-server` as the ordinary Cargo
//! fallback, talks LSP JSON-RPC to it over stdio, and asserts on the responses
//! — exactly what an editor does. A background reader thread parses framed
//! messages, routing responses to blocked requests, buffering notifications
//! (with a condvar so `await_*` can wait), and auto-answering server-initiated
//! requests (`workspace/configuration`) so the server never blocks.
//!
//! The client offers a request / notify / `open_ready` / `await_diagnostics` /
//! `await_log` contract, XDG isolation per server, and a per-section reply to
//! `workspace/configuration`.
//!
//! Not every helper is used by every test file, so `#![allow(dead_code)]` at the
//! module level keeps unused-in-this-binary helpers from warning (each
//! integration-test binary compiles this module independently).
//!
//! # Wall-clock barriers are scaled by measured capacity, never widened
//!
//! Every `await_*` / `request_timeout` deadline here is a *hang backstop*, not
//! an assertion: the thing being asserted is content (tokens match a cold
//! reopen, diagnostics carry the right codes), and the barrier only exists so a
//! genuinely wedged server fails instead of blocking forever. A fixed
//! wall-clock backstop is therefore wrong in one direction only — on a machine
//! the OS is giving a fraction of a core, a correct server misses it and a
//! *content* test fails as a *timeout*.
//!
//! So the backstops are multiplied by [`load_factor`], a measured probe of how
//! much capacity this process is actually getting (see its docs). On an
//! unloaded machine the factor is `1.0` and every deadline is exactly what it
//! reads in the source; under contention it grows with the contention, so the
//! barrier keeps guarding against hangs without turning scheduling delay into a
//! false failure. This is deliberately *not* the same thing as raising the
//! constants: the constants stay honest, and a quiet machine keeps the tight
//! bound.
//!
//! Genuine latency *guarantees* (the fast-tier promises) are a
//! different matter and use [`LatencyBudget`], which additionally measures the
//! server's own no-op round-trip so the guarantee is expressed relative to the
//! machine's demonstrated capacity rather than a wall-clock absolute.

#![allow(dead_code)]

pub mod helpers;

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

mod configuration;
mod diagnostics_barriers;
mod documents;
mod feature_requests;
mod latency_scaling;
mod notifications;
mod reader;
mod startup;

#[cfg(test)]
use configuration::config_reflected;
#[cfg(test)]
use latency_scaling::BARRIER_TIMEOUT_MARKER;
use latency_scaling::latency_barrier_timeout_note;
pub use latency_scaling::{LatencyBudget, load_factor, scaled_timeout};
use reader::read_loop;

/// Default per-request timeout.
/// LSP `MessageType.Warning`, as a `window/logMessage` carries it.
const LOG_MESSAGE_WARNING: i64 = 2;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
/// Longer default for `initialize` / `request` without an explicit deadline.
const REQUEST_TIMEOUT: Duration = Duration::from_mins(1);

/// Process-wide counter so `unique_uri` never collides across tests in one
/// integration-test binary (each binary is its own process).
static URI_COUNTER: AtomicU64 = AtomicU64::new(0);

/// The workspace root every default server is initialised with: one real,
/// empty directory per test process. The server's scan of it completes, so
/// workspace-gated edits are judged against a complete inventory, and every
/// [`unique_uri`] document lies inside it as an editor's documents would.
pub fn harness_root_uri() -> &'static str {
    static ROOT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("tcl-lsp-e2e-root-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create the e2e workspace root");
        tcl_lsp_server::canonical_file_uri(&dir)
            .expect("the e2e workspace root has a file URI")
            .to_string()
            .trim_end_matches('/')
            .to_owned()
    })
}

/// A fresh, unique `file://` URI inside [`harness_root_uri`]. Version-tagged
/// diagnostics never collide because the path is unique per call.
pub fn unique_uri(suffix: &str) -> String {
    let n = URI_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}/{}_{n}.{suffix}", harness_root_uri(), std::process::id())
}

/// A reproducible `xorshift64*` PRNG — the same generator the `tcl-fuzz` crate
/// uses (`rust/tcl-fuzz/src/rng.rs`), so seeded stress tests stay deterministic
/// without pulling the `rand` crate into the workspace. Identical seeds yield
/// identical streams.
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Seed the generator. A zero seed is remapped (xorshift needs non-zero state).
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    /// Next raw 64-bit value.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform float in `[0, 1)` (53-bit mantissa), like `random.random()`.
    pub fn random(&mut self) -> f64 {
        // Split the 53-bit mantissa into two `u32` halves — both exact in
        // `f64` via `f64::from` — so no lossy `u64 as f64` cast is needed.
        let mantissa = self.next_u64() >> 11;
        let high = u32::try_from(mantissa >> 32).expect("21-bit high half");
        let low = u32::try_from(mantissa & 0xFFFF_FFFF).expect("32-bit low half");
        (f64::from(high) * 4_294_967_296.0 + f64::from(low)) / 9_007_199_254_740_992.0
    }

    /// Inclusive integer in `[lo, hi]`, like `random.randint(lo, hi)`.
    pub fn randint(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            return lo;
        }
        lo + usize::try_from(self.next_u64()).unwrap() % (hi - lo + 1)
    }

    /// Pick a reference to a random element of a non-empty slice, like
    /// `random.choice(seq)`.
    pub fn choice<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[usize::try_from(self.next_u64()).unwrap() % items.len()]
    }
}

/// State shared between the harness and its background reader thread.
struct Shared {
    /// The child's stdin write half — shared so both the client (requests) and
    /// the reader thread (auto-replies to server requests) can write frames.
    stdin: Mutex<ChildStdin>,
    /// Responses keyed by request id, awaiting collection.
    responses: Mutex<HashMap<i64, Value>>,
    /// Buffered notifications, guarded by `notify_cv`.
    notifications: Mutex<Vec<Value>>,
    /// Server-initiated *requests* (have an `id`, unlike notifications) —
    /// e.g. `workspace/semanticTokens/refresh`, `workspace/diagnostic/refresh`,
    /// `codeLens/refresh`. Captured separately from `notifications` (which only
    /// holds id-less messages) so a test can assert the server actually asked
    /// for a refresh, in addition to `auto_reply` answering it so the server
    /// never blocks.
    server_requests: Mutex<Vec<Value>>,
    /// Wakes waiters on `notifications`.
    ///
    /// A `Condvar` is bound to exactly one `Mutex` for its lifetime: waiting on
    /// the same one with two different mutexes is a std-documented misuse that
    /// panics with "attempted to use a condition variable with two mutexes"
    /// (detected by the pthread backend on macOS; the futex backend used on
    /// Linux does not notice, so CI never saw it). `server_requests` therefore
    /// has its own `requests_cv` rather than sharing this one.
    notify_cv: Condvar,
    /// Wakes waiters on `server_requests`. See `notify_cv`.
    requests_cv: Condvar,
    /// The `tclLsp` configuration reply for `workspace/configuration`. Mutable
    /// so `apply_configuration` can change what the server re-pulls.
    tcllsp_config: Mutex<Value>,
    /// Per-folder `tclLsp` replies, keyed by the `scopeUri` the server asks
    /// with (trailing slash trimmed). This is what a real multi-root editor
    /// does: the *unscoped* pull returns the workspace-merged settings, and a
    /// scoped pull returns that folder's resolved settings — which is the only
    /// way a folder-level `tclLsp.dialect` ever reaches the server. A scope with
    /// no entry falls back to `tcllsp_config`, so single-root tests are
    /// unaffected.
    folder_configs: Mutex<HashMap<String, Value>>,
    /// Test-controlled delay before replying to `workspace/configuration`.
    /// Zero in every ordinary test; the transport-liveness regression uses a
    /// short delay to put four handlers in the reply-waiting state at once.
    configuration_reply_delay: Mutex<Duration>,
    /// The action title this client picks when the server sends
    /// `window/showMessageRequest`; `None` answers as a user who dismisses the
    /// message without choosing.
    message_action_reply: Mutex<Option<String>>,
    /// Captured stderr text.
    stderr: Mutex<String>,
}

/// A live language-server subprocess plus an LSP JSON-RPC client.
pub struct Lsp {
    child: Child,
    shared: Arc<Shared>,
    next_id: i64,
    /// URIs opened without a matching close, so `Drop` can tidy up.
    open_uris: Vec<String>,
    xdg_root: std::path::PathBuf,
    /// The client capabilities sent at `initialize`; none unless a test says.
    client_capabilities: Value,
    /// The `initialize` result, populated by [`Lsp::initialize`].
    initialize_result: Value,
}

/// The server's test seam that holds the startup pack reload.
const STARTUP_RELOAD_HOLD_ENV: &str = "TCL_LSP_TEST_STARTUP_RELOAD_HOLD_MS";

impl Lsp {
    /// Spawn the server **without** the `initialize` handshake, so a caller can
    /// drive `initialize` itself — e.g. with a deliberately malformed folder
    /// URI.
    pub fn spawn(config: Value) -> Self {
        Self::spawn_with_env(config, &[])
    }

    /// [`Lsp::spawn`] with extra environment variables set on the server
    /// process.
    ///
    /// For the server's *test seams* — the `TCL_LSP_TEST_*` knobs that exist
    /// so an e2e test can force an interleaving it cannot otherwise provoke
    /// (`TCL_LSP_TEST_STARTUP_RELOAD_HOLD_MS`). It has to be the child's
    /// environment rather than this process's: the server is a separate
    /// binary, and `std::env::set_var` here would leak into every other test
    /// sharing the runner process.
    pub fn spawn_with_env(config: Value, env: &[(&str, &str)]) -> Self {
        // Archived nextest runs relocate the executable beside the extracted
        // test binaries. Prefer the runtime path there; ordinary Cargo test
        // keeps using its compile-time path as the fallback.
        let bin = std::env::var_os("NEXTEST_BIN_EXE_tcl-lsp-server")
            .unwrap_or_else(|| env!("CARGO_BIN_EXE_tcl-lsp-server").into());

        // Isolate the server from the developer machine's config/cache so a
        // local `~/.../tcl-lsp/config.ini` can't poison the defaults. Fresh
        // empty XDG dirs make the server fall back to built-in defaults.
        let xdg_root =
            std::env::temp_dir().join(format!("tcl-lsp-e2e-xdg-{}-{}", std::process::id(), {
                URI_COUNTER.fetch_add(1, Ordering::Relaxed)
            }));
        std::fs::create_dir_all(xdg_root.join("config")).expect("mk xdg config");
        std::fs::create_dir_all(xdg_root.join("cache")).expect("mk xdg cache");
        std::fs::create_dir_all(xdg_root.join("state")).expect("mk xdg state");

        // The isolated XDG directories come first, so a test that needs a
        // directory to outlive one server (the state a second session reads)
        // can name its own through `env`.
        let mut command = Command::new(bin);
        command
            .env("XDG_CONFIG_HOME", xdg_root.join("config"))
            .env("XDG_CACHE_HOME", xdg_root.join("cache"))
            .env("XDG_STATE_HOME", xdg_root.join("state"));
        for (key, value) in env {
            command.env(key, value);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn tcl-lsp-server");

        let stdin = child.stdin.take().expect("child stdin");
        let stdout = child.stdout.take().expect("child stdout");
        let stderr = child.stderr.take().expect("child stderr");

        let shared = Arc::new(Shared {
            stdin: Mutex::new(stdin),
            responses: Mutex::new(HashMap::new()),
            notifications: Mutex::new(Vec::new()),
            server_requests: Mutex::new(Vec::new()),
            notify_cv: Condvar::new(),
            requests_cv: Condvar::new(),
            tcllsp_config: Mutex::new(config),
            folder_configs: Mutex::new(HashMap::new()),
            configuration_reply_delay: Mutex::new(Duration::ZERO),
            message_action_reply: Mutex::new(None),
            stderr: Mutex::new(String::new()),
        });

        // Reader thread: parse frames, route messages.
        {
            let shared = Arc::clone(&shared);
            std::thread::spawn(move || read_loop(stdout, &shared));
        }
        // Stderr drain.
        {
            let shared = Arc::clone(&shared);
            std::thread::spawn(move || {
                let mut reader = BufReader::new(stderr);
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.read_line(&mut line) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => shared.stderr.lock().unwrap().push_str(&line),
                    }
                }
            });
        }

        Self {
            child,
            shared,
            next_id: 0,
            open_uris: Vec::new(),
            xdg_root,
            client_capabilities: json!({}),
            initialize_result: Value::Null,
        }
    }

    // Requests / notifications.

    /// Send a request and return its result, panicking on error or timeout.
    pub fn request(&mut self, method: &str, params: Value) -> Value {
        self.request_timeout(method, params, REQUEST_TIMEOUT)
    }

    /// Like [`Lsp::request`] with an explicit timeout.
    pub fn request_timeout(&mut self, method: &str, params: Value, timeout: Duration) -> Value {
        let resp = self.request_response(method, params, timeout);
        if let Some(err) = resp.get("error") {
            panic!("{method} -> error {err}");
        }
        resp.get("result").cloned().unwrap_or(Value::Null)
    }

    /// Send one request under an existing absolute deadline. Settled helpers
    /// use this so each request, completion marker and required refresh shares
    /// one load-scaled backstop.
    fn request_before(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
        deadline: Instant,
    ) -> Value {
        let id = self.send_request_no_wait(method, params);
        let response = self.await_response_before(id, method, timeout, deadline);
        assert!(
            Instant::now() < deadline,
            "timed out after {timeout:?} waiting for response to {method:?}; stderr:\n{}{}",
            self.stderr_text(),
            latency_barrier_timeout_note(),
        );
        if let Some(err) = response.get("error") {
            panic!("{method} -> error {err}");
        }
        response.get("result").cloned().unwrap_or(Value::Null)
    }

    /// The whole JSON-RPC response object, `error` included.
    ///
    /// [`Lsp::request`] panics on an error response, which is right for every
    /// request that must succeed — but the rename **safety gate** answers a
    /// refusal *as* an error (a `null` result would read as "nothing to
    /// rename here"), so its tests need the error itself.
    pub fn request_response(&mut self, method: &str, params: Value, timeout: Duration) -> Value {
        let id = self.send_request_no_wait(method, params);
        self.await_response(id, method, timeout)
    }

    /// Send a request without waiting for its response, returning its id.
    ///
    /// This models an editor's concurrent request burst. Most tests should use
    /// [`Self::request`]; transport tests deliberately need more than the
    /// server's internal queue in flight before they wait.
    pub fn send_request_no_wait(&mut self, method: &str, params: Value) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut msg = json!({ "jsonrpc": "2.0", "id": id, "method": method });
        msg["params"] = params;
        self.send(&msg);
        id
    }

    /// Wait for a previously-sent request id.
    pub fn await_response(&self, id: i64, method: &str, timeout: Duration) -> Value {
        let deadline = Instant::now() + scaled_timeout(timeout);
        self.await_response_before(id, method, timeout, deadline)
    }

    /// Reuse one actual response owner without restarting a caller's deadline.
    fn await_response_before(
        &self,
        id: i64,
        method: &str,
        timeout: Duration,
        deadline: Instant,
    ) -> Value {
        loop {
            {
                let mut responses = self.shared.responses.lock().unwrap();
                if let Some(resp) = responses.remove(&id) {
                    return resp;
                }
            }
            assert!(
                Instant::now() < deadline,
                "timed out after {timeout:?} waiting for response to {method:?}; stderr:\n{}{}",
                self.stderr_text(),
                latency_barrier_timeout_note(),
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    /// Send a notification (no response expected).
    pub fn notify(&mut self, method: &str, params: Value) {
        let mut msg = json!({ "jsonrpc": "2.0", "method": method });
        msg["params"] = params;
        self.send(&msg);
    }

    fn send(&mut self, payload: &Value) {
        let body = serde_json::to_vec(payload).expect("serialise JSON-RPC");
        let mut stdin = self.shared.stdin.lock().unwrap();
        write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("write header");
        stdin.write_all(&body).expect("write body");
        stdin.flush().expect("flush");
    }

    /// Poll an arbitrary request (`query`) until its response satisfies
    /// `predicate`, or `timeout` (load-scaled) elapses; returns the last
    /// response either way, so a genuine content divergence is reported by
    /// the caller's own assertion rather than swallowed here.
    ///
    /// # The race this closes
    ///
    /// [`Self::open_ready`] only waits for the *opened* document's own
    /// diagnostics to settle. It says nothing about `scan_workspace_folders`,
    /// the server's independent background pass that walks the workspace
    /// root and indexes every on-disk file the client has **not** opened, so
    /// cross-file providers (`textDocument/definition`, `references`,
    /// `rename`, call hierarchy, `workspace/symbol`) can see them. A test
    /// that writes a sibling fixture straight to disk, never opens it, opens
    /// only the *other* file with `open_ready`, and then fires a single
    /// cross-file query immediately is racing that scan: on a quiet box the
    /// scan usually wins and the sibling is already indexed, but on a loaded
    /// CI runner it can lose — the server has not indexed the sibling yet, so
    /// it correctly answers with what it currently knows (empty/short), not
    /// a bug in either the server or the query. `wait_for_workspace_scan`
    /// gates a few *internal* server code paths (autoload resolution,
    /// `workspace/symbol`) but the position-based providers above are
    /// deliberately not among them, so the settling has to happen here, the
    /// same way [`Self::await_diagnostics_settled`] settles a diagnostics
    /// push that is refined progressively rather than gating the publish
    /// itself.
    ///
    /// `query` is a live round-trip per call, so it must be cheap and
    /// side-effect-free beyond the request itself (a plain `textDocument/*`
    /// or `workspace/*` read). Sleeps a short, fixed step between attempts —
    /// deliberately not load-scaled itself, only the deadline is, so a slow
    /// machine gets more attempts rather than coarser ones.
    pub fn await_query_settled<T: std::fmt::Display>(
        &mut self,
        timeout: Duration,
        mut query: impl FnMut(&mut Self) -> T,
        predicate: impl Fn(&T) -> bool,
    ) -> T {
        let deadline = Instant::now() + scaled_timeout(timeout);
        loop {
            let result = query(self);
            if predicate(&result) {
                return result;
            }
            assert!(
                Instant::now() < deadline,
                "query never reached the expected state within {timeout:?} \
                 (load-scaled); last response: {result}{}",
                latency_barrier_timeout_note()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Lsp {
    fn drop(&mut self) {
        // Best-effort graceful shutdown, then ensure the child is reaped.
        let shutdown = json!({ "jsonrpc": "2.0", "id": -1, "method": "shutdown", "params": null });
        self.send(&shutdown);
        let exit = json!({ "jsonrpc": "2.0", "method": "exit", "params": null });
        self.send(&exit);
        // Give it a moment, then kill unconditionally.
        std::thread::sleep(Duration::from_millis(50));
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.xdg_root);
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod pack_configuration_settle_tests;

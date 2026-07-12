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
//! The native port of the former `tests/lsp_e2e/` pytest suite. Each test
//! spawns the real `tcl-lsp-server` binary (via `CARGO_BIN_EXE_tcl-lsp-server`),
//! talks LSP JSON-RPC to it over stdio, and asserts on the responses — exactly
//! what an editor does. A background reader thread parses framed messages,
//! routing responses to blocked requests, buffering notifications (with a
//! condvar so `await_*` can wait), and auto-answering server-initiated requests
//! (`workspace/configuration`) so the server never blocks.
//!
//! This mirrors `tests/lsp_e2e/harness.py`'s `LspServerClient`: same request /
//! notify / `open_ready` / `await_diagnostics` / `await_log` contract, same XDG
//! isolation, same per-section configuration reply.
//!
//! Not every helper is used by every test file, so `#![allow(dead_code)]` at the
//! module level keeps unused-in-this-binary helpers from warning (each
//! integration-test binary compiles this module independently).

#![allow(dead_code)]

pub mod helpers;

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// Default per-request timeout, matching the pytest harness (`timeout=30.0`).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
/// Longer default for `initialize` / `request` without an explicit deadline.
const REQUEST_TIMEOUT: Duration = Duration::from_mins(1);

/// A greppable marker every latency-barrier timeout carries, so a CI-log scan
/// (human or tooling) can classify the failure without reading the prose.
const BARRIER_TIMEOUT_MARKER: &str = "LATENCY-BARRIER-TIMEOUT";

/// Diagnostic footer appended to every latency-barrier timeout panic in this
/// harness (`request_timeout`, `await_diagnostics_version`, `await_log`,
/// `await_notification`, and the config-settle barrier).
///
/// A barrier timeout means the server did not *respond* within the deadline — a
/// latency failure, categorically distinct from an oracle/content divergence
/// (those panic with "diverged" / "alignment broke" instead). On CI the
/// dominant cause is the test process being denied CPU on an oversubscribed
/// runner, not a server defect: the latency-sensitive e2e stress tests
/// (`edit_tracking_stress::*` and the semantic-token latency benchmark) can
/// starve each other, which is exactly why `.config/nextest.toml` isolates them
/// into a single-slot `heavy-lsp-e2e` group and the dedicated `lsp-e2e` CI job
/// runs them that way and stays green.
///
/// Rather than leave every investigator to re-derive that from a bare "timed
/// out" message, we *probe scheduling health at the moment of giving up*: a
/// short sleep the OS honours many-fold late is direct, in-log evidence that
/// the runner is starving this process, so the timeout is a scheduling artefact
/// rather than a hang. If the probe finds the scheduler healthy it says so,
/// redirecting the investigation toward a real server-latency regression.
fn latency_barrier_timeout_note() -> String {
    // Sample how late the scheduler wakes us from a short, known sleep. On an
    // unloaded machine `actual ≈ target` (a few percent over, from timer
    // granularity); under heavy oversubscription the wake is delayed many-fold.
    // Median of a handful of samples so one unlucky context switch can't skew
    // the verdict. This runs only on the already-failing path, so its ~100 ms
    // cost is irrelevant.
    const TARGET: Duration = Duration::from_millis(20);
    const SAMPLES: usize = 5;
    let mut ratios = [0f64; SAMPLES];
    for r in &mut ratios {
        let t = Instant::now();
        std::thread::sleep(TARGET);
        *r = t.elapsed().as_secs_f64() / TARGET.as_secs_f64();
    }
    ratios.sort_by(|a, b| a.partial_cmp(b).expect("sleep ratios are finite"));
    let median = ratios[SAMPLES / 2];

    // >=2.5x late is far outside timer-granularity noise and only happens when
    // the OS cannot schedule this thread promptly — i.e. the runner is starved.
    let verdict = if median >= 2.5 {
        format!(
            "PROBE: CPU STARVATION CONFIRMED — a {TARGET:?} sleep is waking {median:.1}x late \
             right now, so the OS is denying this test process CPU. The barrier above expired on \
             scheduling delay, NOT a server hang or a buffer-tracking bug. This is the known \
             flake: re-run the job."
        )
    } else {
        format!(
            "PROBE: could not confirm starvation — a {TARGET:?} sleep woke {median:.1}x late, \
             within normal timer noise. The runner looks healthy *now*; if this repeats with a \
             healthy probe, suspect a real server-latency regression (a hang/deadlock or a \
             genuinely slow analysis), not CPU starvation."
        )
    };

    format!(
        "\n\n{BARRIER_TIMEOUT_MARKER} — this is a TIMEOUT (the server did not respond in time), \
         NOT an oracle/content divergence (those fail with \"diverged\" / \"alignment broke\").\n\
         {verdict}\n\
         CONTEXT: the latency-sensitive e2e stress tests are isolated into the single-slot \
         `heavy-lsp-e2e` nextest group (.config/nextest.toml) so they never starve each other; \
         the dedicated `lsp-e2e` CI job runs them isolated and stays green. A timeout seen only in \
         the *unisolated* `rust-tests` job (which runs them amid the full parallel suite) is the \
         known CPU-starvation flake."
    )
}

/// Process-wide counter so `unique_uri` never collides across tests in one
/// integration-test binary (each binary is its own process).
static URI_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A fresh, unique `file://` URI — the native analogue of pytest's
/// `uri_factory`. Version-tagged diagnostics never collide because the path is
/// unique per call.
pub fn unique_uri(suffix: &str) -> String {
    let n = URI_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("file:///e2e/{}_{n}.{suffix}", std::process::id())
}

/// A reproducible `xorshift64*` PRNG — the same generator the `tcl-fuzz` crate
/// uses (`rust/tcl-fuzz/src/rng.rs`), so seeded stress tests stay deterministic
/// without pulling the `rand` crate into the workspace. Identical seeds yield
/// identical streams. The `random`/`randint`/`choice` surface mirrors Python's
/// `random.Random`, easing ports of seeded pytest fixtures.
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
    /// The `initialize` result, populated by [`Lsp::initialize`].
    initialize_result: Value,
}

impl Lsp {
    /// Spawn + initialise a server whose `workspace/configuration` reply enables
    /// only linked editing (the default-editor contract the pytest `lsp_server`
    /// fixture pins).
    pub fn tcl() -> Self {
        Self::with_config(json!({ "features": { "linkedEditingRange": true } }))
    }

    /// A server dedicated to iRules-dialect documents (dialect switch is
    /// process-global, so those tests use their own server). Same config as
    /// [`Lsp::tcl`].
    pub fn irules() -> Self {
        Self::tcl()
    }

    /// A server with inlay hints opted in (default-off otherwise).
    pub fn inlay() -> Self {
        Self::with_config(json!({
            "features": {
                "linkedEditingRange": true,
                "inlayTypeHints": true,
                "inlayParameterHints": true,
            }
        }))
    }

    /// A server for BIG-IP config documents. Same config as [`Lsp::tcl`].
    pub fn bigip() -> Self {
        Self::tcl()
    }

    /// Spawn + initialise a server whose `tclLsp` configuration reply is
    /// `config`.
    /// Start a server whose `workspace/configuration` reply is `config`, and
    /// **block until that config has actually been applied**.
    ///
    /// The barrier is load-bearing, not belt-and-braces.  The server pulls the
    /// config from inside its `initialized` handler, which runs *concurrently*
    /// with any `didOpen` the client has already queued (see the comment on
    /// `Backend::initialized`).  Without this wait, a document opened right
    /// after construction races the pull and is routinely analysed under the
    /// *default* config — e.g. a test that opts into `optimiser.profile =
    /// standard` and then asserts on an O102 diagnostic gets an empty
    /// diagnostic set, because O102 is outside the default `readability`
    /// profile.  The server does converge (its `initialized` tail reschedules
    /// every open document), so the corrected publish arrives later — but a
    /// test that sampled the first publish has already failed by then.  That
    /// was a real, reproducible flake in `vscode_parity`'s two optimisation
    /// -payload tests.
    ///
    /// Settling here rather than in each test fixes the whole class: every
    /// `with_config` caller is now guaranteed that its config is in effect
    /// before it opens anything.
    pub fn with_config(config: Value) -> Self {
        let mut lsp = Self::spawn(config);
        lsp.initialize();
        // Settle on exactly what this client will reply to
        // `workspace/configuration` with, read back from the shared slot.
        let requested = lsp.shared.tcllsp_config.lock().unwrap().clone();
        lsp.settle_config(&requested);
        lsp
    }

    /// Poll `getEffectiveConfig` until every key of `requested` is reflected in
    /// the server's applied config.
    fn settle_config(&mut self, requested: &Value) {
        let deadline = Instant::now() + DEFAULT_TIMEOUT;
        loop {
            let effective = self.effective_config("");
            if config_reflected(requested, &effective) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "requested config was not applied within {DEFAULT_TIMEOUT:?}\n  \
                 requested: {requested}\n  effective: {effective}{}",
                latency_barrier_timeout_note()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn spawn(config: Value) -> Self {
        let bin = env!("CARGO_BIN_EXE_tcl-lsp-server");

        // Isolate the server from the developer machine's config/cache so a
        // local `~/.../tcl-lsp/config.ini` can't poison the defaults. Fresh
        // empty XDG dirs make the server fall back to built-in defaults.
        let xdg_root =
            std::env::temp_dir().join(format!("tcl-lsp-e2e-xdg-{}-{}", std::process::id(), {
                URI_COUNTER.fetch_add(1, Ordering::Relaxed)
            }));
        std::fs::create_dir_all(xdg_root.join("config")).expect("mk xdg config");
        std::fs::create_dir_all(xdg_root.join("cache")).expect("mk xdg cache");

        let mut child = Command::new(bin)
            .env("XDG_CONFIG_HOME", xdg_root.join("config"))
            .env("XDG_CACHE_HOME", xdg_root.join("cache"))
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
            initialize_result: Value::Null,
        }
    }

    // -- lifecycle --------------------------------------------------------

    /// Run the `initialize` handshake and send `initialized`.
    pub fn initialize(&mut self) -> Value {
        let root = format!("file:///e2e/root-{}", std::process::id());
        let result = self.request_timeout(
            "initialize",
            json!({
                "processId": std::process::id(),
                "rootUri": root,
                "workspaceFolders": [{ "uri": root, "name": "e2e" }],
                "capabilities": {},
                "clientInfo": { "name": "tcl-lsp-e2e", "version": "1.0" },
            }),
            REQUEST_TIMEOUT,
        );
        self.initialize_result = result.clone();
        self.notify("initialized", json!({}));
        result
    }

    /// The full `initialize` result.
    pub fn initialize_result(&self) -> &Value {
        &self.initialize_result
    }

    /// `serverInfo` from the `initialize` result, if present.
    pub fn server_info(&self) -> Option<&Value> {
        self.initialize_result.get("serverInfo")
    }

    // -- requests / notifications ----------------------------------------

    /// Send a request and return its result, panicking on error or timeout.
    pub fn request(&mut self, method: &str, params: Value) -> Value {
        self.request_timeout(method, params, REQUEST_TIMEOUT)
    }

    /// Like [`Lsp::request`] with an explicit timeout.
    pub fn request_timeout(&mut self, method: &str, params: Value, timeout: Duration) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let mut msg = json!({ "jsonrpc": "2.0", "id": id, "method": method });
        msg["params"] = params;
        self.send(&msg);

        let deadline = Instant::now() + timeout;
        loop {
            {
                let mut responses = self.shared.responses.lock().unwrap();
                if let Some(resp) = responses.remove(&id) {
                    if let Some(err) = resp.get("error") {
                        panic!("{method} -> error {err}");
                    }
                    return resp.get("result").cloned().unwrap_or(Value::Null);
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

    // -- document lifecycle ----------------------------------------------

    pub fn open_document(&mut self, uri: &str, text: &str) {
        self.open_document_lang(uri, text, "tcl", 1);
    }

    pub fn open_document_lang(&mut self, uri: &str, text: &str, language_id: &str, version: i64) {
        self.notify(
            "textDocument/didOpen",
            json!({ "textDocument": {
                "uri": uri, "languageId": language_id, "version": version, "text": text,
            }}),
        );
        self.open_uris.push(uri.to_owned());
    }

    /// Send a `didChange` with the raw `contentChanges` array.
    pub fn change_document(&mut self, uri: &str, version: i64, changes: Value) {
        let mut params = json!({ "textDocument": { "uri": uri, "version": version } });
        params["contentChanges"] = changes;
        self.notify("textDocument/didChange", params);
    }

    /// A full-text replace at `version`.
    pub fn replace_document(&mut self, uri: &str, version: i64, text: &str) {
        self.change_document(uri, version, json!([{ "text": text }]));
    }

    pub fn close_document(&mut self, uri: &str) {
        self.notify(
            "textDocument/didClose",
            json!({ "textDocument": { "uri": uri } }),
        );
        self.open_uris.retain(|u| u != uri);
    }

    /// Open `text` and block until its analysis snapshot is ready, returning the
    /// published diagnostics. Waits on both the version-tagged diagnostics and
    /// the per-URI `workspace_state.update` log line (see the pytest docstring).
    pub fn open_ready(&mut self, uri: &str, text: &str) -> Vec<Value> {
        self.open_ready_lang(uri, text, "tcl")
    }

    /// Like [`Lsp::open_ready`] but with an explicit `languageId` (e.g.
    /// `"tcl-irule"` for iRules, `"tk"` for Tk, `"bigip"` for BIG-IP config).
    pub fn open_ready_lang(&mut self, uri: &str, text: &str, language_id: &str) -> Vec<Value> {
        self.open_document_lang(uri, text, language_id, 1);
        let diags = self.await_diagnostics_version(uri, Some(1), DEFAULT_TIMEOUT);
        self.await_log(&["workspace_state.update", uri], DEFAULT_TIMEOUT, 0);
        diags
    }

    /// Force `uri`'s analysis snapshot to `text` at `version` and block until it
    /// is built. Returns `version`.
    ///
    /// The barrier is a **synchronous `textDocument/diagnostic` pull request**,
    /// not an await on a version-tagged `publishDiagnostics` push. That push is
    /// produced by the server's debounced, coalescing diagnostics scheduler, and
    /// a no-op full replace (`text` == the buffer's current content) is a salsa
    /// cache hit: the push tagged with this *exact* version can be legitimately
    /// coalesced away or delayed far past the timeout under CI load, so awaiting
    /// it tripped the barrier's 30s timeout intermittently — a flake, not an
    /// oracle divergence. The pull handler instead reads the *live* buffer and
    /// returns once diagnostics for the document's current revision are settled
    /// (served from the push cache when it already published that revision, else
    /// computed synchronously on demand). It is therefore a deterministic
    /// "analysis settled at the final content" signal the debounce/coalescer
    /// cannot suppress, with [`Lsp::request`]'s own timeout as the safety net.
    pub fn settle_analysis(&mut self, uri: &str, version: i64, text: &str) -> i64 {
        self.replace_document(uri, version, text);
        let _ = self.pull_diagnostics(uri);
        version
    }

    /// Issue a synchronous `textDocument/diagnostic` pull request for `uri` and
    /// return the reported diagnostics. The pull handler reads the *live* buffer
    /// and computes (or returns the already-settled) diagnostics for the current
    /// revision, so — unlike the debounced push channel — the response is a
    /// deterministic barrier: it only returns once the server has analysed the
    /// document's latest content, and cannot be dropped by the diagnostics
    /// debounce/coalescer.
    pub fn pull_diagnostics(&mut self, uri: &str) -> Vec<Value> {
        let report = self.request(
            "textDocument/diagnostic",
            json!({ "textDocument": { "uri": uri } }),
        );
        // A `full` report carries `items`; an `unchanged` report (only returned
        // when a matching `previousResultId` is replayed, which this never sends)
        // carries none.
        report
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    }

    // -- awaiting --------------------------------------------------------

    /// A marker into the notification log for `await_log(..., since)`.
    pub fn notification_cursor(&self) -> usize {
        self.shared.notifications.lock().unwrap().len()
    }

    /// Block until a `publishDiagnostics` for `uri` arrives; return its
    /// diagnostics array.
    pub fn await_diagnostics(&self, uri: &str) -> Vec<Value> {
        self.await_diagnostics_version(uri, None, DEFAULT_TIMEOUT)
    }

    /// Block until a `publishDiagnostics` for `uri` (optionally carrying exactly
    /// `version`) arrives; return the latest matching diagnostics array.
    pub fn await_diagnostics_version(
        &self,
        uri: &str,
        version: Option<i64>,
        timeout: Duration,
    ) -> Vec<Value> {
        let deadline = Instant::now() + timeout;
        let mut notes = self.shared.notifications.lock().unwrap();
        loop {
            let mut matched: Option<Vec<Value>> = None;
            for note in notes.iter() {
                if note.get("method").and_then(Value::as_str)
                    != Some("textDocument/publishDiagnostics")
                {
                    continue;
                }
                let params = note.get("params").cloned().unwrap_or(Value::Null);
                if params.get("uri").and_then(Value::as_str) != Some(uri) {
                    continue;
                }
                if let Some(v) = version
                    && params.get("version").and_then(Value::as_i64) != Some(v)
                {
                    continue;
                }
                matched = Some(
                    params
                        .get("diagnostics")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default(),
                );
            }
            if let Some(diags) = matched {
                return diags;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                // Release the notifications lock before the (sleeping) probe so
                // the reader thread isn't blocked while we diagnose the timeout.
                drop(notes);
                panic!(
                    "no publishDiagnostics for {uri:?} version {version:?} within {timeout:?}{}",
                    latency_barrier_timeout_note()
                );
            }
            let (guard, _) = self
                .shared
                .notify_cv
                .wait_timeout(notes, remaining)
                .unwrap();
            notes = guard;
        }
    }

    /// Block until a `window/logMessage` whose text contains all `needles`
    /// (searching entries at index `since` onward). Returns the message text.
    pub fn await_log(&self, needles: &[&str], timeout: Duration, since: usize) -> String {
        let deadline = Instant::now() + timeout;
        let mut notes = self.shared.notifications.lock().unwrap();
        loop {
            for note in notes.iter().skip(since) {
                if note.get("method").and_then(Value::as_str) != Some("window/logMessage") {
                    continue;
                }
                let msg = note
                    .get("params")
                    .and_then(|p| p.get("message"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if needles.iter().all(|n| msg.contains(n)) {
                    return msg.to_owned();
                }
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                // Release the notifications lock before the (sleeping) probe so
                // the reader thread isn't blocked while we diagnose the timeout.
                drop(notes);
                panic!(
                    "no window/logMessage containing all of {needles:?} within {timeout:?}{}",
                    latency_barrier_timeout_note()
                );
            }
            let (guard, _) = self
                .shared
                .notify_cv
                .wait_timeout(notes, remaining)
                .unwrap();
            notes = guard;
        }
    }

    /// Block until a notification with `method` arrives; return it.
    pub fn await_notification(&self, method: &str, timeout: Duration) -> Value {
        let deadline = Instant::now() + timeout;
        let mut notes = self.shared.notifications.lock().unwrap();
        loop {
            if let Some(note) = notes
                .iter()
                .find(|n| n.get("method").and_then(Value::as_str) == Some(method))
            {
                return note.clone();
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                // Release the notifications lock before the (sleeping) probe so
                // the reader thread isn't blocked while we diagnose the timeout.
                drop(notes);
                panic!(
                    "no {method:?} notification within {timeout:?}{}",
                    latency_barrier_timeout_note()
                );
            }
            let (guard, _) = self
                .shared
                .notify_cv
                .wait_timeout(notes, remaining)
                .unwrap();
            notes = guard;
        }
    }

    /// Block until a server-initiated *request* (has an `id`, unlike a
    /// notification — e.g. `workspace/semanticTokens/refresh`) with `method`
    /// arrives (searching entries at index `since` onward); return it. The
    /// harness auto-replies to it regardless (see `auto_reply`), so this only
    /// observes that the server asked, without blocking that reply.
    pub fn await_server_request(&self, method: &str, timeout: Duration, since: usize) -> Value {
        self.try_await_server_request(method, timeout, since)
            .unwrap_or_else(|| panic!("no server-initiated {method:?} request within {timeout:?}"))
    }

    /// Like [`Lsp::await_server_request`] but returns `None` on timeout instead
    /// of panicking, for callers that treat "no such request arrived" as a
    /// normal, expected outcome rather than a failure — e.g. converging
    /// semantic tokens, where the *absence* of a `workspace/semanticTokens/refresh`
    /// means the first response was already the settled enriched stream.
    pub fn try_await_server_request(
        &self,
        method: &str,
        timeout: Duration,
        since: usize,
    ) -> Option<Value> {
        let deadline = Instant::now() + timeout;
        let mut reqs = self.shared.server_requests.lock().unwrap();
        loop {
            if let Some(req) = reqs
                .iter()
                .skip(since)
                .find(|n| n.get("method").and_then(Value::as_str) == Some(method))
            {
                return Some(req.clone());
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return None;
            }
            let (guard, _) = self
                .shared
                .requests_cv
                .wait_timeout(reqs, remaining)
                .unwrap();
            reqs = guard;
        }
    }

    /// A marker into the server-request log for `await_server_request(...,
    /// since)`, mirroring [`Lsp::notification_cursor`].
    pub fn server_request_cursor(&self) -> usize {
        self.shared.server_requests.lock().unwrap().len()
    }

    /// A snapshot of all buffered server-initiated requests.
    pub fn server_requests(&self) -> Vec<Value> {
        self.shared.server_requests.lock().unwrap().clone()
    }

    /// Drop buffered notifications so a later `await_*` only sees fresh ones.
    pub fn clear_notifications(&self) {
        self.shared.notifications.lock().unwrap().clear();
    }

    /// A snapshot of all buffered notifications.
    pub fn notifications(&self) -> Vec<Value> {
        self.shared.notifications.lock().unwrap().clone()
    }

    pub fn stderr_text(&self) -> String {
        self.shared.stderr.lock().unwrap().clone()
    }

    // -- feature requests ------------------------------------------------

    fn doc_pos(uri: &str, line: u32, ch: u32) -> Value {
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": ch },
        })
    }

    pub fn hover(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request("textDocument/hover", Self::doc_pos(uri, line, ch))
    }
    pub fn completion(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request("textDocument/completion", Self::doc_pos(uri, line, ch))
    }
    pub fn signature_help(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request("textDocument/signatureHelp", Self::doc_pos(uri, line, ch))
    }
    pub fn definition(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request("textDocument/definition", Self::doc_pos(uri, line, ch))
    }
    pub fn type_definition(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request("textDocument/typeDefinition", Self::doc_pos(uri, line, ch))
    }
    pub fn declaration(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request("textDocument/declaration", Self::doc_pos(uri, line, ch))
    }
    pub fn references(&mut self, uri: &str, line: u32, ch: u32, include_decl: bool) -> Value {
        let mut params = Self::doc_pos(uri, line, ch);
        params["context"] = json!({ "includeDeclaration": include_decl });
        self.request("textDocument/references", params)
    }
    pub fn document_highlight(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request(
            "textDocument/documentHighlight",
            Self::doc_pos(uri, line, ch),
        )
    }
    pub fn document_symbols(&mut self, uri: &str) -> Value {
        self.request(
            "textDocument/documentSymbol",
            json!({ "textDocument": { "uri": uri } }),
        )
    }
    pub fn workspace_symbols(&mut self, query: &str) -> Value {
        self.request("workspace/symbol", json!({ "query": query }))
    }
    pub fn semantic_tokens(&mut self, uri: &str) -> Value {
        self.request(
            "textDocument/semanticTokens/full",
            json!({ "textDocument": { "uri": uri } }),
        )
    }
    pub fn semantic_tokens_range(
        &mut self,
        uri: &str,
        start: (u32, u32),
        end: (u32, u32),
    ) -> Value {
        self.request(
            "textDocument/semanticTokens/range",
            json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": start.0, "character": start.1 },
                    "end": { "line": end.0, "character": end.1 },
                },
            }),
        )
    }
    pub fn implementation(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request("textDocument/implementation", Self::doc_pos(uri, line, ch))
    }
    pub fn document_links(&mut self, uri: &str) -> Value {
        self.request(
            "textDocument/documentLink",
            json!({ "textDocument": { "uri": uri } }),
        )
    }
    pub fn code_lens(&mut self, uri: &str) -> Value {
        self.request(
            "textDocument/codeLens",
            json!({ "textDocument": { "uri": uri } }),
        )
    }
    pub fn code_lens_resolve(&mut self, lens: Value) -> Value {
        self.request("codeLens/resolve", lens)
    }
    pub fn inlay_hints(&mut self, uri: &str, start: (u32, u32), end: (u32, u32)) -> Value {
        self.request(
            "textDocument/inlayHint",
            json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": start.0, "character": start.1 },
                    "end": { "line": end.0, "character": end.1 },
                },
            }),
        )
    }
    pub fn linked_editing_range(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request(
            "textDocument/linkedEditingRange",
            Self::doc_pos(uri, line, ch),
        )
    }
    pub fn prepare_call_hierarchy(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request(
            "textDocument/prepareCallHierarchy",
            Self::doc_pos(uri, line, ch),
        )
    }
    pub fn incoming_calls(&mut self, item: Value) -> Value {
        let mut params = json!({});
        params["item"] = item;
        self.request("callHierarchy/incomingCalls", params)
    }
    pub fn outgoing_calls(&mut self, item: Value) -> Value {
        let mut params = json!({});
        params["item"] = item;
        self.request("callHierarchy/outgoingCalls", params)
    }
    pub fn prepare_rename(&mut self, uri: &str, line: u32, ch: u32) -> Value {
        self.request("textDocument/prepareRename", Self::doc_pos(uri, line, ch))
    }
    pub fn rename(&mut self, uri: &str, line: u32, ch: u32, new_name: &str) -> Value {
        let mut params = Self::doc_pos(uri, line, ch);
        params["newName"] = json!(new_name);
        self.request("textDocument/rename", params)
    }
    pub fn folding_range(&mut self, uri: &str) -> Value {
        self.request(
            "textDocument/foldingRange",
            json!({ "textDocument": { "uri": uri } }),
        )
    }
    pub fn selection_range(&mut self, uri: &str, positions: Value) -> Value {
        let mut params = json!({ "textDocument": { "uri": uri } });
        params["positions"] = positions;
        self.request("textDocument/selectionRange", params)
    }
    pub fn formatting(&mut self, uri: &str, tab_size: u32, insert_spaces: bool) -> Value {
        self.request(
            "textDocument/formatting",
            json!({
                "textDocument": { "uri": uri },
                "options": { "tabSize": tab_size, "insertSpaces": insert_spaces },
            }),
        )
    }
    pub fn range_formatting(&mut self, uri: &str, range: Value, tab_size: u32) -> Value {
        let mut params = json!({
            "textDocument": { "uri": uri },
            "options": { "tabSize": tab_size, "insertSpaces": true },
        });
        params["range"] = range;
        self.request("textDocument/rangeFormatting", params)
    }
    pub fn code_actions(&mut self, uri: &str, range: Value, diagnostics: Value) -> Value {
        let mut params = json!({ "textDocument": { "uri": uri }, "context": {} });
        params["range"] = range;
        params["context"]["diagnostics"] = diagnostics;
        self.request("textDocument/codeAction", params)
    }
    pub fn execute_command(&mut self, command: &str, arguments: Value) -> Value {
        let mut params = json!({ "command": command });
        params["arguments"] = arguments;
        self.request("workspace/executeCommand", params)
    }

    // -- configuration ---------------------------------------------------

    /// The server's *resolved* config for `uri` (`tcl-lsp.getEffectiveConfig`) —
    /// the view the analyser/formatter actually applies.
    pub fn effective_config(&mut self, uri: &str) -> Value {
        self.execute_command("tcl-lsp.getEffectiveConfig", json!([uri]))
    }

    /// Make the server adopt `config` as the `tclLsp` section: update the reply
    /// this client returns for `workspace/configuration` and notify
    /// `didChangeConfiguration` so the server re-pulls. Returns the resolved
    /// config for `""`. Because each test owns its server, there is no shared
    /// state to restore (unlike pytest's `config_session`).
    pub fn apply_configuration(&mut self, config: Value) -> Value {
        *self.shared.tcllsp_config.lock().unwrap() = config;
        self.notify(
            "workspace/didChangeConfiguration",
            json!({ "settings": {} }),
        );
        self.effective_config("")
    }

    /// Apply `config`, then poll `getEffectiveConfig` for `settle_uri` until
    /// `predicate` holds (the deterministic barrier pytest's
    /// `apply_configuration(settle=...)` provides). Returns the settled config.
    pub fn apply_configuration_settle(
        &mut self,
        config: Value,
        settle_uri: &str,
        predicate: impl Fn(&Value) -> bool,
    ) -> Value {
        *self.shared.tcllsp_config.lock().unwrap() = config;
        self.notify(
            "workspace/didChangeConfiguration",
            json!({ "settings": {} }),
        );
        let deadline = Instant::now() + DEFAULT_TIMEOUT;
        let mut last = Value::Null;
        while Instant::now() < deadline {
            last = self.effective_config(settle_uri);
            if predicate(&last) {
                return last;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!(
            "config did not settle within {DEFAULT_TIMEOUT:?}; last: {last}{}",
            latency_barrier_timeout_note()
        );
    }
}

/// Whether every key of the `tclLsp` config `requested` is reflected in the
/// `getEffectiveConfig` reply `effective`.
///
/// The two shapes differ (`optimiser.profile` is reported as a flat
/// `optimiser_profile`), so the mapping is explicit. An unmapped key **panics**
/// rather than being skipped: silently ignoring it would silently restore the
/// unsynchronised behaviour this barrier exists to remove, and the next author
/// to add a config key to a test would get a flake instead of an error.
fn config_reflected(requested: &Value, effective: &Value) -> bool {
    let Some(req) = requested.as_object() else {
        return true;
    };
    req.iter().all(|(key, want)| match key.as_str() {
        // Nested feature toggles: each requested toggle must match.
        "features" => want.as_object().is_none_or(|feats| {
            feats.iter().all(|(feat, on)| {
                effective
                    .get("features")
                    .and_then(|f| f.get(feat))
                    .is_some_and(|got| got == on)
            })
        }),
        // Nested optimiser settings, reported flat.
        "optimiser" => want.as_object().is_none_or(|opt| {
            opt.iter().all(|(k, v)| {
                let flat = match k.as_str() {
                    "enabled" => "optimiser_enabled",
                    "profile" => "optimiser_profile",
                    // A per-code override (`optimiser.O109 = false`) lands in
                    // the disabled set, which is reported — but matching it
                    // properly means replicating the profile→disabled
                    // derivation, so refuse rather than pretend to check it.
                    other => panic!(
                        "config_reflected: no settle mapping for `optimiser.{other}` \
                         — add one (see getEffectiveConfig) so the config is a real barrier"
                    ),
                };
                effective.get(flat).is_some_and(|got| got == v)
            })
        }),
        "libraryPaths" => effective
            .get("library_paths")
            .is_some_and(|got| got == want),
        "dialect" => effective.get("dialect").is_some_and(|got| got == want),
        "lineLength" => effective.get("line_length").is_some_and(|got| got == want),
        other => panic!(
            "config_reflected: no settle mapping for `{other}` — add one (see \
             getEffectiveConfig) so `Lsp::with_config` remains a real barrier"
        ),
    })
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

/// The background reader loop: parse framed messages from `stdout` and route
/// them. Server-initiated requests are answered via the shared stdin.
fn read_loop(stdout: std::process::ChildStdout, shared: &Arc<Shared>) {
    let mut reader = BufReader::new(stdout);
    loop {
        // Read headers.
        let mut content_length = 0usize;
        let mut saw_header = false;
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                // EOF or read error: server exited.
                Ok(0) | Err(_) => return,
                Ok(_) => {}
            }
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                break;
            }
            saw_header = true;
            if let Some(rest) = trimmed.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = rest.trim().parse().unwrap_or(0);
            }
        }
        if !saw_header || content_length == 0 {
            continue;
        }
        let mut body = vec![0u8; content_length];
        if reader.read_exact(&mut body).is_err() {
            return;
        }
        let Ok(msg) = serde_json::from_slice::<Value>(&body) else {
            continue;
        };
        route(&msg, shared);
    }
}

fn route(msg: &Value, shared: &Arc<Shared>) {
    let has_id = msg.get("id").is_some_and(|v| !v.is_null());
    let is_request = msg.get("method").is_some();

    if has_id && !is_request {
        // Response to one of our requests.
        if let Some(id) = msg.get("id").and_then(Value::as_i64) {
            shared.responses.lock().unwrap().insert(id, msg.clone());
        }
    } else if has_id && is_request {
        // Server-initiated request — record it (so a test can assert the
        // server actually asked), then answer so the server never blocks.
        shared.server_requests.lock().unwrap().push(msg.clone());
        shared.requests_cv.notify_all();
        auto_reply(msg, shared);
    } else {
        // Notification.
        shared.notifications.lock().unwrap().push(msg.clone());
        shared.notify_cv.notify_all();
    }
}

fn auto_reply(msg: &Value, shared: &Arc<Shared>) {
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let result = if method == "workspace/configuration" {
        let items = msg
            .get("params")
            .and_then(|p| p.get("items"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Value::Array(
            items
                .iter()
                .map(|item| {
                    if item.get("section").and_then(Value::as_str) == Some("tclLsp") {
                        shared.tcllsp_config.lock().unwrap().clone()
                    } else {
                        Value::Null
                    }
                })
                .collect(),
        )
    } else {
        Value::Null
    };
    let payload = json!({ "jsonrpc": "2.0", "id": msg.get("id"), "result": result });
    let body = serde_json::to_vec(&payload).unwrap_or_default();
    let mut stdin = shared.stdin.lock().unwrap();
    let _ = write!(stdin, "Content-Length: {}\r\n\r\n", body.len());
    let _ = stdin.write_all(&body);
    let _ = stdin.flush();
}

#[cfg(test)]
mod tests {
    use super::{BARRIER_TIMEOUT_MARKER, latency_barrier_timeout_note};

    /// The footer every barrier timeout carries must keep classifying the
    /// failure as a *timeout* (not an oracle divergence), render the live
    /// scheduling probe, and point at the `heavy-lsp-e2e` isolation — so a
    /// future edit that guts the message is caught here rather than in a
    /// midnight CI triage. (Regression guard for the self-diagnosing barrier.)
    #[test]
    fn barrier_timeout_note_is_self_classifying() {
        let note = latency_barrier_timeout_note();
        assert!(
            note.contains(BARRIER_TIMEOUT_MARKER),
            "note must carry the greppable marker; got:\n{note}"
        );
        assert!(
            note.contains("TIMEOUT") && note.contains("NOT an oracle/content divergence"),
            "note must distinguish a timeout from a content divergence; got:\n{note}"
        );
        assert!(
            note.contains("PROBE:"),
            "note must render the live scheduling-health probe verdict; got:\n{note}"
        );
        assert!(
            note.contains("heavy-lsp-e2e") && note.contains("rust-tests"),
            "note must point at the nextest isolation and the unisolated job; got:\n{note}"
        );
    }
}

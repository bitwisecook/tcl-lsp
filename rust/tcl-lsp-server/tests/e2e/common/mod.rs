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
//! Each test spawns the real `tcl-lsp-server` binary (via
//! `CARGO_BIN_EXE_tcl-lsp-server`), talks LSP JSON-RPC to it over stdio, and
//! asserts on the responses — exactly what an editor does. A background reader
//! thread parses framed messages,
//! routing responses to blocked requests, buffering notifications (with a
//! condvar so `await_*` can wait), and auto-answering server-initiated requests
//! (`workspace/configuration`) so the server never blocks.
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
//! Genuine latency *guarantees* (issue #829's fast-tier promises) are a
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

/// Default per-request timeout.
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
    // Sample how late the scheduler wakes us from a short, known sleep, and how
    // oversubscribed the run queue is, *right now*. This runs only on the
    // already-failing path, so the probe's ~100 ms cost is irrelevant — and it
    // deliberately re-measures rather than reading the cached `load_factor()`,
    // because what matters here is the machine's state at the moment of giving
    // up.
    let dilation = sleep_dilation();
    let pressure = run_queue_pressure();

    let verdict = if dilation >= STARVATION_RATIO || pressure >= STARVATION_RATIO {
        format!(
            "PROBE: CPU STARVATION CONFIRMED — a {PROBE_SLEEP:?} sleep is waking {dilation:.1}x \
             late and the run queue is {pressure:.1}x oversubscribed right now, so the OS is \
             denying this test process CPU. Note that the barrier had ALREADY been stretched by \
             the measured load factor before it expired (see `scaled_timeout`), so this is a \
             machine that got slower *during* the wait — or a genuinely wedged server."
        )
    } else {
        format!(
            "PROBE: could not confirm starvation — a {PROBE_SLEEP:?} sleep woke {dilation:.1}x \
             late with the run queue {pressure:.1}x oversubscribed, both within normal noise. The \
             runner looks healthy *now*, and the barrier was already load-scaled, so suspect a \
             real server-latency regression (a hang/deadlock or a genuinely slow analysis)."
        )
    };

    format!(
        "\n\n{BARRIER_TIMEOUT_MARKER} — this is a TIMEOUT (the server did not respond in time), \
         NOT an oracle/content divergence (those fail with \"diverged\" / \"alignment broke\").\n\
         {verdict}\n\
         CONTEXT: every barrier deadline in this harness is multiplied by the measured capacity \
         factor (`load_factor`), so a merely-busy machine should not reach this message; the \
         latency-sensitive stress tests are additionally isolated into the single-slot \
         `heavy-lsp-e2e` nextest group (.config/nextest.toml) so they never starve each other."
    )
}

/// How late a short sleep must wake before the scheduler counts as starving
/// this process. Timer granularity alone puts a 20 ms sleep a few percent over;
/// anything at or beyond this multiple only happens when the OS cannot run this
/// thread promptly. Shared by [`load_factor`] and the timeout footer so both
/// speak about starvation in the same units.
const STARVATION_RATIO: f64 = 2.5;

/// Sleep length the scheduling probe samples.
const PROBE_SLEEP: Duration = Duration::from_millis(20);

/// How many samples the scheduling probe takes (median wins, so one unlucky
/// context switch cannot skew the verdict).
const PROBE_SAMPLES: usize = 5;

/// How long a measured [`load_factor`] stays valid before it is re-sampled.
/// The probe costs ~`PROBE_SAMPLES * PROBE_SLEEP`, so caching keeps it off the
/// per-barrier path in a suite that opens hundreds of documents while still
/// tracking load that arrives (or leaves) mid-run.
const LOAD_FACTOR_TTL: Duration = Duration::from_secs(1);

/// Cached `(sampled_at, factor)` for [`load_factor`].
static LOAD_FACTOR_CACHE: Mutex<Option<(Instant, f64)>> = Mutex::new(None);

/// Median of `PROBE_SAMPLES` short sleeps, as a multiple of the requested
/// duration. `≈1.0` when the OS wakes us on time; many-fold when it does not.
///
/// Self-normalising by construction — it is a ratio of a measured wake against
/// the duration we asked for, so it needs no calibrated "quiet machine"
/// constant and means the same thing on every host.
fn sleep_dilation() -> f64 {
    let mut ratios = [0f64; PROBE_SAMPLES];
    for r in &mut ratios {
        let t = Instant::now();
        std::thread::sleep(PROBE_SLEEP);
        *r = t.elapsed().as_secs_f64() / PROBE_SLEEP.as_secs_f64();
    }
    ratios.sort_by(|a, b| a.partial_cmp(b).expect("sleep ratios are finite"));
    ratios[PROBE_SAMPLES / 2]
}

/// Run-queue oversubscription: runnable threads per usable CPU. `≤1.0` when
/// there is a core available for every thread that wants one; `4.0` when four
/// threads are contending for every core — in which case CPU-bound work takes
/// about four times as long, which is precisely the correction a barrier needs.
///
/// Complements [`sleep_dilation`]: a sleeping thread is often woken promptly
/// even on a busy box, so scheduling latency under-reports pure *throughput*
/// starvation — which is what a CPU-bound analysis actually loses to. Neither
/// needs a calibrated "fast machine" constant.
///
/// Reads both numbers `/proc/loadavg` offers and takes the larger: the 1-minute
/// average (which lags — it under-reports the start of a heavy test suite) and
/// the instantaneous runnable count from the `running/total` field (which is
/// noisy but immediate). Returns `0.0` where `/proc/loadavg` does not exist
/// (non-Linux), leaving the sleep probe as the sole signal.
fn run_queue_pressure() -> f64 {
    let Ok(raw) = std::fs::read_to_string("/proc/loadavg") else {
        return 0.0;
    };
    let fields: Vec<&str> = raw.split_whitespace().collect();
    let one_minute = fields
        .first()
        .and_then(|f| f.parse::<f64>().ok())
        .unwrap_or(0.0);
    // Field 4 is `runnable/total`; the numerator counts threads that are
    // currently runnable, this one included.
    let runnable = fields
        .get(3)
        .and_then(|f| f.split('/').next())
        .and_then(|f| f.parse::<f64>().ok())
        .unwrap_or(0.0);
    let cpus = std::thread::available_parallelism().map_or(1.0, |n| {
        f64::from(u32::try_from(n.get()).unwrap_or(u32::MAX))
    });
    one_minute.max(runnable) / cpus
}

/// How much slower than an unloaded machine this process is currently being
/// run, as a multiplier `≥ 1.0`.
///
/// The maximum of two independent, calibration-free measurements — scheduling
/// latency ([`sleep_dilation`]) and run-queue oversubscription
/// ([`run_queue_pressure`]) — so either kind of starvation is caught: a cgroup
/// CPU cap that throttles wakeups, or a machine with more runnable threads than
/// cores. Neither needs a "what is fast" constant, which is the point: the
/// factor means the same thing on a laptop, a CI runner, and a container.
///
/// Cached for [`LOAD_FACTOR_TTL`] so a barrier-heavy suite pays for the probe
/// at most once a second.
#[must_use]
pub fn load_factor() -> f64 {
    let mut cache = LOAD_FACTOR_CACHE.lock().unwrap();
    if let Some((sampled_at, factor)) = *cache
        && sampled_at.elapsed() < LOAD_FACTOR_TTL
    {
        return factor;
    }
    let factor = sleep_dilation().max(run_queue_pressure()).max(1.0);
    *cache = Some((Instant::now(), factor));
    factor
}

/// Scale a wall-clock hang backstop by the machine's measured capacity.
///
/// See the module docs: barriers guard against a wedged server, and a starved
/// runner must not be mistaken for one.
#[must_use]
pub fn scaled_timeout(base: Duration) -> Duration {
    base.mul_f64(load_factor())
}

/// A latency assertion that keeps its teeth on a quiet machine and stays
/// deterministic on a loaded one.
///
/// Some e2e assertions are not content checks with a hang backstop but genuine
/// **latency guarantees** — issue #829's promise that the first
/// `semanticTokens/full` (or `/range`) response is never starved behind the
/// whole-file analysis. Deleting them, or widening them until they cannot fail,
/// would retire the guarantee. Keeping them as wall-clock absolutes makes them
/// fail on a machine that simply has no CPU to give.
///
/// So the limit is the larger of two relative statements:
///
/// * `base × load_factor()` — the quiet-machine budget, stretched by the
///   measured starvation ([`load_factor`]); and
/// * `NOOP_ROUND_TRIPS × noop` — where `noop` is this very server's measured
///   round-trip for a request that does no analysis. That expresses the
///   guarantee in the machine's own currency: "answering a cold viewport may
///   cost at most N trivial round-trips", which is exactly the property #829 is
///   about (the token path must not scale with the analysis) and is meaningful
///   whatever the host's absolute speed.
///
/// The no-op sample is taken **before** the measured operation (so it reflects
/// a server that is up and idle, not one mid-analysis); the scheduling factor
/// is sampled both before and at assertion time, and the larger wins, so load
/// that arrives during the operation still counts.
pub struct LatencyBudget {
    base: Duration,
    /// Median no-op round-trip, measured before the timed operation.
    noop: Duration,
    /// Scheduling factor measured before the timed operation.
    factor_before: f64,
}

impl LatencyBudget {
    /// How many no-op round-trips the guarded operation may cost.
    ///
    /// Chosen so that on a quiet machine `NOOP_ROUND_TRIPS × noop` is
    /// comfortably *below* every `base` this is used with — i.e. the tight
    /// absolute budget is what actually governs there, and the no-op term only
    /// takes over once the machine is slow enough that the absolute number has
    /// stopped describing it.
    const NOOP_ROUND_TRIPS: u32 = 500;

    /// Samples taken for the no-op round-trip (median wins).
    const NOOP_SAMPLES: usize = 5;

    /// Measure the machine's current capacity against `base`, the budget this
    /// guarantee holds to on an unloaded machine.
    pub fn probe(lsp: &mut Lsp, base: Duration) -> Self {
        let mut samples = Vec::with_capacity(Self::NOOP_SAMPLES);
        for _ in 0..Self::NOOP_SAMPLES {
            let started = Instant::now();
            // `getEffectiveConfig` is the cheapest real round-trip the server
            // offers: it reads settled configuration and touches neither the
            // document store nor the analyser, so what it times is the
            // client → event loop → handler → client path itself.
            let _ = lsp.effective_config("");
            samples.push(started.elapsed());
        }
        samples.sort_unstable();
        Self {
            base,
            noop: samples[samples.len() / 2],
            factor_before: load_factor(),
        }
    }

    /// The limit `elapsed` is held to, re-sampling the scheduling factor so
    /// load that arrived during the timed operation is accounted for.
    #[must_use]
    pub fn limit(&self) -> Duration {
        let factor = self.factor_before.max(load_factor());
        self.base
            .mul_f64(factor)
            .max(self.noop * Self::NOOP_ROUND_TRIPS)
    }

    /// Whether `elapsed` honours the guarantee on this machine.
    #[must_use]
    pub fn allows(&self, elapsed: Duration) -> bool {
        elapsed < self.limit()
    }

    /// A wall-clock backstop for an `await_*` barrier that belongs to the same
    /// guarded operation, so the barrier can never expire *before* the budget
    /// it is meant to let the operation reach.
    #[must_use]
    pub fn backstop(&self, base: Duration) -> Duration {
        scaled_timeout(base).max(self.limit())
    }

    /// Why the budget is what it is, for a failure message.
    #[must_use]
    pub fn report(&self) -> String {
        format!(
            "budget={:?} (base={:?} x scheduling factor {:.1}, floor {} no-op round-trips \
             x {:?}); a loaded machine stretches the budget, it does not remove it",
            self.limit(),
            self.base,
            self.factor_before.max(load_factor()),
            Self::NOOP_ROUND_TRIPS,
            self.noop,
        )
    }
}

/// Process-wide counter so `unique_uri` never collides across tests in one
/// integration-test binary (each binary is its own process).
static URI_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A fresh, unique `file://` URI. Version-tagged diagnostics never collide
/// because the path is unique per call.
pub fn unique_uri(suffix: &str) -> String {
    let n = URI_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("file:///e2e/{}_{n}.{suffix}", std::process::id())
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
    /// only linked editing — the default-editor contract every plain Tcl test
    /// runs against.
    pub fn tcl() -> Self {
        Self::with_config(json!({ "features": { "linkedEditingRange": true } }))
    }

    /// [`Lsp::tcl`] with test-only environment seams installed on the child.
    pub fn tcl_with_env(env: &[(&str, &str)]) -> Self {
        Self::with_config_env(json!({ "features": { "linkedEditingRange": true } }), env)
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
        Self::with_config_env(config, &[])
    }

    /// [`Lsp::with_config`] with test-only environment seams installed on the
    /// child process.
    pub fn with_config_env(config: Value, env: &[(&str, &str)]) -> Self {
        let mut lsp = Self::spawn_with_env(config, env);
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
        let deadline = Instant::now() + scaled_timeout(DEFAULT_TIMEOUT);
        loop {
            let effective = self.effective_config("");
            if config_reflected(requested, &effective) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "requested config was not applied within {:?} (load-scaled)\n  \
                 requested: {requested}\n  effective: {effective}{}",
                scaled_timeout(DEFAULT_TIMEOUT),
                latency_barrier_timeout_note()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

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

        let mut command = Command::new(bin);
        for (key, value) in env {
            command.env(key, value);
        }
        let mut child = command
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
            folder_configs: Mutex::new(HashMap::new()),
            configuration_reply_delay: Mutex::new(Duration::ZERO),
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
        self.initialize_at(&root)
    }

    /// [`Lsp::initialize`] against an explicit workspace-root URI.
    pub fn initialize_at(&mut self, root: &str) -> Value {
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

    /// Spawn a server rooted at a **real on-disk directory** and return the
    /// moment the `initialize` handshake is done — no config settle, no wait
    /// for the startup workspace scan.
    ///
    /// [`Lsp::tcl`] deliberately settles the config first, which incidentally
    /// gives the startup scan time to finish; a test that means to *race* that
    /// scan (the way a real editor's first `workspace/symbol` does) must not
    /// pay that barrier.
    pub fn at_workspace_root(root: &std::path::Path) -> Self {
        let mut lsp = Self::spawn(json!({ "features": { "linkedEditingRange": true } }));
        lsp.initialize_at(&format!("file://{}", root.to_string_lossy()));
        lsp
    }

    /// [`Lsp::with_config`] rooted at a **real on-disk directory**.
    ///
    /// The combination the two existing constructors do not cover, and the one
    /// any workspace-scope feature needs: a config the server will actually
    /// apply *and* a root it can walk. Settles the config first, for the same
    /// reason [`Lsp::with_config`] does — a test that opens a document before
    /// its settings have landed is testing the defaults.
    pub fn with_config_at_root(config: Value, root: &std::path::Path) -> Self {
        Self::with_config_at_root_env(config, root, &[])
    }

    /// [`Lsp::with_config_at_root`] with extra environment variables on the
    /// server process — see [`Lsp::spawn_with_env`].
    ///
    /// Settling the config here does **not** settle the startup *pack* reload:
    /// the server pulls and applies its configuration before it starts
    /// loading packs, so a test that means to race that load still can.
    pub fn with_config_at_root_env(
        config: Value,
        root: &std::path::Path,
        env: &[(&str, &str)],
    ) -> Self {
        let mut lsp = Self::spawn_with_env(config, env);
        lsp.initialize_at(&format!("file://{}", root.to_string_lossy()));
        let requested = lsp.shared.tcllsp_config.lock().unwrap().clone();
        lsp.settle_config(&requested);
        lsp
    }

    /// Spawn a **multi-root** server: `unscoped` is what the workspace-wide
    /// `workspace/configuration` pull returns, and each `(root, folder_config)`
    /// pair is what the *scoped* pull for that folder returns.
    ///
    /// This is the shape a real multi-root editor answers with — folder-level
    /// settings are invisible to the unscoped pull — so it is the only way to
    /// exercise per-folder resolution end to end.
    pub fn multi_root(unscoped: Value, folders: &[(&std::path::Path, Value)]) -> Self {
        let mut lsp = Self::spawn(unscoped);
        {
            let mut map = lsp.shared.folder_configs.lock().unwrap();
            for (root, config) in folders {
                map.insert(format!("file://{}", root.to_string_lossy()), config.clone());
            }
        }
        let roots: Vec<Value> = folders
            .iter()
            .enumerate()
            .map(|(i, (root, _))| {
                json!({ "uri": format!("file://{}", root.to_string_lossy()), "name": format!("root{i}") })
            })
            .collect();
        lsp.initialize_with_folders(&roots);
        lsp
    }

    /// [`Lsp::initialize`] against an explicit `workspaceFolders` array.
    pub fn initialize_with_folders(&mut self, folders: &[Value]) -> Value {
        let root = folders
            .first()
            .and_then(|f| f.get("uri"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let result = self.request_timeout(
            "initialize",
            json!({
                "processId": std::process::id(),
                "rootUri": root,
                "workspaceFolders": folders,
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
        let resp = self.request_response(method, params, timeout);
        if let Some(err) = resp.get("error") {
            panic!("{method} -> error {err}");
        }
        resp.get("result").cloned().unwrap_or(Value::Null)
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
    /// the per-URI `workspace_state.update` log line.
    pub fn open_ready(&mut self, uri: &str, text: &str) -> Vec<Value> {
        self.open_ready_lang(uri, text, "tcl")
    }

    /// Like [`Lsp::open_ready`] but with an explicit `languageId` (e.g.
    /// `"tcl-irule"` for iRules, `"tk"` for Tk, `"bigip"` for BIG-IP config).
    pub fn open_ready_lang(&mut self, uri: &str, text: &str, language_id: &str) -> Vec<Value> {
        self.open_ready_lang_timeout(uri, text, language_id, DEFAULT_TIMEOUT)
    }

    /// [`Lsp::open_ready`] with an explicit barrier backstop.
    ///
    /// [`DEFAULT_TIMEOUT`] sizes the barrier for an ordinary test document —
    /// tens of lines, analysed in milliseconds — where 30 s is unmistakably a
    /// hang guard. A handful of tests open *thousands* of lines, and a full
    /// debug-build analysis of one of those, on a machine already running the
    /// rest of the suite in parallel, is legitimately tens of seconds: there the
    /// default stops being a hang guard and becomes a bet on how many cores the
    /// test happened to get. Those call sites pass their own backstop rather
    /// than the whole harness inheriting a number sized for the worst case.
    /// (Load scaling still applies on top — see the module docs.)
    pub fn open_ready_timeout(&mut self, uri: &str, text: &str, timeout: Duration) -> Vec<Value> {
        self.open_ready_lang_timeout(uri, text, "tcl", timeout)
    }

    /// [`Lsp::open_ready_lang`] with an explicit barrier backstop.
    pub fn open_ready_lang_timeout(
        &mut self,
        uri: &str,
        text: &str,
        language_id: &str,
        timeout: Duration,
    ) -> Vec<Value> {
        self.open_document_lang(uri, text, language_id, 1);
        let diags = self.await_diagnostics_version(uri, Some(1), timeout);
        self.await_log(&["workspace_state.update", uri], timeout, 0);
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

    /// Block until the most recent `publishDiagnostics` for `uri` satisfies
    /// `settled`, returning it.
    ///
    /// For facts the server publishes **progressively**: a cross-file
    /// correction (issue #977) lands on a later publish than the document's
    /// own first result, because the project-wide call-site evidence is
    /// refreshed after publishing rather than in front of it — putting it in
    /// front delayed the semantic-token enrichment tier on a large document.
    /// [`Self::open_ready`] returns the *first* publish, so a test asserting a
    /// cross-file outcome has to wait for the converged one instead.
    pub fn await_diagnostics_settled(
        &self,
        uri: &str,
        timeout: Duration,
        settled: impl Fn(&[Value]) -> bool,
    ) -> Vec<Value> {
        let deadline = Instant::now() + scaled_timeout(timeout);
        let mut notes = self.shared.notifications.lock().unwrap();
        loop {
            let mut latest: Option<Vec<Value>> = None;
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
                latest = Some(
                    params
                        .get("diagnostics")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default(),
                );
            }
            if let Some(diags) = &latest
                && settled(diags)
            {
                return latest.unwrap_or_default();
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                drop(notes);
                panic!(
                    "diagnostics for {uri:?} never settled within {timeout:?}{}",
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

    /// Block until a `publishDiagnostics` for `uri` (optionally carrying exactly
    /// `version`) arrives; return the latest matching diagnostics array.
    pub fn await_diagnostics_version(
        &self,
        uri: &str,
        version: Option<i64>,
        timeout: Duration,
    ) -> Vec<Value> {
        let deadline = Instant::now() + scaled_timeout(timeout);
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

    /// Block until the server's `[timing] diagnostics master-off` marker for
    /// `uri` is logged (searching entries at index `since` onward), then
    /// return the diagnostics from the `publishDiagnostics` immediately
    /// preceding it in the notification stream.
    ///
    /// This is the deterministic-barrier analogue of the VS Code extension
    /// suite's `waitForMasterOffDiagnostics` + `getDiagnostics(docUri)`
    /// (`editors/vscode/src/test/helper.ts`): the master switch
    /// (`tclLsp.features.diagnostics = false`) publishes its empty set and
    /// *then* logs this marker (`run_diagnostics_master_off` in
    /// `tcl-lsp-server/src/lib.rs`), so the marker is proof the publish this
    /// test cares about has already landed.
    ///
    /// Do **not** use [`Self::await_diagnostics_version`] for this: it
    /// returns as soon as *any* `publishDiagnostics` for `uri`/`version`
    /// matches, with no way to tell a publish computed under the old
    /// (diagnostics-on) config from one computed under the new
    /// (diagnostics-off) config apart — both carry the same document
    /// version, because a config change never bumps it. A `didOpen`'s own
    /// analysis can still have a later publish for that version in flight
    /// (e.g. a converged/cross-file correction) when the config flips off;
    /// if that stale publish lands in the buffer before the master-off one,
    /// `await_diagnostics_version` returns the stale non-empty result
    /// instead of waiting for the clear (issue #1135). Keying on the
    /// marker — and reading only the publish that precedes it — closes that
    /// window: this scans in one pass under the same lock used by the
    /// condvar wait, so there is no gap between "the marker was observed"
    /// and "the matching publish was read" for a later notification to land
    /// in.
    pub fn await_diagnostics_master_off(
        &self,
        uri: &str,
        timeout: Duration,
        since: usize,
    ) -> Vec<Value> {
        self.await_marker_diagnostics("diagnostics master-off", uri, timeout, since)
    }

    /// Block until the server's `[timing] diagnostics excluded` marker for
    /// `uri` is logged, then return the diagnostics from the
    /// `publishDiagnostics` immediately preceding it — the
    /// `tclLsp.diagnostics.exclude` (#1556) analogue of
    /// [`Self::await_diagnostics_master_off`], with the same rationale: the
    /// marker (`run_diagnostics_excluded` in `tcl-lsp-server/src/lib.rs`) is
    /// logged only after the exclusion's empty publish landed, so keying on it
    /// cannot confuse a pre-exclusion publish with the clear.
    pub fn await_diagnostics_excluded(
        &self,
        uri: &str,
        timeout: Duration,
        since: usize,
    ) -> Vec<Value> {
        self.await_marker_diagnostics("diagnostics excluded", uri, timeout, since)
    }

    /// The shared scan behind the marker-keyed diagnostics barriers: find the
    /// first `window/logMessage` containing `marker` and `uri=<uri>` (from
    /// entry `since` onward) and return the last preceding
    /// `publishDiagnostics` for `uri`, waiting on the notification condvar
    /// until it appears or `timeout` (load-scaled) elapses.
    fn await_marker_diagnostics(
        &self,
        marker: &str,
        uri: &str,
        timeout: Duration,
        since: usize,
    ) -> Vec<Value> {
        let needle_uri = format!("uri={uri}");
        let deadline = Instant::now() + scaled_timeout(timeout);
        let mut notes = self.shared.notifications.lock().unwrap();
        loop {
            let mut last_diags_before_marker: Option<Vec<Value>> = None;
            for note in notes.iter().skip(since) {
                match note.get("method").and_then(Value::as_str) {
                    Some("textDocument/publishDiagnostics") => {
                        let params = note.get("params").cloned().unwrap_or(Value::Null);
                        if params.get("uri").and_then(Value::as_str) == Some(uri) {
                            last_diags_before_marker = Some(
                                params
                                    .get("diagnostics")
                                    .and_then(Value::as_array)
                                    .cloned()
                                    .unwrap_or_default(),
                            );
                        }
                    }
                    Some("window/logMessage") => {
                        let msg = note
                            .get("params")
                            .and_then(|p| p.get("message"))
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        if msg.contains(marker) && msg.contains(&needle_uri) {
                            return last_diags_before_marker.unwrap_or_else(|| {
                                drop(notes);
                                panic!(
                                    "{marker} marker for {uri:?} logged with no preceding \
                                     publishDiagnostics for it — the marker/publish ordering \
                                     contract in the server broke"
                                )
                            });
                        }
                    }
                    _ => {}
                }
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                drop(notes);
                panic!(
                    "no {marker} marker for {uri:?} within {timeout:?}{}",
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

    /// Block until a `window/logMessage` whose text contains all `needles`
    /// (searching entries at index `since` onward). Returns the message text.
    pub fn await_log(&self, needles: &[&str], timeout: Duration, since: usize) -> String {
        self.try_await_log(needles, timeout, since)
            .unwrap_or_else(|| {
                panic!(
                    "no window/logMessage containing all of {needles:?} within {timeout:?}{}",
                    latency_barrier_timeout_note()
                )
            })
    }

    /// Like [`Lsp::await_log`] but returns `None` on timeout instead of
    /// panicking, for callers that can make progress another way when the
    /// marker does not arrive — e.g. a convergence loop that simply re-issues
    /// its request rather than failing on a round the server never settled.
    pub fn try_await_log(
        &self,
        needles: &[&str],
        timeout: Duration,
        since: usize,
    ) -> Option<String> {
        let deadline = Instant::now() + scaled_timeout(timeout);
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
                    return Some(msg.to_owned());
                }
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return None;
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
        let deadline = Instant::now() + scaled_timeout(timeout);
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
        let deadline = Instant::now() + scaled_timeout(timeout);
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
    /// `semanticTokens/full` for `uri`, converged onto the **settled enriched**
    /// stream the way a conformant editor does.
    ///
    /// A bare `semantic_tokens` call races
    /// `SEMANTIC_TOKENS_FAST_PATH_BUDGET`: when the enriched (SSA/SCCP-informed)
    /// computation overruns 40 ms the server answers from the cheap coarse tier
    /// and pushes `workspace/semanticTokens/refresh` once the enriched stream
    /// lands. That is correct server behaviour and an editor re-requests — but a
    /// *test* that asserts on enrichment (a regex source retagged, a user-class
    /// method resolved) and reads only the first response is asserting on
    /// whichever tier happened to win, i.e. on how much CPU the machine had.
    /// Those tests pass on a quiet box and fail under parallel load, which is
    /// not a server defect (issue #1082).
    ///
    /// So this converges the way the client contract says to, driven by the
    /// server's own settled marker rather than by sleeps: request, wait for the
    /// convergence decision to be logged, and re-request when it says a
    /// refresh was warranted. A cancelled read or a repeat-suppressed refresh
    /// also re-races: neither has served the enriched stream. Only a response
    /// marked `served-enriched`, `compared-equal`, or `no-analysis` is final.
    /// Returns the last response, so a genuine content divergence is still
    /// reported by the caller's assertion rather than hanging here.
    pub fn semantic_tokens_settled(&mut self, uri: &str) -> Value {
        let deadline = Instant::now() + scaled_timeout(DEFAULT_TIMEOUT);
        loop {
            let req_since = self.server_request_cursor();
            let log_since = self.notification_cursor();
            let response = self.semantic_tokens(uri);
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return response;
            }
            // The server logs the marker for *every* `full` request, so this
            // arrives promptly whichever tier answered — no fixed grace, no
            // wall-clock guess. A miss means the server stopped talking.
            let Some(settled) = self.try_await_log(
                &["semantic_tokens.full_convergence.settled", uri],
                remaining,
                log_since,
            ) else {
                return response;
            };
            if settled.contains("refresh=true") || settled.contains("outcome=coalesced") {
                // A coalesced request's holder schedules the refresh when it
                // releases its claim, so wait for that just as we do a refresh
                // already recorded by this request's own continuation.
                let remaining = deadline.saturating_duration_since(Instant::now());
                if self
                    .try_await_server_request(
                        "workspace/semanticTokens/refresh",
                        remaining,
                        req_since,
                    )
                    .is_none()
                {
                    return response;
                }
            } else if settled.contains("outcome=served-enriched")
                || settled.contains("outcome=compared-equal")
                || settled.contains("outcome=no-analysis")
            {
                // These are the only outcomes whose response is known to be
                // final. In particular, `refresh-suppressed` has a differing
                // enriched stream, but the server deliberately will not emit a
                // second workspace-wide refresh for those same bytes.
                return response;
            }
            // A cancelled read, a repeat-suppressed refresh, or an unexpected
            // marker has not established that this response is enriched. Loop
            // and re-race under the existing overall deadline.
        }
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
    /// The rename request's JSON-RPC `error` object, or `null` when the
    /// request succeeded — the wire view of a safety-gate refusal.
    pub fn rename_error(&mut self, uri: &str, line: u32, ch: u32, new_name: &str) -> Value {
        let mut params = Self::doc_pos(uri, line, ch);
        params["newName"] = json!(new_name);
        self.request_response("textDocument/rename", params, REQUEST_TIMEOUT)
            .get("error")
            .cloned()
            .unwrap_or(Value::Null)
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
    /// state to restore.
    /// Change what this client answers `workspace/configuration` with, without
    /// telling the server.  For tests that drive the notification themselves —
    /// e.g. a burst, where the point is to count how many pulls the server
    /// actually makes.
    pub fn set_config(&mut self, config: Value) {
        *self.shared.tcllsp_config.lock().unwrap() = config;
    }

    /// Delay configuration replies for a transport-liveness scenario.
    pub fn set_configuration_reply_delay(&self, delay: Duration) {
        *self.shared.configuration_reply_delay.lock().unwrap() = delay;
    }

    pub fn apply_configuration(&mut self, config: Value) -> Value {
        *self.shared.tcllsp_config.lock().unwrap() = config;
        self.notify(
            "workspace/didChangeConfiguration",
            json!({ "settings": {} }),
        );
        self.effective_config("")
    }

    /// Apply `config`, then poll `getEffectiveConfig` for `settle_uri` until
    /// `predicate` holds — a deterministic barrier, so the caller never races
    /// the asynchronous re-pull. Returns the settled config.
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
        let deadline = Instant::now() + scaled_timeout(DEFAULT_TIMEOUT);
        let mut last = Value::Null;
        while Instant::now() < deadline {
            last = self.effective_config(settle_uri);
            if predicate(&last) {
                return last;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!(
            "config did not settle within {:?} (load-scaled); last: {last}{}",
            scaled_timeout(DEFAULT_TIMEOUT),
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
        // `tclLsp.specPacks` is reported verbatim, *and* the packs it produced
        // are reported beside it — so this barrier waits for the discovery +
        // load pass, not merely for the setting to be recorded.  Without that
        // second half a test could open a document before its pack was in the
        // registry and see the very W123 the pack exists to remove.
        "specPacks" => {
            effective.get("spec_packs").is_some_and(|got| got == want)
                && effective.get("spec_packs_loaded").is_some_and(|loaded| {
                    want.as_array().is_none_or(|w| {
                        w.is_empty() || loaded.as_array().is_some_and(|l| !l.is_empty())
                    })
                })
        }
        // Nested formatting settings; only `docstringStyle` has a settle
        // signal (`getEffectiveConfig`'s `docstring_style`) today.
        "formatting" => want.as_object().is_none_or(|fmt| {
            fmt.iter().all(|(k, v)| {
                let flat = match k.as_str() {
                    "docstringStyle" => "docstring_style",
                    other => panic!(
                        "config_reflected: no settle mapping for `formatting.{other}` \
                         — add one (see getEffectiveConfig) so the config is a real barrier"
                    ),
                };
                effective.get(flat).is_some_and(|got| got == v)
            })
        }),
        // `tclLsp.iruleslx` (#1707) is folder-scoped and reported resolved —
        // absolute paths, which the request does not carry — so the barrier is
        // that every declared plugin *name* has reached the applied config.
        // That is the thing a test then depends on: an unapplied declaration
        // resolves nothing at all.
        "iruleslx" => want
            .get("plugins")
            .and_then(Value::as_object)
            .is_none_or(|plugins| {
                let got = effective.get("iruleslx_plugins").and_then(Value::as_array);
                plugins.keys().all(|name| {
                    got.is_some_and(|list| {
                        list.iter()
                            .any(|entry| entry.get("plugin").is_some_and(|p| p == name))
                    })
                })
            }),
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
        let delay = *shared.configuration_reply_delay.lock().unwrap();
        if !delay.is_zero() {
            std::thread::sleep(delay);
        }
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
                    if item.get("section").and_then(Value::as_str) != Some("tclLsp") {
                        return Value::Null;
                    }
                    let scoped = item
                        .get("scopeUri")
                        .and_then(Value::as_str)
                        .and_then(|scope| {
                            shared
                                .folder_configs
                                .lock()
                                .unwrap()
                                .get(scope.trim_end_matches('/'))
                                .cloned()
                        });
                    scoped.unwrap_or_else(|| shared.tcllsp_config.lock().unwrap().clone())
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
    use super::{
        BARRIER_TIMEOUT_MARKER, Duration, latency_barrier_timeout_note, load_factor, scaled_timeout,
    };

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
            note.contains("heavy-lsp-e2e"),
            "note must point at the nextest isolation; got:\n{note}"
        );
        assert!(
            note.contains("load_factor"),
            "note must say the barrier was already load-scaled, so a reader does not \
             re-derive 'just widen the timeout'; got:\n{note}"
        );
    }

    /// The measured capacity factor must never *shrink* a barrier: a machine
    /// that is faster than the constants assume still gets exactly the deadline
    /// the source says, and a slower one gets proportionally more. (Regression
    /// guard: a factor below 1 would silently tighten every deadline in the
    /// harness and manufacture the flakes this exists to remove.)
    #[test]
    fn load_factor_never_tightens_a_barrier() {
        let factor = load_factor();
        assert!(
            factor >= 1.0,
            "the capacity factor must be a multiplier >= 1, got {factor}"
        );
        let base = Duration::from_secs(30);
        assert!(
            scaled_timeout(base) >= base,
            "a scaled barrier must never be shorter than the constant it scales"
        );
    }
}

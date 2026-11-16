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

//! Wall-clock barriers scaled by measured capacity: the load factor, `scaled_timeout`, `LatencyBudget` and the note a barrier timeout carries.

use super::*;

/// A greppable marker every latency-barrier timeout carries, so a CI-log scan
/// (human or tooling) can classify the failure without reading the prose.
pub(super) const BARRIER_TIMEOUT_MARKER: &str = "LATENCY-BARRIER-TIMEOUT";

/// Diagnostic footer appended to every latency-barrier timeout panic in this
/// harness (`request_timeout`, `await_diagnostics_version`, `await_log`,
/// `await_notification`, and the config-settle barrier).
///
/// A barrier timeout means the server did not *respond* within the deadline — a
/// latency failure, categorically distinct from an oracle/content divergence
/// (those panic with "diverged" / "alignment broke" instead). On CI the
/// dominant cause is the test process being denied CPU on an oversubscribed
/// runner, not a server defect. The latency-sensitive stress tests that once
/// made this common are now contention-resistant or belong to the manual tier;
/// CI runs the remaining native e2e surface in deterministic hash partitions.
///
/// Rather than leave every investigator to re-derive that from a bare "timed
/// out" message, we *probe scheduling health at the moment of giving up*: a
/// short sleep the OS honours many-fold late is direct, in-log evidence that
/// the runner is starving this process, so the timeout is a scheduling artefact
/// rather than a hang. If the probe finds the scheduler healthy it says so,
/// redirecting the investigation toward a real server-latency regression.
pub(super) fn latency_barrier_timeout_note() -> String {
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
/// **latency guarantees** — the promise that the first
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
///   cost at most N trivial round-trips", which is exactly the property this
///   guarantee is about (the token path must not scale with the analysis)
///   and is meaningful whatever the host's absolute speed.
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

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

//! Message passing bounded by a timeout — the wait discipline for the VS Code
//! suite.
//!
//! # The shape
//!
//! A test waits on the **actual signal** for the thing it is asserting, so the
//! good case finishes the instant the work does, and a wall-clock deadline
//! exists **only to bound the failure**. Polling with a long deadline gets
//! neither property: it is slow when it works (the answer sits unread until the
//! next tick) and useless as a bound when it does not (the deadline has to be
//! generous enough to cover the slowest legitimate case, so it no longer says
//! anything about a hang).
//!
//! [`awaitSignal`] is that shape, once: subscribe to the event, re-probe on
//! every event, resolve as soon as the predicate holds, and **reject** on a
//! bounded timeout with a message naming what was awaited, how long it waited
//! and what it last saw. A slow interval re-probe stays inside the loop purely
//! as a missed-event backstop — never as the primary mechanism.
//!
//! Rejecting matters as much as the event does. A wait that *resolves* on
//! timeout hands the test whatever happened to be present, so a genuine hang
//! surfaces as a confusing content assertion — or, when the assertion is a
//! negative ("no diagnostic of kind X"), passes vacuously.
//!
//! # Deadlines are scaled by measured capacity, never widened
//!
//! This mirrors the native e2e harness (`rust/tcl-lsp-server/tests/e2e/common/
//! mod.rs`): every deadline here is a *hang backstop*, not an assertion. A
//! fixed wall-clock backstop is wrong in exactly one direction — on a machine
//! the OS is giving a fraction of a core, a correct server misses it and a
//! *content* test fails as a *timeout*. That is the failure mode issue #1274
//! records: the VS Code suite stalled while ~9 agent build trees shared the
//! container.
//!
//! So deadlines are multiplied by [`loadFactor`], a measured probe of how much
//! capacity this process is actually getting. On an idle machine the factor is
//! `1.0` and every deadline is exactly what it reads in the source; under
//! contention it grows with the contention. The constants stay honest and a
//! quiet machine keeps the tight bound.
//!
//! That applies to **mocha's own per-test backstop** too. A test that overrides
//! it with `this.timeout(N)` must (a) scale `N` with [`scaledTimeout`], and (b)
//! choose it to outlast the *sum* of the bounded waits inside the test —
//! `activate` alone can spend 60s on a cold LSP handshake before the test's own
//! wait begins. An override that fires first reports "Timeout of Nms exceeded"
//! and names neither the outstanding wait nor why, which is precisely the
//! unattributable failure this module exists to remove.
//!
//! The cheap signal (run-queue oversubscription) is read up front; the
//! expensive one (scheduling latency, which costs ~100ms to sample) is taken
//! only **at the moment of giving up**, where its cost is irrelevant. If it
//! finds the machine starved, the wait is extended and resumed rather than
//! failed — a busy machine must not manufacture a failure. If it finds the
//! machine healthy, that verdict goes in the failure message, so the next
//! occurrence is attributable instead of needing re-run archaeology.

import * as fs from "fs";
import * as os from "os";

/** Greppable marker every bounded-wait timeout carries, so a CI-log scan can
 *  classify the failure without reading the prose. */
export const WAIT_TIMEOUT_MARKER = "VSCODE-WAIT-TIMEOUT";

/** Sleep length the scheduling probe samples. */
const PROBE_SLEEP_MS = 20;

/** Samples the scheduling probe takes (median wins, so one unlucky context
 *  switch cannot skew the verdict). */
const PROBE_SAMPLES = 5;

/**
 * How late a short sleep must wake before the scheduler counts as starving
 * this process. Timer granularity alone puts a 20ms sleep a few percent over;
 * anything at or beyond this multiple only happens when the OS cannot run this
 * thread promptly.
 */
const STARVATION_RATIO = 2.5;

/** How long a measured [`loadFactor`] stays valid before it is re-sampled. */
const LOAD_FACTOR_TTL_MS = 1_000;

/** Default deadline for [`awaitSignal`] when a caller does not name one. */
export const DEFAULT_SIGNAL_TIMEOUT_MS = 20_000;

/** Default missed-event backstop interval. Deliberately slow: it is a safety
 *  net for a dropped event, not the mechanism. */
export const DEFAULT_BACKSTOP_INTERVAL_MS = 500;

/**
 * How many times a wait may re-measure and extend itself before it gives up.
 * Bounds the worst case at `base x cap x extensions` so a wait can never become
 * unbounded, however loaded the machine claims to be.
 */
const MAX_EXTENSIONS = 3;

/** Ceiling on the measured factor, so a pathological reading cannot turn a
 *  bound into an eternity. */
const MAX_LOAD_FACTOR = 8;

/** Promisified setTimeout. */
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Promisified setTimeout on an **unref'd** timer.
 *
 * For sleeps that outlive the await that started them — [`bounded`]'s guard
 * keeps counting after its work has already won the race. A ref'd timer there
 * would hold the event loop open after the last test finished, which is the
 * "mocha completed but the runner never exited" half of issue #1274.
 */
function sleepDetached(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms).unref();
  });
}

/**
 * Run-queue oversubscription: runnable threads per usable CPU. `<=1.0` when
 * there is a core available for every thread that wants one; `4.0` when four
 * threads contend for every core — in which case CPU-bound work takes about
 * four times as long, which is precisely the correction a backstop needs.
 *
 * Reads both numbers `/proc/loadavg` offers and takes the larger: the 1-minute
 * average (which lags — it under-reports the start of a heavy suite) and the
 * instantaneous runnable count from the `running/total` field (noisy but
 * immediate). Falls back to `os.loadavg()` where `/proc` does not exist, and
 * returns `0` on platforms that report no load average at all (Windows),
 * leaving the sleep probe as the sole signal.
 */
function runQueuePressure(): number {
  const cpus = Math.max(1, os.cpus().length);
  let oneMinute = 0;
  let runnable = 0;
  try {
    const fields = fs.readFileSync("/proc/loadavg", "utf8").trim().split(/\s+/);
    oneMinute = Number(fields[0]);
    runnable = Number((fields[3] ?? "").split("/")[0]);
  } catch {
    oneMinute = os.loadavg()[0];
  }
  if (!Number.isFinite(oneMinute)) oneMinute = 0;
  if (!Number.isFinite(runnable)) runnable = 0;
  return Math.max(oneMinute, runnable) / cpus;
}

/**
 * Median of `PROBE_SAMPLES` short sleeps, as a multiple of the requested
 * duration. `~1.0` when the event loop turns over on time; many-fold when it
 * does not.
 *
 * Self-normalising by construction — a ratio of a measured wake against the
 * duration we asked for — so it needs no calibrated "quiet machine" constant
 * and means the same thing on every host. In the extension host it also
 * catches a *busy event loop*, not just a busy CPU, which is the shape a
 * starved test process actually takes here.
 *
 * Costs `PROBE_SAMPLES * PROBE_SLEEP_MS`, so it is only ever taken on the
 * already-failing path.
 */
async function sleepDilation(): Promise<number> {
  const ratios: number[] = [];
  for (let i = 0; i < PROBE_SAMPLES; i++) {
    const started = Date.now();
    await sleep(PROBE_SLEEP_MS);
    ratios.push((Date.now() - started) / PROBE_SLEEP_MS);
  }
  ratios.sort((a, b) => a - b);
  return ratios[Math.floor(PROBE_SAMPLES / 2)];
}

let _loadFactorCache: { at: number; factor: number } | null = null;

/**
 * How much slower than an unloaded machine this process is currently being
 * run, as a multiplier `>= 1.0`.
 *
 * The cheap half of the measurement: run-queue oversubscription only, so it
 * costs one small file read and can sit on the hot path of every wait. Cached
 * for [`LOAD_FACTOR_TTL_MS`]. The expensive scheduling-latency half is taken by
 * [`measuredLoadFactor`] only when a wait is about to fail.
 */
export function loadFactor(): number {
  const now = Date.now();
  if (_loadFactorCache && now - _loadFactorCache.at < LOAD_FACTOR_TTL_MS) {
    return _loadFactorCache.factor;
  }
  const factor = Math.min(MAX_LOAD_FACTOR, Math.max(1, runQueuePressure()));
  _loadFactorCache = { at: now, factor };
  return factor;
}

/**
 * The full measurement — the maximum of two independent, calibration-free
 * probes: scheduling latency ([`sleepDilation`]) and run-queue
 * oversubscription ([`runQueuePressure`]). Either kind of starvation is
 * caught: a cgroup CPU cap that throttles wakeups, or a machine with more
 * runnable threads than cores.
 *
 * Costs ~100ms, so it is taken only when a wait has otherwise expired.
 */
export async function measuredLoadFactor(): Promise<{ factor: number; note: string }> {
  const dilation = await sleepDilation();
  const pressure = runQueuePressure();
  const raw = Math.max(dilation, pressure, 1);
  const factor = Math.min(MAX_LOAD_FACTOR, raw);
  const starved = dilation >= STARVATION_RATIO || pressure >= STARVATION_RATIO;
  const note = starved
    ? `PROBE: CPU STARVATION CONFIRMED — a ${PROBE_SLEEP_MS}ms sleep is waking ` +
      `${dilation.toFixed(1)}x late and the run queue is ${pressure.toFixed(1)}x ` +
      `oversubscribed right now, so this test process is being denied CPU. The deadline ` +
      `had already been stretched by the measured factor before it expired, so this is a ` +
      `machine that got slower *during* the wait — or a genuinely wedged server.`
    : `PROBE: could not confirm starvation — a ${PROBE_SLEEP_MS}ms sleep woke ` +
      `${dilation.toFixed(1)}x late with the run queue ${pressure.toFixed(1)}x ` +
      `oversubscribed, both within normal noise. The machine looks healthy *now* and the ` +
      `deadline was already load-scaled, so suspect a real server hang or a predicate ` +
      `that can never hold.`;
  return { factor, note };
}

/**
 * Scale a wall-clock hang backstop by the machine's measured capacity.
 *
 * See the module docs: backstops guard against a wedged server, and a starved
 * runner must not be mistaken for one.
 */
export function scaledTimeout(baseMs: number): number {
  return Math.round(baseMs * loadFactor());
}

/** Anything with a `dispose` — `vscode.Disposable`, a `Timeout`, a custom
 *  unsubscribe. Structural so this module stays importable from plain Node
 *  (`runTest.ts`), which has no `vscode` to import. */
export interface Unsubscribe {
  dispose(): void;
}

export interface AwaitSignalOptions<T> {
  /** What is being awaited, in the failure message's own words. */
  label: string;
  /**
   * Subscribe to the event that announces the awaited thing may have changed,
   * calling `notify` each time. Returns the handle to unsubscribe with.
   *
   * This is the primary mechanism: `probe` is re-read on every `notify`, so the
   * good case completes as soon as the signal arrives.
   */
  subscribe: (notify: () => void) => Unsubscribe;
  /** Read the current state. Called once immediately, then on every signal and
   *  on every backstop tick. */
  probe: () => T | PromiseLike<T>;
  /** Whether `probe`'s value satisfies the wait. */
  predicate: (value: T) => boolean;
  /** Base deadline in ms, before load scaling. */
  timeout?: number;
  /** Missed-event backstop interval in ms. */
  backstopInterval?: number;
  /** Render the last-seen value for the failure message. Defaults to JSON. */
  describe?: (value: T | undefined) => string;
  /**
   * Which `probe` rejections mean "the state moved under you, ask again"
   * rather than "the test has failed".
   *
   * A probe that pulls from a provider can be *cancelled* by VS Code when the
   * document or its diagnostics change while the request is in flight — which
   * is precisely when a signal-driven wait probes hardest. Treating that as a
   * failure would make the wait flakier than the polling it replaced; it is
   * information, not an error. Anything this returns false for is rethrown
   * unchanged, so a real provider fault still fails the test immediately.
   */
  retryProbeErrors?: (error: unknown) => boolean;
  /** See [`TimeoutDiagnostic`]. */
  diagnose?: TimeoutDiagnostic;
}

/**
 * An extra investigation run **only** on the failing path, whose prose is
 * appended to the timeout message.
 *
 * A timeout says the answer never came; it does not say what the server was
 * doing instead, which is the gap issue #1294 records — four consecutive waits
 * on one document's `didOpen` drain expired with a healthy machine, and the
 * evidence could not tell "the server is wedged" apart from "this document's
 * queue is wedged". A caller that knows a cheaper, independent question to ask
 * supplies it here, and the answer lands in the same message as the failure.
 *
 * It must be self-bounding and must never throw: this runs when the test is
 * already failing, and a diagnostic that hangs would replace an attributable
 * failure with an unattributable one. [`diagnose`] enforces both.
 */
export type TimeoutDiagnostic = () => string | PromiseLike<string>;

/** Ceiling on a [`TimeoutDiagnostic`], so the diagnostic can never become the
 *  thing that hangs. Load-scaled like every other bound here: the probes a
 *  diagnostic makes are themselves load-scaled, so a fixed cap would truncate
 *  the answer on precisely the machines whose failures are hardest to read. */
const DIAGNOSTIC_BUDGET_MS = 15_000;

/**
 * Run a [`TimeoutDiagnostic`] under a hard cap, converting any failure of the
 * diagnostic itself into prose rather than letting it displace the timeout it
 * was meant to explain.
 */
async function diagnose(probe: TimeoutDiagnostic | undefined): Promise<string> {
  if (!probe) return "";
  const cap = scaledTimeout(DIAGNOSTIC_BUDGET_MS);
  try {
    const note = await Promise.race([
      Promise.resolve(probe()),
      sleepDetached(cap).then(
        () =>
          `follow-up probe did not finish within ${cap}ms, which is itself evidence: the ` +
          `server did not answer an independent question either`,
      ),
    ]);
    return note ? `\n  ${note}` : "";
  } catch (err) {
    return `\n  follow-up probe itself failed: ${err instanceof Error ? err.message : String(err)}`;
  }
}

function defaultDescribe(value: unknown): string {
  if (value === undefined) return "<never probed>";
  try {
    const json = JSON.stringify(value);
    return json === undefined ? String(value) : json;
  } catch {
    return String(value);
  }
}

/**
 * Wait for `predicate` to hold over `probe`, woken by the event `subscribe`
 * registers, bounded by a load-scaled deadline that **rejects**.
 *
 * The one wait shape this suite should use wherever an event exists. See the
 * module docs for why.
 *
 * On expiry it re-measures the machine (see [`measuredLoadFactor`]); a starved
 * machine buys an extension rather than a failure, up to `MAX_EXTENSIONS`, so
 * the wait stays bounded either way.
 */
export async function awaitSignal<T>(opts: AwaitSignalOptions<T>): Promise<T> {
  const base = opts.timeout ?? DEFAULT_SIGNAL_TIMEOUT_MS;
  const backstop = opts.backstopInterval ?? DEFAULT_BACKSTOP_INTERVAL_MS;
  const describe = opts.describe ?? defaultDescribe;
  const started = Date.now();
  let budget = scaledTimeout(base);
  let extensions = 0;
  let probes = 0;
  let signals = 0;
  let retriedProbes = 0;
  let lastRetriedError: unknown;
  let last: T | undefined;
  let wake: (() => void) | null = null;

  const subscription = opts.subscribe(() => {
    signals++;
    wake?.();
  });

  try {
    for (;;) {
      const signalsAtProbe = signals;
      probes++;
      let satisfied = false;
      try {
        last = await opts.probe();
        satisfied = opts.predicate(last);
      } catch (err) {
        if (!opts.retryProbeErrors?.(err)) throw err;
        retriedProbes++;
        lastRetriedError = err;
      }
      if (satisfied) return last as T;

      // A signal that arrived *while* the probe was in flight refers to state
      // the probe may not have seen — re-read at once rather than sleeping on
      // an answer that is already stale.
      if (signals !== signalsAtProbe) continue;

      const remaining = started + budget - Date.now();
      if (remaining <= 0) {
        const { factor, note } = await measuredLoadFactor();
        const wanted = Math.round(base * factor);
        if (extensions < MAX_EXTENSIONS && wanted > budget) {
          budget = wanted;
          extensions++;
          continue;
        }
        const waited = Date.now() - started;
        const retried =
          retriedProbes === 0
            ? ""
            : `\n  ${retriedProbes} probe(s) were retried after a recoverable error, last: ` +
              `${lastRetriedError instanceof Error ? lastRetriedError.message : String(lastRetriedError)}`;
        throw new Error(
          `${WAIT_TIMEOUT_MARKER}: timed out awaiting ${opts.label}\n` +
            `  waited ${waited}ms (base ${base}ms x measured load factor ` +
            `${factor.toFixed(1)}; ${extensions} extension(s); ${probes} probe(s), ` +
            `${signals} signal(s))\n` +
            `  last seen: ${describe(last)}${retried}\n` +
            `  ${note}${await diagnose(opts.diagnose)}`,
        );
      }

      await new Promise<void>((resolve) => {
        const timer = setTimeout(finish, Math.min(remaining, backstop));
        wake = finish;
        function finish() {
          clearTimeout(timer);
          wake = null;
          resolve();
        }
      });
    }
  } finally {
    wake = null;
    subscription.dispose();
  }
}

/**
 * Bound an otherwise-unbounded promise with a load-scaled deadline and a
 * diagnostic rejection.
 *
 * Some awaits have no signal to wait on because they *are* the signal — a
 * command round-trip to the server, `extension.activate()`, opening a
 * document. They resolve when the work is done, which is the right shape
 * already; what they lack is a bound. Without one the only backstop is mocha's
 * per-test timeout, which is how issue #1274's stall became 60s per test and
 * then a runner that never exited: the mocha timeout fires with no idea which
 * await was outstanding.
 *
 * The underlying promise is left running on rejection — the test is failing
 * anyway, and cancelling a VS Code command is not possible.
 */
export function bounded<T>(
  work: PromiseLike<T>,
  label: string,
  opts?: { timeout?: number; diagnose?: TimeoutDiagnostic },
): Promise<T> {
  const base = opts?.timeout ?? DEFAULT_SIGNAL_TIMEOUT_MS;
  const started = Date.now();
  let settled = false;

  const result = Promise.resolve(work);
  result.then(
    () => {
      settled = true;
    },
    () => {
      settled = true;
    },
  );

  const guard = (async (): Promise<never> => {
    let budget = scaledTimeout(base);
    for (let extensions = 0; ;) {
      const remaining = started + budget - Date.now();
      if (remaining > 0) await sleepDetached(remaining);
      if (!settled) {
        const { factor, note } = await measuredLoadFactor();
        const wanted = Math.round(base * factor);
        if (!settled) {
          if (extensions < MAX_EXTENSIONS && wanted > budget) {
            budget = wanted;
            extensions++;
            continue;
          }
          throw new Error(
            `${WAIT_TIMEOUT_MARKER}: timed out awaiting ${label}\n` +
              `  waited ${Date.now() - started}ms (base ${base}ms x measured load factor ` +
              `${factor.toFixed(1)}; ${extensions} extension(s))\n` +
              `  this await has no completion signal to wait on — it *is* the signal — so a ` +
              `timeout here means the request never came back.\n` +
              `  ${note}${await diagnose(opts?.diagnose)}`,
          );
        }
      }
      // `work` already settled: retire without ever settling, so the race below
      // is decided by `result` alone.
      return new Promise<never>(() => {});
    }
  })();
  // The guard is only ever consumed by the race below, which stops listening
  // the moment `work` wins; keep a late rejection from going unhandled.
  guard.catch(() => {});

  return Promise.race([result, guard]);
}

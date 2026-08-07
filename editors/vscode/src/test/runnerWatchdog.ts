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

//! The launch-to-exit watchdog shared by `runTest.ts` and `runMultiFolderTest.ts`.
//!
//! # Progress, not elapsed time
//!
//! The old bound was a flat wall-clock budget (`DEFAULT_EXIT_TIMEOUT_MS =
//! 180_000`): the runner gave up 180s after launch, whatever was happening.
//! Issue #1293 is that the suite's own honest runtime (~190s for 846 tests)
//! grew past that budget, so a fully green run raced its own successful exit
//! and lost — reported as "mocha never completed (likely hung)" with zero
//! failures.
//!
//! The fix is to bound *lack of progress*, not elapsed time: a run that is
//! still completing tests is not hung however long it has taken; a run whose
//! `completed + failed` count and in-flight test title have not moved for
//! [`DEFAULT_NO_PROGRESS_TIMEOUT_MS`] is. The wall-clock ceiling
//! ([`DEFAULT_ABSOLUTE_CEILING_MS`]) stays only as a generous backstop for a
//! run that never stalls but also never finishes.
//!
//! This module holds the *pure* half of that decision ([`evaluateWatchdog`]) —
//! it does no I/O, so it is directly unit-testable with a synthetic sequence
//! of observations and a clock — plus the I/O half ([`runWatchedSuite`],
//! [`createHeartbeatWriter`]) that both runners share instead of each keeping
//! their own copy. Free of `import "vscode"` for the same reason `signal.ts`
//! is: the pure decision function must be callable (and testable) outside the
//! extension host.

import * as fs from "fs";
import * as path from "path";
import { execSync } from "child_process";
import { downloadAndUnzipVSCode, runTests } from "@vscode/test-electron";
import { loadFactor, scaledTimeout } from "./signal";

/**
 * What the heartbeat file records, so a hang is attributable from outside the
 * extension host.
 *
 * The runner's watchdog used to give up with nothing to say but a process
 * list and "mocha never completed (likely hung)" — which named neither the
 * test that stalled nor whether the server was still alive (issue #1274).
 * The extension host is the only place that knows both, so it writes them
 * down continuously and this module reads the last entry after the fact.
 */
export interface Heartbeat {
  /** When this heartbeat was written (ms since epoch). Its *age* at read time
   *  is the evidence: a stale heartbeat means the extension host's event loop
   *  itself stopped turning, which is a different fault from a test stuck in a
   *  wait. */
  at: number;
  /** Tests currently between `test` and `test end` — normally exactly one. */
  inFlight: Array<{ title: string; forMs: number }>;
  completed: number;
  failed: number;
  /** Round-trip to the language server, taken only once a test has been in
   *  flight long enough to be suspicious. `null` until then. */
  server:
    | null
    | { responsive: true; roundTripMs: number }
    | { responsive: false; reason: string; afterMs: number };
  /** Measured machine load at the time of writing — the difference between "the
   *  suite is wedged" and "the container is oversubscribed". */
  loadFactor: number;
}

/** How often the heartbeat is refreshed by [`createHeartbeatWriter`], and the
 *  unit the staleness check in [`reportWatchdogVerdict`] is expressed against. */
export const HEARTBEAT_INTERVAL_MS = 2_000;

/** How long a single test may run before the heartbeat starts probing the
 *  server on every tick. Below this a slow test is just a slow test. */
export const SERVER_PROBE_AFTER_MS = 20_000;

/** Bound on the server probe itself, so the heartbeat can never become the
 *  thing that hangs. */
export const SERVER_PROBE_TIMEOUT_MS = 5_000;

/**
 * Mocha's own per-test backstop (base, before load scaling) — both `index.ts`'s
 * mocha `timeout:` and [`DEFAULT_NO_PROGRESS_TIMEOUT_MS`] below are derived from
 * this single constant so the two cannot drift apart.
 */
export const MOCHA_TEST_TIMEOUT_BASE_MS = 60_000;

/**
 * Base no-progress window (before load scaling): how long `completed + failed`
 * may sit unchanged, with the in-flight test title also unchanged, before the
 * watchdog calls it a stall.
 *
 * Must exceed [`MOCHA_TEST_TIMEOUT_BASE_MS`]: mocha kills a single stuck test
 * at that bound, and `completed + failed` advances the instant it does (the
 * `fail` event fires). A window at or below the mocha timeout would flag
 * *every* mocha-timeout-recovered test as a stall in the moment before mocha's
 * own backstop resolves it — mistaking the cure for the disease. 1.5x leaves a
 * full half of an extra mocha timeout as slack for the heartbeat's own
 * [`HEARTBEAT_INTERVAL_MS`] write cadence and the round trip of mocha's `fail`
 * handler, so one unlucky stuck test never trips this — while a *second*,
 * consecutive one (a genuine wedge, not a single flaky test) still does well
 * before the old flat 180s budget would have.
 */
export const DEFAULT_NO_PROGRESS_TIMEOUT_MS = MOCHA_TEST_TIMEOUT_BASE_MS * 1.5;

/**
 * Base absolute ceiling (before load scaling): bounds a run that never stalls
 * (by the definition above) but also never finishes.
 *
 * Deliberately generous. The old fixed 180s budget *was* the bug this module
 * fixes — it sat below the suite's own honest runtime (~190s for 846 tests),
 * so a fully green run raced its own successful exit. Progress-based
 * detection above is now the primary bound; this ceiling only needs to catch
 * a pathological case that keeps nudging `completed` without ever reaching
 * the end, so it can afford to sit an order of magnitude above a normal green
 * run rather than hugging it the way the old constant did.
 */
export const DEFAULT_ABSOLUTE_CEILING_MS = 900_000;

/**
 * Base grace period (before load scaling) before "no heartbeat file was ever
 * written" becomes its own verdict.
 *
 * Distinct from the no-progress window: there is no progress to have stalled
 * yet, and distinct from the ceiling: waiting out a 15-minute ceiling to
 * notice the extension host never started would make every genuine launch
 * failure slow to report for no reason.
 *
 * It has to cover everything between `runTests` starting and the extension
 * host reaching `run()`: Electron start-up, extension-host boot, and mocha
 * registering the compiled test files. [`createHeartbeatWriter`] writes its
 * first beat synchronously, so nothing after that point is inside this window
 * — a suite that is merely slow to start is bounded by the no-progress window
 * instead, which is the bound that understands progress.
 *
 * It does **not** have to cover downloading VS Code itself, which
 * [`runWatchedSuite`] does before starting the clock: a cold `.vscode-test`
 * fetches ~150MB, and no grace period derived from start-up cost could
 * honestly bound that.
 */
export const DEFAULT_NEVER_STARTED_GRACE_MS = 60_000;

/** Escape hatch that sets the absolute ceiling (base, pre-load-scaling). Was
 *  the *only* budget before issue #1293; now progress detection is primary
 *  and this only stretches (or, at `0`, disables) the backstop. */
export const CEILING_ENV_VAR = "TCL_LSP_VSCODE_TEST_EXIT_TIMEOUT_MS";

/** Escape hatch that sets the no-progress window (base, pre-load-scaling). At
 *  `0` the stall check is disabled and only the ceiling (and never-started
 *  grace) still bound the run. */
export const NO_PROGRESS_ENV_VAR = "TCL_LSP_VSCODE_TEST_NO_PROGRESS_TIMEOUT_MS";

function parseMsEnvVar(varName: string, fallback: number): number {
  const raw = process.env[varName];
  if (!raw) {
    return fallback;
  }
  const parsed = Number(raw);
  if (!Number.isFinite(parsed) || parsed < 0) {
    return fallback;
  }
  return Math.floor(parsed);
}

/** Read the absolute-ceiling escape hatch. `0` means "no ceiling" (the
 *  watchdog is disabled entirely — see [`runWatchedSuite`]). */
export function parseExitTimeoutMs(): number {
  return parseMsEnvVar(CEILING_ENV_VAR, DEFAULT_ABSOLUTE_CEILING_MS);
}

/** Read the no-progress-window escape hatch. `0` disables just the stall
 *  check, leaving the ceiling (and never-started grace) active. */
export function parseNoProgressTimeoutMs(): number {
  return parseMsEnvVar(NO_PROGRESS_ENV_VAR, DEFAULT_NO_PROGRESS_TIMEOUT_MS);
}

export function escapeDoubleQuotes(text: string): string {
  return text.split('"').join('\\"');
}

export function escapeSingleQuotes(text: string): string {
  return text.split("'").join("'\"'\"'");
}

export function cleanupStaleTestHosts(
  extensionDevelopmentPath: string,
  extensionTestsPath: string,
): void {
  try {
    const escapedDevPath = escapeSingleQuotes(extensionDevelopmentPath);
    const escapedTestsPath = escapeSingleQuotes(extensionTestsPath);
    execSync(
      `pkill -f 'extensionDevelopmentPath=${escapedDevPath}' || true; pkill -f 'extensionTestsPath=${escapedTestsPath}' || true`,
      { stdio: "ignore" },
    );
  } catch {
    // Cleanup is best-effort only.
  }
}

export function emitProcessSnapshot(
  extensionDevelopmentPath: string,
  extensionTestsPath: string,
): void {
  try {
    const escapedDevPath = escapeDoubleQuotes(extensionDevelopmentPath);
    const escapedTestsPath = escapeDoubleQuotes(extensionTestsPath);
    const pattern = `extensionDevelopmentPath=${escapedDevPath}|extensionTestsPath=${escapedTestsPath}|node ./out/test/runTest.js`;
    const cmd = `ps -axo pid,ppid,etime,command | grep -E "${pattern}"`;
    const output = execSync(cmd, { encoding: "utf8" });
    if (output.trim()) {
      console.error("Potentially stuck VS Code test processes:");
      console.error(output.trimEnd());
    }
  } catch {
    // Snapshot is best-effort only.
  }
}

/** Safe read of a heartbeat file: `null` covers both "does not exist yet" and
 *  "unreadable/corrupt" — both mean "no progress evidence at this instant". */
export function readHeartbeat(heartbeatMarker: string): Heartbeat | null {
  try {
    return JSON.parse(fs.readFileSync(heartbeatMarker, "utf8")) as Heartbeat;
  } catch {
    return null;
  }
}

/** One sample the watchdog took while racing the suite: the heartbeat file's
 *  contents (or `null` if unreadable) at a known elapsed time. */
export interface WatchdogObservation {
  /** Milliseconds since the watchdog started (i.e. since launch). */
  atMs: number;
  heartbeat: Heartbeat | null;
}

export interface WatchdogConfig {
  /** Load-scaled absolute ceiling in ms. `Infinity` disables it. */
  ceilingMs: number;
  /** Load-scaled no-progress window in ms. `Infinity` disables the stall check. */
  noProgressWindowMs: number;
  /** Load-scaled grace period in ms before "never started" fires. */
  neverStartedGraceMs: number;
}

export type WatchdogVerdict =
  | { kind: "never-started" }
  | { kind: "stalled"; noProgressForMs: number; lastHeartbeat: Heartbeat }
  | { kind: "ceiling"; lastHeartbeat: Heartbeat };

/** The progress signal: `completed + failed` advancing, or the in-flight test
 *  title(s) changing. *Not* heartbeat freshness — a test stuck on a promise
 *  still refreshes the heartbeat every [`HEARTBEAT_INTERVAL_MS`], so freshness
 *  alone would never catch a genuine stall. */
function progressKey(beat: Heartbeat): string {
  return `${beat.completed}|${beat.failed}|${beat.inFlight.map((t) => t.title).join(",")}`;
}

/**
 * The pure decision: given everything observed so far and the current clock,
 * should the watchdog give up, and with which verdict?
 *
 * No I/O, no timers — a deterministic fold over `history`, so it is directly
 * unit-testable with a synthetic timeline. `nowMs` is separate from the last
 * observation's `atMs` so a caller can ask "should we give up *now*" even when
 * the most recent read is a little stale.
 *
 * Priority order matters: never-started is checked first (there is no
 * progress to have stalled, so the stall check would never fire on its own),
 * then stalled, then the ceiling — so "ceiling reached" only fires for a run
 * that was genuinely still making progress, never for one that had already
 * stopped (that is reported as a stall instead, however far past the ceiling
 * it also happens to be).
 */
export function evaluateWatchdog(
  history: readonly WatchdogObservation[],
  nowMs: number,
  config: WatchdogConfig,
): WatchdogVerdict | null {
  let lastHeartbeat: Heartbeat | null = null;
  let lastProgressKey: string | null = null;
  let lastProgressAtMs = 0;
  let everHadHeartbeat = false;

  for (const obs of history) {
    if (obs.heartbeat === null) {
      continue;
    }
    everHadHeartbeat = true;
    lastHeartbeat = obs.heartbeat;
    const key = progressKey(obs.heartbeat);
    if (lastProgressKey === null || key !== lastProgressKey) {
      lastProgressKey = key;
      lastProgressAtMs = obs.atMs;
    }
  }

  if (!everHadHeartbeat) {
    return nowMs >= config.neverStartedGraceMs ? { kind: "never-started" } : null;
  }

  const noProgressForMs = nowMs - lastProgressAtMs;
  if (noProgressForMs >= config.noProgressWindowMs) {
    return { kind: "stalled", noProgressForMs, lastHeartbeat: lastHeartbeat as Heartbeat };
  }

  if (nowMs >= config.ceilingMs) {
    return { kind: "ceiling", lastHeartbeat: lastHeartbeat as Heartbeat };
  }

  return null;
}

/**
 * The one-line summary of why the watchdog gave up. Written so `failed == 0`
 * and progress that was genuinely advancing never gets called "likely hung" —
 * that flat wording (with no distinction between a stall and a slow-but-live
 * run) is what made issue #1293 need re-run archaeology instead of a log line.
 */
export function describeWatchdogVerdict(verdict: WatchdogVerdict): string {
  switch (verdict.kind) {
    case "never-started":
      return (
        "no heartbeat file was ever written, so the suite hung before or during its first " +
        "test (extension host failed to start, or run() never reached mocha.run)"
      );
    case "stalled": {
      const seconds = Math.round(verdict.noProgressForMs / 1000);
      const inFlight = verdict.lastHeartbeat.inFlight.length
        ? verdict.lastHeartbeat.inFlight.map((t) => t.title).join(", ")
        : "none";
      return (
        `no test completed for ${seconds}s (last progress: ${verdict.lastHeartbeat.completed} ` +
        `completed, ${verdict.lastHeartbeat.failed} failed, in flight: ${inFlight})`
      );
    }
    case "ceiling":
      return (
        `absolute ceiling reached while still completing tests (${verdict.lastHeartbeat.completed} ` +
        `completed, ${verdict.lastHeartbeat.failed} failed so far)`
      );
  }
}

/**
 * Print the full attributable breakdown behind a verdict — which test was in
 * flight, for how long, whether the language server was still answering, and
 * whether the extension host's own event loop was still turning. Without this
 * the watchdog giving up printed a process list and a guess, which is what
 * made issue #1274 need re-run archaeology rather than reading a log.
 */
export function reportWatchdogVerdict(verdict: WatchdogVerdict): void {
  console.error("--- watchdog report ---");
  console.error(`  ${describeWatchdogVerdict(verdict)}`);
  if (verdict.kind === "never-started") {
    console.error("--- end watchdog report ---");
    return;
  }
  const beat = verdict.lastHeartbeat;
  const age = Date.now() - beat.at;
  console.error(
    `  heartbeat written ${age}ms ago; ${beat.completed} test(s) completed, ${beat.failed} ` +
      `failed; measured load factor ${beat.loadFactor.toFixed(1)}`,
  );
  if (beat.inFlight.length === 0) {
    console.error(
      "  no test was in flight — mocha had finished the last test but the run() " +
        "callback never fired, or a root-level hook is stuck.",
    );
  } else {
    for (const t of beat.inFlight) {
      console.error(`  IN FLIGHT for ${t.forMs}ms: ${t.title}`);
    }
  }
  if (beat.server === null) {
    console.error(
      "  server: not probed — no test had been in flight long enough to be suspicious " +
        "when this heartbeat was written.",
    );
  } else if (beat.server.responsive) {
    console.error(
      `  server: RESPONSIVE (${beat.server.roundTripMs}ms round trip), so the stall is in ` +
        "the test or the extension host, not a wedged server.",
    );
  } else {
    console.error(
      `  server: NOT RESPONDING after ${beat.server.afterMs}ms (${beat.server.reason}) — ` +
        "the language server stopped answering, so suspect the server, not the test.",
    );
  }
  if (age > 3 * HEARTBEAT_INTERVAL_MS) {
    console.error(
      `  the heartbeat itself is ${age}ms stale (it refreshes every ${HEARTBEAT_INTERVAL_MS}ms), ` +
        "so the extension host's event loop stopped turning — a blocked main thread or a " +
        "crashed host, not a test waiting on a promise.",
    );
  }
  console.error("--- end watchdog report ---");
}

/** Everything [`createHeartbeatWriter`] needs from its host runner. Injected
 *  rather than imported so this module never needs `import "vscode"` — the
 *  probe itself talks to the language server via the extension host's
 *  `vscode.commands.executeCommand`, which only the caller (`index.ts` /
 *  `multiFolder/index.ts`) can provide. */
export interface HeartbeatWriterOptions {
  heartbeatMarker: string;
  /** Ask the language server a question that touches neither the document
   *  store nor the analyser, so what it times is the client→server→client
   *  path itself. Only called once a test has been in flight for
   *  [`SERVER_PROBE_AFTER_MS`]. */
  probeServer: () => Promise<NonNullable<Heartbeat["server"]>>;
}

export interface HeartbeatWriter {
  /** Wire this to mocha's `test` event. */
  onTestStart: (fullTitle: string) => void;
  /** Wire this to mocha's `test end` event. */
  onTestEnd: (fullTitle: string) => void;
  /** Wire this to mocha's `fail` event. */
  onFail: () => void;
  /** Stop the refresh interval. Call from mocha's completion callback. */
  stop: () => void;
}

/**
 * The heartbeat-writing driver shared by `index.ts` and `multiFolder/index.ts`
 * — previously only the former had one, which left a hung multi-folder run
 * with no progress evidence at all and (worse) a runner that treated its own
 * timeout as success.
 *
 * Removes any stale heartbeat file up front and starts an unref'd refresh
 * interval; the heartbeat must never be the reason the extension host stays
 * alive after the suite finishes.
 */
export function createHeartbeatWriter(options: HeartbeatWriterOptions): HeartbeatWriter {
  const { heartbeatMarker, probeServer } = options;

  fs.mkdirSync(path.dirname(heartbeatMarker), { recursive: true });
  try {
    fs.unlinkSync(heartbeatMarker);
  } catch {
    // No stale heartbeat to remove.
  }

  const inFlight = new Map<string, number>();
  let completed = 0;
  let failed = 0;
  let lastServerProbe: Heartbeat["server"] = null;
  let probeInFlight = false;

  const writeHeartbeat = () => {
    const now = Date.now();
    const entries = [...inFlight.entries()].map(([title, startedAt]) => ({
      title,
      forMs: now - startedAt,
    }));
    const stalled = entries.some((e) => e.forMs >= SERVER_PROBE_AFTER_MS);
    if (stalled && !probeInFlight) {
      probeInFlight = true;
      void probeServer().then((result) => {
        lastServerProbe = result;
        probeInFlight = false;
      });
    } else if (!stalled) {
      lastServerProbe = null;
    }
    const beat: Heartbeat = {
      at: now,
      inFlight: entries,
      completed,
      failed,
      server: lastServerProbe,
      loadFactor: loadFactor(),
    };
    try {
      fs.writeFileSync(heartbeatMarker, JSON.stringify(beat) + "\n", "utf8");
    } catch {
      // The heartbeat is diagnostics only — never fail a run over it.
    }
  };

  // Write one immediately rather than waiting out the first interval: the
  // file's *existence* is what tells the watchdog the host started at all, so
  // publishing it the moment `run()` is reached keeps everything after that
  // point inside the progress-aware bound rather than the start-up grace.
  writeHeartbeat();
  const interval = setInterval(writeHeartbeat, HEARTBEAT_INTERVAL_MS);
  interval.unref();

  return {
    onTestStart: (fullTitle: string) => {
      inFlight.set(fullTitle, Date.now());
      writeHeartbeat();
    },
    onTestEnd: (fullTitle: string) => {
      inFlight.delete(fullTitle);
      completed++;
    },
    onFail: () => {
      failed++;
    },
    stop: () => {
      clearInterval(interval);
    },
  };
}

/** What [`runWatchedSuite`] needs on top of the plain `runTests` call. */
export interface RunWatchedSuiteOptions {
  extensionDevelopmentPath: string;
  extensionTestsPath: string;
  launchArgs: string[];
  /** Where the extension host writes its heartbeat (see [`createHeartbeatWriter`]). */
  heartbeatMarker: string;
  /** Where the extension host writes `{ failures: number }` once mocha's `run()`
   *  callback fires, pass or fail. Its *absence* after the watchdog gives up
   *  means mocha never finished at all — never safe to treat as success. */
  resultMarker: string;
  /** How often to re-check the heartbeat while racing the launch. Not itself
   *  load-scaled — polling cost is negligible and this is not a hang backstop,
   *  just the sampling rate for one. Defaults to [`HEARTBEAT_INTERVAL_MS`]. */
  pollIntervalMs?: number;
}

/**
 * Launch the suite via `@vscode/test-electron`, race it against the watchdog,
 * and exit the process with an honest status — the "launch/race/exit" logic
 * both runners previously duplicated (and `runMultiFolderTest.ts` got wrong:
 * it treated its own timeout as `process.exit(0)`, which could silently pass
 * a hung multi-folder run).
 *
 * Every path ends in `process.exit`, matching the shape both runners already
 * had; the `Promise<void>` return is never actually awaited by a caller that
 * does anything after it.
 */
export async function runWatchedSuite(opts: RunWatchedSuiteOptions): Promise<void> {
  const {
    extensionDevelopmentPath,
    extensionTestsPath,
    launchArgs,
    heartbeatMarker,
    resultMarker,
  } = opts;
  const pollIntervalMs = opts.pollIntervalMs ?? HEARTBEAT_INTERVAL_MS;

  cleanupStaleTestHosts(extensionDevelopmentPath, extensionTestsPath);
  for (const marker of [resultMarker, heartbeatMarker]) {
    try {
      fs.unlinkSync(marker);
    } catch {
      // No stale marker to remove.
    }
  }

  const ceilingBaseMs = parseExitTimeoutMs();
  const noProgressBaseMs = parseNoProgressTimeoutMs();
  const watchdogDisabled = ceilingBaseMs <= 0;

  // Fetch VS Code *before* the watchdog's clock starts. `runTests` downloads it
  // on demand, so a cold `.vscode-test` would otherwise spend minutes inside
  // the never-started grace period with no heartbeat to show for it — and be
  // failed for not having started, which is the same "a correct run was killed
  // by a deadline that isn't about it" mistake issue #1293 records. Nothing is
  // fetched when the cached build is already present.
  let vscodeExecutablePath: string | undefined;
  let prefetched = false;
  try {
    vscodeExecutablePath = await downloadAndUnzipVSCode();
    prefetched = true;
  } catch (err) {
    // Fall through to `runTests`, which will attempt (and report) the download
    // itself rather than us swallowing a network failure as a hang. The
    // never-started grace is stood down below: the download is now inside the
    // watchdog's clock, and no grace derived from start-up cost can bound it.
    console.warn("Could not pre-fetch VS Code; falling back to on-demand download:", err);
  }

  const runPromise = runTests({
    extensionDevelopmentPath,
    extensionTestsPath,
    launchArgs,
    vscodeExecutablePath,
  });

  const finishOk = (): void => {
    cleanupStaleTestHosts(extensionDevelopmentPath, extensionTestsPath);
    process.exit(0);
  };

  const finishFromMarkerOrFail = (verdict: WatchdogVerdict | null): void => {
    emitProcessSnapshot(extensionDevelopmentPath, extensionTestsPath);
    cleanupStaleTestHosts(extensionDevelopmentPath, extensionTestsPath);
    if (fs.existsSync(resultMarker)) {
      let failures: number | null = null;
      try {
        failures = JSON.parse(fs.readFileSync(resultMarker, "utf8")).failures;
      } catch {
        // Unparseable marker -- treat like a failure below.
      }
      if (failures === 0) {
        console.warn(
          "VS Code tests completed successfully but the runner did not exit; continuing after cleanup.",
        );
        process.exit(0);
        return;
      }
      console.error(
        `VS Code tests reported ${failures ?? "an unknown number of"} failure(s) before the watchdog gave up.`,
      );
      process.exit(1);
      return;
    }
    // No marker at all means mocha's run() callback never fired -- the suite
    // was still mid-run (hung, or just slower than the watchdog) when we gave
    // up. That is never safe to report as success: it has silently skipped
    // every test after the point it got stuck.
    const reason = verdict ? describeWatchdogVerdict(verdict) : "the runner failed unexpectedly";
    console.error(`${reason} -- mocha did not complete. Treating as failure.`);
    process.exit(1);
  };

  if (watchdogDisabled) {
    try {
      await runPromise;
      finishOk();
    } catch (err) {
      console.error("Failed to run tests:", err);
      process.exit(1);
    }
    return;
  }

  const ceilingMs = scaledTimeout(ceilingBaseMs);
  const config: WatchdogConfig = {
    ceilingMs,
    noProgressWindowMs: noProgressBaseMs > 0 ? scaledTimeout(noProgressBaseMs) : Infinity,
    // With the pre-fetch done, "no heartbeat yet" means start-up, which the
    // grace period bounds. Without it, `runTests` may be fetching ~334MB
    // inside the watchdog's clock, and calling that "the host never started"
    // would be the same mistake as the flat budget this module replaced. So
    // the grace stands down to the ceiling, which still bounds the run.
    neverStartedGraceMs: prefetched ? scaledTimeout(DEFAULT_NEVER_STARTED_GRACE_MS) : ceilingMs,
  };
  console.log(
    `==> watchdog: absolute ceiling ${config.ceilingMs}ms (base ${ceilingBaseMs}ms), ` +
      `no-progress window ${config.noProgressWindowMs}ms (base ${noProgressBaseMs}ms), ` +
      `never-started grace ${config.neverStartedGraceMs}ms` +
      `${prefetched ? "" : " (stood down to the ceiling — VS Code was not pre-fetched)"} — ` +
      `all x measured load factor ${loadFactor().toFixed(1)}`,
  );

  const started = Date.now();
  const history: WatchdogObservation[] = [];
  let verdict: WatchdogVerdict | null = null;
  let pollTimer: NodeJS.Timeout | undefined;

  const watchdogPromise = new Promise<never>((_resolve, reject) => {
    const tick = () => {
      const atMs = Date.now() - started;
      const heartbeat = readHeartbeat(heartbeatMarker);
      history.push({ atMs, heartbeat });
      const decided = evaluateWatchdog(history, atMs, config);
      if (decided) {
        verdict = decided;
        reject(new Error("watchdog gave up"));
        return;
      }
      pollTimer = setTimeout(tick, pollIntervalMs);
      pollTimer.unref();
    };
    pollTimer = setTimeout(tick, pollIntervalMs);
    pollTimer.unref();
  });
  // Keep a late rejection (the loop keeps scheduling until cleared below, so
  // in practice this never fires again once the race is decided) from going
  // unhandled.
  watchdogPromise.catch(() => {});

  try {
    await Promise.race([runPromise, watchdogPromise]);
    if (pollTimer) clearTimeout(pollTimer);
    finishOk();
  } catch (err) {
    if (pollTimer) clearTimeout(pollTimer);
    if (verdict) {
      // Read the heartbeat-derived report *before* the cleanup below kills
      // the host that writes it, so what is reported is the state at the
      // moment of giving up.
      reportWatchdogVerdict(verdict);
      finishFromMarkerOrFail(verdict);
      return;
    }
    // runPromise itself rejected (a real launch failure), not the watchdog.
    emitProcessSnapshot(extensionDevelopmentPath, extensionTestsPath);
    cleanupStaleTestHosts(extensionDevelopmentPath, extensionTestsPath);
    console.error("Failed to run tests:", err);
    process.exit(1);
  }
}

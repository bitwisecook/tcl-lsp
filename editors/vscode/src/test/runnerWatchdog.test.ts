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

// Pure decision-function tests for the watchdog (issue #1293). `evaluateWatchdog`
// does no I/O, so this file needs no `vscode` API — see `runnerWatchdog.ts`'s
// module docs for why that matters (it must be callable, and testable, outside
// the extension host, the same reason `signal.ts` has no `vscode` import).
import * as assert from "assert";
import {
  DEFAULT_ABSOLUTE_CEILING_MS,
  DEFAULT_NEVER_STARTED_GRACE_MS,
  DEFAULT_NO_PROGRESS_TIMEOUT_MS,
  MAX_TEST_TIMEOUT_BASE_MS,
  describeWatchdogVerdict,
  evaluateWatchdog,
  Heartbeat,
  MOCHA_TEST_TIMEOUT_BASE_MS,
  WatchdogConfig,
  WatchdogObservation,
} from "./runnerWatchdog";

/** The project defaults, unscaled (as if the load factor were exactly 1, which
 *  is what the pure function reasons about; scaling happens in
 *  `runWatchedSuite` before the config reaches it).
 *
 *  Taken from the exported constants rather than repeated as literals, so a
 *  change to a default is checked against these properties instead of quietly
 *  leaving them testing a config the product no longer uses. */
const CONFIG: WatchdogConfig = {
  ceilingMs: DEFAULT_ABSOLUTE_CEILING_MS,
  noProgressWindowMs: DEFAULT_NO_PROGRESS_TIMEOUT_MS,
  neverStartedGraceMs: DEFAULT_NEVER_STARTED_GRACE_MS,
};

function beat(overrides: Partial<Heartbeat> = {}): Heartbeat {
  return {
    at: Date.now(),
    inFlight: [],
    completed: 0,
    failed: 0,
    server: null,
    loadFactor: 1,
    ...overrides,
  };
}

function obs(atMs: number, heartbeat: Heartbeat | null): WatchdogObservation {
  return { atMs, heartbeat };
}

suite("runnerWatchdog: evaluateWatchdog (issue #1293)", () => {
  test("the default bounds keep the relationships their rationale depends on", () => {
    // Each of these is an argument made in a doc comment on the constant. A
    // future default that breaks one reintroduces a specific failure mode, so
    // they are asserted rather than left to the prose.
    assert.ok(
      MAX_TEST_TIMEOUT_BASE_MS >= MOCHA_TEST_TIMEOUT_BASE_MS,
      "the per-test ceiling must be at least the suite default it is a ceiling on",
    );
    assert.ok(
      DEFAULT_NO_PROGRESS_TIMEOUT_MS > MAX_TEST_TIMEOUT_BASE_MS,
      "the no-progress window must exceed the longest a single test may run before mocha " +
        "kills it — not merely the suite default. Otherwise the watchdog kills the whole " +
        "run before mocha can kill the one test, losing the per-test attribution",
    );
    assert.ok(
      DEFAULT_NEVER_STARTED_GRACE_MS < DEFAULT_NO_PROGRESS_TIMEOUT_MS,
      "a host that never started must be reported before the progress bound could fire",
    );
    assert.ok(
      DEFAULT_ABSOLUTE_CEILING_MS > 4 * DEFAULT_NO_PROGRESS_TIMEOUT_MS,
      "the ceiling is a backstop, not the primary bound — it must sit well above the " +
        "window that actually detects wedges",
    );
    // The regression itself: the suite's own honest runtime (~190s for 846
    // tests) must be nowhere near the ceiling. The old budget was 180s, which
    // sat *below* it — that is issue #1293.
    assert.ok(
      DEFAULT_ABSOLUTE_CEILING_MS > 4 * 190_000,
      "the ceiling must leave a green run room to grow, not hug its current runtime",
    );
  });

  test("a green-but-slow suite past the old 180s budget is never killed", () => {
    // The ticket's TP->TN fix: 846 tests each taking ~225ms averages out to
    // ~190s total, well past the old flat 180_000ms budget. Simulate a
    // heartbeat every 2s (HEARTBEAT_INTERVAL_MS) with `completed` climbing
    // steadily the whole way, and assert the watchdog never gives up.
    const history: WatchdogObservation[] = [];
    let verdict = null;
    for (let atMs = 0, completed = 0; atMs <= 190_000; atMs += 2_000, completed += 9) {
      const h = beat({ completed, inFlight: [{ title: `test #${completed}`, forMs: 200 }] });
      history.push(obs(atMs, h));
      verdict = evaluateWatchdog(history, atMs, CONFIG);
      assert.strictEqual(
        verdict,
        null,
        `must not give up at ${atMs}ms while completed=${completed} is still advancing`,
      );
    }
  });

  test("a genuinely wedged suite is killed with the stalled verdict", () => {
    // completed/failed/in-flight title all frozen from t=0 onward -- a real
    // wedge, not a slow-but-live run.
    const frozen = beat({
      completed: 40,
      failed: 0,
      inFlight: [{ title: "wedged test", forMs: 0 }],
    });
    const history: WatchdogObservation[] = [obs(0, frozen)];
    for (let atMs = 2_000; atMs < CONFIG.noProgressWindowMs; atMs += 2_000) {
      history.push(obs(atMs, frozen));
      assert.strictEqual(
        evaluateWatchdog(history, atMs, CONFIG),
        null,
        `must not give up before the no-progress window elapses (at ${atMs}ms)`,
      );
    }
    const giveUpAt = CONFIG.noProgressWindowMs;
    history.push(obs(giveUpAt, frozen));
    const verdict = evaluateWatchdog(history, giveUpAt, CONFIG);
    assert.ok(verdict, "must give up once the no-progress window elapses");
    assert.strictEqual(verdict?.kind, "stalled");
    if (verdict?.kind === "stalled") {
      assert.strictEqual(verdict.lastHeartbeat.completed, 40);
      assert.ok(verdict.noProgressForMs >= CONFIG.noProgressWindowMs);
    }
  });

  test("a host that never wrote a heartbeat is killed promptly with the never-started verdict", () => {
    const history: WatchdogObservation[] = [];
    for (let atMs = 0; atMs < CONFIG.neverStartedGraceMs; atMs += 2_000) {
      history.push(obs(atMs, null));
      assert.strictEqual(
        evaluateWatchdog(history, atMs, CONFIG),
        null,
        `must not give up before the never-started grace elapses (at ${atMs}ms)`,
      );
    }
    history.push(obs(CONFIG.neverStartedGraceMs, null));
    const verdict = evaluateWatchdog(history, CONFIG.neverStartedGraceMs, CONFIG);
    assert.ok(verdict, "must give up once the never-started grace elapses");
    assert.strictEqual(verdict?.kind, "never-started");
    // Promptly: well before the absolute ceiling, and before the no-progress
    // window too -- there is no progress evidence to have "not progressed".
    assert.ok(CONFIG.neverStartedGraceMs < CONFIG.noProgressWindowMs);
    assert.ok(CONFIG.neverStartedGraceMs < CONFIG.ceilingMs);
  });

  test("progress via `failed` advancing only (every test failing, but running) is not a stall", () => {
    const history: WatchdogObservation[] = [];
    let verdict = null;
    for (let atMs = 0, failed = 0; atMs <= 100_000; atMs += 2_000, failed++) {
      const h = beat({
        completed: failed,
        failed,
        inFlight: [{ title: `failing #${failed}`, forMs: 50 }],
      });
      history.push(obs(atMs, h));
      verdict = evaluateWatchdog(history, atMs, CONFIG);
      assert.strictEqual(
        verdict,
        null,
        `must treat rising failed count as progress (at ${atMs}ms)`,
      );
    }
  });

  test("an in-flight title changing with `completed` static is treated as progress", () => {
    // A single very long-running test suite (e.g. one slow test after
    // another, none individually finishing inside the sampling window) --
    // completed never moves, but the in-flight title does every tick.
    const titles = ["test A", "test B", "test C", "test D", "test E"];
    const history: WatchdogObservation[] = [];
    let verdict = null;
    for (let i = 0; i < titles.length; i++) {
      const atMs = i * 30_000; // less than the 90s window between each change
      const h = beat({ completed: 0, inFlight: [{ title: titles[i], forMs: 1_000 }] });
      history.push(obs(atMs, h));
      verdict = evaluateWatchdog(history, atMs, CONFIG);
      assert.strictEqual(
        verdict,
        null,
        `an in-flight title change must count as progress (at ${atMs}ms, title=${titles[i]})`,
      );
    }
  });

  test("the absolute ceiling fires while still progressing, with the ceiling verdict -- never 'likely hung'", () => {
    // completed keeps climbing (so the stall check never fires) but the run
    // simply takes longer than the generous ceiling allows.
    const history: WatchdogObservation[] = [];
    let verdict = null;
    for (let atMs = 0, completed = 0; atMs < CONFIG.ceilingMs; atMs += 2_000, completed++) {
      const h = beat({ completed, inFlight: [{ title: `test #${completed}`, forMs: 200 }] });
      history.push(obs(atMs, h));
      verdict = evaluateWatchdog(history, atMs, CONFIG);
      assert.strictEqual(verdict, null, `must not give up before the ceiling (at ${atMs}ms)`);
    }
    const finalCompleted = history.length;
    const lastHeartbeat = beat({
      completed: finalCompleted,
      inFlight: [{ title: `test #${finalCompleted}`, forMs: 200 }],
    });
    history.push(obs(CONFIG.ceilingMs, lastHeartbeat));
    verdict = evaluateWatchdog(history, CONFIG.ceilingMs, CONFIG);
    assert.ok(verdict, "must give up once the ceiling is reached");
    assert.strictEqual(verdict?.kind, "ceiling");
    if (verdict) {
      const message = describeWatchdogVerdict(verdict);
      assert.ok(
        !/likely hung/i.test(message),
        `a ceiling verdict while still progressing must not say "likely hung": ${message}`,
      );
      assert.ok(
        message.includes("absolute ceiling reached while still completing tests"),
        `ceiling wording must be explicit about which bound fired: ${message}`,
      );
    }
  });

  test("a stalled verdict never-started never applies once a heartbeat has ever been seen", () => {
    // A host that started, made progress, then genuinely wedged must be
    // reported as "stalled", not misclassified as "never-started" just
    // because the most recent poll happened to race a missing/corrupt read.
    const live = beat({ completed: 5, inFlight: [{ title: "ok", forMs: 100 }] });
    const history: WatchdogObservation[] = [obs(0, live), obs(2_000, live)];
    const noisyMiss = obs(4_000, null); // one dropped read shouldn't reset anything
    history.push(noisyMiss);
    const atMs = CONFIG.noProgressWindowMs + 4_000;
    history.push(obs(atMs, live));
    const verdict = evaluateWatchdog(history, atMs, CONFIG);
    assert.strictEqual(verdict?.kind, "stalled");
  });

  test("stalled verdict message names the in-flight test and progress counts", () => {
    const wedged = beat({
      completed: 12,
      failed: 2,
      inFlight: [{ title: "shimmerPrecision suite", forMs: 95_000 }],
    });
    const history: WatchdogObservation[] = [obs(0, wedged), obs(CONFIG.noProgressWindowMs, wedged)];
    const verdict = evaluateWatchdog(history, CONFIG.noProgressWindowMs, CONFIG);
    assert.strictEqual(verdict?.kind, "stalled");
    assert.ok(verdict, "expected a verdict");
    const message = describeWatchdogVerdict(verdict);
    assert.ok(message.includes("12 completed"), message);
    assert.ok(message.includes("2 failed"), message);
    assert.ok(message.includes("shimmerPrecision suite"), message);
    assert.ok(!/likely hung/i.test(message), message);
  });
});

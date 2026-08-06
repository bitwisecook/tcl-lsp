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

import * as fs from "fs";
import * as path from "path";
import Mocha from "mocha";
import * as vscode from "vscode";
import { glob } from "glob";
import { loadFactor, scaledTimeout } from "./signal";

// Register a require hook so tsc-compiled output can load .md files at runtime.
// (In production the esbuild bundle uses --loader:.md=text to inline them.)
require.extensions[".md"] = (mod: NodeJS.Module, filename: string) => {
  (mod as NodeJS.Module & { exports: unknown }).exports = fs.readFileSync(filename, "utf8");
};

/**
 * What the heartbeat file records, so a hang is attributable from outside the
 * extension host.
 *
 * The runner's launch-exit budget used to expire with nothing to say but a
 * process list and "mocha never completed (likely hung)" — which names neither
 * the test that stalled nor whether the server was still alive (issue #1274).
 * The extension host is the only place that knows both, so it writes them down
 * continuously and `runTest.ts` reads the last entry after the fact.
 */
interface Heartbeat {
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

/** How often the heartbeat is refreshed. */
const HEARTBEAT_INTERVAL_MS = 2_000;

/** How long a single test may run before the heartbeat starts probing the
 *  server on every tick. Below this a slow test is just a slow test. */
const SERVER_PROBE_AFTER_MS = 20_000;

/** Bound on the server probe itself, so the heartbeat can never become the
 *  thing that hangs. */
const SERVER_PROBE_TIMEOUT_MS = 5_000;

/**
 * Ask the language server a question that touches neither the document store
 * nor the analyser, so what it times is the client → server → client path
 * itself. Mirrors the native harness's `LatencyBudget` no-op probe.
 */
async function probeServer(): Promise<NonNullable<Heartbeat["server"]>> {
  const started = Date.now();
  try {
    const answered = await Promise.race([
      vscode.commands.executeCommand("tcl-lsp.getEffectiveConfig", "").then(() => true),
      new Promise<false>((resolve) =>
        setTimeout(() => resolve(false), scaledTimeout(SERVER_PROBE_TIMEOUT_MS)).unref(),
      ),
    ]);
    return answered
      ? { responsive: true, roundTripMs: Date.now() - started }
      : {
          responsive: false,
          reason: "getEffectiveConfig did not answer",
          afterMs: Date.now() - started,
        };
  } catch (err) {
    return {
      responsive: false,
      reason: err instanceof Error ? err.message : String(err),
      afterMs: Date.now() - started,
    };
  }
}

export async function run(): Promise<void> {
  // Written unconditionally when mocha's run() callback fires (pass or fail)
  // so runTest.ts's exit-timeout watchdog can tell "mocha finished cleanly,
  // the process just didn't exit" apart from "mocha never finished" (a hang).
  // Only the latter may be swallowed as success.
  const resultMarker = path.resolve(__dirname, "../../", ".vscode-test", "mocha-result.json");
  const heartbeatMarker = path.resolve(__dirname, "../../", ".vscode-test", "mocha-heartbeat.json");
  const mocha = new Mocha({
    ui: "tdd",
    color: true,
    // A per-test backstop, not a budget: every wait a test takes is itself
    // bounded and load-scaled (see `signal.ts`), so a test that reaches this
    // number has stalled somewhere with no bound of its own. Scaled by measured
    // load for the same reason those waits are — the shimmer tests in #1274 hit
    // a raw 60s each under ~9 concurrent build trees.
    timeout: scaledTimeout(60_000),
  });
  if (process.env.MOCHA_GREP) {
    mocha.grep(process.env.MOCHA_GREP);
  }

  const testsRoot = path.resolve(__dirname);

  // The config tests write workspace settings via `config.update(key, value,
  // undefined)`, which VS Code persists to the workspace fixture's
  // `.vscode/settings.json`.  Even though each test restores the *value*, VS
  // Code can rewrite the file with different whitespace/key-order than the
  // committed bytes, and a failing test can leave it dirty — so the tracked
  // fixture drifts.  That breaks `.test-slow.stamp` reproducibility (the
  // stamp hashes every tracked file and CI checks it against a clean
  // checkout).  Snapshot the exact bytes before the run and rewrite them
  // after — restoring the file regardless of which test touched it or
  // whether the run passed.
  const fixtureSettings = path.resolve(__dirname, "../../testFixture/.vscode/settings.json");
  const fixtureSnapshot = fs.existsSync(fixtureSettings) ? fs.readFileSync(fixtureSettings) : null;
  const restoreFixtureSettings = () => {
    if (fixtureSnapshot === null) return;
    try {
      if (!fs.readFileSync(fixtureSettings).equals(fixtureSnapshot)) {
        fs.writeFileSync(fixtureSettings, fixtureSnapshot);
      }
    } catch {
      // File was removed by a test — recreate it from the snapshot.
      fs.writeFileSync(fixtureSettings, fixtureSnapshot);
    }
  };

  // The multiFolder/ subdirectory has its own runner (runMultiFolderTest)
  // because those tests need the .code-workspace fixture.  Skip them here.
  const files = await glob("**/*.test.js", {
    cwd: testsRoot,
    ignore: ["multiFolder/**"],
  });
  files.sort();
  for (const f of files) {
    mocha.addFile(path.resolve(testsRoot, f));
  }

  fs.mkdirSync(path.dirname(heartbeatMarker), { recursive: true });
  try {
    fs.unlinkSync(heartbeatMarker);
  } catch {
    // No stale heartbeat to remove.
  }

  return new Promise<void>((resolve, reject) => {
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

    // Unref'd: the heartbeat must never be the reason the host stays alive
    // after the suite finishes.
    const heartbeat = setInterval(writeHeartbeat, HEARTBEAT_INTERVAL_MS);
    heartbeat.unref();

    const runner = mocha.run((failures) => {
      clearInterval(heartbeat);
      restoreFixtureSettings();
      fs.mkdirSync(path.dirname(resultMarker), { recursive: true });
      fs.writeFileSync(resultMarker, JSON.stringify({ failures }) + "\n", "utf8");
      if (failures > 0) {
        reject(new Error(`${failures} test(s) failed.`));
      } else {
        resolve();
      }
    });

    runner.on("test", (test: Mocha.Test) => {
      inFlight.set(test.fullTitle(), Date.now());
      writeHeartbeat();
    });
    runner.on("test end", (test: Mocha.Test) => {
      inFlight.delete(test.fullTitle());
      completed++;
    });
    // Log failure details so they are visible even when the VS Code test
    // host terminates before mocha prints its summary.
    runner.on("fail", (test: Mocha.Test, err: Error) => {
      failed++;
      console.error(`\nFAIL: ${test.fullTitle()}`);
      console.error(`  ${err.message}`);
      if (err.stack) {
        const firstFrame = err.stack.split("\n").slice(1, 3).join("\n");
        console.error(firstFrame);
      }
    });
  });
}

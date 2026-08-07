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

import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import * as crypto from "crypto";
import { execSync } from "child_process";
import { runTests } from "@vscode/test-electron";
import { loadFactor, scaledTimeout } from "./signal";

const DEFAULT_EXIT_TIMEOUT_MS = 180_000;

/** Mirror of the `Heartbeat` the extension host writes in `index.ts`. */
interface Heartbeat {
  at: number;
  inFlight: Array<{ title: string; forMs: number }>;
  completed: number;
  failed: number;
  server:
    | null
    | { responsive: true; roundTripMs: number }
    | { responsive: false; reason: string; afterMs: number };
  loadFactor: number;
}

/**
 * Turn "mocha never completed" into an attributable report: which tests were in
 * flight, for how long, whether the language server was still answering, and
 * whether the extension host's own event loop was still turning.
 *
 * Without this the launch-exit budget expiring printed a process list and a
 * guess, which is what made issue #1274 need re-run archaeology rather than
 * reading a log.
 */
function reportHang(heartbeatMarker: string): void {
  let beat: Heartbeat;
  try {
    beat = JSON.parse(fs.readFileSync(heartbeatMarker, "utf8"));
  } catch {
    console.error(
      "No heartbeat was ever written, so the suite hung before or during its first test " +
        "(extension host failed to start, or `run()` never reached mocha.run).",
    );
    return;
  }

  const age = Date.now() - beat.at;
  console.error("--- hang report (from the extension host's heartbeat) ---");
  console.error(
    `  heartbeat written ${age}ms ago; ${beat.completed} test(s) completed, ` +
      `${beat.failed} failed; measured load factor ${beat.loadFactor.toFixed(1)}`,
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
  if (age > 3 * 2_000) {
    console.error(
      `  the heartbeat itself is ${age}ms stale (it refreshes every 2000ms), so the ` +
        "extension host's event loop stopped turning — a blocked main thread or a " +
        "crashed host, not a test waiting on a promise.",
    );
  }
  console.error("--- end hang report ---");
}

function parseExitTimeoutMs(): number {
  const raw = process.env.TCL_LSP_VSCODE_TEST_EXIT_TIMEOUT_MS;
  if (!raw) {
    return DEFAULT_EXIT_TIMEOUT_MS;
  }
  const parsed = Number(raw);
  if (!Number.isFinite(parsed) || parsed < 0) {
    return DEFAULT_EXIT_TIMEOUT_MS;
  }
  return Math.floor(parsed);
}

function escapeDoubleQuotes(text: string): string {
  return text.split('"').join('\\"');
}

function escapeSingleQuotes(text: string): string {
  return text.split("'").join("'\"'\"'");
}

function emitProcessSnapshot(extensionDevelopmentPath: string, extensionTestsPath: string): void {
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

function cleanupStaleTestHosts(extensionDevelopmentPath: string, extensionTestsPath: string): void {
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

async function main() {
  const extensionDevelopmentPath = path.resolve(__dirname, "../../");
  const extensionTestsPath = path.resolve(__dirname, "./index");
  // index.ts writes this unconditionally when mocha's run() callback fires,
  // pass or fail — its *absence* after the exit-timeout watchdog fires below
  // means mocha never finished at all (a hang), which is never safe to treat
  // as success.
  const resultMarker = path.resolve(extensionDevelopmentPath, ".vscode-test", "mocha-result.json");
  // index.ts refreshes this every couple of seconds with the in-flight tests and
  // a server-liveness probe, so an expiring exit budget below can say *what* was
  // stuck rather than only that something was.
  const heartbeatMarker = path.resolve(
    extensionDevelopmentPath,
    ".vscode-test",
    "mocha-heartbeat.json",
  );
  cleanupStaleTestHosts(extensionDevelopmentPath, extensionTestsPath);
  for (const marker of [resultMarker, heartbeatMarker]) {
    try {
      fs.unlinkSync(marker);
    } catch {
      // No stale marker to remove.
    }
  }

  // The user-data dir's IPC lock file is a Unix domain socket
  // (`<dir>/<version>-main.sock`), whose path is capped at ~107 bytes by
  // `sun_path` — comfortably long for a normal checkout, but a checkout
  // nested several levels deep (e.g. an isolated agent worktree under
  // `.claude/worktrees/<id>/`) can push
  // `<extensionDevelopmentPath>/.vscode-test/user-data/<ver>-main.sock`
  // over that limit, which fails as `EINVAL: listen` with no useful
  // message pointing at the real cause. Keep the user-data dir under the
  // system temp dir instead — short regardless of how deeply this checkout
  // is nested — but derive its name from `extensionDevelopmentPath` so
  // repeated runs against the *same* checkout keep reusing (and clearing)
  // the same directory rather than leaking a fresh one every time, while
  // a different checkout (a different worktree) gets its own.
  const checkoutId = crypto
    .createHash("sha1")
    .update(extensionDevelopmentPath)
    .digest("hex")
    .slice(0, 12);
  const userDataDir = path.join(os.tmpdir(), `tcl-lsp-vscode-test-${checkoutId}`, "user-data");
  // Clear persisted user settings from prior test runs so tests start
  // with a clean slate.  Settings modified via workspace.getConfiguration
  // .update() persist in the user-data directory and can pollute
  // subsequent runs.
  const userSettingsFile = path.resolve(userDataDir, "User", "settings.json");
  try {
    const { writeFileSync, mkdirSync } = require("fs");
    mkdirSync(path.dirname(userSettingsFile), { recursive: true });
    writeFileSync(userSettingsFile, "{}\n", "utf8");
  } catch {
    // Best-effort; if we can't clear it the tests may see stale config.
  }

  try {
    // The workspace to open during tests
    const testWorkspace = path.resolve(extensionDevelopmentPath, "testFixture");

    // The launch-exit budget is a hang backstop, so it is scaled by measured
    // machine load for the same reason every wait inside the suite is: on a
    // container sharing itself with several build trees, a correct run is
    // simply slower, and killing it manufactures a failure. See signal.ts.
    const baseTimeoutMs = parseExitTimeoutMs();
    const timeoutMs = scaledTimeout(baseTimeoutMs);
    if (timeoutMs !== baseTimeoutMs) {
      console.log(
        `==> launch-exit budget ${timeoutMs}ms (base ${baseTimeoutMs}ms x measured load ` +
          `factor ${loadFactor().toFixed(1)})`,
      );
    }
    const runPromise = runTests({
      extensionDevelopmentPath,
      extensionTestsPath,
      launchArgs: [
        testWorkspace,
        "--disable-extensions", // Disable other extensions
        `--user-data-dir=${userDataDir}`,
      ],
    });

    if (timeoutMs <= 0) {
      await runPromise;
      return;
    }

    const timeoutPromise = new Promise<never>((_, reject) => {
      setTimeout(() => {
        reject(new Error(`VS Code test runner did not exit within ${timeoutMs}ms after launch.`));
      }, timeoutMs).unref();
    });

    await Promise.race([runPromise, timeoutPromise]);
    cleanupStaleTestHosts(extensionDevelopmentPath, extensionTestsPath);
    process.exit(0);
  } catch (err) {
    const timedOut = err instanceof Error && err.message.includes("did not exit within");
    if (timedOut) {
      // Read the heartbeat *before* the cleanup below kills the host that
      // writes it, so the age reported is the age at the moment of giving up.
      reportHang(heartbeatMarker);
    }
    emitProcessSnapshot(extensionDevelopmentPath, extensionTestsPath);
    cleanupStaleTestHosts(extensionDevelopmentPath, extensionTestsPath);
    if (timedOut && err instanceof Error) {
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
        }
        console.error(
          `VS Code tests reported ${failures ?? "an unknown number of"} failure(s) before the runner timeout cleanup.`,
        );
        process.exit(1);
      }
      // No marker at all means mocha's run() callback never fired -- the
      // suite was still mid-run (hung, or just slower than the watchdog)
      // when we killed it. That is never safe to report as success: it has
      // silently skipped every test after the point it got stuck.
      console.error(`${err.message} -- mocha never completed (likely hung). Treating as failure.`);
      process.exit(1);
    }
    console.error("Failed to run tests:", err);
    process.exit(1);
  }
}

main();

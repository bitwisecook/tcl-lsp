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
import { glob } from "glob";
import { scaledTimeout } from "./signal";
import { createHeartbeatWriter, MOCHA_TEST_TIMEOUT_BASE_MS } from "./runnerWatchdog";
import { probeServer } from "./serverProbe";
import { serverTransportWedged } from "./helper";

// Register a require hook so tsc-compiled output can load .md files at runtime.
// (In production the esbuild bundle uses --loader:.md=text to inline them.)
require.extensions[".md"] = (mod: NodeJS.Module, filename: string) => {
  (mod as NodeJS.Module & { exports: unknown }).exports = fs.readFileSync(filename, "utf8");
};

export async function run(): Promise<void> {
  // Written unconditionally when mocha's run() callback fires (pass or fail)
  // so runTest.ts's watchdog (runnerWatchdog.ts) can tell "mocha finished
  // cleanly, the process just didn't exit" apart from "mocha never finished"
  // (a hang). Only the latter may be swallowed as success.
  const resultMarker = path.resolve(__dirname, "../../", ".vscode-test", "mocha-result.json");
  const heartbeatMarker = path.resolve(__dirname, "../../", ".vscode-test", "mocha-heartbeat.json");
  const mocha = new Mocha({
    ui: "tdd",
    color: true,
    // A per-test backstop, not a budget: every wait a test takes is itself
    // bounded and load-scaled (see `signal.ts`), so a test that reaches this
    // number has stalled somewhere with no bound of its own. Scaled by measured
    // load for the same reason those waits are — the shimmer tests in #1274 hit
    // a raw 60s each under ~9 concurrent build trees. `runnerWatchdog.ts`'s
    // no-progress window is itself derived from this same constant, so the
    // two cannot drift out of the relationship it depends on.
    timeout: scaledTimeout(MOCHA_TEST_TIMEOUT_BASE_MS),
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

  // Once a liveness probe has confirmed the server answers nothing at all, the
  // remaining tests cannot pass and cannot learn anything new — they can only
  // each burn a full wait budget rediscovering it. Skip them instead, so a
  // wedged run reports in about the time a healthy one takes rather than
  // grinding to the watchdog's absolute ceiling (issue #1294; the second
  // occurrence spent ~27 of its 32 minutes this way).
  //
  // A skip, not a bail: the failure that *did* diagnose the wedge stays in the
  // report as a failure, and the skipped count makes the lost coverage visible
  // instead of silently shrinking the suite.
  mocha.suite.beforeEach(function skipWhenServerWedged(this: Mocha.Context) {
    if (serverTransportWedged()) {
      this.skip();
    }
  });

  const heartbeatWriter = createHeartbeatWriter({ heartbeatMarker, probeServer });

  return new Promise<void>((resolve, reject) => {
    const runner = mocha.run((failures) => {
      heartbeatWriter.stop();
      restoreFixtureSettings();
      fs.mkdirSync(path.dirname(resultMarker), { recursive: true });
      fs.writeFileSync(resultMarker, JSON.stringify({ failures }) + "\n", "utf8");
      if (failures > 0) {
        reject(new Error(`${failures} test(s) failed.`));
      } else {
        resolve();
      }
    });

    runner.on("test", (test: Mocha.Test) => heartbeatWriter.onTestStart(test.fullTitle()));
    runner.on("test end", (test: Mocha.Test) => heartbeatWriter.onTestEnd(test.fullTitle()));
    // Log failure details so they are visible even when the VS Code test
    // host terminates before mocha prints its summary.
    runner.on("fail", (test: Mocha.Test, err: Error) => {
      heartbeatWriter.onFail();
      console.error(`\nFAIL: ${test.fullTitle()}`);
      console.error(`  ${err.message}`);
      if (err.stack) {
        const firstFrame = err.stack.split("\n").slice(1, 3).join("\n");
        console.error(firstFrame);
      }
    });
  });
}

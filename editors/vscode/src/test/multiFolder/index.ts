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

// Mocha entry point for the multi-folder workspace test suite.
// Picks up only ``*.test.js`` files under ``out/test/multiFolder/``.
import * as fs from "fs";
import * as path from "path";
import Mocha from "mocha";
import { glob } from "glob";
import { beginTestDeadline, scaledTimeout } from "../signal";
import { createHeartbeatWriter, MOCHA_TEST_TIMEOUT_BASE_MS } from "../runnerWatchdog";
import { probeServer } from "../serverProbe";

export async function run(): Promise<void> {
  // Written unconditionally when mocha's run() callback fires (pass or fail),
  // and refreshed every couple of seconds while it runs — the same contract
  // `index.ts` gives the single-folder suite (see `runnerWatchdog.ts`). This
  // suite used to write neither, so its own runner (`runMultiFolderTest.ts`)
  // had no progress evidence and — worse — treated its own launch-exit
  // timeout as success, which could silently pass a hung multi-folder run.
  const resultMarker = path.resolve(
    __dirname,
    "../../../",
    ".vscode-test",
    "mocha-result-multifolder.json",
  );
  const heartbeatMarker = path.resolve(
    __dirname,
    "../../../",
    ".vscode-test",
    "mocha-heartbeat-multifolder.json",
  );
  const mocha = new Mocha({
    ui: "tdd",
    color: true,
    // Same backstop and provenance as the single-folder suite's — see
    // `index.ts`'s matching comment.
    timeout: scaledTimeout(MOCHA_TEST_TIMEOUT_BASE_MS),
  });

  // The follow-up diagnostics in `signal.ts` are load-scaled at the moment
  // they run, while this `timeout:` was fixed above under whatever load
  // existed at suite construction — so without a tie the inner budget can
  // outgrow the outer bound and the diagnostic becomes the thing that hangs.
  // Recording the effective per-test deadline here is that tie; the clamp
  // lives in `diagnosticBudget()`.
  mocha.suite.beforeEach(function (this: Mocha.Context) {
    beginTestDeadline(this.timeout());
  });
  // Dropped on the way out for the same reason as `index.ts`'s — see there.
  mocha.suite.afterEach(() => {
    beginTestDeadline(0);
  });

  const testsRoot = path.resolve(__dirname);
  const files = await glob("**/*.test.js", { cwd: testsRoot });
  files.sort();
  for (const f of files) {
    mocha.addFile(path.resolve(testsRoot, f));
  }

  const heartbeatWriter = createHeartbeatWriter({ heartbeatMarker, probeServer });

  return new Promise<void>((resolve, reject) => {
    const runner = mocha.run((failures) => {
      heartbeatWriter.stop();
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

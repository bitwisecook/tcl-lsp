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
import { runWatchedSuite } from "./runnerWatchdog";
import { createTestUserDataDir } from "./testUserDataDir";

async function main() {
  const extensionDevelopmentPath = path.resolve(__dirname, "../../");
  const extensionTestsPath = path.resolve(__dirname, "./index");
  // index.ts writes this unconditionally when mocha's run() callback fires,
  // pass or fail — its *absence* after the watchdog gives up (see
  // runnerWatchdog.ts) means mocha never finished at all (a hang), which is
  // never safe to treat as success.
  const resultMarker = path.resolve(extensionDevelopmentPath, ".vscode-test", "mocha-result.json");
  // index.ts refreshes this every couple of seconds with the in-flight tests and
  // a server-liveness probe, so the watchdog can say *what* was stuck (or that
  // nothing was) rather than only that something was.
  const heartbeatMarker = path.resolve(
    extensionDevelopmentPath,
    ".vscode-test",
    "mocha-heartbeat.json",
  );

  // Kept out of the checkout and short enough for the IPC socket's `sun_path`
  // budget — see createTestUserDataDir for the arithmetic. It is private to
  // this run, so no setting a test writes through
  // `workspace.getConfiguration().update()` can pollute the next one.
  const userDataDir = createTestUserDataDir();

  // The workspace to open during tests
  const testWorkspace = path.resolve(extensionDevelopmentPath, "testFixture");

  await runWatchedSuite({
    extensionDevelopmentPath,
    extensionTestsPath,
    launchArgs: [
      testWorkspace,
      "--disable-extensions", // Disable other extensions
      `--user-data-dir=${userDataDir}`,
    ],
    heartbeatMarker,
    resultMarker,
  });
}

main();

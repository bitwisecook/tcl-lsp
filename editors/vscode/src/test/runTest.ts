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
import { runWatchedSuite } from "./runnerWatchdog";

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
    fs.mkdirSync(path.dirname(userSettingsFile), { recursive: true });
    fs.writeFileSync(userSettingsFile, "{}\n", "utf8");
  } catch {
    // Best-effort; if we can't clear it the tests may see stale config.
  }

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

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
import { execSync } from "child_process";
import { runTests } from "@vscode/test-electron";

const DEFAULT_EXIT_TIMEOUT_MS = 180_000;

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
  cleanupStaleTestHosts(extensionDevelopmentPath, extensionTestsPath);
  try {
    fs.unlinkSync(resultMarker);
  } catch {
    // No stale marker to remove.
  }

  // Clear persisted user settings from prior test runs so tests start
  // with a clean slate.  Settings modified via workspace.getConfiguration
  // .update() persist in the user-data directory and can pollute
  // subsequent runs.
  const userDataDir = path.resolve(extensionDevelopmentPath, ".vscode-test", "user-data");
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

    const timeoutMs = parseExitTimeoutMs();
    const runPromise = runTests({
      extensionDevelopmentPath,
      extensionTestsPath,
      launchArgs: [
        testWorkspace,
        "--disable-extensions", // Disable other extensions
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
    emitProcessSnapshot(extensionDevelopmentPath, extensionTestsPath);
    cleanupStaleTestHosts(extensionDevelopmentPath, extensionTestsPath);
    if (err instanceof Error && err.message.includes("did not exit within")) {
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

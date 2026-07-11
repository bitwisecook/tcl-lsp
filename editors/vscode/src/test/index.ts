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

// Register a require hook so tsc-compiled output can load .md files at runtime.
// (In production the esbuild bundle uses --loader:.md=text to inline them.)
require.extensions[".md"] = (mod: NodeJS.Module, filename: string) => {
  (mod as NodeJS.Module & { exports: unknown }).exports = fs.readFileSync(filename, "utf8");
};

export async function run(): Promise<void> {
  // Written unconditionally when mocha's run() callback fires (pass or fail)
  // so runTest.ts's exit-timeout watchdog can tell "mocha finished cleanly,
  // the process just didn't exit" apart from "mocha never finished" (a hang).
  // Only the latter may be swallowed as success.
  const resultMarker = path.resolve(__dirname, "../../", ".vscode-test", "mocha-result.json");
  const mocha = new Mocha({
    ui: "tdd",
    color: true,
    timeout: 60_000, // LSP startup can be slow
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

  return new Promise<void>((resolve, reject) => {
    const runner = mocha.run((failures) => {
      restoreFixtureSettings();
      fs.mkdirSync(path.dirname(resultMarker), { recursive: true });
      fs.writeFileSync(resultMarker, JSON.stringify({ failures }) + "\n", "utf8");
      if (failures > 0) {
        reject(new Error(`${failures} test(s) failed.`));
      } else {
        resolve();
      }
    });
    // Log failure details so they are visible even when the VS Code test
    // host terminates before mocha prints its summary.
    runner.on("fail", (test: Mocha.Test, err: Error) => {
      console.error(`\nFAIL: ${test.fullTitle()}`);
      console.error(`  ${err.message}`);
      if (err.stack) {
        const firstFrame = err.stack.split("\n").slice(1, 3).join("\n");
        console.error(firstFrame);
      }
    });
  });
}

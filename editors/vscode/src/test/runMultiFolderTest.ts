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

// Variant of runTest.ts that opens the multi-root workspace fixture so
// per-folder configuration tests can verify VS Code accepts and applies
// folder-level tclLsp.* settings (issue #230).
import * as path from "path";
import * as fs from "fs";
import { runWatchedSuite } from "./runnerWatchdog";

async function main() {
  const extensionDevelopmentPath = path.resolve(__dirname, "../../");
  const extensionTestsPath = path.resolve(__dirname, "./multiFolder/index");
  // multiFolder/index.ts writes these the same way index.ts does for the
  // single-folder suite (see runnerWatchdog.ts's createHeartbeatWriter) —
  // distinct filenames so a full `make test-ext` run (both suites,
  // sequentially) never has one runner's markers mistaken for the other's.
  const resultMarker = path.resolve(
    extensionDevelopmentPath,
    ".vscode-test",
    "mocha-result-multifolder.json",
  );
  const heartbeatMarker = path.resolve(
    extensionDevelopmentPath,
    ".vscode-test",
    "mocha-heartbeat-multifolder.json",
  );

  // Open the multi-root .code-workspace fixture so VS Code is in
  // multi-folder mode and folder-level .vscode/settings.json files are
  // honoured.
  const codeWorkspace = path.resolve(
    extensionDevelopmentPath,
    "testFixtureMultiFolder",
    "multiFolder.code-workspace",
  );

  // Clear persisted user settings between runs.
  const userDataDir = path.resolve(extensionDevelopmentPath, ".vscode-test", "user-data");
  const userSettingsFile = path.resolve(userDataDir, "User", "settings.json");
  try {
    fs.mkdirSync(path.dirname(userSettingsFile), { recursive: true });
    fs.writeFileSync(userSettingsFile, "{}\n", "utf8");
  } catch {
    /* best-effort */
  }

  // Materialise the per-folder ``.vscode/settings.json`` fixtures.  These
  // are gitignored (the repo-wide ``.vscode/`` rule), so they have to be
  // re-created on every test run.  Folder A and Folder B carry different
  // ``tclLsp.formatting.maxLineLength`` (issue #230), ``tclLsp.dialect``
  // (issue #407), ``tclLsp.style.nonAscii``, and ``tclLsp.diagnostics.W111``
  // values to exercise the per-folder resolution path.
  try {
    const projA = path.resolve(
      extensionDevelopmentPath,
      "testFixtureMultiFolder",
      "proj-a",
      ".vscode",
    );
    const projB = path.resolve(
      extensionDevelopmentPath,
      "testFixtureMultiFolder",
      "proj-b",
      ".vscode",
    );
    fs.mkdirSync(projA, { recursive: true });
    fs.mkdirSync(projB, { recursive: true });
    fs.writeFileSync(
      path.resolve(projA, "settings.json"),
      JSON.stringify(
        {
          "tclLsp.formatting.maxLineLength": 160,
          "tclLsp.diagnostics.W111": false,
          "tclLsp.dialect": "tcl8.4",
          "tclLsp.style.nonAscii": "strict",
          "tclLsp.extraCommands": ["folder-a-helper", "shared-util"],
          "tclLsp.libraryPaths": ["/opt/proj-a/tcl-lib"],
        },
        null,
        2,
      ) + "\n",
      "utf8",
    );
    fs.writeFileSync(
      path.resolve(projB, "settings.json"),
      JSON.stringify(
        {
          "tclLsp.formatting.maxLineLength": 60,
          "tclLsp.dialect": "f5-irules",
          "tclLsp.style.nonAscii": "off",
          "tclLsp.extraCommands": ["folder-b-helper"],
          "tclLsp.libraryPaths": ["/opt/proj-b/tcl-lib", "/usr/local/share/tcl"],
        },
        null,
        2,
      ) + "\n",
      "utf8",
    );
    // The per-folder diagnostic tests each drive their own scratch file
    // (``extra.tcl`` for the ``extraCommands`` W123 check, ``race.tcl`` for the
    // dialect-change-after-open check).  They replace the contents through the
    // editor, but ``workspace.openTextDocument(Uri.file(...))`` still needs the
    // path to exist, so materialise them here alongside the settings.  Nothing
    // reads the seed text.
    for (const folder of ["proj-a", "proj-b"]) {
      for (const name of ["extra.tcl", "race.tcl"]) {
        fs.writeFileSync(
          path.resolve(extensionDevelopmentPath, "testFixtureMultiFolder", folder, name),
          "# placeholder — each test replaces this through the editor\n",
          "utf8",
        );
      }
    }
  } catch {
    /* best-effort */
  }

  await runWatchedSuite({
    extensionDevelopmentPath,
    extensionTestsPath,
    launchArgs: [codeWorkspace, "--disable-extensions"],
    heartbeatMarker,
    resultMarker,
  });
}

main();

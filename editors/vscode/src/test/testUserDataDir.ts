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

// Where the VS Code test hosts put their `--user-data-dir`.
//
// Shared by runTest.ts (single-root) and runMultiFolderTest.ts (multi-root) so
// the `sun_path` arithmetic below is stated once and neither suite can drift
// back onto a path that blows the cap.

import * as fs from "fs";
import * as path from "path";
import * as os from "os";

// A Unix domain socket path is capped at 103 bytes (`sun_path`, minus the NUL).
const SUN_PATH_MAX = 103;

// The longest socket name VS Code creates inside the user-data dir:
// `<major>.<minor>-main.sock`.
const LONGEST_SOCKET_NAME = "1.234-main.sock";

// `mkdtempSync` appends six random characters to this.
const DIR_PREFIX = "tcl-lsp-vsc-";

/**
 * Create a user-data dir for a VS Code test host.
 *
 * `mkdtempSync` creates it atomically, mode 0700, under a name nobody can
 * predict, so another user of the shared temp dir can neither pre-create the
 * path nor read the settings and workspace state the host writes into it. The
 * directory is removed again when the runner exits, so a run leaves nothing
 * behind and no state carries into the next one.
 *
 * VS Code puts its IPC lock socket at `<user-data-dir>/<version>-main.sock`, so
 * the whole directory path has to leave room under the 103-byte `sun_path` cap.
 * Two things eat that budget:
 *
 *   - A checkout nested several levels deep — an isolated agent worktree under
 *     `.claude/worktrees/<id>/`, say — blows the cap on its own if the dir sits
 *     inside the checkout (test-electron's default). It surfaces as a bare
 *     `EINVAL: listen` naming the socket and nothing else. Hence the system
 *     temp dir, which stays short however deeply this checkout is nested.
 *
 *   - macOS makes `os.tmpdir()` itself a ~48-char per-user path
 *     (`/var/folders/xx/<30 chars>/T`), which leaves ~55 bytes for everything
 *     below it — so the name has to be one short segment, not
 *     `tcl-lsp-vscode-test-<12 hex>/user-data` (that overshot by itself).
 *
 * @throws if the resulting socket path would still exceed the cap — saying so
 * plainly beats leaving Electron's `EINVAL` to be decoded again.
 */
export function createTestUserDataDir(): string {
  // The probe is the exact length `mkdtempSync` will produce, so the budget is
  // checked before anything is created.
  const probeSocket = path.join(os.tmpdir(), `${DIR_PREFIX}XXXXXX`, LONGEST_SOCKET_NAME);
  if (probeSocket.length > SUN_PATH_MAX) {
    throw new Error(
      `VS Code's IPC socket path would be ${probeSocket.length} bytes, over the ` +
        `${SUN_PATH_MAX}-byte sun_path limit: ${probeSocket}\n` +
        `Set TMPDIR to a shorter directory before running the extension tests.`,
    );
  }

  const userDataDir = fs.mkdtempSync(path.join(os.tmpdir(), DIR_PREFIX));
  // Both runners finish inside `process.exit`, so this is the only hook left to
  // clean up in.
  process.on("exit", () => {
    try {
      fs.rmSync(userDataDir, { recursive: true, force: true });
    } catch (err) {
      console.warn(`Could not remove the test user-data dir ${userDataDir}:`, err);
    }
  });
  return userDataDir;
}

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

/**
 * Entry point loaded inside VS Code by @vscode/test-electron for the
 * screenshot demo.  Instead of running Mocha tests, it activates the
 * extension, cleans up the UI for clean screenshots, and fires the
 * screenshot demo command.
 */

import * as vscode from "vscode";

export async function run(): Promise<void> {
  // Activate the extension (which registers the demo command when
  // __SCREENSHOT_MODE__ is true).
  const ext = vscode.extensions.getExtension("bitwisecook.tcl-lsp");
  if (ext && !ext.isActive) {
    await ext.activate();
  }

  // Give the LSP server a moment to fully start.
  await new Promise((resolve) => setTimeout(resolve, 1000));

  // Clear stale notifications but leave sidebars open so the user can
  // interact with Copilot sign-in before capture starts.
  try {
    await vscode.commands.executeCommand("notifications.clearAll");
  } catch {}

  await new Promise((resolve) => setTimeout(resolve, 300));

  // Run the demo — it handles sign-in prompts and UI cleanup internally.
  await vscode.commands.executeCommand("tclLsp.runScreenshotDemo");
}

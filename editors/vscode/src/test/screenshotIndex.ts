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

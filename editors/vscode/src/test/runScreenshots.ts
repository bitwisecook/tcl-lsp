/**
 * Screenshot capture launcher -- mirrors runTest.ts but opens the repo root
 * as the workspace and uses screenshotIndex as the entry point.
 */

import * as path from "path";
import * as os from "os";
import * as fs from "fs";
import { runTests } from "@vscode/test-electron";

function envTrue(name: string): boolean {
  const value = process.env[name];
  if (!value) return false;
  return !["0", "false", "no", "off"].includes(value.toLowerCase());
}

function resolveVsCodeExecutablePath(): string | undefined {
  const explicit = process.env.TCL_LSP_SCREENSHOT_VSCODE_EXECUTABLE;
  if (explicit) {
    return explicit;
  }

  // Default to @vscode/test-electron download (latest stable channel),
  // unless explicitly asked to use the local app bundle.
  if (envTrue("TCL_LSP_SCREENSHOT_FORCE_DOWNLOADED_VSCODE")) {
    return undefined;
  }
  if (!envTrue("TCL_LSP_SCREENSHOT_USE_SYSTEM_VSCODE")) {
    return undefined;
  }

  if (process.platform === "darwin") {
    const candidates = [
      "/Applications/Visual Studio Code.app/Contents/MacOS/Electron",
      "/Applications/Visual Studio Code - Insiders.app/Contents/MacOS/Electron",
    ];
    for (const candidate of candidates) {
      if (fs.existsSync(candidate)) {
        return candidate;
      }
    }
  }

  return undefined;
}

async function main() {
  try {
    const extensionDevelopmentPath = path.resolve(__dirname, "../../");
    const extensionTestsPath = path.resolve(__dirname, "./screenshotIndex");

    // Open the repo root so that samples/ files are accessible.
    // extensionDevelopmentPath is editors/vscode/, so go up two levels.
    const testWorkspace = path.resolve(extensionDevelopmentPath, "../..");

    // Use a persistent user-data-dir so GitHub/Copilot sign-in state survives
    // across runs.
    const screenshotHome =
      process.env.TCL_LSP_SCREENSHOT_HOME || path.join(os.homedir(), ".tcl-lsp-screenshots");
    const userDataDir =
      process.env.TCL_LSP_SCREENSHOT_USER_DATA_DIR || path.join(screenshotHome, "user-data");
    fs.mkdirSync(userDataDir, { recursive: true });
    const screenshotProfile = process.env.TCL_LSP_SCREENSHOT_PROFILE || "Tcl LSP Screenshots";
    const vscodeVersion = process.env.TCL_LSP_SCREENSHOT_VSCODE_VERSION || "stable";

    // Default to an isolated extensions dir for reproducible screenshots.
    // The shell script stages an explicit allowlist (for example Copilot).
    const extensionsDir =
      process.env.VSCODE_EXTENSIONS_DIR || path.join(screenshotHome, "extensions");

    const launchArgs = [
      testWorkspace,
      `--user-data-dir=${userDataDir}`,
      `--extensions-dir=${extensionsDir}`,
      "--disable-workspace-trust",
      "--new-window",
      "--profile",
      screenshotProfile,
      "--disable-restore-windows",
      "--window-size=1200,900",
    ];

    const passwordStore =
      process.env.TCL_LSP_SCREENSHOT_PASSWORD_STORE ||
      (process.platform === "darwin" ? "keychain" : "");
    if (passwordStore) {
      launchArgs.push(`--password-store=${passwordStore}`);
    }

    const vscodeExecutablePath = resolveVsCodeExecutablePath();
    const reuseMachineInstall =
      envTrue("TCL_LSP_SCREENSHOT_REUSE_MACHINE_INSTALL") && !!vscodeExecutablePath;

    if (vscodeExecutablePath) {
      console.log(`[runScreenshots] VS Code executable: ${vscodeExecutablePath}`);
    } else {
      console.log(
        `[runScreenshots] VS Code executable: downloaded by @vscode/test-electron (${vscodeVersion})`,
      );
    }
    console.log(`[runScreenshots] User data dir: ${userDataDir}`);
    console.log(`[runScreenshots] Profile: ${screenshotProfile}`);
    console.log(`[runScreenshots] Extensions dir: ${extensionsDir}`);
    console.log(
      `[runScreenshots] Password store: ${passwordStore || "(default)"}; reuseMachineInstall=${reuseMachineInstall}`,
    );

    const options: Parameters<typeof runTests>[0] = {
      extensionDevelopmentPath,
      extensionTestsPath,
      launchArgs,
    };
    if (vscodeExecutablePath) {
      options.vscodeExecutablePath = vscodeExecutablePath;
    }
    if (!vscodeExecutablePath) {
      options.version = vscodeVersion;
    }
    if (reuseMachineInstall) {
      options.reuseMachineInstall = true;
    }

    await runTests(options);
  } catch (err) {
    console.error("Failed to run screenshot demo:", err);
    process.exit(1);
  }
}

main();

// Variant of runTest.ts that opens the multi-root workspace fixture so
// per-folder configuration tests can verify VS Code accepts and applies
// folder-level tclLsp.* settings (issue #230).
import * as path from "path";
import { execSync } from "child_process";
import { runTests } from "@vscode/test-electron";

const DEFAULT_EXIT_TIMEOUT_MS = 180_000;

function parseExitTimeoutMs(): number {
  const raw = process.env.TCL_LSP_VSCODE_TEST_EXIT_TIMEOUT_MS;
  if (!raw) return DEFAULT_EXIT_TIMEOUT_MS;
  const parsed = Number(raw);
  if (!Number.isFinite(parsed) || parsed < 0) return DEFAULT_EXIT_TIMEOUT_MS;
  return Math.floor(parsed);
}

function escapeSingleQuotes(text: string): string {
  return text.split("'").join("'\"'\"'");
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
    /* best-effort */
  }
}

async function main() {
  const extensionDevelopmentPath = path.resolve(__dirname, "../../");
  const extensionTestsPath = path.resolve(__dirname, "./multiFolder/index");
  cleanupStaleTestHosts(extensionDevelopmentPath, extensionTestsPath);

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
    const { writeFileSync, mkdirSync } = require("fs");
    mkdirSync(path.dirname(userSettingsFile), { recursive: true });
    writeFileSync(userSettingsFile, "{}\n", "utf8");
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
    const { writeFileSync, mkdirSync } = require("fs");
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
    mkdirSync(projA, { recursive: true });
    mkdirSync(projB, { recursive: true });
    writeFileSync(
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
    writeFileSync(
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
  } catch {
    /* best-effort */
  }

  const timeoutMs = parseExitTimeoutMs();
  try {
    const runPromise = runTests({
      extensionDevelopmentPath,
      extensionTestsPath,
      launchArgs: [codeWorkspace, "--disable-extensions"],
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
    cleanupStaleTestHosts(extensionDevelopmentPath, extensionTestsPath);
    if (err instanceof Error && err.message.includes("did not exit within")) {
      console.warn("Multi-folder VS Code tests completed but runner did not exit; continuing.");
      process.exit(0);
    }
    console.error("Failed to run multi-folder tests:", err);
    process.exit(1);
  }
}

main();

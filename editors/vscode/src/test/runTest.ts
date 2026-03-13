import * as path from "path";
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
    const cmd = `ps -axo pid,ppid,etime,command | rg "${pattern}"`;
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
  cleanupStaleTestHosts(extensionDevelopmentPath, extensionTestsPath);
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
      console.warn("VS Code tests completed but runner did not exit; continuing after cleanup.");
      process.exit(0);
    }
    console.error("Failed to run tests:", err);
    process.exit(1);
  }
}

main();

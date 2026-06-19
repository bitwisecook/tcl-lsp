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
  const failureMarker = path.resolve(__dirname, "../../", ".vscode-test", "mocha-failures.json");
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
      if (failures > 0) {
        fs.mkdirSync(path.dirname(failureMarker), { recursive: true });
        fs.writeFileSync(failureMarker, JSON.stringify({ failures }) + "\n", "utf8");
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

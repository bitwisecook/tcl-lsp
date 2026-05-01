// Mocha entry point for the multi-folder workspace test suite.
// Picks up only ``*.test.js`` files under ``out/test/multiFolder/``.
import * as path from "path";
import Mocha from "mocha";
import { glob } from "glob";

export async function run(): Promise<void> {
  const mocha = new Mocha({
    ui: "tdd",
    color: true,
    timeout: 60_000,
  });

  const testsRoot = path.resolve(__dirname);
  const files = await glob("**/*.test.js", { cwd: testsRoot });
  files.sort();
  for (const f of files) {
    mocha.addFile(path.resolve(testsRoot, f));
  }

  return new Promise<void>((resolve, reject) => {
    const runner = mocha.run((failures) => {
      if (failures > 0) {
        reject(new Error(`${failures} test(s) failed.`));
      } else {
        resolve();
      }
    });
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

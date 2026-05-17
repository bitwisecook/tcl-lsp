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
  const mocha = new Mocha({
    ui: "tdd",
    color: true,
    timeout: 60_000, // LSP startup can be slow
  });
  if (process.env.MOCHA_GREP) {
    mocha.grep(process.env.MOCHA_GREP);
  }

  const testsRoot = path.resolve(__dirname);

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

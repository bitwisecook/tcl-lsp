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

  const testsRoot = path.resolve(__dirname);

  const files = await glob("**/*.test.js", { cwd: testsRoot });
  files.sort();
  for (const f of files) {
    mocha.addFile(path.resolve(testsRoot, f));
  }

  return new Promise<void>((resolve, reject) => {
    mocha.run((failures) => {
      if (failures > 0) {
        reject(new Error(`${failures} test(s) failed.`));
      } else {
        resolve();
      }
    });
  });
}

// Guard against the wasm-bindgen externref-table regression.
//
// wasm-bindgen (≥0.2.x with reference-types, the default on modern rustc)
// stores JS values in a growable `externref` table exported as
// `__wbindgen_externrefs`, and grows it by 4 during init
// (`__wbindgen_init_externref_table`). If any post-processing (notably
// `wasm-opt` from binaryen 120) rebinds that export onto the fixed-size
// funcref table, the runtime `Table.grow` throws
//   "WebAssembly.Table.prototype.grow could not grow the table"
// and the compiler explorer fails to initialise.
//
// This script instantiates the built module with stubbed imports and asserts
// the exported externref table can actually grow. Exit non-zero on failure so
// `make explorer-wasm` (and CI) fail loudly instead of shipping a broken GUI.

import { readFileSync } from "node:fs";

const path = process.argv[2];
if (!path) {
  console.error("usage: verify-explorer-wasm.mjs <path-to-bg.wasm>");
  process.exit(2);
}

const mod = await WebAssembly.compile(readFileSync(path));

// Stub every import so instantiation succeeds regardless of the JS glue.
const env = {};
for (const im of WebAssembly.Module.imports(mod)) {
  (env[im.module] ??= {});
  env[im.module][im.name] =
    im.kind === "function"
      ? () => 0
      : im.kind === "global"
        ? new WebAssembly.Global({ value: "i32", mutable: true }, 0)
        : im.kind === "memory"
          ? new WebAssembly.Memory({ initial: 17 })
          : undefined;
}

const inst = await WebAssembly.instantiate(mod, env);
const table = inst.exports.__wbindgen_externrefs;

if (!(table instanceof WebAssembly.Table)) {
  console.error("FAIL: no __wbindgen_externrefs table export found");
  process.exit(1);
}

try {
  const before = table.length;
  table.grow(4);
  console.log(
    `OK: __wbindgen_externrefs is growable (${before} -> ${table.length})`,
  );
} catch (e) {
  console.error(
    `FAIL: __wbindgen_externrefs.grow threw (table len=${table.length}): ${e.message}\n` +
      "The externref export is likely rebound onto the fixed funcref table " +
      "(binaryen/wasm-opt regression). Ship the un-optimised wasm-bindgen output.",
  );
  process.exit(1);
}

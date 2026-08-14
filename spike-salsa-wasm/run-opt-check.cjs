const fs = require('fs');
const path = require('path');
// monkeypatch: load glue but point it at the opt'd wasm bytes
const glueSrc = fs.readFileSync('./out-node-final/spike_salsa_wasm.js', 'utf8');
const Module = require('module');
const m = new Module('spike_salsa_wasm_opt.js', module);
m.filename = path.resolve('./out-node-final/spike_salsa_wasm_opt.js');
m.paths = Module._nodeModulePaths(path.dirname(m.filename));
// Redirect the wasm path the glue reads via __dirname + join('spike_salsa_wasm_bg.wasm')
const patched = glueSrc.replace("'spike_salsa_wasm_bg.wasm'", "'spike_salsa_wasm_bg.wasm.opt'");
fs.writeFileSync('./out-node-final/spike_salsa_wasm_opt.js', patched);
try {
  const { run_spike } = require('./out-node-final/spike_salsa_wasm_opt.js');
  const out = run_spike();
  console.log(out);
  console.log("=== OPT CHECK: returned normally ===");
} catch (e) {
  console.error("=== OPT CHECK: THREW ===");
  console.error(String(e));
  console.error(e && e.stack);
}

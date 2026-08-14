const { run_spike } = require('./out-node-final/spike_salsa_wasm.js');
try {
  const out = run_spike();
  console.log(out);
  console.log("\n=== NODE HARNESS: run_spike() returned normally ===");
} catch (e) {
  console.error("=== NODE HARNESS: run_spike() THREW ===");
  console.error(e);
  console.error(e && e.stack);
  process.exitCode = 1;
}

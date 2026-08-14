const { run_negative_control } = require('./out-node-negctrl/spike_salsa_wasm.js');
try {
  run_negative_control();
  console.log("=== NEGATIVE CONTROL: returned normally (UNEXPECTED — should have panicked) ===");
} catch (e) {
  console.error("=== NEGATIVE CONTROL: THREW (expected) ===");
  console.error(String(e));
  console.error(e && e.stack);
}

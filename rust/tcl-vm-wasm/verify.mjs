// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Smoke-check the tcl-vm-wasm module: load it (no imports/WASI), run a coroutine
// script through the `tcl_alloc`/`tcl_eval`/`tcl_dealloc` ABI, and assert the
// tclsh-9.0.4 oracle value. Exits non-zero on any mismatch. Used by
// `make tcl-vm-wasm` and mirrored by the Rust e2e test.

import { readFileSync } from 'node:fs';

const wasmPath = process.argv[2];
if (!wasmPath) {
  console.error('usage: verify.mjs <vm.wasm>');
  process.exit(2);
}

const { instance } = await WebAssembly.instantiate(readFileSync(wasmPath), {});
const ex = instance.exports;

/** Run a Tcl script through the module and return its captured `puts` output. */
function evalTcl(script) {
  const enc = new TextEncoder().encode(script);
  const ptr = ex.tcl_alloc(enc.length);
  new Uint8Array(ex.memory.buffer, ptr, enc.length).set(enc);
  const packed = ex.tcl_eval(ptr, enc.length); // (out_ptr << 32) | out_len (BigInt)
  ex.tcl_dealloc(ptr, enc.length);
  const outPtr = Number(packed >> 32n);
  const outLen = Number(packed & 0xffffffffn);
  // Re-read memory.buffer after the call — it may have grown during eval.
  const out = new TextDecoder().decode(
    new Uint8Array(ex.memory.buffer, outPtr, outLen).slice(),
  );
  ex.tcl_dealloc(outPtr, outLen);
  return out.trim();
}

const cases = [
  // A generator yielding across a foreach; yieldable command substitution packs
  // 2,3,4 → 234.
  ['generator', 'proc gen {} { foreach x {1 2 3 4} { yield $x } }; ' +
    'coroutine c gen; puts [expr {[c]*100 + [c]*10 + [c]}]', '234'],
  // The `set n [yield $sum]` resume-value idiom.
  ['accumulator', 'proc acc {} { set s 0; while 1 { set n [yield $s]; incr s $n } }; ' +
    'coroutine a acc; puts [expr {[a 5]*10000 + [a 10]*100 + [a 3]}]', '51518'],
  // `yield` crossing a `catch` body.
  ['catch', 'proc g {} { set r [catch {yield inner} m]; yield "done:$r:$m" }; ' +
    'coroutine c g; puts [list [c] [c foo]]', 'done:0: foo'],
  // `yield` crossing a straight-line `lmap` (the inline collecting loop): the
  // first yield (1) is consumed by creation, the resumes see 2,3, and the loop
  // returns the mapped list of resume values.
  ['lmap', 'proc g {} { lmap n {1 2 3} {yield $n} }; ' +
    'coroutine c g; puts [list [c A] [c B] [c C]]', '2 3 {A B C}'],
  // `yield` crossing a `subst` template's `[…]` brackets (the scanner-driven subst
  // frame): resumes fold into the output, which is returned once the scan finishes.
  ['subst', 'proc g {} { subst {a=[yield 1]b=[yield 2]} }; ' +
    'coroutine c g; puts [list [c P] [c Q]]', '2 a=Pb=Q'],
];

let failed = false;
for (const [name, script, want] of cases) {
  const got = evalTcl(script);
  if (got === want) {
    console.log(`  ok   ${name}: ${got}`);
  } else {
    console.error(`  FAIL ${name}: want ${JSON.stringify(want)}, got ${JSON.stringify(got)}`);
    failed = true;
  }
}
process.exit(failed ? 1 : 0);

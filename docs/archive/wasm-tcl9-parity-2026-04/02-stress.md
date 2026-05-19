# 02 — Stress runs: amplified iterations

## Why amplify

The single-shot numbers in [`01-end-to-end.md`](01-end-to-end.md)
are dominated by wasmtime store setup (≈ 9 ms) and `tclsh`
spawn (≈ 72 ms). To see how the runtime cores compare on actual
script work we wrap each runnable sample's body in a
`for {set __i 0} {$__i < N} {incr __i} { … }` loop with N chosen so
total script work is in the 30–300 ms range, then re-time both
backends with the same source. `puts` lines are stripped first so
output volume doesn't drown the work.

## Results

| Sample | iters | wasm med (ms) | tclsh med (ms) | wasm/tclsh |
|---|---:|---:|---:|---:|
| 01_simple_variables.tcl | 5,000 | 11.23 | 103.89 | **0.108×** |
| 02_control_flow_braced.tcl | 5,000 | 16.38 | 104.97 | **0.156×** |
| 03_procs_and_namespaces.tcl | 2,000 | 9.26 | 105.62 | **0.088×** |
| 08_tricky_edge_cases.tcl | 1,000 | 13.41 | 120.23 | **0.112×** |
| 11_minifier_demo.tcl | 1,000 | 13.54 | 136.92 | **0.099×** |
| 12_safe_on_uninit.tcl | 1 (proc defs only) | 10.34 | 103.08 | **0.100×** |
| 13_incr_dialect_difference.tcl | 1 (proc defs only) | 8.58 | 103.58 | **0.083×** |
| compiler_explorer_demo.tcl | 2,000 | 9.66 | 115.71 | **0.083×** |

## What the numbers mean

After subtracting the per-call baselines:

- **Sample 02** does most actual work (5 000 outer × ≈ 10 inner
  ops): wasm 7 ms / 50 000 ops ≈ **140 ns/op**; tclsh 33 ms /
  50 000 ≈ 660 ns/op — **wasm 4.7× faster** on the work itself.
- **Sample 01** (5 000 outer × 3 trivial ops): wasm 2 ms /
  15 000 ≈ 130 ns/op; tclsh 32 ms / 15 000 ≈ 2 100 ns/op
  (heavily dominated by `puts` even after stripping ours, due
  to the warmup phase).
- **Sample 12 / 13** at N=1 — only `proc` definitions, no body
  calls — show that even just compiling+registering procs is
  faster on the precompiled wasm path than tclsh's bytecode
  compiler.

## Caveat

The amplified-loop framing can be optimised more aggressively by
the wasm compiler than by `tclsh`, especially when the loop body
writes to a variable that's never read. See
[`03-microbench.md`](03-microbench.md) for a per-primitive
breakdown that is harder to fold away.

Source: `stress_samples.py`; raw data: `stress_results.json`.

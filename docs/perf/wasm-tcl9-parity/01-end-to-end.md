# 01 — End-to-end: each sample, single run

## Methodology

For every `samples/tcl/*.tcl`:

1. Compile to WASM with `tests.test_wasm_real_tcl._compile_tcl_with_diag`
   (no codegen optimisations).
2. Time **30 wasm runs** (3 warmup), each in a fresh wasmtime store
   so module setup is included.
3. Time **30 tclsh runs** (3 warmup), each as a fresh subprocess.
4. Compare `stdout` after `.strip()`; record `tclsh` exit code and
   any wasm trap.

## Baselines

| | median |
|---|---:|
| wasmtime store + linker + `_initialize` (no-op script) | **9.24 ms** |
| `tclsh` spawn + libc + interp init + exit | **71.93 ms** |

Subtract these from the per-sample numbers below to estimate
real script work.

## Per-sample results

| Sample | wasm bytes | wasm med (ms) | tclsh med (ms) | tclsh rc | stdout match | notes |
|---|---:|---:|---:|---:|---|---|
| 01_simple_variables.tcl | 504 | 9.71 | 73.22 | 0 | ✅ | |
| 02_control_flow_braced.tcl | 634 | 9.87 | 94.55 | 0 | ✅ | |
| 03_procs_and_namespaces.tcl | 838 | 9.04 | 99.30 | 0 | ✅ | |
| 04_substitution_and_quoting.tcl | 803 | 8.88 | 121.45 | 0 | ❌ | `clock format` returns `0` on wasm |
| 05_warning_examples.tcl | 730 | 8.98 | 94.98 | 1 | ❌ | wasm runs the bad code, tclsh rejects it |
| 06_security_smells.tcl | 781 | — | — | — | — | SKIPPED — recursive `source $argv0` |
| 07_arity_and_subcommand_errors.tcl | 452 | TRAP | 97.96 | 1 | n/a | `unsupported in WASM: string (no subcommand)` |
| 08_tricky_edge_cases.tcl | 915 | 9.01 | 100.70 | 0 | ✅ | |
| 09_long_code.tcl | 26,490 | TRAP | 74.08 | 1 | n/a | `unsupported command: oo::class` (TclOO) |
| 10_format_strings.tcl | 2,939 | TRAP | 72.47 | 1 | n/a | `unsupported command: format %2$s` (positional) |
| 11_minifier_demo.tcl | 1,211 | 9.21 | 96.07 | 0 | ✅ | |
| 12_safe_on_uninit.tcl | 2,497 | 9.71 | 95.44 | 0 | ✅ | |
| 13_incr_dialect_difference.tcl | 639 | 9.75 | 96.45 | 0 | ✅ | |
| compiler_explorer_demo.tcl | 1,029 | 9.03 | 98.79 | 0 | ✅ | |

## Observations

- **Compile size is tight**: the seven smallest samples compile to
  ≤ 1 KB of wasm; only `09_long_code.tcl` (539 lines, classes,
  ensembles, dict ops) crosses 25 KB. Code-size growth is roughly
  linear with source size.
- **Wasm wins every direct comparison.** Even before subtracting
  baselines, the largest wasm wall time (9.87 ms) is below the
  smallest tclsh (73 ms). Most of that win is wasmtime not having
  to fork-exec — see [`02-stress.md`](02-stress.md) for the
  isolated-work comparison.
- **Three samples trap on wasm and tclsh exits non-zero on them
  too** — sample 7 is intentional errors, sample 9 needs TclOO,
  sample 10 has undefined vars + `%2$s` format. These are
  correctness gaps, not perf signals.

Detailed traps + stdout/stderr captures are in
[`results.json`](results.json).

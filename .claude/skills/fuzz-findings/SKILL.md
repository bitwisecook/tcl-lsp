---
name: fuzz-findings
description: >
  Drive the native differential fuzzer (rust/tcl-fuzz): run campaigns that
  compare the bytecode VM against a reference tclsh, summarise the findings
  registry by category, and replay a seed to triage or confirm a fix. Use when
  working on fuzz findings, checking what diverges, or verifying a VM fix.
allowed-tools: Bash, Read, Write, Edit
---

# Fuzz Findings Management

The differential fuzzer lives in `rust/tcl-fuzz`. It generates Tcl programs,
runs each on the bytecode VM (`tclvm`, the *subject*) and on a reference
`tclsh`, and records every divergence to a findings registry. Findings are
keyed by their generating **seed** (so they replay exactly) and stored as a
JSON record (see `rust/tcl-fuzz/src/findings.rs`) plus the raw `.tcl` script
under the findings directory (default `fuzz-findings/`, override with
`--findings DIR`).

There is no separate "fixed/unfixed" state: the registry de-duplicates by seed
and categorises by the divergence kind. A finding is "fixed" when its seed no
longer reproduces — confirm that by replaying it (below).

## Usage

Build once, then invoke the binary:

```bash
cargo run -q -p tcl-fuzz -- <command> [args...]
# or, after `cargo build -p tcl-fuzz`:
./target/debug/tcl-fuzz <command> [args...]
```

Global flags: `--findings DIR` (registry location), `--timeout MS` (per-script
timeout), `--tclvm PATH` / `--tclsh PATH` (engine binaries; sensible defaults
are auto-located).

## Commands

| Command | Arguments | What it does |
|---|---|---|
| `run` | `--iterations N [--seed S] [--verbose]` | Run a campaign of N generated scripts; new divergences are appended to the registry |
| `summary` | | Print the registry's finding counts grouped by category |
| `replay` | `SEED` | Regenerate seed S, run both engines, and print their output side by side (triage / confirm-a-fix) |
| `wasm-check` | `--iterations N [--seed S] [--verbose]` | WASM-runnability arm: compile each program to the eval-fallback WASM module and flag codegen panics / instantiation failures / traps |
| `wasm-diff` | `--iterations N [--seed S] [--verbose]` | WASM value-differential arm: compare compiled-WASM control flow (hosted by `tcl-vm`) against direct `tcl-vm`, isolating control-flow miscompiles |

## Categories

A finding's category is the [`Verdict`](../../rust/tcl-fuzz/src/harness.rs) that
produced it:

| Category | Meaning |
|---|---|
| `stdout_mismatch` | The two engines produced different stdout |
| `status_mismatch` | The two engines disagreed on error/success status |
| `timeout` | The subject (`tclvm`) hung |

## Typical workflows

### Run a campaign and see what diverges
```bash
cargo run -q -p tcl-fuzz -- run --iterations 5000 --verbose
cargo run -q -p tcl-fuzz -- summary
```

### Triage a specific finding
```bash
cargo run -q -p tcl-fuzz -- replay 1772893252
```

### Confirm a VM fix resolved a finding
After changing the VM, rebuild and replay the seed; the engines should now
agree (no divergence printed):
```bash
cargo build -p tcl-vm
cargo run -q -p tcl-fuzz -- replay 1772893252
```

## Finding JSON schema

Each finding is one JSON record (`rust/tcl-fuzz/src/findings.rs`):

```json
{
  "seed": 1772822330,
  "category": "stdout_mismatch",
  "script": "…the generated Tcl…",
  "reference_stdout": "…tclsh stdout…",
  "subject_stdout": "…tclvm stdout…",
  "reference_errored": false,
  "subject_errored": false
}
```

$ARGUMENTS

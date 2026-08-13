---
name: fuzz-findings
description: >
  Drive the native differential fuzzer (rust/tcl-fuzz): run campaigns that
  compare any pair of backend engines (tclvm/tclsh, runtime/rust/tclsh,
  tclvm/runtime-rust), summarise the findings registry by category, and
  replay a seed to triage or confirm a fix. Use when working on fuzz
  findings, checking what diverges, or verifying a backend fix.
allowed-tools: Bash, Read, Write, Edit
---

# Fuzz Findings Management

The differential fuzzer lives in `rust/tcl-fuzz`. It generates Tcl programs
and runs each through a pair of **engines** (issue #1313), recording every
divergence to a findings registry. Three engines are available:

| Engine | What it is | Binary |
|---|---|---|
| `tclvm` | The native bytecode VM CLI | `tcl-vm-cli`'s `tclvm` |
| `tclsh` | A reference C Tcl interpreter | `tclsh9.0`/`tclsh` on `PATH` |
| `runtime-rust` | `runtime/rust`'s tree-walking interpreter | `runtime/rust`'s `run_script` dev-tool example (`cargo build --release --example run_script` under `runtime/rust`; needs `TCL_TOMMATH_DIR` pointed at a libtommath source tree for `expr`/math to work — see "Building the `runtime-rust` engine" below) |

`run --reference E1 --subject E2` pairs any two of these (default: `tclsh`
reference, `tclvm` subject — the original, still-default pair). Findings are
keyed by their generating **seed** (so they replay exactly) and stored as a
JSON record (see `rust/tcl-fuzz/src/findings.rs`) plus the raw `.tcl` script
under the findings directory (default `fuzz-findings/`, override with
`--findings DIR`). A non-default pair is namespaced under
`<findings>/<subject>-vs-<reference>/` so two pairs never collide on the same
seed; the default pair keeps using `<findings>/` directly (no migration for
existing registries).

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

Global flags: `--findings DIR` (registry location), `--timeout-ms MS`
(per-script timeout), `--tclvm PATH` / `--tclsh PATH` / `--runtime-rust PATH`
(engine binaries; sensible defaults are auto-located beside `tcl-fuzz` or on
`PATH`).

## Commands

| Command | Arguments | What it does |
|---|---|---|
| `run` | `--iterations N [--seed S] [--verbose] [--reference E] [--subject E] [--compare-error-text] [--subject-tcl-version X.Y]` | Run a campaign of N generated scripts over the `--subject`/`--reference` pair (default `tclvm` subject / `tclsh` reference); new divergences are appended to the pair's registry |
| `summary` | `[--reference E] [--subject E]` | Print a pair's registry finding counts grouped by category (defaults to the same `tclsh`/`tclvm` pair as `run`) |
| `replay` | `SEED [--reference E] [--subject E]` | Regenerate seed S, run both engines, and print their output side by side including stderr (triage / confirm-a-fix) |
| `wasm-check` | `--iterations N [--seed S] [--verbose]` | WASM-runnability arm: compile each program to the eval-fallback WASM module and flag codegen panics / instantiation failures / traps |
| `wasm-diff` | `--iterations N [--seed S] [--verbose]` | WASM value-differential arm: compare compiled-WASM control flow (hosted by `tcl-vm`) against direct `tcl-vm`, isolating control-flow miscompiles |

## Match the reference `tclsh`'s version, or read every finding twice

**A divergence is evidence of a bug only when both engines speak the same
version of Tcl.** Issue #1328 was filed as two `runtime/rust` bugs from a
200-iteration campaign against `tclsh8.6`. Re-run against `tclsh9.0.4`, **all
eight** of that campaign's findings disappear — every one was a deliberate
8.6-vs-9.0 language change, not a defect:

| Cause | Seeds | What differs |
|---|---|---|
| TIP 461 `lt`/`gt`/`le`/`ge` string-comparison operators (9.0+) | 90040, 90061, 90091, 90104, 90112 | 8.6 rejects `expr {{a} lt {b}}` as `invalid bareword "lt"` |
| TIP 521 `isfinite()`/`isinf()`/`isnan()` (9.0+) | 90119 | 8.6: `invalid command name "tcl::mathfunc::isfinite"` |
| Namespace-scope global fallback for relative variable names, removed in 9.0 (TIP 278) | 90022, 90188 | see the [KCS note](../../../docs/kcs/kcs-qa-why-does-a-namespace-variable-behave-differently-on-tcl-8-and-9.md) |

Every campaign now prints both engines' releases before it starts and warns
when they differ, and every finding records `reference_version`,
`subject_version` and `version_skew`. Check those before triaging.

To run version-matched against an older reference, pin the subject:

```bash
# runtime-rust emulating 8.6, against an 8.6 tclsh
cargo run -q -p tcl-fuzz -- --tclsh /path/to/tclsh8.6 \
  run --subject runtime-rust --reference tclsh --subject-tcl-version 8.6 \
  --iterations 200 --seed 90000
```

Only `runtime-rust` can be pinned today (it takes `--tcl-version`); pinning
`tclsh` or `tclvm` is refused with a message rather than silently ignored. With
the subject pinned to 8.6 the two variable-resolution findings above vanish; the
six `expr`-surface ones remain, because neither engine gates its `expr`
operator/mathfunc surface by release yet.

`--compare-error-text` (on `run`) additionally flags an `ErrorTextMismatch`
finding when both engines error but their stderr text differs — off by
default, since independent implementations legitimately word errors
differently (e.g. "wrong # args" phrasing); use it when hunting for an engine
reporting the *wrong kind* of error for a shared failure, not just "an error".

## Building the `runtime-rust` engine

`runtime/rust`'s `expr`/`if`/`while`/`for`/math commands are gated behind a
`have_tommath` cfg that only turns on when a libtommath C source tree is
found at build time (`$TCL_TOMMATH_DIR`, else `tmp/tcl9.0.4/libtommath` — see
`runtime/rust/build.rs`). Without it, `run_script` is missing most of the
generator's surface and a `runtime-rust`-paired campaign is dominated by
false "invalid command name" divergences that are a *build* gap, not a
runtime bug. Point `TCL_TOMMATH_DIR` at any libtommath ≥ 1.3.0 source
checkout (the version matters: 1.2.x's `mp_expt_n` is a deprecated wrapper
excluded by the build script's file filter; 1.3.0 defines it directly in
`bn_mp_expt_n.c`) before building:

```bash
# The Tcl 9.0.4 source tarball already ships libtommath 1.3.0 at
# tmp/tcl9.0.4/libtommath, so extracting it (which building a 9.0 oracle
# does anyway) is usually enough — check for that directory FIRST.
# Only if it is genuinely absent, a shallow tag checkout also works and
# needs no build of its own:
git clone --depth 1 --branch v1.3.0 \
  https://github.com/libtom/libtommath tmp/libtommath-1.3.0

# `runtime/rust` is a standalone crate, excluded from the top-level
# workspace — build it from its own directory so it keeps its own target
# dir (do not export the workspace's CARGO_TARGET_DIR into it).
cd runtime/rust && TCL_TOMMATH_DIR="$PWD/../../tmp/libtommath-1.3.0" \
  cargo build --release --example run_script
```

Verify with a quick smoke script (`expr`/`if` should both work, not error
with `invalid command name`) before trusting a campaign's findings.

## Categories

A finding's category is the [`Verdict`](../../../rust/tcl-fuzz/src/harness.rs) that
produced it:

| Category | Meaning |
|---|---|
| `stdout_mismatch` | The two engines produced different stdout |
| `status_mismatch` | The two engines disagreed on error/success status |
| `error_text_mismatch` | Both engines errored, but their stderr text differed (only recorded with `--compare-error-text`) |
| `timeout` | The subject hung |

## Typical workflows

### Run a campaign and see what diverges (default pair: tclvm subject / tclsh reference)
```bash
cargo run -q -p tcl-fuzz -- run --iterations 5000 --verbose
cargo run -q -p tcl-fuzz -- summary
```

### Run a campaign over a different backend pair
```bash
# runtime/rust vs tclsh (needs runtime-rust built with TCL_TOMMATH_DIR — see above)
cargo run -q -p tcl-fuzz -- run --subject runtime-rust --reference tclsh \
  --iterations 2000 --verbose
cargo run -q -p tcl-fuzz -- summary --subject runtime-rust --reference tclsh

# tclvm vs runtime/rust (both native implementations — isolates a tclvm/runtime-rust
# divergence from a "did either agree with tclsh" question)
cargo run -q -p tcl-fuzz -- run --subject tclvm --reference runtime-rust \
  --iterations 2000 --verbose
```

### Hunt for wrong-kind-of-error divergences
```bash
cargo run -q -p tcl-fuzz -- run --iterations 5000 --compare-error-text
```

### Triage a specific finding
```bash
cargo run -q -p tcl-fuzz -- replay 1772893252
# a non-default pair's finding needs the matching --reference/--subject:
cargo run -q -p tcl-fuzz -- replay 1772893252 --subject runtime-rust --reference tclsh
```

### Confirm a fix resolved a finding
After changing the backend, rebuild and replay the seed; the engines should
now agree (no divergence printed):
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
  "reference_stdout": "…reference engine's stdout…",
  "subject_stdout": "…subject engine's stdout…",
  "reference_errored": false,
  "subject_errored": false,
  "reference_stderr": "",
  "subject_stderr": "",
  "reference_version": "9.0.4",
  "subject_version": "9.0.4",
  "version_skew": false
}
```

`reference_version`/`subject_version` are each engine's `[info patchlevel]`,
probed once per campaign (`null` if the probe failed). `version_skew` is true
when the two engines emulate different release *lines* — treat every finding in
a skewed run as suspect until the version difference is ruled out.

`reference_stderr`/`subject_stderr` are always captured when the engine ran
(independent of `--compare-error-text`), so a triager always has both
engines' error text to hand even for a `stdout_mismatch`/`status_mismatch`
finding.

$ARGUMENTS

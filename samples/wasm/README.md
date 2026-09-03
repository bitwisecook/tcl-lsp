# WASM native-lowering sample tiers

Deterministic Tcl scripts, one tier per directory, that exercise the shapes
the Tcl-to-WebAssembly compiler must lower natively before it can drop Tcl
framing. Every script runs unchanged under `tclsh9.0`; its oracle output is
committed under `expected/` so a differential harness can compare the linked
WASM module against C Tcl byte for byte.

The tiers are ordered by how much interpreter machinery a *correct* compile
needs. A script in tier N is expected to compile with no more framing than
tier N introduces; anything the compiler still routes through
`tcl_eval_code` for a lower tier is a gap the plan in
[`docs/design/compiler/wasm-native-lowering-plan.md`](../../docs/design/compiler/wasm-native-lowering-plan.md)
tracks.

| Tier | Directory | Framing a correct compile needs |
|---|---|---|
| T0 | `t0-straight-line/` | none: native `i64`/`f64` locals, one box at the `puts` boundary |
| T1 | `t1-expr-control/` | none: native expression evaluation and structured control flow |
| T2 | `t2-values/` | boxed `TclObj` values and runtime intrinsics for list/string/dict/regexp ops; no Tcl frame |
| T3 | `t3-procs/` | native functions for leaf procs; a Tcl frame only where a formal-parameter or completion rule needs one |
| T4 | `t4-scopes/` | named variable cells for the exact names `global`/`variable`/`upvar`/arrays/namespaces make observable |
| T5 | `t5-completion/` | the full completion triple (`code`, result, return options) through `catch`/`try`/`return -code` |
| T6 | `t6-tcloo/` | a light object frame: real call chain, `self`/`my`/`next`, instance-variable links; no per-call chain rebuild |
| T7 | `t7-dynamic/` | full Tcl framing on the traced/introspected/suspended cells and commands only; everything else stays native |

## Regenerating the oracles

```
for f in samples/wasm/t*/*.tcl; do
  d=$(dirname "$f"); b=$(basename "$f" .tcl)
  tclsh9.0 "$f" > "samples/wasm/expected/$d/$b.out" 2>&1
done
```

## Measuring what the compiler emits today

```
cargo build -p tcl-compiler --example emit_wasm
target/debug/examples/emit_wasm --wat            script.tcl out.wasm   # default plan
target/debug/examples/emit_wasm --wat --analysis script.tcl out.wasm   # opt-in analysis tier
```

Count `call <import>` sites in the `.wat` to see how much of the script is
still `tcl_eval_code` (source re-parsed at run time), `tcl_invoke_argv`
(compiled words, runtime dispatch), or native instructions. The plan document
carries the baseline table for every script here.

# Lane: WASM native lowering programme

Tracking document for the phased plan in
[`docs/design/compiler/wasm-native-lowering-plan.md`](../compiler/wasm-native-lowering-plan.md).
One sub-lane per phase; each keeps its own section here. Protocol: AGENTS.md
"Long-running agent lanes" (compile before every commit, stage by explicit
path, `wip(<lane>):` prefix, lanes commit locally, the orchestrator pushes).

Branch: `claude/wasm-codegen-architecture-5exvpu`.

## Sub-lanes

| Lane | Phase | Owner files | Status |
|---|---|---|---|
| `p0-harness` | P0 tier harness + runtime unit suite in CI | `rust/tcl-compiler/tests/wasm_tiers.rs`, `samples/wasm/budgets.tsv`, `.github/workflows/ci.yml`, `runtime/rust/examples/run_script.rs` | open |
| `p1-runtime-abi` | P1 runtime ABI v2 groundwork | `runtime/rust/src/{codegen_abi,frame,vars,interp,obj,bignum,builtins,expr}.rs`, `rust/tcl-runtime-api/src/codegen_abi.rs` | open |
| `p2-executable-ir` | P2 executable IR total | `rust/tcl-compiler/src/executable_ir.rs` and its consumers | open |
| `p3-native-lowering` | P3 NLIR + native T0/T1 | new `rust/tcl-compiler/src/native_lowering/`, `codegen/wasm/backend.rs` | blocked on P1, P2 |

## Decisions

- Compiled-code activations count as eval-loop activations: the runtime's
  "outermost eval" rule (`interp.rs` eval loop, depth 0) must never fire
  inside a `tcl_invoke_argv` dispatched from generated code. The fix is an
  activation record the ABI enters and leaves, not a special case in `catch`.
- Compiled procs are reached from the runtime through the shared wasm
  function table: the user module imports the runtime's table, grows it,
  installs its functions, and registers `(name, params, table index)`; the
  runtime treats the index as an `extern "C"` function pointer. Falls back to
  the source body when the native entry declines.
- No emitter reads a compatibility string. `whole_var_reference` and the
  `name.contains('(')` gates in `codegen/wasm/backend.rs` are retired by P3;
  word shapes come from `WordExpr` only.

## Site inventory and status

Filled in by each sub-lane as it lands.

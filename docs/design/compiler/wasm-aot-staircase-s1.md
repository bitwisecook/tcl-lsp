# Stage S1 — “frames everywhere” correctness baseline

> **Status:** landed historical stage. See the
> [AOT staircase](wasm-aot-staircase.md) for the original sequence and
> [current WASM architecture](wasm-codegen.md) for the production contract.

S1 established a correctness floor for later frame-elision work: a compiled
procedure can always retain a runtime frame when common analysis cannot prove
that name-based observation is impossible. Frame elision is an optimisation,
never a semantic assumption.

## Preserved design result

The stage validated these rules:

1. the framed path owns Tcl objects through the runtime frame;
2. elision requires an explicit escape and observation proof;
3. a dynamic or unsupported surface keeps the frame; and
4. runtime and leak tests compare an optimised build with the conservative
   correctness floor.

The detailed implementation checklist in the original Python compiler has
been retired with that compiler. Its former public flags, emitter classes, and
alternate API names are not part of the Rust architecture.

## Current Rust contract

`tcl-compiler::codegen::wasm::compile_wasm` is the sole public
Tcl-to-WebAssembly entry. It consumes a complete `CompilationUnit`, selects a
semantic plan through `BackendRegistry`, and records a typed compatibility
reason when common executable IR or a required proof is unavailable.

There is no user-facing frame-elision backend or alternate “frames everywhere”
code generator. `WasmCompileOptions::for_eval_only_test_host` is an isolated
ABI test policy; it is not a production backend choice. The bytecode VM is
runtime machinery, not another source-to-WASM compiler.

The common var-escape analysis still expresses the S1 safety rule through
`ProcEscapeSummary::safe_for_frame_elision`. Code generation may consume that
fact only inside the canonical semantic pipeline, alongside independent world,
dispatch, trace, completion, ownership, and representation proofs. An absent
proof selects conservative compatibility; it never selects elision.

## Historical acceptance

The stage is recorded as landed by commits `f8d920ea` and `c6831243` in the
staircase index. Its acceptance comparison covered the Tcl test sweep and leak
checks with the conservative framed mode enabled. This page preserves the
architectural result without presenting the removed Python-era task list as a
current implementation guide.

## Related design

- [WASM AOT staircase](wasm-aot-staircase.md)
- [Var-escape analysis](var-escape-analysis.md)
- [WASM code generation](wasm-codegen.md)

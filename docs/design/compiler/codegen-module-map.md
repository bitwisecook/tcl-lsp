# Code-generation module map

Rust code generation lives in `rust/tcl-compiler/src/codegen/`. The shared
compiler front end builds IR, CFG, SSA, semantic analysis, and structured
lowering before target emission.

## Common and bytecode modules

The `codegen` module exports the bytecode API used by the VM and exposes the
WASM submodule:

- `backend.rs` — target-agnostic `Backend` trait and `BytecodeBackend`;
- `emitter/` — bytecode module and function orchestration;
- `statements.rs`, `expressions.rs`, `values.rs`, `control_flow.rs`, and
  `cmd_subst.rs` — bytecode lowering by IR concern;
- `peephole.rs` — bytecode-local rewrites;
- `emit.rs` and `structured.rs` — common structured-emission seam; and
- `wasm/` — the canonical Tcl-to-WebAssembly pipeline.

Bytecode artefact types, instruction layout, and formatting live in the leaf
`tcl-bytecode` crate and are re-exported by `tcl-compiler::codegen`. The VM is
an execution artefact; it is not a selectable Tcl-to-WebAssembly backend.

## WASM modules

`rust/tcl-compiler/src/codegen/wasm/` has one public compilation entry:

- `mod.rs` exports `compile_wasm`, `WasmCompilation`, typed plan evidence,
  packaging options, and WASM IR types;
- `pipeline.rs` owns the sole public semantic-plan ladder;
- `semantic_plan.rs` validates generic prebuilt-argv input from common
  executable IR without emitting a module;
- `backend.rs` is the sole module emitter for both selected semantic invocation
  and general structured lowering;
- `ir.rs` owns the target module, function, instruction, import, and data
  vocabulary; and
- `encoding.rs` serialises the target IR.

`backend`, `semantic_plan`, and `pipeline` are internal implementation modules.
Consumers do not select or invoke them directly.

## Public consumer contract

Every production consumer builds a complete `CompilationUnit` and calls:

```rust,ignore
let output = compile_wasm(&unit, registry, WasmCompileOptions::hosted());
```

`WasmCompilation` contains the module plus `WasmCodegenPlan`. The plan records
either the semantic operation selected from executable IR or the typed reason
that the same emitter used general structured lowering. The CLI, Explorer, fuzzer,
MCP tool, runtime linker, and standalone packager all consume this API.

There is no public backend enum, command-line backend selector, or IR-only WASM
emitter. Link and bundle code consumes the resulting `WasmModule`; packaging
is downstream of code generation and does not create another compiler path.

## Extension rule

New command behaviour starts in `tcl-registry` as data, flags, callbacks, or
typed hooks. Common analyses project those facts into executable IR. A WASM
implementation is selected by semantic operation through `BackendRegistry`
and must decline with a typed reason when its proof is incomplete. It must not
match a Tcl command name in the compiler or add another public emitter.

See [`wasm-codegen.md`](wasm-codegen.md) for the complete pipeline and runtime
ABI contract.

# WASM code generation

> **Status:** `codegen::wasm::compile_wasm` is the sole public Tcl-to-WASM
> code-generation entry point. It selects a semantic plan from executable IR
> first and records a typed compatibility reason when broad source-compatible
> emission is still required. Guarded boxed `string length` and the exact
> sealed native-i64-add demonstration are implemented but explicitly opt-in;
> every semantic AOT pass is off by default.

This document describes the Rust WebAssembly (WASM) compiler and its shared Tcl
runtime boundary. The target-independent semantic contract is defined in
[`common-semantic-compiler.md`](common-semantic-compiler.md).

## Architectural boundary

`tcl-registry` resolves structured invocations and supplies semantic operation,
completion, state transition, world-effect, dispatch-dependency, argument-role,
and transfer descriptors. Common compiler passes consume those facts. The WASM
backend registry selects an immutable implementation plan; only then does an
emitter serialise WASM IR.

```text
Tcl source
  -> Rust lexer, parser, and structured words
  -> CompilationUnit
  -> executable semantic CFG + registry InvocationFacts
  -> scalar, cell, and world-state proofs
  -> BackendRegistry plan selection
       -> generic prebuilt-argv plan
       -> guarded boxed intrinsic with the same prebuilt-argv slow path
       -> exact sealed native-i64-add plan
       -> general structured plan with a typed semantic decline
  -> one WasmEmitter implementation
  -> one WasmCompilation { module, plan evidence }
  -> WAT/binary, Explorer, linker, or standalone bundler
  -> shared Rust Tcl runtime ABI
```

Aliases, `rename`, namespace imports, ensembles, `unknown`, command traces,
safe or child interpreters, and TclOO can change runtime dispatch. A resolved
spelling is not sufficient evidence for direct code generation. Direct and
intrinsic plans require common dispatch-stability, completion, ownership, and
representation proofs. Generic argv invocation reuses runtime dispatch without
replaying source.

## One public entry point

Every production consumer constructs a complete `CompilationUnit` and calls:

```rust,ignore
let output = compile_wasm(&unit, registry, WasmCompileOptions::hosted());
let plan = output.plan;   // durable, typed selection evidence
let module = output.module;
```

The CLI, compiler example, compiler Explorer (including its Rust-to-WASM web
facade), differential fuzzer, MCP tool, runtime-link test, and standalone
packaging use this API. There is no public backend enum, no `--backend` CLI
choice, and no IR-only emitter API. Packaging options choose hosted,
runtime-linked, or standalone layout; they do not choose a compiler.

The returned `WasmCompilation` retains `WasmCodegenPlan`:

- `GenericInvoke` records the registry-owned semantic operation selected from
  executable IR and the retained mixed-region plan. With `GuardedIntrinsic`
  explicitly enabled, its invocation region may select the guarded boxed
  `StringLength` fast path while the top-level plan remains `GenericInvoke`
  because it retains generic argv dispatch as its exact slow path.
- `NativeI64Add` records the common proof-selected callee, exact operands,
  registry-owned boxed boundary, frame-elision fact, and closed-program
  statement count for the sealed demonstration.
- `General` records why the narrower semantic input mode declined: executable
  IR availability, backend selection, plan layout, packaging, or a deliberately
  restricted test host.

Explorer serialises this evidence as `codegenPlan` on the synthetic WASM module
header. It uses stable plan, reason, detail, and operation-category spellings,
not Rust debug output.

## Canonical plan selection

### 1. Preserve Tcl word semantics

Lowering retains each word as a structured `WordExpr`. Literal, substituted,
expanded, and opaque words remain distinguishable. A backend does not recover
meaning by reparsing source text.

### 2. Consume executable semantic IR

`CompilationUnit` already owns `SemanticAnalysisBundle`. Code generation reads
its `ExecutableAnalysisAvailability` rather than rebuilding a second semantic
view. For the bounded invocation shapes it currently represents, executable IR
makes word evaluation, argument expansion, argv assembly, invocation, and the
normal/abrupt completion spine explicit. Already-lowered operations and opaque
structured regions still carry conservative completion and effect facts; they
are not evidence that the complete Tcl completion flow has been projected.

World-state SSA may be unavailable while executable invocation facts remain
sound. The canonical pipeline may still select generic runtime dispatch in
that case; proof-requiring direct plans must abstain. If executable IR itself is
unavailable, the exact `SourceCompatibilityDecline` category is retained in
the general plan's `WasmSemanticDecline` evidence.

### 3. Resolve through `CommandRegistry`

Each `InvocationFacts` snapshot owns the selected command, subcommand, form,
argument roles, semantic operation, and proof descriptors. Code generation
contains no command-name switch. Adding target behaviour starts with a registry
descriptor and a target-neutral operation, not a branch in a WASM emitter.

### 4. Select through common and backend proof plans

The baseline semantic selector supports one literal-safe, prebuilt-argv
invocation. It is registered as `BackendPlanKind::GenericInvoke`; the registry
applies common legality checks before it constructs an immutable
`WasmGenericInvokePlan`.

Two optional selections now sit above that baseline:

- `GuardedIntrinsic` refines only a registry-resolved `StringLength` invocation
  whose completion/effect/transition/role/boundary/ownership/suspension proofs
  satisfy `BackendRegistry`. Its `MixedRegionPlan` owns the stable `NodeId`,
  registry intrinsic identity, dispatch dependencies, guard domains, exact
  argv/completion slow path, and typed candidate declines.
- `NativeI64Add` requires sealed-program policy and the explicit `DirectProc`,
  `MaterialisableSlot`, `FrameElision`, `NativeInteger`, and
  `SemanticOperationSpecialisation` controls. It consumes common direct-call,
  frame, slot, semantic-boundary, closed-program-coverage, type, SCCP, and range
  evidence. The current selector recognises only the exact motivating
  proc/two-constant-set/channel-write shape and requires overflow-impossible
  exact i64 operands.

Dynamic words, `{*}` expansion, multiple invocations, lowered operations,
opaque regions, extra statements in the sealed native shape, and unsupported
executable CFG shapes produce typed declines. They do not make the selector
mutate an emitter and do not erase common facts.

### 5. Feed one emitter

An eligible plan emits a module importing `tcl_invoke_argv`; it does not import
the source-evaluation ABI. Literal data is placed in the runtime-reserved
wasm32 code-generation window beginning at `RESERVED_DATA_BASE`. Hosted and
Explorer compilation use that same safe default, so an eligible hosted script
really exercises semantic invocation mode in the sole emitter.

With the guarded pass enabled, the same emitter additionally imports guard
prepare/check/release and boxed intrinsic invocation. It builds argv once,
prepares and re-checks a live per-interpreter token after word evaluation, and
uses `tcl_invoke_argv` with that same argv when admission or intrinsic execution
declines. No source word is replayed.

The sealed native-add mode emits an exported native `(i64, i64) -> i64`
procedure and a no-argument top function containing the two proved constants.
It imports only the wide-integer boxing and channel-write boundaries used by
that slice. A failed common premise declines before any native instruction is
emitted; this slice has no in-activation guard or deoptimisation edge.

Until executable IR covers the complete Tcl surface, a typed decline invokes
general structured mode in the same `WasmEmitter`. It preserves structured
control flow, direct Tcl-object operations where its established proofs permit
them, and exact source-span runtime evaluation otherwise. The decline changes
typed input to one emitter; it does not select another implementation. This
keeps runtime-evaluation compatibility working while making remaining semantic
coverage observable. It does not imply package-driven extension selection or
linking.

Standalone `_start`, interpreter creation, and optional standard-library
initialisation currently select `General` with the explicit
`standalone-bootstrap` semantic decline. Moving that bootstrap into a narrower
semantic packaging plan does not require a new API or emitter.

## Runtime ABI and ownership

The generic invocation ABI accepts the complete, already-evaluated argv. The
runtime uses its normal interpreter dispatcher, preserving namespace lookup,
aliases, ensembles, `unknown`, safe-interpreter policy, TclOO, and traces. It
returns the complete completion triple:

```text
(completion code, owned result object, owned return-options object)
```

Generated code allocates a re-entrant runtime call frame for argv handles and
the completion output. It retains outbound result and options before releasing
its private completion, argv values, and frame. ABI constants and layout live
in `tcl-runtime-api`; compiler and runtime do not maintain copies.

General lowering's generated values are `i32` pointers to owned
`TclObj` values in shared linear memory. Compiled variables keep an indexed
slot and their Tcl-visible named cell as two ports onto the same object, so
traces, `upvar`, and interpreted regions observe shimmering and writes through
the normal runtime cell. Guarded `StringLength` retains that boxed model. The
sealed native-add slice is the sole current exception: exact closed-program,
frame, slot, representation, and range proofs permit it to omit the two
top-level cells and procedure frame, retain the operands as i64, and box only
the result at the output boundary. No general native-cache, single-
representation, materialisation, reload, or deoptimisation path exists yet.

Relevant modules:

- `tcl-compiler/src/mixed_region_plan.rs` — target-neutral NodeId-keyed
  generic/guarded/lowered/opaque region evidence and exact slow paths;
- `tcl-compiler/src/common_aot_plan.rs` and `native_integer_proof.rs` — common
  direct-call, slot, frame, boundary, closed-coverage, and numeric proofs;
- `tcl-compiler/src/codegen/wasm/pipeline.rs` — sole entry and plan evidence;
- `tcl-compiler/src/codegen/wasm/semantic_plan.rs` — non-emitting executable-IR
  plan validation;
- `tcl-compiler/src/codegen/wasm/backend.rs` — the sole module emitter for
  generic, guarded-intrinsic, sealed-native, and general structured modes;
- `tcl-compiler/src/codegen/wasm/ir.rs` — shared target IR and encoders;
- `tcl-runtime-api/src/guard.rs` — shared guard identity, domain, and token
  vocabulary;
- `tcl-runtime-api/src/codegen_abi.rs` — shared ABI declarations; and
- `runtime/rust/src/codegen_abi.rs` — runtime implementation.

## VM boundary

`tcl-vm` and `tcl-vm-wasm` are execution artefacts. The latter embeds the
bytecode compiler and VM in a self-contained module whose host calls `tcl_eval`.
It remains useful for runtime and differential testing, but it is not a
Tcl-source-to-WASM code-generation backend and is not selectable by
`tcl compwasm` or Explorer. TclVM also does not yet consume the target-neutral
mixed-region, common AOT, guard, or native-integer proof plans; those common
types are the intended integration boundary, not a statement of current VM
specialisation.

## Module and package layout

Runtime functions are imported before defined functions. General-plan modules
export `::top` first, followed by procedures in qualified-name order. Current
generic modules export the selected semantic function as `::top` with the full
completion-triple signature. Runtime-linked constant data occupies the reserved
window below the Rust runtime's data/heap.

There is currently no compiler package-require scan, extension selector,
variant runtime artefact, or package-driven linker. The optional `wasm_stdlib`
runtime feature embeds Tcl scripts and package indices; package loading then
uses the runtime's ordinary `source` and `package require` machinery. The
current boundary and the explicitly future package-aware design are documented
in [`wasm-extensions.md`](wasm-extensions.md).

## Explorer contract

`tcl-explorer` calls `compile_wasm` for both source and optimised source. Its
WASM header includes:

```jsonc
{
  "codegenPlan": {
    "kind": "generic-invoke", // generic-invoke | native-i64-add | general
    "operation": "intrinsic", // null for general
    "semanticDecline": null    // typed reason object for general
  },
  "text": "(module …)"
}
```

The same `WasmModule` supplies WAT text, instruction JSON, binary encoding, and
link input. Explorer does not invoke a display-only emitter.

For `native-i64-add`, the durable `nativeI64Add` record retains the common
proof selection: direct callee identity, exact i64 operands, registry-owned
boxed boundary operation, frame-elision fact, and closed-program coverage
count. It is evidence, not an Explorer reconstruction of backend output.

For the guarded intrinsic, the top-level kind remains `generic-invoke`. The
selected region changes to `guarded-intrinsic`, and its selected candidate
records `string-length` plus the guarded dispatch-dependency domains. With the
pass off, the same candidate remains present with `pass-disabled`, making
ablation visible without changing default output.

## Extending code generation

When a Tcl surface is missing:

1. enrich the appropriate `CommandSpec`, subcommand, form, or shared registry
   descriptor;
2. project the target-neutral fact into executable IR and common analyses;
3. register a target plan by `SemanticOperationId`, with explicit proofs and
   typed declines;
4. add runtime backing when the operation crosses the shared runtime boundary;
5. test Tcl completion, state, trace, ownership, re-entrancy, safe/child
   interpreter behaviour, and representation transitions; and
6. shrink general runtime-evaluation regions without adding another emitter.

Command spellings, Tcl list parsing, option parsing, and runtime semantics do
not belong in the emitter. Pure list, string, numeric, boolean, option,
formal-parameter, and completion algorithms belong in shared crates below
runtime adapters.

Native/direct specialisation, frame elision, and single-representation storage
must additionally satisfy the default-off proof and fallback contract in
[`semantic-aot-optimisation.md`](semantic-aot-optimisation.md).

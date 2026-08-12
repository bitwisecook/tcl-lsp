# WASM code generation

> **Status:** `codegen::wasm::compile_wasm` is the sole public Tcl-to-WASM
> code-generation entry point. It selects a semantic plan from executable IR
> first and records a typed compatibility reason when broad source-compatible
> emission is still required.

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
  executable IR.
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
view. Executable IR makes word evaluation, argument expansion, argv assembly,
invocation, and full Tcl completion flow explicit.

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

### 4. Select through `BackendRegistry`

The current semantic selector supports one literal-safe, prebuilt-argv
invocation. It is registered as `BackendPlanKind::GenericInvoke` for the
generic semantic operation. The registry applies common legality checks before
it constructs an immutable `WasmGenericInvokePlan`.

Dynamic words, `{*}` expansion, multiple invocations, lowered operations,
opaque regions, and unsupported executable CFG shapes produce typed declines.
They do not make the selector mutate an emitter and do not erase common facts.

### 5. Feed one emitter

An eligible plan emits a module importing `tcl_invoke_argv`; it does not import
the source-evaluation ABI. Literal data is placed in the runtime-reserved
wasm32 code-generation window beginning at `RESERVED_DATA_BASE`. Hosted and
Explorer compilation use that same safe default, so an eligible hosted script
really exercises semantic invocation mode in the sole emitter.

Until executable IR covers the complete Tcl surface, a typed decline invokes
general structured mode in the same `WasmEmitter`. It preserves structured
control flow, direct Tcl-object operations where its established proofs permit
them, compiled prebuilt-argv invocation for every leaf command whose words it
can evaluate, and exact source-span runtime evaluation only for what is left.
The decline changes typed input to one emitter; it does not select another
implementation. This keeps broad package/link functionality working while
making remaining semantic coverage observable.

## The general tier's leaf-command path

Inside general structured mode, the **normal** path for a leaf
`Statement::Call` is compiled word evaluation followed by `tcl_invoke_argv`.
Source-span `tcl_eval_code` is the last-resort fallback, reached only when a
word shape declines.

Selection is per statement and stays registry-driven. `tcl-registry` resolves
the statement's structured words to an `InvocationFacts` snapshot; the
resulting `SemanticOperationId` keys a `BackendRegistry` selection over the
`PrebuiltArgvInvocation` region, and the registered `BackendPlanKind::GenericInvoke`
selector returns either a complete `WasmLeafInvokePlan` or one typed
`WasmLeafInvokeDecline`. The emitter never inspects a command name, and a
command the registry cannot resolve still selects under
`SemanticOperationId::Invoke` through the registry's generic fallback. This
needs no binding, trace, or rename proof: the runtime performs its ordinary
command resolution on the argv it is handed, so aliases, `rename`, ensembles,
`unknown`, TclOO, and execution traces all behave exactly as they do for
interpreted Tcl. Only the *words* require proof.

### Word shapes that compile

- a bare literal word;
- a braced literal word, whose value is its unsubstituted content;
- a `$var` scalar read;
- a `$arr(key)` element read with a statically known key;
- a `[nested command]` word — recursively the same argv invocation, whose
  completion result becomes the enclosing word's value; and
- a quoted or otherwise compound word, whose parts are evaluated left to right
  and joined by the runtime.

Everything else declines with a typed reason and keeps the whole statement on
the source-span fallback: `{*}` expansion, any word carrying backslash
substitution, a computed variable name or array key, a command substitution
that is not exactly one complete command, an opaque recovery word, and word
nesting past the planner's recursion cap.

Two Tcl spellings share one compatibility text: `$a(b)` and `${a(b)}` both
render as `${a(b)}`, yet the first is an array-element access and the second
names a scalar. The planner separates them from the recorded lexical extent —
a `${…}` reference is exactly two bytes longer than its name, a `$…` reference
exactly one — and emits `tcl_codegen_var_get_element` or `tcl_codegen_var_get`
accordingly.

### Frame layout and the single cleanup path

One statement allocates one transient call frame:

```text
[ object slots : N * 4 bytes  ]   every owned value the statement creates
[ completions  : M * 12 bytes ]   one TclCompletionAbi per invocation
```

An invocation's argv is the contiguous run of object slots holding its words,
so `tcl_invoke_argv` receives a pointer directly into the frame. A nested
command substitution shares the same frame; its completion result is adopted
into the enclosing word's own slot and its options into a slot of its own.

Ownership is the hard part, because completion dispatch can branch out of the
middle of a statement. The emitted shape makes that impossible to leak:

1. the prologue allocates the frame and writes null to every object slot;
2. the whole statement sits inside one `block`; a failed word read or an
   abrupt nested completion records the code and branches to its end;
3. after the block — on **every** path — each object slot is released
   (`tcl_obj_release` is null-safe) and the frame is freed; and
4. only then does the completion dispatch run and possibly `br`/`return`.

Because cleanup precedes every branch, there is exactly one release path and no
exit can leave an owned word, an adopted completion handle, or a frame behind.
`tcl_codegen_call_frame_outstanding` and the runtime's allocation counters
prove it in `runtime/rust`'s round-trip suite, including the mid-statement
error, `break`, and `return` cases.

### Relationship to the whole-function `GenericInvoke` plan

`semantic_plan.rs`'s `WasmGenericInvokePlan` and this per-statement path are
**not** the same selection, and must not be allowed to become two
implementations of one thing. The whole-function plan is an *export-contract*
decision: it fires only when the entire function is a single all-literal
invocation, and it exports `::top` with the full completion-triple signature so
the caller receives `(code, result, options)`. The per-statement path lowers a
statement inside an ordinary `() -> ()` `::top` and consumes the completion
itself.

What they do share — argv assembly, `tcl_invoke_argv`, and the owned
result/options release discipline — is currently written twice
(`finish_semantic_invoke` versus `emit_invoke_node`). The per-statement path is
also strictly the broader of the two: it proves variable, nested-command, and
compound words that the whole-function plan declines. Consolidating so the
whole-function plan builds a `WasmLeafInvokePlan` and reuses the same emission,
forwarding the adopted handles instead of releasing them, is the intended
direction; until that lands, a change to one ownership sequence must be
mirrored in the other.

### What the fallback still costs

Source-span evaluation re-lexes, re-parses, re-substitutes, and dispatches the
original text at run time. The compiled path skips all of that, at the price of
the `while executing` context `eval` would add to `-errorinfo` for the
statement itself. Conditions are unchanged: `if`/`while`/`for` guards still go
through `tcl_expr_bool` over their source text.

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
the normal runtime cell. A future representation plan may replace the indexed
port only after common alias, trace, and escape proofs allow it.

Relevant modules:

- `tcl-compiler/src/codegen/wasm/pipeline.rs` — sole entry and plan evidence;
- `tcl-compiler/src/codegen/wasm/semantic_plan.rs` — non-emitting executable-IR
  plan validation;
- `tcl-compiler/src/codegen/wasm/leaf_invoke.rs` — non-emitting per-statement
  prebuilt-argv planning, frame layout, and typed word declines;
- `tcl-compiler/src/codegen/wasm/backend.rs` — the sole module emitter for
  semantic invocation and general structured modes;
- `tcl-compiler/src/codegen/wasm/ir.rs` — shared target IR and encoders;
- `tcl-runtime-api/src/codegen_abi.rs` — shared ABI declarations; and
- `runtime/rust/src/codegen_abi.rs` — runtime implementation.

## VM boundary

`tcl-vm` and `tcl-vm-wasm` are execution artefacts. The latter embeds the
bytecode compiler and VM in a self-contained module whose host calls `tcl_eval`.
It remains useful for runtime and differential testing, but it is not a
Tcl-source-to-WASM code-generation backend and is not selectable by
`tcl compwasm` or Explorer.

## Module and package layout

Runtime functions are imported before defined functions. General-plan modules
export `::top` first, followed by procedures in qualified-name order. Current
generic modules export the selected semantic function as `::top` with the full
completion-triple signature. Runtime-linked constant data occupies the reserved
window below the Rust runtime's data/heap.

The optional package linker scans the merged IR for `package require`, selects
runtime extensions, and bundles their modules with the canonical compiler
output. Link and bundle stages consume encoded `WasmModule` output; they are
packaging stages, not alternate code generators.

## Explorer contract

`tcl-explorer` calls `compile_wasm` for both source and optimised source. Its
WASM header includes:

```jsonc
{
  "codegenPlan": {
    "kind": "generic-invoke", // or "general"
    "operation": "intrinsic", // null for general
    "semanticDecline": null    // typed reason object for general
  },
  "text": "(module …)"
}
```

The same `WasmModule` supplies WAT text, instruction JSON, binary encoding, and
link input. Explorer does not invoke a display-only emitter.

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

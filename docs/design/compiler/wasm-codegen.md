# WASM code generation

> **Status:** two current compiler paths coexist during the common semantic
> compiler migration. The analysis-aware tree-walker is the established
> direct-object path. The executable-IR `generic-invoke` backend is a bounded
> implementation of the target-independent architecture. Neither path alone
> represents the final pipeline.

This document describes the Rust WebAssembly (WASM) compiler and its
relationship with the shared Tcl runtime. The target-independent semantic
contract is defined in
[`common-semantic-compiler.md`](common-semantic-compiler.md).

## Architectural boundary

The destination architecture understands Tcl command semantics above code
generation. `tcl-registry` resolves a structured invocation and supplies its
semantic operation, completion behaviour, state transitions, world effects,
dispatch dependencies, argument roles, and transfer descriptors. Common
compiler passes consume those facts. A WASM backend registry then selects an
implementation for an already-understood operation, and the emitter serialises
the selected immutable plan.

```text
Tcl source
  -> Rust lexer, parser, and structured words
  -> target-independent IR and semantic executable CFG
  -> registry InvocationFacts
  -> scalar, cell, and world-state analyses
  -> representation, ownership, completion, and dispatch proofs
  -> WASM backend selection
  -> WASM IR and binary encoding
  -> shared Rust Tcl runtime ABI
```

Aliases, `rename`, namespace imports, ensembles, `unknown`, command traces,
safe or child interpreters, and TclOO can change runtime dispatch. A resolved
command spelling is therefore not sufficient evidence for direct code
generation. Direct and intrinsic plans require dispatch-stability and
representation proofs. Generic argv invocation is the ordinary compatibility
path in the destination architecture.

## Current entry points

The command-line compiler exposes three WASM backends:

| Backend | Current role |
|---|---|
| `vm` | Default self-contained bytecode-VM runner. |
| `tree-walker` | Analysis-aware Tcl-object code generation with exact source-span runtime fallback. |
| `generic-invoke` | Opt-in, literal-safe executable-IR argv transport used to exercise common semantic selection. |

`codegen::wasm::wasm_codegen_compilation_unit` is the analysis-aware entry
point used by `tcl compwasm --backend tree-walker`. It consumes the existing
`CompilationUnit`, including its lowered IR, control-flow graph, static
single-assignment facts, sparse conditional constant propagation results, type
lattices, and original concrete-syntax-tree-derived source spans.

`wasm_codegen_module` remains available for callers that only have an IR
module. It emits structured control flow and source-evaluation fallback, but
does not attempt direct calls because it has no binding proof.

`compile_literal_safe_wasm` is the current executable-IR entry point behind
`--backend generic-invoke`. It builds common invocation facts, selects a
generic argv plan through `BackendRegistry`, and either returns a complete
module or a typed decline.

The two compiler paths share WASM IR and runtime ABI infrastructure, but their
selection mechanisms have not yet been unified. The tree-walker uses
registry-stamped `WasmCodegenHookId` values together with existing analysis
proofs. The generic-invoke path selects by target-neutral
`SemanticOperationId`. Existing tree-walker behaviour remains the baseline
until equivalent common facts and plans replace each specialisation.

## Target-independent stages

### 1. Preserve Tcl word semantics

Lowering retains each word as a structured `WordExpr`. Literal, substituted,
expanded, and opaque words remain distinguishable. A backend must not recover
meaning by reparsing a source string.

Relevant modules:

- `tcl-compiler/src/ir.rs`
- `tcl-compiler/src/registry_invocation.rs`

### 2. Build executable semantic control flow

`executable_ir` makes Tcl evaluation order and completion flow explicit. Each
word evaluation, argument expansion, argv assembly, and invocation produces a
completion. A `CompletionSwitch` separates `TCL_OK` from error, return, break,
continue, and custom integer completion codes. A slow path consumes the argv
already built by the fast path; it never repeats substitutions.

The compatibility builder preserves multi-statement sequencing. Assignments,
increments, expression evaluation, and return become registry-identified
`ExecuteLowered` operations. Structured bodies and control forms that do not
yet have an exact executable CFG become typed `ExecuteOpaqueRegion`
operations. An opaque region retains its source IR and provenance, but common
analyses give it conservative completion and all-world effects; they do not
inspect its payload to rediscover command semantics.

If legacy IR lacks exact word structure even for bounded forms, the builder
returns a typed source-compatibility decline rather than inventing semantics.

### 3. Resolve through `CommandRegistry`

Structured words are passed to `CommandRegistry`. The resulting
`InvocationFacts` snapshot owns the selected command, subcommand, and form
outcome together with:

- `SemanticOperationId`;
- argument roles and transfer facts;
- exact completion-code and payload obligations;
- cell and world-state transitions with completion-edge commit policy;
- effect footprints and re-entrant callback information; and
- mutable dispatch dependencies.

Dynamic or expanded command and subcommand words remain typed unresolved
outcomes. No compiler pass substitutes command-name matching for a missing
registry descriptor.

### 4. Establish common legality proofs

The current function view joins the existing scalar SSA, optional memory SSA,
and the semantic sidecar's executable world-state SSA without duplicating
their ownership. Registry-declared world writes that depend on Tcl completion
are attached to CFG edges, not treated as unconditional predecessor
statements. Lowered and opaque compatibility operations currently widen the
world conservatively because their precise transfer facts have not yet been
projected.

Representation-planning types describe string and internal representations,
copy-on-write obligations, ownership, materialisation boundaries, suspension,
and safepoints. They are not yet a connected proof-producing optimisation
pipeline. Current generated code retains runtime Tcl objects and a
runtime-allocated call frame; frame removal and single-representation variable
lowering are future optimisations.

These facts belong in the shared layer so the LSP, TclVM, WASM, eBPF, and
future targets can consume one result. Those consumers have not all migrated
to the semantic sidecar. Absence of a proof is a refusal, not an optimistic
default.

### 5. Select a target plan

`BackendRegistry` models the legalisation ladder:

```text
direct target operation
  -> shared-runtime intrinsic
  -> generic invocation with prebuilt argv
  -> eval only for a genuinely dynamic or opaque script region
```

A selector receives immutable semantic facts, target capabilities, and an
immutable backend context. It either returns a complete plan or a typed
decline. It cannot partially write to an emitter and then fall through.

The current executable-IR WASM pipeline registers only the generic-invocation
rung for its literal-safe bounded shape. Direct and runtime-intrinsic entries,
opaque-region evaluation, guards, deoptimisation, and
representation-changing plans are not connected to this emitter.

Target profiles describe execution, value, control, memory, runtime, and
resource constraints. WASM and native CPU targets may use the shared runtime.
eBPF accepts only verifier-safe closed regions. GPU and FPGA host continuation
is legal only at an explicit enclosing offload boundary, never as an arbitrary
per-operation escape.

Relevant modules:

- `tcl-compiler/src/backend_registry.rs`
- `tcl-compiler/src/target_contract.rs`
- `tcl-compiler/src/representation_plan.rs`

### 6. Emit and invoke the shared runtime

The destination emitter consumes only a selected plan. WASM ABI layouts and
imports are centralised in `tcl-runtime-api`; the compiler and runtime do not
maintain independent constants.

The generic invocation ABI accepts the complete, already-evaluated argv. The
runtime calls its normal interpreter dispatcher, so namespace lookup, aliases,
ensembles, `unknown`, safe-interpreter policy, and TclOO use the same code as
interpreted execution. It returns the full completion triple:

```text
(completion code, owned result object, owned return-options object)
```

Generated generic-invoke code uses a runtime-allocated call frame for argv
handles and the completion output structure. Frames are re-entrant and
validated by the runtime allocator; fixed low-memory scratch is not used. The
generated callee retains the outbound result and options before releasing its
private completion, argv objects, and frame.

Relevant modules:

- `tcl-compiler/src/codegen/wasm/executable.rs`
- `tcl-compiler/src/codegen/wasm/pipeline.rs`
- `tcl-compiler/src/codegen/wasm/ir.rs`
- `tcl-runtime-api/src/codegen_abi.rs`
- `runtime/rust/src/codegen_abi.rs`

## Analysis-aware tree-walker

### Direct-emission proof

The established tree-walker specialises a statement only when all required
compiler facts agree:

1. Lowering produced a typed IR statement or expression AST the emitter
   supports.
2. The flow-sensitive command-binding lattice proves that a builtin or user
   procedure name still denotes the expected command at that statement.
3. The whole-module command-mutation summary proves no procedure body can
   rename or alias that binding later.
4. A direct arithmetic procedure has a numeric return in the type lattice and
   a supported expression tree.
5. The command's `CommandSpec.wasm_codegen_hook` selects the runtime operation.

If any proof is absent, the structured walk passes the statement's original
source span to `tcl_eval_code`. This keeps dynamic Tcl semantics as the
conservative compatibility boundary for this path.

### Tcl-object stack and variables

Generated values are `i32` pointers to owned `TclObj` values in shared linear
memory. Procedure parameters are WebAssembly parameters of that same type.

Compiled procedure locals have two ports onto one runtime variable cell:

- an indexed slot used by generated `local_get` and `local_set` operations;
- the ordinary Tcl name used by traces, `upvar`, and interpreted fallback.

The procedure prologue binds each slot index to its Tcl-visible name in the
normal call frame. This preserves Tcl semantics before future escape and
representation passes prove that a variable can become a plain WebAssembly
local.

### Current direct tier

The direct tier covers:

- literal top-level `set` assignments;
- lowered procedure registration without evaluating the `proc` command;
- fixed-arity procedures whose body is a supported numeric return expression;
- variable reads through indexed procedure slots or named top-level cells;
- Tcl numeric-tower addition;
- binding-proven direct user-procedure calls in command substitution; and
- the one-argument stdout form of registry-stamped `puts`.

For example:

```tcl
proc add {b c} {
    return [expr {$b + $c}]
}

set e 2
set f 4
puts [add $e $f]
```

The procedure is emitted as an `(i32, i32) -> i32` WebAssembly function. The
top-level function registers its source metadata, stores `e` and `f`, loads
both values, calls the generated `::add` function, and passes its result to the
runtime `puts` primitive. None of those statements calls `tcl_eval_code`.

### Runtime ownership contract

The codegen ABI in `runtime/rust/src/codegen_abi.rs` uses one owned reference
for every generated operand-stack value:

| Operation | Ownership |
|---|---|
| `tcl_value_new_string` | Returns `+1`. |
| Variable load | Returns a new `+1` beside the cell's reference. |
| Variable store or bind | Consumes the operand `+1`; the cell retains its own. |
| Arithmetic add | Consumes both operands and returns `+1`. |
| Direct procedure call | Transfers argument references to the callee. |
| Procedure return | Transfers one result reference to the caller. |
| `tcl_codegen_puts` | Consumes its value. |

Procedure frame push and pop use the runtime's ordinary `FrameStack`; popping
a frame releases its stored variable references.

## Current executable-IR generic-invoke slice

The `generic-invoke` backend is intentionally narrower than the executable
compatibility builder. The builder can preserve a sequence of calls, lowered
operations, and opaque structured regions. The WASM planner accepts exactly
one flat invocation whose words are literal-safe. It selects the generic argv
rung through `BackendRegistry` and emits a module importing
`tcl_invoke_argv`. It preserves the registry semantic operation even though
the selected implementation is generic.

The planner returns typed declines for dynamic words, `{*}` expansion,
multiple invocations, `ExecuteLowered`, `ExecuteOpaqueRegion`, and unsupported
CFG or instruction shapes. These declines are backend decisions; they do not
erase common facts and are not silently converted to source-level `eval`.

The generic ABI dispatches through the runtime's current interpreter. That is
the required foundation for namespace, alias, `unknown`, TclOO, safe-policy,
and trace behaviour, but the bounded compiler path is not proof of broad
surface parity. It does not compile Safe Base scripts or provide a
Safe-Base-specific plan; safe and child interpreter coverage remains a
separate runtime concern.

The common eBPF bridge is similarly conservative and disconnected from this
pipeline. It audits candidate semantic bundles, rejects lowered or opaque
instructions and unsupported control flow, and may produce a sealed
eligibility record. The established BPF-Tcl lowerer and eBPF emitter do not
consume that record yet.

The CLI exposes the bounded path explicitly:

```sh
tcl compwasm --backend generic-invoke \
  --source 'string length hello' \
  --dialect tcl8.6 \
  -o out.wasm
```

## Module layout

Runtime functions are imported first. `::top` is the first defined function,
followed by user procedures in qualified-name order, so direct call indices
are deterministic. A relocated module places its constant pool at
`RESERVED_DATA_BASE`, inside the memory window reserved by the Rust runtime.

The WAT renderer and binary encoder both consume the same `WasmModule` IR in
`rust/tcl-compiler/src/codegen/wasm/ir.rs`.

## Migration and adding support

New command knowledge belongs in the registry and common semantic layer. Do
not add command-name branches to an emitter. The existing tree-walker hook is
a live compatibility interface; remove or replace a hook only after the
equivalent common plan has behavioural and performance parity.

When a Tcl surface is missing:

1. enrich the appropriate `CommandSpec`, subcommand, form, or shared registry
   descriptor;
2. project the new target-neutral fact in common analysis;
3. register a target implementation by `SemanticOperationId`, or select
   generic argv invocation;
4. add runtime backing when the operation crosses the shared runtime boundary;
5. test exact Tcl completion, state, trace, ownership, and re-entrancy
   behaviour; and
6. keep emitters free of command spellings, Tcl list parsing, option parsing,
   and runtime semantic decisions.

Pure list, string, numeric, boolean, option, formal-parameter, and completion
algorithms belong in shared crates below runtime adapters. A target-specific
copy of such an algorithm is not a code-generation hook.

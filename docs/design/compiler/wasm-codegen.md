# WASM code generation

> **Status:** `codegen::wasm::compile_wasm` is the sole public Tcl-to-WASM
> code-generation entry point. It selects a semantic plan from executable IR
> first and records a typed compatibility reason when broad source-compatible
> emission is still required. The **native tier**
> (`WasmCompileOptions::native_tier()`, plan §7 row P3) lowers every function
> through the native lowered IR and emits it natively — every T0 and T1
> sample but `15_switch` compiles with no `tcl_eval_code` and no
> `tcl_expr_bool` — but it, the
> guarded boxed `string length`, and the exact sealed native-i64-add
> demonstration are all explicitly opt-in; every semantic AOT pass is off by
> default. `--analysis` still selects the legacy analysis tier; `--native`
> selects the native tier.

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
not Rust debug output. `codegenPlan.nativeLowering` carries the native tier's
record: per function `lowered` or `declined` with its reason, and per
statement the outcome (`native`, `native-intrinsic`, `native-completion`,
`generic-invoke`, `eval-source` with its reason, `empty`), the representation
kinds of the values it defines, and every cell access with its storage, its
trace-barrier decision, and whether it reused a shadow.

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
declines. No source word is replayed. The selected `string length` operation is
not evidence that other Tcl 8 string operations are safe to specialise: Tcl 8
indices can denote isolated UTF-16 surrogates that Rust `String` cannot retain.
The exact oracle and required shared-value representation are documented in
the [semantic AOT contract](semantic-aot-optimisation.md#tcl-8-supplementary-character-boundary).

The sealed native-add mode emits an exported native `(i64, i64) -> i64`
procedure and a no-argument top function containing the two proved constants.
It imports only the wide-integer boxing and channel-write boundaries used by
that slice. A failed common premise declines before any native instruction is
emitted; this slice has no in-activation guard or deoptimisation edge.

Until executable IR covers the complete Tcl surface, a typed decline invokes
general structured mode in the same `WasmEmitter`. It preserves structured
control flow, direct Tcl-object operations where its established proofs permit
them, compiled prebuilt-argv invocation for every leaf command whose words it
can evaluate, and exact source-span runtime evaluation only for what is left.
The decline changes typed input to one emitter; it does not select another
implementation. This keeps runtime-evaluation compatibility working while
making remaining semantic coverage observable. It does not imply
package-driven extension selection or linking.

## The native tier

`WasmCompileOptions::native_tier()` enables four semantic passes together —
`NativeLowering`, `RepresentationInference`, `TraceBarrierElision`,
`CellDemotion` — and routes general mode through
`tcl_compiler::native_lowering` and `codegen::wasm::native_emit` instead of the
structured walk. It supersedes the leaf-command path below for every function
the lowering accepts; the legacy walk remains the path for a function the
lowering declines, with the typed `FunctionDecline` recorded.

### Lowering

Every executable block becomes one NLIR block and every executable instruction
one statement owning that instruction's completion, so the completion spine of
the executable IR — every `CompletionSwitch`, every handler edge — survives
unchanged. Statements lower by registry descriptor and dispatch proof:

| Executable instruction | Native shape | Proof |
|---|---|---|
| `ExecuteLowered(Set)` | `CellWrite` of a boxed constant, word, or expression | statically spelled cell name |
| `ExecuteLowered(Incr)` | native `i64.add` on the cell's shadow when the interval proof admits it, else `CellIncr` (the runtime's own `incr`) | shadow live and in range |
| `ExecuteLowered(Expr)`, conditions | native `i64`/`f64` arithmetic, comparisons, `&&`/`||`/`?:` branches; dynamic operations with a native fast path and the runtime operator on the slow edge; the runtime expression intrinsic for `[cmd]` operands | representation lattice |
| `Invoke` resolved to `NativeLowering::Intrinsic(ChannelWrite)` | `Puts` | site dispatch proof, `puts VALUE` shape |
| `Invoke` resolved to `NativeLowering::Completion` | `Complete{break/continue}` | site dispatch proof |
| `Invoke` resolved to `NativeLowering::CellReadModifyWrite` | `CellIncr` / `CellAppend` | site dispatch proof, literal name |
| every other `Invoke` | `Invoke(argv)` — the prebuilt-argv path | none needed |
| `ExecuteOpaqueRegion`, a word with expansion, backslashes, a computed name | `EvalSource(text, reason)` — the last rung | — |

Words are evaluated structurally from `WordExpr` only: the `puts` fast path
that reparsed compatibility text (issue #1772) is gone from every tier, and a
`[…]` word resolved to `expr` over one braced literal lowers as a native
expression when the module keeps `expr` bound to its builtin. The one reading
a backend still makes of a `$…` spelling — which cell a variable word names,
and whether `$a(b)` is an element or a scalar spelt `${a(b)}` — has a single
owner, `native_lowering::cells::variable_word_place`, built on
`tcl_syntax::naming::split_element_ref`; `cell_place` is its counterpart for a
statically spelled name word. No tier keeps a copy.

### Representation and cells

Each value carries `NativeInt(interval)`, `NativeDouble{finite}`,
`NativeBool`, or `Boxed(shape)`. A native integer operation is emitted only
when the interval proof shows every result fits `i64` and every precondition
holds (a non-zero divisor, an in-range shift); otherwise the operation is a
dynamic op: the emitter tries the `i64` path (checked, with Tcl's floor
division and modulo), then the `f64` path, and takes `tcl_codegen_mathop` on
the slow edge — the runtime's own bignum and error semantics. Nothing wraps
and nothing traps. Boxing happens at the boundary that needs a Tcl object
(`puts`, a cell write, an argv word); unboxing through the erroring
`tcl_value_get_*` getters is C Tcl's exact error.

Top-level variables stay named cells written at the defining statement (a
hosted module's globals stay observable), but the value *between* statements
is the NLIR shadow: a later read reuses it unless a trace barrier was kept, an
invocation may have written the cell, or control joined from more than one
path. Trace barriers are decided from the module's variable-trace ledger:
elided when no literal or dynamic `trace` target can reach the cell, kept
with `variable-traced` or `trace-ledger-unknown` otherwise (an `incr` under an
unknown ledger records the runtime trace-bit guard).

### Emission

`native_emit.rs` structurises the NLIR CFG with the dominator-tree algorithm
of *Beyond Relooper*: merge nodes get a `block` opened by their immediate
dominator, loop headers a `loop`, everything else is inlined where its one
forward predecessor branches. Every value is one WASM local of its machine
type; boxed locals are owned, released on redefinition and in the epilogue.
How a function is entered depends on its **entry protocol**
(`NativeFunction.protocol`), and only two exist. The module's own entry point
`::top` is `() -> ()`: it enters an activation
(`tcl_codegen_activation_enter`), allocates one transient call frame for its
typed out slots, completion triple, and argv array, and leaves the activation
with its completion code. A **procedure body** is
`(argv, argc, out) -> status` and is *prologue-free*: it allocates the same
transient frame and nothing else. It must not enter an activation or push a
Tcl call frame, because `Interp::run_proc` has already pushed the variable
frame in the proc's own namespace, recorded `info level`'s words and bound the
formals by name, and `Interp::run_native_body` holds the activation and the
`CmdFrame`. Emitting the script prologue there would push a second, nameless
frame at the *caller's* namespace — `namespace current`, `upvar 1` and `info
level` all one level out — and halve the recursion depth Tcl allows.
`argv`/`argc` are the bound call arguments, reserved for P5's native formal
binder; a P5-lite body reads its formals as named cells.

A procedure body ends by writing its completion triple into `out` and
answering `NATIVE_PROC_STATUS_RAN`. A **null** result there is not an omission:
it means the runtime's own current result is the body's answer, which is what
an evaluated source rung leaves and what every error edge leaves. So the
emitter materialises the Tcl result of the two operations that leave neither —
`set` answers with the value it stored, `incr` with the new one — and **declines
to bind** a body whose returned completion determines no result at all: an
`append`/`lappend` reaching the in-place cell shape, or a structured region
(`if` with no `else`) whose completion the executable IR produces empty. Such a
body is still emitted; it is simply not installed, and the definition keeps its
source body. `FunctionReport.binding` records which, and the Explorer shows it.

A compiled `return` is the `return` command, not merely its completion code:
it records the pending `-level 1 -code ok` state through
`tcl_codegen_return_state` before completing with `Return`. The enclosing
procedure's return boundary consumes that state whether or not anything set
it, so without the write a `catch {return -level 2 …}` anywhere earlier in the
program leaves a level behind and the *next* compiled `return` propagates a
return instead of its value. Every option-carrying `return` keeps the generic
invocation, which records the state itself.

Each statement also logs its own `errorInfo` frame on its error edge
(`tcl_codegen_log_command`, with the statement's exact text and its line within
the body it was compiled from). Generated code reaches no eval loop, so without
this a failing compiled statement leaves no `while executing "<text>"` frame,
never advances `errorLine`, and — because `error_stack_push_call` will not chain
a `CALL` onto an error stack with no inner context — loses the TIP 348 `CALL`
entry as well. The runtime owns the `already_logged` protocol, so the innermost
statement of a nest logs and the rest are no-ops, exactly as C dedups within one
bytecode frame.

### Binding a compiled body to its procedure

A `proc` statement whose body became one of this module's functions lowers to
the **definition** shape rather than a generic invocation
(`NativeLowering::Definition`, the registry descriptor `proc` has always
carried): `tcl_codegen_proc_define_native(name, params, body, entry)`, where
`entry` is the body's index in the runtime's shared function table, or `0` for
"source body only". The runtime then binds both — the written source body,
exactly as `proc` does, and the compiled entry beside it.

`::top` owns the install. Its prologue, guarded on a module global that starts
at `-1` so a second call is a no-op, grows the shared table once by the
module's entry count, keeps the returned base in that global, and writes one
`ref.func` per bound body; `table.grow` answering `-1` is a runtime linked
without `--growable-table`, and the module traps there rather than dispatching
to whatever sits at slot 0. Each definition statement then reads its entry as
`base + slot`. `ProcedurePlan` is the single owner of proc → function index and
proc → table slot; `ProcDef` owns proc → entry on the runtime side.

Every word a definition registers has to be a word the statement writes out
literally. `Procedure` records the *written* name, parameter list and body
text, but lowering may have compiled the body from a value it materialised
instead — a const-mapped `$body`, or a `[subst -nocommands …]` template — and
it records the original word beside that compiled body. Registering that word
would report the wrong `info body` and, worse, make any later run of the source
body (a step trace, or a declined entry) evaluate the substitution *in the
procedure's own frame*, where its operands do not exist. A substituted name,
parameter list or body therefore keeps the generic invocation, and the
runtime's own `proc` — which evaluates the word at the call site, as Tcl does
— defines the procedure.

Two consequences fall straight out of where the front end keeps procedures.
Lowering keeps the **first** definition of a name (a later `proc p` only
records a redefinition), so only that statement can name a compiled body and a
second `proc p` stays a generic invocation — which installs an ordinary
source-only procedure at run time, which is what makes a mid-script redefinition
behave. And a module whose `::top` stayed on the legacy path installs nothing at
all, since the install sequence lives there; every definition it emits passes
`entry = 0`.

### What still declines

`foreach`/`lmap` cursor loops, `switch` pattern matches, and `catch`/`try`
completion handlers are executable-IR instructions the native lowering does
not project yet (`iterate-lists`, `match-pattern`, `join-completion`,
`write-completion-cell`, `operand-expression`); a function containing one
stays on the legacy structured walk with that reason. Procedure bodies lower
with named cells (no slot storage yet).

A `proc` statement only takes the definition shape while its own dispatch is
still proven, so a definition written after a command that widens the world
state — a `catch`, a `namespace eval`, anything the tier lowers as an opaque
region — keeps the generic invocation and its body stays source-only. In
practice that means the definitions a script wants compiled belong near its
top.

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

### The shared function table

Calls in the other direction — the runtime calling a function an emitted
module defines — go through one wasm table the **runtime** owns.
`runtime/rust/build.rs` links every wasm target with `--export-table` and
`--growable-table`, so `tcl_runtime.wasm` publishes `wasm-ld`'s indirect
function table as `__indirect_function_table` (the name
`WASM32_FUNCTION_TABLE_IMPORT` owns) with no maximum. A module that wants the
runtime to call one of its functions imports that table, `table.grow`s room
for its own entries, `ref.func`s them into the new slots, and passes the slot
index across the ABI — a wasm32 function pointer *is* such an index, so the
runtime calls it with an ordinary indirect call.

Two link-time failures are worth naming because their symptoms are far apart.
A runtime linked without `--export-table` fails *instantiation* of any module
that imports the table (`unknown import`), so a module imports it only when it
actually installs an entry; one linked without `--growable-table` keeps
`min == max` and `table.grow` answers `-1`, which the installing module must
treat as a build error rather than proceeding with a bogus base.

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
- `tcl-compiler/src/codegen/wasm/leaf_invoke.rs` — non-emitting per-statement
  prebuilt-argv planning, frame layout, and typed word declines;
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

A module that installs at least one compiled procedure body additionally
imports the runtime's `__indirect_function_table`, defines one mutable `i32`
global for the base of its window in it, and carries a declarative element
segment naming every function a `ref.func` may name. All three are absent from
a module that installs none, so such a module still instantiates against a
runtime with no exported table. `_start` (standalone packaging) and a host
calling `::top` both reach the install, because it is `::top`'s own prologue.

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

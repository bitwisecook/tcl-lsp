# WASM native lowering

> **Status:** the architecture behind the opt-in native tier of
> `compile_wasm` — the native lowered IR, its lattices and framing elision,
> the runtime ABI it targets, the sample tiers and framing budgets that gate
> it, and the corpus evidence behind its priorities. The tier's emitted
> shapes are documented in [wasm-codegen.md](wasm-codegen.md#the-native-tier).

## 1. Goal and governing rule

A script doing simple work — `set a 1; incr a; puts $a`, a loop over a list,
a leaf proc with integer arithmetic, a small TclOO class — should compile to
native WASM: `i64`/`f64` locals, structured control flow, direct calls,
runtime intrinsics for value operations, and one boxed `TclObj` at each
boundary where the runtime must see a Tcl value. Tcl framing (a named
variable cell, a Tcl call frame, the completion triple, an execution-trace
boundary, an interpreter dispatch) is retained only where an observer can
reach it: a trace, `upvar`/`uplevel`, `info`, a coroutine, a rename, a
callback into the interpreter.

The governing rule is the one in
[`semantic-aot-optimisation.md`](semantic-aot-optimisation.md): a registry
resolution is a candidate, not a proof. What the native tier changes is the
*default shape* of the output. The legacy tiers eval the source text unless a
narrow proof admits something better; the native tier lowers every statement
to its native shape with explicit framing operations, removes each framing
operation whose observer is provably absent, and keeps or guards the rest —
so every unproved fact costs one framing operation rather than the whole
statement.

One runtime. Every semantic operation the compiled code needs already has an
implementation in `runtime/rust`; the native tier adds *transport* (typed
intrinsic entry points, indexed slots, activation records, a function table)
and *bookkeeping* (per-cell trace bits, an expression cache) to that runtime,
never a second implementation of a Tcl command in the emitter. WASM-specific
runtime code is confined to `runtime/rust/src/codegen_abi.rs`,
`codegen_native.rs`, and the `Frame`/`Interp` fields they need.

## 2. Current state

### 2.1 Input modes

`compile_wasm` (`rust/tcl-compiler/src/codegen/wasm/pipeline.rs`) selects one
input mode for the single emitter in `backend.rs`:

| Mode | Fires when | What it emits |
|---|---|---|
| `NativeI64Add` | five opt-in passes **and** `for_sealed_program()` **and** the exact four-statement `proc add {b c}` shape | one `(i64,i64)->i64` function plus a boxed `puts` |
| `GenericInvoke` / `GuardedIntrinsic` | the whole script is one all-literal invocation | one prebuilt-argv `tcl_invoke_argv` (optionally guarded `string length`) |
| native tier | `WasmCompileOptions::native_tier()` (`tcl compwasm --codegen-passes native-tier`), for every function `native_lowering` accepts | NLIR emitted natively by `native_emit.rs` |
| `General` | everything else | structured `if`/loop scaffolding around per-statement emission |

Inside `General`, the per-statement ladder in
`WasmEmitter::try_emit_typed_statement` (the *analysis* tier) is reachable
only when `SemanticOptimisationPassId::LegacyAnalysisSpecialisation` is
enabled. **It is off by default**, including for `tcl compwasm`, the
Explorer, and the fuzzer. With default options the structured walk in
`codegen/structured.rs` sends every `Statement::Call`, every assignment,
every `foreach`/`switch`/`catch`/`try`, and every `return` to
`Emit::emit_command`, which is `tcl_eval_code` over the statement's source
span; conditions of `if`/`while`/`for` are `tcl_expr_bool` over the
condition's source text in both legacy tiers. The analysis tier moves leaf
commands from re-parsed source to prebuilt argv, but its *values* are still
boxed strings, its *conditions* are still parsed at run time, and its
*procs* are still interpreted from source.

### 2.2 Framing budgets and the differential harness

`samples/wasm/` holds deterministic scripts in eight tiers (T0 straight-line
… T7 dynamic), each with its `tclsh9.0` oracle output under `expected/`.
`samples/wasm/budgets.tsv` is the golden framing budget: per sample and per
plan (`default`, `analysis`, `native`) the count of `call` sites reaching
`tcl_eval_code`, `tcl_expr_bool`, and `tcl_invoke_argv`, and the count of
native 64-bit numeric instructions. A number that goes up without a matching
design change is a framing regression; regenerate with
`UPDATE_WASM_BUDGETS=1 cargo test -p tcl-compiler --test wasm_tiers
framing_budgets`. Every T0 and T1 sample but `15_switch` compiles under the
native plan with no `tcl_eval_code` and no `tcl_expr_bool`.

`rust/tcl-compiler/tests/wasm_tiers.rs` links every sample against
`tcl_runtime.wasm` (`wasm32-wasip1`, `--global-base=0x200000`, `wasmtime
--preload`, as `wasm_real_link.rs` does) and diffs stdout against the oracle.
`EXPECTED_DIVERGENCES` is the self-cleaning ledger of known defects: a sample
that diverges without an entry fails, and a sample with an entry that stops
diverging also fails. Its one entry is `73_coroutine`: the wasm build refuses
`coroutine` outright ("coroutines are not supported in the single-threaded
wasm build").

### 2.3 A tier may only register a definition it can quote

Both tiers that register a `proc` themselves — the native tier's
`NativeOp::DefineProc` and the general tier's `tcl_codegen_proc_register` —
hand the runtime a name, a parameter list and a body text taken from the
surviving `ir::Procedure`. `Procedure` records the **written** words. Lowering
may have compiled the body from a value it materialised instead — a
const-mapped `$body`, or a `[subst -nocommands …]` template — and it records
the original word beside that compiled body rather than the text it compiled.

Registering the written word in that case is wrong twice over: `info body`
reports the substitution rather than the body, and any later run of the source
body — a step trace, or a declined native entry — evaluates that substitution
**in the procedure's own frame**, where the variables it names do not exist.

A tier therefore registers a definition only when the statement wrote every
word out and wrote the parameter list and body the `Procedure` recorded;
anything else keeps the generic invocation, where the runtime's own `proc`
evaluates the word at the call site as Tcl does. It is one predicate,
`native_lowering::lower::definition_words_are_written_out`, called by both
tiers. The general tier's path is unreachable today: a substituted word in a
definition also defeats the enclosing function's command-binding proof, so no
`StructuredLowering(Proc)` operation is recorded for the statement at all.
That is two independent proofs happening to agree, not a design.

### 2.4 What each tier consumes

The analysis tier's `function_facts` (`backend.rs`) builds
`direct_assignments`, `operations`, `direct_calls`, and `leaf_invocations`
keyed by source span, from the command binding lattice and the registry
resolution; it consults none of `FunctionUnit::types`, `sccp`, intervals,
`var_escape`, `memory_ssa`, `world_state_ssa`, or the dispatch-stability
proof. The whole-function `GenericInvoke` plan and the per-statement leaf plan
emit the same argv/completion sequence twice (`finish_semantic_invoke` vs
`emit_invoke_node`). Direct procs are limited to `Var`/`Literal`/`+`.

The native tier consumes the front-end facts: the executable semantic IR with
its completion spine, `types` and intervals for the representation lattice,
the module's variable-trace ledger for barrier elision, `var_escape` for cell
demotion, and the dispatch-stability proof for every registry-resolved site.

The bytecode backend (`codegen/{statements,values,expressions,cmd_subst,
control_flow,emitter}`) compiles expressions natively from `ExprNode`, keeps
proc locals in indexed slots, and specialises 32 command forms — 15
statement-position `CodegenHookId`s and 17 value-position
`InlineCodegenHookId`s in `tcl-registry/src/hooks.rs`, dispatched by typed
hook, never by name. The WASM tiers share none of it below the `Emit` seam.
The `NativeLowering` descriptors in §3.3 are the target-neutral form of the
same knowledge; the bytecode backend does not consume them.

### 2.5 Runtime limits that bound any compiled fast path

1. **No command cache in dispatch.** `dispatch_inner` copies the command
   name into a fresh `Vec<u8>` per call and walks the namespace path;
   `CmdArena` (dense `u32` ids) exists but dispatch does not use it. The
   `CommandEnvironment` guard epoch is exactly the validation a direct-call
   handle would need.
2. **`run_proc` cost.** Renders every argument to bytes for `info level`
   unconditionally and binds parameters by name; a compiled body is entered
   through it.
3. **TclOO rebuilds the call chain on every method call**, and
   `GuardDomain::ObjectDispatch` is poisoned at interpreter creation, so no
   TclOO fast path can be guarded.
4. **`execute_intrinsic` has one arm** (`StringLength`), while `IntrinsicId`
   declares about twenty list/dict/string operations.
5. **Coroutines** are refused in the wasm build; `after`/`vwait` and `clock`
   are host gaps (see [wasm-target-surfaces.md](wasm-target-surfaces.md)).

## 3. Architecture: lower first, then prove framing away

### 3.1 Pipeline

```text
CST + structured words
  -> common semantic IR / executable CFG (executable_ir.rs)
  -> value SSA, cell SSA, world SSA, SCCP, types, intervals, escape, dispatch proofs
  -> NATIVE LOWERING (native_lowering/lower.rs, target-neutral)
       every statement -> native ops + explicit framing ops
  -> FRAMING ELISION (native_lowering/{representation,cells,elide}.rs)
       each framing op removed, guarded, or kept with a typed reason
  -> backend plan (BackendRegistry)
  -> WASM emitter (codegen/wasm/native_emit.rs, consuming NLIR, not source spans)
  -> shared Rust runtime ABI (§4)
```

The lowering and the elision passes live in `tcl-compiler` above `codegen/`
and are target-neutral. Every decision is recorded per statement in a
`FunctionReport`, which the Explorer serialises as `codegenPlan.nativeLowering`.

### 3.2 Executable IR coverage

`executable_ir.rs` makes word evaluation, argv assembly, invocation, and
completion explicit for plain calls, and projects `If`, `For`, `While`,
`Foreach`, `Catch`, `Try`, `Switch`, and `Return` into real executable edges:
loop headers with back edges and break/continue targets, `catch`/`try`
handler edges keyed by completion class (`CompletionSwitch`,
`JoinCompletion`), `switch` as a decision tree (`MatchPattern`), and
`foreach` as a list-cursor loop (`IterateLists`). `Block` and `UpFrame`
remain `ExecuteOpaqueRegion` barriers.

The native lowering does not yet project `IterateLists`, `MatchPattern`,
`JoinCompletion`, write-completion-cell, or an expression operand it cannot
type (`iterate-lists`, `match-pattern`, `join-completion`,
`write-completion-cell`, `operand-expression`); a function containing one
stays on the structured walk with that typed `FunctionDecline`.

### 3.3 Native Lowered IR (NLIR)

A statement lowers to a small vocabulary of typed operations over SSA values
with an explicit representation (`native_lowering/ir.rs`):

| Op family | Examples | Framing it carries |
|---|---|---|
| value construction | constants, `Box`, `Unbox{Int,Double,Bool}` | `Unbox` is a conversion that can fail: it has an error edge |
| native arithmetic | checked `i64` add/sub/mul, `i64` div/mod with Tcl rounding, `f64` arithmetic, comparisons, bitwise ops; a dynamic op with a native fast path and the runtime operator on the slow edge | none |
| cell access | `CellWrite(place)`, cell reads through the shadow, `CellIncr`, `CellAppend` | **cell framing**: the named Tcl cell; a `TraceBarrier` before and after |
| invocation | `Invoke(argv) -> completion`, `Puts`, `DefineProc` | **dispatch framing**: binding/namespace/trace domains |
| completion | `Complete(code)`, the completion switch | **completion framing** |
| source | `EvalSource(text, reason)` — the last rung | the whole statement |

The lowering is driven by registry data. Each `CommandSpec` form carries a
`NativeLowering` descriptor (`tcl-registry/src/native_lowering.rs`, a sibling
of `codegen_hook` and `inline_codegen_hook`):

- `Intrinsic { id, arity }` — pure or read-only value operation; arguments
  are values, result is a value.
- `CellReadModifyWrite(CellUpdate)` — `incr`, `append`, `lappend`: the
  var-write argument is a place, the rest are values, and the runtime
  intrinsic operates on the cell's object in place with copy-on-write.
- `Structured(LoweringHookId)` — `if`/`while`/`for`/`foreach`/`switch`/
  `catch`/`try`/`return`, projected by §3.2.
- `Completion(CompletionCode)` — `break`/`continue`.
- `Scope(ScopeKind)` — `global`, `variable`, `upvar`, `namespace upvar`: the
  named cells are frame-observable.
- `Definition` — `proc`: emit `DefineProc(native fn, source)` so the runtime
  binds both.
- `Generic` — everything else: `Invoke(argv)` exactly as the leaf-invoke path
  does.

### 3.4 Representation and cell lattices

Two lattices sit on the lowered IR and are computed once, target-neutrally.

**Value representation** per SSA value (`representation.rs`):
`NativeInt(interval)`, `NativeDouble{finite}`, `NativeBool`, `Boxed(shape)`.
It is seeded from `types` and intervals, refined by the checked-arithmetic
edges, and raised to `Boxed` at every operand of an `Invoke`, a `CellWrite`
to a frame-resident cell, or a completion. Boxing is inserted at the last
possible point; unboxing at the first use after a boxed read, with its
failure edge routed to the ordinary Tcl error path (never a trap). The Tcl 8
supplementary-character rule in `semantic-aot-optimisation.md` stays:
string-indexed intrinsics decline to `Invoke` on Tcl 8 dialects.

**Cell storage** per place (`cells.rs`): `Cell` (named cell, traces fire) is
the seed; a value stays in the NLIR shadow between statements unless a trace
barrier was kept, an invocation may have written the cell, or control joined
from more than one path. `CellDemotion` records the `Cell -> Slot` decision
for a proven-local procedure variable; slot emission is not implemented, so
procedure bodies still read their formals as named cells.

### 3.5 Framing elision

| Framing op | Removed when | Guarded when | Kept when |
|---|---|---|---|
| `TraceBarrier(cell)` | the module's variable-trace ledger proves no literal or dynamic `trace` target can reach the cell | the ledger is unknown and the runtime per-cell trace bit can be tested (`incr` records the trace-bit guard) | `variable-traced` or `trace-ledger-unknown` |
| checked `i64` arithmetic | the interval proof shows every result fits `i64` and every precondition holds (a non-zero divisor, an in-range shift) | — | otherwise a dynamic op: the `i64` path, then the `f64` path, then `tcl_codegen_mathop` on the slow edge |
| `Invoke -> Intrinsic` / `-> Puts` / `-> CellIncr` | the registry form resolves **and** the site's dispatch proof holds | — | the head is dynamic or the world state is widened |

Every decision is recorded per op with its typed reason, so the Explorer can
show "framing kept: cell `x` is `variable-traced`". The controls are the four
`SemanticOptimisationPassId`s the native tier enables together —
`NativeLowering`, `RepresentationInference`, `TraceBarrierElision`,
`CellDemotion` — each independently disableable and off by default.

### 3.6 What stays out of the emitter

`native_emit.rs` consumes NLIR and a plan; it never sees a command name, a
source span for evaluation (only for `errorInfo` line attribution), or a
compatibility string. `structured.rs` remains the driver for a function the
lowering declines; `EvalSource` is the last rung, reached only by an
`ExecuteOpaqueRegion`, an expanded word, a backslash-bearing word, or a
computed name.

## 4. Runtime ABI

All additions are in `runtime/rust` (`codegen_abi.rs`, `codegen_native.rs`),
declared through `tcl-runtime-api`'s `CodegenAbiImportId` so compiler and
runtime share one descriptor table.

- **Typed values.** `tcl_value_get_wide_int` / `_double` / `_bool` write the
  parsed rep back onto the object (a hot loop parses a spelling once) and
  return a Tcl error status on failure; the non-erroring
  `tcl_codegen_value_try_*` reads let generated code choose a native fast
  path without setting an error. The runtime's own `incr`/`append`/`lappend`,
  expression evaluator, `::tcl::mathop` operators, and `::tcl::mathfunc`
  dispatch run over prebuilt operands (`tcl_codegen_mathop`), so bignum
  promotion, copy-on-write growth, traces, and every error message are
  exactly interpreted Tcl's.
- **Cells and slots.** `tcl_codegen_slot_get`/`_set` and friends address a
  frame slot by index while `info vars`, `upvar`, and traces still find it by
  name; a `traced` bit on `Var` (set by `trace add variable`, cleared on
  removal) backs `tcl_codegen_slot_traced`, the runtime half of a guarded
  `TraceBarrier`.
- **Activations.** `tcl_codegen_activation_enter`/`_leave` push a real
  activation around compiled code so the eval loop's outermost-eval rule and
  `errorInfo` accumulation see it as one; `tcl_codegen_proc_define_native`
  binds a compiled body beside its source body through the runtime's shared
  indirect function table (see
  [wasm-codegen.md](wasm-codegen.md#binding-a-compiled-body-to-its-procedure));
  `tcl_codegen_return_state` and `tcl_codegen_log_command` give a compiled
  `return` and a failing compiled statement the same observable state as
  interpreted ones.
- **Expressions.** `parse_runtime_expr_cached` caches the parsed and
  validated `ExprNode` on the condition object, so the remaining
  `tcl_expr_bool` calls (dynamic or unbraced expressions) parse once.
  Compiled conditions do not use it: they are native.

## 5. Corpus evidence

Two corpora set the priorities in §3.

**The 21-repository AOT corpus** named in
[`aot-command-priority.md`](aot-command-priority.md), re-run with
`examples/aot_command_priority.rs`, matches the committed census within a
fraction of a percent (`set` 101,272 vs 101,221; `foreach` 11,307 vs 11,302);
the only large differences (`if`, and the `<`/`>`/`emit` rows) come from the
fresh run not applying the committed census's `filetypes.tcl` exclusion, a
generated data table. The top 25 forms cover about 81% of literal-head call
sites and the top 60 cover 92%. Every form in the top 25 is in T0–T5 of the
sample tiers.

**tcllib 2.0, the Tcl 9.0.4 script library, and `samples/`** were surveyed
for *shape*, not just frequency (759 cleaned tcllib files, 294k lines;
generated data tables excluded):

| Fact | Number | Consequence |
|---|---|---|
| `set` sites whose value is `[cmd …]` | 48% | intrinsic refinement and completion narrowing pay on half of all assignments |
| `expr` braced | 95–98% (tcllib 97.9%) | native expression lowering covers nearly everything; unbraced `expr` stays a slow path |
| `if` conditions braced | 98–99.8% | same |
| `expr` bodies with only `$var`/literal/operator | 45% (27% are exactly `$a op $b`) | representation inference on scalars is the main win, not intrinsic-in-expr |
| `if` conditions containing a `[cmd]` | 55% (`info exists`, `string …`, `llength`, `dict exists`, `catch`) | conditions must lower through the same intrinsic path as statements; a condition is not a special case |
| procs with no `expr` at all | 79% (tcllib) | "native" for most procs means slots, intrinsics, and direct calls, not arithmetic |
| straight-line procs (no control flow) | 41% (tcllib), 60% (samples) | frame elision applies to a large fraction of real procs |
| procs using `upvar` | **20%** of tcllib procs; half alias a caller-supplied name | linked cells and the full-frame plan are first-class, not exotic; `upvar 1 $name` needs a runtime link, never a compile-time alias |
| procs using `variable` | 26% (tcllib), 46% (tcl9 lib) | namespace cells with slots are as important as proc locals |
| computed variable names (`set $n`) | ~2% of `set` sites | the `DynamicVariableName` decline is acceptable as a per-statement fallback |
| `uplevel`/`eval $x`/`subst` | under 0.7% of sites | dynamic-script framing can be per-activation |
| loops | `foreach` 78%, `for` 21%, `while` mostly `while 1` | `foreach` as a cursor loop matters more than `for`/`while` |
| `foreach {k v} …` destructuring | 32% of `foreach` | multi-variable binding in the cursor loop from day one |
| error handling | `catch`/`return -code error`/`error` outnumber `try`/`throw` about 10:1 | `catch` and `return -code` before `try` |
| TclOO | 69 classes vs 7,800 procs in tcllib; 550 methods | correctness on the common shape matters more than breadth |
| TclOO features in class bodies | `superclass`/`variable`/`constructor`/`method`/`my`/`next` cover ~99%; `mixin` 6 uses, `filter` 0, `export` 2 | a light object frame targets exactly that shape; mixin/filter/objdefine fall back to chain dispatch |
| `oo::objdefine` per-instance tweaks | 59 in tcllib | a per-object "customised" flag is needed, not optional |
| object-as-namespace idiom (`variable ${self}::field`) | ~600 sites | namespace-variable cells with computed namespace names stay `Cell`, with a fast path when the namespace prefix is a proved value |

By site count the 25 commands `set if return expr variable string list lappend
foreach lindex dict info file llength array upvar incr append puts catch switch
error lrange while for` cover 65.5% of all 197k command sites in the shaped
corpus (71% once definition-time `proc`/`package`/`namespace`/`method` sites
are excluded); adding `unset lassign regexp format join split binary uplevel`
reaches about 74%. The tail is user-defined procs, which is what compiled
procedure bodies make cheap to call.

## Related

- [wasm-codegen.md](wasm-codegen.md) — the pipeline, the native tier's
  emitted shapes, and the runtime ABI contracts.
- [semantic-aot-optimisation.md](semantic-aot-optimisation.md) — the proof
  contract every elision in §3.5 must satisfy.
- [common-semantic-compiler.md](common-semantic-compiler.md) — the
  target-neutral IR this lowering consumes.
- [dispatch-stability-proof.md](dispatch-stability-proof.md) — the contents
  lattice behind trace and dispatch elision.
- [aot-command-priority.md](aot-command-priority.md) — the corpus census.
- [wasm-target-surfaces.md](wasm-target-surfaces.md) — WASI vs browser host
  limits that bound T7.
- [`samples/wasm/README.md`](../../../samples/wasm/README.md) — the sample tiers.

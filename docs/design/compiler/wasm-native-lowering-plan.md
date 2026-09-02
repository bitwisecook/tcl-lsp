# WASM native lowering: review and architecture plan

> **Status:** design plan, written and reviewed 2026-09-02 against
> `claude/wasm-codegen-architecture-5exvpu` (base `e11887f3`). It records a
> deep review of the current Tcl-to-WebAssembly code generator and the
> `runtime/rust` ABI it targets, the empirical baseline on the sample tiers in
> [`samples/wasm/`](../../../samples/wasm/README.md), the real-corpus evidence,
> and the phased architecture for compiling simple Tcl to native WASM while
> keeping Tcl framing only where the program makes it observable. Nothing in
> this document changes compiler output yet.

## 1. Goal and governing rule

The goal is that a script doing simple work — `set a 1; incr a; puts $a`, a
loop over a list, a leaf proc with integer arithmetic, a small TclOO class —
compiles to native WASM: `i64`/`f64` locals, structured control flow, direct
calls, runtime intrinsics for value operations, and one boxed `TclObj` at each
boundary where the runtime must see a Tcl value. Tcl framing (a named
variable cell, a Tcl call frame, the completion triple, an execution-trace
boundary, an interpreter dispatch) is retained only where an observer can
reach it: a trace, `upvar`/`uplevel`, `info`, a coroutine, a rename, a
callback into the interpreter. Over time the set of commands that still
compile to `tcl_eval_code` of their source text goes to zero.

The governing rule stays the one in
[`semantic-aot-optimisation.md`](semantic-aot-optimisation.md): a registry
resolution is a candidate, not a proof. What changes is the *default shape*
of the compiler's output. Today the default is "eval the source text unless a
narrow proof admits something better". The target is "lower every statement
to its native shape with explicit framing operations, then remove each
framing operation whose observer is provably absent, and guard or deoptimise
the rest". The proofs are the same; the direction of the ladder is reversed
so that every unproved fact costs one framing operation rather than the whole
statement.

One runtime. Every semantic operation the compiled code needs already has an
implementation in `runtime/rust`; the plan adds *transport* (typed intrinsic
entry points, indexed slots, activation records, a function table) and
*bookkeeping* (per-cell trace bits, a command-handle cache, an expression
cache, a call-chain cache) to that runtime, never a second implementation of
a Tcl command in the emitter. Where a change is WASM-specific it is confined
to `runtime/rust/src/codegen_abi.rs` and the `Frame`/`Interp` fields it needs.

## 2. What exists today: review findings

### 2.1 Three tiers, and the default is "eval everything"

`compile_wasm` (`rust/tcl-compiler/src/codegen/wasm/pipeline.rs`) selects one
of three input modes for the single emitter in `backend.rs`:

| Mode | Fires when | What it emits |
|---|---|---|
| `NativeI64Add` | five opt-in passes **and** `for_sealed_program()` **and** the exact four-statement `proc add {b c}` shape | one `(i64,i64)->i64` function plus a boxed `puts` |
| `GenericInvoke` / `GuardedIntrinsic` | the whole script is one all-literal invocation | one prebuilt-argv `tcl_invoke_argv` (optionally guarded `string length`) |
| `General` | everything else | structured `if`/loop scaffolding around per-statement emission |

Inside `General`, the per-statement ladder in `WasmEmitter::try_emit_typed_statement`
(`backend.rs:573`) is only reachable when `SemanticOptimisationPassId::LegacyAnalysisSpecialisation`
is enabled (`pipeline.rs`, `analysis_specialisations()`). **It is off by
default**, including for `tcl compwasm`, the Explorer, and the fuzzer. With
the default options the structured walk in `codegen/structured.rs` sends every
`Statement::Call`, every assignment, every `foreach`/`switch`/`catch`/`try`,
and every `return` to `Emit::emit_command`, which is `tcl_eval_code` over the
statement's source span. Conditions of `if`/`while`/`for` are always
`tcl_expr_bool` over the condition's source text, in both tiers.

The measured baseline on the 36 sample scripts confirms this. Counts are
`call` sites in the emitted module (`eval` = `tcl_eval_code`, `xbool` =
`tcl_expr_bool`, `argv` = `tcl_invoke_argv`; the analysis tier adds
`tcl_codegen_var_get`/`var_set` and the narrow `puts`/`expr_add`/direct-proc
paths). No sample emits a single native arithmetic instruction in either tier.

| Sample | default `eval`/`xbool` | analysis `eval`/`xbool`/`argv` | native ops |
|---|---|---|---|
| t0 `00_set_incr_puts` | 3 / 0 | 1 / 0 / 0 | 0 |
| t0 `02_arith_chain` | 7 / 0 | 3 / 0 / 2 | 0 |
| t1 `11_while_loop` | 5 / 3 | 2 / 3 / 0 | 0 |
| t1 `13_expr_ops` | 19 / 0 | 0 / 0 / 34 | 0 |
| t2 `20_lists` | 18 / 0 | 0 / 0 / 26 | 0 |
| t2 `21_foreach` | 9 / 0 | 4 / 0 / 2 | 0 |
| t3 `30_simple_proc` | 6 / 0 | 1 / 0 / 3 (+1 direct call, 1 `expr_add`) | 0 |
| t3 `31_recursion` | 8 / 2 | 4 / 2 / 4 | 0 |
| t4 `42_arrays` | 13 / 0 | 5 / 0 / 14 | 0 |
| t5 `50_catch_error` | 16 / 1 | 3 / 1 / 11 | 0 |
| t6 `60_class_basic` | 11 / 0 | 2 / 0 / 14 | 0 |
| t7 `70_var_traces` | 16 / 0 | 3 / 0 / 4 | 0 |

The full table for every sample is reproduced by the recipe in
`samples/wasm/README.md`. Read across a row: the analysis tier moves leaf
commands from re-parsed source to prebuilt argv (a real win: no lexing or
substitution at run time), but the *values* are still boxed strings, the
*conditions* are still parsed at run time, and the *procs* are still
interpreted from source.

### 2.2 Running the samples against the real runtime

Linking each sample against `tcl_runtime.wasm` (the `wasm_real_link.rs`
recipe: `wasm32-wasip1`, `--global-base=0x200000`, `wasmtime --preload`) and
diffing stdout against the `tclsh9.0` oracle gives:

| Result | default tier | analysis tier |
|---|---|---|
| byte-identical to tclsh 9.0 | 34 / 36 | 29 / 36 |
| diverges | `70_var_traces`, `73_coroutine` | those two plus `11_while_loop`, `20_lists`, `24_regex`, `41_upvar`, `50_catch_error` |

The two default-tier divergences are runtime bugs, not codegen: `incr` with a
read trace fires only `write` (issue #1633, row 3) and the wasm build refuses
`coroutine` outright ("coroutines are not supported in the single-threaded
wasm build"). The five extra analysis-tier divergences reduce to two defects
found by this review:

- **The `puts` fast path re-parses compatibility text.** `try_emit_direct_operation`
  (`backend.rs:~660`) admits `ChannelWrite` when
  `whole_var_reference(&args[0])` succeeds. For `puts "$p$q"` the argument's
  compatibility text is `${p}${q}`, and `whole_var_reference`
  (`codegen/values.rs:65`) strips the outer `${`…`}` and returns the variable
  name `p}${q`. The emitted `tcl_codegen_var_get` fails, the null return is
  treated as an error, and `::top` silently stops. Every failing sample has a
  quoted word with two substitutions in a `puts`. The same string-shape
  heuristics appear in `emit_var_get` (`name.contains('(')`) and the
  `AssignConst` gate. These are exactly the "reparse the argument string"
  shortcuts `common-semantic-compiler.md` forbids; the structured `WordExpr`
  the leaf-invoke planner uses gets this case right.
- **A compiled statement is not an eval-loop boundary.** `tcl_invoke_argv`
  dispatches at `eval_depth == 0`. When the dispatched command is `catch`, its
  body runs through `eval_control_body`, and the eval loop's outermost-eval
  rule (`interp.rs:5032`: publish and reset the error state when depth returns
  to 0 and the last code was `error`) fires *inside* the body evaluation,
  before `catch` reads `error_code()`. `catch {risky} r opts` then reports
  `-errorcode NONE`. Interpreted Tcl never hits this because the enclosing
  script eval holds depth ≥ 1. This is a runtime-side contract gap: a
  compiled activation must count as an activation.

Both are filed for roll-in in §8 and are fixed by the architecture in §3
(structured words everywhere; a real activation record around compiled code)
rather than by patching the heuristics.

### 2.3 The emitted proc functions are never executed

`codegen()` emits one exported WASM function per non-namespaced proc and
walks its body through the same structured driver. But the runtime learns of
the proc only through `tcl_codegen_proc_register(name, params, body_source)`,
which defines an ordinary source-bodied proc (`codegen_abi.rs:851`,
`interp.define_proc`). There is no function table, no
`call_indirect`, and no import that binds a proc name to a WASM function
index. A call to a user proc from compiled code becomes `tcl_invoke_argv`
(or `tcl_eval_code`), the runtime resolves the name, and `run_proc` re-parses
the *source* body on every call (`interp.rs:7016`). The only compiled proc
body that ever runs is a `DirectProc` — the single-statement
`return [expr {$a + $b}]` shape — called directly from top level. Every other
emitted proc function is dead weight in the module.

### 2.4 The analysis tier is a second, string-keyed proof channel

`function_facts` (`backend.rs`) builds `direct_assignments`, `operations`,
`direct_calls`, and `leaf_invocations` keyed by source span, from the command
binding lattice and the registry resolution. It does not consult
`FunctionUnit::types`, `sccp`, intervals, `var_escape`, `memory_ssa`,
`world_state_ssa`, or the dispatch-stability proof — none of which any
emitter consumes today (the sole exception is `exact_i64` in the sealed
native-add selector). The whole-function `GenericInvoke` plan and the
per-statement leaf plan emit the same argv/completion sequence twice
(`finish_semantic_invoke` vs `emit_invoke_node`), as `wasm-codegen.md`
already admits. Direct procs are limited to `Var`/`Literal`/`+`.

### 2.5 The bytecode backend already has what WASM lacks, in registry form

`codegen/{statements,values,expressions,cmd_subst,control_flow,emitter}` is
the TclVM bytecode emitter. It compiles expressions natively from `ExprNode`,
keeps proc locals in indexed slots, and specialises 32 command forms — 15
statement-position `CodegenHookId`s and 17 value-position
`InlineCodegenHookId`s, all declared in `tcl-registry/src/hooks.rs` and
dispatched by typed hook, never by name. The WASM tier shares none of it
below the `Emit` seam: the seam is a binary consume/decline per statement
with source text as its only vocabulary. The two backends therefore hold two
different answers to "what does `lappend x $y` mean", and only one of them is
a good answer.

### 2.6 Runtime findings that bound any compiled fast path

From `runtime/rust` (file:line in the survey notes in §10):

1. **No numeric write-back.** `bignum::read` (`bignum.rs:161`) parses the
   string rep of a non-numeric-typed object on every arithmetic use and never
   caches the result. `incr x` in a loop re-parses `x` each iteration unless
   the object already carries an int rep.
2. **No expression cache.** `parse_runtime_expr` (`builtins.rs:943`) re-lexes,
   re-parses, and re-validates the condition text of every `if`/`while` on
   every evaluation. There is no `TCL_EXPR_TYPE` internal rep.
3. **Compiled slots are names, not cells.** `Frame::compiled_slots` is a
   `Vec<Vec<u8>>` slot→name table; `tcl_codegen_local_get/set` do a level
   scan (`rposition`), a `Vec<u8>` clone, a `BTreeMap` lookup, and a link
   walk per access.
4. **No per-cell trace bit.** Variable traces are one flat `Vec<VarTrace>` on
   the interpreter; the only cheap question is "any trace anywhere".
5. **No command cache in dispatch.** `dispatch_inner` copies the command
   name into a fresh `Vec<u8>` per call and walks the namespace path;
   `CmdArena` (dense `u32` ids) exists but dispatch does not use it. The
   `CommandEnvironment` guard epoch is exactly the validation a direct-call
   handle needs.
6. **`run_proc` cost.** Renders every argument to bytes for `info level`
   unconditionally, binds parameters by name, and evaluates the body from
   source.
7. **TclOO rebuilds the call chain on every method call** and
   `GuardDomain::ObjectDispatch` is poisoned at interpreter creation, so no
   TclOO fast path can currently be guarded at all.
8. **`execute_intrinsic` has one arm** (`StringLength`), while
   `IntrinsicId` declares about twenty list/dict/string operations.
9. **Coroutines** are refused in the wasm build; `after`/`vwait` and `clock`
   are host gaps (see `wasm-target-surfaces.md`).

### 2.7 Front-end facts already available and unused by codegen

`FunctionUnit` carries, per function: the CFG and SSA, `def_use`, `sccp`
(constants and reachability), `types: HashMap<ValueKey, TypeLattice>` with
`TypeShape::{Int,Bignum,Double,Boolean,String,List,Dict,Object,…}`,
intervals via `compute_intervals`, taints, rendered properties, optional
memory SSA, and `dynamic_names`. The unit carries interprocedural summaries,
`var_escape`, `object_types` (a class-shape lattice for TclOO), the command
binding lattice, `scan_module_command_mutations`, the executable semantic IR
with world-state SSA, and the dispatch-stability contents lattice. The
registry supplies per-form `SemanticOperationId`, `IntrinsicId`,
`DispatchDependencies`, state transitions, effect footprints, result
stability, argument roles, and `TclType` return types. This is more than
enough to drive the proofs in §3; the missing piece is a lowering that
*consumes* them.

## 3. Target architecture: lower first, then prove framing away

### 3.1 Pipeline

```text
CST + structured words (unchanged)
  -> common semantic IR / executable CFG (executable_ir.rs, made total: §3.2)
  -> value SSA, cell SSA, world SSA, SCCP, types, intervals, escape, dispatch proofs (unchanged, now consumed)
  -> NATIVE LOWERING (new, target-neutral)        docs: this section
       every statement -> native ops + explicit framing ops
  -> FRAMING ELISION (new optimiser passes over lowered IR)
       each framing op removed, guarded, or kept with a typed reason
  -> backend plan (BackendRegistry, unchanged contract)
  -> WASM emitter (backend.rs, rewritten to consume lowered IR, not source spans)
  -> shared Rust runtime ABI v2 (§4)
```

The lowering and the elision passes are target-neutral and live in
`tcl-compiler` above `codegen/`. TclVM can adopt the same lowered IR later
(its existing hooks are the seed of the registry descriptors in §3.3); the
LSP gains nothing directly but its optimisation catalogue (O-codes) can report
"this statement needs a Tcl frame because of X" from the same typed reasons.

### 3.2 Make the executable IR total

`executable_ir.rs` today makes word evaluation, argv assembly, invocation,
and completion explicit only for plain calls; `If`, `For`, `While`, `Foreach`,
`Catch`, `Try`, `Switch`, `Block`, and `UpFrame` are `ExecuteOpaqueRegion`
barriers, and already-lowered `set`/`incr`/`expr`/`return` are
`ExecuteLowered` with conservative effects. Step one is to project all of
these into real executable edges: loop headers with back edges and
break/continue targets, `catch`/`try` handler edges keyed by completion class,
`switch` as a decision tree over `tcl_syntax::case_list`, and `foreach` as a
list-cursor loop with per-iteration cell writes. This is prerequisite work
that the common-semantic-compiler contract already schedules (its stage 4);
without it every loop body is an all-world clobber and no framing inside a
loop can ever be elided.

### 3.3 Native Lowered IR (NLIR)

A statement lowers to a small vocabulary of typed operations over SSA values
with an explicit representation:

| Op family | Examples | Framing it carries |
|---|---|---|
| value construction | `ConstInt`, `ConstDouble`, `ConstStr`, `Box`, `Unbox{Int,Double,Bool}` | `Unbox` is a conversion that can fail: it has an error edge |
| native arithmetic | `IAdd/ISub/IMul` (checked, overflow edge to bignum), `IDiv/IMod` (Tcl rounding), `FAdd…`, `ICmp/FCmp/StrCmp`, `Shl/Shr/And/Or/Xor/Not` | none |
| cell access | `CellRead(place)`, `CellWrite(place, v)`, `CellIncr`, `CellAppend`, `CellUnset`, `ElemRead/ElemWrite(place, key)` | **cell framing**: the named Tcl cell; a `TraceBarrier` before and after |
| intrinsic | `Intrinsic(id, [values]) -> value` | none beyond boxing its operands |
| invocation | `Invoke(argv) -> completion`, `DirectCall(proc, [values]) -> value`, `MethodCall(obj, method, [values])` | **dispatch framing**: binding/namespace/trace domains |
| activation | `FrameEnter(plan)`, `FrameExit`, `BindFormal(slot, name, v)`, `LinkCell(local, target)` | **frame framing** |
| completion | `Complete(code, result, options)`, `Dispatch(completion)`, `Unwind` | **completion framing** |
| observation | `TraceBarrier(cell, kind)`, `Guard(domains, identity)`, `Materialise(slot -> cell)`, `Reload(cell -> slot)`, `Safepoint` | the explicit hooks the elision passes remove |

The lowering is driven by registry data. Each `CommandSpec` form gains a
`NativeLowering` descriptor (a sibling of the existing `codegen_hook` and
`inline_codegen_hook` fields) with one of a few shapes:

- `Intrinsic(IntrinsicId, ArityRule)` — pure or read-only value operation;
  arguments are values, result is a value (`llength`, `lindex`, `string
  length`, `dict get`, `format`, `split`, `join`, `file join`, …).
- `CellReadModifyWrite(IntrinsicId)` — `incr`, `append`, `lappend`, `lset`,
  `dict set/incr/lappend`, `array set`: the var-write argument is a place,
  the rest are values, and the runtime intrinsic operates on the cell's
  object in place with copy-on-write.
- `Structured(LoweringHookId)` — `if`/`while`/`for`/`foreach`/`switch`/
  `catch`/`try`/`return`/`break`/`continue`: already registry data, now
  projected by §3.2.
- `Scope(ScopeKind)` — `global`, `variable`, `upvar`, `namespace upvar`:
  emit `LinkCell` and mark the named cells frame-observable.
- `Definition` — `proc`, `apply` literal, `oo::class create` bodies: emit
  `DefineProc(native fn, source)` so the runtime binds both.
- `Generic` — everything else: `Invoke(argv)` exactly as the leaf-invoke
  path does today.

The bytecode backend's 32 hooks map onto these shapes one for one; migrating
its emitter onto the shared descriptors is a later step, but the descriptors
are designed so it can.

### 3.4 Representation and framing lattices

Two lattices sit on the lowered IR and are computed once, target-neutrally.

**Value representation** per SSA value: `NativeInt(i64, interval)`,
`NativeDouble`, `NativeBool`, `Boxed(TypeShape)`, `Unknown`. It is seeded
from `types` and intervals, refined by the checked-arithmetic edges, and
raised to `Boxed` at every operand of an `Intrinsic`, `Invoke`, `CellWrite`
to a frame-resident cell, or `Complete`. Boxing is a `Box` op inserted at
the last possible point; unboxing is a `Unbox` op inserted at the first use
after a boxed read, with its failure edge routed to the ordinary Tcl error
path (never a trap). The Tcl 8 supplementary-character rule in
`semantic-aot-optimisation.md` stays: string-indexed intrinsics decline to
`Invoke` on Tcl 8 dialects.

**Cell storage** per place: `Register` (no cell, no name), `Slot` (indexed
runtime slot, name bound lazily), `Cell` (named cell, traces fire), `Linked`
(`upvar`/`global` target). Seeded `Cell`; lowered by the elision passes below
when the observers are absent.

**Frame plan** per function (the table already in
`semantic-aot-optimisation.md`): `Full`, `CellsAndMetadata`, `MetadataOnly`,
`Materialisable`, `Absent`.

### 3.5 Framing elision: which proof removes which framing

| Framing op | Removed when | Guarded when | Kept when |
|---|---|---|---|
| `TraceBarrier(cell)` | dispatch-proof `variable_traces` ledger proves no registration reaches the cell **and** the module has no dynamic `trace add` target for it | the ledger is unknown but the runtime per-cell trace bit (§4) can be tested: emit `CellTraced?` branch to the traced path | the cell is a `Linked` target of an unknown frame |
| `Cell -> Slot` | `var_escape` says `Local`, no `Linked` alias, no `uplevel`/`info`/`eval` in the activation, no trace barrier remains | never (a slot is already the cheap runtime form) | frame plan is `Full` |
| `Slot -> Register` | the value never crosses a `Materialise` point: no `Invoke`, no `Safepoint`, no callback-capable intrinsic between def and last use | `Materialise` at the boundary, `Reload` after | the value is live across an unknown call |
| `FrameEnter/Exit` | frame plan `Absent`: every cell is `Register`/`Slot`, no `info level`/`info frame`/error-stack observer, no `uplevel`/`upvar` into or out of it, no coroutine, no execution trace on the proc | `MetadataOnly`: push an activation record (§4.4) but no cell table | `Full` |
| `Invoke -> Intrinsic` | registry form resolved **and** dispatch-proof proves `CommandBinding`, `NamespaceLookup`, `CommandTraces` untouched at the site | `Guard(domains)` token before the intrinsic, `Invoke` on failure (the existing guarded-intrinsic mechanism, made general) | the head is dynamic |
| `Invoke -> DirectCall` | as above for the proc's binding, plus the formal-parameter binder proves the exact form (required/default/`args`) | guard on `CommandEnvironment` epoch | rename/trace observed in the module |
| `Dispatch(completion)` | completion lattice proves `{Ok}` for the callee (intrinsics with declared completion sets; direct procs whose body proves it) | — | an `Invoke` |
| checked `IAdd` overflow edge | interval proof shows the result fits i64 | — | otherwise (edge to bignum intrinsic) |

Every elision decision is recorded per op with the typed reason, exactly as
`MixedRegionPlan` records intrinsic candidates today, so Explorer can show
"framing kept: cell `x` is an `upvar` target at line 12".

### 3.6 Push optimisation early

Constant folding, SCCP, branch folding, GVN, LICM, inlining of small procs,
`foreach` element extraction, and tail-call rewriting already run on the
common IR before any backend. Two things move earlier so both backends and
the LSP benefit:

- **Proc-call specialisation** (arity check, default filling, `args`
  packing) becomes a common transform on the executable IR once a proc's
  binding is proved, so the callee's formal-parameter list is bound at
  compile time and both backends emit a direct call.
- **Expression lowering** from `ExprNode` to NLIR arithmetic with
  representation facts happens in the common layer; the bytecode backend's
  `emit_expr` and the WASM backend both consume the same lowered tree, and
  the `guaranteed_numeric` bit the bytecode backend computes locally becomes
  the shared representation lattice.

### 3.7 What stays out of the emitter

The emitter consumes NLIR and a plan; it never sees a command name, a source
span for evaluation (only for `errorInfo` line attribution), or a
compatibility string. `structured.rs` survives as the driver for control
flow but its `emit_command(source_text)` becomes the last rung reached only
by an `ExecuteOpaqueRegion` that §3.2 has not yet projected — and the goal is
that this rung emits nothing for the sample tiers T0–T6.

## 4. Runtime ABI v2 (one runtime, new transport)

All additions are in `runtime/rust`, exported through `tcl-runtime-api`'s
`CodegenAbiImportId` so compiler and runtime share one descriptor table.

### 4.1 Typed values

- `tcl_value_new_wide_int` (exists), `tcl_value_new_double`, `tcl_value_new_bool`.
- `tcl_value_get_wide_int(obj, out *i64) -> status`, `tcl_value_get_double`,
  `tcl_value_get_bool`: conversions that **write the parsed rep back onto the
  object** (closing finding 2.6.1) and return a Tcl error status on failure.
- Small-int cache for `-1..=255` and interned literal pool for the module's
  constant strings (allocated once at module load via
  `tcl_codegen_literal_table(ptr, count)`), so a hot loop never allocates for
  a constant.

### 4.2 Intrinsic table

One entry point per registry `IntrinsicId` in the pure/value family,
generated from the registry by `cargo xtask` so the runtime cannot drift
from the compiler: `tcl_intrinsic_<snake_id>(argv, argc, out)` for the
general form plus typed fast forms where the operand representation is
native (`tcl_intrinsic_list_index_i64(list, idx, out)`,
`tcl_intrinsic_string_length_i64(str) -> i64`). The existing
`tcl_intrinsic_invoke_argv` and `execute_intrinsic` remain the guarded
generic path; `execute_intrinsic` grows to cover every `IntrinsicId` and the
`command-backing` drift gate learns a third classification, "native
intrinsic backed".

### 4.3 Cells and slots

- `Frame.slots: Vec<Var>` with a `name -> slot` side table, so a compiled
  local is an O(1) index while `info vars`, `upvar`, and traces still find it
  by name (the invariant `compiled_slots_and_named_access_share_one_cell`
  already pins).
- `tcl_codegen_slot_get(slot) -> obj`, `slot_set`, `slot_incr_i64`,
  `slot_append`, `slot_lappend`: read-modify-write on the slot's object with
  copy-on-write.
- A `traced: bool` bit on `Var` (set by `trace add variable`, cleared on
  removal) and `tcl_codegen_slot_traced(slot) -> i32`, the runtime half of the
  guarded `TraceBarrier`. Namespace and global cells get the same bit.

### 4.4 Activations

- `tcl_codegen_activation_enter(proc_handle, argv, argc, plan) -> frame_id`
  and `activation_leave(frame_id, code)`: pushes a real `Frame`/`CmdFrame`
  according to the frame plan (`MetadataOnly` pushes level, namespace, and
  the `info level` words lazily from a callback; `Full` binds formals by
  name). It increments the eval depth so the outermost-eval rule and
  `errorInfo` accumulation see compiled code as an activation (closing 2.2's
  second defect).
- `tcl_codegen_proc_define_native(name, params, body_source, table_index)`
  registers a proc whose runtime dispatch calls the compiled function through
  the module's exported function table (`call_indirect`), falling back to the
  source body when the compiled entry declines (wrong arity is the runtime's
  `wrong # args`, not a trap). Emitted proc functions stop being dead code.
- `tcl_codegen_command_handle(name) -> u32` + `tcl_codegen_handle_valid(handle,
  epoch) -> i32` over `CmdArena`, validated by the `CommandEnvironment` epoch,
  for guarded direct dispatch without name resolution.

### 4.5 Expressions

An expression internal rep (`TCL_EXPR_TYPE`) caching the parsed and validated
`ExprNode` on the condition object, so the remaining `tcl_expr_bool` calls
(dynamic or unbraced expressions) parse once. Compiled conditions do not use
it: they are native.

### 4.6 TclOO (see §5)

Un-poison `ObjectDispatch` by giving TclOO one mutation owner that bumps the
domain epoch; add a call-chain cache keyed by `(class generation, object
customisation flag, method, external)`; expose
`tcl_oo_method_handle(class, method)`, `tcl_oo_dispatch(obj, handle, argv,
argc, out)`, and `tcl_oo_frame_enter/leave` so a compiled method body runs
inside a real `OoFrame` with the chain index `self`/`next` need.

## 5. Light object framing for TclOO

Real TclOO use in the corpus (tcllib's `oo::` modules, ticklecharts, the
sample tier T6) is dominated by: classes with `variable` declarations, a
constructor, a handful of methods, `my` calls, `self`, one level of
`superclass` with `next`, and objects held in lists or dicts and dispatched
as `$obj method`. Mixins, filters, `oo::objdefine`, `forward`, and dynamic
`oo::define` are present but rare and almost always at class-definition time.

The object-types lattice (`object_types.rs`) already tracks, per SSA value,
"instance of class C, created by `C new`". Combined with the definition-body
grammar the analyser uses, the compiler knows every method body, its formal
parameters, its declared instance variables, and the class's static
superclass chain.

Light framing compiles a method to a native function
`fn C::m(self_handle: i32, args…) -> completion` with:

- instance variables as slots resolved once per activation from the object's
  variable namespace (`tcl_oo_frame_enter` returns the object's `var_ns` and
  binds the declared `variable`s into the frame's slot table exactly as
  `run_proc` does today via `link_vars`, but by slot index);
- `my m2 …` and `next …` as direct calls to the compiled method function when
  the class-shape proof says the chain is static (no mixin/filter/objdefine
  on the class or object, no `oo::define` after creation in the module, no
  `unknown` method handler), guarded by the un-poisoned `ObjectDispatch`
  epoch; otherwise `tcl_oo_dispatch` through the cached chain;
- `self`, `self class`, `self target` served from the real `OoFrame` the
  activation pushed, so no compiled path breaks introspection;
- `$obj method args` at a call site whose object type is a known class and
  whose method resolves statically becomes a guarded direct call; unknown
  receivers stay `Invoke`.

The runtime keeps its full engine for everything else (filters, `oo::copy`,
destructors, `info object`, TIP 500 private variables). The light frame is a
faster *transport* onto the same engine state, which is what keeps one
runtime. The `t6-tcloo` samples are the acceptance set: 60 and 61 should
compile with native method bodies and direct `my`/`next`; 62 must fall back to
chain dispatch for the mixed-in/filtered object and 63 must still resolve
`$p dist2 $o` directly because the receiver type is proved.

## 6. Optimiser changes

Concrete passes, in order after native lowering:

1. **Representation inference** (§3.4) with interval-backed overflow proofs;
   emits `Box`/`Unbox` at boundaries.
2. **Trace-barrier elision** using the dispatch-proof variable-trace ledger
   plus a new flow-insensitive "trace target set" per module (exact names or
   wildcard).
3. **Cell demotion** (`Cell -> Slot -> Register`) driven by var-escape, the
   frame plan, and materialisation points.
4. **Frame-plan selection** per function from the surviving cells and
   observers.
5. **Invoke refinement** (`Invoke -> Intrinsic | DirectCall | MethodCall`)
   with guard insertion from `DispatchDependencies` and the contents lattice.
6. **Completion narrowing**: drop `Dispatch` after ops whose completion set
   is `{Ok}`; collapse the per-statement `block` cleanup path to a per-region
   one when no owned value is live across the abrupt edge.
7. **Materialise/Reload sinking**: move boxing to the edge that needs it,
   hoist unboxing out of loops when the cell is a `Slot`/`Register`.
8. **Inline direct procs** whose body is small and whose frame plan is
   `Absent` (tail-call and single-use-inline facts already exist).
9. **Peephole on the WASM IR**: local reuse, `i32.const`+`i32.add` folding,
   dead `block` removal, duplicated `local.get frame` elimination — the
   emitter currently has no peephole at all.

Each pass is a `SemanticOptimisationPassId` (new ids, independently
disableable, off until its differential matrix is green, then flipped on by
default tier by tier as §7 lands).

## 7. Phased plan

Each phase names its acceptance set from `samples/wasm/` and the gates it
must keep green: `wasm_real_link.rs` with `TCL_REQUIRE_WASM_LINK=1`, the
linked-WASM fuzz arm, `command-backing`, and byte-identical output against
`tclsh9.0` for every sample in the tiers claimed.

| Phase | Deliverable | Acceptance | Issues rolled in |
|---|---|---|---|
| **P0 harness** | `wasm_tiers.rs` test that links every `samples/wasm` script and diffs against `expected/`; per-tier framing budget assertions (counts of `tcl_eval_code`/`tcl_expr_bool`/`tcl_invoke_argv` per sample recorded as goldens); run `runtime/rust` unit suite in CI | all 36 samples run; budgets recorded | #1768, #1589 (register the full builtin set in `run_script`), #1716 (wasm32-capable clang on macOS) |
| **P1 runtime ABI v2 groundwork** | typed value get/set with write-back, small-int cache, literal table, `Frame.slots`, per-cell trace bit, activation enter/leave with eval-depth accounting, native proc table + `call_indirect` binding, `CmdArena`-backed handles | 2.2's `catch` defect fixed; `70_var_traces` shape passes once #1633 rows land | #1633 (incr/lappend read traces, errorInfo frame, re-entrancy rows), #1574, #1575, #1569, #1577 (array-element names in `lassign`/`catch`/`foreach` writes), #1425 (boolean words via `tcl_syntax::boolean`), #1432 (`rand` owner), #1428, #1382, #1581 |
| **P2 executable IR total** | `If`/`While`/`For`/`Foreach`/`Switch`/`Catch`/`Try`/`Return` as executable edges with completion classes | Explorer shows no `opaque` region for T1/T5 samples | #1648 (fold gating across proc/eval unit boundaries), #1603 (per-command parse model: an opaque tail must not abort earlier statements) |
| **P3 native lowering T0–T1** | NLIR, `NativeLowering` descriptors for `set`/`incr`/`append`/`expr`/conditions; representation lattice; native `i64`/`f64` arithmetic with checked edges; native conditions; `puts` boundary | T0 and T1 samples emit zero `tcl_eval_code`/`tcl_expr_bool`; `02_arith_chain` is straight-line `i64` ops | the `puts` compat-text defect (2.2), `whole_var_reference` retired from codegen |
| **P4 values T2** | intrinsic table generated from the registry; list/string/dict/regexp/format/split/join intrinsics; `foreach`/`lmap` as cursor loops; `CellReadModifyWrite` for `lappend`/`dict set`/`lset` | T2 samples emit no `tcl_eval_code`; `command-backing` reports "native intrinsic" coverage | #1607 (option tables for the intrinsics' subcommand parsing go through `OptionTable`), #1576, #1586, #1646 |
| **P5 procs T3** | compiled proc bodies executed via the function table; formal binder proof; direct calls; frame plans `Absent`/`MetadataOnly`; recursion; `apply` literal lambdas | `31_recursion` runs `fib 20` with no Tcl frame; `32_defaults_and_args` binds defaults/`args` natively and reports the exact `wrong # args` text | #1765 (tailcall tick protocol, shared with VM) |
| **P6 scopes T4** | `LinkCell` for `global`/`variable`/`upvar`; arrays as element cells with slot-backed keys; namespace cells; ensembles as static dispatch when sealed | T4 samples; `41_upvar` keeps a `Full` frame only in `incrby`/`swap` | #1751, #1752, #1753, #1755 (namespace token lifecycle, needed so a compiled namespace cell survives deferred deletion), #1412 (rename semantics the direct-call guard relies on) |
| **P7 completion T5** | `catch`/`try`/`return -code`/`-errorcode`/`-errorstack` through the completion spine; `Dispatch` narrowing | T5 samples byte-identical including `-errorcode` and `$::errorCode` | #1750 (`-errorstack` parity with the VM) |
| **P8 TclOO T6** | §5 light object framing; `ObjectDispatch` un-poisoned; chain cache | T6 samples; 60/61 with direct `my`/`next`; 62/63 via cached chain | #1764 (stable class/object identity), #1763 (trace sidecars by token), #1594, #1703, #1704 |
| **P9 dynamic T7** | trace-guarded cells; `uplevel`/`eval` with `Full` frames only in the affected activations; `info` observers; coroutine support in the wasm build (asyncify-based stack switching or a CPS lowering of `yield` — decided in P9's own design note) | T7 samples; everything outside the traced/introspected region stays native | #1598 (byte-valued output encoding), #1732 (array-index brace rule per dialect) |
| **P10 bytecode convergence** | TclVM emitter consumes `NativeLowering` descriptors and the shared expression lowering | byte-identity gate still green; hook tables retired in favour of descriptors | — |

P0–P1 are independent of each other and can run as parallel lanes. P2 gates
P3 onwards. P4–P7 are largely independent once P3 has landed. P8 needs P5
and P6.

## 8. Open-issue roll-in: PR buckets and owners

Every open issue was read in full for this section. Issues are grouped into
PR-sized buckets that share an owner module and a test surface, so one lane
can land each as a single reviewable PR. `wasm-runtime`/`runtime` issues are
`runtime/rust`; `tclvm`-only issues are the bytecode VM and are out of this
programme unless a shared owner is named.

| Bucket | Issues | Owner module | Phase | Agent |
|---|---|---|---|---|
| **R1 activation and completion** | #1773 (eval-depth `-errorcode` loss), #1750 counterpart check that `-errorstack` reaches the ABI options dict | `codegen_abi.rs`, `interp.rs` eval loop | P1 (in flight) | Opus |
| **R2 typed values and boolean owner** | #1425 (boolean words via `tcl_syntax::boolean`), numeric write-back, `tcl_value_*` getters | `obj.rs`, `bignum.rs`, `expr.rs`, `value_ops.rs`, `cmd_namespace.rs` | P1 (in flight) | Opus |
| **R3 numeric tower parity** | #1428 (`0 ** -1`, exponent ceiling via `number_tower`), #1382 (`entier`/`int`/`wide` bignum path for float operands in the shared `mathfunc`), #1432 (one `rand`/`srand` owner next to `mathfunc::dispatch`), #1581 (errorCode taxonomy: `ARITH IOVERFLOW`, `TCL VALUE DOUBLE NAN`, boolean-context codes, 8.6 `IllegalExprOperandType` wording — needs an error channel on `mathfunc::dispatch`, so it lands last) | `tcl-syntax` `number_tower`/`mathfunc`, `runtime/rust` `cmd_mathfunc.rs`/`bignum.rs`, `tcl-vm` `cmd_math.rs`/`expr.rs` for the shared halves | after P1 | Opus (shared-owner change touches both engines) |
| **R4 word-parser gaps** | #1576 (unterminated `{` must raise `missing close-brace`), #1586 (unterminated `${` in script words: consume `braced_var_name_end`), #1577 (`lassign`/`catch`/`regexp`/`scan`/`binary scan`/`foreach` must resolve `arr(k)` element targets) | `runtime/rust/src/parse.rs`, the var-write sites in `cmd_list.rs`/`cmd_error.rs`/`cmd_regex.rs`/`cmd_scan.rs`/`cmd_binary.rs`/`cmd_control.rs` | after P1 | Sonnet (oracle-driven, mechanical) |
| **R5 trace semantics** | #1633 runtime rows (`incr` read trace, write-trace `errorInfo` frame, `trace info` during firing, command-delete trace recreating the command, unset trace reviving the variable), #1574 (per-cell re-entrancy unit), #1575 (proc-frame teardown unset traces, per-element firing on whole-array unset), #1569 (`array` traces) | `cmd_trace.rs`, `interp.rs` firing sites, `frame.rs` | after P1 (same files as the trace bit) | Opus |
| **R6 command-table and namespace lifecycle** | #1412 (rename onto occupied destination, proc re-homing, `interp` subcommand list, `hide`/`expose`, `invokehidden` flags), #1751 (retain deleted namespace tables while active), #1752 (Tcl hash order during teardown), #1763 (command-trace sidecars by token), #1764 (TclOO identity across deferred deletion) | `namespace.rs`, `cmd_namespace.rs`, `cmd_alias.rs`, `cmd_trace.rs`, `cmd_oo.rs` | P6/P8 prerequisite; independent of P1–P3 | Opus, two PRs: #1412 alone, then the four lifecycle issues together |
| **R7 option tables** | #1607 (~78 hand-spelled `bad option` sites, per ensemble family) | every `cmd_*.rs`, VM siblings | any time after P1; also the intrinsic table's subcommand parsing (P4) | Sonnet, one PR per family |
| **R8 output encoding** | #1598 (byte-valued characters rendered through UTF-8 on `puts`/error text) | `cmd_chan.rs` write path, VM channel layer | P9 | Sonnet |
| **R9 infrastructure** | #1768 (runtime unit suite in CI), #1589 (`run_script` builtins), #1716 (macOS wasm32-capable clang probe), #1570 (clippy in `cmd_fs.rs`) | `ci.yml`, `Makefile`, `scripts/dev/ensure-test-deps.sh`, examples | P0 (in flight, #1716 and #1570 absorbed) | Opus |
| **C1 codegen defects** | #1772 (`puts` fast path reparses compatibility text), #1774 (emitted proc functions never executed) | `codegen/wasm/backend.rs`, ABI proc table | P3 / P5 | Fable |
| **C2 TclOO callback prefixes** | #1703, #1704 (method callback prefixes in `CommandPrefix` slots; one-hop stored prefixes) | `analyser/`, LSP navigation | feeds P8's "does a stored prefix force a `Full` frame" proof; not on the WASM critical path | Opus, separate PR |

Verified against the linked runtime while bucketing: #1732's 9.x array-index
rule is already correct in `runtime/rust` (merged via #1741; only the VM half
was reopened), while #1576, #1428, #1382, and #1577 all still reproduce
exactly as filed.

**tclvm-only, out of this programme**: #1755, #1753, #1750 (VM half), #1594,
#1646, #1603, #1648 (fix in flight as #1754), #1765, #1732 (VM half). Where
one of these names a shared owner (`number_tower`, `mathfunc`, the namespace
token model), the bucket above that touches the shared crate keeps the VM
green but does not take on the VM-only behaviour.

**Not wasm work**: #1734, #1714, #1712, #1710, #1708, #1693, #1685, #1684,
#1678, #1655, #1650, #1643, #1631, #1599, #1543, #1524, #1473, #1372.

Closing rule for every bucket: the PR closes its issues with a short root
cause and fix statement per issue.

## 9. Corpus evidence

Two corpora were read for this plan.

**The 21-repository AOT corpus** named in
[`aot-command-priority.md`](aot-command-priority.md) was re-fetched into
`experiments/aot-corpus/` and the census re-run with
`examples/aot_command_priority.rs`. The ranked forms match the committed CSV
within a fraction of a percent (`set` 101,272 vs 101,221; `foreach` 11,307
vs 11,302); the only large differences (`if`, and the `<`/`>`/`emit` rows)
come from the fresh run not applying the committed census's `filetypes.tcl`
exclusion (its §2.3), a generated data table. The top 25 forms cover about 81% of literal-head call sites and the
top 60 cover 92%. Every form in the top 25 is in T0–T5 of the sample tiers.

**tcllib 2.0, the Tcl 9.0.4 script library, and `samples/`** were surveyed
for *shape*, not just frequency (759 cleaned tcllib files, 294k lines;
generated data tables excluded). The facts that shaped §3–§7:

| Fact | Number | Consequence |
|---|---|---|
| `set` sites whose value is `[cmd …]` | 48% | the `Invoke -> Intrinsic` refinement and completion narrowing pay on half of all assignments |
| `expr` braced | 95–98% (tcllib 97.9%) | native expression lowering covers nearly everything; unbraced `expr` stays a slow path |
| `if` conditions braced | 98–99.8% | same |
| `expr` bodies with only `$var`/literal/operator | 45% (27% are exactly `$a op $b`) | representation inference on scalars is the main win, not intrinsic-in-expr |
| `if` conditions containing a `[cmd]` | 55% (`info exists`, `string …`, `llength`, `dict exists`, `catch`) | conditions must lower through the same intrinsic path as statements; a condition is not a special case |
| procs with no `expr` at all | 79% (tcllib) | "native" for most procs means slots, intrinsics, and direct calls, not arithmetic |
| straight-line procs (no control flow) | 41% (tcllib), 60% (samples) | the P5 `Absent` frame plan applies to a large fraction of real procs |
| procs using `upvar` | **20%** of tcllib procs; half alias a caller-supplied name | `LinkCell` and the `Full`-frame plan are first-class, not exotic; `upvar 1 $name` needs a runtime link, never a compile-time alias |
| procs using `variable` | 26% (tcllib), 46% (tcl9 lib) | namespace cells with slots are as important as proc locals |
| computed variable names (`set $n`) | ~2% of `set` sites | the `DynamicVariableName` decline is acceptable as a per-statement fallback |
| `uplevel`/`eval $x`/`subst` | under 0.7% of sites | dynamic-script framing can be per-activation |
| loops | `foreach` 78%, `for` 21%, `while` mostly `while 1` | `foreach` as a cursor loop (P4) matters more than `for`/`while` |
| `foreach {k v} …` destructuring | 32% of `foreach` | multi-variable binding in the cursor loop from day one |
| error handling | `catch`/`return -code error`/`error` outnumber `try`/`throw` about 10:1 | P7 prioritises `catch` and `return -code`; `try` follows |
| TclOO | 69 classes vs 7,800 procs in tcllib; 550 methods | correctness on the common shape matters more than breadth |
| TclOO features in class bodies | `superclass`/`variable`/`constructor`/`method`/`my`/`next` cover ~99%; `mixin` 6 uses, `filter` 0, `export` 2 | §5's light frame targets exactly that shape; mixin/filter/objdefine fall back to the cached chain |
| `oo::objdefine` per-instance tweaks | 59 in tcllib | the per-object "customised" flag in §5 is needed, not optional |
| object-as-namespace idiom (`variable ${self}::field`) | ~600 sites | namespace-variable cells with computed namespace names stay `Cell`, with a fast path when the namespace prefix is a proved value |

By site count the 25 commands `set if return expr variable string list lappend
foreach lindex dict info file llength array upvar incr append puts catch switch
error lrange while for` cover 65.5% of all 197k command sites in the shaped
corpus (71% once definition-time `proc`/`package`/`namespace`/`method` sites
are excluded); adding `unset lassign regexp format join split binary uplevel`
reaches about 74%. The tail is user-defined procs, which is exactly what P5
makes cheap to call.

## 10. Survey notes

The file-and-line survey notes gathered for this review (general-tier
codegen, runtime internals, corpus shapes) are summarised in §2 and §9; the
helper scripts for the shape survey live outside the repository. When the
first phase opens, its lane tracking document under `docs/design/lanes/`
carries the site inventory.

## Related

- [wasm-codegen.md](wasm-codegen.md) — the current pipeline this plan replaces
  stage by stage.
- [semantic-aot-optimisation.md](semantic-aot-optimisation.md) — the proof
  contract every elision in §3.5 must satisfy.
- [common-semantic-compiler.md](common-semantic-compiler.md) — the
  target-neutral IR this plan extends.
- [dispatch-stability-proof.md](dispatch-stability-proof.md) — the contents
  lattice behind trace and dispatch elision.
- [aot-command-priority.md](aot-command-priority.md) — the corpus census.
- [wasm-target-surfaces.md](wasm-target-surfaces.md) — WASI vs browser host
  limits that bound T7.
- [`samples/wasm/README.md`](../../../samples/wasm/README.md) — the sample tiers.

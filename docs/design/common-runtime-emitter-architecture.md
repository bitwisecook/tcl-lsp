# Common runtime & emitter interfaces — cross-backend architecture

> **Status: steering document (design, not yet implemented).** This is the
> source-of-truth for the *shape* of the shared interfaces that let the new
> native bytecode VM ("TCLVM"), the existing WASM backend, and future backends
> share as much code as is actually shareable. It reasons across the **whole
> space** — the bytecode (TCLVM) emitter, the WASM emitter, and the runtimes —
> to fix the right split before code is written.
>
> **It deliberately does NOT touch the WASM emitter or the Rust WASM runtime.**
> Those have had the most work and encode hard-won solutions; they stay the
> behavioural **oracle**. The WASM side migrates onto these interfaces *later*,
> guided by [§7 WASM migration steering](#7-wasm-migration-steering). The
> near-term implementation target is the bytecode VM + the emitter trait on the
> Rust side.

## 1. Why this document exists

The repo already has a shared compiler frontend (lexer → green/red CST → IR →
CFG → SSA → optimiser) feeding **two codegen backends** that live side by side
(`compiler/codegen/bytecode/` and `compiler/codegen/wasm/`;
[docs/design/compiler/codegen-module-map.md]) and **multiple runtimes** (the Rust
WASM runtime `runtime/rust/`, and the
Python reference VM `tooling/vm/`). We are adding a native, idiomatic Rust **bytecode
VM** that executes the TCLVM bytecode the Rust codegen already emits
(`rust/tcl-compiler/src/codegen/`).

The naive framing — "share an emitter base class / share the command
implementations" — is wrong, and the WASM work proves why. Two facts dominate:

1. **The emitter↔runtime contract is a *state-mutation protocol*, not a
   value-passing interface.** The runtime is a **reified mutable store** —
   namespace tree, frame stack, variable tables, trace registry, retained proc
   bodies — that outlives any single compilation. The genuinely hard Tcl
   features (`info`, `trace`, `upvar`/`uplevel`, namespaces, coroutines,
   ensembles, `rename`/alias, child interps, `errorInfo`) are all *runtime
   state* concerns, not codegen concerns (evidence:
   `runtime/rust/src/namespace.rs`, `frame.rs`, `cmd_trace.rs`,
   `cmd_proc.rs`; [docs/design/runtime/namespace-tree.md],
   [docs/design/runtime/proc-call-and-stack-traces.md],
   [docs/design/runtime/command-introspection.md],
   [docs/design/runtime/trace-implementation.md]).

2. **The two value models are irreconcilable as one concrete type.** The WASM
   runtime's `Tcl_Obj` is a 24-byte `#[repr(C)]`, i32-pointer, raw-`*mut`,
   manually-refcounted struct mandated by C-extension interop over one linear
   memory ([docs/design/runtime/c-extension-abi.md] §4.2). The VM wants an
   idiomatic `Rc`-based dual-rep enum under `unsafe_code = "forbid"`. You cannot
   have both be the *same* type.

The consequence threads through everything below: **the bytecode VM is a
runtime, not a string-stack machine.** It must adopt the same reified-state
architecture as the WASM runtime, or it cannot implement `info`/`trace`/`upvar`
faithfully. Convergence happens at the **interface and pure-logic** layers, not
the storage layer.

## 2. The whole-space map

Three layers, with their consumers:

```
                ┌─────────────────────────────────────────────────────────┐
   FRONTEND     │ lexer → green/red CST → IR → CFG → SSA → optimiser       │  shared, done
   (shared)     │ tcl-lexer / tcl-syntax / tcl-compiler{ir,cfg,ssa,...}    │  (Rust + Python)
                └───────────────┬─────────────────────────────────────────┘
                                │  CfgModule / IrModule  (one vocabulary)
              ┌─────────────────┴───────────────────┐
   CODEGEN    │  Interface family A: EMITTER         │  per-backend artifact
   (lower)    │  shared CFG driver + emission hooks  │
              │   ┌──────────────┐  ┌──────────────┐ │
              │   │ bytecode/TCLVM│  │ WASM          │ │  future: native, ...
              │   │ → FunctionAsm │  │ → WasmModule  │ │
              │   └───────┬──────┘  └──────┬────────┘ │
              └───────────┼────────────────┼──────────┘
                          │ FunctionAsm    │ WasmModule + import table
              ┌───────────┴────────────────┴──────────┐
   RUNTIME    │  Interface family B: RUNTIME STATE     │  state-mutation protocol
   (execute)  │  namespaces · frames/upvar · traces ·  │  + shared pure value logic
              │  command dispatch · catch/return · info│
              │   ┌──────────┐ ┌──────────┐             │
              │   │ TCLVM    │ │ Rust WASM│             │  Python VM = reference
              │   │ (new)    │ │ runtime  │             │
              │   └──────────┘ └──────────┘             │
              └───────────────────────────────────────┘
```

Key correction to the original mental model: **there are two interface
families, not one.** "Agnostic emitter" is family A. The hard stuff the WASM
work solved is family B. The bytecode VM lives in the *runtime* layer alongside
the WASM runtimes; the bytecode emitter lives in the *codegen* layer alongside
the WASM emitter. They meet at the **emitter↔runtime contract** (§5).

## 3. Interface family A — the emitter (codegen → artifact)

Grounded in a read of both backends (`rust/tcl-compiler/src/codegen/`,
`compiler/codegen/wasm/`). Both consume the *same* `CfgModule`/`IrModule` and
switch on the *same* IR statement / expression / terminator shapes; they diverge
only in the value model and instruction encoding.

### 3a. What is already shared / identical

- **The IR/CFG vocabulary.** `Statement{AssignConst,AssignExpr,AssignValue,
  Incr,ExprEval,Call,If,While,For,Foreach,Catch,Try,Switch,Return,...}`,
  `Terminator{Goto,Branch,Return}`, `Block`, `Function`, `CfgModule`
  (`rust/tcl-compiler/src/{ir.rs,cfg.rs}`). Both backends already match on these.
- **The command-lowering registry seam.** Both backends dispatch per-command
  codegen through the *same* `CommandRegistry`, with parallel per-backend hooks
  (`register_codegen` for bytecode, `register_wasm_emitter` for WASM), a shared
  hook signature `(emitter, args, defs, context) -> bool`, and a shared
  `EmitContext{STATEMENT,VALUE}` (`compiler/registry/models.py`,
  `rust/tcl-compiler/src/codegen/emitter/bytecoded.rs`,
  `compiler/codegen/wasm/_emitter/cmds/*`). A weaker form of the "common
  command-lowering interface" therefore **already exists** to generalise.

### 3b. What is structurally parallel but duplicated (lift candidates)

- **The CFG-walk driver.** Block linearisation, dead-branch constant folding,
  bottom-tested loop reordering, loop-body collection, loop-context building,
  terminator *dispatch decisions* (fallthrough vs jump), try/finally detection,
  and proc-def interleaving. In Rust this is `codegen/emitter/{ordering,
  terminator,loop_blocks,try_blocks,generate}.rs` and operates **only** on the
  CFG vocabulary — no `Op`. The WASM emitter re-implements the equivalent walk
  independently (and does *not* currently do the const-fold/reorder pass). This
  is the prime lift: a shared `cfg_walk` driver parameterised over an emission
  trait (§3d).

### 3c. What is irreducibly backend-specific

- **Value model & instruction encoding.** Abstract operand stack of `Tcl_Obj`
  (bytecode) vs typed i32/i64 with explicit box/unbox + runtime imports (WASM).
- **Control-flow form.** Address jumps + a layout/jump-shrink pass
  (`codegen/layout.rs`) vs structured WASM blocks/loops/`br_if` + LEB128.
- **Optimisation passes** (peephole shapes differ: address-based vs
  instruction-based).

### 3d. The family-A interface shape

A `Backend` trait generic over the artifact, fed by a shared CFG driver:

```rust
// rust/tcl-compiler/src/codegen/backend.rs  (NEW, additive — lives in tcl-compiler)
pub trait Backend {
    type FuncArtifact;     // FunctionAsm | WasmFunc | ...
    type ModuleArtifact;   // ModuleAsm   | WasmModule | ...

    fn lower_function(&mut self, cfg: &CfgFunction, params: &[&str],
        is_proc: bool, proc_defs: &[IrProcedure],
        registry: &CommandRegistry) -> Self::FuncArtifact;

    fn lower_module(&mut self, cfg: &CfgModule, ir: &IrModule,
        registry: &CommandRegistry) -> Self::ModuleArtifact;
}
```

- The bytecode backend implements it by delegating to today's
  `codegen_function`/`codegen_module` — **zero churn** to the working emitter.
- A future WASM backend sets `type FuncArtifact = WasmFunc` and walks the same
  `CfgFunction.blocks`/`Terminator`; the associated types absorb the value-model
  difference.
- The shared CFG-walk driver (the §3b lift) is factored out *only once the
  second consumer exists* — do not speculatively abstract it; the bytecode
  emitter keeps its working driver until WASM is ready to share it.
- **Artifact types** (`FunctionAsm`, `Op`, `Instruction`, layout, format) move
  into a lean `tcl-bytecode` crate so the VM can depend on them without pulling
  the whole compiler; the *emitter* stays in `tcl-compiler`. See §6.

## 4. Interface family B — the runtime (the hard part)

This is what the WASM work actually invested in, and where the original plan was
naïve. The runtime is a **store + dispatch system**; the interface is a set of
**traits over mutable interp state**, plus genuinely **pure value logic** that
can be shared as concrete code. Evidence is cited inline.

### 4a. Pure value→value logic — share as concrete code

No interp state; identical across runtimes; already converging into `tcl-syntax`:
list split/merge & quoting, the expr Pratt parser + numeric tower, canonical
number/double formatting, format/scan grammars, glob match, backslash/subst,
string ops. The VM implements `tcl-syntax`'s `ExprOps`-style traits over its
`Value`; the runtimes do likewise over their obj. **This is the main lever for
"write the command once".** (See §6 `tcl-cmd-core`.)

### 4b. Interp-state operations — share an interface (trait), not the storage

Each is mutable per-interpreter state with backend-specific storage (the WASM
runtime: open-addressed hash tables in linear memory with tagged handles; Rust VM:
`HashMap`/arena). The *semantics* are shared; the *layout* is not.

**This is not speculative — the shape already exists in `runtime/rust`.** That
crate already implements the whole model idiomatically, just over `*mut TclObj`:
`FrameStack::{push(ns)->level, pop, current_level, set_active_level (uplevel!),
frame_ns}` (`runtime/rust/src/frame.rs`), `Var{Scalar, Array(BTreeMap),
Link(Link)}` with `VarHome{Frame(level), Namespace(NsId)}` and
`Link{home,name,elem}` (the `upvar`/`global`/`variable` model verbatim),
`Command{Builtin, Alias{target,prefix}, Imported{source}, Ensemble, Proc(Rc<ProcDef>),
child-interp}` (`runtime/rust/src/interp.rs`), and `Code{Ok,Error,Return,Break,
Continue}` with `as_int()` → TCL codes. **Family B is exactly these types with
`*mut TclObj` replaced by an associated `Value` — the identical move
`tcl-syntax::ExprOps` already makes** (`type Value; type Error;` + `&mut self`
methods; `rust/tcl-syntax/src/expr/eval.rs`). The const-folder, the runtimes, and
the VM each implement `ExprOps` over their own value today; Family B generalises
that proven pattern from "evaluate an expr" to "mutate the interp store".

**Granularity decision: many small role-traits, not one umbrella `Interp`
trait.** Different consumers need different subsets — the AOT emitter's
contract boundary (§5) touches only a handful of operations, a command core
touches the var store and value ops but not namespace creation, etc. One fat
trait would force every impl/consumer to know everything and would not match the
already-decomposed `runtime/rust` modules (`frame.rs`/`vars.rs`/`namespace.rs`/
`interp.rs`). Compose small traits instead.

**Handle model.** `NsId`/`FrameId`/`CommandId`/`VarId` are opaque arena indices
(newtype `u32`/`usize`), exactly as `runtime/rust` already uses `NsId` ("arena
id") and frame *level* (`usize`). Handles cross the trait boundary; the concrete
storage behind them does not. **Ownership under `forbid(unsafe)`:** the VM's
`Value = Rc<Obj>` makes the manual `release`/`decr_ref_count` discipline in
`frame.rs` *vanish* — `Drop` balances refs — so the VM's impl is strictly
*simpler* than `runtime/rust`'s `*mut TclObj` impl, not harder. That asymmetry is
fine: the trait is the contract; each impl meets it with its own memory model.

Trait shapes (illustrative — many small role-traits over an associated
`Value`, names/ids finalised in `tcl-runtime-api`):

```rust
trait NamespaceSystem {              // runtime/rust/src/namespace.rs analogue
    fn resolve_qualified(&self, cxt: NsId, name: &str) -> (NsId, /*simple*/ Range, Option<NsId>);
    fn find_command(&self, cxt: NsId, name: &str) -> Option<CommandId>;   // unqualified: cxt → path → root
    fn add_command(&mut self, ns: NsId, name: &str) -> CommandId;
    fn export(&mut self, ns: NsId, pat: &str);
    fn import(&mut self, dest: NsId, src: NsId, pat: &str);
    fn set_path(&mut self, ns: NsId, targets: &[NsId]);
}
trait FrameModel {                   // runtime/rust/src/frame.rs analogue
    fn push(&mut self, ns: NsId) -> FrameId;
    fn pop(&mut self);
    fn local_get(&self, f: FrameId, name: &str) -> Option<Value>;
    fn local_set(&mut self, f: FrameId, name: &str, v: Value);
    fn upvar(&mut self, here: FrameId, target: FrameId, local: &str, target_name: &str);
    fn global(&mut self, here: FrameId, name: &str);     // link to ::name
    fn variable(&mut self, here: FrameId, ns_var: VarId, local: &str);
    // uplevel runs a body with varFramePtr != framePtr — the two-pointer duality
}
trait TraceManager {                 // runtime/rust/src/cmd_trace.rs analogue
    fn add(&mut self, var_path: &str, ops: TraceOps, callback: Value);
    fn fire(&mut self, var_path: &str, op: TraceOp) -> Result<(), Error>;  // re-entrancy guarded
}
trait Introspection {                // info: command-introspection.md
    fn proc_body(&self, c: CommandId) -> Option<Value>;   // retained source (interpreted procs)
    fn proc_args(&self, c: CommandId) -> Option<Value>;
    fn level_argv(&self, level: i32) -> Option<Value>;    // requires per-frame argv retention
    fn commands(&self, cxt: NsId, pat: Option<&str>) -> Vec<Value>;  // walk cxt → path → root
    fn exists(&self, f: FrameId, name: &str) -> bool;
}
trait CommandDispatcher {            // runtime/rust/src/interp.rs analogue
    fn dispatch(&mut self, cxt: NsId, name: &str, argv: &[Value]) -> Completion;
    // routes Command{compiled, interpreted-proc, alias, ensemble, coroutine, builtin}
}
```

Plus the **catch/return model**: a `Completion{code, result, options}` with
`Code{Ok,Error,Return,Break,Continue}`, return-options dict, and incremental
`errorInfo`/`errorCode` accumulation on unwind
([docs/design/runtime/proc-call-and-stack-traces.md] §1.4–1.6;
`runtime/rust/src/cmd_control.rs`, `runtime/rust/src/interp.rs::settle_return`).

### 4c. Why these force the VM to be a *reified-state* runtime

Direct consequences for the bytecode VM — it cannot be a string-stack machine:

- **`upvar`/`uplevel`/`global`/`variable`** require a real frame stack with the
  C-Tcl two-pointer (`framePtr` vs `varFramePtr`) duality and by-name link
  descriptors; a variable that *might* be observed indirectly cannot live only
  in a local slot. Escape analysis (`var_escape.py`) decides slot vs frame-table
  per variable — the VM needs the frame table as the fallback store.
- **`info`** forces the runtime to **retain** what compilation would otherwise
  discard: interpreted-proc **bodies** (`info body`), **params** (`info args`/
  `default`), per-frame **argv** (`info level N`), and — for faithful
  `errorInfo`/`info frame` — a **CmdFrame** stack carrying command text + source
  line. (The Rust runtime keeps proc bodies; `info level N`/`info frame` are
  *still gaps* there — see §8.)
- **`trace`** means every variable write goes through a bottleneck that checks a
  trace registry; the VM's `local_set`/`global_set` must be those bottlenecks.
- **namespaces** are a mutable tree resolved at runtime; unqualified name
  resolution is context-dependent and `namespace path` is dynamic — most calls
  resolve at runtime, exactly as the WASM "eval fallback" does.

The VM therefore implements family-B traits over idiomatic Rust storage, reusing
family-A pure logic and `tcl-cmd-core` for command bodies.

### 4d. The `ValueOps` seam — write the command once

`tcl-cmd-core` holds the *body logic* of the value-shaped builtins (string/list/
dict/format/scan/math) generic over the value type, modelled **directly on
`ExprOps`** — associated `Value`/`Error`, `&mut self` receiver (so interning,
shimmer caching, and result-object construction stay the impl's business):

```rust
// rust/tcl-cmd-core/src/lib.rs  (M5)
pub trait ValueOps {
    type Value;
    type Error;
    // construction / shimmer
    fn from_str(&mut self, s: &str) -> Self::Value;
    fn from_int(&mut self, n: i64) -> Self::Value;
    fn as_str(&mut self, v: &Self::Value) -> Rc<str>;          // generates+caches string rep
    fn as_int(&mut self, v: &Self::Value) -> Result<i64, Self::Error>;
    // list (COW: mutate in place when unshared, copy when shared)
    fn list_len(&mut self, v: &Self::Value) -> Result<usize, Self::Error>;
    fn list_index(&mut self, v: &Self::Value, i: usize) -> Result<Option<Self::Value>, Self::Error>;
    fn list_from(&mut self, items: Vec<Self::Value>) -> Self::Value;
    fn list_append(&mut self, v: Self::Value, item: Self::Value) -> Result<Self::Value, Self::Error>;
    // dict, string, … (same shape)
}

// A command core is generic and storage-agnostic:
pub fn lrange_core<O: ValueOps>(ops: &mut O, list: &O::Value, from: &str, to: &str)
    -> Result<O::Value, O::Error> { /* parse end-relative indices, slice */ }
```

- **Zero-cost:** associated types monomorphise per impl. The VM (`Value =
  Rc<Obj>`) and `runtime/rust` (`Value = *mut TclObj`) each get a specialised
  copy; there is no dynamic dispatch and no boxing in the hot path. This is the
  same reason `ExprOps` is free today.
- **What stays out of `tcl-cmd-core`:** the *stateful* builtins (`set`/`proc`/
  `upvar`/`namespace`/`catch`/`trace`/`info`) — those need the §4b interp-state
  traits, not just `ValueOps`, and are reimplemented per runtime against those
  traits (the logic is thin once the traits exist).
- **Migration:** re-base the *Rust* WASM runtime (`runtime/rust`) command cores
  onto `ValueOps` first (same language, `*mut TclObj` impl); the VM consumes the
  same cores over `Rc<Obj>`. Two impls, one body. (§7.)

### 4e. CmdFrame / `errorInfo` / `info` — the subsystem the VM can lead

Faithful `errorInfo`, `info frame`, and `info level N` need a **CmdFrame stack
distinct from the var-frame stack** — C Tcl's `framePtr`/`varFramePtr` split has
a third axis: the *command* being evaluated, with its source text and line. This
is a **gap in the Rust runtime today** (no CmdFrame stack; `info frame`/`info
level N` unimplemented — [docs/design/runtime/command-introspection.md]). The VM
is the natural place to lead it, because **the bytecode already carries the
inputs**: `Instruction.source_line` and `Instruction.source_cmd_text`
(`rust/tcl-compiler/src/codegen/mod.rs`), plus `START_CMD` boundaries.

Design for the VM:

```rust
struct CmdFrame {
    cmd_text: Rc<str>,    // original command text (from Instruction.source_cmd_text)
    line: u32,            // 1-based source line (Instruction.source_line)
    kind: FrameKind,      // Toplevel | Proc(name) | Eval | Uplevel
}
```

- The VM pushes a `CmdFrame` at each `START_CMD`/`INVOKE_*` boundary from the
  instruction metadata, pops on completion. Cost is bounded (one push/pop per
  command) and pays for itself in fidelity.
- `info level` reads the var-frame depth; `info level N` reads the retained
  **per-frame argv** (captured at proc entry); `info frame` reads the CmdFrame
  stack; `info body`/`info args`/`info default` read the retained `ProcDef`
  (params + body source) — interpreted procs keep their body, exactly as the Rust
  runtime's `Command::Proc(Rc<ProcDef>)` does.
- `errorInfo` accumulates on unwind by walking the CmdFrame stack and appending
  `"\n    while executing\n\"<cmd_text>\""` / `"    (procedure \"name\" line N)"`
  frames — `TclLogCommandInfo`/`MakeProcError` semantics
  ([docs/design/runtime/proc-call-and-stack-traces.md] §1.5–1.6).
- This same `CmdFrame` design is what the Rust runtime would later adopt; the VM
  proving it out de-risks that.

### 4f. Return / catch / options — the completion model

The completion type threads through every dispatch and is the spine of
`catch`/`return`/`error`/`break`/`continue`:

```rust
struct Completion { code: Code, result: Value, options: Value }   // options = a dict Value
```

- **`return -code C -level L -options D ...`** constructs the completion: the
  bytecode is `returnImm {code, level}` + the result/options on the stack
  (`RETURN_IMM`/`RETURN_STK`/`PUSH_RETURN_OPTS`). `-level` decrements as the
  completion propagates up frames (`settle_return`/`TclUpdateReturnInfo`,
  `runtime/rust/src/interp.rs`); a proc boundary turns the final `Return` into
  `Ok` for the caller.
- **`catch script resVar optsVar`** is `beginCatch4 <depth>` … `endCatch` around
  the body, then `PUSH_RESULT` (the result/error message), `PUSH_RETURN_CODE`
  (the integer `Code::as_int`), and `PUSH_RETURN_OPTS` (the options dict). The VM
  truncates the exec stack to the catch entry's recorded depth on unwind, then
  pushes those three so the assignments to `resVar`/`optsVar` see them.
- **The options dict** carries `-code`, `-level`, and — on error — `-errorinfo`
  (the §4e CmdFrame-accumulated trace), `-errorcode`, and `-errorstack`. This is
  the single source `::errorInfo`/`::errorCode` are published from at the
  outermost eval, and what `return -options [dict get ...]` re-raises faithfully.
- **Family-B placement:** this is *not* `ValueOps` (it mutates interp result
  state and reads the CmdFrame stack) — it is part of the core `Interp` role
  traits, implemented per runtime. The *opcodes* (`BEGIN_CATCH4`/`END_CATCH`/
  `PUSH_*`) are the bytecode seam; the WASM seam is `tcl_catch_enter/leave/
  result/options` imports. Same semantics, two encodings (§5).

### 4g. The `tcl-runtime-api` decomposition (exact small traits)

The role-traits, each mirroring a `runtime/rust` module, all generic over the
associated `Value` (and parameterised by the opaque handles of §4b). A consumer
depends only on the subset it needs:

| Trait | Mirrors | Core methods (sketch) | Consumed by |
|-------|---------|------------------------|-------------|
| `ValueOps` | `tcl-syntax` value shape | `from_str/int`, `as_str/int/bool`, list/dict/string ops, COW append/set | every command core (§4d); `tcl-cmd-core` is generic over it |
| `VarStore` | `frame.rs` `Var`/`VarTable` | `get/set/unset/exists(frame,name[,elem])`, `array_*`, `link(upvar/global/variable)` | `set`/`incr`/`append`/`lappend`/`upvar`/`array`; the `*Stk` opcodes |
| `Frames` | `frame.rs` `FrameStack` | `push(ns)`, `pop`, `current_level`, `active_level` (uplevel), `frame_ns`, `argv(level)` | proc call/return, `uplevel`, `info level` |
| `Namespaces` | `namespace.rs` | `resolve_qualified`, `find_command`, `add_command`, `export/import`, `set_path`, `create`, `current` | command resolution, `namespace`, qualified names |
| `Commands` | `interp.rs` `Command` | `define/rename/delete/lookup → Command{Builtin,Proc,Alias,Imported,Ensemble,Child}`, `dispatch(cxt,name,argv)->Completion` | `INVOKE_STK`, `proc`/`rename`/`interp alias`, ensembles |
| `Traces` | `vars.rs` / `cmd_trace.rs` | `add/remove(var,ops,cb)`, `fire(var,op)` (re-entrancy-guarded) | the `VarStore` write/read/unset bottlenecks; `trace` |
| `Introspect` | `cmd_info.rs` + §4e CmdFrame | `proc_body/args/default`, `level_argv`, `cmd_frames`, `commands(pat)`, `exists` | the `info` family |
| `Completionʹ` | §4f | `set_result`, `return_options`, `accumulate_error_info`, code/level propagation | `catch`/`return`/`error`; `beginCatch`/`PUSH_*`/`returnImm` |

The `Vm` (and, symmetrically, `runtime/rust`'s `Interp`) implements **all** of
them; the point of splitting is that `tcl-cmd-core` cores and the *Rust* WASM
runtime port depend on minimal subsets, and the emitter↔runtime contract (§5)
names operations that each resolve to exactly one trait method. Do **not**
collapse them into one `Interp` trait — that re-couples everything and defeats
the per-consumer minimal-dependency goal.

## 5. The seam — the emitter↔runtime contract

What compiled code is allowed to assume about the runtime store. This is the
most important existing analog to the abstraction we're formalising:

- **WASM:** an explicit **import table** declared by
  `tcl-compiler/src/codegen/wasm/{executable,backend}.rs` from the canonical
  runtime ABI. The semantic plan emits the narrow `tcl_invoke_argv` contract
  when its proofs hold; a typed compatibility plan retains the general runtime
  surface. This *is* the runtime interface, expressed as WASM imports.
- **Bytecode/TCLVM:** the **opcode set + `INVOKE_STK` semantics**. `loadStk`/
  `storeStk`/`loadScalar`/`incrScalar`/`beginCatch`/`returnImm` are the
  contract; `INVOKE_STK` defers to the dispatcher exactly as `tcl_eval` does.

Both are the same idea — *"compiled code names a runtime operation; the runtime
store performs it"* — at different granularities. The canonical WASM pipeline
selects a semantic plan from registry and compiler proofs, then records a typed
compatibility reason when it must retain the general runtime path. The bytecode
emitter similarly uses a registry `bytecoded` hook for a known command shape,
or `INVOKE_STK` to defer to the dispatcher. The *shape* is shared and the
contract should be documented uniformly ([docs/design/contracts/], a future
`emitter-runtime-contract.md`).

**The VM is downstream of this selection — it has no separate codegen path.** The
tiering is a *compile-time* (emitter) decision; by the time the VM runs, the
inline-vs-`INVOKE_STK` choice is already baked into the bytecode. So "staircase
parity" is an **emitter-to-emitter** concern (does the bytecode `bytecoded` hook
inline the same constructs the WASM AOT tier does?), never a VM concern. The VM
just executes opcodes; when it hits `INVOKE_STK` it runs `CommandDispatcher`,
which is the runtime's own (always-available) fallback — there is no "can't
compile this" case at execution time. One corollary: the VM also executes
`EVAL_STK`/`evalStk` (compile-then-run an arbitrary script string) by invoking
the *compiler frontend at runtime* on the dynamic string — the VM's analogue of
the WASM runtime's `tcl_eval` interpreter, and the reason even a "fully AOT"
program keeps the compiler resident.

## 5b. Worked example — `upvar` + `info level`, both backends

Validating the interface shapes against a genuinely tricky construct rather than
asserting them. Consider a proc body:

```tcl
proc bump {varName} {
    upvar 1 $varName v       ;# dynamic target name → link into caller's frame
    incr v
    return [info level]      ;# introspection of the call stack
}
```

Trace each construct down the layers. The point: **both backends converge on the
same Family-B trait calls; only the value type and the emitter↔runtime
*mechanism* (opcode vs import) differ.**

| Construct | Family A — emitter (how it's lowered) | Seam (§5) | Family B — runtime trait call (identical semantics) |
|-----------|----------------------------------------|-----------|------------------------------------------------------|
| `upvar 1 $varName v` | `$varName` is dynamic ⇒ neither backend inlines; **TCLVM** emits `loadScalar`(varName) + `INVOKE_STK` to the `upvar` builtin; **WASM** emits the `tcl_upvar` import call (or eval fallback). | `INVOKE_STK` ↔ `tcl_upvar`/`tcl_eval` | `FrameModel::upvar(here, target=here.level-1, local="v", target_name=<value of varName>)` → installs `Var::Link{home:Frame(target), name, elem}`. Same call for both; value of `$varName` is `Rc<Obj>` (VM) or `*mut TclObj` (WASM). |
| `incr v` | `v` is alias-tainted (escape analysis saw `upvar`) ⇒ **not** a local slot; **TCLVM** emits `incrStk`/`INVOKE_STK incr`; **WASM** emits `tcl_local_incr`/import. | `incrStk` ↔ `tcl_local_incr` | `FrameModel::local_get/local_set` *follows the `Link`* to the caller's frame; the write goes through the trace-checked `set` bottleneck (`TraceManager::fire(write)`). Both backends, same trait path. |
| `info level` | introspection ⇒ both defer; **TCLVM** `INVOKE_STK info`; **WASM** `tcl_info_level`/eval. | `INVOKE_STK` ↔ `tcl_info_level` | `Introspection::level_argv`/var-frame depth (read of the §4e frame + CmdFrame state). |
| `return [info level]` | `return` of a value | `returnStk` ↔ `tcl_return` | `Completion{code: Return, result, options}`; proc boundary maps `Return`→`Ok` (`settle_return`). |

What this validates:
- The **dynamic name** (`$varName`) forces the *same* tier choice (eval/invoke
  fallback) in both backends — the staircase is shared in shape (§5).
- `upvar`/`incr`/`info` are **pure Family-B trait calls** — no shared *concrete*
  code, but one shared *interface*; the VM and WASM runtime each satisfy it over
  their own value/storage. This is precisely the "trait not type" split (§1.2).
- The only genuinely backend-specific artefacts are the **opcode vs import**
  encoding and the **value representation** — exactly the two things §3c and §6
  mark as irreducible.

A counter-example that *is* shareable concrete code: `return [lreverse $v]`
inside the same proc — `lreverse` is a `ValueOps` command core (§4d), one body
compiled for both value types. The contrast (trait for `upvar`, shared body for
`lreverse`) is the whole architecture in miniature.

## 6. The split — what gets lifted where

Concrete crate/module placement (Rust side; Python mirrors where relevant):

| Layer | Artifact | Home | Shared as |
|------|----------|------|-----------|
| Pure value logic | list/expr/number/format/glob/subst | `tcl-syntax` (exists) | **concrete code** (traits over the value: `ExprOps`) |
| Command cores | string/list/dict/format/scan/math command bodies | `tcl-cmd-core` (new, M5) | **concrete code** generic over a `ValueOps`/`Store` trait |
| Bytecode types | `Op`/`Instruction`/`FunctionAsm`/`ModuleAsm`/layout/format | `tcl-bytecode` (new, M0) | **shared types**, leaf crate (dep: `tcl-syntax`) |
| Emitter trait + CFG driver | `Backend`, shared `cfg_walk` | `tcl-compiler::codegen` | **trait + driver** (family A) |
| Runtime-state protocol | `NamespaceSystem`/`FrameModel`/`TraceManager`/`Introspection`/`CommandDispatcher`/`Completion` | `tcl-runtime-api` (new, design now / impl later) | **traits** (family B) |
| VM engine | dispatch loop, exec stack, reified frames/ns/traces | `tcl-vm` (new) | backend-specific, implements family-B traits |
| Value representation | `Rc<Obj>` dual-rep enum (VM) vs 24-byte ABI `Tcl_Obj` (WASM) | `tcl-vm` / `runtime/rust` | **NOT shared** (§1.2) |
| Storage/ABI/host-bridge/asyncify | linear-memory layout, LEB128, tagged handles | `runtime/rust` | **NOT shared** |

Dependency diamond (mirrors the existing `tcl-syntax` leaf pattern):

```
tcl-lexer ← tcl-syntax ← tcl-bytecode ← { tcl-compiler, tcl-vm }
                      ← tcl-cmd-core ← { tcl-vm, runtime/rust }      (M5)
                      ← tcl-runtime-api ← { tcl-vm, runtime/rust }   (traits)
```

`tcl-runtime-api` is the family-B trait crate, implemented directly by both
`tcl-vm` and the Rust runtime (`runtime/rust`). The non-Rust participants — the
Python WASM emitter and reference VM — do not *import* it; they converge by
**parity of shape**, enforced by the existing command-parity gate, not by a
shared link.

## 6b. Cross-language reality — concrete reuse is Rust-only

A caveat that qualifies the entire split, and which the "shared interface"
framing must not paper over: **the same-binary, concrete code reuse only spans
the *Rust* implementations.** The one runtime/emitter participant that is
not Rust today:

- The **WASM emitter is Python** (`compiler/codegen/wasm/`). The Rust bytecode
  emitter and a Python WASM emitter cannot literally share a `Backend` trait or a
  `cfg_walk` driver. They converge by **parity of shape** — the same IR/CFG
  vocabulary and the same `CommandRegistry` hook seam (which already exists in
  both languages) — not by code reuse. Family-A *concrete* sharing
  (the `Backend` trait, the lifted driver) is **Rust-emitter-to-Rust-emitter**
  and only becomes real once/if the WASM emitter is ported to Rust (a large,
  separate effort, explicitly out of scope here — §7).

The **canonical WASM runtime is now Rust** (`runtime/rust`), so it *does*
implement `tcl-runtime-api` and shares concrete code via `tcl-cmd-core`; that
convergence is the highest-value one below.

So the **trait/parity-not-link** story is:

| Participant | Language | Shares concrete code with VM? | Converges by |
|-------------|----------|-------------------------------|--------------|
| TCLVM bytecode emitter | Rust | n/a (it *is* the codegen the VM runs) | — |
| `tcl-vm` (the VM) | Rust | — | — |
| `runtime/rust` WASM runtime | Rust | **yes** — `tcl-cmd-core` + `tcl-runtime-api` impls | shared crates |
| WASM emitter | Python | no | parity of IR/registry shape |

Concretely, the **only pair that exchanges concrete Rust code** is `tcl-vm` ↔
`runtime/rust` (via `tcl-cmd-core` value cores and the `tcl-runtime-api` traits).
That is still the highest-value convergence — it is what lets a builtin be
written once and run in both the native VM and the WASM runtime — but the
doc should not imply the Python emitter gets it for free. It gets *interface
discipline*, not *implementation reuse*.

## 7. WASM migration steering

The WASM emitter and runtime are **not changed now**. They remain the oracle.
The migration path, for when we choose to converge them:

1. **Land family A for bytecode first** (the `Backend` trait + the VM consuming
   `tcl-bytecode`). Prove the trait shape against a real second consumer (the
   VM) before touching WASM.
2. **Introduce `tcl-cmd-core`** (§6, M5) and re-base the *Rust* WASM runtime
   (`runtime/rust`) command cores onto it first — same language, `Tcl_Obj` impl
   of `ValueOps`. The C Tcl 9 test suite stays the behavioural oracle.
3. **Extract the shared `cfg_walk` driver** only once both the bytecode and a
   *Rust* WASM emitter want it. The Python WASM emitter is migrated last, or
   left in place behind the same registry seam indefinitely.
4. **Document the emitter↔runtime contract** (§5) as the stable surface; mark
   the unstable surfaces (the runtime's linear-memory handle layouts,
   frame-alias bit-tagging, `Tcl_Obj` 24-byte layout) as explicitly *not* part of
   the shared interface.
5. **Never regress** the compiler/LSP or the Rust runtime; the C Tcl 9 test suite
   (`tmp/tcl9.0.3/tests/*.test`) stays the gold standard.

## 8. Longest poles / risks (call these out before building)

- **`errorInfo`/`info frame`/`info level N`** need a **CmdFrame** stack (command
  text + source line) and per-frame argv retention. The Rust runtime *doesn't yet
  have these*; the bytecode is already carrying `source_line`/`source_cmd_text`
  on `Instruction`, so the VM is actually well placed to lead here — but it is
  real work, not free.
- **Exception ranges.** C TEBC uses an explicit `ExceptionRange` table; our
  bytecode encodes catch via `BEGIN_CATCH4`/`END_CATCH` and loops via jump
  shape. The VM derives a range table at load (C1) for fidelity without an
  emitter change; byte-true parity (emitting the table, C2) is deferred.
- **Coroutines** require saving/restoring the whole frame stack (the recursive
  `runtime/rust` uses OS worker threads + channel handoff — see §8b); the VM
  would use native Rust control or an explicit
  continuation/segmented model — a parallel implementation, not shared.
- **The value-model split is permanent** while C-extension interop is required
  (§1.2). Convergence is at the trait + pure-logic layer only.
- **Don't over-abstract early.** Family A's shared driver and `tcl-cmd-core` are
  extracted *when the second consumer exists*, not speculatively.

## 8b. VM execution model & subsystem scope

Four decisions that the "deepen the thin areas" pass forces into the open. They
are VM-internal (they don't change the family-A/B interfaces) but they determine
whether the hard subsystems are even reachable.

### Non-recursive (NRE / trampoline) execution — decide this up front

The VM must **not** call `run()` recursively for nested `INVOKE_STK`/proc calls.
It should own an explicit stack of **activation records** (`{asm, pc, exec_stack,
frame_id, catch_stack}`) and trampoline over them, exactly as C Tcl rewrote TEBC
into the non-recursive `TEBCresume` + NRE callback stack (`tmp/tcl9.0.3/generic/
tclExecute.c`). Evidence this matters: the *recursive* tree-walking
`runtime/rust` cannot suspend mid-evaluation, so it implements `yield` with **OS
worker threads + channel handoff** (`runtime/rust/src/cmd_coro.rs`: "yield must
suspend execution arbitrarily deep in the recursive evaluator… rather than
rewrite"). A bytecode VM whose activation
stack *is data* needs neither. This is the **single biggest architectural reason
to prefer a bytecode VM**, and it is free only if NRE is designed in from M2
(proc calls), not retrofitted.

### Coroutines / `yield` / `yieldto` / `tailcall`

Fall out of NRE: a coroutine is a **saved activation sub-stack** (the records it
owns, with their pc/exec/frame state) held off to the side; `yield` returns
control to the resumer and parks the sub-stack; resume re-installs it and
continues. No threads, no asyncify. `tailcall` reuses the current activation
record instead of pushing. These are the constructs the Python reference VM and
the tree-walker fake; the VM does them natively. (Implementation lands post-M2,
but the NRE shape that enables them is an M2 decision.)

### `EVAL_STK` ⇒ the compiler is a runtime dependency (resolve the tension)

`eval`/`uplevel`/dynamic command names compile a string **at runtime**, so the
VM needs a compiler available during execution — which conflicts with the plan's
"`tcl-vm` must not depend on `tcl-compiler`" (kept lean). Resolution: the VM is
generic over compilation via an injected trait —

```rust
pub trait CompileService { fn compile(&self, src: &str, ns: NsId) -> Result<ModuleAsm, Diag>; }
```

`tcl-vm` depends only on `tcl-bytecode` (to *execute*) and on this trait (to
*request* compilation); the binary that wants `eval` wires a `tcl-compiler`-backed
impl at the top level (CLI/embedder), and tests use a stub. This keeps the crate
lean, keeps `tcl-vm ↛ tcl-compiler`, and makes the "compiler stays resident"
cost explicit and opt-in. It mirrors C Tcl, which always carries its bytecode
compiler; a "fully AOT" program that never hits `EVAL_STK` can link a
panicking stub and drop the compiler entirely.

### Scope: ensembles in / child-interps later / TclOO deferred

- **Ensembles** — in scope and cheap: `Commands` already routes
  `Command::Ensemble` (argv[1] → target prefix), and `runtime/rust` has
  `ensemble.rs` to mirror. Dispatch-only, no new machinery.
- **Child interpreters** — in scope but later (post-M2): each interp is its own
  `Vm`/interp-state; the Family-B handles (`NsId` per-interp root, the
  recursion/re-entrancy bound `runtime/rust` already models) accommodate it. Not
  needed for core fidelity.
- **TclOO** — deferred, matching the WASM stance (an extension, not core). Note
  `runtime/rust` carries a `cmd_oo.rs` stub; the VM can adopt the same
  "present-but-minimal" posture and grow it via the same `Commands`/`Namespaces`
  traits later. Out of scope for M0–M4.

## 9. Cross-links

- [docs/design/compiler/codegen-module-map.md] — the two codegen backends.
- [docs/design/compiler/wasm-codegen.md] — canonical WASM codegen pipeline.
- [docs/design/compiler/wasm-runtime-primitives.md] — the import boundary.
- [docs/design/runtime/namespace-tree.md], [docs/design/runtime/proc-call-and-stack-traces.md], [docs/design/runtime/command-introspection.md], [docs/design/runtime/trace-implementation.md], [docs/design/runtime/rename-alias.md], [docs/design/runtime/child-interp.md] — the family-B subsystems.
- [docs/design/runtime/c-extension-abi.md] — why the value model can't be one type.
- [docs/design/contracts/vm-bytecode-test-boundary.md] — bytecode identity/test boundary.

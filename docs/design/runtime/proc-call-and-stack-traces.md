# Proc call, exceptions, and stack traces — the call protocol

> *The proper way to call something*, and *what information every call must
> carry*, so that stack traces, `info frame` / `info level`, exceptions,
> `eval` / `uplevel` / `source` / `package`, and **AOT ↔ interpreter interop**
> all work. Grounded in the C Tcl 9 truth
> (`tmp/tcl9.0.3/generic/{tclProc,tclBasic,tclCmdMZ,tclResult,tclNamesp}.c`,
> `tclInt.h`). Section numbers are cited from `runtime/rust/src/interp.rs`,
> `cmd_proc.rs`, and `cmd_error.rs`, so keep them stable.

## Guiding principle — conservative first, prove-and-drop last

**Every call carries the full frame / source / line / error information — in
the interpreter *and* in the AOT-emitted code.** A compiled proc must produce a
byte-identical stack trace to the same proc interpreted, and the test suite
asserts exact `errorInfo` / `info frame` output. Information is dropped only
where it is *proved* unobservable (§7), never by default.

Corollary for codegen: the AOT emit path's default is **not** "skip the
bookkeeping"; it is "emit the bookkeeping, then elide it only behind a proof".
The proof obligations are enumerated in [§7](#7-the-end-of-project-elision-pass).

## The hard core — Tcl's dynamic scoping vs tighter languages

The genuinely tricky part of porting Tcl to Rust/WASM is **not** the per-command
work; it is that Tcl lets code reach into *other* execution contexts in ways a
tighter language structurally forbids:

- **`upvar`** binds a local name to a variable **in another frame** (the
  caller's, or any level) — a live alias across stack frames.
- **`uplevel N $script`** runs a script **in another frame's variable scope** —
  arbitrary code execution against someone else's locals.
- **`namespace eval ns $script`** / **`variable`** run/resolve against a
  **different namespace's** variables and commands.
- **`eval`/`subst`/dynamic command+var names** construct and run code (and reach
  variables) **decided at runtime**, not statically.

Rust's borrow/scope model — and any compiled language's stack frames — would
**not permit** one function to mutate another's locals by name, or to execute a
string against a chosen scope. So the runtime cannot lean on Rust stack frames
for Tcl scope: it owns a **reified** frame/variable model (the `FrameStack` of
the two-chain `CallFrame` of §3) where *any* frame's variables are
addressable by index, and cross-frame links are first-class. That reification is
exactly what makes the dynamic behaviour expressible at all.

**Order of work, non-negotiable: get all of this dynamic behaviour correct
first, then optimise from that point.** Concretely:

1. The reified frame/var model must support **arbitrary** cross-frame
   (`upvar`/`uplevel`), cross-namespace (`namespace eval`/`variable`), and
   dynamic-name access — fully and correctly — before any speed work.
2. The AOT compiler's **default is the safe, fully-reified path**: a proc that
   has *any* of these dynamic constructs reachable keeps a **real runtime
   frame** (no elision), and dynamic eval/name constructs **fall back to the
   interpreter** (the staircase rule). The S7 metaprogramming heuristics and the
   S2/S3 frame-elision proofs are how we *recover* speed — strictly as
   optimisations *over* a correct, fully-dynamic baseline, never as a
   prerequisite for it.
3. Frame elision (S2: locals → WASM locals) and the §7 diagnostic-elision pass
   fire **only** where escape analysis proves no `upvar`/`uplevel`/`eval`/
   `namespace`/`info frame`/`info level` can observe the frame. Absent the
   proof, the real frame stays. This is the same posture as the conservative
   principle above, applied to *scope* rather than to *diagnostics*.

So the build order is: **reified frames + full dynamic semantics (correct) →
then measure → then prove-and-optimise.** The tricky cross-scope cases are the
foundation, not an afterthought.

---

## 1. The C Tcl truth — two stacks, three pointers

Tcl threads **three pointers** (`Interp` fields) over **two distinct stacks**:

| Pointer | Stack | Purpose |
|---|---|---|
| `framePtr` | `CallFrame` | Top of the **call chain** — proc activation records (who is running). |
| `varFramePtr` | `CallFrame` | The frame whose **variables** are currently visible. Equals `framePtr` **except** inside `uplevel` (and `namespace eval`, `apply`), which run a body with a *different* variable scope than the call chain. |
| `cmdFramePtr` | `CmdFrame` | Top of the **source-location** stack — for `info frame`, line numbers, and stack traces. |

### 1.1 `CallFrame` — variable scope + activation (`tclInt.h:1275`)

The load-bearing fields:

- `nsPtr` — namespace for command/global-var resolution.
- `objc` / `objv` — **the arguments of this call** (`info level N` returns these).
- `callerPtr` — `framePtr` at call time: the **call chain** (one higher proc).
- `callerVarPtr` — `varFramePtr` at call time: the **variable-scope chain**
  (same as `callerPtr` *unless* the caller was inside an `uplevel`).
- `level` — `uplevel` nesting depth (1 = outermost proc, 0 = top level).
- `procPtr` — the `Proc` being run (params, body, compiled-local count).
- `varTablePtr` / `compiledLocals` — locals (hash for dynamic names, indexed
  array for compiler-known names — the latter is the AOT fast path's analogue).

**The two-chain distinction is the whole game for `upvar`/`uplevel`/`info
level`.** Scalar-var resolution is split from the call by storing
links by path; this doc makes the *frame* carry both chains explicitly.

### 1.2 `CmdFrame` — source location (`tclInt.h:1366`)

A **separate** stack, pushed per evaluated script/command, holding everything a
diagnostic needs:

- `type` — `TCL_LOCATION_{EVAL, BC, PREBC, SOURCE, PROC}` (dynamic eval /
  bytecode / precompiled / sourced-file / proc-body).
- `line[]` + `nline` — **the source line each word of the command starts on**.
- `framePtr` — the `CallFrame` this command runs in (may be NULL).
- `nextPtr` — link to the calling `CmdFrame`.
- `data.eval.path` — the **sourced file's path** (for `TCL_LOCATION_SOURCE`).
- `cmd` / `len` / `cmdObj` — the **command string** being executed.

This is exactly the information `info frame` returns and that the error
unwinder needs to say *which file, which line, which command*.

### 1.3 The unified call (`TclEvalObjvInternal`, `tclBasic.c`)

Every command — builtin, proc, extension — is invoked the same way:

1. Resolve `objv[0]` (the command name) in the command table (current ns → global).
2. Call the command's `objProc(clientData, interp, objc, objv)`.
3. A **proc**'s `objProc` is `TclObjInterpProc` (`tclProc.c:1613`): it pushes a
   `CallFrame` (binding `objv` to the params), pushes/updates the `CmdFrame`
   context to `TCL_LOCATION_PROC`, evaluates the body, and pops on return.

So "the proper way to call something" is: **one dispatch entry point that
establishes the callee's frame context, regardless of callee kind.** This is
the seam that makes AOT↔interp interop uniform (§5).

### 1.4 Exceptions — the return-options dict (`tclResult.c:618`)

The completion code (`TCL_OK/ERROR/RETURN/BREAK/CONTINUE`) is only half the
model. The full exception state is a **return-options dictionary** with keys:

`-code` · `-level` · `-errorcode` · `-errorinfo` · `-errorline` ·
`-errorstack` · `-options`.

- `return -code error -level 0` re-raises an error in the caller; `-level N`
  controls how many frames the return unwinds (`TclProcessReturn` /
  `TclMergeReturnOptions`, `tclCmdMZ.c`).
- `catch {script} resultVar optionsVar` captures the code + the full options
  dict, so `return -options $opts` faithfully re-raises (TIP 90).
- `Interp` carries `errorInfo`, `errorCode`, `errorLine`, `errorStack`,
  `returnOpts` (`tclInt.h:2001/2157-2163/2345`).

### 1.5 How a stack trace is built (the key mechanism)

`errorInfo` is **accumulated as the error unwinds**, not computed at the throw:

- At the throw site, `errorInfo` starts as the error message.
- As each command frame unwinds, **`Tcl_LogCommandInfo`** (`tclBasic.c:4828/
  5303/5576`) appends `"\n    while executing\n\"<cmd>\""` or `"\n    invoked
  from within\n\"<cmd>\""`, using the **`CmdFrame`'s command string + line**.
- When a proc frame unwinds, **`MakeProcError`** (`tclProc.c:2085`) appends
  `"\n    (procedure \"<name>\" line <N>)"`, where `<N>` is
  `Tcl_GetErrorLine(interp)` — the line **within the proc body**.
- The `ERR_ALREADY_LOGGED` interp flag prevents a frame from logging twice.

**Therefore the runtime must know, at every unwinding step: the command string,
its source line, and (for procs) the proc name + the body-relative line.** That
is precisely what the `CmdFrame` stack stores — and precisely what the AOT path
must also emit (the conservative principle).

---

## 2. Why both stacks are needed

Tracking only the `CallFrame` side (frames, argv, `upvar`) leaves out the
`CmdFrame` / `errorInfo` / `info frame` machinery entirely — no
source-location stack, no incremental `errorInfo` unwinder — and those are
exactly what a user sees when something goes wrong. For stack traces the **C
source is the ground truth**, and the Tcl 9 test suite is the arbiter.

---

## 3. The Rust design — frame model

Two stacks, mirroring Tcl, on top of the runtime's `FrameStack`.

### 3.1 `Frame` — the call / variable frame

```rust
struct Frame {
    table:           VarTable,      // this frame's locals (and its links)
    compiled_slots:  Vec<…>,        // indexed ports for AOT-bound locals
    level:           usize,         // logical call level (`info level` arithmetic)
    ns:              NsId,          // namespace current when this frame was pushed
    words:           Vec<Vec<u8>>,  // the invoking command words (`info level N`)
    is_proc:         bool,          // proc call vs `namespace eval`/`inscope` frame
    saved_active:    usize,         // the active level to restore on pop
}
```

C's two chains are realised without two parent pointers. `FrameStack` is a
`Vec<Frame>`, so the **call chain** is stack order; the **variable-scope chain**
is `FrameStack::active_level`, the level whose frame unqualified names resolve
against. `uplevel` redirects `active_level` and a frame records the value to
put back in `saved_active`, so the redirection unwinds correctly even when a
proc is invoked *while* an `uplevel` is in effect — the case where the logical
`level` and the stack index genuinely diverge (`uplevel 1 [list SomeProc …]`,
tcltest's idiom). `push_same_level` is the push for that case.

That divergence is why `level` alone cannot identify a frame, and why the
`CmdFrame` below carries a separate `frame_index`.

### 3.2 `CmdFrame` — the source / diagnostic stack

```rust
enum FrameKind { Eval, Source, Proc }        // ~ TCL_LOCATION_*
struct CmdFrame {
    kind:            FrameKind,
    file:            Option<Rc<[u8]>>,  // sourced-file path
    proc:            Option<Vec<u8>>,   // the proc FQN this frame runs in
    level:           usize,             // the proc call level
    omit_level:      bool,              // C drops `level` for an `uplevel` body
    frame_index:     usize,             // stack index of the CallFrame (identity)
    line_base:       u32,               // added to a body-relative line
    proc_line_base:  u32,               // the enclosing body's base, for errorLine
    cmd:             Vec<u8>,           // the executing command (the `cmd` key)
    line:            u32,               // its reported source line
}
```

A parallel `Vec<CmdFrame>` on the interp (the `cmdFramePtr` stack). `eval`
pushes `Eval`; `source` pushes `Source` with the file path; a proc call pushes
`Proc`. The line is computed from the script and the command's byte offset (the
parser supplies byte offsets — line = count of `\n` before the offset).

The two line bases are the subtle part. `line_base` is what a body-relative
line is added to, and an inline body (`if` / `while` / `catch`) temporarily
re-points it at the sub-body's own base while it runs. `proc_line_base` is the
base of the enclosing `codePtr->source` — the proc, lambda, or `eval` body the
commands ultimately belong to — captured at frame creation and deliberately
*not* moved by those inline shifts, because C computes `errorLine` against
`codePtr->source`, which an inline body shares with its proc.

### 3.3 Exception state

`Code` stays the completion code; `ExceptionState` carries the accumulating
trace:

```rust
struct ExceptionState {
    info:            Option<Vec<u8>>, // accumulating errorInfo; None until the
                                      // first frame is appended (C's NULL)
    code:            Vec<u8>,         // ::errorCode
    code_explicit:   bool,            // an explicit `-errorcode {}` reads back
                                      // empty rather than defaulting to NONE
    already_logged:  bool,            // ERR_ALREADY_LOGGED
}
```

`info` being `Option` is load-bearing: `None` is what selects `while executing`
over `invoked from within` and seeds the buffer from the result message.

The rest of the exception state lives beside it on `InterpState` rather than
inside this struct, because its lifetime differs: `error_line` (C's
`iPtr->errorLine`) is **persistent** interp state written only by
`log_command_info`, surviving `catch` and the start of a fresh error;
`return_code` / `return_level` are the pending `return -code`/`-level`; `during`
holds the TIP 329 `-during` chain; and `error_stack` / `reset_error_stack` are
the TIP 348 stack. `error` / `catch` / `try` / `return -options` operate across
all of them, and `snapshot_error` / `restore_error` move the accumulating slice
between flows (a coroutine swap).

---

## 4. The call protocol (one entry point)

One dispatch establishes the callee's frame context for **every** callee kind,
so any caller (interpreter or AOT) calls any callee uniformly:

```
fn invoke(interp, argv) -> Code:
    cmd = lookup(argv[0])                 # command table (ns → global)
    push CmdFrame for this command         # source/line bookkeeping (always)
    match cmd:
        Builtin(f)      => f(interp, argv)                 # no CallFrame
        Proc(p)         => call_proc(interp, p, argv)      # push CallFrame+bind
        AotCompiled(fi) => call_aot(interp, fi, argv)      # push CallFrame+bind, call wasm fn
        External(idx)   => call_indirect(idx, argv)        # extension (§4.6 ABI)
    pop CmdFrame
    on Code::Error: log_command_info(interp, this CmdFrame)   # append errorInfo
```

The `Command` enum carries `Proc`, `AotCompiled`, and `External` variants. The
`call_proc`/`call_aot` paths push the **same** `CallFrame` + bind args
identically; they differ only in whether the body runs through `eval_str`
(interp) or a WASM function call (AOT). **Both push the `CmdFrame`/`CallFrame`
and set the proc-error context**, so the stack trace is identical either way.

`call_proc` (mirrors `TclObjInterpProc`):
1. arity-check `argv` vs the proc's params (→ `wrong # args`, the param-list form).
2. push `CallFrame { argv, caller, caller_var, level: caller.level+1, proc, ns }`.
3. bind params to locals (defaults; `args` collects the rest).
4. set `error_line` base; eval the body (`Code::Return` from the body becomes
   `Code::Ok` of the call — a proc-level `return`).
5. on `Code::Error`, append `MakeProcError`'s `(procedure "name" line N)` frame.
6. pop `CallFrame`.

---

## 5. AOT ↔ interpreter interop

The north star: most procs compile AOT; the interpreter is the fallback. Both
must call each other **through the one protocol above**, with identical frame
info — this is what the conservative principle buys.

- **Interpreter → AOT proc:** the command table entry is `AotCompiled(fn_index)`;
  `invoke` pushes the `CallFrame`/`CmdFrame`, then `call_indirect`s the WASM
  function. The compiled function receives the bound frame.
- **AOT proc → any command:** compiled code does **not** inline an arbitrary
  command (it can't know load-time/extension commands); it emits a call to the
  runtime's `invoke` (the same entry point), so an interpreted/extension callee
  gets its frame established normally. (Provably-safe inlining is the staircase's
  job; the *fallback* call is always `invoke`.)
- **The shared frame stacks live in the runtime**, not in compiled code, so a
  mixed AOT/interp call chain produces one coherent `CallFrame`/`CmdFrame` stack
  — and therefore one coherent stack trace and `info frame`/`info level`.
- **What the AOT emitter must emit (conservative default):** for each compiled
  proc, the prologue pushes the `CallFrame` + a `Proc`/`AotProc` `CmdFrame` with
  the proc name and the body's source line table; each compiled command site
  updates the current line; the epilogue pops. This is bookkeeping the compiler
  emits **by default** so traces match — see §7 for when it may be dropped.

---

## 6. How the rest fits

- **`eval $script`** — push an `Eval` `CmdFrame`; evaluate via the interp
  (`eval_str`). `uplevel N $script` — evaluate with `var_frame` set to the
  frame N up the **var-scope** chain (`caller_var`), not the call chain. These
  are the metaprogramming-fallback paths (the AOT staircase S7 compiles the
  static cases; the dynamic cases land here).
- **`source file`** — read the file, push a `Source` `CmdFrame` carrying the
  file path, evaluate the contents; errors get `TCL_LOCATION_SOURCE` line/file
  info. The path + line is what makes `errorInfo` cite the real file.
- **`package require`/`provide`** — a command (no special frame protocol) that
  triggers `source`/load of the providing script; it inherits the `source`
  frame machinery for traces. (Loading a **C** extension is the Track-2 loader.)
- **`info level ?N?`** — reads the `CallFrame` chain (`argv`, `level`).
  **`info frame ?N?`** — reads the `CmdFrame` chain (type/file/line/cmd).
  **`catch`/`error`/`return`** — operate on the `ExceptionState` + options dict.

---

## 7. The end-of-project elision pass

Per the conservative principle, an optimisation stage (new AOT-staircase
sub-stage, after S6/S7) may **drop frame/source bookkeeping it can prove
unobservable**. A compiled proc may elide:

- the `CmdFrame` push / per-command line updates **iff** no reachable code can
  observe them: no `error`/`catch` that inspects options, no `info frame`/`info
  level`/`info errorstack`, no `uplevel`/`upvar` depending on `level`, and no
  call into a callee that could (transitively) — i.e. an escape-analysis over
  "is the diagnostic state observed?".
- the `CallFrame` `argv` retention **iff** `info level` is unreachable.
- proc-error context **iff** the proc is proven non-throwing.

Each elision is gated by a proof; absent the proof, the bookkeeping stays. This
is the *only* place information is dropped, and it lands last, measured against
the Tcl 9 suite's exact-`errorInfo` tests so nothing regresses.

---

## 8. What the runtime does, and where it approximates

The frame model, `proc` / `apply` calling (defaults, `args` catch-all,
`wrong # args`, the recursion bound), the dynamic cross-scope core
(`uplevel` / `upvar` / `global` / `variable` / `namespace eval` / `eval`, and
dynamic command and variable names), the exception machinery, and `info frame`
are all implemented and byte-verified against tclsh 9.0.

### Source location

`parse::Command` carries `start` (C's `commandStart`) and `end` (the
terminator-excluded command end). The logged command slice keeps trailing
whitespace but drops the `\n` / `;` terminator, and the 1-based line is
`1 + count('\n' in src[0..start])` — both byte-verified.

### `errorInfo`

`ExceptionState` (`info` / `code` / `line` / `already_logged`) is the analogue
of C's `iPtr` `errorInfo` / `errorCode` / `errorLine` / `ERR_ALREADY_LOGGED`,
and the unwinder builds the trace incrementally:

- `Interp::log_command_info` at each `eval_command` — `while executing` /
  `invoked from within`, 150-byte truncation;
- `make_proc_error` in `run_proc` — `(procedure "x" line N)` /
  `(lambda term "…" line N)`, 60-byte truncation;
- body frames — `("eval" / "uplevel" / "foreach" body line N)`.

`error` / `throw` / `catch` / `try` are all built on it, and the trace is
published to `::errorInfo` / `::errorCode` at the catch or outermost-eval
boundary. Logging happens at the *unwinding* site, where the source and the
command range are both in scope, so the success path pays no per-command
push/pop — matching C, which calls `TclLogCommandInfo` as the error returns
through each level.

### `info frame`

A persistent `CmdFrame` stack (`{kind, file, proc(FQN), level, line_base, cmd,
line}`) is pushed per script-eval level Tcl tracks — the root script, a proc
call, an `eval` / `uplevel` body, and a `source`d file — but **not** a `[cmd]`
substitution and **not** an inline `if` / `while` / `for` / `foreach` body.
The eval loop unifies on `eval_script(src, owned)`; each command updates the
current frame's `cmd` / `line`, and a substitution reports the substituted
command at the enclosing line.

`ProcDef` records its FQN, defining source, and body line-base, so a proc frame
is either `type proc` (eval-defined, body-relative lines) or `type source` plus
`file` (source-defined, **file-absolute** lines via the line-base — C's literal
line table). An `eval` body inherits the enclosing kind and file with a
file-absolute base. An `uplevel` body is `type eval`, carries no file, is
body-relative, names the invoking proc, and **omits the `level` key**, because
its scope is redirected — C's var-chain-reachability rule.

### Known approximations

These are places where the message is always correct and only the trace framing
differs, all of them instances of the same bytecode-boundary class — C inlines
constructs when it compiles the enclosing body, and a tree-walker has no
equivalent inlining decision:

- `foreach` adds its body frame only at top level (`!in_proc()`), because C
  inlines `foreach` inside a compiled proc body and produces no frame there;
  `if` / `while` / `for` / `switch` are always inlined and never produce one.
- `expr {1/0}` and `expr` *parse* errors report `while executing` where
  tclsh's bytecode engine seeds the inner context to report
  `invoked from within`. Variable-not-found, domain errors, and traces
  propagated out of a `[cmd]` substitution all match exactly.
- An `eval` body's `type` can differ from tclsh when its bytecode compiler
  inlines the literal body (sometimes `proc` where we say `eval`). The
  inherit rule matches the sourced-file cases but not every inlined
  `eval {literal}`.

### Deliberate exclusion: `info errorstack`

TIP 348's `INNER {…}` element is the **bytecode** execution context (tclvm
opcodes — `returnImm`, `loadStk`, …), present even at top level because modern
Tcl compiles everything. A runtime that emits WASM rather than tclvm bytecode
cannot reproduce it; this is the same class as the suite's bytecode and
disassembly exclusions. It degrades to a clean `unknown subcommand` error.

### The AOT path

The AOT emit path carries the same bookkeeping, and the compiler keeps a real
frame plus an interpreter fallback wherever any dynamic construct is
reachable. The interop gate is that a compiled proc and an interpreted proc in
one chain produce identical `errorInfo`, `info frame`, and `info level`.
Elision (§7) applies only where escape analysis proves no dynamic construct can
observe the dropped information.

## Cross-references

- [`c-extension-abi.md`](c-extension-abi.md) §4.6 — extension-command dispatch
  (the `External` call path).
- [`namespace-tree.md`](namespace-tree.md) — namespace resolution and the
  `nsPtr` field.
- [`../contracts/runtime-variable-frame-model.md`](../contracts/runtime-variable-frame-model.md)
  — the frame/cell model this call protocol pushes and pops.
- [`../compiler/var-escape-analysis.md`](../compiler/var-escape-analysis.md) —
  the proof contract that gates frame elision.

# Proc call, exceptions, and stack traces — the call protocol

> Design doc, written **before** implementing procs (gates the T1.6 proc chunk). It figures out *the proper
> way to call something* and *what information every call must carry* so that
> stack traces, `info frame`/`info level`, exceptions, `eval`/`uplevel`/`source`/
> `package`, and **AOT↔interpreter interop** all work. Grounded in the C Tcl 9
> truth (`tmp/tcl9.0.3/generic/{tclProc,tclBasic,tclCmdMZ,tclResult,tclNamesp}.c`,
> `tclInt.h`), then reasoned to a Rust design.

Based on `rust`@`8150eca` (see the port doc's base-hash banner).

## Guiding principle — conservative first, prove-and-drop last

**Carry the full frame / source / line / error information on every call — in
the interpreter *and* in the AOT-emitted code — from the start.** A compiled
proc must produce a byte-identical stack trace to the same proc interpreted.
Only at the **end of the project** do we add an optimisation pass that *drops
information we can prove is unobservable* (a new AOT-staircase sub-stage). This
mirrors the staircase rule (emit only what's provably correct; here: keep
everything until provably droppable) and keeps the gold-standard test suite
(which asserts exact `errorInfo`/`info frame` output) green throughout.

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
T1.3 + the two-chain `CallFrame` of §3) where *any* frame's variables are
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
level`.** T1.3 already split scalar-var resolution from the call by storing
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

## 2. The gap to close

A minimal port tracks the `CallFrame`-side (frames, argv, upvar) but **none** of
the `CmdFrame` / `errorInfo` / `info frame` machinery — no source-location stack
or incremental-`errorInfo` unwinder. So for
stack traces the **C source is the ground truth** (per the project's "defer to
C + the Tcl 9 test suite" rule), and this is net-new design in the Rust
runtime, not a port of prior runtime code.

---

## 3. The Rust design — frame model

Two stacks, mirroring Tcl, extending the T1.3 `FrameStack`.

### 3.1 `CallFrame` (extend T1.3)

```
struct CallFrame {
    vars:        BTreeMap<Vec<u8>, Var>,   // T1.3 (locals)
    ns:          NsId,                     // namespace (T1.5)
    argv:        Vec<*mut TclObj>,         // this call's args — `info level`
    caller:      usize,                    // call-chain parent  (callerPtr)
    caller_var:  usize,                    // var-scope parent   (callerVarPtr)
    level:       usize,                    // uplevel nesting
    proc:        Option<ProcId>,           // the running proc, if any
}
```

The `caller` vs `caller_var` split is the `uplevel`/`info level` keystone (T1.3
resolved var *links* by path; the frame now records both chains so `info level`
/ `uplevel N` / `info frame` are exact).

### 3.2 `CmdFrame` (new — the source/diagnostic stack)

```
enum FrameKind { Eval, Source, Proc, AotProc }      // ~ TCL_LOCATION_*
struct CmdFrame {
    kind:    FrameKind,
    file:    Option<Rc<[u8]>>,    // sourced-file path (Source)
    cmd:     Range<usize>,        // span of the executing command in its script
    line:    u32,                 // 1-based source line of the command
    word_lines: Vec<u32>,         // per-word start lines (info frame)
    call:    usize,               // the CallFrame index this runs in
    proc_name: Option<Vec<u8>>,   // for the "(procedure ... line N)" frame
}
```

A parallel `Vec<CmdFrame>` on the interp (the `cmdFramePtr` stack). `eval`
pushes `Eval`; `source` pushes `Source` with the file path; a proc call pushes
`Proc`/`AotProc`. The line is computed from the script + the command's byte
offset (we already have byte offsets from T1.2 parse — line = count of `\n`
before the offset, cached per script).

### 3.3 Exception state (extend the `Code` enum)

`Code` (T1.4) stays the completion code. Add the options the unwinder needs:

```
struct ExceptionState {
    error_info:  Vec<u8>,       // accumulated trace (errorInfo)
    error_code:  *mut TclObj,   // errorCode (a list)
    error_line:  u32,           // line within the current frame
    return_level: usize,        // `return -level`
    return_code:  Code,         // `return -code`
    already_logged: bool,       // ERR_ALREADY_LOGGED
}
```

`error`/`catch`/`return -code -level -options` operate on this. `catch` snapshots
it into the options dict; `return -options $d` restores it.

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

`Command` (T1.4's enum) gains `Proc`, `AotCompiled`, `External` variants. The
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
  (`eval_str`, T1.4). `uplevel N $script` — evaluate with `var_frame` set to the
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

## 8. Implementation plan (the proc chunk series)

**Correctness of the dynamic cross-scope core comes first (the hard core
above); speed is recovered only afterwards.**

1. **PC-1 — frame model. ✅ landed (source/line half).** `parse::Command` now
   carries `start` (C's `commandStart`) + `end` (terminator-excluded command
   end); the logged command slice keeps trailing whitespace but drops the
   `\n`/`;` terminator and the 1-based line is `1 + count('\n' in src[0..start])`
   — both byte-verified against tclsh 9.0. The `CallFrame` two-chain (argv +
   caller/caller_var) already landed with PC-3. The **persistent `CmdFrame`
   source stack** (needed for `info frame`) is **deferred to PC-5**: PC-4 logs at
   the unwinding site (where `src` + the command range are in scope), so the
   success path pays no per-command push/pop — matching C, which calls
   `TclLogCommandInfo` as the error returns through each level.
2. **PC-2 — `proc` + call. ✅ landed** (`cmd_proc.rs`, `Interp::run_proc`):
   defaults, `args` catch-all, `wrong # args`, recursion bound, `apply`.
3. **PC-3 — the dynamic cross-scope core. ✅ landed:** `uplevel`/`upvar`/
   `global`/`variable`/`namespace eval`, `eval`, dynamic command+var names
   through the interpreter (T1.5 + the proc chunk).
4. **PC-4 — exceptions. ✅ landed** (the headline). `ExceptionState`
   (`info`/`code`/`line`/`already_logged`, the `iPtr` errorInfo/errorCode/
   errorLine/`ERR_ALREADY_LOGGED` analogue) + the incremental `errorInfo`
   unwinder: `Interp::log_command_info` (`while executing` / `invoked from
   within`, 150-byte truncation) at each `eval_command`, `make_proc_error`
   (`(procedure "x" line N)` / `(lambda term "..." line N)`, 60-byte truncation)
   in `run_proc`, and `("eval"/"uplevel"/"foreach" body line N)` body-frames.
   `error`/`throw`/`catch`/`try` rewired onto it; the trace is published to the
   `::errorInfo`/`::errorCode` globals at the catch / outermost-eval boundary.
   `return -code -level -options` errorinfo restore is the remaining sub-item.
   Gate: byte-exact `errorInfo` unit tests + a tclsh 9.0 differential sweep.
   - **Tree-walker approximations of the bytecode boundary** (message always
     correct; only the trace framing differs): `foreach` adds its body-frame
     only at top level (`!in_proc()`) — C inlines `foreach` when it compiles the
     enclosing proc body, so no frame there; `if`/`while`/`for`/`switch` are
     always inlined (never a frame). `expr {1/0}` and expr *parse* errors show
     `while executing` where tclsh's TEBC seeds the inner context to show
     `invoked from within`; var-not-found / domain errors / propagated
     `[cmd]`-substitution traces all match.
5. **PC-5 — `info frame` + `source` frames. ✅ landed, byte-exact.** A
   persistent `CmdFrame` stack (`{kind, file, proc(FQN), level, line_base, cmd,
   line}`) is pushed per script-eval level Tcl tracks — the root script, a proc
   call, an `eval`/`uplevel` body, and a `source`d file — but **not** a `[cmd]`
   substitution or an inline `if`/`while`/`for`/`foreach` body (verified vs
   tclsh). The eval loop unifies on `eval_script(src, owned)`; each command
   updates the current frame's `cmd`/`line` (a substitution reports the
   substituted command at the enclosing line). `ProcDef` records its FQN,
   defining-source, and body line-base, so a proc frame is `type proc`
   (eval-defined, body-relative lines) or `type source`+`file` (source-defined,
   **file-absolute** lines via the line-base — C's literal line table). An
   `eval` body inherits the enclosing kind/file with a file-absolute base; an
   `uplevel` body is `type eval`, no file, body-relative, names the invoking
   proc, and omits the `level` key (its scope is redirected — C's
   var-chain-reachability rule). `info frame` (depth) and `info frame N` (N>0
   absolute, N≤0 relative) return the `type line [file] cmd [proc] [level]`
   dict. **Byte-verified vs tclsh 9.0** across root / proc / nested-proc /
   eval-body / uplevel / sourced-file / source-defined-proc frames (the dev
   `run_script` example now `source`s a file argument, like `tclsh`, so the
   differential is exact). `info level` landed earlier (PC-2/PC-3).
   - **Approximation:** an `eval`-body's `type` can differ from tclsh when its
     bytecode compiler inlines the literal body (sometimes `proc` vs `eval`) —
     the inherit rule matches the sourced-file suite but not every inlined
     `eval {literal}`. This is the same bytecode-boundary class as the
     `foreach`/`expr` errorInfo notes.
   - **`info errorstack` (TIP 348) — out of scope.** Its `INNER {…}` element is
     the **bytecode** execution context (tclvm opcodes — `returnImm`, `loadStk`,
     …), present even at top level since modern Tcl compiles everything. A
     runtime that emits WASM, not tclvm bytecode, cannot reproduce it — the same
     class as the suite's bytecode/disassembly exclusions. It degrades to a
     clean `unknown subcommand` error.
6. **PC-6 — AOT emit path** carries the same bookkeeping (conservative), and the
   AOT compiler keeps a **real frame + interpreter fallback** wherever any
   PC-3 dynamic construct is reachable; the AOT↔interp interop gate (a compiled
   proc and an interpreted proc in one chain produce identical `errorInfo` and
   `info frame`/`info level`).
7. **(end of project) — the elision pass** (§7): drop
   reified frames / diagnostic bookkeeping **only** where escape analysis proves
   no dynamic construct can observe them. Gated by proofs + the suite.

## Cross-references

- [`c-extension-abi.md`](c-extension-abi.md) §4.6 — extension-command dispatch
  (the `External` call path).
- [`namespace-tree.md`](namespace-tree.md) — namespace resolution (T1.5), the
  `nsPtr` field.
- [`../compiler/var-escape-analysis.md`](../compiler/var-escape-analysis.md) —
  the proof contract that gates frame elision.

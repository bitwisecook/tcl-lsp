# The C Tcl extension shim

> **Status:** first leg landed (the argument-handling core); the WASM leg is
> design only. Issue #1372, part of the spec-pack DSL design
> ([spec-packs.md](spec-packs.md) § "Covering the hooks").

The Tcl extension interface (`tcl-engine-api`) was designed for exactly two
consumers: the Rust hook host, and a shim that lets a **C Tcl extension** run
behind the same surface. This document is that shim: crate `rust/tcl-cshim`,
its C header `include/tclshim.h`, and the rules that keep it a shim rather
than a second interface.

```text
  C extension            compiled against include/tclshim.h
 ------------------------ rust/tcl-cshim ------------------------
  ffi.rs      the exported Tcl_* symbols, each panic-guarded
  obj.rs      Tcl_Obj: refcounted, dual-rep, typed across the boundary
  state.rs    Tcl_Interp: result slot, error code, command table, packages
  Interp<E>   owns an engine; publishes C commands as HostCommands
 ------------------------ tcl-engine-api ------------------------
  tcl-vm engine (tcl-engine-tclvm)   |   Tcl->WASM codegen engine (later)
```

The three interface rules from the spec-pack design hold here by
construction. The interface stays **common to the backends**: the shim is
`Interp<E: Engine>` and names no engine. **All C-required mangling lives in
the shim**: string lifetimes, interp pointers, result codes, variadics, and
the `int`-versus-`ptrdiff_t` size type are absorbed in `ffi.rs` and the
header, and the interface gained nothing C-shaped. And the shim is the
interface's **second consumer, not a bypass**: a C command reaches the engine
through `Engine::define_command` like an emitter verb does.

## Trust model

**A shimmed extension is trusted native code.** It runs with the host
process's authority, loaded only by the host process's own configuration —
Rust code calling `Interp::load_static`, which is an `unsafe fn` precisely
because calling it is the act of trusting native code.

The rest of the model follows from what packs and hooks are:

- **A `.tclspec` cannot reference one.** There is no word in the `SpecTcl`
  vocabulary for loading native code, and none is planned. A SpecTcl 2.0 pack
  is a full Tcl script evaluated in the sandboxed `tcl-vm`
  (`tcl-spec-hooks/src/pack_eval.rs`); `load`, `source`, and `exec` are not
  among the commands that sandbox has, so a pack that spells `load` reaches
  the unknown-word handler and fails its evaluation. A hook body runs under
  the closed whitelist in `tcl-spec-hooks/src/sandbox.rs`, which has no
  `load` either.
- **A shimmed command is invisible from the sandbox.** Commands are
  registered only into host-owned interpreters by host code — each
  `Interp<E>` owns its own engine, and a pack's or hook's engine is a
  different, fresh one. `rust/tcl-cshim/tests/sandbox_isolation.rs` proves
  both halves negatively: a pack program and a hook body that try `load`, and
  that try to call a command the shim registered into a separate host
  interpreter, are refused, while the host interpreter answers.
- **`-native ID` is the only sanctioned way engine-native code reaches a
  pack** (registry redesign § 6.3): a shipped pack names a native hook by a
  stable identifier, and the identifier resolves to Rust the host already
  contains. A shimmed C command is *not* a `-native` hook. The two ideas are
  kept separate: `-native` binds a hook family's calling convention to
  compiled-in Rust; the shim binds a Tcl command name to C code that answers
  with a result. Unifying them — a `-native` identifier that resolves to a
  shimmed C command — would need an engine-neutral reason and a trust story
  for the identifier table; it is noted as possible future work only, and
  not proposed.

**Containment stops where fuel stops.** A hook body is bytecode with a
command budget, a wall-clock cap, and a value-size cap. C code has none of
those: the shim cannot stop a C loop or bound a C allocation. What it does
contain:

- **Rust panics at the boundary.** Every exported function runs under
  `catch_unwind`; a panic (a NULL `Tcl_Obj`, a defect in the shim itself) is
  converted to a benign fallback return and parked, and the invocation is
  reported as `EngineError::Crashed` once the C procedure returns. An
  `extern "C"` function that unwinds aborts the process, so this is not
  optional.
- **Rust panics around the call.** `load_static` and each command
  invocation are themselves under `catch_unwind`.

What it does not contain, stated plainly: **undefined behaviour in the C
code** — a wild pointer, a use-after-free of a `Tcl_Obj`, a buffer overrun.
No unwinding boundary catches that, and a build with `panic = "abort"` has
no unwinding at all. This is the same posture C Tcl itself has for a loaded
extension, and it is why the shim is a host-configuration facility and not
a pack facility.

## Value marshalling

`Tcl_Obj` is `obj::Obj`: a reference count, an optional string
representation, and an internal representation, exactly as C Tcl's dual-rep
object. The rule that decides what crosses the interface is one flag:

| the object was | rep | string | crosses as |
|---|---|---|---|
| built by C from a number (`Tcl_NewIntObj`, `Tcl_NewDoubleObj`) | `Int` / `Double` | none, or rendered from the rep | `Value::Int` / `Value::Double` |
| built by C as a list (`Tcl_NewListObj`, appends) | `List` | none, or rendered | `Value::List`, recursively |
| born as text and parsed (`Tcl_GetIntFromObj` on `"0x10"`) | `Int` (cached) | authoritative | `Value::Str("0x10")` |
| born as text, never parsed | none | authoritative | `Value::Str` |

So an integer or list the C code *built* never takes a detour through text,
and a value the C code merely *read* keeps its spelling — `0x10` stays
`0x10`, and `a  b` with two spaces stays that way even after
`Tcl_ListObjGetElements` parsed it. Inbound, `Value::Int` becomes an `Int`
rep with no string, `Value::List` a list of objects, `Value::Dict` the flat
key/value list a Tcl dict is. Text is Tcl's modified UTF-8 (an interior NUL
is `C0 80`) so a string rep is always a valid C string, and every C string
the shim reads — a result, an error-code element, a `Tcl_UtfNcmp` operand —
is decoded the same way; the interface's strings are ordinary Rust text.

The string rep is generated lazily by `Tcl_GetString` and cached; the
pointer it returns is valid until the object is mutated or freed, the same
contract C Tcl gives. `Tcl_ListObjGetElements` returns a pointer into the
list's own element array (`ObjRef` is `repr(transparent)` over the object
pointer, so a `Vec<ObjRef>` *is* the `Tcl_Obj **`), valid under the same
rule.

**Reference counts map onto Rust ownership.** `ObjRef` is one unit of the
count: cloning it is `Tcl_IncrRefCount`, dropping it is `Tcl_DecrRefCount`,
and the object is freed when the count reaches zero — including from zero,
as C Tcl does, because a freshly created object has count zero and belongs
to whoever first takes a reference. `Tcl_SetObjResult` and
`Tcl_ListObjAppendElement` take their own reference; `Tcl_DuplicateObj`
returns a fresh, unshared, zero-count copy. Arguments arrive at a C
procedure with a count of one held by the shim for the duration of the
call: C code that duplicates-if-shared simply mutates its private copy,
which is harmless, and nothing the engine holds is ever reachable through
`objv`.

**Conversions are the shared owners' conversions.** Integers and doubles go
through `tcl-syntax::number` (`parse_whole_with` with the integer-only flag
for `Tcl_GetWideIntFromObj`, `format_double` for `Tcl_PrintDouble`'s output),
lists through `tcl-syntax::list` (`split_list`, `join_list`, and
`list_element` for `Tcl_WrongNumArgs`'s per-word quoting), booleans through
`tcl-syntax::boolean::parse_boolean_strict` plus the number grammar (C's
`Tcl_GetBooleanFromObj` accepts any number, non-zero being true), and option
tables through `tcl-cmd-core::prefix` (`scan` for the unique-prefix rule,
`bad_key_message` for the `bad …: must be …` / `ambiguous …` wording). The
error codes are C Tcl's: `TCL VALUE NUMBER`, `ARITH IOVERFLOW {…}`,
`TCL VALUE LIST BRACE|QUOTE|JUNK`, `TCL VALUE DOUBLE NAN`,
`TCL LOOKUP INDEX <msg> <key>`, `TCL WRONGARGS`.

`Tcl_GetIntFromObj` follows Tcl 9: a wide within the *unsigned* 32-bit range
is truncated two's-complement (`2147483648` reads as `-2147483648`), outside
it is the overflow error. `Tcl_GetLongFromObj` writes the width `long` has on
the target.

## Registration

`Interp<E: Engine>` owns an engine and an `Rc<InterpState>`; the
`Tcl_Interp *` C code sees is the pointer to that state, and the state is
what the C API reads and writes through it: the result slot, the error
code, the command table, the provided packages. The engine is never behind
the pointer — the C side has no way to reach it, and no API here evaluates a
script from C.

`Interp::load_static(init)` calls `<Pkg>_Init(Tcl_Interp *)`. During the
call, `Tcl_CreateObjCommand` records a `CommandEntry` (name, procedure,
client data, delete procedure) in the state and queues a `Created` change;
`Tcl_PkgProvideEx` records `(name, version)`. On `TCL_OK` the queued changes
are applied to the engine by `Interp::sync`: each created command becomes a
`ShimCommand` — a `HostCommand` holding the state and the name — registered
through `Engine::define_command`, the same door the hook host's emitter
verbs use. `Loaded` reports the commands and packages; a non-`TCL_OK` return
is `LoadError::InitFailed` carrying the result the init left.

On invocation the engine hands `ShimCommand` the call's words as `Value`s.
It builds `objv` (the command name first), resets the result and error
code, calls the C procedure under `catch_unwind`, and maps the return code:
`TCL_OK` (and `TCL_RETURN`) to the result's `Value`, `TCL_ERROR` to
`EngineError::Script { message, code }` with the result text and the error
code the C code set, `TCL_BREAK` / `TCL_CONTINUE` to the "invoked outside of
a loop" errors Tcl reports at a non-loop level — the interface carries
results and errors, not loop completion codes, and adding them would be a
Tcl-shaped wart on a value interface.

Command-table changes made *during* an invocation — a factory command
calling `Tcl_CreateObjCommand`, or `Tcl_DeleteCommand` on a sibling — are
queued in the state and published through the engine's **registration
door** the moment the C procedure returns: `ShimCommand` implements
`HostCommand::invoke_with_registrar`, and the `CommandRegistrar` the engine
passes is live for that call, so `factory x; x` works within one script.
An engine that does not open the door (the trait method has a default) is
still correct, only later: `Interp::eval` (compile a parameterless unit,
invoke, `sync`) applies what is left afterwards, and a host driving the
engine directly calls `sync` itself. `Tcl_DeleteCommand` runs the delete
procedure immediately and its `Deleted` change reaches the engine as
`remove_command`; deleting the very command that is executing is safe
because the engine holds its own reference for the call. Dropping the
`InterpState` runs every remaining delete procedure, as deleting a C Tcl
interpreter does.

### What the interface needed

Three changes, all engine-neutral:

- **`Engine::remove_command(name) -> Result<bool, EngineError>`** — the
  other half of `define_command`. Default implementation declines with
  `Unsupported`, so an engine that cannot unregister says so rather than
  leaving a command callable; the tclvm engine implements it with
  `Vm::remove_command`.
- **`CommandRegistrar` and `HostCommand::invoke_with_registrar`** — the
  registration half of the engine, opened to a host command for the
  duration of its invocation (exactly `define_command` and
  `remove_command`, nothing that reaches the interpreter). Defaulted, so
  every existing host command is unchanged; the tclvm engine implements it
  over the `&mut Vm` its native-command seam already hands over. What it
  buys is factories: a command that creates commands, which C extensions
  do routinely and the hook host's emitter verbs never did.
- **The tclvm engine now passes a host command's `Script { message, code }`
  error through verbatim**, with the `-errorcode` in the completion options,
  instead of rendering it as `error: <message>`. A `catch` in Tcl therefore
  sees exactly what the C code set, which is what byte-for-byte fidelity
  requires — and what the hook host's own emitter verbs should always have
  produced.

Nothing else: no interp pointer, no result slot, no completion codes.

## The subset, and the order for the rest

Every declaration in `include/tclshim.h` is implemented — the header is
honest by rule. What is in it is the argument-handling core the spec-author
skill's evidence patterns name, plus what Tcl's own `dltest/pkga.c` and
`pkgb.c` need to compile:

| group | functions |
|---|---|
| registration | `Tcl_CreateObjCommand`, `Tcl_DeleteCommand`, `Tcl_PkgProvide` / `Tcl_PkgProvideEx`, `Tcl_InitStubs` (a no-op macro yielding `TCL_PATCH_LEVEL`) |
| objects | `Tcl_NewStringObj`, `Tcl_NewIntObj` / `Tcl_NewLongObj` / `Tcl_NewWideIntObj`, `Tcl_NewBooleanObj`, `Tcl_NewDoubleObj`, `Tcl_NewListObj`, `Tcl_IncrRefCount` / `Tcl_DecrRefCount` / `Tcl_IsShared` / `Tcl_DuplicateObj` |
| reading | `Tcl_GetString`, `Tcl_GetStringFromObj`, `Tcl_GetIntFromObj` / `Tcl_GetLongFromObj` / `Tcl_GetWideIntFromObj`, `Tcl_GetBooleanFromObj`, `Tcl_GetDoubleFromObj`, `Tcl_GetIndexFromObj` / `Tcl_GetIndexFromObjStruct` |
| lists | `Tcl_ListObjAppendElement`, `Tcl_ListObjGetElements`, `Tcl_ListObjLength` |
| result | `Tcl_SetObjResult`, `Tcl_GetObjResult`, `Tcl_ResetResult`, `Tcl_SetResult`, `Tcl_AppendResult`, `Tcl_WrongNumArgs`, `Tcl_SetErrorCode`, `Tcl_SetObjErrorCode` |
| UTF-8 | `Tcl_NumUtfChars`, `Tcl_UtfNcmp` |
| definitions | `Tcl_Interp`, `Tcl_Obj` (both opaque), `Tcl_Command`, `Tcl_ObjCmdProc`, `Tcl_CmdDeleteProc`, `Tcl_FreeProc`, `ClientData`, `Tcl_WideInt`, `Tcl_Size` / `TCL_SIZE_MAX` / `TCL_INDEX_NONE`, the `TCL_OK` … `TCL_CONTINUE` codes, `TCL_STATIC` / `TCL_VOLATILE` / `TCL_DYNAMIC`, `TCL_EXACT` / `TCL_NULL_OK` / `TCL_INDEX_TEMP_TABLE` |

Three header conventions carry the C-side mangling:

- **Variadics are inline C.** Stable Rust cannot define a C variadic, so
  `Tcl_AppendResult`, `Tcl_SetErrorCode`, and `Tcl_SetResult` are
  `static inline` functions in the header that fan out into fixed-arity
  exports (`TclShim_AppendResultString`, `TclShim_SetResultString`, and the
  ordinary `Tcl_SetObjErrorCode`). `Tcl_SetResult` resolves the freeing
  convention there too: the string is always copied, `TCL_DYNAMIC` is freed
  with the C allocator, any other procedure is called.
- **`Tcl_Size` follows the source's Tcl major.** The exports use the Tcl 9
  ABI (`ptrdiff_t`). `TCL_SHIM_TCL_MAJOR=8` gives an 8.x source `int` for
  `Tcl_Size` and inline wrappers for the three functions that write a size
  through a pointer — the same device Tcl 9's header uses for its own
  compatibility mode.
- **`Tcl_Obj` is opaque.** An extension that reaches into `objPtr->bytes`
  or `objPtr->refCount` directly does not compile against the shim; that
  is the one source change the header can demand, and the compiler reports
  it.

The order for the rest, driven by what real extensions use next: string
building (`Tcl_AppendToObj`, `Tcl_AppendStringsToObj`, `Tcl_ObjPrintf`,
`Tcl_NewByteArrayObj`); the dict API; variables (`Tcl_SetVar2Ex`,
`Tcl_GetVar2Ex`, `Tcl_ObjSetVar2`), which the interface would need a
variable door for; then evaluation (`Tcl_EvalObjEx`, `Tcl_EvalEx`), which
needs the engine reachable *during* an invocation and is therefore an
interface question before it is a shim one. Each step extends the header
only with what it implements.

## Testing

`rust/tcl-cshim/tests/c/pkga.c` is a real C extension shaped like Tcl's own
`dltest/pkga.c` — `pkga_eq` and `pkga_quote` verbatim in behaviour — plus
`pkga_calc`, which dispatches with `Tcl_GetIndexFromObj` over subcommands
that exercise every value function, and a clientData-carrying counter with
a delete procedure. `build.rs` compiles it with the `cc` crate against the
shim header on non-Windows targets and sets the `cshim_c_tests` cfg;
`tests/pkga_e2e.rs` loads it into a tclvm-backed `Interp` and drives it
from Tcl.

Every expected string in that test was captured by compiling the **same
`pkga.c` against Tcl 9.0.4's own `tcl.h`**, loading it into `tclsh9.0`, and
recording the result and `$errorCode` of each call — results, error
messages, error codes, number formatting, list quoting, and prefix
resolution are asserted byte-for-byte against C Tcl, not against the
documentation.

The registration and marshalling story is also tested with an extension
defined in Rust through the same exports (`src/lib.rs` tests, and the
trust-posture proof in `tests/sandbox_isolation.rs`), so every platform,
Windows included, runs it. The smoke tier has one test in each file.

## The WASM leg (design only — not built)

The Tcl→WASM codegen engine will host the same C code the same way: the
C source is compiled to `wasm32-wasi` with wasi-sdk (the shim's header
already passes `clang --target=wasm32-wasi -fsyntax-only` over `pkga.c`),
and the shim's exports become the module's **imports** — the module calls
`Tcl_CreateObjCommand` and friends as host functions the Rust shim
provides, with `Tcl_Obj *` and `Tcl_Interp *` as handles into the shim's
tables rather than raw addresses, and `objv` as a handle array copied into
linear memory for the call. `Obj`, `InterpState`, and `ShimCommand` are
unchanged; only `ffi.rs`'s signatures gain a memory-and-handle adapter.
That leg also gives the shim what native cannot: fuel and memory limits on
the C code, from the WASM engine, which would let a WASM-hosted extension
sit under a budget. None of this is built, and this document does not
promise its shape beyond the sentence above.

## Out of scope

Not shimmed, and not planned as part of this leg: `Tcl_Channel` and the I/O
API; the event loop and notifier (`Tcl_DoOneEvent`, `Tcl_CreateFileHandler`,
timers); threads (`Tcl_CreateThread`, mutexes, thread-specific data);
`Tcl_Eval*` (an interface question first, see above); and **stubs-table
binary compatibility** with real `libtcl` builds — the shim is linked, not
loaded against a stub table, so an extension is recompiled against
`tclshim.h`, never dropped in as an existing `.so`/`.dll`.

## Files

- `rust/tcl-cshim/include/tclshim.h` — the header.
- `rust/tcl-cshim/src/{ffi,obj,state,lib}.rs` — the shim.
- `rust/tcl-cshim/tests/c/pkga.c`, `tests/pkga_e2e.rs`,
  `tests/sandbox_isolation.rs` — the tests.
- `rust/tcl-engine-api/src/lib.rs` — `Engine::remove_command`.
- `rust/tcl-engine-tclvm/src/lib.rs` — the error mapping and
  `remove_command`.
- KCS: [What is the C extension shim and when should I use it?](../kcs/kcs-qa-what-is-the-c-extension-shim.md).

# The C Tcl extension shim

The Tcl extension interface (`tcl-engine-api`) has exactly two consumers: the
Rust hook host, and a shim that lets a **C Tcl extension** run behind the same
surface. This document is that shim: crate `rust/tcl-cshim`, the native leg of
the C header `runtime/rust/include/tcl.h`, and the rules that keep it a shim
rather than a second interface. It is part of the spec-pack DSL design
([spec-packs.md](../registry/spec-packs.md) § "Covering the hooks").

```text
  C extension            compiled against runtime/rust/include/tcl.h
 ------------------------ rust/tcl-cshim ------------------------
  ffi.rs      the exported Tcl_* symbols, each panic-guarded
  obj.rs      Tcl_Obj: refcounted, dual-rep, typed across the boundary
  state.rs    Tcl_Interp: result slot, error code, command table, packages
  Interp<E>   owns an engine; publishes C commands as HostCommands
 ------------------------ tcl-engine-api ------------------------
  tcl-vm engine (tcl-engine-tclvm)
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
because calling it is the act of trusting native code. A host may also let
its own scripts `load` an extension, over a table of entry points it has linked
in and vouched for ([The host's `load`](#the-hosts-load)); building that table
is `unsafe` for the same reason, and registering the command is the host's
choice and nothing a script can make.

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
  for the identifier table, and neither exists.

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
object, behind the layout the header declares (see the header conventions
below). The rule that decides what crosses the interface is one flag:

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
count, in the `refCount` field C's macros read and write: cloning it is what
`Tcl_IncrRefCount` does, dropping it is `Tcl_DecrRefCount`, and the object is
freed when the count reaches zero — including from zero, as C Tcl does, because
a freshly created object has count zero and belongs to whoever first takes a
reference. C's own `Tcl_DecrRefCount` is the header's macro: it lowers the same
field and calls the exported `TclFreeObj` when the count was one or less. `Tcl_SetObjResult` and
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
the pointer: the C side reaches it only through the door an invocation opens
(§ *The doors*).

`Interp::load_static(init)` calls `<Pkg>_Init(Tcl_Interp *)`. During the
call, `Tcl_CreateObjCommand` records a `CommandEntry` (name, procedure,
client data, delete procedure) in the state and queues a `Created` change;
`Tcl_PkgProvideEx` records `(name, version)` and, with the engine's door open,
provides it to the engine's package database too (§ *The doors*). On `TCL_OK` the queued changes
are applied to the engine by `Interp::sync`: each created command becomes a
`ShimCommand` — a `HostCommand` holding the state and the name — registered
through `Engine::define_command`, the same door the hook host's emitter
verbs use. `Loaded` reports the commands and packages; a non-`TCL_OK` return
is `LoadError::InitFailed` carrying the result the init left.

On invocation the engine hands `ShimCommand` the call's words as `Value`s.
It builds `objv` (the command name first), resets the result and error
code, calls the C procedure under `catch_unwind`, and maps the return code:
`TCL_OK`, `TCL_RETURN`, `TCL_BREAK` and `TCL_CONTINUE` to the `HostOutcome` they
are, the result's `Value` and a `CompletionCode`, which the engine carries as
the code Tcl does (the calling procedure returns, the enclosing loop ends or
goes on, and where there is no loop the engine reports what Tcl reports,
`invoked "break" outside of a loop`); `TCL_ERROR` to `EngineError::Script {
message, code }` with the result text and the error code the C code set; and
any other value to a code of the command's own (`CompletionCode::Other`), which
reaches the `catch` that reports it. A `TCL_RETURN` carries the options of the
`return` the last `Tcl_EvalObjEx` ended in (§ *The doors*), so the procedure that
called the command ends as C Tcl's does; one the command answers with no
evaluation behind it is a plain return.

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

### What the interface gives the shim

Engine-neutral pieces, and nothing that is an interp pointer or a result slot:

- **`Engine::remove_command(name) -> Result<bool, EngineError>`** — the
  other half of `define_command`. The default implementation declines with
  `Unsupported`, so an engine that cannot unregister says so rather than
  leaving a command callable; the tclvm engine implements it with
  `Vm::remove_command`.
- **`HostOutcome` and `CompletionCode`** — what a host command answers: its
  value, and `Ok`, `Return`, `Break`, `Continue` or `Other(n)`, the codes a C
  command's `TCL_OK`, `TCL_RETURN`, `TCL_BREAK`, `TCL_CONTINUE` and any other
  integer are, and for a `Return` the options of the `return` behind it (`-code`,
  `-level`, `-errorcode`, `-errorinfo`), as a Tcl dictionary. An error is the `Err`
  of the call. The tclvm engine answers each as the VM's own code, so a `Break`
  ends the loop the command is in, and hands the VM a `Return`'s options as the
  completion's, which the calling procedure's boundary reads.
- **`CommandRegistrar` and `HostCommand::invoke_with_registrar`** — the door of
  the engine, opened to a host command for the duration of its invocation:
  `define_command` and `remove_command`, which buy factories (a command that
  creates commands, which C extensions do routinely); `provide_package`, which
  does what `package provide` does, so a later `package require` is satisfied,
  and `library_loaded`, which `info loaded` lists; `variable`, `set_variable`
  and `unset_variable`, which do what `set` and `unset` do in the frame that
  called the command, an array element spelt `a(k)`, traces and Tcl's own errors
  included; and `eval_in_invocation`, which runs a script in that frame and
  answers the `HostOutcome` it completed with. Defaulted on `HostCommand`, so an
  ordinary host command is unaffected, and each door but the first two declines
  with `Unsupported` in an engine that has none; each has an `Engine` twin for a
  host that drives the engine itself, outside any invocation, where the frame is
  the global one. The tclvm engine implements the door over the `&mut Vm` its
  native-command seam hands over, and the twins over its own VM.
- **Verbatim host-command errors.** The tclvm engine passes a host command's
  `Script { message, code }` through with the `-errorcode` in the completion
  options, so a `catch` in Tcl sees exactly what the C code set, and reports a
  failed unit's `-errorcode` as the `code` of the error. A budget the host
  command's own evaluation outran stays the budget the invocation reports.

## The doors

`Tcl_GetVar2Ex`, `Tcl_ObjSetVar2`, `Tcl_UnsetVar2` and `Tcl_EvalObjEx` act on the
frame that called the running command (`rust/tcl-cshim/src/doors.rs`). The
engine is not behind the `Tcl_Interp *`, so the shim reaches it the one way it
can, through the engine's door: `ShimCommand::invoke_with_registrar` holds the
registrar it was given for the length of the C procedure, and `run_init` does the
same for an entry point, so an `Init` reads and writes the frame that loaded it.
The state keeps a pointer to the reference the invoking function holds and never
a reference of its own, and a scope closes it: a command that evaluates a script
that calls a command opens a door for the inner one and has the outer one back
when it returns, and while a call through a door runs that door is closed to
everything but the call.

- **Names.** The door takes one name, spelt as `set` spells it. An array element
  given as two parts is composed (`a` and `k` are `a(k)`); a name
  `TCL_GLOBAL_ONLY` asks for is rooted (`x` is `::x`), and the engine's message
  gives the name back as the C code spelt it; a name Tcl reads as an element
  (`a(k)`) beside an index is Tcl's `can't set "a(k)(j)": variable isn't array`
  with `TCL VALUE VARNAME`; and an array whose own name contains a parenthesis,
  which Tcl takes literally and one name cannot say, is refused.
- **Flags.** Only the flags the header defines are honoured: `TCL_GLOBAL_ONLY`
  and `TCL_LEAVE_ERR_MSG` for a variable call, and `TCL_EVAL_DIRECT`, a hint to
  Tcl's compiler that changes nothing here, for an evaluation. A source cannot
  name another, and one passed by number is refused with an error that names the
  bits, so it fails where it asked and does not run as if it had been heard.
  `TCL_NAMESPACE_ONLY`, `TCL_APPEND_VALUE`, `TCL_LIST_ELEMENT` and
  `TCL_EVAL_GLOBAL` are not implemented.
- **Errors.** A miss is the engine's own Tcl error. Its message and `-errorcode`
  go in the interpreter's result when the call asked for it
  (`TCL_LEAVE_ERR_MSG`, which an evaluation always has) and nothing is touched
  when it did not, as in Tcl. A failed evaluation leaves `$errorCode` and
  `$errorInfo` as a caught error would (`Vm::publish_caught_error`), so a C command
  that swallows the failure and goes on leaves the globals a script reads. The
  errors C cannot swallow are a budget the engine enforces and a crash: the call
  fails like any other, the error is kept (`InterpState::set_fatal`), every later
  call fails at once without reaching the engine, and the command that is running
  fails with it whatever it returns, so a C command that catches what a script
  raised cannot run on past the fuel. With no door open (an engine that invokes a
  command without one) each call is a Tcl error and never an empty answer, and a
  door the engine lacks is its own `Unsupported`.
- **Objects.** What `Tcl_GetVar2Ex` answers is a new object, kept until the
  command returns, so the pointer stays good through later calls that change or
  unset the variable. An object passed in (the value of `Tcl_ObjSetVar2`, the
  script of `Tcl_EvalObjEx`) is held as a reference for the call, so one with a
  count of zero is consumed as it is in Tcl: freed when the store fails, and when
  the command returns when it succeeded. `Tcl_ObjSetVar2` answers the object it
  was given, not a copy, and the value reaches the engine typed (an integer
  object is an integer).
- **Packages.** `Tcl_PkgProvideEx` goes through the same door before the shim
  records the package: the engine's package database takes it as `package
  provide` does, so a version in conflict with the one provided is the engine's
  own error (`conflicting versions provided for package "pkga": 2.0, then 1.0`,
  `TCL PACKAGE VERSIONCONFLICT`), left in the result as `Tcl_PkgProvideEx` leaves
  it and returned by the entry point before it registers anything, as C Tcl's
  does. An engine with no such door, or no door open, leaves the package to the
  shim's own record (`Interp::provided_packages`).
- **Return options.** A script a C command evaluates may end in a `return`:
  `Tcl_EvalObjEx` answers `TCL_RETURN`, and the options of that `return` (`-code`,
  `-level`, `-errorcode`, `-errorinfo`) are the interpreter's until the next
  evaluation or `Tcl_ResetResult`, as they are in C Tcl. The shim keeps them
  (`InterpState`), and a command that answers `TCL_RETURN` returns with them, so
  the procedure that called it ends as they say: an error for `-code error`, with
  its `-errorcode`; a break for `-code break`; a return from its caller too for
  `-level 2`. A `TCL_RETURN` the command answers with no evaluation behind it is a
  plain one-level return, and every other code carries none.
- **What it does not report.** The value stored is the value given: a write trace
  that rewrites it is not reported back. The `-errorcode` of a variable error is
  the engine's own, and the VM's is `NONE` for a variable that is not there where
  C Tcl's is `TCL LOOKUP VARNAME x` (`tests/doors_e2e.rs` records each difference
  beside the vectors that agree).

## The host's `load`

The shim is linked, not loaded against a stub table, so there is no shared
library for `load` to open. What a host can offer a script is Tcl 9's model of
a **static library**: an entry point the program carries, named by its prefix
(`Pkga`, whose entry point is `Pkga_Init`), which `load {} Pkga` initialises
into the interpreter. `StaticExtensions` (`rust/tcl-cshim/src/load.rs`) is that
table as a host command.

- **The table and the command are the host's.** `StaticExtensions::new` takes
  a `&'static [(&str, InitProc)]` and is `unsafe`, for the reason
  `load_static` is: putting an entry point in the table is the act of trusting
  native code. `StaticExtensions::bundled` is the shim's own test extension
  (`Pkga`, built by `build.rs`; empty where no C compiler built it), which a
  program carries only if it calls it. The command exists on an engine only
  because the host registered it: `Interp::enable_static_extensions`, which
  also gives it the interpreter's state, so `commands` and `provided_packages`
  show what a script loaded; or `Engine::define_command("load", …)` on an
  engine the host drives itself; or `tcl_engine_tclvm::register_host_command`
  on a bare VM. No pack word, hook body or script registers it, and the hook
  host's engines never have one (`tests/sandbox_isolation.rs`, unchanged). An
  engine that also runs untrusted bodies must not be given one:
  `restrict_commands` keeps what `define_command` registered.
- **It publishes through the door.** A running host command holds the
  engine's registration door and not the engine, so the load runs the entry
  point and publishes what it registered through the door, by the same
  `run_init` that `load_static` publishes with through the engine. Without a
  door (`invoke` rather than `invoke_with_registrar`) it refuses.
- **It answers `load` as Tcl 9 does.** `load ?-global? ?-lazy? ?--? fileName
  ?prefix? ?interp?`: the options resolve by unique prefix with the shared
  `bad option` message, and the two flags mean nothing to a loader of linked
  libraries, so they are accepted and ignored. The file name is a label and is
  never opened: the prefix is the one given, or the one Tcl guesses from the
  file name (its last path element, a leading `lib` and then `tcl9` taken off,
  up to the first character that is not a letter or an underscore, with the
  first letter in capitals and the rest in lower case), so an unchanged
  `package ifneeded pkga 1.0 [list load [file join $dir
  libpkga[info sharedlibextension]] Pkga]` reaches the table. An empty file
  name needs a prefix, the `interp` argument must be empty (a child
  interpreter is refused), and a prefix loads once per interpreter: a second
  `load` of it is the no-op Tcl makes of it, and the entry point does not run
  again. A prefix the table does not hold is `no library with prefix "X" is
  loaded statically` for an empty file name (Tcl's own message for a static
  library) and `couldn't load file "F": no extension with the prefix "X" is
  linked into this program` for a file (the shape of Tcl's `couldn't load
  file "F": …`, with the reason a table has to give). A failing entry point's
  result and error code are the `load`'s own, and the prefix is not marked
  loaded, so a later `load` runs the entry point again.
- **What it tells the engine.** The packages an entry point provides reach the
  engine's package database as it provides them (§ *The doors*), so an unchanged
  `package ifneeded pkga 1.0 [list load [file join $dir libpkga[info
  sharedlibextension]] Pkga]` is satisfied by `package require pkga` once the
  entry point has run, and a second `package require` is satisfied from the
  database and runs nothing. A successful load also tells the engine the library
  (`library_loaded`), once per prefix and under the file `load` was given (empty
  for a static library), which is what `info loaded` lists. An engine with
  neither door is told nothing and the load is unaffected. A failed entry point
  lists nothing; C Tcl lists the library in the process-wide `info loaded` and
  not in `info loaded {}`, and one interpreter has only the second.
- **`tclvm --static-extensions`.** The `tcl-vm-cli` crate's `static-extensions`
  feature links the shim's bundled extension, the way a `tclsh` test build
  links `Tcltest`, and the flag registers `load` over it on the VM; without the
  flag there is no `load`, and a build without the feature refuses the flag
  with a usage error.

## The implemented subset

**One header, two hosts.** The authored, API-compatible `tcl.h`
(`runtime/rust/include/tcl.h`) of [c-extension-abi.md](c-extension-abi.md) is
*the* C hosting contract, and this shim is its native host beside the WASM one,
so one extension source compiles for both legs. The header declares for each
host the functions it implements and nothing else: the shim's exports are the
`TCL_HOST_NATIVE` leg (`build.rs` compiles the test extension with it named), a
declaration the shim does not implement is absent from that leg rather than
opaque or present and failing, which keeps the header honest by rule, and
`Tcl_Obj` carries the ABI's declared layout (§ 4.2) rather than an opaque handle
of the shim's own. The `Tcl_Size` switch, `TCL_MAJOR_VERSION=8`, is the header's.

The subset is the argument-handling core the spec-author skill's evidence
patterns name, plus what Tcl's own `dltest/pkga.c` and `pkgb.c` need to
compile:

| group | functions |
|---|---|
| registration | `Tcl_CreateObjCommand`, `Tcl_DeleteCommand`, `Tcl_PkgProvide` / `Tcl_PkgProvideEx`, `Tcl_InitStubs` (a no-op macro yielding `TCL_PATCH_LEVEL`) |
| objects | `Tcl_NewStringObj`, `Tcl_NewIntObj` / `Tcl_NewLongObj` / `Tcl_NewWideIntObj`, `Tcl_NewBooleanObj`, `Tcl_NewDoubleObj`, `Tcl_NewListObj`, `Tcl_DuplicateObj`, `TclFreeObj`; `Tcl_IncrRefCount` / `Tcl_DecrRefCount` / `Tcl_IsShared` are the header's macros over `refCount` |
| reading | `Tcl_GetString`, `Tcl_GetStringFromObj`, `Tcl_GetIntFromObj` / `Tcl_GetLongFromObj` / `Tcl_GetWideIntFromObj`, `Tcl_GetBooleanFromObj`, `Tcl_GetDoubleFromObj`, `Tcl_GetIndexFromObj` / `Tcl_GetIndexFromObjStruct` |
| lists | `Tcl_ListObjAppendElement`, `Tcl_ListObjGetElements`, `Tcl_ListObjLength` |
| result | `Tcl_SetObjResult`, `Tcl_GetObjResult`, `Tcl_ResetResult`, `Tcl_SetResult`, `Tcl_AppendResult`, `Tcl_WrongNumArgs`, `Tcl_SetErrorCode`, `Tcl_SetObjErrorCode` |
| UTF-8 | `Tcl_NumUtfChars`, `Tcl_UtfNcmp` |
| the caller's frame | `Tcl_GetVar2Ex`, `Tcl_ObjSetVar2`, `Tcl_UnsetVar2`, `Tcl_EvalObjEx`, through the door of the running command |
| definitions | `Tcl_Interp` (opaque), `Tcl_Obj` (the declared layout), `Tcl_Command`, `Tcl_ObjCmdProc`, `Tcl_CmdDeleteProc`, `Tcl_FreeProc`, `ClientData`, `Tcl_WideInt`, `Tcl_Size` / `TCL_SIZE_MAX` / `TCL_INDEX_NONE`, the `TCL_OK` … `TCL_CONTINUE` codes, `TCL_STATIC` / `TCL_VOLATILE` / `TCL_DYNAMIC`, `TCL_EXACT` / `TCL_NULL_OK` / `TCL_INDEX_TEMP_TABLE`, `TCL_GLOBAL_ONLY` / `TCL_LEAVE_ERR_MSG` / `TCL_EVAL_DIRECT` |

Three header conventions carry the C-side mangling:

- **Variadics are inline C.** Stable Rust cannot define a C variadic, so
  `Tcl_AppendResult`, `Tcl_SetErrorCode`, and `Tcl_SetResult` are
  `static inline` functions in the header that fan out into fixed-arity
  exports (`TclShim_AppendResultString`, `TclShim_SetResultString`, and the
  ordinary `Tcl_SetObjErrorCode`). `Tcl_SetResult` resolves the freeing
  convention there too: the string is always copied, `TCL_DYNAMIC` is freed
  with the C allocator, any other procedure is called.
- **`Tcl_Size` follows the source's Tcl major.** The exports and the `Tcl_Obj`
  fields use the Tcl 9 ABI (`ptrdiff_t`, spelt `TclHost_Size`).
  `TCL_MAJOR_VERSION=8` gives an 8.x source `int` for `Tcl_Size` and inline
  wrappers for the three functions that write a size through a pointer — the
  same device Tcl 9's header uses for its own compatibility mode.
- **`Tcl_Obj` carries the declared layout.** The type an extension sees is the
  ABI's § 4.2 layout rather than an opaque handle, so an extension that reaches
  into `objPtr->bytes` or `objPtr->refCount` compiles unchanged, and the
  reference-count macros are the header's own: `Tcl_IncrRefCount` is
  `++refCount`, and `Tcl_DecrRefCount` calls `TclFreeObj` when the count was one
  or less, so one release of a fresh object frees it, as in Tcl.
  `rust/tcl-cshim/src/obj.rs` is the shim's side of it. `Obj` is `#[repr(C)]`
  with the five declared fields first, the union's room unused, and its own
  state after them, which C never sees. `bytes` and `length` follow the string
  rep the shim owns, so they are null and zero while an object has no text
  (`Tcl_GetString` makes one) and a change to the value withdraws them, and
  `typePtr` is always null. `tests/c/layout.c` compiles against the header and
  reports the layout it declares, and `obj.rs`'s tests hold `Obj` to those
  offsets and run the macros against it.

An extension that needs string building (`Tcl_AppendToObj`,
`Tcl_ObjPrintf`, `Tcl_NewByteArrayObj`), the dict API, or a variable or
evaluation call beyond the four above (`Tcl_SetVar2Ex`, `Tcl_GetVar`,
`Tcl_Eval`, `Tcl_EvalEx`) is outside the implemented subset. A declaration
outside the subset is absent from the shim's leg of the header rather than
present and unimplemented.

## Testing

`rust/tcl-cshim/tests/c/pkga.c` is a real C extension shaped like Tcl's own
`dltest/pkga.c` — `pkga_eq` and `pkga_quote` verbatim in behaviour — plus
`pkga_calc`, which dispatches with `Tcl_GetIndexFromObj` over subcommands
that exercise every value function, and a clientData-carrying counter with
a delete procedure. `build.rs` compiles it, and `layout.c` beside it, with the
`cc` crate against the header's native leg on non-Windows targets and sets the
`cshim_c_tests` cfg; `tests/pkga_e2e.rs` loads it into a tclvm-backed `Interp`
and drives it from Tcl.

Every expected string in that test was captured by compiling the **same
`pkga.c` against Tcl 9.0.4's own `tcl.h`**, loading it into `tclsh9.0`, and
recording the result and `$errorCode` of each call — results, error
messages, error codes, number formatting, list quoting, and prefix
resolution are asserted byte-for-byte against C Tcl, not against the
documentation.

`tests/c/doors.c` is a second test extension for the doors: one command for each
call (`doors_get`, `doors_set`, `doors_unset`, `doors_eval`, and the variants that
pass `TCL_GLOBAL_ONLY`, no flags or `TCL_EVAL_DIRECT`), `doors_try`, which answers
the code and result a script gave and so swallows its error, `doors_keep`, which
holds a value past the call that unsets its variable, `doors_eval_twice` and
`doors_eval_reset`, which evaluate a second script or reset the result after the
first, for the options a `return` leaves in the interpreter, and an entry point
that sets a global through the door. `tests/doors_e2e.rs` holds a table of scripts
whose code, result and `-errorcode` were captured the same way, from `doors.c`
built against Tcl 9.0.4's own `tcl.h` and run in `tclsh9.0` at the global level
under `catch`, and a second table of the cases where the engine's `-errorcode` is
not C Tcl's, each with the reason.

The registration and marshalling story is also tested with an extension
defined in Rust through the same exports (`src/lib.rs` and `src/load.rs`
tests, and the trust-posture proof in `tests/sandbox_isolation.rs`), so every
platform, Windows included, runs it. The host's `load` runs the same vectors:
`the_same_vectors_run_through_the_host_load_bridge` loads the extension from a
script and then holds every case above to its bytes, and
`load_through_the_host_bridge_defines_the_commands` holds the commands, the
refusal of a prefix the table lacks, and the single load. Three more hold the
package flow: an unchanged `ifneeded` script that is a plain `load`, required
twice, and a package already provided at another version, which fails the entry
point before it registers anything, each against `tclsh9.0`'s answers for the same
`pkga.c` built against Tcl 9.0.4's own `tcl.h`; and the same `ifneeded` script when
the table refuses the load, which leaves `package require` the load's own
`couldn't load file "…": …`, as in C Tcl. The smoke tier has
one test in each file.

The header is held to the shim from the other side by
`make check-c-extension-wasm` ([c-extension-abi.md](c-extension-abi.md) § 7):
every function the native leg declares is one `src/ffi.rs` exports and every
function it exports is declared, and `pkga.c`, `doors.c` and `layout.c` compile for
`wasm32` against the header, `pkga.c` and `doors.c` against both legs at once
and each refused by the WASM leg alone.

## Out of scope

Not shimmed, and so absent from the shim's leg of the header:
`Tcl_Channel` and the I/O API; the event loop and notifier
(`Tcl_DoOneEvent`, `Tcl_CreateFileHandler`, timers); threads
(`Tcl_CreateThread`, mutexes, thread-specific data); and the `Tcl_Eval*` and
variable calls beyond the four the subset has. What the authored header covers
across both hosts is its own scope list, [c-extension-abi.md](c-extension-abi.md)
§ 7. **Stubs-table binary compatibility** with real `libtcl` builds is out
of scope for both legs — the shim is linked, not loaded against a stub
table, so an extension is recompiled from source, never dropped in as an
existing `.so`/`.dll`.

## Files

- `runtime/rust/include/tcl.h` — the header; the shim is its `TCL_HOST_NATIVE`
  leg.
- `rust/tcl-cshim/src/{ffi,obj,state,doors,lib,load}.rs` — the shim; `doors.rs`
  is the caller's frame, `load.rs` the host's `load`.
- `rust/tcl-cshim/tests/c/pkga.c`, `tests/c/doors.c`, `tests/c/layout.c`,
  `tests/pkga_e2e.rs`, `tests/doors_e2e.rs`, `tests/completion.rs`,
  `tests/factory.rs`, `tests/sandbox_isolation.rs` — the tests.
- `rust/tcl-engine-api/src/lib.rs` — `HostOutcome`, `CompletionCode`, the
  registration door's methods and their `Engine` twins, `Engine::remove_command`.
- `rust/tcl-engine-tclvm/src/lib.rs` — the error mapping, the doors,
  `remove_command` and `register_host_command`.
- `rust/tcl-vm-cli/src/main.rs` — `--static-extensions`.
- KCS: [What is the C extension shim and when should I use it?](../../kcs/kcs-qa-what-is-the-c-extension-shim.md).

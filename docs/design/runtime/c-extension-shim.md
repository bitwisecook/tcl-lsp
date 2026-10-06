# The C Tcl extension shim

The Tcl extension interface (`tcl-engine-api`) has exactly two consumers: the
Rust hook host, and a shim that lets a **C Tcl extension** run behind the same
surface. This document is that shim: crate `rust/tcl-cshim`, its C header
`include/tclshim.h`, and the rules that keep it a shim rather than a second
interface. It is part of the spec-pack DSL design
([spec-packs.md](../registry/spec-packs.md) § "Covering the hooks").

```text
  C extension            compiled against include/tclshim.h
 ------------------------ rust/tcl-cshim ------------------------
  ffi.rs      the exported Tcl_* symbols, each panic-guarded
  obj.rs      Tcl_Obj: refcounted, dual-rep, typed across the boundary
  state.rs    Tcl_Interp: result slot, error code, command table, packages
  Interp<E>   owns an engine; publishes C commands as HostCommands
 ------------------------ tcl-engine-api ------------------------
  tcl-vm engine (tcl-engine-tclvm)
```

The engine interface carries exact values and guest failures. C ABI sizes,
reference counts, pointer lifetimes and panic containment remain in the shim.
Command publication uses an independently owned engine capability: the native
address is prepared before callbacks and committed through the live registrar.
The shim cannot derive an interpreter or namespace incarnation from a display
name or a Unicode conversion.

## Trust model

**A shimmed extension is trusted native code.** It runs with the host
process's authority, loaded only by the host process's own configuration —
Rust code calling `Interp::load_static`, which is an `unsafe fn` precisely
because calling it is the act of trusting native code.

The rest of the model follows from what packs and hooks are:

- **A `.tclspec` cannot reference one.** There is no word in the `SpecTcl`
  vocabulary for loading native code. A SpecTcl 2.0 pack
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
object. Data values and original-object callbacks use separate carriers:

| Actual storage | Interface carrier |
|---|---|
| Native integer, double or list with no resident string | `Value::Int`, `Double` or `List` |
| Exported full scalar cache, including bignum, word-Boolean or Jim coerced integer | `Value::NativeScalar` |
| Exported native typed payload with a rendered or explicitly retained string | `Value::Resident { value, string, storage }` |
| String-only value with raw NUL or non-Unicode bytes | `Value::StringBytes` |
| Pure binary backing with no resident string | `Value::ByteArray` |
| Binary backing with its independently resident string | `Resident` around `ByteArray` |

`StringBytes` borrows no Unicode identity. Raw `00`, modified NUL `c0 80` and
`ff` remain distinct. `Value::Str` is a Unicode convenience carrier whose UTF-8
bytes enter the shim unchanged. An explicit `Obj::from_text` construction uses
the shared C9 unit encoder; it is not a resident-byte decoder.

A pure C byte array acquires its string through `NativeStringProtocol`: each
payload byte becomes a native U00XX unit, so `00` becomes `c0 80` and `ff`
becomes `c3 bf`. Its original backing remains separate while that representation
is current. The VM bridge inspects `resident_string_bytes` without materialising
a pure byte array, and reconstructs compound values through
`with_resident_string_bytes` or `byte_array_with_resident_string`.

The VM command bridge's `MaterializedStrings` view retains full scalar caches
alongside authentic string bytes. `NativeObjectSnapshots` inspects existing
storage without generating a string; it accepts supported scalar, canonical
List and ByteArray payloads and explicitly refuses other primary caches.
Snapshots construct independent receiving objects and grant no mutation or
original-object identity authority.

The C shim requests `OriginalObjects`. Each argument has an owned capability
for its actual VM object, an actual native issuer, and a private engine receipt.
The current shim ABI requires C9.0; another native issuer or logical simulation
cannot authorize its primitive conversions. Getters publish reached string and
primary-cache effects to the same original object, including on failure.
Repeated arguments and cached List/Dictionary members retain their original
aliases. A weak mirror pool preserves retained C object identity and leaves an
unchanged resident-string pointer stable across callbacks.

`OriginalObjectResult` separates independent data from original handles,
compound member handles and native Index cache receipts. Returning an original
requires the receiving engine's private receipt and matching actual interpreter
and issuer; an equality key alone grants no authority. Unsupported
expression, completion and frame primary caches refuse before native code sees
the object. Canonical empty storage is mapped to the shim's actual canonical
empty allocation only with an independently retained storage marker; equal
empty bytes do not supply that marker.

A callback-only `NativeStringCache` retains the native count, Unicode units,
descriptor origin and separate resident storage. The checked receiving factory
preserves those fields without invoking a getter. Data snapshots that cannot
carry this descriptor refuse instead of flattening it into a String value.

`ResidentStringMutation` is issued by the shim's allocation owner. `Preserve`
retains the original allocation while committing a reached cache change;
`Replace` adopts new storage and `Discard` removes it. Equal bytes alone never
establish pointer preservation. The receiving engine validates the disposition
before mutating the original object.

An Index cache owns its original table lifetime, stride and selected index.
Matching-table lookups reuse the cache independently of new `EXACT` flags;
string regeneration rereads the live canonical entry. Temporary tables do not
install a cache. `load_static` authenticates the extension lifetime promise:
code, persistent tables and entry strings outlive all retained or duplicated
cache objects, including after command/interpreter retirement. An arbitrary
pointer is insufficient; dynamic loading needs real library ownership.

`Value::as_bytes` exposes only an existing string. `as_str` and `Obj::text`
provide checked Rust UTF-8 views; they never replace invalid bytes or reinterpret
modified NUL. SpecTcl metadata and declaration handlers require that checked
Unicode view and return an explicit host refusal when it is unavailable.

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
returns a fresh, unshared, zero-count copy. Arguments are retained for the native call. Original capabilities capture source
sharing before adding their own references, and `Tcl_IsShared` combines that
fact with actual shim references. Conversions mutate shared primary caches as
native getters do; mutating List operations check the actual sharing receipt.
A zero-element native List constructor creates canonical empty NULL-type
storage. Importing an existing cached List retains its independent cache flag.

**Primitive conversions use the selected shared protocol.** The shim's object
implementation uses the C9 `NativeScalarGetterProtocol` independently of a
host engine's authored profile and of expression numeral grammar. It inspects
the original cache first, materialises the original string when required, then
applies the full `Number` or `WordBoolean` cache before returning either success
or failure. `NativeScalarGetterError` retains failure origin and an explicit
`Unchanged` or `Set(bytes)` error-code update; `Unchanged` does not rewrite the
private interpreter state. Primitive Boolean extraction is separate from
expression truthiness.

Lists use the shared byte parser and selected C native list-result renderer;
option tables use shared byte prefix selection and error rendering. Guest
messages, error-code lists and retained return options cross as exact bytes in
`EngineError::ScriptBytes`, with `Script` remaining a lossless Unicode-compatible
carrier. Display escaping has no role in native lookup or guest completion.

The C API entry determines byte extent. Registration, deletion and package
names consume C strings. `Tcl_NewStringObj` and `Tcl_GetStringFromObj` retain
explicit lengths. `Tcl_NumUtfChars` distinguishes an explicit length from its
negative-length C-string form. `Tcl_UtfNcmp` consumes the supplied count of
native UTF units and can compare beyond a raw NUL; its guarded reader delegates
to `NativeTclUtf` rather than scanning for a terminator.

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

`Interp::load_static(init)` obtains `CommandPublicationService` from its engine
before entering native code. `Tcl_CreateObjCommand` prepares its address before
any delete callback. `PreparedCommandPublication` retains original reporting
bytes, an interpreter-scoped primary key and opaque engine authority. The key
contains owner, interpreter, actual namespace incarnation and exact simple-name
bytes. The consuming engine validates the opaque receipt; public key fields do
not grant registration authority.

Unqualified C API creation is global. A visibly qualified relative name uses
the actual current namespace, including a terminal-colon namespace whose display
name cannot be reparsed into the same slot. Creation and deletion have separate
native purposes. A prepared address is never reconstructed from original bytes
when queued changes are committed.

The local table keeps the original entry visible during its delete callback.
An outer replacement wins over a callback-created replacement. An outer deletion
removes only the original generation, leaving a callback-created replacement in
place. Queued changes keep only the final disposition of each exact primary slot
before publication through `define_prepared_command` or
`remove_prepared_command`.

The owned preparation capability cannot retain a borrowed executing `Vm` or
`CommandRegistrar`, use thread-local default scope, or reborrow an executing
engine's `RefCell`. Missing scope, unavailable namespace authority and unsupported
synchronous script deletion traces remain typed host refusals. The supported
shim ABI has no `Tcl_Eval` or namespace-mutation entry; table-only preparation
cannot claim authority to execute such callbacks.

Invocation marshals the command name and arguments as exact native objects.
`HostCommand::invoke_original_completion_with_registrar` returns
`Completion<OriginalObjectResult>`. The raw code distinguishes normal, Error,
Return, Break, Continue and custom completions; result and options remain
original objects. The executing procedure or loop boundary interprets abrupt
codes. Panic and host refusal remain outside this guest completion.

`Tcl_GetReturnOptions` retains the supported C9 private state: `-code` and
`-level`, an existing error code on any completion, and Error's default
`NONE`, result-backed `-errorinfo`, line 1 and empty error stack. This header
has no `Tcl_SetReturnOptions` entry; it does not synthesize unsupported custom
interpreter option state. Result-only original callbacks explicitly refuse an abrupt completion. The
data-only callback can retain guest Error as exact `ScriptBytes` with full
options; Return, Break, Continue and custom codes require the completion entry.

`OriginalObjectResult::Shared` retains repeated fresh object nodes across
result, options and compound members. Export and recovery use one memo per
callback graph; cyclic native graphs refuse. A shared data node grants no
original engine authority. Original handles still require the engine-private
receipt and actual interpreter. The native fixtures in
`rust/tcl-cshim/tests/data/native_callback_completions` distinguish direct
callback codes from script-level interpretation and record C9's result/error-info
alias separately from older C releases.

`Loaded` exposes byte command names and byte package keys/versions.
`Tcl_PkgProvideEx` uses its C-string package primary key without a Unicode
normalisation.

### What the interface gives the shim

The engine exposes exact `Value` and `EngineError` carriers, an owned native
publication preparation service, and consuming prepared-address methods on both
`Engine` and `CommandRegistrar`. Byte spelling convenience methods remain
separate from prepared native C publication. Unicode-only engines explicitly
refuse byte names they cannot represent; engines without an authentic native
publication scope refuse the prepared operation.

## The implemented subset

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
| result | `Tcl_SetObjResult`, `Tcl_GetObjResult`, `Tcl_GetReturnOptions`, `Tcl_ResetResult`, `Tcl_SetResult`, `Tcl_AppendResult`, `Tcl_WrongNumArgs`, `Tcl_SetErrorCode`, `Tcl_SetObjErrorCode` |
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

An extension that needs string building (`Tcl_AppendToObj`,
`Tcl_ObjPrintf`, `Tcl_NewByteArrayObj`), the dict API, variables
(`Tcl_SetVar2Ex`, `Tcl_ObjSetVar2` — which the interface has no variable
door for) or evaluation (`Tcl_EvalObjEx`, which needs the engine reachable
*during* an invocation) does not compile against the shim. The header
extends only with what it implements.

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

## Out of scope

Not shimmed: `Tcl_Channel` and the I/O API; the event loop and notifier (`Tcl_DoOneEvent`, `Tcl_CreateFileHandler`,
timers); threads (`Tcl_CreateThread`, mutexes, thread-specific data);
`Tcl_Eval*` (an interface question first, see above); and **stubs-table
binary compatibility** with real `libtcl` builds — the shim is linked, not
loaded against a stub table, so an extension is recompiled against
`tclshim.h`, never dropped in as an existing `.so`/`.dll`.

## Files

- `rust/tcl-cshim/include/tclshim.h` — the header.
- `rust/tcl-cshim/src/{ffi,obj,state,lib}.rs` — the shim.
- `rust/tcl-cshim/tests/c/pkga.c`, `tests/pkga_e2e.rs`, `tests/factory.rs`,
  `tests/sandbox_isolation.rs` — the tests.
- `rust/tcl-engine-api/src/{lib,value}.rs` — exact carriers and native publication capabilities.
- `rust/tcl-engine-tclvm/src/lib.rs` — native value/error, original-object and registration bridges.
- KCS: [What is the C extension shim and when should I use it?](../../kcs/kcs-qa-what-is-the-c-extension-shim.md).

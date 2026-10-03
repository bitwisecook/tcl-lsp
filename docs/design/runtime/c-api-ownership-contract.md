# C-API ownership / error contract

The ownership and error-path categories for the C Tcl API surface an
unmodified extension compiles and links against (`tcl.h` + `tclOO.h` +
`tclTomMath.h` — see [`c-extension-abi.md`](c-extension-abi.md) §4.1, which is
where that surface's design and its current implementation state live).

**Every** public C-API function carries an ownership category (what it does to
each `Tcl_Obj` handle's refcount) and an error-path category (how it signals
and records failure). This document fixes those categories and records one row
per function. It is the C-API sibling of
[`refcount-contract.md`](refcount-contract.md), which covers the *internal*
`tcl_*` / `obj_*` runtime exports.

> **Scope.** The rows below describe the API contract, transcribed from the C
> Tcl documentation and sources. `runtime/rust/src/capi.rs` implements a subset
> of that surface today; a row exists whether or not its function is
> implemented yet, because the contract is what an extension compiles against,
> not what happens to be wired up.

Sources transcribed: `tmp/tcl9.0.4/doc/*.3` (`Object.3`, `StringObj.3`,
`SetResult.3`, `ListObj.3`, `GetInt.3`, `Eval.3`, `CrtObjCmd.3`, `Alloc.3`,
`Hash.3`, `SetVar.3`, `CrtChannel.3`, `FileSystem.3`, `Class.3`, …)
cross-checked against `tmp/tcl9.0.4/generic/{tclObj.c,tclBasic.c,tclExecute.c,
tclCmdIL.c,tclIO.c,tclOO.c}`.

---

## Why the categories have to be exhaustive

Two contracts make an extension *correct*: who owns each handle, and how errors
propagate. One wrong refcount category leaks or double-frees; one wrong error
category swallows a stack trace. Neither is recoverable at the call site,
because the extension author cannot see our implementation — the header plus
this document is the whole of what they have.

### The `fresh_zero` convention — the subtlety that matters most

In the C Tcl API, **a newly created `Tcl_Obj` has refCount 0**, *not* 1
(`StringObj.3`: "a newly-created value whose ref count is zero"). The caller owns
nothing until it either calls `Tcl_IncrRefCount`, or hands the object to a
function that *takes a reference* (`Tcl_SetObjResult`,
`Tcl_ListObjAppendElement`, …). This is why

```c
Tcl_SetObjResult(interp, Tcl_NewStringObj("hi", -1));   /* correct, no leak */
```

is leak-free with no explicit refcount call: the fresh object is created at
rc=0 and `Tcl_SetObjResult` increments it to rc=1, owned by the interp.

**This is the opposite of the internal `obj_new_*` primitives**
(`refcount-contract.md`), which return rc=1 (`owned`). The C-API boundary must
honour the documented rc=0 convention exactly, because extension source is
written to it. We tag every C-API constructor `fresh_zero` to make the
distinction impossible to miss.

#### The reference-count macros and `TclFreeObj`

The header (`runtime/rust/include/tcl.h`) defines `Tcl_IncrRefCount`,
`Tcl_DecrRefCount` and `Tcl_IsShared` as macros over the `refCount` field, as
Tcl's own header does, so an extension compiled against it calls no exported
function for them:

- `Tcl_IncrRefCount(o)` is `++o->refCount`.
- `Tcl_DecrRefCount(o)` is `if (o->refCount-- <= 1) TclFreeObj(o);`. A *single*
  decref of a fresh (rc 0) object therefore frees it, and
  `o = Tcl_NewObj(); …; Tcl_DecrRefCount(o);` is as valid as it is in Tcl.
- `TclFreeObj` is where an extension's decref reaches the host's allocator. The
  runtime and the shim both export it; it frees an object whatever count the
  macro left, so the runtime's `TclObj` and the shim's `Obj` both keep the
  header's `Tcl_Obj` layout (`refCount`, `bytes`, `length`, `typePtr`, then the
  internal representation) for the macros and for `objPtr->bytes`.

The exported `Tcl_IncrRefCount` and `Tcl_DecrRefCount` functions are for a host
that calls through the symbol, not for compiled extension code. The function
`Tcl_DecrRefCount` keeps a leak guard the macro has no place for: at rc 0 it
counts a double free and refuses to free again.

---

## Categories

### `Tcl_Obj *` argument ownership

| Category | Meaning |
|---|---|
| `borrowed` | Caller keeps its reference; the function must not release it. It may retain temporarily for the call's duration but must release before returning. |
| `consumed` | Caller transfers a reference to the function (the function will store or release it). The only consumers in the shipped surface are `Tcl_DecrRefCount` and `TclFreeObj`. |
| `borrowed→stored` | Borrowed for refcount purposes, but the function **retains its own reference** into a structure that outlives the call (interp result, list element, …). The caller's reference is unaffected; combined with `fresh_zero` this is how a freshly created object becomes owned solely by the structure. |
| `n/a` | The function takes no `Tcl_Obj` handle (string/scalar/struct/opaque only). |

### `Tcl_Obj *` return ownership

| Category | Meaning |
|---|---|
| `fresh_zero` | **C-API creation convention**: result has refCount **0**. Caller must `Tcl_IncrRefCount` to keep it, or hand it to a `borrowed→stored` consumer that retains it. Differs from the internal `obj_new_*` (rc=1). |
| `borrowed` | Caller does **not** own the result (e.g. the interp's current result object, a list's element, an object's name). Reading is fine; valid only until the owning structure changes; `Tcl_IncrRefCount` to keep it past that. |
| `owned` | Caller gets a +1 it must release. (Rare on this boundary — most creation is `fresh_zero`.) |

### Non-`Tcl_Obj` returns

`char* borrowed-rep` (pointer into an obj's string rep — invalidated when the
obj is modified/freed) · `char* owned-buffer` (`Tcl_Alloc` — free via
`Tcl_Free`) · `char* borrowed-buffer` (into a `Tcl_DString` / var table —
invalidated by the next mutation) · `status` (`int` TCL_OK/ERROR/RETURN/…) ·
`token` (opaque `Tcl_Command`/`Tcl_Channel`/`Tcl_HashEntry*`/`Tcl_Object`/… —
owned by the subsystem, borrowed by the caller) · `void`.

### Error-path category

| Category | Meaning |
|---|---|
| `sets-result` | On failure sets the interp result message (and may `Tcl_SetErrorCode`); returns a status or NULL/`token`=NULL. |
| `sets-errorInfo` | Eval family: on `TCL_ERROR` sets `errorInfo` (the stack trace), `errorCode`, and the result. |
| `no-error` | Cannot fail / has no status channel (pure reader, `void`, or `Tcl_Obj`-constructor — OOM is reported through the runtime OOM flag, not a return). |
| `nominal-ok` | Always succeeds in our direct-ABI model (`Tcl_InitStubs`, `Tcl_OOInitStubs` — there is no stub table to negotiate; returns the version string). |

A `Tcl_Obj` constructor that hits OOM returns a sentinel and raises the
runtime's OOM flag — recorded on the leak counters (`counters::oom_set`, read
by `counters::oom`), which has no C export of its own today. A constructor has
no per-call error channel, hence `no-error` for constructors.

---

## Subsystems

### Bootstrap / packages

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_InitStubs` | n/a | `char* borrowed-buffer` (version string) | `nominal-ok` | No table to negotiate; returns runtime Tcl-API version (§4.3). |
| `Tcl_PkgProvideEx` | n/a | `status` | `sets-result` | `clientData` opaque. `Tcl_PkgProvide` is a macro over this. |

### Command creation / teardown

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_CreateObjCommand` | n/a | `token` (`Tcl_Command`) | `no-error` | `proc` is a shared-table index (§4.5); `clientData` opaque, freed by `deleteProc`. The runtime answers NULL, binding nothing, for a NULL interpreter, name or `proc`; replacing a name deletes the old command. |
| `Tcl_DeleteCommand` | n/a | `status` | `no-error` | `0` when the command existed, `-1` when not (`rename name {}`). Runs the command's `deleteProc` when its last handle drops: at once for an idle command, and when the call returns for one that deletes itself, so its `clientData` stays live while its own procedure runs. |
| `Tcl_CreateObjCommand2` | n/a | `token` | `no-error` | Tcl 9 `Tcl_Size`-arity variant. |
| `Tcl_CreateObjTrace2` | n/a | `token` (`Tcl_Trace`) | `no-error` | Trace proc is a shared-table index. |
| `Tcl_NRCreateCommand` | n/a | `token` | `no-error` | NRE variant; both `proc`/`nreProc` are table indices. |
| `Tcl_NRCallObjProc` | `objv[]` `borrowed` | `status` | `sets-errorInfo` | Calls an obj proc through the NRE path. |
| `Tcl_DeleteCommandFromToken` | n/a | `status` | `sets-result` | -1 if token already gone. Runs the command's `deleteProc`. |

### Object creation (all `fresh_zero`)

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_NewObj` | n/a | `fresh_zero` | `no-error` | Empty string obj, rc=0. |
| `Tcl_NewStringObj` | n/a | `fresh_zero` | `no-error` | Copies the bytes (length −1 ⇒ `strlen`). |
| `Tcl_NewWideIntObj` | n/a | `fresh_zero` | `no-error` | Tcl 9's own header makes `Tcl_NewIntObj` and `Tcl_NewLongObj` macros over this; the authored header declares them as functions. |
| `Tcl_NewIntObj` | n/a | `fresh_zero` | `no-error` | An integer object from a C `int`. |
| `Tcl_NewLongObj` | n/a | `fresh_zero` | `no-error` | An integer object from a C `long`: 32 bits on `wasm32`, 64 on an LP64 host. |
| `Tcl_NewDoubleObj` | n/a | `fresh_zero` | `no-error` | |
| `Tcl_NewBooleanObj` | n/a | `fresh_zero` | `no-error` | |
| `Tcl_NewListObj` | `objv[]` `borrowed→stored` | `fresh_zero` | `no-error` | Retains each element into the new list. |
| `Tcl_NewBignumObj` | n/a (`value` is `mp_int*` `borrowed`) | `fresh_zero` | `no-error` | Consumes/zeroes the `mp_int` per Tcl 9 semantics — see `IntObj.3`. |
| `Tcl_DuplicateObj` | `objPtr` `borrowed` | `fresh_zero` | `no-error` | Deep-copies value + internal rep. |

### Refcount management

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_IncrRefCount` | `objPtr` `borrowed` | `void` | `no-error` | +1. A macro over `refCount` in the header; this row is the exported function. Internal analogue: `tcl_obj_retain`. |
| `Tcl_DecrRefCount` | `objPtr` `consumed` | `void` | `no-error` | −1; frees at 0. A macro in the header that frees through `TclFreeObj` at a count of one or less; this row is the exported function, which counts a decrement at rc 0 as a double free. Internal analogue: `tcl_obj_release`. **Null-safe** per Tcl macro. |
| `TclFreeObj` | `objPtr` `consumed` | `void` | `no-error` | Frees an object whose count the header's `Tcl_DecrRefCount` macro has lowered to zero or below — a fresh object's single decrement included: the type's free proc, the string rep, the header. **Null-safe.** |

### Object accessors (arg `borrowed`; may shimmer)

These may regenerate an object's internal/string representation (shimmer); they
mutate `internalRep`/`bytes` but **not** the logical value, so a `borrowed`
(even shared) object is safe.

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_GetString` | `objPtr` `borrowed` | `char* borrowed-rep` | `no-error` | Forces the string rep; valid until the obj is modified/freed. |
| `Tcl_GetStringFromObj` | `objPtr` `borrowed` | `char* borrowed-rep` | `no-error` | As above + writes length out. |
| `Tcl_GetIntFromObj` | `objPtr` `borrowed` | `status` | `sets-result` | Shimmers to int; on failure sets `expected integer…`. An integer in the unsigned 32-bit range is truncated, as C Tcl does; past it, `integer value too large to represent` with `ARITH IOVERFLOW` and the message. |
| `Tcl_GetLongFromObj` | `objPtr` `borrowed` | `status` | `sets-result` | As `Tcl_GetIntFromObj` where `long` is 32 bits (`wasm32`); any wide integer where it is 64. |
| `Tcl_GetWideIntFromObj` | `objPtr` `borrowed` | `status` | `sets-result` | |
| `Tcl_GetDoubleFromObj` | `objPtr` `borrowed` | `status` | `sets-result` | |
| `Tcl_GetBooleanFromObj` | `objPtr` `borrowed` | `status` | `sets-result` | |
| `Tcl_GetIndexFromObjStruct` | `objPtr` `borrowed` | `status` | `sets-result` | Resolves the word against a NULL-terminated table by unique prefix (`TCL_EXACT` asks for an exact match); on failure `bad`/`ambiguous … must be …` with `TCL LOOKUP INDEX`. Keeps the matched entry on the word's internal rep, unless `TCL_INDEX_TEMP_TABLE`, so `Tcl_WrongNumArgs` spells an abbreviation in full; the table must outlive the word. `Tcl_GetIndexFromObj` is a macro over this. |
| `Tcl_GetBignumFromObj` | `objPtr` `borrowed` | `status` | `sets-result` | Writes an `mp_int` through `void* value` (caller-owned, caller `mp_clear`s). |
| `Tcl_NumUtfChars` | n/a | `Tcl_Size` | `no-error` | Pure reader over a `char*`. |
| `Tcl_UtfNcmp` | n/a | `int` | `no-error` | Pure reader. |

### Lists

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_ListObjAppendElement` | `listPtr` `borrowed` (mutated), `objPtr` `borrowed→stored` | `status` | `sets-result` | Retains `objPtr` into the list; `listPtr` must be unshared to mutate in place (else shimmers/dup). |
| `Tcl_ListObjGetElements` | `listPtr` `borrowed` | `status` (out: `Tcl_Obj*** objvPtr`) | `sets-result` | Returned array + its element handles are `borrowed` (owned by the list); valid until the list is modified. |
| `Tcl_ListObjLength` | `listPtr` `borrowed` | `status` (out: `Tcl_Size* lengthPtr`) | `sets-result` | Shimmers a string to a list; a string that is not one is the list parser's error. |

### `Tcl_ObjType` registration

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_RegisterObjType` | n/a | `void` | `no-error` | `typePtr` is `borrowed-persistent` — must outlive the process (static). |
| `Tcl_GetObjType` | n/a | `const Tcl_ObjType* borrowed` (or NULL) | `no-error` | |

### Result / error

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_SetObjResult` | `resultObjPtr` `borrowed→stored` | `void` | `no-error` | Interp **retains** the obj (+1) and releases the prior result. A `fresh_zero` obj thereby becomes interp-owned with no explicit refcount call. |
| `Tcl_GetObjResult` | n/a | `Tcl_Obj* borrowed` | `no-error` | The interp's current result; valid until the next result-changing call; `Tcl_IncrRefCount` to keep. |
| `Tcl_ResetResult` | n/a | `void` | `no-error` | An empty result, and no error or pending `return` in flight. |
| `Tcl_WrongNumArgs` | `objv[]` `borrowed` | `void` | `sets-result` | Builds and sets the `wrong # args` message, `-errorcode TCL WRONGARGS`. |
| `Tcl_SetObjErrorCode` | `errorObjPtr` `borrowed→stored` | `void` | `no-error` | The `-errorcode` of the error the command returns. The runtime keeps the code's text, so a `fresh_zero` code object is freed by the call, as storing and dropping it would. `Tcl_SetErrorCode` is an inline function over this in the header. |
| `Tcl_AppendResult` | n/a (varargs `char*`) | `void` | `no-error` | Appends strings to the (string) result; NULL-terminated varargs. An inline function in the header over `TclHost_AppendResultString`. |
| `TclHost_AppendResultString` | n/a | `void` | `no-error` | The fixed-arity export behind the header's inline `Tcl_AppendResult`: one piece. |
| `TclHost_SetResultString` | n/a | `void` | `no-error` | The fixed-arity export behind the header's inline `Tcl_SetResult`: a copy of the string; the inline function resolves the freeing convention. |
| `Tcl_GetErrorLine` | n/a | `int` | `no-error` | Reader. |

### Eval

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_EvalEx` | n/a (`script` `char* borrowed`) | `status` | `sets-errorInfo` | Result via `Tcl_GetObjResult`. |
| `Tcl_EvalObjEx` | `objPtr` `borrowed` (retained for the eval) | `status` | `sets-errorInfo` | `TCL_EVAL_DIRECT`/`GLOBAL` flags. |
| `Tcl_EvalObjv` | `objv[]` `borrowed` | `status` | `sets-errorInfo` | Pre-parsed argv eval. |

### Channels

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_CreateChannel` | n/a | `token` (`Tcl_Channel`) | `no-error` | `typePtr` `borrowed-persistent` (static); `instanceData` opaque, extension-owned. |
| `Tcl_RegisterChannel` | n/a | `void` | `no-error` | |
| `Tcl_GetChannel` | n/a | `token` or NULL | `sets-result` | |
| `Tcl_StackChannel` | n/a | `token` or NULL | `sets-result` | |
| `Tcl_GetChannelInstanceData` | n/a | `void* borrowed` | `no-error` | |
| `Tcl_GetChannelType` | n/a | `const Tcl_ChannelType* borrowed` | `no-error` | |

### Filesystem

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_FSRegister` | n/a | `status` | `sets-result` | `fsPtr` `borrowed-persistent` (static). |
| `Tcl_FSUnregister` | n/a | `status` | `sets-result` | |

### Threading

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_CreateThread` | n/a | `status` | `no-error` | `proc` is a shared-table index; `clientData` opaque. WASM mapping per [`c-extension-abi.md`](c-extension-abi.md) §10. |
| `Tcl_JoinThread` | n/a | `status` | `no-error` | |
| `Tcl_MutexLock` / `Tcl_MutexUnlock` | n/a | `void` | `no-error` | |
| `Tcl_ConditionWait` / `Tcl_ConditionNotify` | n/a | `void` | `no-error` | |
| `Tcl_GetThreadData` | n/a | `void* borrowed` | `no-error` | Thread-local block owned by the thread subsystem; zero-filled on first use. |

### `Tcl_DString`

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_DStringInit` | n/a | `void` | `no-error` | `dsPtr` is caller-owned (often stack). |
| `Tcl_DStringAppend` | n/a | `char* borrowed-buffer` | `no-error` | Into the DString's buffer; invalidated by the next append/free. |
| `Tcl_DStringFree` | n/a | `void` | `no-error` | |

### Allocation

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_Alloc` | n/a | `void* owned-buffer` | `no-error` | The **one** runtime allocator (§4.4); boundary-crossing memory must use this. |
| `Tcl_Free` | n/a | `void` | `no-error` | Returns the buffer to the same allocator. |

### Hash tables

The value slot (`Tcl_GetHashValue`/`Tcl_SetHashValue`, **macros**, not exported
functions) is opaque `clientData`; if an extension stores a `Tcl_Obj*` there it
owns that handle's retain/release — the hash API does not refcount it.

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_InitHashTable` | n/a | `void` | `no-error` | `tablePtr` caller-owned. |
| `Tcl_DeleteHashTable` | n/a | `void` | `no-error` | Frees entries, not opaque values. |
| `Tcl_CreateHashEntry` | n/a | `token` (`Tcl_HashEntry*` `borrowed`) | `no-error` | Owned by the table; `newPtr` out-flags creation. |
| `Tcl_FindHashEntry` | n/a | `token` or NULL | `no-error` | |
| `Tcl_DeleteHashEntry` | n/a | `void` | `no-error` | Does not touch the opaque value. |
| `Tcl_FirstHashEntry` | n/a | `token` or NULL | `no-error` | |
| `Tcl_NextHashEntry` | n/a | `token` or NULL | `no-error` | |

### Variables

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_SetVar2` | n/a | `const char* borrowed-buffer` (or NULL) | `sets-result` | Returns the var's new string value (owned by the var table); string API, no `Tcl_Obj`. |

### TclOO (`tclOO.h`)

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_CopyObjectInstance` | n/a | `token` (`Tcl_Object`) | `sets-result` | |
| `Tcl_GetObjectFromObj` | `objPtr` `borrowed` | `token` or NULL | `sets-result` | Resolves a command name obj to its object. |
| `Tcl_GetObjectName` | n/a | `Tcl_Obj* borrowed` | `no-error` | Owned by the object. |
| `Tcl_GetObjectAsClass` | n/a | `token` (`Tcl_Class` `borrowed`) | `no-error` | |
| `Tcl_GetClassAsObject` | n/a | `token` (`Tcl_Object` `borrowed`) | `no-error` | |
| `Tcl_NewObjectInstance` | `objv[]` `borrowed` | `token` or NULL | `sets-result` | Runs the constructor. |
| `Tcl_ObjectContextInvokeNext` | `objv[]` `borrowed` | `status` | `sets-errorInfo` | `next`-method dispatch. |
| `Tcl_OOInitStubs` | n/a | `char* borrowed-buffer` | `nominal-ok` | Version string; no table to negotiate. |

### TomMath (`tclTomMath.h`)

`mp_*` operate on **caller-owned `mp_int` structs**, not `Tcl_Obj`. They have no
refcount interaction, but their digit storage (`mp_int.dp`) must be allocated
through the runtime allocator so the single-allocator invariant (§4.4) holds
across the boundary.

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `mp_init` | n/a | `mp_err` | `sets-result`=n/a (returns `mp_err`) | Allocates `dp` (runtime allocator). |
| `mp_clear` | n/a | `void` | `no-error` | Frees `dp`. Caller must call to avoid leaking digits. |
| `mp_set` | n/a | `void` | `no-error` | |
| `mp_add` / `mp_sub` / `mp_mul` | n/a | `mp_err` | — | `a`,`b` const (`borrowed`); `c` caller-owned out. |

---

## Enforcement

`make check-c-api-ownership` (`scripts/check_c_api_ownership.py`, reached from
`make xtask-check` and so from `make rust-check`) keeps the rows and the
exported surface in step. Every `#[no_mangle] extern "C"` export in
`runtime/rust/src/capi.rs` named `Tcl_*` or `mp_*` must have a row here; an
export without one is a hard failure. The runtime's own bootstrap and test
scaffolding (`tcl_runtime_*`, `tcl_test_*`) is excluded, as are macros and the
stub-table data symbols — a macro carries no refcount semantics of its own (it
is field access or a thin wrapper, documented under the function it expands
to), and the stub-table data pointers are nominal.

The other direction — a row naming a function the real headers never declared,
or a header function with no row — needs the C headers, so it runs only when
asked: `make check-c-api-ownership TCL_SOURCE=tmp/tcl9.0.4` (equivalently
`--tcl-source`, or `CHECK_C_API_OWNERSHIP_TCL_SOURCE`). Selection is always
explicit rather than "whichever `tmp/tcl*` tree sorts first": pointing it at an
8.x tree compares an 8.4 header surface against this Tcl-9.0-era contract and
reports hundreds of 8.x-only functions as missing rows. The Make target runs
`--self-test` — the checker's own parsing regressions against synthetic
fixtures — before the real pass.

The rows in [`refcount-contract.md`](refcount-contract.md) are still
hand-maintained; extending this checker to them is the remaining half.

The behavioural half is tested independently, for the implemented subset:
`runtime/rust/src/lib.rs`'s `mod tests` drives the canonical round trip
(`Tcl_NewObj` → `Tcl_IncrRefCount` → `Tcl_SetObjResult` → `Tcl_DecrRefCount` →
interp teardown) and asserts zero residual under the leak counters
(`runtime/rust/src/counters.rs`), alongside a `fresh_zero`-is-really-rc-0 case
and a string-object buffer-ownership case. That is what demonstrates the
`fresh_zero` / `borrowed→stored` / `consumed` categories are implemented as
documented — for the dozen functions `capi.rs` exports. Every other row is
transcription only.

## Cross-references

- Internal runtime contract: [`refcount-contract.md`](refcount-contract.md).
- The ABI this surface sits on: [`c-extension-abi.md`](c-extension-abi.md).
- The runtime's refcount discipline:
  [`memory-management.md`](memory-management.md).

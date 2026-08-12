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

Sources transcribed: `tmp/tcl9.0.3/doc/*.3` (`Tcl_Obj.3`, `Tcl_NewObj.3`,
`Tcl_SetObjResult.3`, `Tcl_ListObj.3`, `Tcl_GetInt.3`, `Tcl_Eval.3`,
`Tcl_CreateObjCommand.3`, `Tcl_Alloc.3`, `Tcl_Hash.3`, `Tcl_SetVar.3`,
`Tcl_CreateChannel.3`, `Tcl_FSRegister.3`, `Tcl_Class.3`, …) cross-checked
against `tmp/tcl9.0.3/generic/{tclObj.c,tclBasic.c,tclExecute.c,tclCmdIL.c,
tclIO.c,tclOO.c}`.

---

## Why the categories have to be exhaustive

Two contracts make an extension *correct*: who owns each handle, and how errors
propagate. One wrong refcount category leaks or double-frees; one wrong error
category swallows a stack trace. Neither is recoverable at the call site,
because the extension author cannot see our implementation — the header plus
this document is the whole of what they have.

### The `fresh_zero` convention — the subtlety that matters most

In the C Tcl API, **a newly created `Tcl_Obj` has refCount 0**, *not* 1
(`Tcl_NewObj.3`: "The reference count … is initially 0."). The caller owns
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

#### Open: single-decref of a fresh obj, and the macro / `TclFreeObj` path

Two related unresolved questions, both of which an extension can hit:

1. **`Tcl_DecrRefCount` on an rc-0 `fresh_zero` obj.** The shipped `tcl.h` macro
   is `if ((objPtr)->refCount-- <= 1) TclFreeObj(objPtr);`, so a *single* decref
   of a fresh (rc 0) object **frees it** — and the idiom
   `o = Tcl_NewObj(); …; Tcl_DecrRefCount(o);` appears in real extension code.
   The current Rust `Tcl_DecrRefCount` treats decref-at-rc-0 as a counted
   double-free (the leak-guard) and refuses to free. Decide whether to (a) match
   the macro (free at rc≤1, dropping the guard for this path) or (b) require the
   `fresh_zero` "incr-before-decr" discipline and confirm it against the
   unmodified extension source we intend to run.
2. **`Tcl_DecrRefCount`/`Tcl_IncrRefCount` are macros**, so a C extension
   compiled against `tcl.h` never calls our exported functions — it inlines the
   refcount test and calls `TclFreeObj` directly. `TclFreeObj` is **not** in the
   current export surface; settle how extension-side decrefs reach this
   allocator's free path — export `TclFreeObj`, or ship the refcount ops as
   real functions. Until it is settled, an extension's decrefs do not reach
   this allocator at all.

---

## Categories

### `Tcl_Obj *` argument ownership

| Category | Meaning |
|---|---|
| `borrowed` | Caller keeps its reference; the function must not release it. It may retain temporarily for the call's duration but must release before returning. |
| `consumed` | Caller transfers a reference to the function (the function will store or release it). The only consumer in the shipped surface is `Tcl_DecrRefCount`. |
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
| `Tcl_CreateObjCommand` | n/a | `token` (`Tcl_Command`) | `no-error` | `proc` is a shared-table index (§4.5); `clientData` opaque, freed by `deleteProc`. |
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
| `Tcl_NewWideIntObj` | n/a | `fresh_zero` | `no-error` | `Tcl_NewIntObj` is a macro over this. |
| `Tcl_NewDoubleObj` | n/a | `fresh_zero` | `no-error` | |
| `Tcl_NewBooleanObj` | n/a | `fresh_zero` | `no-error` | |
| `Tcl_NewListObj` | `objv[]` `borrowed→stored` | `fresh_zero` | `no-error` | Retains each element into the new list. |
| `Tcl_NewBignumObj` | n/a (`value` is `mp_int*` `borrowed`) | `fresh_zero` | `no-error` | Consumes/zeroes the `mp_int` per Tcl 9 semantics — see `Tcl_NewBignumObj.3`. |
| `Tcl_DuplicateObj` | `objPtr` `borrowed` | `fresh_zero` | `no-error` | Deep-copies value + internal rep. |

### Refcount management

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_IncrRefCount` | `objPtr` `borrowed` | `void` | `no-error` | +1. Internal analogue: `tcl_obj_retain`. |
| `Tcl_DecrRefCount` | `objPtr` `consumed` | `void` | `no-error` | −1; frees at 0. Internal analogue: `tcl_obj_release`. **Null-safe** per Tcl macro. |

### Object accessors (arg `borrowed`; may shimmer)

These may regenerate an object's internal/string representation (shimmer); they
mutate `internalRep`/`bytes` but **not** the logical value, so a `borrowed`
(even shared) object is safe.

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_GetString` | `objPtr` `borrowed` | `char* borrowed-rep` | `no-error` | Forces the string rep; valid until the obj is modified/freed. |
| `Tcl_GetStringFromObj` | `objPtr` `borrowed` | `char* borrowed-rep` | `no-error` | As above + writes length out. |
| `Tcl_GetIntFromObj` | `objPtr` `borrowed` | `status` | `sets-result` | Shimmers to int; on failure sets `expected integer…`. |
| `Tcl_GetWideIntFromObj` | `objPtr` `borrowed` | `status` | `sets-result` | |
| `Tcl_GetDoubleFromObj` | `objPtr` `borrowed` | `status` | `sets-result` | |
| `Tcl_GetBooleanFromObj` | `objPtr` `borrowed` | `status` | `sets-result` | |
| `Tcl_GetBignumFromObj` | `objPtr` `borrowed` | `status` | `sets-result` | Writes an `mp_int` through `void* value` (caller-owned, caller `mp_clear`s). |
| `Tcl_NumUtfChars` | n/a | `Tcl_Size` | `no-error` | Pure reader over a `char*`. |
| `Tcl_UtfNcmp` | n/a | `int` | `no-error` | Pure reader. |

### Lists

| Function | Obj args | Return | Errors | Notes |
|---|---|---|---|---|
| `Tcl_ListObjAppendElement` | `listPtr` `borrowed` (mutated), `objPtr` `borrowed→stored` | `status` | `sets-result` | Retains `objPtr` into the list; `listPtr` must be unshared to mutate in place (else shimmers/dup). |
| `Tcl_ListObjGetElements` | `listPtr` `borrowed` | `status` (out: `Tcl_Obj*** objvPtr`) | `sets-result` | Returned array + its element handles are `borrowed` (owned by the list); valid until the list is modified. |

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
| `Tcl_WrongNumArgs` | `objv[]` `borrowed` | `void` | `sets-result` | Builds and sets the `wrong # args` message. |
| `Tcl_AppendResult` | n/a (varargs `char*`) | `void` | `no-error` | Appends strings to the (string) result; NULL-terminated varargs. |
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
| `Tcl_CreateThread` | n/a | `status` | `no-error` | `proc` is a shared-table index; `clientData` opaque. WASM mapping per [`c-extension-abi.md`](c-extension-abi.md) §11. |
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

## Known gap: no enforcement

Nothing mechanically checks that every function this surface declares has a row
here, or that a row names a function the surface actually declares. Both
directions matter: a declared function with no row ships an unspecified
ownership contract, and a row naming nothing is a stale claim.

Closing it means a check that collects the declared functions from the header
surface and the row names from this document and fails on either mismatch —
with macros and data symbols deliberately excluded, since a macro carries no
refcount semantics of its own (it is field access or a thin wrapper, documented
under the function it expands to) and the stub-table data pointers are nominal.
The same check should cross-reference `runtime/rust/src/capi.rs`'s
`#[no_mangle] extern "C"` exports, so an export cannot land without an
ownership annotation. [`refcount-contract.md`](refcount-contract.md) has the
identical gap and wants the same tool.

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

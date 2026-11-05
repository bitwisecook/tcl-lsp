# naming.variable.original-local-cache-unavailable-updater

Kind: `native-observation`

## Problem statement

Trace handling or a missing local value can require a diagnostic name after successful cached lookup. Later C local-name headers have no safe updater; a process abort must not be converted to an ordinary guest error or fabricated string.

## Question

What process/guest outcomes follow original local-name Get/Set with trace callbacks or unset after invalidating its string?

## Conclusion

C8.4 completes all three processes: trace Get and Set report guest success with resident names; unset then Get reports guest Error and the exact missing-x message. C8.5/8.6 terminate each process with−6 and updater diagnostics; C9 terminate with−4 and their updater diagnostic. Those fatal windows emit no result row, so they establish process availability failure, not a guest Error value, callback result or post-abort cell identity.

## Scope

Five fresh interpreters per C release, actual ASCII procedure body proc p {} {set x VALUE;probe x};p, private TclObjLookupVar original cell comparison before byte/result observation. Mode0=Get,1=Set,2=read/write trace then Get,3=trace then Set,4=unset then Get. Name string is explicitly invalidated but localVarName remains. Each capture retains separate status/stdout/stderr; no signal window supplies unprinted observer rows. The printed result getter is on interpreter result after cell lookup, not on the name. No Jim/BIG-IP attempt.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Public C evaluator enters original compiled procedure; public ObjGetVar2/ObjSetVar2 and private original-cell lookup surround explicit Tcl_InvalidateStringRep. Version: tcl8.4 8.4.20 (captured release association; launched patchlevel unqueried).

Exact selected independent subprocess outcomes:

```json
[
  {
    "case": 2,
    "exit": 0,
    "stdout": "2\t0\t1\tlocalVarName\t1\t\n",
    "stderr": ""
  },
  {
    "case": 3,
    "exit": 0,
    "stdout": "3\t0\t1\tlocalVarName\t1\t\n",
    "stderr": ""
  },
  {
    "case": 4,
    "exit": 0,
    "stdout": "4\t1\t1\tlocalVarName\t1\t63616e27742072656164202278223a206e6f2073756368207661726961626c65\n",
    "stderr": ""
  }
]
```
Process0 is distinct from the printed guest completion. Fatal processes have no reached guest/result snapshot.

### tcl8.5, tcl8.6

Status: `observed`. Channel: Public C evaluator enters original compiled procedure; public ObjGetVar2/ObjSetVar2 and private original-cell lookup surround explicit Tcl_InvalidateStringRep. Version: tcl8.5 8.5.19 (captured release association; launched patchlevel unqueried); tcl8.6 8.6.18 (captured release association; launched patchlevel unqueried).

Exact selected independent subprocess outcomes:

```json
[
  {
    "case": 2,
    "exit": -6,
    "stdout": "",
    "stderr": "updateStringProc of type localVarName should not be called\n"
  },
  {
    "case": 3,
    "exit": -6,
    "stdout": "",
    "stderr": "updateStringProc of type localVarName should not be called\n"
  },
  {
    "case": 4,
    "exit": -6,
    "stdout": "",
    "stderr": "updateStringProc of type localVarName should not be called\n"
  }
]
```
Process0 is distinct from the printed guest completion. Fatal processes have no reached guest/result snapshot.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Public C evaluator enters original compiled procedure; public ObjGetVar2/ObjSetVar2 and private original-cell lookup surround explicit Tcl_InvalidateStringRep. Version: tcl9.0 9.0.4 (captured release association; launched patchlevel unqueried); tcl9.1 9.1.0 (captured release association; launched patchlevel unqueried).

Exact selected independent subprocess outcomes:

```json
[
  {
    "case": 2,
    "exit": -4,
    "stdout": "",
    "stderr": "UpdateStringProc should not be invoked for type localVarName\n"
  },
  {
    "case": 3,
    "exit": -4,
    "stdout": "",
    "stderr": "UpdateStringProc should not be invoked for type localVarName\n"
  },
  {
    "case": 4,
    "exit": -4,
    "stdout": "",
    "stderr": "UpdateStringProc should not be invoked for type localVarName\n"
  }
]
```
Process0 is distinct from the printed guest completion. Fatal processes have no reached guest/result snapshot.

### jim, bigip

Status: `not-tested`.

No observation of this exact question is retained for this provider.

## Consumer bindings

- [rust/tcl-syntax/src/native_variable_name.rs](../../../../rust/tcl-syntax/src/native_variable_name.rs), `NativeVariableNameProtocol::cache_precedes_string_getter`: Selects cached-name versus byte-getter order without fabricating an unavailable original updater.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::original_local_cache_getter_order_matches_all_25_native_paths` (linked): Compares 25 ordinary/fatal native paths; Rust host refusal is an independent implementation response to the measured fatal native updater, not a native guest completion.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::original_local_cache_getter_order_matches_all_25_native_paths` (linked): Compares 25 ordinary/fatal native paths; Rust host refusal is an independent implementation response to the measured fatal native updater, not a native guest completion.

## Evidence

- [rust/tcl-syntax/tests/data/native_variable_name/cache_lookup/probe.c](../../../../rust/tcl-syntax/tests/data/native_variable_name/cache_lookup/probe.c)
- [rust/tcl-syntax/tests/data/native_variable_name/cache_lookup/manifest.json](../../../../rust/tcl-syntax/tests/data/native_variable_name/cache_lookup/manifest.json)
- [rust/tcl-syntax/tests/data/native_variable_name/cache_lookup/paths.txt](../../../../rust/tcl-syntax/tests/data/native_variable_name/cache_lookup/paths.txt)

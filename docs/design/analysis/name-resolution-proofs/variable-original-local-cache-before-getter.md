# naming.variable.original-local-cache-before-getter

Kind: `native-observation`

## Problem statement

A procedure local-name primary can select the same cell with no resident name string. Obtaining display bytes first would alter the original object and may invoke an unavailable updater.

## Question

After warming original x then invalidating its string, do ordinary original-object Get/Set retain absent strings and the same Var?

## Conclusion

Both ordinary modes succeed with the same Var in all five C captures. C8.4 regenerates the original name string; C8.5–9.1 leave it absent and retain localVarName. This is exact same-cell/current-name cache selection for two modes, not evidence for trace/missing-value paths or unentered source lookup.

## Scope

Five fresh interpreters per C release, actual ASCII procedure body proc p {} {set x VALUE;probe x};p, private TclObjLookupVar original cell comparison before byte/result observation. Mode0=Get,1=Set,2=read/write trace then Get,3=trace then Set,4=unset then Get. Name string is explicitly invalidated but localVarName remains. Each capture retains separate status/stdout/stderr; no signal window supplies unprinted observer rows. The printed result getter is on interpreter result after cell lookup, not on the name. No Jim/BIG-IP attempt.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Public C evaluator enters original compiled procedure; public ObjGetVar2/ObjSetVar2 and private original-cell lookup surround explicit Tcl_InvalidateStringRep. Version: tcl8.4 8.4.20 (captured release association; launched patchlevel unqueried).

Exact selected independent subprocess outcomes:

```json
[
  {
    "case": 0,
    "exit": 0,
    "stdout": "0\t0\t1\tlocalVarName\t1\t\n",
    "stderr": ""
  },
  {
    "case": 1,
    "exit": 0,
    "stdout": "1\t0\t1\tlocalVarName\t1\t\n",
    "stderr": ""
  }
]
```
Process0 is distinct from the printed guest completion. Fatal processes have no reached guest/result snapshot.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Public C evaluator enters original compiled procedure; public ObjGetVar2/ObjSetVar2 and private original-cell lookup surround explicit Tcl_InvalidateStringRep. Version: tcl8.5 8.5.19 (captured release association; launched patchlevel unqueried); tcl8.6 8.6.18 (captured release association; launched patchlevel unqueried); tcl9.0 9.0.4 (captured release association; launched patchlevel unqueried); tcl9.1 9.1.0 (captured release association; launched patchlevel unqueried).

Exact selected independent subprocess outcomes:

```json
[
  {
    "case": 0,
    "exit": 0,
    "stdout": "0\t0\t0\tlocalVarName\t1\t\n",
    "stderr": ""
  },
  {
    "case": 1,
    "exit": 0,
    "stdout": "1\t0\t0\tlocalVarName\t1\t\n",
    "stderr": ""
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

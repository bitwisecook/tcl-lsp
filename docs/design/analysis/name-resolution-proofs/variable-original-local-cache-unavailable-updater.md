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

Status: `observed`. Version: 8.4.20 (captured release association; launched patchlevel unqueried). Build: headers={'/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tcl.h': '824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf', '/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInt.h': 'f51c8e66c562de424b84a35a04b6b86d9a135ef7d4ca891b35c3b0586bc0ce1a'}; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; binary_sha256=f3b2a6c5e22b77dd12b505f0dedc3910c79df7f795cc13f27e692f300a088e24; compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public C evaluator enters original compiled procedure; public ObjGetVar2/ObjSetVar2 and private original-cell lookup surround explicit Tcl_InvalidateStringRep.. Dialect: C Tcl.

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

### tcl8.5

Status: `observed`. Version: 8.5.19 (captured release association; launched patchlevel unqueried). Build: headers={'/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tcl.h': 'c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5', '/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInt.h': '72078ca207924e61b4fbf637eff263d280a0dbc78e6a5fbb39e2cd980d08a8aa'}; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; binary_sha256=46784d208e7a888d2dad712f3248cfb140ae587ee13133b68d121bfd9800fa10; compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public C evaluator enters original compiled procedure; public ObjGetVar2/ObjSetVar2 and private original-cell lookup surround explicit Tcl_InvalidateStringRep.. Dialect: C Tcl.

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

### tcl8.6

Status: `observed`. Version: 8.6.18 (captured release association; launched patchlevel unqueried). Build: headers={'/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tcl.h': 'aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245', '/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInt.h': 'e9e70f6461d7c4b37173173956040b647914157c28425ec859730cd8eaef4d2e'}; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; binary_sha256=6f93ead73f66a8d172e812af785d7f9bd1d604cb561f12cecbfa68664c7cca73; compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public C evaluator enters original compiled procedure; public ObjGetVar2/ObjSetVar2 and private original-cell lookup surround explicit Tcl_InvalidateStringRep.. Dialect: C Tcl.

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

### tcl9.0

Status: `observed`. Version: 9.0.4 (captured release association; launched patchlevel unqueried). Build: headers={'/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tcl.h': 'eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a', '/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInt.h': 'f000e14fdc4e00da1c5ab98a8087c41be3cbf26564a274152236e0531bb55053'}; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; binary_sha256=1bfc0c661d2670e53c7f11755b8e1be77c0960b5f414ed7635496fe0c4ef608b; compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public C evaluator enters original compiled procedure; public ObjGetVar2/ObjSetVar2 and private original-cell lookup surround explicit Tcl_InvalidateStringRep.. Dialect: C Tcl.

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

### tcl9.1

Status: `observed`. Version: 9.1.0 (captured release association; launched patchlevel unqueried). Build: headers={'/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tcl.h': '30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950', '/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInt.h': 'fea9d99020df9aec6efebf6e3f18592ee267a9bf80d3843df0d6439efebe65cc'}; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; binary_sha256=bf9f9c4051f17e2cfc46f999d01cb06ba5abc56fadc3cbd3920f76936d2e2af5; compile_exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public C evaluator enters original compiled procedure; public ObjGetVar2/ObjSetVar2 and private original-cell lookup surround explicit Tcl_InvalidateStringRep.. Dialect: C Tcl.

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

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No observation of this exact question is retained for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No observation of this exact question is retained for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-syntax/tests/data/native_variable_name/cache_lookup/probe.c](../../../../rust/tcl-syntax/tests/data/native_variable_name/cache_lookup/probe.c). SHA-256 `75eda78208cfd7af1ada6b0ad57c4735eaff74250a2d232a29e49f3186f2caa4`. Exact original object/cache/getter order and five modes.
- `receipt` (observation): [rust/tcl-syntax/tests/data/native_variable_name/cache_lookup/manifest.json](../../../../rust/tcl-syntax/tests/data/native_variable_name/cache_lookup/manifest.json). SHA-256 `559c1d434d826cab00e78f6fe9d73c95bc104efdb75e1f432a4eabc9edcea024`. Original full per-mode native process status/stdout/stderr and release-specific fatal diagnostics.
- `projection` (observation): [rust/tcl-syntax/tests/data/native_variable_name/cache_lookup/paths.txt](../../../../rust/tcl-syntax/tests/data/native_variable_name/cache_lookup/paths.txt). SHA-256 `d804a750ac091297a0d6be1b7a8297e80e90fc6a4ae9f52b3c010fdee2c52658`. All 25 checked projected paths. updater-unavailable labels summarize fatal status; exact signals remain in receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/native_variable_name.rs](../../../../rust/tcl-syntax/src/native_variable_name.rs), `NativeVariableNameProtocol::cache_precedes_string_getter`: Selects cached-name versus byte-getter order without fabricating an unavailable original updater.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::original_local_cache_getter_order_matches_all_25_native_paths` (linked): Compares 25 ordinary/fatal native paths; Rust host refusal is an independent implementation response to the measured fatal native updater, not a native guest completion.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::original_local_cache_getter_order_matches_all_25_native_paths` (linked): Compares 25 ordinary/fatal native paths; Rust host refusal is an independent implementation response to the measured fatal native updater, not a native guest completion.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test result is asserted. A new comparison must compile the exact retained probe against independently identified release headers and archive, preserve its original API/flags and declared observer references, then capture separate process status, stdout and stderr. Original absolute compiler paths are attribution metadata, not a portable replay command. The probe is input source, not an excerpt of the Tcl/Jim implementation.

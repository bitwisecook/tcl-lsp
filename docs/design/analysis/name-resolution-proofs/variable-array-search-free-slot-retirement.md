# naming.variable.array-search-free-slot-retirement

Kind: `native-observation`

## Problem statement

An array-search primary has no free hook in older C releases. Clearing every primary or retaining every no-free-hook primary before a variable lookup predicts different original-header states; duplication adds a separate retirement window.

## Question

What primary/free-hook/resident states remain on the original array-search handle and its duplicate before and after missing variable reads?

## Conclusion

C8.4 retains array search with no free hook on both missing reads. C8.5/8.6 begin with that primary on both objects and retire both to an untyped primary. C9 begins with a String primary on the original and an untyped duplicate; those states remain after reads. All four snapshots retain resident strings. This measures the selected descriptor retirement and copy windows, not search validity or any arbitrary variable read completion.

## Scope

Twenty original header observations across five C captures. Fields are phase, primary, free-hook-present and string-resident. Search identifier content is not printed; the missing read result/completion is not sampled. One original handle and one duplicate have explicit external references. No Jim or BIG-IP attempt.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (captured release association; launched patchlevel unqueried). Build: header_sha256=824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public Tcl_EvalEx creates a search; original Tcl_EvalObjv selects array anymore, then Tcl_ObjGetVar2 reads the original handle and duplicate.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
converted|array search|0|1
duplicate|array search|0|1
missing|array search|0|1
duplicate-missing|array search|0|1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl8.5

Status: `observed`. Version: 8.5.19 (captured release association; launched patchlevel unqueried). Build: header_sha256=c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public Tcl_EvalEx creates a search; original Tcl_EvalObjv selects array anymore, then Tcl_ObjGetVar2 reads the original handle and duplicate.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
converted|array search|0|1
duplicate|array search|0|1
missing|none|0|1
duplicate-missing|none|0|1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl8.6

Status: `observed`. Version: 8.6.18 (captured release association; launched patchlevel unqueried). Build: header_sha256=aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public Tcl_EvalEx creates a search; original Tcl_EvalObjv selects array anymore, then Tcl_ObjGetVar2 reads the original handle and duplicate.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
converted|array search|0|1
duplicate|array search|0|1
missing|none|0|1
duplicate-missing|none|0|1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl9.0

Status: `observed`. Version: 9.0.4 (captured release association; launched patchlevel unqueried). Build: header_sha256=eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public Tcl_EvalEx creates a search; original Tcl_EvalObjv selects array anymore, then Tcl_ObjGetVar2 reads the original handle and duplicate.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
converted|string|1|1
duplicate|none|0|1
missing|string|1|1
duplicate-missing|none|0|1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl9.1

Status: `observed`. Version: 9.1.0 (captured release association; launched patchlevel unqueried). Build: header_sha256=30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public Tcl_EvalEx creates a search; original Tcl_EvalObjv selects array anymore, then Tcl_ObjGetVar2 reads the original handle and duplicate.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
converted|string|1|1
duplicate|none|0|1
missing|string|1|1
duplicate-missing|none|0|1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No observation of this exact question is retained for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No observation of this exact question is retained for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-syntax/tests/data/native_variable_name/array_search_free_slot.c](../../../../rust/tcl-syntax/tests/data/native_variable_name/array_search_free_slot.c). SHA-256 `36b70978da15c76f1b1358ece97bc6b356dba0f635fb713dcf46e86dbb2a24bb`. Exact original public/private observer and declared reference ownership.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_variable_name/array_search_free_slot_provenance.json](../../../../rust/tcl-syntax/tests/data/native_variable_name/array_search_free_slot_provenance.json). SHA-256 `dbe9d5e543bb0e65bb13c9febdf18d73fa43d17505dd2a70ee6ec09682fbf3b7`. Original independently attributed compilation and process captures; inline observations preserve full raw outcomes.
- `rows` (observation): [rust/tcl-syntax/tests/data/native_variable_name/array_search_free_slot.txt](../../../../rust/tcl-syntax/tests/data/native_variable_name/array_search_free_slot.txt). SHA-256 `5e9ea90fc5e52e398665b48c47cfde60d9e98ac3a0426ae89f5582e4c48e5cb0`. Exact checked per-provider projection. Selection is specified in the provider answer; omitted call lines remain in the receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/native_variable_name.rs](../../../../rust/tcl-syntax/src/native_variable_name.rs), `NativeVariableNameProtocol::retires_before_simple_lookup`: Selects whether the actual original free-hook/primary is retired before simple lookup.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::search_free_slots_match_all_20_actual_c_windows` (linked): Compares the 20 retained primary/free-hook/resident windows; host variable read and duplicate ownership remain independent implementation obligations.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::search_free_slots_match_all_20_actual_c_windows` (linked): Compares the 20 retained primary/free-hook/resident windows; host variable read and duplicate ownership remain independent implementation obligations.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test result is asserted. A new comparison must compile the exact retained probe against independently identified release headers and archive, preserve its original API/flags and declared observer references, then capture separate process status, stdout and stderr. Original absolute compiler paths are attribution metadata, not a portable replay command. The probe is input source, not an excerpt of the Tcl/Jim implementation.

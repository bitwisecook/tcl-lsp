# naming.variable.original-element-alias-header-roles

Kind: `native-observation`

## Problem statement

Combined-name parser root/index children, table key ownership, two live aliases and callback/frame retirement each add different header or Var roles. A test observer may add its own reference and must declare it.

## Question

Which original combined-name/root/key and live Var counts survive alias linking, element/array unset and both frame removals?

## Conclusion

C8.4 keeps combined/root refs1 and has no observed index header; its original Var refs move0→2→1 before both frames retire. C8.5/8.6 explicitly pin the separate key once and begin root/key refs2; whole-array removal reduces those child/key counts to1, while element-only removal preserves them until frame retirement. C9 owns both root/index children at2, with no extra observer pin; its Var roles begin1 and reach3 for two aliases. Jim combined-name refs reach3 for two aliases and decline2 then1 at frame removal; whole-array removal reduces its root/index children to1. Unobserved C Var fields in Jim are−1, and retired C node contents are never dereferenced.

## Scope

Six exact processes, 38 raw lines each, element-only and whole-array removal routes. Combined-name child pointers are borrowed from the separately owned unchanged name. C8.5/8.6 add one explicit external key observer pin; C9/Jim add no child observer references. C Var fields are read only while table or aliases own the old node. Jim has no C Var/entry fields. This is neither arbitrary frame relocation nor a namespace/path/normal-write grant. Jim patch/revision/configuration are unqueried; BIG-IP not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (captured release association; launched patchlevel unqueried). Build: header_sha256=824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; binary_sha256=1d7bee24290a58e692d01fd7f484b8b105140ae7519a78fa56931a90fd71cf28; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: C public variable APIs plus Tcl_UpVar and ASCII Tcl_EvalEx procedures; Jim original Jim_SetVariable/GetVariable/SetVariableLink and Jim_Eval with explicitly selected top frame. Private parser/header/live Var snapshots.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
call|created|0
observer-key-pin|0
window|element|created|1|1|-1|1|1|1|0|0
call|link-a|0
call|link-b|0
window|element|linked|1|1|-1|1|1|1|0|2
call|unset-target|0
window|element|unset-target|1|1|-1|1|1|0|0|2
value|alias-after-unset|null
call|recreate|0
window|element|recreated|1|1|-1|1|1|1|0|2
value|alias-after-recreate|54574f
value|fresh-after-recreate|54574f
call|unset-recreated|0
window|element|unset-recreated|1|1|-1|1|1|0|0|2
call|inner|0
window|element|one-alias|1|1|-1|1|1|0|0|1
call|procedure|0
window|element|no-alias|1|1|-1|0|0|-1|-1|-1
call|created|0
observer-key-pin|0
window|whole|created|1|1|-1|1|1|1|0|0
call|link-a|0
call|link-b|0
window|whole|linked|1|1|-1|1|1|1|0|2
call|unset-target|0
window|whole|unset-target|1|1|-1|0|0|0|1|2
value|alias-after-unset|null
call|recreate|0
window|whole|recreated|1|1|-1|1|0|0|1|2
value|alias-after-recreate|null
value|fresh-after-recreate|54574f
call|unset-recreated|0
window|whole|unset-recreated|1|1|-1|0|0|0|1|2
call|inner|0
window|whole|one-alias|1|1|-1|0|0|0|1|1
call|procedure|0
window|whole|no-alias|1|1|-1|0|0|-1|-1|-1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl8.5

Status: `observed`. Version: 8.5.19 (captured release association; launched patchlevel unqueried). Build: header_sha256=c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; binary_sha256=8f0216468e8d31509cbd0313bf328d832965ef26788b9f30f9afb8cd10e4b84c; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: C public variable APIs plus Tcl_UpVar and ASCII Tcl_EvalEx procedures; Jim original Jim_SetVariable/GetVariable/SetVariableLink and Jim_Eval with explicitly selected top frame. Private parser/header/live Var snapshots.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
call|created|0
observer-key-pin|1
window|element|created|1|2|2|1|1|1|0|1
call|link-a|0
call|link-b|0
window|element|linked|1|2|2|1|1|1|0|3
call|unset-target|0
window|element|unset-target|1|2|2|1|1|0|0|3
value|alias-after-unset|null
call|recreate|0
window|element|recreated|1|2|2|1|1|1|0|3
value|alias-after-recreate|54574f
value|fresh-after-recreate|54574f
call|unset-recreated|0
window|element|unset-recreated|1|2|2|1|1|0|0|3
call|inner|0
window|element|one-alias|1|2|2|1|1|0|0|2
call|procedure|0
window|element|no-alias|1|2|1|0|0|-1|-1|-1
call|created|0
observer-key-pin|1
window|whole|created|1|2|2|1|1|1|0|1
call|link-a|0
call|link-b|0
window|whole|linked|1|2|2|1|1|1|0|3
call|unset-target|0
window|whole|unset-target|1|1|1|0|0|0|1|2
value|alias-after-unset|null
call|recreate|0
window|whole|recreated|1|1|1|1|0|0|1|2
value|alias-after-recreate|null
value|fresh-after-recreate|54574f
call|unset-recreated|0
window|whole|unset-recreated|1|1|1|0|0|0|1|2
call|inner|0
window|whole|one-alias|1|1|1|0|0|0|1|1
call|procedure|0
window|whole|no-alias|1|1|1|0|0|-1|-1|-1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl8.6

Status: `observed`. Version: 8.6.18 (captured release association; launched patchlevel unqueried). Build: header_sha256=aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; binary_sha256=e705cc15a199cfe6667400ee22b6b12b7312c0d8f963983dc015d39a35094312; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: C public variable APIs plus Tcl_UpVar and ASCII Tcl_EvalEx procedures; Jim original Jim_SetVariable/GetVariable/SetVariableLink and Jim_Eval with explicitly selected top frame. Private parser/header/live Var snapshots.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
call|created|0
observer-key-pin|1
window|element|created|1|2|2|1|1|1|0|1
call|link-a|0
call|link-b|0
window|element|linked|1|2|2|1|1|1|0|3
call|unset-target|0
window|element|unset-target|1|2|2|1|1|0|0|3
value|alias-after-unset|null
call|recreate|0
window|element|recreated|1|2|2|1|1|1|0|3
value|alias-after-recreate|54574f
value|fresh-after-recreate|54574f
call|unset-recreated|0
window|element|unset-recreated|1|2|2|1|1|0|0|3
call|inner|0
window|element|one-alias|1|2|2|1|1|0|0|2
call|procedure|0
window|element|no-alias|1|2|1|0|0|-1|-1|-1
call|created|0
observer-key-pin|1
window|whole|created|1|2|2|1|1|1|0|1
call|link-a|0
call|link-b|0
window|whole|linked|1|2|2|1|1|1|0|3
call|unset-target|0
window|whole|unset-target|1|1|1|0|0|0|1|2
value|alias-after-unset|null
call|recreate|0
window|whole|recreated|1|1|1|1|0|0|1|2
value|alias-after-recreate|null
value|fresh-after-recreate|54574f
call|unset-recreated|0
window|whole|unset-recreated|1|1|1|0|0|0|1|2
call|inner|0
window|whole|one-alias|1|1|1|0|0|0|1|1
call|procedure|0
window|whole|no-alias|1|1|1|0|0|-1|-1|-1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl9.0

Status: `observed`. Version: 9.0.4 (captured release association; launched patchlevel unqueried). Build: header_sha256=eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; binary_sha256=d7e7b0beefa92716e0204dbff83bb3379d52c2c4526152f2fc9cb433db025e10; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: C public variable APIs plus Tcl_UpVar and ASCII Tcl_EvalEx procedures; Jim original Jim_SetVariable/GetVariable/SetVariableLink and Jim_Eval with explicitly selected top frame. Private parser/header/live Var snapshots.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
call|created|0
observer-key-pin|0
window|element|created|1|2|2|1|1|1|0|1
call|link-a|0
call|link-b|0
window|element|linked|1|2|2|1|1|1|0|3
call|unset-target|0
window|element|unset-target|1|2|2|1|1|0|0|3
value|alias-after-unset|null
call|recreate|0
window|element|recreated|1|2|2|1|1|1|0|3
value|alias-after-recreate|54574f
value|fresh-after-recreate|54574f
call|unset-recreated|0
window|element|unset-recreated|1|2|2|1|1|0|0|3
call|inner|0
window|element|one-alias|1|2|2|1|1|0|0|2
call|procedure|0
window|element|no-alias|1|2|1|0|0|-1|-1|-1
call|created|0
observer-key-pin|0
window|whole|created|1|2|2|1|1|1|0|1
call|link-a|0
call|link-b|0
window|whole|linked|1|2|2|1|1|1|0|3
call|unset-target|0
window|whole|unset-target|1|1|1|0|0|0|1|2
value|alias-after-unset|null
call|recreate|0
window|whole|recreated|1|1|1|1|0|0|1|2
value|alias-after-recreate|null
value|fresh-after-recreate|54574f
call|unset-recreated|0
window|whole|unset-recreated|1|1|1|0|0|0|1|2
call|inner|0
window|whole|one-alias|1|1|1|0|0|0|1|1
call|procedure|0
window|whole|no-alias|1|1|1|0|0|-1|-1|-1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl9.1

Status: `observed`. Version: 9.1.0 (captured release association; launched patchlevel unqueried). Build: header_sha256=30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; binary_sha256=54c468ec0bf3e82b77b647457cd8baba35a83cc9fef360151df3b88ac34965eb; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: C public variable APIs plus Tcl_UpVar and ASCII Tcl_EvalEx procedures; Jim original Jim_SetVariable/GetVariable/SetVariableLink and Jim_Eval with explicitly selected top frame. Private parser/header/live Var snapshots.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
call|created|0
observer-key-pin|0
window|element|created|1|2|2|1|1|1|0|1
call|link-a|0
call|link-b|0
window|element|linked|1|2|2|1|1|1|0|3
call|unset-target|0
window|element|unset-target|1|2|2|1|1|0|0|3
value|alias-after-unset|null
call|recreate|0
window|element|recreated|1|2|2|1|1|1|0|3
value|alias-after-recreate|54574f
value|fresh-after-recreate|54574f
call|unset-recreated|0
window|element|unset-recreated|1|2|2|1|1|0|0|3
call|inner|0
window|element|one-alias|1|2|2|1|1|0|0|2
call|procedure|0
window|element|no-alias|1|2|1|0|0|-1|-1|-1
call|created|0
observer-key-pin|0
window|whole|created|1|2|2|1|1|1|0|1
call|link-a|0
call|link-b|0
window|whole|linked|1|2|2|1|1|1|0|3
call|unset-target|0
window|whole|unset-target|1|1|1|0|0|0|1|2
value|alias-after-unset|null
call|recreate|0
window|whole|recreated|1|1|1|1|0|0|1|2
value|alias-after-recreate|null
value|fresh-after-recreate|54574f
call|unset-recreated|0
window|whole|unset-recreated|1|1|1|0|0|0|1|2
call|inner|0
window|whole|one-alias|1|1|1|0|0|0|1|1
call|procedure|0
window|whole|no-alias|1|1|1|0|0|-1|-1|-1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### jim

Status: `observed`. Version: Jim (patchlevel, revision and configuration unrecorded). Build: header_sha256=d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d; library_sha256=75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; binary_sha256=d4035f7eb637fa1b535dbd8dc739da0bc146a35921ebde0a44769b6a55b73e9d; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: C public variable APIs plus Tcl_UpVar and ASCII Tcl_EvalEx procedures; Jim original Jim_SetVariable/GetVariable/SetVariableLink and Jim_Eval with explicitly selected top frame. Private parser/header/live Var snapshots.. Dialect: Jim Tcl.

Exact selected observer rows, in original column order:

```text
call|created|0
observer-key-pin|0
window|element|created|1|2|2|-1|-1|-1|-1|-1
call|link-a|0
call|link-b|0
window|element|linked|3|2|2|-1|-1|-1|-1|-1
call|unset-target|0
window|element|unset-target|3|2|1|-1|-1|-1|-1|-1
value|alias-after-unset|null
call|recreate|0
window|element|recreated|3|2|1|-1|-1|-1|-1|-1
value|alias-after-recreate|54574f
value|fresh-after-recreate|54574f
call|unset-recreated|0
window|element|unset-recreated|3|2|1|-1|-1|-1|-1|-1
call|inner|0
window|element|one-alias|2|2|1|-1|-1|-1|-1|-1
call|procedure|0
window|element|no-alias|1|2|1|-1|-1|-1|-1|-1
call|created|0
observer-key-pin|0
window|whole|created|1|2|2|-1|-1|-1|-1|-1
call|link-a|0
call|link-b|0
window|whole|linked|3|2|2|-1|-1|-1|-1|-1
call|unset-target|0
window|whole|unset-target|3|1|1|-1|-1|-1|-1|-1
value|alias-after-unset|null
call|recreate|0
window|whole|recreated|3|1|1|-1|-1|-1|-1|-1
value|alias-after-recreate|54574f
value|fresh-after-recreate|54574f
call|unset-recreated|0
window|whole|unset-recreated|3|1|1|-1|-1|-1|-1|-1
call|inner|0
window|whole|one-alias|2|1|1|-1|-1|-1|-1|-1
call|procedure|0
window|whole|no-alias|1|1|1|-1|-1|-1|-1|-1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No observation of this exact question is retained for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-syntax/tests/data/native_variable_name/element_alias_lifecycle.c](../../../../rust/tcl-syntax/tests/data/native_variable_name/element_alias_lifecycle.c). SHA-256 `8ad9fdc08a5eb3ad539d95dbd1d9c0d0856ebb4f75934a14a0539d576c45ac6f`. Exact original public/private observer and declared reference ownership.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_variable_name/element_alias_lifecycle.provenance.json](../../../../rust/tcl-syntax/tests/data/native_variable_name/element_alias_lifecycle.provenance.json). SHA-256 `7cbb0d09f92d0e628e6634220b790f873cd53def5ceddc09d6e9a46d1b87a7b1`. Original independently attributed compilation and process captures; inline observations preserve full raw outcomes.
- `rows` (observation): [rust/tcl-syntax/tests/data/native_variable_name/element_alias_lifecycle.txt](../../../../rust/tcl-syntax/tests/data/native_variable_name/element_alias_lifecycle.txt). SHA-256 `c750947708fb8498600e7032656d18978509733afc25f281efaa970c886c81ab`. Exact checked per-provider projection. Selection is specified in the provider answer; omitted call lines remain in the receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/native_variable_name.rs](../../../../rust/tcl-syntax/src/native_variable_name.rs), `NativeVariableNameProtocol::parsed_element_table_key`: Selects original versus separately issued parser-to-table keys without donating equal-name identity.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::element_entry_lifecycle_matches_all_70_actual_c_windows` (linked): Compares the 70 C node/header snapshots across both mutation routes; Jim returned values and header references remain separately retained native observations.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test result is asserted. A new comparison must compile the exact retained probe against independently identified release headers and archive, preserve its original API/flags and declared observer references, then capture separate process status, stdout and stderr. Original absolute compiler paths are attribution metadata, not a portable replay command. The probe is input source, not an excerpt of the Tcl/Jim implementation.

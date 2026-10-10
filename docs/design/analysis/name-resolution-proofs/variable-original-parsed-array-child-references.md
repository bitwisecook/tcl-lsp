# naming.variable.original-parsed-array-child-references

Kind: `native-observation`

## Problem statement

A combined parsed name owns child storage that a duplicate may share. Table removal and duplicate destruction change child counts independently of the parent name or lookup result.

## Question

What child header references remain after original arr(k) store, name duplication and duplicate/table removal?

## Conclusion

C8.4 reports root child refs1,2,1. C8.5/8.6 report root2,3,1. C9 reports root and index2,3,1 independently. Pre-C9 index fields are deliberately unobserved and reported−1; those sentinels do not mean no index bytes exist. These three windows do not supply arbitrary alias allocation or table membership proof.

## Scope

Five exact C processes, 29 raw lines per process; the retained aggregate projects only the 19 alias/child snapshots per release. Full call/procedure outcomes remain in each original inline receipt and match its recorded raw log SHA. The scalar before pointer is compared but its fields are not dereferenced after retirement. Whole strings are ordinary k/arr(k), not opaque byte inputs. No Jim or BIG-IP attempt.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (captured release association; launched patchlevel unqueried). Build: header_sha256=824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; binary_sha256=d3ecfedafcd368cecfdd8b74c4f9671900be264ff221f923b97b8f0a4cbb8d4d; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public original Tcl_ObjSetVar2/GetVar2, Tcl_EvalObjv upvar/unset inside a procedure worker, and Tcl_DuplicateObj; private original Var/hash-entry/header observer.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
parts|stored|1|-1
parts|duplicated|2|-1
parts|unset|1|-1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl8.5

Status: `observed`. Version: 8.5.19 (captured release association; launched patchlevel unqueried). Build: header_sha256=c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; binary_sha256=af4e160f0da1dcb0eff47d4f00d174feae463807aa5b9e7a47ef211bf00886dc; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public original Tcl_ObjSetVar2/GetVar2, Tcl_EvalObjv upvar/unset inside a procedure worker, and Tcl_DuplicateObj; private original Var/hash-entry/header observer.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
parts|stored|2|-1
parts|duplicated|3|-1
parts|unset|1|-1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl8.6

Status: `observed`. Version: 8.6.18 (captured release association; launched patchlevel unqueried). Build: header_sha256=aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; binary_sha256=5a60e19d1b28c66ae0501119057144731182f946a892f422368faa9941e3d4ca; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public original Tcl_ObjSetVar2/GetVar2, Tcl_EvalObjv upvar/unset inside a procedure worker, and Tcl_DuplicateObj; private original Var/hash-entry/header observer.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
parts|stored|2|-1
parts|duplicated|3|-1
parts|unset|1|-1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl9.0

Status: `observed`. Version: 9.0.4 (captured release association; launched patchlevel unqueried). Build: header_sha256=eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; binary_sha256=fd5a7bb1950df87490e4276a0cd5ed326f93631305d2d5d1ff476fe30dc5665f; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public original Tcl_ObjSetVar2/GetVar2, Tcl_EvalObjv upvar/unset inside a procedure worker, and Tcl_DuplicateObj; private original Var/hash-entry/header observer.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
parts|stored|2|2
parts|duplicated|3|3
parts|unset|1|1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### tcl9.1

Status: `observed`. Version: 9.1.0 (captured release association; launched patchlevel unqueried). Build: header_sha256=30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; binary_sha256=d600ef742d436b627f9f74027648fcc0b379567a95cad4a407f2e719d2a600e3; compile_exit=0; exit=0. Full configuration/compiler version and launched runtime patchlevel were not queried. Any absent binary/status/hash field is unrecorded, not reconstructed from the currently selected providers.. Channel: Public original Tcl_ObjSetVar2/GetVar2, Tcl_EvalObjv upvar/unset inside a procedure worker, and Tcl_DuplicateObj; private original Var/hash-entry/header observer.. Dialect: C Tcl.

Exact selected observer rows, in original column order:

```text
parts|stored|2|2
parts|duplicated|3|3
parts|unset|1|1
```
Counts include the explicitly declared probe references. Unobserved fields and operations are not inferred.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No observation of this exact question is retained for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No observation of this exact question is retained for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries.c](../../../../rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries.c). SHA-256 `d1e56adf155f6fff0818a1a18fedc138751323dfb8b0c0a7245e4ddc4cf9b58c`. Exact original public/private observer and declared reference ownership.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries_provenance.json](../../../../rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries_provenance.json). SHA-256 `af16b31c27a19cdaa44e528d75b34fce13bbd74ca38da9423bc2ae908ea2460d`. Original independently attributed compilation and process captures; inline observations preserve full raw outcomes.
- `rows` (observation): [rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries.txt](../../../../rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries.txt). SHA-256 `fd02218da3f15cca5e8f4eb9e2fe58e8f34652edbe3a4dd228133a502367514c`. Exact checked per-provider projection. Selection is specified in the provider answer; omitted call lines remain in the receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/native_variable_name.rs](../../../../rust/tcl-syntax/src/native_variable_name.rs), `NativeVariableNameProtocol::element_table_retains_original`: Keeps actual object-key ownership distinct from parser header/entry lifetime.
- [runtime/rust/src/interp/native_variable_names.rs](../../../../runtime/rust/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::scalar_alias_entries_and_array_parts_match_all_95_native_windows` (linked): Compares 95 projected original scalar entry/key and parsed array child-reference snapshots; actual raw guest failure remains separately retained.
- [rust/tcl-vm/src/interp/native_variable_names.rs](../../../../rust/tcl-vm/src/interp/native_variable_names.rs), `interp::native_variable_names::tests::scalar_alias_entries_and_array_parts_match_all_95_native_windows` (linked): Compares 95 projected original scalar entry/key and parsed array child-reference snapshots; actual raw guest failure remains separately retained.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test result is asserted. A new comparison must compile the exact retained probe against independently identified release headers and archive, preserve its original API/flags and declared observer references, then capture separate process status, stdout and stderr. Original absolute compiler paths are attribution metadata, not a portable replay command. The probe is input source, not an excerpt of the Tcl/Jim implementation.

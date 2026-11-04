# naming.namespace-native.opaque-child-enumeration

Kind: `native-observation`

## Problem statement

Opaque parent/child namespace names may be lost if native enumeration is reconstructed through host Unicode or displayed-name parsing.

## Question

Does original namespace children enumerate the raw-FF parent's raw-FD child with exact returned resident bytes?

## Conclusion

All five C providers return code0 and list-primary result bytes 3a3a726177ff3a3a6368696c64fd. The original opaque child spelling survives this native enumeration; no namespace allocation or pattern-matching rule beyond the observed child is inferred.

## Scope

Five C builds each create ::raw-FF and its ::child-FD namespace via native APIs. Original Tcl_NewStringObj parent/pattern argv reaches namespace children with flags0. Result primary is sampled before the counted string getter. Complete original four-row streams are reconstructed only by exact original log SHA256 agreement. No native refcount, return-option, allocator, Jim or BIG-IP observation.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel unqueried). Build: Header; library; observer; compile/process0. Compiler version/configure flags unrecorded.. Channel: Native namespace creation and original object-vector children query with flags0; original result primary followed by counted result getter.. Dialect: C Tcl.

Selected original label/code/result-primary/result-hex rows:

```text
enumerate|0|list|3a3a726177ff3a3a6368696c64fd
```
Whole original stream bytes agree with the original recorded log digest; no new native run is claimed.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel unqueried). Build: Header; library; observer; compile/process0. Compiler version/configure flags unrecorded.. Channel: Native namespace creation and original object-vector children query with flags0; original result primary followed by counted result getter.. Dialect: C Tcl.

Selected original label/code/result-primary/result-hex rows:

```text
enumerate|0|list|3a3a726177ff3a3a6368696c64fd
```
Whole original stream bytes agree with the original recorded log digest; no new native run is claimed.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel unqueried). Build: Header; library; observer; compile/process0. Compiler version/configure flags unrecorded.. Channel: Native namespace creation and original object-vector children query with flags0; original result primary followed by counted result getter.. Dialect: C Tcl.

Selected original label/code/result-primary/result-hex rows:

```text
enumerate|0|list|3a3a726177ff3a3a6368696c64fd
```
Whole original stream bytes agree with the original recorded log digest; no new native run is claimed.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel unqueried). Build: Header; library; observer; compile/process0. Compiler version/configure flags unrecorded.. Channel: Native namespace creation and original object-vector children query with flags0; original result primary followed by counted result getter.. Dialect: C Tcl.

Selected original label/code/result-primary/result-hex rows:

```text
enumerate|0|list|3a3a726177ff3a3a6368696c64fd
```
Whole original stream bytes agree with the original recorded log digest; no new native run is claimed.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel unqueried). Build: Header; library; observer; compile/process0. Compiler version/configure flags unrecorded.. Channel: Native namespace creation and original object-vector children query with flags0; original result primary followed by counted result getter.. Dialect: C Tcl.

Selected original label/code/result-primary/result-hex rows:

```text
enumerate|0|list|3a3a726177ff3a3a6368696c64fd
```
Whole original stream bytes agree with the original recorded log digest; no new native run is claimed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No original opaque-child query capture for this provider is attached.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No original opaque-child query capture for this provider is attached.

## Historical references

- `probe` (input): [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children.c](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children.c). Exact original namespace creation, pattern argv and result-primary observer.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json). Original selected build/launch/stream associations.
- `projection` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children.txt](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children.txt). Exact20 release/label/result-primary/value projection rows.
- `stream-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-8.4.20.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-8.4.20.stdout). Complete original stdout bytes matching the original recorded whole-stream digest.
- `provider-tcl8.4` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json). JSON pointer `/runs/0`. This exact original header/library/observer/build/process association.
- `stream-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-8.5.19.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-8.5.19.stdout). Complete original stdout bytes matching the original recorded whole-stream digest.
- `provider-tcl8.5` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json). JSON pointer `/runs/1`. This exact original header/library/observer/build/process association.
- `stream-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-8.6.18.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-8.6.18.stdout). Complete original stdout bytes matching the original recorded whole-stream digest.
- `provider-tcl8.6` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json). JSON pointer `/runs/2`. This exact original header/library/observer/build/process association.
- `stream-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-9.0.4.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-9.0.4.stdout). Complete original stdout bytes matching the original recorded whole-stream digest.
- `provider-tcl9.0` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json). JSON pointer `/runs/3`. This exact original header/library/observer/build/process association.
- `stream-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-9.1.0.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-9.1.0.stdout). Complete original stdout bytes matching the original recorded whole-stream digest.
- `provider-tcl9.1` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json). JSON pointer `/runs/4`. This exact original header/library/observer/build/process association.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-cmd-core/src/namespace.rs](../../../../rust/tcl-cmd-core/src/namespace.rs), `children_original`: Select native original parent/pattern query and produce the observed result primary through the shared backend.
- [runtime/rust/src/interp/native_namespace_names/tests.rs](../../../../runtime/rust/src/interp/native_namespace_names/tests.rs), `interp::native_namespace_names::tests::opaque_children_match_20_native_query_and_primary_windows` (linked): Compares all20 actual query-primary-value projections, including these exact original pattern or enumeration rows.
- [rust/tcl-vm/src/interp/native_namespace_names.rs](../../../../rust/tcl-vm/src/interp/native_namespace_names.rs), `interp::native_namespace_names::tests::opaque_children_match_20_native_query_and_primary_windows` (linked): Compares all20 actual query-primary-value projections, including these exact original pattern or enumeration rows.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact observer source and complete original streams are attached. A fresh reconfirmation must compile against explicitly identified recorded public/private headers and static libraries and preserve original flags0 argv, compile/process statuses and separate streams. Reconstructing retained stream bytes by whole original digest agreement supplies no new provider run or implementation source explanation.

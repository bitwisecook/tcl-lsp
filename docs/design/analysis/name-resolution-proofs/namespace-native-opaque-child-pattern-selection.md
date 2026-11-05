# naming.namespace-native.opaque-child-pattern-selection

Kind: `native-observation`

## Problem statement

An exact opaque child pattern and a wildcard can take different native matching paths. Treating a code0 empty result as successful exact matching would erase the C8.5 distinction.

## Question

How do exact opaque full-name, star and negative patterns select the original opaque namespace child?

## Conclusion

All C releases return the child for star and an empty untyped result for the negative pattern. Exact full-name matching returns the child in C8.4, C8.6 and C9, while C8.5 returns empty. Every query has code0. This is a measured pattern selection difference, not a general lookup or UTF normalization equivalence.

## Scope

Five C builds each create ::raw-FF and its ::child-FD namespace via native APIs. Original Tcl_NewStringObj parent/pattern argv reaches namespace children with flags0. Result primary is sampled before the counted string getter. The retained projection contains four reconstructed rows per build. No native refcount, return-option, allocator, Jim or BIG-IP observation.

## Provider answers

### tcl8.4, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Native namespace creation and original object-vector children query with flags0; original result primary followed by counted result getter. Version: tcl8.4 8.4.20 (capture association; launched patchlevel unqueried); tcl8.6 8.6.18 (capture association; launched patchlevel unqueried); tcl9.0 9.0.4 (capture association; launched patchlevel unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel unqueried).

Selected original label/code/result-primary/result-hex rows:

```text
exact|0|list|3a3a726177ff3a3a6368696c64fd
star|0|list|3a3a726177ff3a3a6368696c64fd
negative|0|none|
```
Whole original stream bytes agree with the original recorded log digest; no new native run is claimed.

### tcl8.5

Status: `observed`. Channel: Native namespace creation and original object-vector children query with flags0; original result primary followed by counted result getter. Version: tcl8.5 8.5.19 (capture association; launched patchlevel unqueried).

Selected original label/code/result-primary/result-hex rows:

```text
exact|0|none|
star|0|list|3a3a726177ff3a3a6368696c64fd
negative|0|none|
```
Whole original stream bytes agree with the original recorded log digest; no new native run is claimed.

### jim, bigip

Status: `not-tested`.

No original opaque-child query capture for this provider is attached.

## Consumer bindings

- [rust/tcl-cmd-core/src/namespace.rs](../../../../rust/tcl-cmd-core/src/namespace.rs), `children_original`: Select native original parent/pattern query and produce the observed result primary through the shared backend.
- [runtime/rust/src/interp/native_namespace_names/tests.rs](../../../../runtime/rust/src/interp/native_namespace_names/tests.rs), `interp::native_namespace_names::tests::opaque_children_match_20_native_query_and_primary_windows` (linked): Compares all20 actual query-primary-value projections, including these exact original pattern or enumeration rows.
- [rust/tcl-vm/src/interp/native_namespace_names.rs](../../../../rust/tcl-vm/src/interp/native_namespace_names.rs), `interp::native_namespace_names::tests::opaque_children_match_20_native_query_and_primary_windows` (linked): Compares all20 actual query-primary-value projections, including these exact original pattern or enumeration rows.

## Evidence

- [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children.c](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children.c)
- [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children_provenance.json)
- [rust/tcl-syntax/tests/data/native_namespace_name/opaque_children.txt](../../../../rust/tcl-syntax/tests/data/native_namespace_name/opaque_children.txt)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-8.4.20.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-8.4.20.stdout)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-8.5.19.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-8.5.19.stdout)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-8.6.18.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-8.6.18.stdout)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-9.0.4.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-9.0.4.stdout)
- [rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-9.1.0.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_name/retained-streams/children-9.1.0.stdout)

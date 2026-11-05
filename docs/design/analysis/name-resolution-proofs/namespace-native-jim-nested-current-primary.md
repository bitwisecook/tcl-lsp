# naming.namespace-native.jim-nested-current-primary

Kind: `native-observation`

## Problem statement

Jim nested namespace current is a produced result object, not a C nsName token or another lookup of the result spelling. Borrowing the C9 producer rule would assign an unmeasured native descriptor.

## Question

What primary and bytes does the retained Jim original nested ::N::q current result expose?

## Conclusion

The retained Jim row returns code0 with string primary and bytes ::N::q. No native C nsName descriptor or parent getter result is measured in this row.

## Scope

One retained Jim result projection with exact original observer source and pinned source revision; C providers did not run this Jim-only branch. Full JSONL/process/refcounts and build configuration are unretained.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, bigip

Status: `not-tested`.

This exact nested-current observation comes from the Jim-only probe branch; no result for this provider is attached.

### jim

Status: `observed`. Channel: Jim_EvalObj of exact ASCII nested namespace source, then original result primary and counted string getter. Version: jim Jim (patchlevel unqueried).

Exact retained row:

```text
Jim|nested-current|0|string|3a3a4e3a3a71|-|-
```
No parent getter or private C namespace descriptor observation follows from this result.

## Consumer bindings

- [rust/tcl-cmd-core/src/namespace.rs](../../../../rust/tcl-cmd-core/src/namespace.rs), `current_original`: Produce the selected Jim current result through its own implementation contract.
- [runtime/rust/src/interp/native_namespace_names/tests.rs](../../../../runtime/rust/src/interp/native_namespace_names/tests.rs), `interp::native_namespace_names::tests::current_namespace_results_match_current_jim_original_producers` (linked): Compares exact Jim nested-current result primary and bytes; no C descriptor grant.

## Evidence

- [rust/tcl-syntax/tests/data/native_namespace_name/producer_getter.c](../../../../rust/tcl-syntax/tests/data/native_namespace_name/producer_getter.c)
- [rust/tcl-syntax/tests/data/native_namespace_name/provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/provenance.json)
- [rust/tcl-syntax/tests/data/native_namespace_name/producer_getter.txt](../../../../rust/tcl-syntax/tests/data/native_namespace_name/producer_getter.txt)

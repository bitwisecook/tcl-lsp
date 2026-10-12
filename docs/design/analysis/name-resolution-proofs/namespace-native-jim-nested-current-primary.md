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

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This exact nested-current observation comes from the Jim-only probe branch; no result for this provider is attached.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This exact nested-current observation comes from the Jim-only probe branch; no result for this provider is attached.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This exact nested-current observation comes from the Jim-only probe branch; no result for this provider is attached.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This exact nested-current observation comes from the Jim-only probe branch; no result for this provider is attached.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This exact nested-current observation comes from the Jim-only probe branch; no result for this provider is attached.

### jim

Status: `observed`. Version: Jim (patchlevel unqueried). Build: Pinned source revision 5bac7c99ad65864c87da513e22e2f01703fa4e03; observer SHA256 c3d665bc13ac6378b50b9a35636ce6d52d30978a689682f0a1ab2880cf0e8d7d; compile/library/configure/process closure unrecorded.. Channel: Jim_EvalObj of exact ASCII nested namespace source, then original result primary and counted string getter.. Dialect: Jim Tcl.

Exact retained row:

```text
Jim|nested-current|0|string|3a3a4e3a3a71|-|-
```
No parent getter or private C namespace descriptor observation follows from this result.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This exact nested-current observation comes from the Jim-only probe branch; no result for this provider is attached.

## Exact evidence

- `probe` (input): [rust/tcl-syntax/tests/data/native_namespace_name/producer_getter.c](../../../../rust/tcl-syntax/tests/data/native_namespace_name/producer_getter.c). SHA-256 `8161c6ac39fb84c076e20a0f6cdef5f6ae5957d890443d99cda49aadd68a2b2c`. Exact reached Jim nested-source branch.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/provenance.json). SHA-256 `e78aa86921e4db5bedefd17b89c4fd76a608bbd59feee9447bbb6d0a9a0aa6f6`. JSON pointer `/runs/5`. Original Jim observer/revision/log association.
- `rows` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/producer_getter.txt](../../../../rust/tcl-syntax/tests/data/native_namespace_name/producer_getter.txt). SHA-256 `e37094ea5691ea012f350a21b4d23d2d524a00351a61e10d1ed2b7fd8d6210a8`. Exact original nested-current projection.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-cmd-core/src/namespace.rs](../../../../rust/tcl-cmd-core/src/namespace.rs), `current_original`: Produce the selected Jim current result through its own implementation contract.
- [runtime/rust/src/interp/native_namespace_names/tests.rs](../../../../runtime/rust/src/interp/native_namespace_names/tests.rs), `interp::native_namespace_names::tests::current_namespace_results_match_current_jim_original_producers` (linked): Compares exact Jim nested-current result primary and bytes; no C descriptor grant.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact Jim branch remains in the retained probe. A fresh run must select and identify the pinned Jim build and retain full output/status/streams. The current projection alone proves no same-object cache, reference count or current namespace allocation.

# naming.property.original-accessor-name-cache-children

Kind: `native-observation`

## Problem statement

Selecting a property can cache reader/writer method-name objects on the original enumeration member. Duplicating that name can either retain those same child objects or recreate them; equal accessor spelling alone cannot establish child ownership or release behavior.

## Question

Does duplicate/free of the selected C9.1 property-name primary retain and release the same original reader/writer child objects?

## Conclusion

After property read, the original member has tcl::oo property name primary with refcount3. Its cached reader/writer children have counts2/1, and the reader primary is TclOO method name. Duplicating the member shares both exact child pointers, raises their counts to3/2 and keeps the property-name primary; releasing the duplicate restores2/1. These are actual private original-child pointer/reference windows. The ASCII selected property name is independent of the opaque-name management probe and of any claim about a Rust cache being live.

## Scope

The exact C9.1 property-name TU retains an11-row full output; this question selects property-read/accessor-reader/name-duplicate/duplicate-release (rows7–10). Earlier enumeration/header/epoch/structure rows0–6 remain independent shared owner questions. The probe inspects private PNames layout after an entered configure read, then calls Tcl_DuplicateObj and decrements the copy. No initial child count1 invariant, raw opaque accessor-name conversion, string invalidation or other release/provider behavior is established by these rows.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This exact original-object C9.1 probe has no observation for this provider.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This exact original-object C9.1 probe has no observation for this provider.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This exact original-object C9.1 probe has no observation for this provider.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This exact original-object C9.1 probe has no observation for this provider.

### tcl9.1

Status: `observed`. Version: 9.1.0 (original build/source association; probe does not query runtime patchlevel). Build: Exact original source/header/library/executable SHA-256 map in receipt; compiler command/status0 retained, compiler version not recorded.. Channel: compiled public Tcl_EvalObjv original-object argv and entered Tcl_Eval script; private object/cache observer where stated. Dialect: tcl9.1.

Selected original rows: [["property-read","34","0","1","0","3","tcl::oo property name"],["accessor-reader","34","0","1","2","1","TclOO method name"],["name-duplicate","34","0","1","3","2","tcl::oo property name"],["duplicate-release","34","0","1","2","1","none"]]. Columns are label,Foundation epoch,delta,pointer predicate,first count,second count,type; the counts/types refer to the property member or accessor children according to the exact row producer. Compile/process status0, empty stderr, no timeout. Earlier seven native rows are retained prerequisites and have their own shared header/epoch questions.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This exact original-object C9.1 probe has no observation for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This exact original-object C9.1 probe has no observation for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-registry/tests/data/native_property_names/probe.c](../../../../rust/tcl-registry/tests/data/native_property_names/probe.c). SHA-256 `d8d45219397efbb3ace20232576030f87b77d06b7f2ab9b9787283eecbd9c5e4`. Exact original-object constructors, invocation and pre-render cache observer.
- `receipt` (provider): [rust/tcl-registry/tests/data/native_property_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_property_names/manifest.json). SHA-256 `62716a386cf1022111c59359b4783ea36a0206f66150dedb0be9ffccf3dde5b8`. Original compile/process status, original source/header/library/executable digests, environment and full streams.
- `rows` (observation): [rust/tcl-registry/tests/data/native_property_names/native.tsv](../../../../rust/tcl-registry/tests/data/native_property_names/native.tsv). SHA-256 `9e9b96d1fe13e4267e47425ae247b3355dc438fad86aa43c144f2b81b8bdbafd`. Exact full original stdout retained separately.
- `capture` (limitation): [rust/tcl-registry/tests/data/native_property_names/capture.py](../../../../rust/tcl-registry/tests/data/native_property_names/capture.py). SHA-256 `39cca46544d8c83baa597fcc2babe2e5611f8020b577b4708215585f1d3fb7c3`. Original fixed-path capture adapter; it overwrites local outputs and is not a read-only verifier.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/obj/native_property_name.rs](../../../../runtime/rust/src/obj/native_property_name.rs), `accessor`: Owns the original cached reader/writer child objects independently of the optional caller management-name cache.
- [rust/tcl-vm/src/value/native_property_name.rs](../../../../rust/tcl-vm/src/value/native_property_name.rs), `native_property_accessor`: Owns the original cached reader/writer child objects independently of the optional caller management-name cache.
- [runtime/rust/src/obj/native_property_name.rs](../../../../runtime/rust/src/obj/native_property_name.rs), `tests::original_property_name_duplicate_reuses_and_releases_accessor_children` (linked): Compares identity-preserving duplicate retain/release deltas for actual original accessor children. Its raw-FF/zero input and missing string/provider refusal assertions are separate Rust contract controls, not observed in this ASCII native child-pointer probe.
- [rust/tcl-vm/src/value/native_property_name.rs](../../../../rust/tcl-vm/src/value/native_property_name.rs), `tests::original_property_name_duplicate_reuses_and_releases_accessor_children` (linked): Compares identity-preserving duplicate retain/release deltas for actual original accessor children. Its raw-FF/zero input and missing string/provider refusal assertions are separate Rust contract controls, not observed in this ASCII native child-pointer probe.

A named test is a coverage binding, not a claim that it executed.

## Replay

The retained capture.py requires its fixed /workspace native paths and writes new outputs into the fixture directory. It is not run by this record. A fresh replay must independently select the release headers/library/source hashes, preserve the original observation order, and retain the new source/status/streams separately; no executed Rust parity follows from the captured native rows.

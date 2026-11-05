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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, jim, bigip

Status: `not-tested`.

This exact original-object C9.1 probe has no observation for this provider.

### tcl9.1

Status: `observed`. Channel: compiled public Tcl_EvalObjv original-object argv and entered Tcl_Eval script; private object/cache observer where stated. Version: tcl9.1 9.1.0 (original build/source association; probe does not query runtime patchlevel).

Selected original rows: [["property-read","34","0","1","0","3","tcl::oo property name"],["accessor-reader","34","0","1","2","1","TclOO method name"],["name-duplicate","34","0","1","3","2","tcl::oo property name"],["duplicate-release","34","0","1","2","1","none"]]. Columns are label,Foundation epoch,delta,pointer predicate,first count,second count,type; the counts/types refer to the property member or accessor children according to the exact row producer. Compile/process status0, empty stderr, no timeout. Earlier seven native rows are retained prerequisites and have their own shared header/epoch questions.

## Consumer bindings

- [runtime/rust/src/obj/native_property_name.rs](../../../../runtime/rust/src/obj/native_property_name.rs), `accessor`: Owns the original cached reader/writer child objects independently of the optional caller management-name cache.
- [rust/tcl-vm/src/value/native_property_name.rs](../../../../rust/tcl-vm/src/value/native_property_name.rs), `native_property_accessor`: Owns the original cached reader/writer child objects independently of the optional caller management-name cache.
- [runtime/rust/src/obj/native_property_name.rs](../../../../runtime/rust/src/obj/native_property_name.rs), `tests::original_property_name_duplicate_reuses_and_releases_accessor_children` (linked): Compares identity-preserving duplicate retain/release deltas for actual original accessor children. Its raw-FF/zero input and missing string/provider refusal assertions are separate Rust contract controls, not observed in this ASCII native child-pointer probe.
- [rust/tcl-vm/src/value/native_property_name.rs](../../../../rust/tcl-vm/src/value/native_property_name.rs), `tests::original_property_name_duplicate_reuses_and_releases_accessor_children` (linked): Compares identity-preserving duplicate retain/release deltas for actual original accessor children. Its raw-FF/zero input and missing string/provider refusal assertions are separate Rust contract controls, not observed in this ASCII native child-pointer probe.

## Evidence

- [rust/tcl-registry/tests/data/native_property_names/probe.c](../../../../rust/tcl-registry/tests/data/native_property_names/probe.c)
- [rust/tcl-registry/tests/data/native_property_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_property_names/manifest.json)
- [rust/tcl-registry/tests/data/native_property_names/native.tsv](../../../../rust/tcl-registry/tests/data/native_property_names/native.tsv)
- [rust/tcl-registry/tests/data/native_property_names/capture.py](../../../../rust/tcl-registry/tests/data/native_property_names/capture.py)

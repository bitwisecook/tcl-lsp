# naming.property.original-enumeration-header-and-members

Kind: `native-observation`

## Problem statement

A property enumeration can return a cached List header whose child objects are separately shared. Copying the header is not equivalent to copying its members, and a consumer cannot infer header ownership from a rendered list. This check inspects the original result and its TclListObjCopy before any later mutation.

## Question

What header and child identity does C9.1 property enumeration retain across an original list-header copy?

## Conclusion

The first enumeration returns the exact readable-property cache header (identity1, refcount2); its first child has refcount3 and String primary. TclListObjCopy creates a different header while retaining the same first child (identity predicate1, copied-header refcount1, child refcount3). These two windows establish neither a stable cache after mutation nor general list-copy authority.

## Scope

Only the recorded C Tcl9.1.0 process linked to the hashed private-header build is observed. Other C releases, Jim and BIG-IP are not tested for this question. Native object/cache/reference snapshots are distinct from source-name bytes, command dispatch and current runtime ownership. An independently retained full11-row property-name probe also supplies the same earlier scoped windows; its additional reader/writer duplicate rows belong to naming.property.original-accessor-name-cache-children. Exact whole probe and full-stream hashes are retained independently, not substituted as a duplicate capture.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, jim

Status: `not-tested`.

No observation for this question.

### tcl9.1

Status: `observed`. Channel: ASCII source passed to Tcl_Eval; original property cache header and TclListObjCopy are inspected through matching private headers. Version: tcl9.1 9.1.0.

all-header: epoch32, result equals cached header, refs2, child refs3/String; copy-header: distinct header with same child, refs1, child refs3/String. Independent property-name probe rows0–1 exactly reproduce these earlier row bytes; its original full11-row input/stream remains separately retained.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_property_owners/manifest.json](../../../../rust/tcl-registry/tests/data/native_property_owners/manifest.json)
- [rust/tcl-registry/tests/data/native_property_owners/probe.c](../../../../rust/tcl-registry/tests/data/native_property_owners/probe.c)
- [rust/tcl-registry/tests/data/native_property_owners/native.tsv](../../../../rust/tcl-registry/tests/data/native_property_owners/native.tsv)
- [rust/tcl-registry/tests/data/native_property_names/probe.c](../../../../rust/tcl-registry/tests/data/native_property_names/probe.c)
- [rust/tcl-registry/tests/data/native_property_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_property_names/manifest.json)
- [rust/tcl-registry/tests/data/native_property_names/native.tsv](../../../../rust/tcl-registry/tests/data/native_property_names/native.tsv)

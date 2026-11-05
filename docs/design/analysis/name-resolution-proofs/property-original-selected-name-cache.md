# naming.property.original-selected-name-cache

Kind: `native-observation`

## Problem statement

An abbreviated caller property option and an enumerated original property-name member are separate objects. Applying a property-name cache to the caller because the bytes match would attach owner-dependent state to the wrong object. The check inspects the enumerated member after an actual configure -y lookup.

## Question

What primary cache does the original C9.1 enumerated property-name member acquire after management read?

## Conclusion

After o configure -y, the original first readable-property member has primary tcl::oo property name and refcount3 at epoch34. Only that original member is inspected; the record does not establish the caller option primary or a current owner outside this process.

## Scope

Only the recorded C Tcl9.1.0 process linked to the hashed private-header build is observed. Other C releases, Jim and BIG-IP are not tested for this question. Native object/cache/reference snapshots are distinct from source-name bytes, command dispatch and current runtime ownership. An independently retained full11-row property-name probe also supplies the same earlier scoped windows; its additional reader/writer duplicate rows belong to naming.property.original-accessor-name-cache-children. Exact whole probe and full-stream hashes are retained independently, not substituted as a duplicate capture.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, jim

Status: `not-tested`.

No observation for this question.

### tcl9.1

Status: `observed`. Channel: ASCII Tcl_Eval enumeration and abbreviated configure -y read; private cache inspection of the original enumerated child. Version: tcl9.1 9.1.0.

property-read: epoch34, first original member refs3, primary tcl::oo property name. Independent property-name probe rows7–7 exactly reproduce these earlier row bytes; its original full11-row input/stream remains separately retained.

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

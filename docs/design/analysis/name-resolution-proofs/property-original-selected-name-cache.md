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

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: ASCII Tcl_Eval enumeration and abbreviated configure -y read; private cache inspection of the original enumerated child.. Dialect: Tcl.

property-read: epoch34, first original member refs3, primary tcl::oo property name. Independent property-name probe rows7–7 exactly reproduce these earlier row bytes; its original full11-row input/stream remains separately retained.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `e0` (observation): [rust/tcl-registry/tests/data/native_property_owners/manifest.json](../../../../rust/tcl-registry/tests/data/native_property_owners/manifest.json). Exact successful private-header compile/run receipt and build/probe/executable hashes.
- `e1` (input): [rust/tcl-registry/tests/data/native_property_owners/probe.c](../../../../rust/tcl-registry/tests/data/native_property_owners/probe.c). Exact property enumeration, list-header copy, definition mutation and cache inspection order.
- `e2` (observation): [rust/tcl-registry/tests/data/native_property_owners/native.tsv](../../../../rust/tcl-registry/tests/data/native_property_owners/native.tsv). All eight exact rows; each question uses only its stated snapshot window.
- `names-supplement-probe.c` (input): [rust/tcl-registry/tests/data/native_property_names/probe.c](../../../../rust/tcl-registry/tests/data/native_property_names/probe.c). Independent full original11-row property-name probe; its earlier scoped rows are byte-equal to the current property-owner output, while later accessor-child duplicate windows remain separate.
- `names-supplement-manifest.json` (provider): [rust/tcl-registry/tests/data/native_property_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_property_names/manifest.json). Independent original compile/process/build/source metadata and full11-row captured stdout.
- `names-supplement-native.tsv` (observation): [rust/tcl-registry/tests/data/native_property_names/native.tsv](../../../../rust/tcl-registry/tests/data/native_property_names/native.tsv). Lines 8–8. Original full output; this question uses rows7–7 (zero-based), independently equal to the original property-owner controls.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cc",
  "-DSTDC_HEADERS=1",
  "-DHAVE_UNISTD_H=1",
  "-I${TCL_SOURCE}/generic",
  "-I${TCL_SOURCE}/unix",
  "rust/tcl-registry/tests/data/native_property_owners/probe.c",
  "${TCL_BUILD}/libtcl${TCL_ABI}.a",
  "-lm",
  "-ldl",
  "-lpthread",
  "-lz",
  "-o",
  "${PROBE_EXE}"
]
```

 Replay requires the matching recorded headers/library and original source bytes; compare process exit, stdout and stderr exactly before comparing projected rows. Private header/pointer observations are valid only within each original process and cannot authenticate a different interpreter. No Rust execution or fresh native reconfirmation is asserted.

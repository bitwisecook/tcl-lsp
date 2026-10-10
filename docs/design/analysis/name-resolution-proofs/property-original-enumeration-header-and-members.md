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

Status: `observed`. Version: 9.1.0. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: ASCII source passed to Tcl_Eval; original property cache header and TclListObjCopy are inspected through matching private headers.. Dialect: Tcl.

all-header: epoch32, result equals cached header, refs2, child refs3/String; copy-header: distinct header with same child, refs1, child refs3/String. Independent property-name probe rows0–1 exactly reproduce these earlier row bytes; its original full11-row input/stream remains separately retained.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (observation): [rust/tcl-registry/tests/data/native_property_owners/manifest.json](../../../../rust/tcl-registry/tests/data/native_property_owners/manifest.json). SHA-256 `447d2fe2ba34bc7acc4f6855ef8141de6cc93a4a10b72a10381fe280adcca6a9`. Exact successful private-header compile/run receipt and build/probe/executable hashes.
- `e1` (input): [rust/tcl-registry/tests/data/native_property_owners/probe.c](../../../../rust/tcl-registry/tests/data/native_property_owners/probe.c). SHA-256 `816d811eb93e055f1f5389071e5690fcc9ba6a09e7b65aa7eeecc4c77d44589b`. Exact property enumeration, list-header copy, definition mutation and cache inspection order.
- `e2` (observation): [rust/tcl-registry/tests/data/native_property_owners/native.tsv](../../../../rust/tcl-registry/tests/data/native_property_owners/native.tsv). SHA-256 `124315feba04c07a36ff10b961bf66642ccf4c2f05cf617baca2815e211b1ac0`. All eight exact rows; each question uses only its stated snapshot window.
- `names-supplement-probe.c` (input): [rust/tcl-registry/tests/data/native_property_names/probe.c](../../../../rust/tcl-registry/tests/data/native_property_names/probe.c). SHA-256 `d8d45219397efbb3ace20232576030f87b77d06b7f2ab9b9787283eecbd9c5e4`. Independent full original11-row property-name probe; its earlier scoped rows are byte-equal to the current property-owner output, while later accessor-child duplicate windows remain separate.
- `names-supplement-manifest.json` (provider): [rust/tcl-registry/tests/data/native_property_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_property_names/manifest.json). SHA-256 `62716a386cf1022111c59359b4783ea36a0206f66150dedb0be9ffccf3dde5b8`. Independent original compile/process/build/source metadata and full11-row captured stdout.
- `names-supplement-native.tsv` (observation): [rust/tcl-registry/tests/data/native_property_names/native.tsv](../../../../rust/tcl-registry/tests/data/native_property_names/native.tsv). SHA-256 `9e9b96d1fe13e4267e47425ae247b3355dc438fad86aa43c144f2b81b8bdbafd`. Lines 1–2. Original full output; this question uses rows0–1 (zero-based), independently equal to the original property-owner controls.

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

# naming.namespace-native.missing-parent-cache

Kind: `native-observation`

## Problem statement

A missing original namespace operand can retain an unresolved primary on one release while retaining no primary on another. A failed result alone cannot identify that original input cache state.

## Question

What result and original input primary remain after a fresh original ::missing parent query?

## Conclusion

All five C releases return code1 with their recorded missing-namespace message. C8.4 leaves an unresolved nsName input; later C leaves none. A failed unresolved descriptor does not prove namespace presence, a new allocation or a successful normal lookup.

## Scope

The original public C/Jim object-vector observer samples result primary before string materialisation, counted result bytes and original parent-input primary/resolved marker after the reached getter. C uses private namespace call frames to construct a: then q and to retain deleted ::Z; the source is retained exactly. Only the103-row permanent projection survives; the532-line full JSONL, its reference counts, duplicate-pointer observation and complete separate process streams are not attached. Version associations and original observer binary digests are retained; full launched patchlevel and C build closure are unrecorded. No return-options, arbitrary cache lifetime or BIG-IP claim.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; full launched patchlevel unqueried). Build: Original observer SHA256 c2596e165ed556a99ebe4eca81754e223c3618a59859bdc9127d1bf56174c284. Compiler/header/library/configure closure and process status are unrecorded in this receipt.. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs.. Dialect: C Tcl.

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
8.4.20|fresh-missing|1|string|756e6b6e6f776e206e616d65737061636520223a3a6d697373696e672220696e206e616d65737061636520706172656e7420636f6d6d616e64|nsName|false
```
These are the sampled fields only; absent full JSONL reference/pointer/process fields are not reconstructed.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; full launched patchlevel unqueried). Build: Original observer SHA256 65602d5cc8570b2e9c9f500d7c84e710188870661397587376a2f3e477c51dde. Compiler/header/library/configure closure and process status are unrecorded in this receipt.. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs.. Dialect: C Tcl.

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
8.5.19|fresh-missing|1|string|6e616d65737061636520223a3a6d697373696e6722206e6f7420666f756e64|none|false
```
These are the sampled fields only; absent full JSONL reference/pointer/process fields are not reconstructed.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; full launched patchlevel unqueried). Build: Original observer SHA256 211f1c1c697ac748cd57aed54d11ba34d2d8be2ef4062da77f8fad17b485ed78. Compiler/header/library/configure closure and process status are unrecorded in this receipt.. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs.. Dialect: C Tcl.

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
8.6.18|fresh-missing|1|string|6e616d65737061636520223a3a6d697373696e6722206e6f7420666f756e64|none|false
```
These are the sampled fields only; absent full JSONL reference/pointer/process fields are not reconstructed.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; full launched patchlevel unqueried). Build: Original observer SHA256 a67c5a796806d256ca7d8d6cef501acec8b9dfc0cd635e787993709e5f0ef2b4. Compiler/header/library/configure closure and process status are unrecorded in this receipt.. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs.. Dialect: C Tcl.

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
9.0.4|fresh-missing|1|string|6e616d65737061636520223a3a6d697373696e6722206e6f7420666f756e64|none|false
```
These are the sampled fields only; absent full JSONL reference/pointer/process fields are not reconstructed.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; full launched patchlevel unqueried). Build: Original observer SHA256 00c256b7557f76c3b011cdf19e40d72414eb422426fba647ef434bd998c6d1e4. Compiler/header/library/configure closure and process status are unrecorded in this receipt.. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs.. Dialect: C Tcl.

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
9.1.0|fresh-missing|1|string|6e616d65737061636520223a3a6d697373696e6722206e6f7420666f756e64|none|false
```
These are the sampled fields only; absent full JSONL reference/pointer/process fields are not reconstructed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No retained original getter/lifecycle row for this question is attached for this provider. A C-only API or probe sentinel is not an observed Jim/BIG-IP guest rejection.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No retained original getter/lifecycle row for this question is attached for this provider. A C-only API or probe sentinel is not an observed Jim/BIG-IP guest rejection.

## Exact evidence

- `probe` (input): [rust/tcl-syntax/tests/data/native_namespace_name/producer_getter.c](../../../../rust/tcl-syntax/tests/data/native_namespace_name/producer_getter.c). SHA-256 `8161c6ac39fb84c076e20a0f6cdef5f6ae5957d890443d99cda49aadd68a2b2c`. Exact original result and input primary/materialisation/lifecycle observer; not a native implementation excerpt.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_namespace_name/provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/provenance.json). SHA-256 `e78aa86921e4db5bedefd17b89c4fd76a608bbd59feee9447bbb6d0a9a0aa6f6`. Actual original version/observer/log associations; missing full logs remain explicit.
- `rows` (observation): [rust/tcl-syntax/tests/data/native_namespace_name/producer_getter.txt](../../../../rust/tcl-syntax/tests/data/native_namespace_name/producer_getter.txt). SHA-256 `e37094ea5691ea012f350a21b4d23d2d524a00351a61e10d1ed2b7fd8d6210a8`. 103 exact retained primary/result/input-marker projection rows; selected phases below.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-cmd-core/src/namespace.rs](../../../../rust/tcl-cmd-core/src/namespace.rs), `parent_original`: Consume the selected native original producer/getter input, actual namespace token and independent currentness; source display cannot manufacture it.
- [runtime/rust/src/interp/native_namespace_names/tests.rs](../../../../runtime/rust/src/interp/native_namespace_names/tests.rs), `interp::native_namespace_names::tests::original_namespace_objects_match_100_native_c_producer_and_getter_windows` (linked): Compares the100 actual C result/input-primary windows, including these named phases; independent allocation/lifetime assertions are not donated by the projected native rows.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact observer source remains attached, but the original full JSONL is unretained despite its provenance digest. Reconfirmation must compile this unchanged observer against explicitly selected matching native public/private headers and libraries, record actual version/build and complete process/stream outputs, and project only the named result/input fields. This existing projected capture supplies no unretained reference-count/pointer proof, new native run or Rust result.

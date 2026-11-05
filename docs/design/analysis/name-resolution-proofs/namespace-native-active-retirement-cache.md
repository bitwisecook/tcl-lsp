# naming.namespace-native.active-retirement-cache

Kind: `native-observation`

## Problem statement

A deleted namespace can still have an active frame. Retaining its namespace-name cache through that frame or immediately declining the original lookup produces release-specific outcomes, independently of a future same-name table.

## Question

What does the original ::Z parent input resolve before deletion, during the deleted namespace's active frame, and after the frame pops?

## Conclusion

All C releases first resolve ::Z to parent ::. During active retirement C8.4 still returns an empty successful parent result with resolved nsName input; C8.5 and later reject with unresolved none input. After frame pop every release rejects; C8.4 samples unresolved nsName and later releases none. No frame or object currency is inferred beyond these exact windows.

## Scope

The original public C/Jim object-vector observer samples result primary before string materialisation, counted result bytes and original parent-input primary/resolved marker after the reached getter. C uses private namespace call frames to construct a: then q and to retain deleted ::Z; the source is retained exactly. Only the103-row permanent projection survives; the532-line full JSONL, its reference counts, duplicate-pointer observation and complete separate process streams are not attached. Version associations and original observer binary digests are retained; full launched patchlevel and C build closure are unrecorded. No return-options, arbitrary cache lifetime or BIG-IP claim.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs. Version: tcl8.4 8.4.20 (capture association; full launched patchlevel unqueried).

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
8.4.20|active-retire-before|0|none|3a3a|nsName|true
8.4.20|active-retire-cached|0|none||nsName|true
8.4.20|active-retire-after-pop|1|string|756e6b6e6f776e206e616d65737061636520223a3a5a2220696e206e616d65737061636520706172656e7420636f6d6d616e64|nsName|false
```
These are the sampled fields only; absent full JSONL reference/pointer/process fields are not reconstructed.

### tcl8.5

Status: `observed`. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs. Version: tcl8.5 8.5.19 (capture association; full launched patchlevel unqueried).

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
8.5.19|active-retire-before|0|none|3a3a|nsName|true
8.5.19|active-retire-cached|1|string|6e616d65737061636520223a3a5a22206e6f7420666f756e64|none|false
8.5.19|active-retire-after-pop|1|string|6e616d65737061636520223a3a5a22206e6f7420666f756e64|none|false
```
These are the sampled fields only; absent full JSONL reference/pointer/process fields are not reconstructed.

### tcl8.6

Status: `observed`. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs. Version: tcl8.6 8.6.18 (capture association; full launched patchlevel unqueried).

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
8.6.18|active-retire-before|0|none|3a3a|nsName|true
8.6.18|active-retire-cached|1|string|6e616d65737061636520223a3a5a22206e6f7420666f756e64|none|false
8.6.18|active-retire-after-pop|1|string|6e616d65737061636520223a3a5a22206e6f7420666f756e64|none|false
```
These are the sampled fields only; absent full JSONL reference/pointer/process fields are not reconstructed.

### tcl9.0

Status: `observed`. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs. Version: tcl9.0 9.0.4 (capture association; full launched patchlevel unqueried).

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
9.0.4|active-retire-before|0|nsName|3a3a|nsName|true
9.0.4|active-retire-cached|1|string|6e616d65737061636520223a3a5a22206e6f7420666f756e64|none|false
9.0.4|active-retire-after-pop|1|string|6e616d65737061636520223a3a5a22206e6f7420666f756e64|none|false
```
These are the sampled fields only; absent full JSONL reference/pointer/process fields are not reconstructed.

### tcl9.1

Status: `observed`. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs. Version: tcl9.1 9.1.0 (capture association; full launched patchlevel unqueried).

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
9.1.0|active-retire-before|0|nsName|3a3a|nsName|true
9.1.0|active-retire-cached|1|string|6e616d65737061636520223a3a5a22206e6f7420666f756e64|none|false
9.1.0|active-retire-after-pop|1|string|6e616d65737061636520223a3a5a22206e6f7420666f756e64|none|false
```
These are the sampled fields only; absent full JSONL reference/pointer/process fields are not reconstructed.

### jim, bigip

Status: `not-tested`.

No retained original getter/lifecycle row for this question is attached for this provider. A C-only API or probe sentinel is not an observed Jim/BIG-IP guest rejection.

## Consumer bindings

- [rust/tcl-cmd-core/src/namespace.rs](../../../../rust/tcl-cmd-core/src/namespace.rs), `parent_original`: Consume the selected native original producer/getter input, actual namespace token and independent currentness; source display cannot manufacture it.
- [runtime/rust/src/interp/native_namespace_names/tests.rs](../../../../runtime/rust/src/interp/native_namespace_names/tests.rs), `interp::native_namespace_names::tests::original_namespace_objects_match_100_native_c_producer_and_getter_windows` (linked): Compares the100 actual C result/input-primary windows, including these named phases; independent allocation/lifetime assertions are not donated by the projected native rows.

## Evidence

- [rust/tcl-syntax/tests/data/native_namespace_name/producer_getter.c](../../../../rust/tcl-syntax/tests/data/native_namespace_name/producer_getter.c)
- [rust/tcl-syntax/tests/data/native_namespace_name/provenance.json](../../../../rust/tcl-syntax/tests/data/native_namespace_name/provenance.json)
- [rust/tcl-syntax/tests/data/native_namespace_name/producer_getter.txt](../../../../rust/tcl-syntax/tests/data/native_namespace_name/producer_getter.txt)

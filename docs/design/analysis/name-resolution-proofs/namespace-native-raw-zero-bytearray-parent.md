# naming.namespace-native.raw-zero-bytearray-parent

Kind: `native-observation`

## Problem statement

Counted raw-zero String, modified-zero String and pure ByteArray inputs have different materialisation paths. Treating equal semantic characters as equal namespace input bytes predicts the wrong selected table before a modified-zero namespace exists.

## Question

Which parent outcomes and original input primaries follow raw-zero, C080 and ByteArray namespace names before and after creating the C080-named namespace?

## Conclusion

Every C release resolves raw-zero ::a-zero-z to the existing ::a parent. C080 and ByteArray forms fail before the separate C080-named namespace is created, then both succeed with :: parent bytes after creation. C8.4 failed forms sample unresolved nsName; later C preserves none for failed C080 String and bytearray for failed ByteArray. These are distinct original conversion/lookup paths, not a universal zero-byte key equivalence.

## Scope

The original public C/Jim object-vector observer samples result primary before string materialisation, counted result bytes and original parent-input primary/resolved marker after the reached getter. C uses private namespace call frames to construct a: then q and to retain deleted ::Z; the source is retained exactly. Only the103-row permanent projection survives; the532-line full JSONL, its reference counts, duplicate-pointer observation and complete separate process streams are not attached. Version associations and original observer binary digests are retained; full launched patchlevel and C build closure are unrecorded. No return-options, arbitrary cache lifetime or BIG-IP claim.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs. Version: tcl8.4 8.4.20 (capture association; full launched patchlevel unqueried).

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
8.4.20|raw-NUL-parent|0|none|3a3a|nsName|true
8.4.20|modified-NUL-negative-parent|1|string|756e6b6e6f776e206e616d65737061636520223a3a61c0807a2220696e206e616d65737061636520706172656e7420636f6d6d616e64|nsName|false
8.4.20|pure-ByteArray-negative-parent|1|string|756e6b6e6f776e206e616d65737061636520223a3a61c0807a2220696e206e616d65737061636520706172656e7420636f6d6d616e64|nsName|false
8.4.20|modified-NUL-positive-parent|0|none|3a3a|nsName|true
8.4.20|pure-ByteArray-positive-parent|0|none|3a3a|nsName|true
```
These are the sampled fields only; absent full JSONL reference/pointer/process fields are not reconstructed.

### tcl8.5

Status: `observed`. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs. Version: tcl8.5 8.5.19 (capture association; full launched patchlevel unqueried).

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
8.5.19|raw-NUL-parent|0|none|3a3a|nsName|true
8.5.19|modified-NUL-negative-parent|1|string|6e616d65737061636520223a3a61c0807a22206e6f7420666f756e64|none|false
8.5.19|pure-ByteArray-negative-parent|1|string|6e616d65737061636520223a3a61c0807a22206e6f7420666f756e64|bytearray|false
8.5.19|modified-NUL-positive-parent|0|none|3a3a|nsName|true
8.5.19|pure-ByteArray-positive-parent|0|none|3a3a|nsName|true
```
These are the sampled fields only; absent full JSONL reference/pointer/process fields are not reconstructed.

### tcl8.6

Status: `observed`. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs. Version: tcl8.6 8.6.18 (capture association; full launched patchlevel unqueried).

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
8.6.18|raw-NUL-parent|0|none|3a3a|nsName|true
8.6.18|modified-NUL-negative-parent|1|string|6e616d65737061636520223a3a61c0807a22206e6f7420666f756e64|none|false
8.6.18|pure-ByteArray-negative-parent|1|string|6e616d65737061636520223a3a61c0807a22206e6f7420666f756e64|bytearray|false
8.6.18|modified-NUL-positive-parent|0|none|3a3a|nsName|true
8.6.18|pure-ByteArray-positive-parent|0|none|3a3a|nsName|true
```
These are the sampled fields only; absent full JSONL reference/pointer/process fields are not reconstructed.

### tcl9.0

Status: `observed`. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs. Version: tcl9.0 9.0.4 (capture association; full launched patchlevel unqueried).

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
9.0.4|raw-NUL-parent|0|nsName|3a3a|nsName|true
9.0.4|modified-NUL-negative-parent|1|string|6e616d65737061636520223a3a61c0807a22206e6f7420666f756e64|none|false
9.0.4|pure-ByteArray-negative-parent|1|string|6e616d65737061636520223a3a61c0807a22206e6f7420666f756e64|bytearray|false
9.0.4|modified-NUL-positive-parent|0|nsName|3a3a|nsName|true
9.0.4|pure-ByteArray-positive-parent|0|nsName|3a3a|nsName|true
```
These are the sampled fields only; absent full JSONL reference/pointer/process fields are not reconstructed.

### tcl9.1

Status: `observed`. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs. Version: tcl9.1 9.1.0 (capture association; full launched patchlevel unqueried).

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
9.1.0|raw-NUL-parent|0|nsName|3a3a|nsName|true
9.1.0|modified-NUL-negative-parent|1|string|6e616d65737061636520223a3a61c0807a22206e6f7420666f756e64|none|false
9.1.0|pure-ByteArray-negative-parent|1|string|6e616d65737061636520223a3a61c0807a22206e6f7420666f756e64|bytearray|false
9.1.0|modified-NUL-positive-parent|0|nsName|3a3a|nsName|true
9.1.0|pure-ByteArray-positive-parent|0|nsName|3a3a|nsName|true
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

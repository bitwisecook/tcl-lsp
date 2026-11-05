# naming.namespace-native.deleted-current-object-recreation

Kind: `native-observation`

## Problem statement

Deleting a namespace and recreating a same-display child could accidentally refresh an old produced object or duplicate. The original result object must not acquire a new incarnation merely because its spelling matches.

## Question

Do the retained current object or duplicate resolve after child deletion, and does the old current object resolve after same-name recreation?

## Conclusion

All five C projections report code1 for the old current object and duplicate after deletion and for the old current object after recreation. C8.4 samples unresolved nsName inputs; later releases sample none inputs. No old object is observed to regain a resolved descriptor from the recreated child.

## Scope

The original public C/Jim object-vector observer samples result primary before string materialisation, counted result bytes and original parent-input primary/resolved marker after the reached getter. C uses private namespace call frames to construct a: then q and to retain deleted ::Z; the source is retained exactly. Only the103-row permanent projection survives; the532-line full JSONL, its reference counts, duplicate-pointer observation and complete separate process streams are not attached. Version associations and original observer binary digests are retained; full launched patchlevel and C build closure are unrecorded. No return-options, arbitrary cache lifetime or BIG-IP claim.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs. Version: tcl8.4 8.4.20 (capture association; full launched patchlevel unqueried).

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
8.4.20|deleted-current-object-parent|1|string|756e6b6e6f776e206e616d65737061636520223a3a613a3a3a712220696e206e616d65737061636520706172656e7420636f6d6d616e64|nsName|false
8.4.20|deleted-duplicate-parent|1|string|756e6b6e6f776e206e616d65737061636520223a3a613a3a3a712220696e206e616d65737061636520706172656e7420636f6d6d616e64|nsName|false
8.4.20|recreated-current-object-parent|1|string|756e6b6e6f776e206e616d65737061636520223a3a613a3a3a712220696e206e616d65737061636520706172656e7420636f6d6d616e64|nsName|false
```
These are the sampled fields only; absent full JSONL reference/pointer/process fields are not reconstructed.

### tcl8.5

Status: `observed`. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs. Version: tcl8.5 8.5.19 (capture association; full launched patchlevel unqueried).

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
8.5.19|deleted-current-object-parent|1|string|6e616d65737061636520223a3a613a3a3a7122206e6f7420666f756e64|none|false
8.5.19|deleted-duplicate-parent|1|string|6e616d65737061636520223a3a613a3a3a7122206e6f7420666f756e64|none|false
8.5.19|recreated-current-object-parent|1|string|6e616d65737061636520223a3a613a3a3a7122206e6f7420666f756e64|none|false
```
These are the sampled fields only; absent full JSONL reference/pointer/process fields are not reconstructed.

### tcl8.6

Status: `observed`. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs. Version: tcl8.6 8.6.18 (capture association; full launched patchlevel unqueried).

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
8.6.18|deleted-current-object-parent|1|string|6e616d65737061636520223a3a613a3a3a7122206e6f7420666f756e64|none|false
8.6.18|deleted-duplicate-parent|1|string|6e616d65737061636520223a3a613a3a3a7122206e6f7420666f756e64|none|false
8.6.18|recreated-current-object-parent|1|string|6e616d65737061636520223a3a613a3a3a7122206e6f7420666f756e64|none|false
```
These are the sampled fields only; absent full JSONL reference/pointer/process fields are not reconstructed.

### tcl9.0

Status: `observed`. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs. Version: tcl9.0 9.0.4 (capture association; full launched patchlevel unqueried).

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
9.0.4|deleted-current-object-parent|1|string|6e616d65737061636520223a3a613a3a3a7122206e6f7420666f756e64|none|false
9.0.4|deleted-duplicate-parent|1|string|6e616d65737061636520223a3a613a3a3a7122206e6f7420666f756e64|none|false
9.0.4|recreated-current-object-parent|1|string|6e616d65737061636520223a3a613a3a3a7122206e6f7420666f756e64|none|false
```
These are the sampled fields only; absent full JSONL reference/pointer/process fields are not reconstructed.

### tcl9.1

Status: `observed`. Channel: Original public object-vector invocation; counted result getter and private primary/resolved marker. C producer/lifecycle cases additionally use native namespace and call-frame APIs. Version: tcl9.1 9.1.0 (capture association; full launched patchlevel unqueried).

Selected retained release/phase/code/result-primary/result-hex/input-primary/resolved rows:

```text
9.1.0|deleted-current-object-parent|1|string|6e616d65737061636520223a3a613a3a3a7122206e6f7420666f756e64|none|false
9.1.0|deleted-duplicate-parent|1|string|6e616d65737061636520223a3a613a3a3a7122206e6f7420666f756e64|none|false
9.1.0|recreated-current-object-parent|1|string|6e616d65737061636520223a3a613a3a3a7122206e6f7420666f756e64|none|false
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

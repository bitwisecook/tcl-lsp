# naming.variable-table.original-array-growth-and-reinsertion

Kind: `native-observation`

## Problem statement

Insertion count and delete/reinsert chronology affect bucket growth and enumeration order. A sorted-name or fresh-table reconstruction can silently lose the actual retained chains.

## Question

What exact table growth and enumeration results follow16 array sizes, ordinary/opaque keys and a k03 delete/reinsert?

## Conclusion

Across the five C releases, physical array buckets grow4→16 at12entries,16→64 at48,64→256 at192 and256→1024 at768; enumeration follows the exact retained64 array-names controls per release rather than sorted order. Deleting/reinserting k03 preserves grown capacity; at size3 it adds the previously absent fourth key. All names/get/search C calls complete normally. Jim names/get likewise complete, with actual insertion-vector order and the later inserted k03 at its new position; array startsearch is an observed unsupported command, so no Jim search order is inferred. Physical Jim metadata is the global vars table, not an unprinted array hash.

## Scope

One exact native probe uses fresh interpreters per case, counted original String/List object-vector array set and public script-object evaluation for query/trace procedures. Its opaque inputs are rawzero/FF/modifiedzero byte strings, not document source literals. Results are retained before rendering; no error-global observer. C private fields measure real global/array tables; Jim table metadata is its global vars hash only. All1788 original JSONL rows are retained and whole-stream/source/header/library hashes checked. Launch patchlevel/Jim revision/configuration, compiler version and executable hashes are not recorded. Missing additional-artifact hashes in the manifest do not reconstruct absent artifacts; no runtime comparison or Rust execution claim.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer. Version: tcl8.4 8.4.20.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"array-3-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":4,"array_entries":3}
{"case":"array-3-opaque0","op":"physical-table","global_buckets":16,"global_entries":12,"array_buckets":4,"array_entries":4}
{"case":"array-11-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":4,"array_entries":11}
{"case":"array-11-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":4,"array_entries":11}
{"case":"array-12-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":12}
{"case":"array-12-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":12}
{"case":"array-13-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":13}
{"case":"array-13-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":13}
{"case":"array-15-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":15}
{"case":"array-15-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":15}
{"case":"array-16-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":16}
{"case":"array-16-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":16}
{"case":"array-17-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":17}
{"case":"array-17-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":17}
{"case":"array-47-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":47}
{"case":"array-47-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":47}
{"case":"array-48-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":64,"array_entries":48}
{"case":"array-48-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":64,"array_entries":48}
{"case":"array-49-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":64,"array_entries":49}
{"case":"array-49-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":64,"array_entries":49}
{"case":"array-191-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":64,"array_entries":191}
{"case":"array-191-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":64,"array_entries":191}
{"case":"array-192-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":256,"array_entries":192}
{"case":"array-192-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":256,"array_entries":192}
{"case":"array-193-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":256,"array_entries":193}
{"case":"array-193-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":256,"array_entries":193}
{"case":"array-767-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":256,"array_entries":767}
{"case":"array-767-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":256,"array_entries":767}
{"case":"array-768-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":1024,"array_entries":768}
{"case":"array-768-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":1024,"array_entries":768}
{"case":"array-769-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":1024,"array_entries":769}
{"case":"array-769-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":1024,"array_entries":769}
{"case":"array-3-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":4,"array_entries":3}
{"case":"array-3-opaque1","op":"physical-table","global_buckets":16,"global_entries":12,"array_buckets":4,"array_entries":4}
{"case":"array-11-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":4,"array_entries":11}
{"case":"array-11-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":4,"array_entries":11}
{"case":"array-12-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":12}
{"case":"array-12-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":12}
{"case":"array-13-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":13}
{"case":"array-13-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":13}
{"case":"array-15-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":15}
{"case":"array-15-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":15}
{"case":"array-16-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":16}
{"case":"array-16-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":16}
{"case":"array-17-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":17}
{"case":"array-17-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":17}
{"case":"array-47-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":47}
{"case":"array-47-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":47}
{"case":"array-48-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":64,"array_entries":48}
{"case":"array-48-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":64,"array_entries":48}
{"case":"array-49-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":64,"array_entries":49}
{"case":"array-49-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":64,"array_entries":49}
{"case":"array-191-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":64,"array_entries":191}
{"case":"array-191-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":64,"array_entries":191}
{"case":"array-192-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":256,"array_entries":192}
{"case":"array-192-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":256,"array_entries":192}
{"case":"array-193-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":256,"array_entries":193}
{"case":"array-193-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":256,"array_entries":193}
{"case":"array-767-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":256,"array_entries":767}
{"case":"array-767-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":256,"array_entries":767}
{"case":"array-768-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":1024,"array_entries":768}
{"case":"array-768-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":1024,"array_entries":768}
{"case":"array-769-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":1024,"array_entries":769}
{"case":"array-769-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":1024,"array_entries":769}
```

### tcl8.5, tcl8.6

Status: `observed`. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer. Version: tcl8.5 8.5.19; tcl8.6 8.6.18.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"array-3-opaque0","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":4,"array_entries":3}
{"case":"array-3-opaque0","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":4,"array_entries":4}
{"case":"array-11-opaque0","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":4,"array_entries":11}
{"case":"array-11-opaque0","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":4,"array_entries":11}
{"case":"array-12-opaque0","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":16,"array_entries":12}
{"case":"array-12-opaque0","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":16,"array_entries":12}
{"case":"array-13-opaque0","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":16,"array_entries":13}
{"case":"array-13-opaque0","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":16,"array_entries":13}
{"case":"array-15-opaque0","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":16,"array_entries":15}
{"case":"array-15-opaque0","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":16,"array_entries":15}
{"case":"array-16-opaque0","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":16,"array_entries":16}
{"case":"array-16-opaque0","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":16,"array_entries":16}
{"case":"array-17-opaque0","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":16,"array_entries":17}
{"case":"array-17-opaque0","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":16,"array_entries":17}
{"case":"array-47-opaque0","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":16,"array_entries":47}
{"case":"array-47-opaque0","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":16,"array_entries":47}
{"case":"array-48-opaque0","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":64,"array_entries":48}
{"case":"array-48-opaque0","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":64,"array_entries":48}
{"case":"array-49-opaque0","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":64,"array_entries":49}
{"case":"array-49-opaque0","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":64,"array_entries":49}
{"case":"array-191-opaque0","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":64,"array_entries":191}
{"case":"array-191-opaque0","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":64,"array_entries":191}
{"case":"array-192-opaque0","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":256,"array_entries":192}
{"case":"array-192-opaque0","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":256,"array_entries":192}
{"case":"array-193-opaque0","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":256,"array_entries":193}
{"case":"array-193-opaque0","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":256,"array_entries":193}
{"case":"array-767-opaque0","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":256,"array_entries":767}
{"case":"array-767-opaque0","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":256,"array_entries":767}
{"case":"array-768-opaque0","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":1024,"array_entries":768}
{"case":"array-768-opaque0","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":1024,"array_entries":768}
{"case":"array-769-opaque0","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":1024,"array_entries":769}
{"case":"array-769-opaque0","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":1024,"array_entries":769}
{"case":"array-3-opaque1","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":4,"array_entries":3}
{"case":"array-3-opaque1","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":4,"array_entries":4}
{"case":"array-11-opaque1","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":4,"array_entries":11}
{"case":"array-11-opaque1","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":4,"array_entries":11}
{"case":"array-12-opaque1","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":16,"array_entries":12}
{"case":"array-12-opaque1","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":16,"array_entries":12}
{"case":"array-13-opaque1","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":16,"array_entries":13}
{"case":"array-13-opaque1","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":16,"array_entries":13}
{"case":"array-15-opaque1","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":16,"array_entries":15}
{"case":"array-15-opaque1","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":16,"array_entries":15}
{"case":"array-16-opaque1","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":16,"array_entries":16}
{"case":"array-16-opaque1","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":16,"array_entries":16}
{"case":"array-17-opaque1","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":16,"array_entries":17}
{"case":"array-17-opaque1","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":16,"array_entries":17}
{"case":"array-47-opaque1","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":16,"array_entries":47}
{"case":"array-47-opaque1","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":16,"array_entries":47}
{"case":"array-48-opaque1","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":64,"array_entries":48}
{"case":"array-48-opaque1","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":64,"array_entries":48}
{"case":"array-49-opaque1","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":64,"array_entries":49}
{"case":"array-49-opaque1","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":64,"array_entries":49}
{"case":"array-191-opaque1","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":64,"array_entries":191}
{"case":"array-191-opaque1","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":64,"array_entries":191}
{"case":"array-192-opaque1","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":256,"array_entries":192}
{"case":"array-192-opaque1","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":256,"array_entries":192}
{"case":"array-193-opaque1","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":256,"array_entries":193}
{"case":"array-193-opaque1","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":256,"array_entries":193}
{"case":"array-767-opaque1","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":256,"array_entries":767}
{"case":"array-767-opaque1","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":256,"array_entries":767}
{"case":"array-768-opaque1","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":1024,"array_entries":768}
{"case":"array-768-opaque1","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":1024,"array_entries":768}
{"case":"array-769-opaque1","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":1024,"array_entries":769}
{"case":"array-769-opaque1","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":1024,"array_entries":769}
```

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"array-3-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":4,"array_entries":3}
{"case":"array-3-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":4,"array_entries":4}
{"case":"array-11-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":4,"array_entries":11}
{"case":"array-11-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":4,"array_entries":11}
{"case":"array-12-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":12}
{"case":"array-12-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":12}
{"case":"array-13-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":13}
{"case":"array-13-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":13}
{"case":"array-15-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":15}
{"case":"array-15-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":15}
{"case":"array-16-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":16}
{"case":"array-16-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":16}
{"case":"array-17-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":17}
{"case":"array-17-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":17}
{"case":"array-47-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":47}
{"case":"array-47-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":47}
{"case":"array-48-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":64,"array_entries":48}
{"case":"array-48-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":64,"array_entries":48}
{"case":"array-49-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":64,"array_entries":49}
{"case":"array-49-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":64,"array_entries":49}
{"case":"array-191-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":64,"array_entries":191}
{"case":"array-191-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":64,"array_entries":191}
{"case":"array-192-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":256,"array_entries":192}
{"case":"array-192-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":256,"array_entries":192}
{"case":"array-193-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":256,"array_entries":193}
{"case":"array-193-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":256,"array_entries":193}
{"case":"array-767-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":256,"array_entries":767}
{"case":"array-767-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":256,"array_entries":767}
{"case":"array-768-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":1024,"array_entries":768}
{"case":"array-768-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":1024,"array_entries":768}
{"case":"array-769-opaque0","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":1024,"array_entries":769}
{"case":"array-769-opaque0","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":1024,"array_entries":769}
{"case":"array-3-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":4,"array_entries":3}
{"case":"array-3-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":4,"array_entries":4}
{"case":"array-11-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":4,"array_entries":11}
{"case":"array-11-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":4,"array_entries":11}
{"case":"array-12-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":12}
{"case":"array-12-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":12}
{"case":"array-13-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":13}
{"case":"array-13-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":13}
{"case":"array-15-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":15}
{"case":"array-15-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":15}
{"case":"array-16-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":16}
{"case":"array-16-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":16}
{"case":"array-17-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":17}
{"case":"array-17-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":17}
{"case":"array-47-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":47}
{"case":"array-47-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":47}
{"case":"array-48-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":64,"array_entries":48}
{"case":"array-48-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":64,"array_entries":48}
{"case":"array-49-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":64,"array_entries":49}
{"case":"array-49-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":64,"array_entries":49}
{"case":"array-191-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":64,"array_entries":191}
{"case":"array-191-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":64,"array_entries":191}
{"case":"array-192-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":256,"array_entries":192}
{"case":"array-192-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":256,"array_entries":192}
{"case":"array-193-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":256,"array_entries":193}
{"case":"array-193-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":256,"array_entries":193}
{"case":"array-767-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":256,"array_entries":767}
{"case":"array-767-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":256,"array_entries":767}
{"case":"array-768-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":1024,"array_entries":768}
{"case":"array-768-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":1024,"array_entries":768}
{"case":"array-769-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":1024,"array_entries":769}
{"case":"array-769-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":1024,"array_entries":769}
```

### jim

Status: `observed`. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer. Version: jim Jim capture.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"array-3-opaque0","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-3-opaque0","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-11-opaque0","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-11-opaque0","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-12-opaque0","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-12-opaque0","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-13-opaque0","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-13-opaque0","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-15-opaque0","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-15-opaque0","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-16-opaque0","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-16-opaque0","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-17-opaque0","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-17-opaque0","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-47-opaque0","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-47-opaque0","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-48-opaque0","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-48-opaque0","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-49-opaque0","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-49-opaque0","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-191-opaque0","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-191-opaque0","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-192-opaque0","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-192-opaque0","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-193-opaque0","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-193-opaque0","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-767-opaque0","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-767-opaque0","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-768-opaque0","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-768-opaque0","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-769-opaque0","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-769-opaque0","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-3-opaque1","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-3-opaque1","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-11-opaque1","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-11-opaque1","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-12-opaque1","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-12-opaque1","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-13-opaque1","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-13-opaque1","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-15-opaque1","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-15-opaque1","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-16-opaque1","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-16-opaque1","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-17-opaque1","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-17-opaque1","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-47-opaque1","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-47-opaque1","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-48-opaque1","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-48-opaque1","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-49-opaque1","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-49-opaque1","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-191-opaque1","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-191-opaque1","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-192-opaque1","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-192-opaque1","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-193-opaque1","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-193-opaque1","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-767-opaque1","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-767-opaque1","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-768-opaque1","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-768-opaque1","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-769-opaque1","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-769-opaque1","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
```

### bigip

Status: `not-tested`.

No original observation for this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-registry/src/native_variable_table.rs](../../../../rust/tcl-registry/src/native_variable_table.rs), `native_variable_table_protocol`: Selects table/name semantics only with an independently supplied actual ABI.
- [rust/tcl-core-types/src/native_hash_order.rs](../../../../rust/tcl-core-types/src/native_hash_order.rs), `NativeEntryLedger`: Keeps original entry chronology and optional selected hash order separately from defined inventory.
- [rust/tcl-core-types/src/native_hash_order.rs](../../../../rust/tcl-core-types/src/native_hash_order.rs), `native_hash_order::tests::native_c_array_order_retains_original_keys_growth_and_reinsertion` (linked): Checks all320 exact C array-order controls against independently selected arithmetic; it does not test Jim search, global metadata or every attached callback case.
- [runtime/rust/src/frame.rs](../../../../runtime/rust/src/frame.rs), `frame::native_inventory_tests::array_entry_ledgers_match_all_320_native_table_controls` (linked): Compares current element ledgers to320 original C order controls using selected native key extent and hash recipe; real native launches are not performed by this test.

## Evidence

- [rust/tcl-core-types/tests/data/native_variable_tables](../../../../rust/tcl-core-types/tests/data/native_variable_tables)

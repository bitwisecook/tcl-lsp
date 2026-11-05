# naming.list-index.original-list-path-and-invalid-conversion

Kind: `native-observation`

## Problem statement

A multi-index path or rejected scalar can retain a List/none/end-offset primary differently across releases. Error paths can leave extra original references without giving a successful selected member.

## Question

What original index/result header and completion follows0 1,bad,unfinished brace and1+1 runtime operands?

## Conclusion

The original0 1 path becomes List with resident bytes/refs1 and returns C at result refs2 in every capture. bad leaves original List; unfinished brace leaves untyped original and returns the exact bad-index error. Both error operands end atrefs1 in C8.4/8.5 andrefs2 in C8.6–9.1. C8.4 rejects1+1 with List primary; C8.5/8.6 accept it with untyped primary and empty result; C9 accepts it with end-offset primary and empty result. These before-result-getter windows do not expose intermediate private list backing copies or prove a getter never ran.

## Scope

Selected input IDs [3, 4, 5, 8] from nine exact original String index constructors on each C release,45 windows. Probe defines p{x i}{lindex$x$i} through public NUL-terminated source evaluation then passes original retained String x/index headers via Tcl_EvalObjv(TCL_EVAL_GLOBAL). It samples original index and result primary/resident/refs before result materialization. Full source/input/stdout/lib/exe/process metadata is retained; compile status, compiler version, header configuration and actual launched patchlevel query are not recorded by this manifest. No intermediate backing identity, end-updater callback or Jim/BIG-IP attempt.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Public object-vector invocation with original native index String object; pre-render header snapshots. Version: tcl8.4 8.4.20.

Exact selected rows in the manifest9-column order: case/guest code/original index primary,resident,refs/result primary,resident,refs/result hex.

```text
3|0|list|1|1|none|1|2|43
4|1|list|1|1|string|1|1|62616420696e6465782022626164223a206d75737420626520696e7465676572206f7220656e643f2d696e74656765723f
5|1|none|1|1|string|1|1|62616420696e64657820227b223a206d75737420626520696e7465676572206f7220656e643f2d696e74656765723f
8|1|list|1|1|string|1|1|62616420696e6465782022312b31223a206d75737420626520696e7465676572206f7220656e643f2d696e74656765723f
```

### tcl8.5

Status: `observed`. Channel: Public object-vector invocation with original native index String object; pre-render header snapshots. Version: tcl8.5 8.5.19.

Exact selected rows in the manifest9-column order: case/guest code/original index primary,resident,refs/result primary,resident,refs/result hex.

```text
3|0|list|1|1|none|1|2|43
4|1|list|1|1|string|1|1|62616420696e6465782022626164223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f
5|1|none|1|1|string|1|1|62616420696e64657820227b223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f
8|0|none|1|1|none|1|1|
```

### tcl8.6

Status: `observed`. Channel: Public object-vector invocation with original native index String object; pre-render header snapshots. Version: tcl8.6 8.6.18.

Exact selected rows in the manifest9-column order: case/guest code/original index primary,resident,refs/result primary,resident,refs/result hex.

```text
3|0|list|1|1|none|1|2|43
4|1|list|1|2|string|1|1|62616420696e6465782022626164223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f
5|1|none|1|2|string|1|1|62616420696e64657820227b223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f
8|0|none|1|1|none|1|1|
```

### tcl9.0, tcl9.1

Status: `observed`. Channel: Public object-vector invocation with original native index String object; pre-render header snapshots. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Exact selected rows in the manifest9-column order: case/guest code/original index primary,resident,refs/result primary,resident,refs/result hex.

```text
3|0|list|1|1|none|1|2|43
4|1|list|1|2|string|1|1|62616420696e6465782022626164223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f
5|1|none|1|2|string|1|1|62616420696e64657820227b223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f
8|0|end-offset|1|1|none|1|1|
```

### jim, bigip

Status: `not-tested`.

No original observation for this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-registry/src/native_list_index_compilation.rs](../../../../rust/tcl-registry/src/native_list_index_compilation.rs), `NativeListIndexOperation`: Keeps runtime index object conversion separate from literal compiler coordinates.
- [rust/tcl-vm/src/exec/native_list_index_tests.rs](../../../../rust/tcl-vm/src/exec/native_list_index_tests.rs), `exec::native_list_index_tests::original_objects::original_index_getters_match_45_native_header_and_result_windows` (linked): Compares45 attached original index/result primary/resident/ref and guest windows; no intermediate native backing allocation assertion is borrowed.
- [runtime/rust/src/interp/native_body_artifact/native_list_index.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_list_index.rs), `interp::native_body_artifact::native_list_index::original_objects::original_index_getters_match_45_native_header_and_result_windows` (linked): Runtime checks45 original result/index header windows under the selected release; source/native currentness remains independent.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_list_index_original_objects/probe.c](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/probe.c)
- [rust/tcl-registry/tests/data/native_list_index_original_objects/inputs.json](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/inputs.json)
- [rust/tcl-registry/tests/data/native_list_index_original_objects/manifest.json](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/manifest.json)
- [rust/tcl-registry/tests/data/native_list_index_original_objects/8.4.20.txt](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/8.4.20.txt)
- [rust/tcl-registry/tests/data/native_list_index_original_objects/8.5.19.txt](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/8.5.19.txt)
- [rust/tcl-registry/tests/data/native_list_index_original_objects/8.6.18.txt](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/8.6.18.txt)
- [rust/tcl-registry/tests/data/native_list_index_original_objects/9.0.4.txt](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/9.0.4.txt)
- [rust/tcl-registry/tests/data/native_list_index_original_objects/9.1.0.txt](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/9.1.0.txt)

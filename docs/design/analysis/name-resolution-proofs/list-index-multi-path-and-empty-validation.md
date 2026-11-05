# naming.list-index.multi-path-and-empty-validation

Kind: `native-observation`

## Problem statement

No-index lindex can return the actual original list, whereas a multi-index walk can copy or return a member. An empty list may withdraw a later invalid index in one release and validate it in another.

## Question

What original result identity and native instruction follow no-index, list-path, multiword path and empty-list/invalid-index controls?

## Conclusion

No-index lindex returns the original x object with lindexMulti1 in every capture. A list path{0 1} uses listindex/listIndex and a multiword0 1 uses lindexMulti3; both return C with original header fields attached. A path{0 bad} fails in every release. For lindex{}0 bad, C8.4 returns normal empty without validating bad; C8.5–9.1 return the exact bad-index error. This is a finite executed procedure/compiler comparison, not an arbitrary walk/refetch allocation explanation.

## Scope

Selected body IDs [2, 3, 4, 15, 16] from19 actual procedure bodies per C release,95 original windows. C Tcl_EvalEx(-1,TCL_EVAL_GLOBAL) defines p{x i}; actual Tcl_EvalObjv passes separately retained original x/i String objects. Guest result primary/resident/refs/result==x are sampled before materialization, then native compiled body code is scanned for list-index opcodes/immediates. The listing is stored compilation output, not an instruction execution trace. Full original stdout hashes/source/lib/exe/status are retained; native source/header snippets, compiler version and launched patchlevel query are absent. No Jim/BIG-IP capture.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original ASCII native procedure source; actual public Tcl_EvalObjv original argument headers and post-invocation bytecode listing. Version: tcl8.4 8.4.20.

Selected exact original rows: case/guest code/result primary/resident/refs/result==original x/result hex/stored compiled opcode and optional immediate.

```text
2|0|none|1|2|1|7b7b4120427d20437d2044|lindexMulti:1
3|0|none|1|2|0|43|listindex
4|0|none|1|2|0|43|lindexMulti:3
15|1|string|1|1|0|62616420696e6465782022626164223a206d75737420626520696e7465676572206f7220656e643f2d696e74656765723f|listindex
16|0|none|1|1|0||lindexMulti:3
```

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original ASCII native procedure source; actual public Tcl_EvalObjv original argument headers and post-invocation bytecode listing. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Selected exact original rows: case/guest code/result primary/resident/refs/result==original x/result hex/stored compiled opcode and optional immediate.

```text
2|0|none|1|2|1|7b7b4120427d20437d2044|lindexMulti:1
3|0|none|1|2|0|43|listIndex
4|0|none|1|2|0|43|lindexMulti:3
15|1|string|1|1|0|62616420696e6465782022626164223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f|listIndex
16|1|string|1|1|0|62616420696e6465782022626164223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f|lindexMulti:3
```

### jim, bigip

Status: `not-tested`.

No original observation for this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-registry/src/native_list_index_compilation.rs](../../../../rust/tcl-registry/src/native_list_index_compilation.rs), `compile_native_list_index`: Retains native coordinate/immediate selection separately from original operand headers and actual child completion.
- [rust/tcl-registry/src/native_list_index_compilation.rs](../../../../rust/tcl-registry/src/native_list_index_compilation.rs), `native_list_index_compilation::tests::original_list_index_recipes_match_95_native_instruction_and_result_windows` (linked): Compares selected nonempty opcode listings against original recipes within95 rows. Rows without a listed opcode are counted but skipped by this pure instruction test; result/header parity is a separate consumer binding.
- [rust/tcl-vm/src/exec/native_list_index_tests.rs](../../../../rust/tcl-vm/src/exec/native_list_index_tests.rs), `exec::native_list_index_tests::original_compiled_list_index_preserves_95_native_headers_and_completions` (linked): Compares original95 result/header/guest windows for authentic compiled source; no fresh native run or Rust pass is inferred.
- [runtime/rust/src/interp/native_body_artifact/native_list_index.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_list_index.rs), `interp::native_body_artifact::native_list_index::tests::original_list_index_artifact_preserves_95_native_header_and_completion_windows` (linked): Runtime artifact compares the same95 original result/header/guest outcomes independently of a native instruction-entry trace.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_list_index_compilation/probe.c](../../../../rust/tcl-registry/tests/data/native_list_index_compilation/probe.c)
- [rust/tcl-registry/tests/data/native_list_index_compilation/cases.json](../../../../rust/tcl-registry/tests/data/native_list_index_compilation/cases.json)
- [rust/tcl-registry/tests/data/native_list_index_compilation/manifest.json](../../../../rust/tcl-registry/tests/data/native_list_index_compilation/manifest.json)
- [rust/tcl-registry/tests/data/native_list_index_compilation/8.4.20.txt](../../../../rust/tcl-registry/tests/data/native_list_index_compilation/8.4.20.txt)
- [rust/tcl-registry/tests/data/native_list_index_compilation/8.5.19.txt](../../../../rust/tcl-registry/tests/data/native_list_index_compilation/8.5.19.txt)
- [rust/tcl-registry/tests/data/native_list_index_compilation/8.6.18.txt](../../../../rust/tcl-registry/tests/data/native_list_index_compilation/8.6.18.txt)
- [rust/tcl-registry/tests/data/native_list_index_compilation/9.0.4.txt](../../../../rust/tcl-registry/tests/data/native_list_index_compilation/9.0.4.txt)
- [rust/tcl-registry/tests/data/native_list_index_compilation/9.1.0.txt](../../../../rust/tcl-registry/tests/data/native_list_index_compilation/9.1.0.txt)

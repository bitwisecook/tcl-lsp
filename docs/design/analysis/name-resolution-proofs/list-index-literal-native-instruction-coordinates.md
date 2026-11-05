# naming.list-index.literal-native-instruction-coordinates

Kind: `native-observation`

## Problem statement

A native literal can select an immediate coordinate, a runtime index operation, or a rejected index. Re-encoding end/outside through a portable coordinate can change the selected opcode or silently accept malformed source.

## Question

Which emitted native list-index opcode/immediate and completion follow the selected constant, end-offset, arithmetic and runtime operand controls?

## Conclusion

C8.4 uses listindex for these selected cases; malformed bad/end+1/1+1 fail, while end/end-1 and0/escaped0 return the attached members. C8.5 selects listIndexImm0 for0, listIndexImm2147483647 for that literal and runtime listIndex for the other selected operands. C8.6 selects end=-2, end-1=-3, outside=-1, arithmetic2 and escaped0 immediates. C9 keeps those end coordinates but selects2147483647 directly and leaves end+1 to runtime listIndex. Exact error/result/header rows remain release-specific; no arbitrary native target-width or portable opcode equivalence is inferred.

## Scope

Selected body IDs [0, 1, 5, 6, 7, 8, 9, 10, 11, 14] from19 actual procedure bodies per C release,95 original windows. C Tcl_EvalEx(-1,TCL_EVAL_GLOBAL) defines p{x i}; actual Tcl_EvalObjv passes separately retained original x/i String objects. Guest result primary/resident/refs/result==x are sampled before materialization, then native compiled body code is scanned for list-index opcodes/immediates. The listing is stored compilation output, not an instruction execution trace. Full original stdout hashes/source/lib/exe/status are retained; native source/header snippets, compiler version and launched patchlevel query are absent. No Jim/BIG-IP capture.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original ASCII native procedure source; actual public Tcl_EvalObjv original argument headers and post-invocation bytecode listing. Version: tcl8.4 8.4.20.

Selected exact original rows: case/guest code/result primary/resident/refs/result==original x/result hex/stored compiled opcode and optional immediate.

```text
0|0|none|1|2|0|7b4120427d2043|listindex
1|0|none|1|2|0|7b4120427d2043|listindex
5|1|string|1|1|0|62616420696e6465782022626164223a206d75737420626520696e7465676572206f7220656e643f2d696e74656765723f|listindex
6|0|none|1|1|0||listindex
7|0|none|1|2|0|44|listindex
8|0|none|1|2|0|7b4120427d2043|listindex
9|1|string|1|1|0|62616420696e6465782022656e642b31223a206d75737420626520696e7465676572206f7220656e643f2d696e74656765723f|listindex
10|1|string|1|1|0|62616420696e6465782022312b31223a206d75737420626520696e7465676572206f7220656e643f2d696e74656765723f|listindex
11|0|none|1|2|0|7b4120427d2043|listindex
14|0|none|1|1|0||listindex
```

### tcl8.5

Status: `observed`. Channel: Original ASCII native procedure source; actual public Tcl_EvalObjv original argument headers and post-invocation bytecode listing. Version: tcl8.5 8.5.19.

Selected exact original rows: case/guest code/result primary/resident/refs/result==original x/result hex/stored compiled opcode and optional immediate.

```text
0|0|none|1|2|0|7b4120427d2043|listIndexImm:0
1|0|none|1|2|0|7b4120427d2043|listIndex
5|1|string|1|1|0|62616420696e6465782022626164223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f|listIndex
6|0|none|1|1|0||listIndex
7|0|none|1|2|0|44|listIndex
8|0|none|1|2|0|7b4120427d2043|listIndex
9|0|none|1|1|0||listIndex
10|0|none|1|1|0||listIndex
11|0|none|1|2|0|7b4120427d2043|listIndex
14|0|none|1|1|0||listIndexImm:2147483647
```

### tcl8.6

Status: `observed`. Channel: Original ASCII native procedure source; actual public Tcl_EvalObjv original argument headers and post-invocation bytecode listing. Version: tcl8.6 8.6.18.

Selected exact original rows: case/guest code/result primary/resident/refs/result==original x/result hex/stored compiled opcode and optional immediate.

```text
0|0|none|1|2|0|7b4120427d2043|listIndexImm:0
1|0|none|1|2|0|7b4120427d2043|listIndex
5|1|string|1|1|0|62616420696e6465782022626164223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f|listIndex
6|0|none|1|1|0||listIndexImm:-1
7|0|none|1|2|0|44|listIndexImm:-2
8|0|none|1|2|0|7b4120427d2043|listIndexImm:-3
9|0|none|1|1|0||listIndexImm:-1
10|0|none|1|1|0||listIndexImm:2
11|0|none|1|2|0|7b4120427d2043|listIndexImm:0
14|0|none|1|1|0||listIndexImm:-1
```

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original ASCII native procedure source; actual public Tcl_EvalObjv original argument headers and post-invocation bytecode listing. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Selected exact original rows: case/guest code/result primary/resident/refs/result==original x/result hex/stored compiled opcode and optional immediate.

```text
0|0|none|1|2|0|7b4120427d2043|listIndexImm:0
1|0|none|1|2|0|7b4120427d2043|listIndex
5|1|string|1|1|0|62616420696e6465782022626164223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f|listIndex
6|0|none|1|1|0||listIndexImm:-1
7|0|none|1|2|0|44|listIndexImm:-2
8|0|none|1|2|0|7b4120427d2043|listIndexImm:-3
9|0|none|1|1|0||listIndex
10|0|none|1|1|0||listIndexImm:2
11|0|none|1|2|0|7b4120427d2043|listIndexImm:0
14|0|none|1|1|0||listIndexImm:2147483647
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

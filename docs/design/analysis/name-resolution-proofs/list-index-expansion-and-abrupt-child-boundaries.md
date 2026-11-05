# naming.list-index.expansion-and-abrupt-child-boundaries

Kind: `native-observation`

## Problem statement

Expansion and an abrupt child command can choose the generic command or end evaluation before an index opcode is reached. A stored opcode in the compiled body does not prove it executed.

## Question

Which completion/result/header and compiled body instruction are observed for original expansion, malformed List and early-return operand controls?

## Conclusion

C8.4 rejects both expansion source forms as extra characters after close-brace; C8.5–9.1 accept computed expansion with result0 but the fixed a b expansion reaches a bad-b index error. The malformed original list source always gives unmatched open brace in list. The [return EARLY] operand returns EARLY normally from the procedure for all captures; the retained bytecode listing may contain lindexMulti3 in C8.4/8.5 while later releases have no printed list-index instruction. The observer lists compiled body opcodes and does not trace their execution, so none is claimed entered after the abrupt child.

## Scope

Selected body IDs [12, 13, 17, 18] from19 actual procedure bodies per C release,95 original windows. C Tcl_EvalEx(-1,TCL_EVAL_GLOBAL) defines p{x i}; actual Tcl_EvalObjv passes separately retained original x/i String objects. Guest result primary/resident/refs/result==x are sampled before materialization, then native compiled body code is scanned for list-index opcodes/immediates. The listing is stored compilation output, not an instruction execution trace. Full original stdout hashes/source/lib/exe/status are retained; native source/header snippets, compiler version and launched patchlevel query are absent. No Jim/BIG-IP capture.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original ASCII native procedure source; actual public Tcl_EvalObjv original argument headers and post-invocation bytecode listing. Version: tcl8.4 8.4.20.

Selected exact original rows: case/guest code/result primary/resident/refs/result==original x/result hex/stored compiled opcode and optional immediate.

```text
12|1|none|1|1|0|6578747261206368617261637465727320616674657220636c6f73652d6272616365|
13|1|none|1|1|0|6578747261206368617261637465727320616674657220636c6f73652d6272616365|
17|1|none|1|1|0|756e6d617463686564206f70656e20627261636520696e206c697374|listindex
18|0|none|1|3|0|4541524c59|lindexMulti:3
```

### tcl8.5

Status: `observed`. Channel: Original ASCII native procedure source; actual public Tcl_EvalObjv original argument headers and post-invocation bytecode listing. Version: tcl8.5 8.5.19.

Selected exact original rows: case/guest code/result primary/resident/refs/result==original x/result hex/stored compiled opcode and optional immediate.

```text
12|0|none|1|2|0|30|
13|1|string|1|1|0|62616420696e646578202262223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f|lindexMulti:3
17|1|none|1|1|0|756e6d617463686564206f70656e20627261636520696e206c697374|listIndexImm:0
18|0|none|1|3|0|4541524c59|lindexMulti:3
```

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original ASCII native procedure source; actual public Tcl_EvalObjv original argument headers and post-invocation bytecode listing. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Selected exact original rows: case/guest code/result primary/resident/refs/result==original x/result hex/stored compiled opcode and optional immediate.

```text
12|0|none|1|2|0|30|
13|1|string|1|1|0|62616420696e646578202262223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f|lindexMulti:3
17|1|string|1|1|0|756e6d617463686564206f70656e20627261636520696e206c697374|listIndexImm:0
18|0|none|1|3|0|4541524c59|
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

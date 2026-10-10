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

Status: `observed`. Version: 8.4.20; exact launched patchlevel not queried by this probe. Build: library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; binary_sha256=58bf75833c2e69d22bce7ae961ce9e511e144247a2f2aa4cabbf3404b6a05182; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original ASCII native procedure source; actual public Tcl_EvalObjv original argument headers and post-invocation bytecode listing.. Dialect: C Tcl.

Selected exact original rows: case/guest code/result primary/resident/refs/result==original x/result hex/stored compiled opcode and optional immediate.

```text
12|1|none|1|1|0|6578747261206368617261637465727320616674657220636c6f73652d6272616365|
13|1|none|1|1|0|6578747261206368617261637465727320616674657220636c6f73652d6272616365|
17|1|none|1|1|0|756e6d617463686564206f70656e20627261636520696e206c697374|listindex
18|0|none|1|3|0|4541524c59|lindexMulti:3
```

### tcl8.5

Status: `observed`. Version: 8.5.19; exact launched patchlevel not queried by this probe. Build: library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; binary_sha256=26762953019c5513eaf15de170cae4cac291c0614ff0a725d993a306bb0e2c1b; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original ASCII native procedure source; actual public Tcl_EvalObjv original argument headers and post-invocation bytecode listing.. Dialect: C Tcl.

Selected exact original rows: case/guest code/result primary/resident/refs/result==original x/result hex/stored compiled opcode and optional immediate.

```text
12|0|none|1|2|0|30|
13|1|string|1|1|0|62616420696e646578202262223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f|lindexMulti:3
17|1|none|1|1|0|756e6d617463686564206f70656e20627261636520696e206c697374|listIndexImm:0
18|0|none|1|3|0|4541524c59|lindexMulti:3
```

### tcl8.6

Status: `observed`. Version: 8.6.18; exact launched patchlevel not queried by this probe. Build: library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; binary_sha256=a4c3e1028c6b4d77ecbe06d886621c2a9dbe7d417ad66e38cb595b0f9a37ae7a; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original ASCII native procedure source; actual public Tcl_EvalObjv original argument headers and post-invocation bytecode listing.. Dialect: C Tcl.

Selected exact original rows: case/guest code/result primary/resident/refs/result==original x/result hex/stored compiled opcode and optional immediate.

```text
12|0|none|1|2|0|30|
13|1|string|1|1|0|62616420696e646578202262223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f|lindexMulti:3
17|1|string|1|1|0|756e6d617463686564206f70656e20627261636520696e206c697374|listIndexImm:0
18|0|none|1|3|0|4541524c59|
```

### tcl9.0

Status: `observed`. Version: 9.0.4; exact launched patchlevel not queried by this probe. Build: library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; binary_sha256=9357c6b92aaf0dad96d14ea7e42fb4d614bf87084e64b339e18466a9d34d129b; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original ASCII native procedure source; actual public Tcl_EvalObjv original argument headers and post-invocation bytecode listing.. Dialect: C Tcl.

Selected exact original rows: case/guest code/result primary/resident/refs/result==original x/result hex/stored compiled opcode and optional immediate.

```text
12|0|none|1|2|0|30|
13|1|string|1|1|0|62616420696e646578202262223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f|lindexMulti:3
17|1|string|1|1|0|756e6d617463686564206f70656e20627261636520696e206c697374|listIndexImm:0
18|0|none|1|3|0|4541524c59|
```

### tcl9.1

Status: `observed`. Version: 9.1.0; exact launched patchlevel not queried by this probe. Build: library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; binary_sha256=6d760daca842c01f32bec078d92b26710fe190353ded05856dcdcec1ea70ed42; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original ASCII native procedure source; actual public Tcl_EvalObjv original argument headers and post-invocation bytecode listing.. Dialect: C Tcl.

Selected exact original rows: case/guest code/result primary/resident/refs/result==original x/result hex/stored compiled opcode and optional immediate.

```text
12|0|none|1|2|0|30|
13|1|string|1|1|0|62616420696e646578202262223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f|lindexMulti:3
17|1|string|1|1|0|756e6d617463686564206f70656e20627261636520696e206c697374|listIndexImm:0
18|0|none|1|3|0|4541524c59|
```

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No original observation for this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No original observation for this exact question is attached for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-registry/tests/data/native_list_index_compilation/probe.c](../../../../rust/tcl-registry/tests/data/native_list_index_compilation/probe.c). SHA-256 `4b885cff4a55d6e817f27d8c35460fb9f0efd6f42b92eea00fb61bc70aa2feb0`. Exact19 body literals, public definition/invocation and private post-invocation compiled-body listing.
- `bodies` (input): [rust/tcl-registry/tests/data/native_list_index_compilation/cases.json](../../../../rust/tcl-registry/tests/data/native_list_index_compilation/cases.json). SHA-256 `97f7e039efcb4ea18a5b221a1f6ddfd55303b9484db587a4385bd974fb822840`. Exact19 body strings byte-equal to the retained C literal vector.
- `receipt` (provider): [rust/tcl-registry/tests/data/native_list_index_compilation/manifest.json](../../../../rust/tcl-registry/tests/data/native_list_index_compilation/manifest.json). SHA-256 `738762535b986de6ed54572485f4ca223a8bbf8c3e137987ee53b568eee365b1`. Five original source/library/executable/full-output/compiler/process associations.
- `rows-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_list_index_compilation/8.4.20.txt](../../../../rust/tcl-registry/tests/data/native_list_index_compilation/8.4.20.txt). SHA-256 `6f93ffbca3c80ea862787ec62591b2b165dd10ac1db7d4da3de668008b085681`. Full19-row source-native compilation/result stream; selected case IDs [12, 13, 17, 18].
- `rows-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_list_index_compilation/8.5.19.txt](../../../../rust/tcl-registry/tests/data/native_list_index_compilation/8.5.19.txt). SHA-256 `2247fdb576755e21ffe0bdce4378af571f1593801523b7a4abdad16a1eb1ba68`. Full19-row source-native compilation/result stream; selected case IDs [12, 13, 17, 18].
- `rows-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_list_index_compilation/8.6.18.txt](../../../../rust/tcl-registry/tests/data/native_list_index_compilation/8.6.18.txt). SHA-256 `efe36e866f4b6ace6dbe5f8146a910b9da4c1fd3e76da235caf60cbe754daa48`. Full19-row source-native compilation/result stream; selected case IDs [12, 13, 17, 18].
- `rows-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_list_index_compilation/9.0.4.txt](../../../../rust/tcl-registry/tests/data/native_list_index_compilation/9.0.4.txt). SHA-256 `f810a08941b7a67cd473a33a09e0ea8bd3573d8e4ff58f0fff6da1e145e354b4`. Full19-row source-native compilation/result stream; selected case IDs [12, 13, 17, 18].
- `rows-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_list_index_compilation/9.1.0.txt](../../../../rust/tcl-registry/tests/data/native_list_index_compilation/9.1.0.txt). SHA-256 `f810a08941b7a67cd473a33a09e0ea8bd3573d8e4ff58f0fff6da1e145e354b4`. Full19-row source-native compilation/result stream; selected case IDs [12, 13, 17, 18].

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_list_index_compilation.rs](../../../../rust/tcl-registry/src/native_list_index_compilation.rs), `compile_native_list_index`: Retains native coordinate/immediate selection separately from original operand headers and actual child completion.
- [rust/tcl-registry/src/native_list_index_compilation.rs](../../../../rust/tcl-registry/src/native_list_index_compilation.rs), `native_list_index_compilation::tests::original_list_index_recipes_match_95_native_instruction_and_result_windows` (linked): Compares selected nonempty opcode listings against original recipes within95 rows. Rows without a listed opcode are counted but skipped by this pure instruction test; result/header parity is a separate consumer binding.
- [rust/tcl-vm/src/exec/native_list_index_tests.rs](../../../../rust/tcl-vm/src/exec/native_list_index_tests.rs), `exec::native_list_index_tests::original_compiled_list_index_preserves_95_native_headers_and_completions` (linked): Compares original95 result/header/guest windows for authentic compiled source; no fresh native run or Rust pass is inferred.
- [runtime/rust/src/interp/native_body_artifact/native_list_index.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_list_index.rs), `interp::native_body_artifact::native_list_index::tests::original_list_index_artifact_preserves_95_native_header_and_completion_windows` (linked): Runtime artifact compares the same95 original result/header/guest outcomes independently of a native instruction-entry trace.

A named test is a coverage binding, not a claim that it executed.

## Replay

No new native run or Rust execution result is claimed. Replay must compile the retained exact probe against independently identified release headers/libraries and preserve separate compile/process status and raw streams before comparing the attached original hashes. Original absolute compile paths are capture metadata, not a portable executable command. No evaluator/source/parser or native allocation authority may be obtained by matching display text. Probe code is input/observer code; absent native source excerpts supply no implementation explanation.

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

Status: `observed`. Version: 8.4.20; exact launched patchlevel not queried by this probe. Build: library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; binary_sha256=ad897754dfa6d3c67ca85b73c268b35ddbaefde7a3755cc7f925219eb73014b6; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Public object-vector invocation with original native index String object; pre-render header snapshots.. Dialect: C Tcl.

Exact selected rows in the manifest9-column order: case/guest code/original index primary,resident,refs/result primary,resident,refs/result hex.

```text
3|0|list|1|1|none|1|2|43
4|1|list|1|1|string|1|1|62616420696e6465782022626164223a206d75737420626520696e7465676572206f7220656e643f2d696e74656765723f
5|1|none|1|1|string|1|1|62616420696e64657820227b223a206d75737420626520696e7465676572206f7220656e643f2d696e74656765723f
8|1|list|1|1|string|1|1|62616420696e6465782022312b31223a206d75737420626520696e7465676572206f7220656e643f2d696e74656765723f
```

### tcl8.5

Status: `observed`. Version: 8.5.19; exact launched patchlevel not queried by this probe. Build: library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; binary_sha256=1412d6c3443166176d687f2eb0302ec7fe474a898166641acb2b51d8eaf0695d; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Public object-vector invocation with original native index String object; pre-render header snapshots.. Dialect: C Tcl.

Exact selected rows in the manifest9-column order: case/guest code/original index primary,resident,refs/result primary,resident,refs/result hex.

```text
3|0|list|1|1|none|1|2|43
4|1|list|1|1|string|1|1|62616420696e6465782022626164223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f
5|1|none|1|1|string|1|1|62616420696e64657820227b223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f
8|0|none|1|1|none|1|1|
```

### tcl8.6

Status: `observed`. Version: 8.6.18; exact launched patchlevel not queried by this probe. Build: library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; binary_sha256=f56526a05c262fee6fec769fc7547e7762f4cced0b2acc6525826e9a0901b795; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Public object-vector invocation with original native index String object; pre-render header snapshots.. Dialect: C Tcl.

Exact selected rows in the manifest9-column order: case/guest code/original index primary,resident,refs/result primary,resident,refs/result hex.

```text
3|0|list|1|1|none|1|2|43
4|1|list|1|2|string|1|1|62616420696e6465782022626164223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f
5|1|none|1|2|string|1|1|62616420696e64657820227b223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f
8|0|none|1|1|none|1|1|
```

### tcl9.0

Status: `observed`. Version: 9.0.4; exact launched patchlevel not queried by this probe. Build: library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; binary_sha256=d156829db5204640b866f50463a296a8ec838f1c4b3507d003a843f67b8c582d; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Public object-vector invocation with original native index String object; pre-render header snapshots.. Dialect: C Tcl.

Exact selected rows in the manifest9-column order: case/guest code/original index primary,resident,refs/result primary,resident,refs/result hex.

```text
3|0|list|1|1|none|1|2|43
4|1|list|1|2|string|1|1|62616420696e6465782022626164223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f
5|1|none|1|2|string|1|1|62616420696e64657820227b223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f
8|0|end-offset|1|1|none|1|1|
```

### tcl9.1

Status: `observed`. Version: 9.1.0; exact launched patchlevel not queried by this probe. Build: library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; binary_sha256=c24f35feeaeb5221a085e1dfdbd46bfdc952cba8af7a035997feac1855f3f705; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Public object-vector invocation with original native index String object; pre-render header snapshots.. Dialect: C Tcl.

Exact selected rows in the manifest9-column order: case/guest code/original index primary,resident,refs/result primary,resident,refs/result hex.

```text
3|0|list|1|1|none|1|2|43
4|1|list|1|2|string|1|1|62616420696e6465782022626164223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f
5|1|none|1|2|string|1|1|62616420696e64657820227b223a206d75737420626520696e74656765723f5b2b2d5d696e74656765723f206f7220656e643f5b2b2d5d696e74656765723f
8|0|end-offset|1|1|none|1|1|
```

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No original observation for this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No original observation for this exact question is attached for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-registry/tests/data/native_list_index_original_objects/probe.c](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/probe.c). SHA-256 `cc979475c04ee938022110b4f5d101ea9643a1572fb820516286b31fd7e4bdf4`. Exact original index constructors/public procedure invocation and pre-result-getter header observer.
- `inputs` (input): [rust/tcl-registry/tests/data/native_list_index_original_objects/inputs.json](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/inputs.json). SHA-256 `a8a1495d5808748a26cf5635f2bb9ea5c73889bbcecf96ecc21890ff513be6a8`. Nine input strings matched exactly to native C literals.
- `receipt` (provider): [rust/tcl-registry/tests/data/native_list_index_original_objects/manifest.json](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/manifest.json). SHA-256 `acdb4d5976579b520100bad38a4daa23645deaed87474d3fa2e8006c7c808c7f`. Five original retained source/build/process/output closures; compile status absent remains unknown.
- `rows-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_list_index_original_objects/8.4.20.txt](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/8.4.20.txt). SHA-256 `250b9fad8364bd9d0f79f0a0e0417c4cb600a388b64f8a3da592bf5bfd8788d1`. Full9-row original stream; selected case IDs [3, 4, 5, 8].
- `rows-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_list_index_original_objects/8.5.19.txt](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/8.5.19.txt). SHA-256 `e7053739b3065c40c79be2e6378d60607617e0d974dbda0fb9659babae5e07e9`. Full9-row original stream; selected case IDs [3, 4, 5, 8].
- `rows-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_list_index_original_objects/8.6.18.txt](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/8.6.18.txt). SHA-256 `52abdad8a526635ff6afc22cdaf3b907ab0863bb3d02c50f255dff31bdfd1d6f`. Full9-row original stream; selected case IDs [3, 4, 5, 8].
- `rows-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_list_index_original_objects/9.0.4.txt](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/9.0.4.txt). SHA-256 `823fd3746162cc54bc2329c149545752ea2cf84667ec5f6815e10a367f475b09`. Full9-row original stream; selected case IDs [3, 4, 5, 8].
- `rows-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_list_index_original_objects/9.1.0.txt](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/9.1.0.txt). SHA-256 `823fd3746162cc54bc2329c149545752ea2cf84667ec5f6815e10a367f475b09`. Full9-row original stream; selected case IDs [3, 4, 5, 8].

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_list_index_compilation.rs](../../../../rust/tcl-registry/src/native_list_index_compilation.rs), `NativeListIndexOperation`: Keeps runtime index object conversion separate from literal compiler coordinates.
- [rust/tcl-vm/src/exec/native_list_index_tests.rs](../../../../rust/tcl-vm/src/exec/native_list_index_tests.rs), `exec::native_list_index_tests::original_objects::original_index_getters_match_45_native_header_and_result_windows` (linked): Compares45 attached original index/result primary/resident/ref and guest windows; no intermediate native backing allocation assertion is borrowed.
- [runtime/rust/src/interp/native_body_artifact/native_list_index.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_list_index.rs), `interp::native_body_artifact::native_list_index::original_objects::original_index_getters_match_45_native_header_and_result_windows` (linked): Runtime checks45 original result/index header windows under the selected release; source/native currentness remains independent.

A named test is a coverage binding, not a claim that it executed.

## Replay

No new native run or Rust execution result is claimed. Replay must compile the retained exact probe against independently identified release headers/libraries and preserve separate compile/process status and raw streams before comparing the attached original hashes. Original absolute compile paths are capture metadata, not a portable executable command. No evaluator/source/parser or native allocation authority may be obtained by matching display text. Probe code is input/observer code; absent native source excerpts supply no implementation explanation.

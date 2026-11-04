# naming.list-index.original-integer-and-end-offset-headers

Kind: `native-observation`

## Problem statement

An index immediate coordinate does not manufacture an original runtime index object. The actual original operand can gain an integer or end-offset primary while the returned member has its own result reference.

## Question

What original index and returned-result headers follow runtime0/end/end-1/large/-1 operands to lindex$x$i?

## Conclusion

Every captured release converts0/2147483647/-1 to original int and end/end-1 to original end-offset, preserving resident strings and final original refs1. Successful member results have untyped resident headers at refs2; out-of-range large/-1 results are untyped resident empty at refs1. End returns D and end-1 returns{A B}C. Native end-offset updater availability, index backing pins during execution and arbitrary interpreter currentness are not sampled here.

## Scope

Selected input IDs [0, 1, 2, 6, 7] from nine exact original String index constructors on each C release,45 windows. Probe defines p{x i}{lindex$x$i} through public NUL-terminated source evaluation then passes original retained String x/index headers via Tcl_EvalObjv(TCL_EVAL_GLOBAL). It samples original index and result primary/resident/refs before result materialization. Full source/input/stdout/lib/exe/process metadata is retained; compile status, compiler version, header configuration and actual launched patchlevel query are not recorded by this manifest. No intermediate backing identity, end-updater callback or Jim/BIG-IP attempt.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20; exact launched patchlevel not queried by this probe. Build: exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Public object-vector invocation with original native index String object; pre-render header snapshots.. Dialect: C Tcl.

Exact selected rows in the manifest9-column order: case/guest code/original index primary,resident,refs/result primary,resident,refs/result hex.

```text
0|0|int|1|1|none|1|2|7b4120427d2043
1|0|end-offset|1|1|none|1|2|44
2|0|end-offset|1|1|none|1|2|7b4120427d2043
6|0|int|1|1|none|1|1|
7|0|int|1|1|none|1|1|
```

### tcl8.5

Status: `observed`. Version: 8.5.19; exact launched patchlevel not queried by this probe. Build: exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Public object-vector invocation with original native index String object; pre-render header snapshots.. Dialect: C Tcl.

Exact selected rows in the manifest9-column order: case/guest code/original index primary,resident,refs/result primary,resident,refs/result hex.

```text
0|0|int|1|1|none|1|2|7b4120427d2043
1|0|end-offset|1|1|none|1|2|44
2|0|end-offset|1|1|none|1|2|7b4120427d2043
6|0|int|1|1|none|1|1|
7|0|int|1|1|none|1|1|
```

### tcl8.6

Status: `observed`. Version: 8.6.18; exact launched patchlevel not queried by this probe. Build: exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Public object-vector invocation with original native index String object; pre-render header snapshots.. Dialect: C Tcl.

Exact selected rows in the manifest9-column order: case/guest code/original index primary,resident,refs/result primary,resident,refs/result hex.

```text
0|0|int|1|1|none|1|2|7b4120427d2043
1|0|end-offset|1|1|none|1|2|44
2|0|end-offset|1|1|none|1|2|7b4120427d2043
6|0|int|1|1|none|1|1|
7|0|int|1|1|none|1|1|
```

### tcl9.0

Status: `observed`. Version: 9.0.4; exact launched patchlevel not queried by this probe. Build: exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Public object-vector invocation with original native index String object; pre-render header snapshots.. Dialect: C Tcl.

Exact selected rows in the manifest9-column order: case/guest code/original index primary,resident,refs/result primary,resident,refs/result hex.

```text
0|0|int|1|1|none|1|2|7b4120427d2043
1|0|end-offset|1|1|none|1|2|44
2|0|end-offset|1|1|none|1|2|7b4120427d2043
6|0|int|1|1|none|1|1|
7|0|int|1|1|none|1|1|
```

### tcl9.1

Status: `observed`. Version: 9.1.0; exact launched patchlevel not queried by this probe. Build: exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Public object-vector invocation with original native index String object; pre-render header snapshots.. Dialect: C Tcl.

Exact selected rows in the manifest9-column order: case/guest code/original index primary,resident,refs/result primary,resident,refs/result hex.

```text
0|0|int|1|1|none|1|2|7b4120427d2043
1|0|end-offset|1|1|none|1|2|44
2|0|end-offset|1|1|none|1|2|7b4120427d2043
6|0|int|1|1|none|1|1|
7|0|int|1|1|none|1|1|
```

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No original observation for this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No original observation for this exact question is attached for this provider.

## Historical references

- `probe` (input): [rust/tcl-registry/tests/data/native_list_index_original_objects/probe.c](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/probe.c). Exact original index constructors/public procedure invocation and pre-result-getter header observer.
- `inputs` (input): [rust/tcl-registry/tests/data/native_list_index_original_objects/inputs.json](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/inputs.json). Nine input strings matched exactly to native C literals.
- `receipt` (provider): [rust/tcl-registry/tests/data/native_list_index_original_objects/manifest.json](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/manifest.json). Five original retained source/build/process/output closures; compile status absent remains unknown.
- `rows-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_list_index_original_objects/8.4.20.txt](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/8.4.20.txt). Full9-row original stream; selected case IDs [0, 1, 2, 6, 7].
- `rows-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_list_index_original_objects/8.5.19.txt](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/8.5.19.txt). Full9-row original stream; selected case IDs [0, 1, 2, 6, 7].
- `rows-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_list_index_original_objects/8.6.18.txt](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/8.6.18.txt). Full9-row original stream; selected case IDs [0, 1, 2, 6, 7].
- `rows-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_list_index_original_objects/9.0.4.txt](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/9.0.4.txt). Full9-row original stream; selected case IDs [0, 1, 2, 6, 7].
- `rows-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_list_index_original_objects/9.1.0.txt](../../../../rust/tcl-registry/tests/data/native_list_index_original_objects/9.1.0.txt). Full9-row original stream; selected case IDs [0, 1, 2, 6, 7].

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_list_index_compilation.rs](../../../../rust/tcl-registry/src/native_list_index_compilation.rs), `NativeListIndexOperation`: Keeps runtime index object conversion separate from literal compiler coordinates.
- [rust/tcl-vm/src/exec/native_list_index_tests.rs](../../../../rust/tcl-vm/src/exec/native_list_index_tests.rs), `exec::native_list_index_tests::original_objects::original_index_getters_match_45_native_header_and_result_windows` (linked): Compares45 attached original index/result primary/resident/ref and guest windows; no intermediate native backing allocation assertion is borrowed.
- [runtime/rust/src/interp/native_body_artifact/native_list_index.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_list_index.rs), `interp::native_body_artifact::native_list_index::original_objects::original_index_getters_match_45_native_header_and_result_windows` (linked): Runtime checks45 original result/index header windows under the selected release; source/native currentness remains independent.

A named test is a coverage binding, not a claim that it executed.

## Replay

No new native run or Rust execution result is claimed. Replay must compile the retained exact probe against independently identified release headers/libraries and preserve separate compile/process status and raw streams before comparing the attached original hashes. Original absolute compile paths are capture metadata, not a portable executable command. No evaluator/source/parser or native allocation authority may be obtained by matching display text. Probe code is input/observer code; absent native source excerpts supply no implementation explanation.


## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

[Exact original body hashes and process outcomes](rust-validation/frozen349/question-body-outcomes.json) join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

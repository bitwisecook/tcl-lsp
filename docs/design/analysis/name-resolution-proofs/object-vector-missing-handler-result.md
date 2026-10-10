# naming.object-vector.missing-handler-result

Kind: `native-observation`

## Problem statement

Calling an absent command through an original object vector can render arguments to publish an error, while invoking a source command reaches a different reporting path. A rendered missing-command message does not prove that argument storage stayed untouched.

## Question

For an absent handler, do original object-vector and source evaluation leave the same argument residency and result primary?

## Conclusion

All five C releases return code1 and a string result with invalid command name "missing_original". The original List argument is resident after direct Tcl_EvalObjv but remains stringless after Tcl_EvalEx source evaluation. Jim returns the same message with an untyped result and a stringless original List for both routes. These snapshots precede reading result bytes and do not measure an unknown-handler replacement or full errorInfo.

## Scope

The probe creates one original stringless List containing A B, installs it as arg, and observes five commands through public original-object argv (flags0) and source evaluation (flags0). Case0–4 are direct; case5–9 are source. Result primary and original argument residency are sampled before the result byte observer. Full source, six captured streams and compile/library/header/executable metadata are retained. C release labels are capture metadata; the probe does not query runtime patchlevel. Jim revision/patch/configuration is not recorded. BIG-IP is not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; runtime patch/revision not queried). Build: binary_sha256=307d02f4f52dbbec9d94357e24d3c81311dee907b82738ce17e900832acbde12; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; headers={"/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tcl.h": "824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf", "/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInt.h": "f51c8e66c562de424b84a35a04b6b86d9a135ef7d4ca891b35c3b0586bc0ce1a"}. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Dialect: tcl8.4.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["0","1","1","string","696e76616c696420636f6d6d616e64206e616d6520226d697373696e675f6f726967696e616c22"],["5","1","0","string","696e76616c696420636f6d6d616e64206e616d6520226d697373696e675f6f726967696e616c22"]]. Process/compile status0; empty stderr.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; runtime patch/revision not queried). Build: binary_sha256=1e61bffb5a6a11817e7197b622076fa1e4b43d96fc2307ae15956f5d2defe069; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; headers={"/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tcl.h": "c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5", "/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInt.h": "72078ca207924e61b4fbf637eff263d280a0dbc78e6a5fbb39e2cd980d08a8aa"}. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Dialect: tcl8.5.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["0","1","1","string","696e76616c696420636f6d6d616e64206e616d6520226d697373696e675f6f726967696e616c22"],["5","1","0","string","696e76616c696420636f6d6d616e64206e616d6520226d697373696e675f6f726967696e616c22"]]. Process/compile status0; empty stderr.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; runtime patch/revision not queried). Build: binary_sha256=1a417192e6d3627198897d2b520cca0718efda00781c122731f40a913aa9727d; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; headers={"/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tcl.h": "aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245", "/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInt.h": "e9e70f6461d7c4b37173173956040b647914157c28425ec859730cd8eaef4d2e"}. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Dialect: tcl8.6.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["0","1","1","string","696e76616c696420636f6d6d616e64206e616d6520226d697373696e675f6f726967696e616c22"],["5","1","0","string","696e76616c696420636f6d6d616e64206e616d6520226d697373696e675f6f726967696e616c22"]]. Process/compile status0; empty stderr.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; runtime patch/revision not queried). Build: binary_sha256=1332569b797c9e2710d61e1909cabf522703e58c36e4baf8be05788251a45dbf; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; headers={"/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tcl.h": "eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a", "/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInt.h": "f000e14fdc4e00da1c5ab98a8087c41be3cbf26564a274152236e0531bb55053"}. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Dialect: tcl9.0.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["0","1","1","string","696e76616c696420636f6d6d616e64206e616d6520226d697373696e675f6f726967696e616c22"],["5","1","0","string","696e76616c696420636f6d6d616e64206e616d6520226d697373696e675f6f726967696e616c22"]]. Process/compile status0; empty stderr.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; runtime patch/revision not queried). Build: binary_sha256=590439782c09d46f6970197ee4b0fa48241bc3e1b3258e27c62b2d49788a6a1f; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; headers={"/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tcl.h": "30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950", "/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInt.h": "fea9d99020df9aec6efebf6e3f18592ee267a9bf80d3843df0d6439efebe65cc"}. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Dialect: tcl9.1.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["0","1","1","string","696e76616c696420636f6d6d616e64206e616d6520226d697373696e675f6f726967696e616c22"],["5","1","0","string","696e76616c696420636f6d6d616e64206e616d6520226d697373696e675f6f726967696e616c22"]]. Process/compile status0; empty stderr.

### jim

Status: `observed`. Version: Jim (capture association; runtime patch/revision not queried). Build: binary_sha256=9fcbf55c858d67da55851f5d329c94e192b92193674491382e082704f17d050b; library_sha256=75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; headers={"/tmp/2286-oracles/jimtcl/jim.h": "d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d"}. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Dialect: jim.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["0","1","0","none","696e76616c696420636f6d6d616e64206e616d6520226d697373696e675f6f726967696e616c22"],["5","1","0","none","696e76616c696420636f6d6d616e64206e616d6520226d697373696e675f6f726967696e616c22"]]. Process/compile status0; empty stderr.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No native observation of this exact question is attached for this provider.

## Exact evidence

- `probe` (input): [runtime/rust/tests/data/native_object_vector/probe.c](../../../../runtime/rust/tests/data/native_object_vector/probe.c). SHA-256 `4019582c89320db840006a2f5c99019d7470d44f443f5590d09355f11c524dae`. Exact native original List constructor, selected command/callback, direct/source flags and physical result observer.
- `receipt` (provider): [runtime/rust/tests/data/native_object_vector/manifest.json](../../../../runtime/rust/tests/data/native_object_vector/manifest.json). SHA-256 `8ffcfc6a89e0fc6a102a27bf669543d843fccf43c41125cd34811c6be686f478`. Full original stdout/stderr/status plus compiler, library/header and executable metadata.
- `rows-tcl8.4` (observation): [runtime/rust/tests/data/native_object_vector/8.4.20.txt](../../../../runtime/rust/tests/data/native_object_vector/8.4.20.txt). SHA-256 `d441ffee0c85eb5d7889e69111bdab31f9ccc92042cc76244ef57ef700b2e605`. Original full output; relevant cases [0, 5]. Columns: case,completion,original argument resident,result primary,result hex.
- `rows-tcl8.5` (observation): [runtime/rust/tests/data/native_object_vector/8.5.19.txt](../../../../runtime/rust/tests/data/native_object_vector/8.5.19.txt). SHA-256 `d441ffee0c85eb5d7889e69111bdab31f9ccc92042cc76244ef57ef700b2e605`. Original full output; relevant cases [0, 5]. Columns: case,completion,original argument resident,result primary,result hex.
- `rows-tcl8.6` (observation): [runtime/rust/tests/data/native_object_vector/8.6.18.txt](../../../../runtime/rust/tests/data/native_object_vector/8.6.18.txt). SHA-256 `d441ffee0c85eb5d7889e69111bdab31f9ccc92042cc76244ef57ef700b2e605`. Original full output; relevant cases [0, 5]. Columns: case,completion,original argument resident,result primary,result hex.
- `rows-tcl9.0` (observation): [runtime/rust/tests/data/native_object_vector/9.0.4.txt](../../../../runtime/rust/tests/data/native_object_vector/9.0.4.txt). SHA-256 `d441ffee0c85eb5d7889e69111bdab31f9ccc92042cc76244ef57ef700b2e605`. Original full output; relevant cases [0, 5]. Columns: case,completion,original argument resident,result primary,result hex.
- `rows-tcl9.1` (observation): [runtime/rust/tests/data/native_object_vector/9.1.0.txt](../../../../runtime/rust/tests/data/native_object_vector/9.1.0.txt). SHA-256 `d441ffee0c85eb5d7889e69111bdab31f9ccc92042cc76244ef57ef700b2e605`. Original full output; relevant cases [0, 5]. Columns: case,completion,original argument resident,result primary,result hex.
- `rows-jim` (observation): [runtime/rust/tests/data/native_object_vector/Jim.txt](../../../../runtime/rust/tests/data/native_object_vector/Jim.txt). SHA-256 `4db06c56f638d3ef26a4a0373bc42ea44bb428786cdb73608ac0da5feee5af2b`. Original full output; relevant cases [0, 5]. Columns: case,completion,original argument resident,result primary,result hex.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_object_vector.rs](../../../../runtime/rust/src/interp/native_object_vector.rs), `eval_original_object_vector`: Consumes original borrowed argv through the independently selected evaluator/error/completion protocol; the captured native row does not manufacture a Rust original object.
- [runtime/rust/src/interp/native_object_vector.rs](../../../../runtime/rust/src/interp/native_object_vector.rs), `tests::original_public_vectors_and_source_commands_match_all_60_native_windows` (linked): The named original cases independently compare completion, original argument residency, result primary and bytes before the physical observer renders output.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. The exact probe and captured output can support a new run only after independently pinning the actual release, headers, library and build configuration; capture-time absolute compiler paths are metadata, not a current portable replay runner.

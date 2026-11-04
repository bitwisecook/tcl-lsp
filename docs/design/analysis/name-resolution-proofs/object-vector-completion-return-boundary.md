# naming.object-vector.completion.return-boundary

Kind: `native-observation`

## Problem statement

A return completion can be consumed or surfaced by the evaluator, and direct original vectors can cross a different completion boundary from source text. Treating all successful-looking return bytes as code0 loses the release-specific boundary.

## Question

How do direct original-object return and source return normalize completion while preserving the original List result storage?

## Conclusion

Direct return is code0/List/stringless in C8.4 and C8.6–9.1, but C8.5 returns code1, materializes the original argument, and publishes string "command returned bad code: 0". Source return is code0/List/stringless in all five C releases. Jim returns code2/List/stringless through both routes. Successful List result bytes are7B4120427D. No result pointer is sampled by the native probe, so List type/bytes alone do not attest original-object identity.

## Scope

The probe creates one original stringless List containing A B, installs it as arg, and observes five commands through public original-object argv (flags0) and source evaluation (flags0). Case0–4 are direct; case5–9 are source. Result primary and original argument residency are sampled before the result byte observer. Full source, six captured streams and compile/library/header/executable metadata are retained. C release labels are capture metadata; the probe does not query runtime patchlevel. Jim revision/patch/configuration is not recorded. BIG-IP is not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (capture association; runtime patch/revision not queried). Build: headers={"/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tcl.h": "824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf", "/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclInt.h": "f51c8e66c562de424b84a35a04b6b86d9a135ef7d4ca891b35c3b0586bc0ce1a"}. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Dialect: tcl8.4.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["0","0","0","list","7b4120427d"],["5","0","0","list","7b4120427d"]]. Process/compile status0; empty stderr.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; runtime patch/revision not queried). Build: headers={"/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tcl.h": "c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5", "/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclInt.h": "72078ca207924e61b4fbf637eff263d280a0dbc78e6a5fbb39e2cd980d08a8aa"}. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Dialect: tcl8.5.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["0","1","1","string","636f6d6d616e642072657475726e65642062616420636f64653a2030"],["5","0","0","list","7b4120427d"]]. Process/compile status0; empty stderr.

### tcl8.6

Status: `observed`. Version: 8.6.18 (capture association; runtime patch/revision not queried). Build: headers={"/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tcl.h": "aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245", "/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclInt.h": "e9e70f6461d7c4b37173173956040b647914157c28425ec859730cd8eaef4d2e"}. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Dialect: tcl8.6.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["0","0","0","list","7b4120427d"],["5","0","0","list","7b4120427d"]]. Process/compile status0; empty stderr.

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; runtime patch/revision not queried). Build: headers={"/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tcl.h": "eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a", "/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclInt.h": "f000e14fdc4e00da1c5ab98a8087c41be3cbf26564a274152236e0531bb55053"}. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Dialect: tcl9.0.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["0","0","0","list","7b4120427d"],["5","0","0","list","7b4120427d"]]. Process/compile status0; empty stderr.

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; runtime patch/revision not queried). Build: headers={"/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tcl.h": "30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950", "/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclInt.h": "fea9d99020df9aec6efebf6e3f18592ee267a9bf80d3843df0d6439efebe65cc"}. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Dialect: tcl9.1.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["0","0","0","list","7b4120427d"],["5","0","0","list","7b4120427d"]]. Process/compile status0; empty stderr.

### jim

Status: `observed`. Version: Jim (capture association; runtime patch/revision not queried). Build: headers={"/tmp/2286-oracles/jimtcl/jim.h": "d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d"}. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Dialect: jim.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["0","2","0","list","7b4120427d"],["5","2","0","list","7b4120427d"]]. Process/compile status0; empty stderr.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No native observation of this exact question is attached for this provider.

## Historical references

- `probe` (input): [runtime/rust/tests/data/native_object_vector/completions/probe.c](../../../../runtime/rust/tests/data/native_object_vector/completions/probe.c). Exact native original List constructor, selected command/callback, direct/source flags and physical result observer.
- `receipt` (provider): [runtime/rust/tests/data/native_object_vector/completions/manifest.json](../../../../runtime/rust/tests/data/native_object_vector/completions/manifest.json). Full original stdout/stderr/status plus compiler, library/header and executable metadata.
- `rows-tcl8.4` (observation): [runtime/rust/tests/data/native_object_vector/completions/8.4.20.txt](../../../../runtime/rust/tests/data/native_object_vector/completions/8.4.20.txt). Original full output; relevant cases [0, 5]. Columns: case,completion,original argument resident,result primary,result hex.
- `rows-tcl8.5` (observation): [runtime/rust/tests/data/native_object_vector/completions/8.5.19.txt](../../../../runtime/rust/tests/data/native_object_vector/completions/8.5.19.txt). Original full output; relevant cases [0, 5]. Columns: case,completion,original argument resident,result primary,result hex.
- `rows-tcl8.6` (observation): [runtime/rust/tests/data/native_object_vector/completions/8.6.18.txt](../../../../runtime/rust/tests/data/native_object_vector/completions/8.6.18.txt). Original full output; relevant cases [0, 5]. Columns: case,completion,original argument resident,result primary,result hex.
- `rows-tcl9.0` (observation): [runtime/rust/tests/data/native_object_vector/completions/9.0.4.txt](../../../../runtime/rust/tests/data/native_object_vector/completions/9.0.4.txt). Original full output; relevant cases [0, 5]. Columns: case,completion,original argument resident,result primary,result hex.
- `rows-tcl9.1` (observation): [runtime/rust/tests/data/native_object_vector/completions/9.1.0.txt](../../../../runtime/rust/tests/data/native_object_vector/completions/9.1.0.txt). Original full output; relevant cases [0, 5]. Columns: case,completion,original argument resident,result primary,result hex.
- `rows-jim` (observation): [runtime/rust/tests/data/native_object_vector/completions/Jim.txt](../../../../runtime/rust/tests/data/native_object_vector/completions/Jim.txt). Original full output; relevant cases [0, 5]. Columns: case,completion,original argument resident,result primary,result hex.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_object_vector.rs](../../../../runtime/rust/src/interp/native_object_vector.rs), `eval_original_object_vector`: Consumes original borrowed argv through the independently selected evaluator/error/completion protocol; the captured native row does not manufacture a Rust original object.
- [runtime/rust/src/interp/native_object_vector.rs](../../../../runtime/rust/src/interp/native_object_vector.rs), `tests::public_original_return_matches_all_six_native_completion_boundaries` (linked): Compares the six direct case0 return completion/residency/primary/byte windows; additional Rust pointer equality is an implementation assertion rather than a native pointer measurement. Source return rows5 are retained empirical controls, not asserted by this selector.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. The exact probe and captured output can support a new run only after independently pinning the actual release, headers, library and build configuration; capture-time absolute compiler paths are metadata, not a current portable replay runner.

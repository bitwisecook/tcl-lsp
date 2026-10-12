# naming.procedure.borrowed-slot-modified-zero-and-opaque-bytes

Kind: `native-observation`

## Problem statement

Modified-zero native units and FF bytes are original counted names rather than ordinary document text. They must not be collapsed by display decoding or borrowed-slot behavior inferred from raw-zero controls.

## Question

What eval/uplevel/dynamic values follow exact modified-zero and FF original formal names?

## Conclusion

Every recorded provider installs both one/two-formal variants and reaches the body. For these exact pairs, eval/uplevel read DYNAMIC, original dynamic A stays ONE, dynamic B reads DYNAMIC, and the fragment write changes only B to ALTER. The raw bytes and formal axes are retained independently; this proves neither UTF document ingress nor a general numeric/NUL equivalence.

## Scope

Ten exact original counted name pairs enter public C Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector procedure definition with one/two original String formals. The body uses separately returned original A/B name objects, newly generated counted String source through eval/uplevel0, and dynamic reads/writes. Full body.hex and names.tsv match the retained C input. Forty per-provider TSV rows preserve definition/invocation guest code and result hex; their original full JSONL with error/options fields is not retained, so its receipt digest is metadata only. No cache/header/reference-count/native allocation claim. Six original build associations are recorded; Jim patch/revision/configuration and launched C patchlevels are unqueried. BIG-IP not tested; original rust_comparisons is0.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (original capture association; full launched patchlevel unqueried). Build: header_sha256=824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; binary_sha256=30027909fe6c563d7c15855182925f3efd854aff06c19644be12c94b8c201ea4; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation.. Dialect: C Tcl.

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
modified-nul	one-original-formal	original-definition	0	
modified-nul	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
raw-invalid-byte	one-original-formal	original-definition	0	
raw-invalid-byte	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
modified-nul	two-original-formals	original-definition	0	
modified-nul	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
raw-invalid-byte	two-original-formals	original-definition	0	
raw-invalid-byte	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
```
Readable exact returned result bytes:

modified-nul/one-original-formal/original-definition: 
modified-nul/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
raw-invalid-byte/one-original-formal/original-definition: 
raw-invalid-byte/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
modified-nul/two-original-formals/original-definition: 
modified-nul/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
raw-invalid-byte/two-original-formals/original-definition: 
raw-invalid-byte/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER

### tcl8.5

Status: `observed`. Version: 8.5.19 (original capture association; full launched patchlevel unqueried). Build: header_sha256=c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; binary_sha256=697d70b8a45aa755c2d474f324d6d08e87eac3bd7ac1ac4add7d7a87c221f8ff; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation.. Dialect: C Tcl.

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
modified-nul	one-original-formal	original-definition	0	
modified-nul	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
raw-invalid-byte	one-original-formal	original-definition	0	
raw-invalid-byte	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
modified-nul	two-original-formals	original-definition	0	
modified-nul	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
raw-invalid-byte	two-original-formals	original-definition	0	
raw-invalid-byte	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
```
Readable exact returned result bytes:

modified-nul/one-original-formal/original-definition: 
modified-nul/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
raw-invalid-byte/one-original-formal/original-definition: 
raw-invalid-byte/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
modified-nul/two-original-formals/original-definition: 
modified-nul/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
raw-invalid-byte/two-original-formals/original-definition: 
raw-invalid-byte/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER

### tcl8.6

Status: `observed`. Version: 8.6.18 (original capture association; full launched patchlevel unqueried). Build: header_sha256=aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; binary_sha256=482a0061b6dc721117ee293dff903c41046c2f82516bcb37bfea5339a3e213e2; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation.. Dialect: C Tcl.

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
modified-nul	one-original-formal	original-definition	0	
modified-nul	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
raw-invalid-byte	one-original-formal	original-definition	0	
raw-invalid-byte	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
modified-nul	two-original-formals	original-definition	0	
modified-nul	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
raw-invalid-byte	two-original-formals	original-definition	0	
raw-invalid-byte	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
```
Readable exact returned result bytes:

modified-nul/one-original-formal/original-definition: 
modified-nul/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
raw-invalid-byte/one-original-formal/original-definition: 
raw-invalid-byte/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
modified-nul/two-original-formals/original-definition: 
modified-nul/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
raw-invalid-byte/two-original-formals/original-definition: 
raw-invalid-byte/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER

### tcl9.0

Status: `observed`. Version: 9.0.4 (original capture association; full launched patchlevel unqueried). Build: header_sha256=eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; binary_sha256=9e925776725fe923f4f8460552f91749cac752abb0c16eabcba34ab1c6e08935; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation.. Dialect: C Tcl.

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
modified-nul	one-original-formal	original-definition	0	
modified-nul	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
raw-invalid-byte	one-original-formal	original-definition	0	
raw-invalid-byte	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
modified-nul	two-original-formals	original-definition	0	
modified-nul	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
raw-invalid-byte	two-original-formals	original-definition	0	
raw-invalid-byte	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
```
Readable exact returned result bytes:

modified-nul/one-original-formal/original-definition: 
modified-nul/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
raw-invalid-byte/one-original-formal/original-definition: 
raw-invalid-byte/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
modified-nul/two-original-formals/original-definition: 
modified-nul/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
raw-invalid-byte/two-original-formals/original-definition: 
raw-invalid-byte/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER

### tcl9.1

Status: `observed`. Version: 9.1.0 (original capture association; full launched patchlevel unqueried). Build: header_sha256=30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; binary_sha256=17645e6d71e03da712a8e84288a391858d698e3772bd0637daa157814199f6d2; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation.. Dialect: C Tcl.

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
modified-nul	one-original-formal	original-definition	0	
modified-nul	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
raw-invalid-byte	one-original-formal	original-definition	0	
raw-invalid-byte	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
modified-nul	two-original-formals	original-definition	0	
modified-nul	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
raw-invalid-byte	two-original-formals	original-definition	0	
raw-invalid-byte	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
```
Readable exact returned result bytes:

modified-nul/one-original-formal/original-definition: 
modified-nul/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
raw-invalid-byte/one-original-formal/original-definition: 
raw-invalid-byte/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
modified-nul/two-original-formals/original-definition: 
modified-nul/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
raw-invalid-byte/two-original-formals/original-definition: 
raw-invalid-byte/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER

### jim

Status: `observed`. Version: Jim (original patchlevel/revision/configuration unqueried). Build: header_sha256=d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d; library_sha256=75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; binary_sha256=af2947168729205fb1f13ce508e4ce35182f921055959be96c0384f184fbc48d; compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation.. Dialect: Jim Tcl.

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
modified-nul	one-original-formal	original-definition	0	70
modified-nul	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
raw-invalid-byte	one-original-formal	original-definition	0	70
raw-invalid-byte	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
modified-nul	two-original-formals	original-definition	0	70
modified-nul	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
raw-invalid-byte	two-original-formals	original-definition	0	70
raw-invalid-byte	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
```
Readable exact returned result bytes:

modified-nul/one-original-formal/original-definition: p
modified-nul/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
raw-invalid-byte/one-original-formal/original-definition: p
raw-invalid-byte/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
modified-nul/two-original-formals/original-definition: p
modified-nul/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
raw-invalid-byte/two-original-formals/original-definition: p
raw-invalid-byte/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No retained observation of this exact question is attached for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/probe.c](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/probe.c). SHA-256 `1824431d75ce9f200d33848417a993a4ffd03bc8c8671fbf24276be94016a6ce`. Exact public evaluator pathways, original names, body and saved-result observer.
- `names` (input): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/names.tsv](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/names.tsv). SHA-256 `8d2dcb724692aff56ccfee46a2271fe901cb5be285666a8f39efbb5dad65a360`. Exact original name byte pairs matched to source inputs.
- `body` (input): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/body.hex](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/body.hex). SHA-256 `03ecb7ed28ea42cda7677e06024cd45162657b4d269c7e093cfffd73fe500f0c`. Exact counted original body matched to the C literal.
- `receipt` (provider): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/manifest.json](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/manifest.json). SHA-256 `4fcf429d9108979de99e43f95dd3088e58233a0fd149395c5214df2255d1bab6`. Original compile/process and full JSONL digests; absent full raw logs are an explicit replay limit.
- `rows-tcl8.4` (observation): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/8.4.20.tsv). SHA-256 `9c213d188f515f0d8958928b58225d2c6d3a91fb029346065d429a76a2557a0b`. Exact projected five-column guest outcomes; relevant cases ['modified-nul', 'raw-invalid-byte'].
- `rows-tcl8.5` (observation): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/8.5.19.tsv). SHA-256 `11d24c1eeeab0c83452a1d1a830e0bd9d41605bc7368b60ef5f09a76cc41bb0d`. Exact projected five-column guest outcomes; relevant cases ['modified-nul', 'raw-invalid-byte'].
- `rows-tcl8.6` (observation): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/8.6.18.tsv). SHA-256 `6e51bb39da264cb4dce4cb8d464a993fed31b67ecac143197644a7ecac60f59e`. Exact projected five-column guest outcomes; relevant cases ['modified-nul', 'raw-invalid-byte'].
- `rows-tcl9.0` (observation): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/9.0.4.tsv). SHA-256 `6e51bb39da264cb4dce4cb8d464a993fed31b67ecac143197644a7ecac60f59e`. Exact projected five-column guest outcomes; relevant cases ['modified-nul', 'raw-invalid-byte'].
- `rows-tcl9.1` (observation): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/9.1.0.tsv). SHA-256 `4d73b2772e7a1ef1e24271e3af390fb9515dea7cfa753f2b5820f144a50158a0`. Exact projected five-column guest outcomes; relevant cases ['modified-nul', 'raw-invalid-byte'].
- `rows-jim` (observation): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/Jim.tsv](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/Jim.tsv). SHA-256 `aa68708b44799da09ad4b9de6a68e88982e03cefdfede24378eabf3eb01d1c5b`. Exact projected five-column guest outcomes; relevant cases ['modified-nul', 'raw-invalid-byte'].

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/interp/native_compiled_locals.rs](../../../../rust/tcl-vm/src/interp/native_compiled_locals.rs), `compiled_local_binding`: Keeps authentic selected compiled-slot ownership distinct from original dynamic name lookup.
- [rust/tcl-vm/src/interp/native_compiled_locals.rs](../../../../rust/tcl-vm/src/interp/native_compiled_locals.rs), `interp::native_compiled_locals::tests::original_eval_and_uplevel_sources_match_240_borrowed_slot_native_references` (linked): Compares exact 240 projected native definition/invocation references and original byte providers; each selected question is limited to its named cases.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test pass is claimed. A new comparison must retain the exact original program/API flags, independently identify the selected provider build, and capture separate process status/stdout/stderr. Original absolute paths do not constitute a portable runnable replay command. An original raw stream absent from this corpus cannot be validated from its digest alone. Probe source is an input observer, not native implementation source.

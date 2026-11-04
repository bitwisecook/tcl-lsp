# naming.procedure.borrowed-slot-array-shaped-formal-boundary

Kind: `native-observation`

## Problem statement

Array-like original formal names can be rejected at definition or fail only when a body attempts a variable store. The two boundaries give different prior effects and cannot share a generic unsupported result.

## Question

Do ordinary and after-zero array-shaped formals install, and which subsequent body/store or missing-command boundary is observed?

## Conclusion

All C releases reject ordinary a(q)/a(r) at definition. C8.4/8.5 accept after-zero array-shaped names, but the one-formal body fails at its initial dynamic store because k(q) is not an array, and the two-argument call fails arity. C8.6–9.1 reject the after-zero shape at definition. Jim accepts both variants/axes and reaches the distinct A=ONE/B=DYNAMIC then B=ALTER results. No successful C body outcome or physical array-header scope is inferred from rejected cases.

## Scope

Ten exact original counted name pairs enter public C Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector procedure definition with one/two original String formals. The body uses separately returned original A/B name objects, newly generated counted String source through eval/uplevel0, and dynamic reads/writes. Full body.hex and names.tsv match the retained C input. Forty per-provider TSV rows preserve definition/invocation guest code and result hex; their original full JSONL with error/options fields is not retained, so its receipt digest is metadata only. No cache/header/reference-count/native allocation claim. Six original build associations are recorded; Jim patch/revision/configuration and launched C patchlevels are unqueried. BIG-IP not tested; original rust_comparisons is0.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (original capture association; full launched patchlevel unqueried). Build: compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation.. Dialect: C Tcl.

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
opaque-array-after-nul	one-original-formal	original-definition	0	
opaque-array-after-nul	one-original-formal	compiled-and-dynamic-invocation	1	63616e27742073657420226b287129223a207661726961626c652069736e2774206172726179
opaque-array-ordinary	one-original-formal	original-definition	1	70726f636564757265202270222068617320666f726d616c20706172616d6574657220226128712922207468617420697320616e20617272617920656c656d656e74
opaque-array-ordinary	one-original-formal	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
opaque-array-after-nul	two-original-formals	original-definition	0	
opaque-array-after-nul	two-original-formals	compiled-and-dynamic-invocation	1	77726f6e67202320617267733a2073686f756c64206265202270206b22
opaque-array-ordinary	two-original-formals	original-definition	1	70726f636564757265202270222068617320666f726d616c20706172616d6574657220226128712922207468617420697320616e20617272617920656c656d656e74
opaque-array-ordinary	two-original-formals	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
```
Readable exact returned result bytes:

opaque-array-after-nul/one-original-formal/original-definition: 
opaque-array-after-nul/one-original-formal/compiled-and-dynamic-invocation: can't set "k(q)": variable isn't array
opaque-array-ordinary/one-original-formal/original-definition: procedure "p" has formal parameter "a(q)" that is an array element
opaque-array-ordinary/one-original-formal/compiled-and-dynamic-invocation: invalid command name "p"
opaque-array-after-nul/two-original-formals/original-definition: 
opaque-array-after-nul/two-original-formals/compiled-and-dynamic-invocation: wrong # args: should be "p k"
opaque-array-ordinary/two-original-formals/original-definition: procedure "p" has formal parameter "a(q)" that is an array element
opaque-array-ordinary/two-original-formals/compiled-and-dynamic-invocation: invalid command name "p"

### tcl8.5

Status: `observed`. Version: 8.5.19 (original capture association; full launched patchlevel unqueried). Build: compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation.. Dialect: C Tcl.

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
opaque-array-after-nul	one-original-formal	original-definition	0	
opaque-array-after-nul	one-original-formal	compiled-and-dynamic-invocation	1	63616e27742073657420226b287129223a207661726961626c652069736e2774206172726179
opaque-array-ordinary	one-original-formal	original-definition	1	666f726d616c20706172616d657465722022612871292220697320616e20617272617920656c656d656e74
opaque-array-ordinary	one-original-formal	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
opaque-array-after-nul	two-original-formals	original-definition	0	
opaque-array-after-nul	two-original-formals	compiled-and-dynamic-invocation	1	77726f6e67202320617267733a2073686f756c64206265202270206b22
opaque-array-ordinary	two-original-formals	original-definition	1	666f726d616c20706172616d657465722022612871292220697320616e20617272617920656c656d656e74
opaque-array-ordinary	two-original-formals	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
```
Readable exact returned result bytes:

opaque-array-after-nul/one-original-formal/original-definition: 
opaque-array-after-nul/one-original-formal/compiled-and-dynamic-invocation: can't set "k(q)": variable isn't array
opaque-array-ordinary/one-original-formal/original-definition: formal parameter "a(q)" is an array element
opaque-array-ordinary/one-original-formal/compiled-and-dynamic-invocation: invalid command name "p"
opaque-array-after-nul/two-original-formals/original-definition: 
opaque-array-after-nul/two-original-formals/compiled-and-dynamic-invocation: wrong # args: should be "p k"
opaque-array-ordinary/two-original-formals/original-definition: formal parameter "a(q)" is an array element
opaque-array-ordinary/two-original-formals/compiled-and-dynamic-invocation: invalid command name "p"

### tcl8.6

Status: `observed`. Version: 8.6.18 (original capture association; full launched patchlevel unqueried). Build: compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation.. Dialect: C Tcl.

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
opaque-array-after-nul	one-original-formal	original-definition	1	666f726d616c20706172616d6574657220226b2220697320616e20617272617920656c656d656e74
opaque-array-after-nul	one-original-formal	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
opaque-array-ordinary	one-original-formal	original-definition	1	666f726d616c20706172616d657465722022612871292220697320616e20617272617920656c656d656e74
opaque-array-ordinary	one-original-formal	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
opaque-array-after-nul	two-original-formals	original-definition	1	666f726d616c20706172616d6574657220226b2220697320616e20617272617920656c656d656e74
opaque-array-after-nul	two-original-formals	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
opaque-array-ordinary	two-original-formals	original-definition	1	666f726d616c20706172616d657465722022612871292220697320616e20617272617920656c656d656e74
opaque-array-ordinary	two-original-formals	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
```
Readable exact returned result bytes:

opaque-array-after-nul/one-original-formal/original-definition: formal parameter "k" is an array element
opaque-array-after-nul/one-original-formal/compiled-and-dynamic-invocation: invalid command name "p"
opaque-array-ordinary/one-original-formal/original-definition: formal parameter "a(q)" is an array element
opaque-array-ordinary/one-original-formal/compiled-and-dynamic-invocation: invalid command name "p"
opaque-array-after-nul/two-original-formals/original-definition: formal parameter "k" is an array element
opaque-array-after-nul/two-original-formals/compiled-and-dynamic-invocation: invalid command name "p"
opaque-array-ordinary/two-original-formals/original-definition: formal parameter "a(q)" is an array element
opaque-array-ordinary/two-original-formals/compiled-and-dynamic-invocation: invalid command name "p"

### tcl9.0

Status: `observed`. Version: 9.0.4 (original capture association; full launched patchlevel unqueried). Build: compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation.. Dialect: C Tcl.

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
opaque-array-after-nul	one-original-formal	original-definition	1	666f726d616c20706172616d6574657220226b2220697320616e20617272617920656c656d656e74
opaque-array-after-nul	one-original-formal	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
opaque-array-ordinary	one-original-formal	original-definition	1	666f726d616c20706172616d657465722022612871292220697320616e20617272617920656c656d656e74
opaque-array-ordinary	one-original-formal	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
opaque-array-after-nul	two-original-formals	original-definition	1	666f726d616c20706172616d6574657220226b2220697320616e20617272617920656c656d656e74
opaque-array-after-nul	two-original-formals	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
opaque-array-ordinary	two-original-formals	original-definition	1	666f726d616c20706172616d657465722022612871292220697320616e20617272617920656c656d656e74
opaque-array-ordinary	two-original-formals	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
```
Readable exact returned result bytes:

opaque-array-after-nul/one-original-formal/original-definition: formal parameter "k" is an array element
opaque-array-after-nul/one-original-formal/compiled-and-dynamic-invocation: invalid command name "p"
opaque-array-ordinary/one-original-formal/original-definition: formal parameter "a(q)" is an array element
opaque-array-ordinary/one-original-formal/compiled-and-dynamic-invocation: invalid command name "p"
opaque-array-after-nul/two-original-formals/original-definition: formal parameter "k" is an array element
opaque-array-after-nul/two-original-formals/compiled-and-dynamic-invocation: invalid command name "p"
opaque-array-ordinary/two-original-formals/original-definition: formal parameter "a(q)" is an array element
opaque-array-ordinary/two-original-formals/compiled-and-dynamic-invocation: invalid command name "p"

### tcl9.1

Status: `observed`. Version: 9.1.0 (original capture association; full launched patchlevel unqueried). Build: compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation.. Dialect: C Tcl.

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
opaque-array-after-nul	one-original-formal	original-definition	1	666f726d616c20706172616d6574657220226b2220697320616e20617272617920656c656d656e74
opaque-array-after-nul	one-original-formal	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
opaque-array-ordinary	one-original-formal	original-definition	1	666f726d616c20706172616d657465722022612871292220697320616e20617272617920656c656d656e74
opaque-array-ordinary	one-original-formal	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
opaque-array-after-nul	two-original-formals	original-definition	1	666f726d616c20706172616d6574657220226b2220697320616e20617272617920656c656d656e74
opaque-array-after-nul	two-original-formals	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
opaque-array-ordinary	two-original-formals	original-definition	1	666f726d616c20706172616d657465722022612871292220697320616e20617272617920656c656d656e74
opaque-array-ordinary	two-original-formals	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
```
Readable exact returned result bytes:

opaque-array-after-nul/one-original-formal/original-definition: formal parameter "k" is an array element
opaque-array-after-nul/one-original-formal/compiled-and-dynamic-invocation: invalid command name "p"
opaque-array-ordinary/one-original-formal/original-definition: formal parameter "a(q)" is an array element
opaque-array-ordinary/one-original-formal/compiled-and-dynamic-invocation: invalid command name "p"
opaque-array-after-nul/two-original-formals/original-definition: formal parameter "k" is an array element
opaque-array-after-nul/two-original-formals/compiled-and-dynamic-invocation: invalid command name "p"
opaque-array-ordinary/two-original-formals/original-definition: formal parameter "a(q)" is an array element
opaque-array-ordinary/two-original-formals/compiled-and-dynamic-invocation: invalid command name "p"

### jim

Status: `observed`. Version: Jim (original patchlevel/revision/configuration unqueried). Build: compile_exit=0; exit=0. Only this original capture supplies these build fields. Missing configuration/compiler/runtime version fields remain unrecorded.. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation.. Dialect: Jim Tcl.

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
opaque-array-after-nul	one-original-formal	original-definition	0	70
opaque-array-after-nul	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
opaque-array-ordinary	one-original-formal	original-definition	0	70
opaque-array-ordinary	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
opaque-array-after-nul	two-original-formals	original-definition	0	70
opaque-array-after-nul	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
opaque-array-ordinary	two-original-formals	original-definition	0	70
opaque-array-ordinary	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
```
Readable exact returned result bytes:

opaque-array-after-nul/one-original-formal/original-definition: p
opaque-array-after-nul/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
opaque-array-ordinary/one-original-formal/original-definition: p
opaque-array-ordinary/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
opaque-array-after-nul/two-original-formals/original-definition: p
opaque-array-after-nul/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
opaque-array-ordinary/two-original-formals/original-definition: p
opaque-array-ordinary/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No retained observation of this exact question is attached for this provider.

## Historical references

- `probe` (input): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/probe.c](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/probe.c). Exact public evaluator pathways, original names, body and saved-result observer.
- `names` (input): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/names.tsv](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/names.tsv). Exact original name byte pairs matched to source inputs.
- `body` (input): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/body.hex](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/body.hex). Exact counted original body matched to the C literal.
- `receipt` (provider): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/manifest.json](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/manifest.json). Original compile/process and full JSONL digests; absent full raw logs are an explicit replay limit.
- `rows-tcl8.4` (observation): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/8.4.20.tsv). Exact projected five-column guest outcomes; relevant cases ['opaque-array-after-nul', 'opaque-array-ordinary'].
- `rows-tcl8.5` (observation): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/8.5.19.tsv). Exact projected five-column guest outcomes; relevant cases ['opaque-array-after-nul', 'opaque-array-ordinary'].
- `rows-tcl8.6` (observation): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/8.6.18.tsv). Exact projected five-column guest outcomes; relevant cases ['opaque-array-after-nul', 'opaque-array-ordinary'].
- `rows-tcl9.0` (observation): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/9.0.4.tsv). Exact projected five-column guest outcomes; relevant cases ['opaque-array-after-nul', 'opaque-array-ordinary'].
- `rows-tcl9.1` (observation): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/9.1.0.tsv). Exact projected five-column guest outcomes; relevant cases ['opaque-array-after-nul', 'opaque-array-ordinary'].
- `rows-jim` (observation): [rust/tcl-vm/tests/data/native_borrowed_frame_slots/Jim.tsv](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots/Jim.tsv). Exact projected five-column guest outcomes; relevant cases ['opaque-array-after-nul', 'opaque-array-ordinary'].

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/interp/native_compiled_locals.rs](../../../../rust/tcl-vm/src/interp/native_compiled_locals.rs), `compiled_local_binding`: Keeps authentic selected compiled-slot ownership distinct from original dynamic name lookup.
- [rust/tcl-vm/src/interp/native_compiled_locals.rs](../../../../rust/tcl-vm/src/interp/native_compiled_locals.rs), `interp::native_compiled_locals::tests::original_eval_and_uplevel_sources_match_240_borrowed_slot_native_references` (linked): Compares exact 240 projected native definition/invocation references and original byte providers; each selected question is limited to its named cases.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native execution or Rust test pass is claimed. A new comparison must retain the exact original program/API flags, independently identify the selected provider build, and capture separate process status/stdout/stderr. Original absolute paths do not constitute a portable runnable replay command. An original raw stream absent from this corpus cannot be validated from its digest alone. Probe source is an input observer, not native implementation source.

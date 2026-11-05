# naming.procedure.borrowed-slot-counted-zero-length

Kind: `native-observation`

## Problem statement

Two original names may share a zero-terminated prefix but differ in counted suffix and length. A newly compiled eval/uplevel fragment can reuse an existing formal slot while dynamic lookup still reaches another cell.

## Question

Do equal-length and different-length raw-zero original names select the same values through eval/uplevel versus original-object dynamic lookup?

## Conclusion

C8.4/8.5 one-formal controls collapse both raw-zero pairs to DYNAMIC, then ALTER, and reject both two-argument calls. C8.6–9.1 equal-length controls return ONE through eval/uplevel but DYNAMIC through dynamic B; a fragment write changes A to ALTER while B stays DYNAMIC. Different-length pairs keep A at ONE while eval/uplevel read B as DYNAMIC and later change B to ALTER. Jim follows that distinct A/B value pattern for both lengths. These returned values discriminate the reached consumer paths without asserting unobserved header identity.

## Scope

Ten exact original counted name pairs enter public C Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector procedure definition with one/two original String formals. The body uses separately returned original A/B name objects, newly generated counted String source through eval/uplevel0, and dynamic reads/writes. Full body.hex and names.tsv match the retained C input. Forty per-provider TSV rows preserve definition/invocation guest code and result hex; their original full JSONL with error/options fields is not retained, so its receipt digest is metadata only. No cache/header/reference-count/native allocation claim. Six original build associations are recorded; Jim patch/revision/configuration and launched C patchlevels are unqueried. BIG-IP not tested; original rust_comparisons is0.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation. Version: tcl8.4 8.4.20 (original capture association; full launched patchlevel unqueried); tcl8.5 8.5.19 (original capture association; full launched patchlevel unqueried).

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
nul-equal-length	one-original-formal	original-definition	0	
nul-equal-length	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d6963412044594e414d49432064796e616d6963422044594e414d4943207772697465203020414c5445522061667465724120414c5445522061667465724220414c544552
nul-different-length	one-original-formal	original-definition	0	
nul-different-length	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d6963412044594e414d49432064796e616d6963422044594e414d4943207772697465203020414c5445522061667465724120414c5445522061667465724220414c544552
nul-equal-length	two-original-formals	original-definition	0	
nul-equal-length	two-original-formals	compiled-and-dynamic-invocation	1	77726f6e67202320617267733a2073686f756c64206265202270206b22
nul-different-length	two-original-formals	original-definition	0	
nul-different-length	two-original-formals	compiled-and-dynamic-invocation	1	77726f6e67202320617267733a2073686f756c64206265202270206b22
```
Readable exact returned result bytes:

nul-equal-length/one-original-formal/original-definition: 
nul-equal-length/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA DYNAMIC dynamicB DYNAMIC write 0 ALTER afterA ALTER afterB ALTER
nul-different-length/one-original-formal/original-definition: 
nul-different-length/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA DYNAMIC dynamicB DYNAMIC write 0 ALTER afterA ALTER afterB ALTER
nul-equal-length/two-original-formals/original-definition: 
nul-equal-length/two-original-formals/compiled-and-dynamic-invocation: wrong # args: should be "p k"
nul-different-length/two-original-formals/original-definition: 
nul-different-length/two-original-formals/compiled-and-dynamic-invocation: wrong # args: should be "p k"

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation. Version: tcl8.6 8.6.18 (original capture association; full launched patchlevel unqueried); tcl9.0 9.0.4 (original capture association; full launched patchlevel unqueried); tcl9.1 9.1.0 (original capture association; full launched patchlevel unqueried).

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
nul-equal-length	one-original-formal	original-definition	0	
nul-equal-length	one-original-formal	compiled-and-dynamic-invocation	0	6576616c2030204f4e452075706c6576656c2030204f4e452064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c5445522061667465724120414c544552206166746572422044594e414d4943
nul-different-length	one-original-formal	original-definition	0	
nul-different-length	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
nul-equal-length	two-original-formals	original-definition	0	
nul-equal-length	two-original-formals	compiled-and-dynamic-invocation	0	6576616c2030204f4e452075706c6576656c2030204f4e452064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c5445522061667465724120414c544552206166746572422044594e414d4943
nul-different-length	two-original-formals	original-definition	0	
nul-different-length	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
```
Readable exact returned result bytes:

nul-equal-length/one-original-formal/original-definition: 
nul-equal-length/one-original-formal/compiled-and-dynamic-invocation: eval 0 ONE uplevel 0 ONE dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ALTER afterB DYNAMIC
nul-different-length/one-original-formal/original-definition: 
nul-different-length/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
nul-equal-length/two-original-formals/original-definition: 
nul-equal-length/two-original-formals/compiled-and-dynamic-invocation: eval 0 ONE uplevel 0 ONE dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ALTER afterB DYNAMIC
nul-different-length/two-original-formals/original-definition: 
nul-different-length/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER

### jim

Status: `observed`. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation. Version: jim Jim (original patchlevel/revision/configuration unqueried).

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
nul-equal-length	one-original-formal	original-definition	0	70
nul-equal-length	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
nul-different-length	one-original-formal	original-definition	0	70
nul-different-length	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
nul-equal-length	two-original-formals	original-definition	0	70
nul-equal-length	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
nul-different-length	two-original-formals	original-definition	0	70
nul-different-length	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
```
Readable exact returned result bytes:

nul-equal-length/one-original-formal/original-definition: p
nul-equal-length/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
nul-different-length/one-original-formal/original-definition: p
nul-different-length/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
nul-equal-length/two-original-formals/original-definition: p
nul-equal-length/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
nul-different-length/two-original-formals/original-definition: p
nul-different-length/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER

### bigip

Status: `not-tested`.

No retained observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-vm/src/interp/native_compiled_locals.rs](../../../../rust/tcl-vm/src/interp/native_compiled_locals.rs), `compiled_local_binding`: Keeps authentic selected compiled-slot ownership distinct from original dynamic name lookup.
- [rust/tcl-vm/src/interp/native_compiled_locals.rs](../../../../rust/tcl-vm/src/interp/native_compiled_locals.rs), `interp::native_compiled_locals::tests::original_eval_and_uplevel_sources_match_240_borrowed_slot_native_references` (linked): Compares exact 240 projected native definition/invocation references and original byte providers; each selected question is limited to its named cases.

## Evidence

- [rust/tcl-vm/tests/data/native_borrowed_frame_slots](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots)

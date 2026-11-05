# naming.procedure.borrowed-slot-duplicate-original-formals

Kind: `native-observation`

## Problem statement

Repeated ordinary or counted-zero formals can install successfully yet accept different argv counts. A later eval/uplevel fragment and dynamic original-object lookup can touch the same slot or a different cell; displayed equality cannot decide either axis.

## Question

How do one/two identical ordinary or raw-zero formals affect invocation and the measured eval/uplevel/dynamic read/write results?

## Conclusion

All ordinary duplicate controls install and return DYNAMIC through eval, uplevel and both dynamic reads, then ALTER through both final reads. Identical raw-zero duplicates behave the same with one formal; C8.4/8.5 reject the two-argument invocation with usage p k, while C8.6–9.1 and Jim reach the body and return the same measured values. This is accepted argv and returned values for these two exact declarations, not proof of physical duplicate-slot allocation.

## Scope

Ten exact original counted name pairs enter public C Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector procedure definition with one/two original String formals. The body uses separately returned original A/B name objects, newly generated counted String source through eval/uplevel0, and dynamic reads/writes. Full body.hex and names.tsv match the retained C input. Forty per-provider TSV rows preserve definition/invocation guest code and result hex; their original full JSONL with error/options fields is not retained, so its receipt digest is metadata only. No cache/header/reference-count/native allocation claim. Six original build associations are recorded; Jim patch/revision/configuration and launched C patchlevels are unqueried. BIG-IP not tested; original rust_comparisons is0.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation. Version: tcl8.4 8.4.20 (original capture association; full launched patchlevel unqueried); tcl8.5 8.5.19 (original capture association; full launched patchlevel unqueried).

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
ordinary-duplicate	one-original-formal	original-definition	0	
ordinary-duplicate	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d6963412044594e414d49432064796e616d6963422044594e414d4943207772697465203020414c5445522061667465724120414c5445522061667465724220414c544552
identical-duplicate	one-original-formal	original-definition	0	
identical-duplicate	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d6963412044594e414d49432064796e616d6963422044594e414d4943207772697465203020414c5445522061667465724120414c5445522061667465724220414c544552
ordinary-duplicate	two-original-formals	original-definition	0	
ordinary-duplicate	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d6963412044594e414d49432064796e616d6963422044594e414d4943207772697465203020414c5445522061667465724120414c5445522061667465724220414c544552
identical-duplicate	two-original-formals	original-definition	0	
identical-duplicate	two-original-formals	compiled-and-dynamic-invocation	1	77726f6e67202320617267733a2073686f756c64206265202270206b22
```
Readable exact returned result bytes:

ordinary-duplicate/one-original-formal/original-definition: 
ordinary-duplicate/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA DYNAMIC dynamicB DYNAMIC write 0 ALTER afterA ALTER afterB ALTER
identical-duplicate/one-original-formal/original-definition: 
identical-duplicate/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA DYNAMIC dynamicB DYNAMIC write 0 ALTER afterA ALTER afterB ALTER
ordinary-duplicate/two-original-formals/original-definition: 
ordinary-duplicate/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA DYNAMIC dynamicB DYNAMIC write 0 ALTER afterA ALTER afterB ALTER
identical-duplicate/two-original-formals/original-definition: 
identical-duplicate/two-original-formals/compiled-and-dynamic-invocation: wrong # args: should be "p k"

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation. Version: tcl8.6 8.6.18 (original capture association; full launched patchlevel unqueried); tcl9.0 9.0.4 (original capture association; full launched patchlevel unqueried); tcl9.1 9.1.0 (original capture association; full launched patchlevel unqueried).

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
ordinary-duplicate	one-original-formal	original-definition	0	
ordinary-duplicate	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d6963412044594e414d49432064796e616d6963422044594e414d4943207772697465203020414c5445522061667465724120414c5445522061667465724220414c544552
identical-duplicate	one-original-formal	original-definition	0	
identical-duplicate	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d6963412044594e414d49432064796e616d6963422044594e414d4943207772697465203020414c5445522061667465724120414c5445522061667465724220414c544552
ordinary-duplicate	two-original-formals	original-definition	0	
ordinary-duplicate	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d6963412044594e414d49432064796e616d6963422044594e414d4943207772697465203020414c5445522061667465724120414c5445522061667465724220414c544552
identical-duplicate	two-original-formals	original-definition	0	
identical-duplicate	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d6963412044594e414d49432064796e616d6963422044594e414d4943207772697465203020414c5445522061667465724120414c5445522061667465724220414c544552
```
Readable exact returned result bytes:

ordinary-duplicate/one-original-formal/original-definition: 
ordinary-duplicate/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA DYNAMIC dynamicB DYNAMIC write 0 ALTER afterA ALTER afterB ALTER
identical-duplicate/one-original-formal/original-definition: 
identical-duplicate/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA DYNAMIC dynamicB DYNAMIC write 0 ALTER afterA ALTER afterB ALTER
ordinary-duplicate/two-original-formals/original-definition: 
ordinary-duplicate/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA DYNAMIC dynamicB DYNAMIC write 0 ALTER afterA ALTER afterB ALTER
identical-duplicate/two-original-formals/original-definition: 
identical-duplicate/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA DYNAMIC dynamicB DYNAMIC write 0 ALTER afterA ALTER afterB ALTER

### jim

Status: `observed`. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation. Version: jim Jim (original patchlevel/revision/configuration unqueried).

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
ordinary-duplicate	one-original-formal	original-definition	0	70
ordinary-duplicate	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d6963412044594e414d49432064796e616d6963422044594e414d4943207772697465203020414c5445522061667465724120414c5445522061667465724220414c544552
identical-duplicate	one-original-formal	original-definition	0	70
identical-duplicate	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d6963412044594e414d49432064796e616d6963422044594e414d4943207772697465203020414c5445522061667465724120414c5445522061667465724220414c544552
ordinary-duplicate	two-original-formals	original-definition	0	70
ordinary-duplicate	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d6963412044594e414d49432064796e616d6963422044594e414d4943207772697465203020414c5445522061667465724120414c5445522061667465724220414c544552
identical-duplicate	two-original-formals	original-definition	0	70
identical-duplicate	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d6963412044594e414d49432064796e616d6963422044594e414d4943207772697465203020414c5445522061667465724120414c5445522061667465724220414c544552
```
Readable exact returned result bytes:

ordinary-duplicate/one-original-formal/original-definition: p
ordinary-duplicate/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA DYNAMIC dynamicB DYNAMIC write 0 ALTER afterA ALTER afterB ALTER
identical-duplicate/one-original-formal/original-definition: p
identical-duplicate/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA DYNAMIC dynamicB DYNAMIC write 0 ALTER afterA ALTER afterB ALTER
ordinary-duplicate/two-original-formals/original-definition: p
ordinary-duplicate/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA DYNAMIC dynamicB DYNAMIC write 0 ALTER afterA ALTER afterB ALTER
identical-duplicate/two-original-formals/original-definition: p
identical-duplicate/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA DYNAMIC dynamicB DYNAMIC write 0 ALTER afterA ALTER afterB ALTER

### bigip

Status: `not-tested`.

No retained observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-vm/src/interp/native_compiled_locals.rs](../../../../rust/tcl-vm/src/interp/native_compiled_locals.rs), `compiled_local_binding`: Keeps authentic selected compiled-slot ownership distinct from original dynamic name lookup.
- [rust/tcl-vm/src/interp/native_compiled_locals.rs](../../../../rust/tcl-vm/src/interp/native_compiled_locals.rs), `interp::native_compiled_locals::tests::original_eval_and_uplevel_sources_match_240_borrowed_slot_native_references` (linked): Compares exact 240 projected native definition/invocation references and original byte providers; each selected question is limited to its named cases.

## Evidence

- [rust/tcl-vm/tests/data/native_borrowed_frame_slots](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots)

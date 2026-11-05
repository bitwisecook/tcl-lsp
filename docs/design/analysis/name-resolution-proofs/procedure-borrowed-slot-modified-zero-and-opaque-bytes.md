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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation. Version: tcl8.4 8.4.20 (original capture association; full launched patchlevel unqueried); tcl8.5 8.5.19 (original capture association; full launched patchlevel unqueried); tcl8.6 8.6.18 (original capture association; full launched patchlevel unqueried); tcl9.0 9.0.4 (original capture association; full launched patchlevel unqueried); tcl9.1 9.1.0 (original capture association; full launched patchlevel unqueried).

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

Status: `observed`. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation. Version: jim Jim (original patchlevel/revision/configuration unqueried).

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

Status: `not-tested`.

No retained observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-vm/src/interp/native_compiled_locals.rs](../../../../rust/tcl-vm/src/interp/native_compiled_locals.rs), `compiled_local_binding`: Keeps authentic selected compiled-slot ownership distinct from original dynamic name lookup.
- [rust/tcl-vm/src/interp/native_compiled_locals.rs](../../../../rust/tcl-vm/src/interp/native_compiled_locals.rs), `interp::native_compiled_locals::tests::original_eval_and_uplevel_sources_match_240_borrowed_slot_native_references` (linked): Compares exact 240 projected native definition/invocation references and original byte providers; each selected question is limited to its named cases.

## Evidence

- [rust/tcl-vm/tests/data/native_borrowed_frame_slots](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots)

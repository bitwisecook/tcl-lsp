# naming.procedure.borrowed-slot-qualified-formal-boundary

Kind: `native-observation`

## Problem statement

Namespace separators on either side of raw zero can be validated before a procedure is installed. An invocation error following failed definition must not be described as body or compiled-slot execution.

## Question

Which qualified raw-zero formal declarations install, and what exact subsequent eval/uplevel invocation is reached?

## Conclusion

C8.4/8.5 accept qualification after raw zero with one formal and return the collapsed DYNAMIC/ALTER values, but reject its two-argument call; qualification before zero fails definition. C8.6–9.1 reject both qualification positions at definition. Each rejected definition is followed by invalid-command p. Jim installs both positions/axes and reaches the distinct A=ONE/B=DYNAMIC then B=ALTER body results. Exact error bytes remain separate from counted validation inputs.

## Scope

Ten exact original counted name pairs enter public C Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector procedure definition with one/two original String formals. The body uses separately returned original A/B name objects, newly generated counted String source through eval/uplevel0, and dynamic reads/writes. Full body.hex and names.tsv match the retained C input. Forty per-provider TSV rows preserve definition/invocation guest code and result hex; their original full JSONL with error/options fields is not retained, so its receipt digest is metadata only. No cache/header/reference-count/native allocation claim. Six original build associations are recorded; Jim patch/revision/configuration and launched C patchlevels are unqueried. BIG-IP not tested; original rust_comparisons is0.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation. Version: tcl8.4 8.4.20 (original capture association; full launched patchlevel unqueried).

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
qualification-after-nul	one-original-formal	original-definition	0	
qualification-after-nul	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d6963412044594e414d49432064796e616d6963422044594e414d4943207772697465203020414c5445522061667465724120414c5445522061667465724220414c544552
qualification-before-nul	one-original-formal	original-definition	1	70726f636564757265202270222068617320666f726d616c20706172616d6574657220226b3a3a222074686174206973206e6f7420612073696d706c65206e616d65
qualification-before-nul	one-original-formal	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
qualification-after-nul	two-original-formals	original-definition	0	
qualification-after-nul	two-original-formals	compiled-and-dynamic-invocation	1	77726f6e67202320617267733a2073686f756c64206265202270206b22
qualification-before-nul	two-original-formals	original-definition	1	70726f636564757265202270222068617320666f726d616c20706172616d6574657220226b3a3a222074686174206973206e6f7420612073696d706c65206e616d65
qualification-before-nul	two-original-formals	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
```
Readable exact returned result bytes:

qualification-after-nul/one-original-formal/original-definition: 
qualification-after-nul/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA DYNAMIC dynamicB DYNAMIC write 0 ALTER afterA ALTER afterB ALTER
qualification-before-nul/one-original-formal/original-definition: procedure "p" has formal parameter "k::" that is not a simple name
qualification-before-nul/one-original-formal/compiled-and-dynamic-invocation: invalid command name "p"
qualification-after-nul/two-original-formals/original-definition: 
qualification-after-nul/two-original-formals/compiled-and-dynamic-invocation: wrong # args: should be "p k"
qualification-before-nul/two-original-formals/original-definition: procedure "p" has formal parameter "k::" that is not a simple name
qualification-before-nul/two-original-formals/compiled-and-dynamic-invocation: invalid command name "p"

### tcl8.5

Status: `observed`. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation. Version: tcl8.5 8.5.19 (original capture association; full launched patchlevel unqueried).

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
qualification-after-nul	one-original-formal	original-definition	0	
qualification-after-nul	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d6963412044594e414d49432064796e616d6963422044594e414d4943207772697465203020414c5445522061667465724120414c5445522061667465724220414c544552
qualification-before-nul	one-original-formal	original-definition	1	666f726d616c20706172616d6574657220226b3a3a22206973206e6f7420612073696d706c65206e616d65
qualification-before-nul	one-original-formal	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
qualification-after-nul	two-original-formals	original-definition	0	
qualification-after-nul	two-original-formals	compiled-and-dynamic-invocation	1	77726f6e67202320617267733a2073686f756c64206265202270206b22
qualification-before-nul	two-original-formals	original-definition	1	666f726d616c20706172616d6574657220226b3a3a22206973206e6f7420612073696d706c65206e616d65
qualification-before-nul	two-original-formals	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
```
Readable exact returned result bytes:

qualification-after-nul/one-original-formal/original-definition: 
qualification-after-nul/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA DYNAMIC dynamicB DYNAMIC write 0 ALTER afterA ALTER afterB ALTER
qualification-before-nul/one-original-formal/original-definition: formal parameter "k::" is not a simple name
qualification-before-nul/one-original-formal/compiled-and-dynamic-invocation: invalid command name "p"
qualification-after-nul/two-original-formals/original-definition: 
qualification-after-nul/two-original-formals/compiled-and-dynamic-invocation: wrong # args: should be "p k"
qualification-before-nul/two-original-formals/original-definition: formal parameter "k::" is not a simple name
qualification-before-nul/two-original-formals/compiled-and-dynamic-invocation: invalid command name "p"

### tcl8.6, tcl9.0

Status: `observed`. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation. Version: tcl8.6 8.6.18 (original capture association; full launched patchlevel unqueried); tcl9.0 9.0.4 (original capture association; full launched patchlevel unqueried).

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
qualification-after-nul	one-original-formal	original-definition	1	666f726d616c20706172616d6574657220226b003a3a6122206973206e6f7420612073696d706c65206e616d65
qualification-after-nul	one-original-formal	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
qualification-before-nul	one-original-formal	original-definition	1	666f726d616c20706172616d6574657220226b3a3a006122206973206e6f7420612073696d706c65206e616d65
qualification-before-nul	one-original-formal	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
qualification-after-nul	two-original-formals	original-definition	1	666f726d616c20706172616d6574657220226b003a3a6122206973206e6f7420612073696d706c65206e616d65
qualification-after-nul	two-original-formals	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
qualification-before-nul	two-original-formals	original-definition	1	666f726d616c20706172616d6574657220226b3a3a006122206973206e6f7420612073696d706c65206e616d65
qualification-before-nul	two-original-formals	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
```
Readable exact returned result bytes:

qualification-after-nul/one-original-formal/original-definition: formal parameter "k\x00::a" is not a simple name
qualification-after-nul/one-original-formal/compiled-and-dynamic-invocation: invalid command name "p"
qualification-before-nul/one-original-formal/original-definition: formal parameter "k::\x00a" is not a simple name
qualification-before-nul/one-original-formal/compiled-and-dynamic-invocation: invalid command name "p"
qualification-after-nul/two-original-formals/original-definition: formal parameter "k\x00::a" is not a simple name
qualification-after-nul/two-original-formals/compiled-and-dynamic-invocation: invalid command name "p"
qualification-before-nul/two-original-formals/original-definition: formal parameter "k::\x00a" is not a simple name
qualification-before-nul/two-original-formals/compiled-and-dynamic-invocation: invalid command name "p"

### tcl9.1

Status: `observed`. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation. Version: tcl9.1 9.1.0 (original capture association; full launched patchlevel unqueried).

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
qualification-after-nul	one-original-formal	original-definition	1	666f726d616c20706172616d6574657220226b22206973206e6f7420612073696d706c65206e616d65
qualification-after-nul	one-original-formal	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
qualification-before-nul	one-original-formal	original-definition	1	666f726d616c20706172616d6574657220226b3a3a22206973206e6f7420612073696d706c65206e616d65
qualification-before-nul	one-original-formal	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
qualification-after-nul	two-original-formals	original-definition	1	666f726d616c20706172616d6574657220226b22206973206e6f7420612073696d706c65206e616d65
qualification-after-nul	two-original-formals	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
qualification-before-nul	two-original-formals	original-definition	1	666f726d616c20706172616d6574657220226b3a3a22206973206e6f7420612073696d706c65206e616d65
qualification-before-nul	two-original-formals	compiled-and-dynamic-invocation	1	696e76616c696420636f6d6d616e64206e616d6520227022
```
Readable exact returned result bytes:

qualification-after-nul/one-original-formal/original-definition: formal parameter "k" is not a simple name
qualification-after-nul/one-original-formal/compiled-and-dynamic-invocation: invalid command name "p"
qualification-before-nul/one-original-formal/original-definition: formal parameter "k::" is not a simple name
qualification-before-nul/one-original-formal/compiled-and-dynamic-invocation: invalid command name "p"
qualification-after-nul/two-original-formals/original-definition: formal parameter "k" is not a simple name
qualification-after-nul/two-original-formals/compiled-and-dynamic-invocation: invalid command name "p"
qualification-before-nul/two-original-formals/original-definition: formal parameter "k::" is not a simple name
qualification-before-nul/two-original-formals/compiled-and-dynamic-invocation: invalid command name "p"

### jim

Status: `observed`. Channel: Original counted String/List object-vector proc definition/invocation; counted original source objects enter eval/uplevel compilation. Version: jim Jim (original patchlevel/revision/configuration unqueried).

Exact projected original rows (case, formal axis, operation, guest code, result hex):

```tsv
qualification-after-nul	one-original-formal	original-definition	0	70
qualification-after-nul	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
qualification-before-nul	one-original-formal	original-definition	0	70
qualification-before-nul	one-original-formal	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
qualification-after-nul	two-original-formals	original-definition	0	70
qualification-after-nul	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
qualification-before-nul	two-original-formals	original-definition	0	70
qualification-before-nul	two-original-formals	compiled-and-dynamic-invocation	0	6576616c20302044594e414d49432075706c6576656c20302044594e414d49432064796e616d696341204f4e452064796e616d6963422044594e414d4943207772697465203020414c54455220616674657241204f4e452061667465724220414c544552
```
Readable exact returned result bytes:

qualification-after-nul/one-original-formal/original-definition: p
qualification-after-nul/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
qualification-before-nul/one-original-formal/original-definition: p
qualification-before-nul/one-original-formal/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
qualification-after-nul/two-original-formals/original-definition: p
qualification-after-nul/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER
qualification-before-nul/two-original-formals/original-definition: p
qualification-before-nul/two-original-formals/compiled-and-dynamic-invocation: eval 0 DYNAMIC uplevel 0 DYNAMIC dynamicA ONE dynamicB DYNAMIC write 0 ALTER afterA ONE afterB ALTER

### bigip

Status: `not-tested`.

No retained observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-vm/src/interp/native_compiled_locals.rs](../../../../rust/tcl-vm/src/interp/native_compiled_locals.rs), `compiled_local_binding`: Keeps authentic selected compiled-slot ownership distinct from original dynamic name lookup.
- [rust/tcl-vm/src/interp/native_compiled_locals.rs](../../../../rust/tcl-vm/src/interp/native_compiled_locals.rs), `interp::native_compiled_locals::tests::original_eval_and_uplevel_sources_match_240_borrowed_slot_native_references` (linked): Compares exact 240 projected native definition/invocation references and original byte providers; each selected question is limited to its named cases.

## Evidence

- [rust/tcl-vm/tests/data/native_borrowed_frame_slots](../../../../rust/tcl-vm/tests/data/native_borrowed_frame_slots)

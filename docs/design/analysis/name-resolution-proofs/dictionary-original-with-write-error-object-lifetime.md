# naming.dictionary.original-with-write-error-object-lifetime

Kind: `native-observation`

## Problem statement

The compiled-with write-error cleanup has a distinct dictionary allocation/release owner. Captured later caller fields cannot establish defined storage; a precise conclusion requires live selection/refcount checks, reached free evidence and separately preserved later exit/signal outcomes.

## Question

For the unchanged original compiled-with write-error source on independently pinned current C8.6/C9.0/C9.1 builds, does TclDictWithFinish allocate a refcount0 header, leave the Var selecting a defined scalar after failed set, and reach final free before later reporting; how do plain-with and independent update controls differ?

## Conclusion

On each independently built C8.6.18/C9.0.4/C9.1.0 observer, original compiled-with-write-error case19 reaches TclDictWithFinish with refcount=0 and allocated=1. Immediately after failed TclPtrSetVarIdx, the Var still selects the same defined scalar (same_cell=1, defined_scalar=1), with no watched free yet. The unchanged allocated-dictionary decrement then reaches watched TclFreeObj: final_free=1 before the body-return integer report. Every later original guest catch/result/diagnostic/read-status/outer field is a captured post-free process observation, not a defined storage or completion/read/effect guarantee. C8.6 baseline and observer case19 exit0 with byte-identical whole stdout; C9.0/C9.1 baseline and observer case19 retain that stdout but later terminate by signal4 with their original allocation diagnostics. Plain with case10 records failed=0 and final_free=0 at body return, then a later cleanup free; all three providers exit0. Independent update-error case9 records entered=0 for this With observer: C8.6 exits0, C9.0/C9.1 later signal4, without a With selection/lifetime conclusion. The39 exact closed operations comprise18 build/compile steps and21 processes (18 case runs plus3 independently successful version queries);31 exit0 and8 signal4. No successful aggregate result, generic dictionary lifetime, safe freed-byte reproduction, old executable private-header identity, Native frame/handler/compiler permission or Rust pass follows.

Two current VM software bindings retain the unchanged329 observations with326 whole result comparisons and three bounded captured post-free diagnostic/read-status windows. The independent With host-storage control checks the exact405 integer/raw C8.6 witness and plain/update controls, then checks its own retained live BASE dictionary header/member under the same unchanged guest. That software storage policy is independent of native freed contents.

## Scope

The exact original probe and compiled-with-write-error guest from Native329 are unchanged; each fresh process receives only original case index19, plain-with10 or independent update-error9. Original counted Tcl_EvalObjEx uses Tcl_CreateInterp without Tcl_Init; this does not measure full-distribution initialisation. Original SDK sources/headers/configured Makefiles/archives and baseline/version executables remain separate from independently compiled observed tclDictObj.o/tclObj.o, patched archive and observer executable. The observer reads the live actual refcount and allocdict before set, then compares the Var pointer and scalar/undefined flags after set failure; it does not dereference a released header, retain the watched header, or modify Tcl ownership. The free-entry hook increments an integer and clears the watch before ordinary freeing. The probe reports saved integers before source-object release and interpreter teardown. Complete requests, original/observed sources and patches, build templates, original/observer objects/archives/ELFs, command receipts, versions, raw streams and signal diagnostics are retained byte-for-byte. Source inspection explains this exact current observer path and does not certify the private state of the older archived329 executable. C8.4, C8.5, Jim and BIG-IP are not tested by this request. UpdateEnd398/399 and their watched counters are independent questions; no result is transferred between cleanup owners.

These current definitions have no fresh Rust assertion receipt. The C9 baseline/observer signals remain original capture outcomes without invented software expectations. Retaining outer/diagnostic/caller-read fields does not turn post-free output into defined Native completion/storage/read/effect or another executable's private lifetime.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Exact original WithFinish write-error object lifetime.

No process or WithFinish lifetime observation for this provider is executed by request405.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Exact original WithFinish write-error object lifetime.

No process or WithFinish lifetime observation for this provider is executed by request405.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original archive SHA 980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb; separate observer archive SHA e7705daca0352994e13625981cc957c1e53940ea8a0ce99b7d5dea0c3e2d2c79. Actual original sources/headers/configured Makefile, original and observed objects/archives/executables and exact command receipts retained. Compiler version and inherited environment unrecorded.. Channel: Unchanged original counted C API source Tcl_CreateInterp/Tcl_EvalObjEx without Tcl_Init; distinct live integer observer counters, exact case processes and public streams.. Dialect: Actual independently queried original C release; exact WithFinish writeback cleanup purpose.

Case19 live pre-set refcount0/allocated1; failed set retains same defined scalar with free0, then watched free1 before body-return report. Baseline/observer case19 exit0 and whole stdout match; all later fields are post-free process observations. Plain-with10 succeeds with free0 at body return and later cleanupfree1. Independent update9 does not enter With; it exits0.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original archive SHA dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4; separate observer archive SHA 9e27978cbf0ac0b2a38da1bb1718ea7c76f452507b9acfb0bad20a0449956471. Actual original sources/headers/configured Makefile, original and observed objects/archives/executables and exact command receipts retained. Compiler version and inherited environment unrecorded.. Channel: Unchanged original counted C API source Tcl_CreateInterp/Tcl_EvalObjEx without Tcl_Init; distinct live integer observer counters, exact case processes and public streams.. Dialect: Actual independently queried original C release; exact WithFinish writeback cleanup purpose.

Case19 live pre-set refcount0/allocated1; failed set retains same defined scalar with free0, then watched free1 before body-return report. Baseline/observer case19 whole stdout match but both later terminate by signal4; raw allocation diagnostics remain exact, not a successful completion. Plain-with10 succeeds with free0 at body return and later cleanupfree1. Independent update9 does not enter With; it later signals4.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original archive SHA 513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db; separate observer archive SHA 89f6c90d95d5e55c9d38710a4b554794727bceaa72f9f41573639392ede575bb. Actual original sources/headers/configured Makefile, original and observed objects/archives/executables and exact command receipts retained. Compiler version and inherited environment unrecorded.. Channel: Unchanged original counted C API source Tcl_CreateInterp/Tcl_EvalObjEx without Tcl_Init; distinct live integer observer counters, exact case processes and public streams.. Dialect: Actual independently queried original C release; exact WithFinish writeback cleanup purpose.

Case19 live pre-set refcount0/allocated1; failed set retains same defined scalar with free0, then watched free1 before body-return report. Baseline/observer case19 whole stdout match but both later terminate by signal4; raw allocation diagnostics remain exact, not a successful completion. Plain-with10 succeeds with free0 at body return and later cleanupfree1. Independent update9 does not enter With; it later signals4.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Exact original WithFinish write-error object lifetime.

No process or WithFinish lifetime observation for this provider is executed by request405.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Exact original WithFinish write-error object lifetime.

No process or WithFinish lifetime observation for this provider is executed by request405.

## Historical references

- `native_dict_with_write_error_lifetime405-request-8.6.18-observed-tclDictObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/observed-tclDictObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/observed-tclDictObj.c). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-8.6.18-observed-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/observed-tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/observed-tclObj.c). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-8.6.18-original-tclDictObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/original-tclDictObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/original-tclDictObj.c). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-8.6.18-original-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/original-tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/original-tclObj.c). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-8.6.18-tclDictObj.c.patch` (input): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/tclDictObj.c.patch](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/tclDictObj.c.patch). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-8.6.18-tclObj.c.patch` (input): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/tclObj.c.patch](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/tclObj.c.patch). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.0.4-observed-tclDictObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/observed-tclDictObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/observed-tclDictObj.c). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.0.4-observed-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/observed-tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/observed-tclObj.c). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.0.4-original-tclDictObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/original-tclDictObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/original-tclDictObj.c). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.0.4-original-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/original-tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/original-tclObj.c). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.0.4-tclDictObj.c.patch` (input): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/tclDictObj.c.patch](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/tclDictObj.c.patch). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.0.4-tclObj.c.patch` (input): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/tclObj.c.patch](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/tclObj.c.patch). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.1.0-observed-tclDictObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/observed-tclDictObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/observed-tclDictObj.c). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.1.0-observed-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/observed-tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/observed-tclObj.c). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.1.0-original-tclDictObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/original-tclDictObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/original-tclDictObj.c). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.1.0-original-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/original-tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/original-tclObj.c). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.1.0-tclDictObj.c.patch` (input): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/tclDictObj.c.patch](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/tclDictObj.c.patch). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.1.0-tclObj.c.patch` (input): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/tclObj.c.patch](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/tclObj.c.patch). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-Makefile.observer.template` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/Makefile.observer.template](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/Makefile.observer.template). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-Makefile.preview` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/Makefile.preview](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/Makefile.preview). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-capture.py` (input): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/capture.py](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/capture.py). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-observed-probe.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/observed-probe.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/observed-probe.c). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-original-probe.c` (input): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/original-probe.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/original-probe.c). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-original-source.tcl` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/original-source.tcl](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/original-source.tcl). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-request.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/request.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/request.json). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-stage.py` (input): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/stage.py](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/stage.py). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-version-template.c` (input): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/version-template.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/version-template.c). Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-8.6.18-Makefile.observer` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/Makefile.observer](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/Makefile.observer). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-archive-copy.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/archive-copy.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/archive-copy.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-case10.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case10.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case10.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-case10.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case10.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case10.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-case10.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case10.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case10.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case19.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case19.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case19.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case9.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case9.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case9.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-compile.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-compile.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-compile.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-probe` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-probe](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-probe). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-build-archive.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-archive.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-archive.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-build-archive.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-archive.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-archive.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-build-archive.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-archive.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-archive.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-build-dictionary.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-dictionary.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-dictionary.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-build-dictionary.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-dictionary.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-dictionary.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-build-dictionary.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-dictionary.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-dictionary.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-build-object.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-object.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-object.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-build-object.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-object.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-object.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-build-object.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-object.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-object.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-libtcl8.6.a` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/libtcl8.6.a](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/libtcl8.6.a). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-case10.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case10.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case10.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-case10.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case10.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case10.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-case10.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case10.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case10.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case19.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case19.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case19.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case9.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case9.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case9.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-compile.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-compile.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-compile.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-probe` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-probe](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-probe). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-summary.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/summary.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/summary.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-tclDictObj.o` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/tclDictObj.o](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/tclDictObj.o). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-tclObj.o` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/tclObj.o](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/tclObj.o). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-version-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version-compile.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-version-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version-compile.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-version-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version-compile.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-version-probe` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version-probe](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version-probe). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-version.c` (input): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version.c). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-version.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-version.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-version.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-Makefile.observer` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/Makefile.observer](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/Makefile.observer). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-archive-copy.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/archive-copy.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/archive-copy.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-case10.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case10.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case10.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-case10.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case10.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case10.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-case10.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case10.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case10.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case19.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case19.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case19.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case9.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case9.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case9.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-compile.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-compile.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-compile.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-probe` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-probe](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-probe). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-build-archive.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-archive.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-archive.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-build-archive.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-archive.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-archive.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-build-archive.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-archive.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-archive.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-build-dictionary.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-dictionary.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-dictionary.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-build-dictionary.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-dictionary.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-dictionary.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-build-dictionary.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-dictionary.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-dictionary.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-build-object.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-object.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-object.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-build-object.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-object.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-object.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-build-object.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-object.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-object.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-libtcl9.0.a` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/libtcl9.0.a](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/libtcl9.0.a). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-case10.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case10.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case10.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-case10.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case10.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case10.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-case10.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case10.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case10.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case19.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case19.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case19.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case9.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case9.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case9.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-compile.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-compile.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-compile.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-probe` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-probe](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-probe). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-summary.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/summary.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/summary.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-tclDictObj.o` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/tclDictObj.o](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/tclDictObj.o). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-tclObj.o` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/tclObj.o](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/tclObj.o). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-version-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version-compile.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-version-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version-compile.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-version-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version-compile.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-version-probe` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version-probe](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version-probe). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-version.c` (input): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version.c). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-version.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-version.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-version.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-Makefile.observer` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/Makefile.observer](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/Makefile.observer). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-archive-copy.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/archive-copy.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/archive-copy.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-case10.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case10.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case10.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-case10.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case10.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case10.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-case10.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case10.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case10.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case19.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case19.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case19.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case9.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case9.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case9.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-compile.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-compile.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-compile.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-probe` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-probe](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-probe). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-build-archive.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-archive.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-archive.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-build-archive.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-archive.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-archive.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-build-archive.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-archive.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-archive.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-build-dictionary.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-dictionary.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-dictionary.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-build-dictionary.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-dictionary.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-dictionary.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-build-dictionary.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-dictionary.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-dictionary.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-build-object.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-object.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-object.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-build-object.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-object.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-object.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-build-object.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-object.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-object.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-libtcl9.1.a` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/libtcl9.1.a](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/libtcl9.1.a). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-case10.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case10.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case10.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-case10.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case10.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case10.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-case10.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case10.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case10.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case19.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case19.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case19.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case9.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case9.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case9.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-compile.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-compile.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-compile.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-probe` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-probe](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-probe). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-summary.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/summary.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/summary.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-tclDictObj.o` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/tclDictObj.o](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/tclDictObj.o). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-tclObj.o` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/tclObj.o](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/tclObj.o). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-version-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version-compile.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-version-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version-compile.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-version-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version-compile.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-version-probe` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version-probe](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version-probe). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-version.c` (input): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version.c). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-version.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version.receipt.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-version.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version.stderr). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-version.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version.stdout). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-summary.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/summary.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/summary.json). Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-reviewed-case-processes.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/reviewed-case-processes.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/reviewed-case-processes.json). Original compact review of the eighteen exact case process windows; independently checked against full receipts/streams, not a new observation.
- `file-14` (input): [rust/tcl-vm/tests/data/native_dictionary_body/probe.c](../../../../rust/tcl-vm/tests/data/native_dictionary_body/probe.c). Exact retained input/program bytes; purpose is limited to this question.
- `file-6` (observation): [rust/tcl-vm/tests/data/native_dictionary_body/cases.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_body/cases.tsv). Exact retained capture/provenance artifact; purpose is limited to this question.
- `native_dict_with_write_error_lifetime405-request-sdk-8.6.18-unix-libtcl8.6.a` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/8.6.18/unix/libtcl8.6.a](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/8.6.18/unix/libtcl8.6.a). Exact required original SDK source/header/configured Makefile or archive used by the baseline/version/observer build.
- `native_dict_with_write_error_lifetime405-request-sdk-8.6.18-generic-tcl.h` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/8.6.18/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/8.6.18/generic/tcl.h). Exact required original SDK source/header/configured Makefile or archive used by the baseline/version/observer build.
- `native_dict_with_write_error_lifetime405-request-sdk-8.6.18-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/8.6.18/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/8.6.18/unix/Makefile). Exact required original SDK source/header/configured Makefile or archive used by the baseline/version/observer build.
- `native_dict_with_write_error_lifetime405-request-sdk-9.0.4-unix-libtcl9.0.a` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.0.4/unix/libtcl9.0.a](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.0.4/unix/libtcl9.0.a). Exact required original SDK source/header/configured Makefile or archive used by the baseline/version/observer build.
- `native_dict_with_write_error_lifetime405-request-sdk-9.0.4-generic-tcl.h` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.0.4/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.0.4/generic/tcl.h). Exact required original SDK source/header/configured Makefile or archive used by the baseline/version/observer build.
- `native_dict_with_write_error_lifetime405-request-sdk-9.0.4-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.0.4/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.0.4/unix/Makefile). Exact required original SDK source/header/configured Makefile or archive used by the baseline/version/observer build.
- `native_dict_with_write_error_lifetime405-request-sdk-9.1.0-unix-libtcl9.1.a` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.1.0/unix/libtcl9.1.a](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.1.0/unix/libtcl9.1.a). Exact required original SDK source/header/configured Makefile or archive used by the baseline/version/observer build.
- `native_dict_with_write_error_lifetime405-request-sdk-9.1.0-generic-tcl.h` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.1.0/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.1.0/generic/tcl.h). Exact required original SDK source/header/configured Makefile or archive used by the baseline/version/observer build.
- `native_dict_with_write_error_lifetime405-request-sdk-9.1.0-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.1.0/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.1.0/unix/Makefile). Exact required original SDK source/header/configured Makefile or archive used by the baseline/version/observer build.
- `native_dict_with_write_error_lifetime405-recorded-path-map` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/recorded-path-map.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/recorded-path-map.json). Publication path map of independently checked exact required inputs and captures.

## Source inspection

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/8.6.18/original-tclDictObj.c`, function `TclDictWithFinish original dictionary selection and allocation`, lines 3655–3764. Retained reference `native_dict_with_write_error_lifetime405-request-8.6.18-original-tclDictObj.c`.

```text
TclDictWithFinish(
    Tcl_Interp *interp,		/* Command interpreter in which variable
				 * exists. Used for state management, traces
				 * and error reporting. */
    Var *varPtr,		/* Reference to the variable holding the
				 * dictionary. */
    Var *arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Name of an array (if part2 is non-NULL) or
				 * the name of a variable. NULL if the 'index'
				 * parameter is >= 0 */
    Tcl_Obj *part2Ptr,		/* If non-NULL, gives the name of an element
				 * in the array part1. */
    int index,			/* Index into the local variable table of the
				 * variable, or -1. Only used when part1Ptr is
				 * NULL. */
    int pathc,			/* The number of elements in the path into the
				 * dictionary. */
    Tcl_Obj *const pathv[],	/* The elements of the path to the subdict. */
    Tcl_Obj *keysPtr)		/* List of keys to be synchronized. This is
				 * the result value from TclDictWithInit. */
{
    Tcl_Obj *dictPtr, *leafPtr, *valPtr;
    int i, allocdict, keyc;
    Tcl_Obj **keyv;

    /*
     * If the dictionary variable doesn't exist, drop everything silently.
     */

    dictPtr = TclPtrGetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    TCL_LEAVE_ERR_MSG, index);
    if (dictPtr == NULL) {
	return TCL_OK;
    }

    /*
     * Double-check that it is still a dictionary.
     */

    if (Tcl_DictObjSize(interp, dictPtr, &i) != TCL_OK) {
	return TCL_ERROR;
    }

    if (Tcl_IsShared(dictPtr)) {
	dictPtr = Tcl_DuplicateObj(dictPtr);
	allocdict = 1;
    } else {
	allocdict = 0;
    }

    if (pathc > 0) {
	/*
	 * Want to get to the dictionary which we will update; need to do
	 * prepare-for-update unsharing along the path *but* avoid generating
	 * an error on a non-extant path (we'll treat that the same as a
	 * non-extant variable. Luckily, the unsharing operation isn't
	 * deeply damaging if we don't go on to update; it's just less than
	 * perfectly efficient (but no memory should be leaked).
	 */

	leafPtr = TclTraceDictPath(interp, dictPtr, pathc, pathv,
		DICT_PATH_EXISTS | DICT_PATH_UPDATE);
	if (leafPtr == NULL) {
	    if (allocdict) {
		TclDecrRefCount(dictPtr);
	    }
	    return TCL_ERROR;
	}
	if (leafPtr == DICT_PATH_NON_EXISTENT) {
	    if (allocdict) {
		TclDecrRefCount(dictPtr);
	    }
	    return TCL_OK;
	}
    } else {
	leafPtr = dictPtr;
    }

    /*
     * Now process our updates on the leaf dictionary.
     */

    TclListObjGetElements(NULL, keysPtr, &keyc, &keyv);
    for (i=0 ; i<keyc ; i++) {
	valPtr = Tcl_ObjGetVar2(interp, keyv[i], NULL, 0);
	if (valPtr == NULL) {
	    Tcl_DictObjRemove(NULL, leafPtr, keyv[i]);
	} else if (leafPtr == valPtr) {
	    /*
	     * Someone is messing us around, trying to build a recursive
	     * structure. [Bug 1786481]
	     */

	    Tcl_DictObjPut(NULL, leafPtr, keyv[i], Tcl_DuplicateObj(valPtr));
	} else {
	    Tcl_DictObjPut(NULL, leafPtr, keyv[i], valPtr);
	}
    }

    /*
     * Ensure that none of the dictionaries in the chain still have a string
     * rep.
     */

    if (pathc > 0) {
	InvalidateDictChain(leafPtr);
    }


```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/8.6.18/original-tclDictObj.c`, function `TclDictWithFinish original failed writeback cleanup`, lines 3765–3785. Retained reference `native_dict_with_write_error_lifetime405-request-8.6.18-original-tclDictObj.c`.

```text
    /*
     * Write back the outermost dictionary to the variable.
     */

    if (TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    dictPtr, TCL_LEAVE_ERR_MSG, index) == NULL) {
	if (allocdict) {
	    TclDecrRefCount(dictPtr);
	}
	return TCL_ERROR;
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * TclInitDictCmd --
 *
 *	This function is create the "dict" Tcl command. See the user
 *	documentation for details on what it does, and TIP#111 for the formal

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/8.6.18/observed-tclDictObj.c`, function `TclDictWithFinish live pre-set refcount/allocdict and post-failure Var-only observer`, lines 3773–3792. Retained reference `native_dict_with_write_error_lifetime405-request-8.6.18-observed-tclDictObj.c`.

```text
    tclLspWithEntered++;
    tclLspWithBeforeRefcount = dictPtr->refCount;
    tclLspWithAllocated = allocdict;
    tclLspWithWatch = dictPtr;
    fprintf(stderr,"WITH_BEFORE_SET|refcount=%d|allocated=%d\n",tclLspWithBeforeRefcount,tclLspWithAllocated);
    if (TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    dictPtr, TCL_LEAVE_ERR_MSG, index) == NULL) {
        tclLspWithFailed++;
        tclLspWithSameCell = varPtr->value.objPtr == dictPtr;
        tclLspWithDefinedScalar = TclIsVarScalar(varPtr) && !TclIsVarUndefined(varPtr);
        fprintf(stderr,"WITH_SET_ERROR|same_cell=%d|defined_scalar=%d|final_free=%d\n",tclLspWithSameCell,tclLspWithDefinedScalar,tclLspWithFinalFree);
	if (allocdict) {
	    TclDecrRefCount(dictPtr);
	}
	return TCL_ERROR;
    }
    return TCL_OK;
}

/*

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/8.6.18/observed-tclObj.c`, function `TclFreeObj watched free-entry integer counter; debug/non-debug branch`, lines 1295–1308. Retained reference `native_dict_with_write_error_lifetime405-request-8.6.18-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)	/* The object to be freed. */
{
    if (objPtr == tclLspWithWatch) {
        tclLspWithFinalFree++;
        tclLspWithWatch = NULL;
        fprintf(stderr,"WITH_FINAL_FREE|count=%d\n",tclLspWithFinalFree);
    }
    const Tcl_ObjType *typePtr = objPtr->typePtr;

    /*
     * This macro declares a variable, so must come here...
     */

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/8.6.18/observed-tclObj.c`, function `TclFreeObj watched free-entry integer counter; debug/non-debug branch`, lines 1425–1438. Retained reference `native_dict_with_write_error_lifetime405-request-8.6.18-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)	/* The object to be freed. */
{
    if (objPtr == tclLspWithWatch) {
        tclLspWithFinalFree++;
        tclLspWithWatch = NULL;
        fprintf(stderr,"WITH_FINAL_FREE|count=%d\n",tclLspWithFinalFree);
    }
    /*
     * Invalidate the string rep first so we can use the bytes value for our
     * pointer chain, and signal an obj deletion (as opposed to shimmering)
     * with 'length == -1'.
     */

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/observed-probe.c`, function `Unchanged source evaluation with integer-only body-return report before public report/source release/teardown`, lines 124–124. Retained reference `native_dict_with_write_error_lifetime405-request-observed-probe.c`.

```text
static void probe_bytes(const char*id,const char*src,int source_len){caseid=id;pathid="fixed-grammar";active=create();Obj*o=NEW(active,src,source_len);INC(o);int code=SCRIPT(active,o);fprintf(stderr,"WITH_BODY_RETURN|entered=%d|before_refcount=%d|allocated=%d|failed=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n",tclLspWithEntered,tclLspWithBeforeRefcount,tclLspWithAllocated,tclLspWithFailed,tclLspWithSameCell,tclLspWithDefinedScalar,tclLspWithFinalFree);report("native-result",code);DEC(active,o);DESTROY(active);}

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.0.4/original-tclDictObj.c`, function `TclDictWithFinish original dictionary selection and allocation`, lines 3842–3951. Retained reference `native_dict_with_write_error_lifetime405-request-9.0.4-original-tclDictObj.c`.

```text
TclDictWithFinish(
    Tcl_Interp *interp,		/* Command interpreter in which variable
				 * exists. Used for state management, traces
				 * and error reporting. */
    Var *varPtr,		/* Reference to the variable holding the
				 * dictionary. */
    Var *arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Name of an array (if part2 is non-NULL) or
				 * the name of a variable. NULL if the 'index'
				 * parameter is >= 0 */
    Tcl_Obj *part2Ptr,		/* If non-NULL, gives the name of an element
				 * in the array part1. */
    int index,			/* Index into the local variable table of the
				 * variable, or -1. Only used when part1Ptr is
				 * NULL. */
    int pathc,			/* The number of elements in the path into the
				 * dictionary. */
    Tcl_Obj *const pathv[],	/* The elements of the path to the subdict. */
    Tcl_Obj *keysPtr)		/* List of keys to be synchronized. This is
				 * the result value from TclDictWithInit. */
{
    Tcl_Obj *dictPtr, *leafPtr, *valPtr;
    Tcl_Size i, allocdict, keyc;
    Tcl_Obj **keyv;

    /*
     * If the dictionary variable doesn't exist, drop everything silently.
     */

    dictPtr = TclPtrGetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    TCL_LEAVE_ERR_MSG, index);
    if (dictPtr == NULL) {
	return TCL_OK;
    }

    /*
     * Double-check that it is still a dictionary.
     */

    if (Tcl_DictObjSize(interp, dictPtr, &i) != TCL_OK) {
	return TCL_ERROR;
    }

    if (Tcl_IsShared(dictPtr)) {
	dictPtr = Tcl_DuplicateObj(dictPtr);
	allocdict = 1;
    } else {
	allocdict = 0;
    }

    if (pathc > 0) {
	/*
	 * Want to get to the dictionary which we will update; need to do
	 * prepare-for-update unsharing along the path *but* avoid generating
	 * an error on a non-extant path (we'll treat that the same as a
	 * non-extant variable. Luckily, the unsharing operation isn't
	 * deeply damaging if we don't go on to update; it's just less than
	 * perfectly efficient (but no memory should be leaked).
	 */

	leafPtr = TclTraceDictPath(interp, dictPtr, pathc, pathv,
		DICT_PATH_EXISTS | DICT_PATH_UPDATE);
	if (leafPtr == NULL) {
	    if (allocdict) {
		TclDecrRefCount(dictPtr);
	    }
	    return TCL_ERROR;
	}
	if (leafPtr == DICT_PATH_NON_EXISTENT) {
	    if (allocdict) {
		TclDecrRefCount(dictPtr);
	    }
	    return TCL_OK;
	}
    } else {
	leafPtr = dictPtr;
    }

    /*
     * Now process our updates on the leaf dictionary.
     */

    TclListObjGetElements(NULL, keysPtr, &keyc, &keyv);
    for (i=0 ; i<keyc ; i++) {
	valPtr = Tcl_ObjGetVar2(interp, keyv[i], NULL, 0);
	if (valPtr == NULL) {
	    Tcl_DictObjRemove(NULL, leafPtr, keyv[i]);
	} else if (leafPtr == valPtr) {
	    /*
	     * Someone is messing us around, trying to build a recursive
	     * structure. [Bug 1786481]
	     */

	    Tcl_DictObjPut(NULL, leafPtr, keyv[i], Tcl_DuplicateObj(valPtr));
	} else {
	    Tcl_DictObjPut(NULL, leafPtr, keyv[i], valPtr);
	}
    }

    /*
     * Ensure that none of the dictionaries in the chain still have a string
     * rep.
     */

    if (pathc > 0) {
	InvalidateDictChain(leafPtr);
    }


```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.0.4/original-tclDictObj.c`, function `TclDictWithFinish original failed writeback cleanup`, lines 3952–3972. Retained reference `native_dict_with_write_error_lifetime405-request-9.0.4-original-tclDictObj.c`.

```text
    /*
     * Write back the outermost dictionary to the variable.
     */

    if (TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    dictPtr, TCL_LEAVE_ERR_MSG, index) == NULL) {
	if (allocdict) {
	    TclDecrRefCount(dictPtr);
	}
	return TCL_ERROR;
    }
    return TCL_OK;
}

/*
 *----------------------------------------------------------------------
 *
 * TclInitDictCmd --
 *
 *	This function is create the "dict" Tcl command. See the user
 *	documentation for details on what it does, and TIP#111 for the formal

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.0.4/observed-tclDictObj.c`, function `TclDictWithFinish live pre-set refcount/allocdict and post-failure Var-only observer`, lines 3960–3979. Retained reference `native_dict_with_write_error_lifetime405-request-9.0.4-observed-tclDictObj.c`.

```text
    tclLspWithEntered++;
    tclLspWithBeforeRefcount = dictPtr->refCount;
    tclLspWithAllocated = allocdict;
    tclLspWithWatch = dictPtr;
    fprintf(stderr,"WITH_BEFORE_SET|refcount=%d|allocated=%d\n",tclLspWithBeforeRefcount,tclLspWithAllocated);
    if (TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    dictPtr, TCL_LEAVE_ERR_MSG, index) == NULL) {
        tclLspWithFailed++;
        tclLspWithSameCell = varPtr->value.objPtr == dictPtr;
        tclLspWithDefinedScalar = TclIsVarScalar(varPtr) && !TclIsVarUndefined(varPtr);
        fprintf(stderr,"WITH_SET_ERROR|same_cell=%d|defined_scalar=%d|final_free=%d\n",tclLspWithSameCell,tclLspWithDefinedScalar,tclLspWithFinalFree);
	if (allocdict) {
	    TclDecrRefCount(dictPtr);
	}
	return TCL_ERROR;
    }
    return TCL_OK;
}

/*

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.0.4/observed-tclObj.c`, function `TclFreeObj watched free-entry integer counter; debug/non-debug branch`, lines 1271–1284. Retained reference `native_dict_with_write_error_lifetime405-request-9.0.4-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)	/* The object to be freed. */
{
    if (objPtr == tclLspWithWatch) {
        tclLspWithFinalFree++;
        tclLspWithWatch = NULL;
        fprintf(stderr,"WITH_FINAL_FREE|count=%d\n",tclLspWithFinalFree);
    }
    const Tcl_ObjType *typePtr = objPtr->typePtr;

    /*
     * This macro declares a variable, so must come here...
     */

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.0.4/observed-tclObj.c`, function `TclFreeObj watched free-entry integer counter; debug/non-debug branch`, lines 1401–1414. Retained reference `native_dict_with_write_error_lifetime405-request-9.0.4-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)	/* The object to be freed. */
{
    if (objPtr == tclLspWithWatch) {
        tclLspWithFinalFree++;
        tclLspWithWatch = NULL;
        fprintf(stderr,"WITH_FINAL_FREE|count=%d\n",tclLspWithFinalFree);
    }
    /*
     * Invalidate the string rep first so we can use the bytes value for our
     * pointer chain, and signal an obj deletion (as opposed to shimmering)
     * with 'length == -1'.
     */

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/observed-probe.c`, function `Unchanged source evaluation with integer-only body-return report before public report/source release/teardown`, lines 124–124. Retained reference `native_dict_with_write_error_lifetime405-request-observed-probe.c`.

```text
static void probe_bytes(const char*id,const char*src,int source_len){caseid=id;pathid="fixed-grammar";active=create();Obj*o=NEW(active,src,source_len);INC(o);int code=SCRIPT(active,o);fprintf(stderr,"WITH_BODY_RETURN|entered=%d|before_refcount=%d|allocated=%d|failed=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n",tclLspWithEntered,tclLspWithBeforeRefcount,tclLspWithAllocated,tclLspWithFailed,tclLspWithSameCell,tclLspWithDefinedScalar,tclLspWithFinalFree);report("native-result",code);DEC(active,o);DESTROY(active);}

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.1.0/original-tclDictObj.c`, function `TclDictWithFinish original dictionary selection and allocation`, lines 3845–3954. Retained reference `native_dict_with_write_error_lifetime405-request-9.1.0-original-tclDictObj.c`.

```text
TclDictWithFinish(
    Tcl_Interp *interp,		/* Command interpreter in which variable
				 * exists. Used for state management, traces
				 * and error reporting. */
    Var *varPtr,		/* Reference to the variable holding the
				 * dictionary. */
    Var *arrayPtr,		/* Reference to the array containing the
				 * variable, or NULL if the variable is a
				 * scalar. */
    Tcl_Obj *part1Ptr,		/* Name of an array (if part2 is non-NULL) or
				 * the name of a variable. NULL if the 'index'
				 * parameter is >= 0 */
    Tcl_Obj *part2Ptr,		/* If non-NULL, gives the name of an element
				 * in the array part1. */
    Tcl_Size index,		/* Index into the local variable table of the
				 * variable, or -1. Only used when part1Ptr is
				 * NULL. */
    Tcl_Size pathc,		/* The number of elements in the path into the
				 * dictionary. */
    Tcl_Obj *const pathv[],	/* The elements of the path to the subdict. */
    Tcl_Obj *keysPtr)		/* List of keys to be synchronized. This is
				 * the result value from TclDictWithInit. */
{
    Tcl_Obj *dictPtr, *leafPtr, *valPtr;
    Tcl_Size i, allocdict, keyc;
    Tcl_Obj **keyv;

    /*
     * If the dictionary variable doesn't exist, drop everything silently.
     */

    dictPtr = TclPtrGetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    TCL_LEAVE_ERR_MSG, index);
    if (dictPtr == NULL) {
	return TCL_OK;
    }

    /*
     * Double-check that it is still a dictionary.
     */

    if (Tcl_DictObjSize(interp, dictPtr, &i) != TCL_OK) {
	return TCL_ERROR;
    }

    if (Tcl_IsShared(dictPtr)) {
	dictPtr = Tcl_DuplicateObj(dictPtr);
	allocdict = 1;
    } else {
	allocdict = 0;
    }

    if (pathc > 0) {
	/*
	 * Want to get to the dictionary which we will update; need to do
	 * prepare-for-update unsharing along the path *but* avoid generating
	 * an error on a non-extant path (we'll treat that the same as a
	 * non-extant variable. Luckily, the unsharing operation isn't
	 * deeply damaging if we don't go on to update; it's just less than
	 * perfectly efficient (but no memory should be leaked).
	 */

	leafPtr = TclTraceDictPath(interp, dictPtr, pathc, pathv,
		DICT_PATH_EXISTS | DICT_PATH_UPDATE);
	if (leafPtr == NULL) {
	    if (allocdict) {
		TclDecrRefCount(dictPtr);
	    }
	    return TCL_ERROR;
	}
	if (leafPtr == DICT_PATH_NON_EXISTENT) {
	    if (allocdict) {
		TclDecrRefCount(dictPtr);
	    }
	    return TCL_OK;
	}
    } else {
	leafPtr = dictPtr;
    }

    /*
     * Now process our updates on the leaf dictionary.
     */

    TclListObjGetElements(NULL, keysPtr, &keyc, &keyv);
    for (i=0 ; i<keyc ; i++) {
	valPtr = Tcl_ObjGetVar2(interp, keyv[i], NULL, 0);
	if (valPtr == NULL) {
	    Tcl_DictObjRemove(NULL, leafPtr, keyv[i]);
	} else if (leafPtr == valPtr) {
	    /*
	     * Someone is messing us around, trying to build a recursive
	     * structure. [Bug 1786481]
	     */

	    Tcl_DictObjPut(NULL, leafPtr, keyv[i], Tcl_DuplicateObj(valPtr));
	} else {
	    Tcl_DictObjPut(NULL, leafPtr, keyv[i], valPtr);
	}
    }

    /*
     * Ensure that none of the dictionaries in the chain still have a string
     * rep.
     */

    if (pathc > 0) {
	InvalidateDictChain(leafPtr);
    }


```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.1.0/original-tclDictObj.c`, function `TclDictWithFinish original failed writeback cleanup`, lines 3955–3975. Retained reference `native_dict_with_write_error_lifetime405-request-9.1.0-original-tclDictObj.c`.

```text
    /*
     * Write back the outermost dictionary to the variable.
     */

    if (TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    dictPtr, TCL_LEAVE_ERR_MSG, index) == NULL) {
	if (allocdict) {
	    TclDecrRefCount(dictPtr);
	}
	return TCL_ERROR;
    }
    return TCL_OK;
}

/*
 * Local Variables:
 * mode: c
 * c-basic-offset: 4
 * fill-column: 78
 * End:
 */

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.1.0/observed-tclDictObj.c`, function `TclDictWithFinish live pre-set refcount/allocdict and post-failure Var-only observer`, lines 3963–3982. Retained reference `native_dict_with_write_error_lifetime405-request-9.1.0-observed-tclDictObj.c`.

```text
    tclLspWithEntered++;
    tclLspWithBeforeRefcount = dictPtr->refCount;
    tclLspWithAllocated = allocdict;
    tclLspWithWatch = dictPtr;
    fprintf(stderr,"WITH_BEFORE_SET|refcount=%d|allocated=%d\n",tclLspWithBeforeRefcount,tclLspWithAllocated);
    if (TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,
	    dictPtr, TCL_LEAVE_ERR_MSG, index) == NULL) {
        tclLspWithFailed++;
        tclLspWithSameCell = varPtr->value.objPtr == dictPtr;
        tclLspWithDefinedScalar = TclIsVarScalar(varPtr) && !TclIsVarUndefined(varPtr);
        fprintf(stderr,"WITH_SET_ERROR|same_cell=%d|defined_scalar=%d|final_free=%d\n",tclLspWithSameCell,tclLspWithDefinedScalar,tclLspWithFinalFree);
	if (allocdict) {
	    TclDecrRefCount(dictPtr);
	}
	return TCL_ERROR;
    }
    return TCL_OK;
}

/*

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.1.0/observed-tclObj.c`, function `TclFreeObj watched free-entry integer counter; debug/non-debug branch`, lines 1274–1287. Retained reference `native_dict_with_write_error_lifetime405-request-9.1.0-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)		/* The object to be freed. */
{
    if (objPtr == tclLspWithWatch) {
        tclLspWithFinalFree++;
        tclLspWithWatch = NULL;
        fprintf(stderr,"WITH_FINAL_FREE|count=%d\n",tclLspWithFinalFree);
    }
    const Tcl_ObjType *typePtr = objPtr->typePtr;

    /*
     * This macro declares a variable, so must come here...
     */

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.1.0/observed-tclObj.c`, function `TclFreeObj watched free-entry integer counter; debug/non-debug branch`, lines 1404–1417. Retained reference `native_dict_with_write_error_lifetime405-request-9.1.0-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)		/* The object to be freed. */
{
    if (objPtr == tclLspWithWatch) {
        tclLspWithFinalFree++;
        tclLspWithWatch = NULL;
        fprintf(stderr,"WITH_FINAL_FREE|count=%d\n",tclLspWithFinalFree);
    }
    /*
     * Invalidate the string rep first so we can use the bytes value for our
     * pointer chain, and signal an obj deletion (as opposed to shimmering)
     * with 'length == -1'.
     */

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/observed-probe.c`, function `Unchanged source evaluation with integer-only body-return report before public report/source release/teardown`, lines 124–124. Retained reference `native_dict_with_write_error_lifetime405-request-observed-probe.c`.

```text
static void probe_bytes(const char*id,const char*src,int source_len){caseid=id;pathid="fixed-grammar";active=create();Obj*o=NEW(active,src,source_len);INC(o);int code=SCRIPT(active,o);fprintf(stderr,"WITH_BODY_RETURN|entered=%d|before_refcount=%d|allocated=%d|failed=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n",tclLspWithEntered,tclLspWithBeforeRefcount,tclLspWithAllocated,tclLspWithFailed,tclLspWithSameCell,tclLspWithDefinedScalar,tclLspWithFinalFree);report("native-result",code);DEC(active,o);DESTROY(active);}

```


## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

- [rust/tcl-vm/src/cmd_dict.rs](../../../../rust/tcl-vm/src/cmd_dict.rs), `cmd_dict::native_rmw_fixture_tests::c86_dictionary_with_write_error_retains_independent_defined_host_storage`: Keep original With405 counters and raw streams separate from the software backend's explicit retained live dictionary storage policy; no old archive private-header equivalence.
- [rust/tcl-vm/src/cmd_dict.rs](../../../../rust/tcl-vm/src/cmd_dict.rs), `cmd_dict::native_rmw_fixture_tests::dictionary_body_writeback_compares_original_native_observation_windows` (linked): Retain all329 captured outer process-code references,326 complete results and three bounded post-free observations: independent C8.5/C8.6 update-error and C8.6 with-error edges. Later fields remain captured process output, without defined storage/read/completion/effect or old executable lifetime guarantees.
- [rust/tcl-vm/src/cmd_dict.rs](../../../../rust/tcl-vm/src/cmd_dict.rs), `cmd_dict::native_rmw_fixture_tests::c86_dictionary_with_write_error_retains_independent_defined_host_storage` (linked): Read exact405 C8.6 baseline/observer stdout equality and reached integer rows with plain/update controls, then independently check the VM's own genuine live BASE dictionary header/member under unchanged original source; no native freed-content reproduction.

These source bindings establish no executed assertion result; exact software outcomes belong to the independently pinned Rust validation receipts.

## Replay

The exact request/launcher, required input path mapping, original and observed whole sources/patches/templates/objects/archives/ELFs, command vectors/cwds, stdout/stderr digests and exits are retained. Absolute paths and unrecorded inherited compiler environment are not reconstructed. All receipts record timeout=false. Signal(-4) remains a process failure, not completion. Publication launches no compiler/provider/Rust test. Re-execution is a new observation and can change post-free fields, raw addresses and signal timing.

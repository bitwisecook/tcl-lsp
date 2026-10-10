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

## Exact evidence

- `native_dict_with_write_error_lifetime405-request-8.6.18-observed-tclDictObj.c` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/observed-tclDictObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/observed-tclDictObj.c). SHA-256 `d8178284dafdb563f9dd17f502095936fac7a8a601fc8c36e091cfd737c61df9`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-8.6.18-observed-tclObj.c` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/observed-tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/observed-tclObj.c). SHA-256 `ef6c4f2465f4937f8babf19171d4ec2dd8445ce8ac70963e14e2238638af0c15`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-8.6.18-original-tclDictObj.c` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/original-tclDictObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/original-tclDictObj.c). SHA-256 `532bd64ff27831be331be1028f08b541b6f206b927ff769431ea2d99326f2c77`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-8.6.18-original-tclObj.c` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/original-tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/original-tclObj.c). SHA-256 `7a7ef8ec85c74581129e8f3bef939e3191a432dcb021e0a16f1cf9971049f90d`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-8.6.18-tclDictObj.c.patch` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/tclDictObj.c.patch](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/tclDictObj.c.patch). SHA-256 `7f3d6cf9c8687be5b5c67e2dd7a645840a26251d95b874946910cdf22dde1c59`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-8.6.18-tclObj.c.patch` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/tclObj.c.patch](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/8.6.18/tclObj.c.patch). SHA-256 `ddf25011e8e40d4dad113ba2b13cea3a7acc18dc29b94ab370a5052b5ebf31f2`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.0.4-observed-tclDictObj.c` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/observed-tclDictObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/observed-tclDictObj.c). SHA-256 `f6ae80c55b265a1b94c2c210810c6d9ffaea71a78781c81ca7189485126f8c2b`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.0.4-observed-tclObj.c` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/observed-tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/observed-tclObj.c). SHA-256 `614f12bde244a2e20e5315b0da7920cf39978fe754f445f080a2146e912e9d52`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.0.4-original-tclDictObj.c` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/original-tclDictObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/original-tclDictObj.c). SHA-256 `7b9c1e3a88e956222dfb76d6642814ed08066556f72c02e0efa0eb541a77f33a`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.0.4-original-tclObj.c` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/original-tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/original-tclObj.c). SHA-256 `91a390bd24fbe71108108eeccf9955df0389f0f81dff24f303c32344469e8d4a`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.0.4-tclDictObj.c.patch` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/tclDictObj.c.patch](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/tclDictObj.c.patch). SHA-256 `2b5b2b04515e0173e3468bcb0037e0898f8287b1377c976eb5d1925c1d128e64`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.0.4-tclObj.c.patch` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/tclObj.c.patch](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.0.4/tclObj.c.patch). SHA-256 `0b4169f1703b9787891d10658414867d05efc7e59aa0d83fdfe6e5226e7b9230`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.1.0-observed-tclDictObj.c` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/observed-tclDictObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/observed-tclDictObj.c). SHA-256 `e43e41396f1a3738ce59e44fb4ca88b0b3dea97be7fa86f6b0a0a2d6ca24eeca`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.1.0-observed-tclObj.c` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/observed-tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/observed-tclObj.c). SHA-256 `8203ae396514737f252422c86563acac22fe34e4fa5b1dc642816b1cf1796791`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.1.0-original-tclDictObj.c` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/original-tclDictObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/original-tclDictObj.c). SHA-256 `01011866986e55b372fcca2f6d6f500917acc6a4e0ebe7b3993e858e5875df71`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.1.0-original-tclObj.c` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/original-tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/original-tclObj.c). SHA-256 `58c041dbf20bba3c8fef5782969c661d4d60a6ee0bc915f30e55c19cedb31976`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.1.0-tclDictObj.c.patch` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/tclDictObj.c.patch](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/tclDictObj.c.patch). SHA-256 `6f325f69306f9261c407c4771546c3cdbfd0853c9186f93d2eae04f278fa742d`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-9.1.0-tclObj.c.patch` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/tclObj.c.patch](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/9.1.0/tclObj.c.patch). SHA-256 `a0a702807e57d7b11ec99fa355deaea87fbca3d96e5ec13daf4602e7d3bdd587`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-Makefile.observer.template` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/Makefile.observer.template](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/Makefile.observer.template). SHA-256 `864e18f2bcd0cde86a5d15a333ad12f4eb7eeabb168bd38c25d4fc74131b713b`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-Makefile.preview` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/Makefile.preview](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/Makefile.preview). SHA-256 `69ac0546cb3a1b7bac422b6f87fac06f39d5698563d819d84f3677fd5b46d81f`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-capture.py` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/capture.py](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/capture.py). SHA-256 `84e419d0f705deb9aa3be5a055b68cedae0f70ecfb2ea6ac967bd43d2b8212a5`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-observed-probe.c` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/observed-probe.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/observed-probe.c). SHA-256 `41af7df7e377b7f12bc03df0bd8269c649738f85e2c97d4c958ff6abdeb0fdf1`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-original-probe.c` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/original-probe.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/original-probe.c). SHA-256 `e95451728352a915dfbc033719e60aabcab9ae4f53566088b583bbd4f12c6f3d`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-original-source.tcl` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/original-source.tcl](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/original-source.tcl). SHA-256 `f71aab88f0525c340914c59ebeed45be364d37efa6a395688fe220b95e97cf2c`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-request.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/request.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/request.json). SHA-256 `d1c0b7f06139c75df5a0b35ce14e909c26398f4ebd769a31a57c00f708cc0073`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-stage.py` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/stage.py](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/stage.py). SHA-256 `b1a53fd07905e8b3ccc72e9e7c3d4eecc47dfa4a9d02710bf24ba7102c60cbe1`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-request-version-template.c` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/version-template.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/version-template.c). SHA-256 `04f294f670b2404c8cffc5e5caddd7a1d1df58e842e6914c856a8f2b152d18f6`. Exact original request, unchanged original probe/guest, independent observer source copies and patches, version/build templates and launcher; no source text is rewritten.
- `native_dict_with_write_error_lifetime405-8.6.18-Makefile.observer` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/Makefile.observer](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/Makefile.observer). SHA-256 `5ebfbebf0bb757b6c20d1fea3aa4e72078b0345e49297cd33f948fb327ea0085`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-archive-copy.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/archive-copy.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/archive-copy.json). SHA-256 `e647b1fa17a2715616d714281c3da7f9ec1575fcdbf1efb4ee586d41cb4190cd`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-case10.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case10.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case10.receipt.json). SHA-256 `072bc44cb6b775310871ffcd34f931ede4a16a24a8cae48cd0f982c4702b9b9b`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-case10.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case10.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case10.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-case10.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case10.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case10.stdout). SHA-256 `a64cfa092bf0b7712a4c4dec86beab92a5272d0669b757cfa5a26f082c9fd29f`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case19.receipt.json). SHA-256 `904f52e23b5a835d805296793524175b554057aba4e3870eacf9db9b0dce43df`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case19.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case19.stdout). SHA-256 `9f2d927a77f3c38175d0b65e621c65d348b72c39f17bdb2e609560217fead8f4`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case9.receipt.json). SHA-256 `4804c11895c096adef12c33a91e169584631c8f71616f1cb94e8e7671792393b`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case9.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-case9.stdout). SHA-256 `b4b81dbe05b966e99bf8ccc05f9ea11bced7fa8673026bf59ca5b83bfdece4ac`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-compile.receipt.json). SHA-256 `96f1e9c1f55148f3430d0240cb557f311f2f9fa8271b79500f688f61ec44b5fc`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-baseline-probe` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-probe](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/baseline-probe). SHA-256 `19511affa8c25c04c5bbfbc582ed7eb8c3385c4b7c6002dcf15580d244498efb`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-build-archive.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-archive.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-archive.receipt.json). SHA-256 `cb004ca0708130ac023dac868d9a41b053f4ab4d6e6cb8f3244e8fc4b50f38d0`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-build-archive.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-archive.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-archive.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-build-archive.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-archive.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-archive.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-build-dictionary.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-dictionary.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-dictionary.receipt.json). SHA-256 `91abab8fcc740a6a9bcb461b291b1d586169a246273428d6f69b612673a8a361`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-build-dictionary.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-dictionary.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-dictionary.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-build-dictionary.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-dictionary.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-dictionary.stdout). SHA-256 `27da5be86ea51cc00e2da47cd6e509a4348a27e48c321b8571fd1d534ea901ab`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-build-object.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-object.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-object.receipt.json). SHA-256 `4878860d5eb9306bfb1034b58304451174689b7ba0b16495e207ab780ee2cf8b`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-build-object.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-object.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-object.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-build-object.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-object.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/build-object.stdout). SHA-256 `a9c2e414de09e979efa80f5ad677592c218db0a0ece6287faaa3e2b7c14fa780`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-libtcl8.6.a` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/libtcl8.6.a](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/libtcl8.6.a). SHA-256 `e7705daca0352994e13625981cc957c1e53940ea8a0ce99b7d5dea0c3e2d2c79`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-case10.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case10.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case10.receipt.json). SHA-256 `f52b2a4a738a672214f78d2272002b33eb2f44683861293b7c7a53e7f5dd60d3`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-case10.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case10.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case10.stderr). SHA-256 `e53c35b96910aa688e294e3d5adf47974e1698ed95af204c17d0d38aa23f8e0d`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-case10.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case10.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case10.stdout). SHA-256 `a64cfa092bf0b7712a4c4dec86beab92a5272d0669b757cfa5a26f082c9fd29f`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case19.receipt.json). SHA-256 `31e0aa3e36a03401e87a4a81ab0c4a1ebfc57021909f2470381cf8243b86e34a`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case19.stderr). SHA-256 `153bd88e7fc451be537549af2e979b975bcddcf8e0e1ccdced0dbc3b68a122cb`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case19.stdout). SHA-256 `9f2d927a77f3c38175d0b65e621c65d348b72c39f17bdb2e609560217fead8f4`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case9.receipt.json). SHA-256 `65f957b999d5f06d29a50369eda70cd94252c738ca2b57f4949f1aea759cfab4`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case9.stderr). SHA-256 `0b7823272ee3d935bb84b2b857e11a10eee63398f67b10cd6d19e2dbf5ecbd0c`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-case9.stdout). SHA-256 `b4b81dbe05b966e99bf8ccc05f9ea11bced7fa8673026bf59ca5b83bfdece4ac`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-compile.receipt.json). SHA-256 `3564a6909a37dfb25d081b1088859b708d35de3ec98db0cbe894b9c02db30cc1`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-observed-probe` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-probe](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/observed-probe). SHA-256 `c42e5a3b1423a31cde7c1e72a33af7bc57428f476566a30f2696c918f9dbe8a0`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-summary.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/summary.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/summary.json). SHA-256 `85c437259a98f19d37570f4ee80744a9e6b0158af7b88cdea1ae215b8297509a`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-tclDictObj.o` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/tclDictObj.o](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/tclDictObj.o). SHA-256 `df3d548c903116ff20b0a27fa7c35ae5b67eb5109a25ff93f5dff500ca107b62`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-tclObj.o` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/tclObj.o](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/tclObj.o). SHA-256 `7f97e7e02d9e39974fef1891bbac7e42b8368f0a3f04476e4e8e6faf1ea9fb81`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-version-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version-compile.receipt.json). SHA-256 `e931216e6e52e7210396e3ee4840ac149923d2071dbc281dd9ed735d6d65f858`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-version-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version-compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-version-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version-compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-version-probe` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version-probe](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version-probe). SHA-256 `9920ad8b0714b4f569db55b97a77d1fb3d9511ba91b323fc19d3335f69fed9be`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-version.c` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version.c). SHA-256 `04f294f670b2404c8cffc5e5caddd7a1d1df58e842e6914c856a8f2b152d18f6`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-version.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version.receipt.json). SHA-256 `da1126d6f797ac0b3f004f7ea5fb729f3d0ade3f1f0faa06b1cd883ee76d1a29`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-version.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-8.6.18-version.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/8.6.18/version.stdout). SHA-256 `14b8b5ab5e57aa72195601892ac554c63d31a187b1d2e50ef29d605f6a6acb06`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-Makefile.observer` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/Makefile.observer](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/Makefile.observer). SHA-256 `4744bc5b45e02429c9f7bf02d6cf15524932e6cd3188cfcccb88538b0938e18e`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-archive-copy.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/archive-copy.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/archive-copy.json). SHA-256 `97259923ba40342b0b90d67a48386b9ed4f66b0d8eb2f74557395046bf8d82f6`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-case10.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case10.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case10.receipt.json). SHA-256 `825d3fc37b6e6b81a041b28eb7e7b35dc6da4389167b9311e0206c47ef968322`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-case10.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case10.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case10.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-case10.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case10.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case10.stdout). SHA-256 `a64cfa092bf0b7712a4c4dec86beab92a5272d0669b757cfa5a26f082c9fd29f`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case19.receipt.json). SHA-256 `c822d69c916668e0ad63affdfcf03a1443268cc1f22b942664ddc492e412008a`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case19.stderr). SHA-256 `1a42b5a39cee934400a33668696c5040f77a2d7d50c8d021e70d6882d81dba10`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case19.stdout). SHA-256 `9f2d927a77f3c38175d0b65e621c65d348b72c39f17bdb2e609560217fead8f4`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case9.receipt.json). SHA-256 `e0c99d8e4f3b826a1768f344c0f6d9f9c5a47256c0e8a5b428fdf79225667134`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case9.stderr). SHA-256 `fc7245e7b8681e66c229ca6079cc73355afc96586254869a546a6678c2988187`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-case9.stdout). SHA-256 `b4b81dbe05b966e99bf8ccc05f9ea11bced7fa8673026bf59ca5b83bfdece4ac`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-compile.receipt.json). SHA-256 `fa1ea4c767448c8e83fa3206041d22a553242ead7fb17e5adf2b16ee834138fe`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-baseline-probe` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-probe](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/baseline-probe). SHA-256 `a5bfc924b3dbcf5f80b6066ae8561eb83e8fcfa5138b7aa8d523ca399a074204`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-build-archive.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-archive.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-archive.receipt.json). SHA-256 `52a249f4bf731ede75abf31905470aa9770e69896158c2786ec6c27045fba564`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-build-archive.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-archive.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-archive.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-build-archive.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-archive.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-archive.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-build-dictionary.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-dictionary.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-dictionary.receipt.json). SHA-256 `c04f01d1552e49173ea6d13d1545b4e5f3271e2a4404ea3e741131016e5dcd1c`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-build-dictionary.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-dictionary.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-dictionary.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-build-dictionary.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-dictionary.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-dictionary.stdout). SHA-256 `0dd5bc4e683da19db916a4978faaa86a7b0719c4b69765b1756275607f0d8a73`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-build-object.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-object.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-object.receipt.json). SHA-256 `283f70a203bdd95a5180e14dad1701611a7d5fa2136e110a43150bcee06d6b6e`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-build-object.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-object.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-object.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-build-object.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-object.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/build-object.stdout). SHA-256 `0939e30e53def99f53692c5dca5adab0c1a62dc6ba25bb7f05cdaa9d6c7767fb`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-libtcl9.0.a` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/libtcl9.0.a](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/libtcl9.0.a). SHA-256 `9e27978cbf0ac0b2a38da1bb1718ea7c76f452507b9acfb0bad20a0449956471`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-case10.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case10.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case10.receipt.json). SHA-256 `509a5b663b64e82e7b3ab23dafd6ed31e4d79e3b79e12ca0cff2468a718928a6`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-case10.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case10.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case10.stderr). SHA-256 `e53c35b96910aa688e294e3d5adf47974e1698ed95af204c17d0d38aa23f8e0d`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-case10.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case10.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case10.stdout). SHA-256 `a64cfa092bf0b7712a4c4dec86beab92a5272d0669b757cfa5a26f082c9fd29f`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case19.receipt.json). SHA-256 `bc63fc1c917abf6578f7390a50922fb0d3b29ab3092f18780f79c4b393f3bec7`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case19.stderr). SHA-256 `c204a102f196ca7747fbda9c012d6f8d9a813b0ab0c48230fcee61c5fa7740a8`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case19.stdout). SHA-256 `9f2d927a77f3c38175d0b65e621c65d348b72c39f17bdb2e609560217fead8f4`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case9.receipt.json). SHA-256 `49fc7287829c7a1dc893cd7d0f55be60cd247e0554f6affba6a90f8fb653f13f`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case9.stderr). SHA-256 `4a0beeae9c66d1c776616383763344a711ba65333ede51ea4c01aed7225d2e66`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-case9.stdout). SHA-256 `b4b81dbe05b966e99bf8ccc05f9ea11bced7fa8673026bf59ca5b83bfdece4ac`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-compile.receipt.json). SHA-256 `b27e6b5bb2d8afb735520d0ef1e0b21d4fbb0b956dc799e5ca76a00af3730501`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-observed-probe` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-probe](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/observed-probe). SHA-256 `f73b4529d84da6d56fb4474021117b326a705a46b1c38928af3abcb31b35cd68`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-summary.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/summary.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/summary.json). SHA-256 `73b05e512a2400bbae1aa34bb2b43291443c9aa8b8a78028d11a8e1c7e47a052`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-tclDictObj.o` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/tclDictObj.o](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/tclDictObj.o). SHA-256 `a45445083c55e5158180febfc7911c7f6deb44101f8ceffd427cad07043e0186`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-tclObj.o` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/tclObj.o](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/tclObj.o). SHA-256 `8086f70adde3355349dc1d568062d095880ee3c319fc4130260731e69f774a86`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-version-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version-compile.receipt.json). SHA-256 `b06aa49112cd71b8995e63c3017c9a86172e81d3cbcdfed0c9b62f09733687e0`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-version-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version-compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-version-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version-compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-version-probe` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version-probe](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version-probe). SHA-256 `5c84b380e27cc8eb663e0c81f865738bf6f10ce4ab5787a01343d0e5e8a78e22`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-version.c` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version.c). SHA-256 `04f294f670b2404c8cffc5e5caddd7a1d1df58e842e6914c856a8f2b152d18f6`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-version.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version.receipt.json). SHA-256 `e0c13c9456c1a8b17bfe01f5c8e8b6aa56695425139d87b4de05b5b48b4a7da2`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-version.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.0.4-version.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.0.4/version.stdout). SHA-256 `8daa9d62bc1b051c17a0fdd4a5a32675e98792c2e5006680ab98bef740476050`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-Makefile.observer` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/Makefile.observer](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/Makefile.observer). SHA-256 `4ea748fbef1b984164fd88f5405d1d8e8a5ee24e5feaada5b503fb0fa996c696`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-archive-copy.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/archive-copy.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/archive-copy.json). SHA-256 `18853ed3c42338e23d81a6a47d602692e3a733b3d977bb7ee8f35ae8406c49eb`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-case10.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case10.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case10.receipt.json). SHA-256 `01bddc4b653116e23b5054b4cd43c14113f09dcc092238f92b7604b273c021a0`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-case10.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case10.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case10.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-case10.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case10.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case10.stdout). SHA-256 `a64cfa092bf0b7712a4c4dec86beab92a5272d0669b757cfa5a26f082c9fd29f`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case19.receipt.json). SHA-256 `bb33655e892b0e9b69396826c5bfbf6787570a7dc5649950aaf258caeb645e4b`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case19.stderr). SHA-256 `f41f9c7119cd057b7c9a5af8711061b072cfc95e4350c9cc951130cb866e76a2`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case19.stdout). SHA-256 `9f2d927a77f3c38175d0b65e621c65d348b72c39f17bdb2e609560217fead8f4`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case9.receipt.json). SHA-256 `431e87c4ede19c78955d493312acbb37c743419207c864224b501bda751a1b04`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case9.stderr). SHA-256 `382859aaa548a246df28657f82f86c272bd41a5738699614dc7f7defc214b91d`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-case9.stdout). SHA-256 `b4b81dbe05b966e99bf8ccc05f9ea11bced7fa8673026bf59ca5b83bfdece4ac`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-compile.receipt.json). SHA-256 `64b9db374f0e5f0c744a4a503fe8522a57212ee5a6b3bc5c3df3e467dac5da4e`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-baseline-probe` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-probe](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/baseline-probe). SHA-256 `35848cfce4b263b4804255f220f21a03eacff0fc3cab49d3565e7639c6a20427`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-build-archive.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-archive.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-archive.receipt.json). SHA-256 `ae5e12ccfa84e8ebfc28a094ecfb1b9503e3792f6d6157af354c5652892f3c49`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-build-archive.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-archive.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-archive.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-build-archive.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-archive.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-archive.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-build-dictionary.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-dictionary.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-dictionary.receipt.json). SHA-256 `2eeec6bec842c385132ad508b04a4d830de3ef187f1b903e60f60a23361a9e8d`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-build-dictionary.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-dictionary.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-dictionary.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-build-dictionary.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-dictionary.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-dictionary.stdout). SHA-256 `590acfeb9ab3e1c6c77934323d5bd697c90d0b79f2968cbbb68d4eda2b18db9f`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-build-object.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-object.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-object.receipt.json). SHA-256 `408c173b2c986ba88839212eef46c42054186bcb8c07661069f5fd2f24bdcec0`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-build-object.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-object.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-object.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-build-object.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-object.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/build-object.stdout). SHA-256 `7913780fde73163d4d3cfc16f4f5d891d4bb35a659670ccc79244edecb8a9ef9`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-libtcl9.1.a` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/libtcl9.1.a](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/libtcl9.1.a). SHA-256 `89f6c90d95d5e55c9d38710a4b554794727bceaa72f9f41573639392ede575bb`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-case10.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case10.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case10.receipt.json). SHA-256 `8562ace575d54b2f8cb27b34622280da5b1bfa186e80a2cd2d7380ece96775a6`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-case10.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case10.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case10.stderr). SHA-256 `e53c35b96910aa688e294e3d5adf47974e1698ed95af204c17d0d38aa23f8e0d`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-case10.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case10.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case10.stdout). SHA-256 `a64cfa092bf0b7712a4c4dec86beab92a5272d0669b757cfa5a26f082c9fd29f`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case19.receipt.json). SHA-256 `1baab0b9e5bf7768a811693398f3282b45e953914fadf1b6a7a0958fb7c1f188`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case19.stderr). SHA-256 `0ec245043ab5e1d18193e6f445a9099e99a92efd228f443fa3cc2cebe918ae53`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case19.stdout). SHA-256 `9f2d927a77f3c38175d0b65e621c65d348b72c39f17bdb2e609560217fead8f4`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case9.receipt.json). SHA-256 `80fe6016fab9ffc9a62b54556abf507e671f5f80127d3514ca7358b6b0e7df2a`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case9.stderr). SHA-256 `527b848f7feb2771ecbb38e4f295aad9c7c391b7c03fab64aaaf0ace6acc45ec`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-case9.stdout). SHA-256 `b4b81dbe05b966e99bf8ccc05f9ea11bced7fa8673026bf59ca5b83bfdece4ac`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-compile.receipt.json). SHA-256 `c3ad1dc532763f21b7e00849bb9cddcb8bfe58d4f5b86f6aa277e6f42f5776cf`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-observed-probe` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-probe](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/observed-probe). SHA-256 `5520beeb21b41e607841f07365796a9b04a32753f9598bd41cf734d7dcea4ccb`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-summary.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/summary.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/summary.json). SHA-256 `fe0668e7d136d0d5b529057cb20d6f197a1196b010213b5e9fbc1c879bd2cf76`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-tclDictObj.o` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/tclDictObj.o](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/tclDictObj.o). SHA-256 `ea2b2f7826edc9029a23b5f0e5ca18718596f35118a4ea1b4ba1459cb62f5af8`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-tclObj.o` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/tclObj.o](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/tclObj.o). SHA-256 `0c30414e55f4b05a87b6b94602abdff552053c33da982de0d9980858658f24c6`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-version-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version-compile.receipt.json). SHA-256 `783dd9b51b86ae5e6177af4d975c23bdd6645983fede7027435f7764dfd059f7`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-version-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version-compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-version-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version-compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-version-probe` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version-probe](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version-probe). SHA-256 `53a8bea6c448a744ec4fa784c3ad4e1557d23108bf5774f6a20d6063bb8ed634`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-version.c` (source): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version.c](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version.c). SHA-256 `04f294f670b2404c8cffc5e5caddd7a1d1df58e842e6914c856a8f2b152d18f6`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-version.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version.receipt.json). SHA-256 `b31d4d034deb758f6543c442d5403446fe9d02eeadf6db17cb53bb566777f53c`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-version.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version.stderr](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-9.1.0-version.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version.stdout](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/9.1.0/version.stdout). SHA-256 `e99a8295b6adcc82e54aa294d384cd4c613c91b7529c199b41589a8f1c1db84f`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-summary.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/summary.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/summary.json). SHA-256 `02d86e7f6deb2ae17122855c25dc6dc4778f78116a76ea6608e43987f17a4d26`. Complete actual command receipt, stdout/stderr, summary, version probe, original or observer executable, object and separate archive; signal diagnostics and raw addresses are preserved.
- `native_dict_with_write_error_lifetime405-reviewed-case-processes.json` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/reviewed-case-processes.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/reviewed-case-processes.json). SHA-256 `77e6e348843ffdb58d481146221786f1c1b30362e6ef5d2716235feffe4692e3`. Original compact review of the eighteen exact case process windows; independently checked against full receipts/streams, not a new observation.
- `file-14` (input): [rust/tcl-vm/tests/data/native_dictionary_body/probe.c](../../../../rust/tcl-vm/tests/data/native_dictionary_body/probe.c). SHA-256 `e95451728352a915dfbc033719e60aabcab9ae4f53566088b583bbd4f12c6f3d`. Exact retained input/program bytes; purpose is limited to this question.
- `file-6` (observation): [rust/tcl-vm/tests/data/native_dictionary_body/cases.tsv](../../../../rust/tcl-vm/tests/data/native_dictionary_body/cases.tsv). SHA-256 `a457008448a336a389ab7c049c44f787c8361714358c6dd802f312464f9340f0`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `native_dict_with_write_error_lifetime405-request-sdk-8.6.18-unix-libtcl8.6.a` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/8.6.18/unix/libtcl8.6.a](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/8.6.18/unix/libtcl8.6.a). SHA-256 `980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb`. Exact required original SDK source/header/configured Makefile or archive used by the baseline/version/observer build.
- `native_dict_with_write_error_lifetime405-request-sdk-8.6.18-generic-tcl.h` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/8.6.18/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/8.6.18/generic/tcl.h). SHA-256 `aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245`. Exact required original SDK source/header/configured Makefile or archive used by the baseline/version/observer build.
- `native_dict_with_write_error_lifetime405-request-sdk-8.6.18-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/8.6.18/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/8.6.18/unix/Makefile). SHA-256 `8afb8697cb70b90518876861086bdb43f6e31b5e96e6d8091ae7b1de33d90d7e`. Exact required original SDK source/header/configured Makefile or archive used by the baseline/version/observer build.
- `native_dict_with_write_error_lifetime405-request-sdk-9.0.4-unix-libtcl9.0.a` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.0.4/unix/libtcl9.0.a](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.0.4/unix/libtcl9.0.a). SHA-256 `dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4`. Exact required original SDK source/header/configured Makefile or archive used by the baseline/version/observer build.
- `native_dict_with_write_error_lifetime405-request-sdk-9.0.4-generic-tcl.h` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.0.4/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.0.4/generic/tcl.h). SHA-256 `eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a`. Exact required original SDK source/header/configured Makefile or archive used by the baseline/version/observer build.
- `native_dict_with_write_error_lifetime405-request-sdk-9.0.4-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.0.4/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.0.4/unix/Makefile). SHA-256 `69f1915c208d66c7e38e6871c7f51c8f7be7c2138b45a5361641f9e91854fea5`. Exact required original SDK source/header/configured Makefile or archive used by the baseline/version/observer build.
- `native_dict_with_write_error_lifetime405-request-sdk-9.1.0-unix-libtcl9.1.a` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.1.0/unix/libtcl9.1.a](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.1.0/unix/libtcl9.1.a). SHA-256 `513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db`. Exact required original SDK source/header/configured Makefile or archive used by the baseline/version/observer build.
- `native_dict_with_write_error_lifetime405-request-sdk-9.1.0-generic-tcl.h` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.1.0/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.1.0/generic/tcl.h). SHA-256 `30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950`. Exact required original SDK source/header/configured Makefile or archive used by the baseline/version/observer build.
- `native_dict_with_write_error_lifetime405-request-sdk-9.1.0-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.1.0/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/request/sdk/9.1.0/unix/Makefile). SHA-256 `c1ecfb5a77697f0dc6f1057f63ff7aabd75b444f62b5954480aff01ff0565ab1`. Exact required original SDK source/header/configured Makefile or archive used by the baseline/version/observer build.
- `native_dict_with_write_error_lifetime405-recorded-path-map` (provider): [rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/recorded-path-map.json](../../../../rust/tcl-registry/tests/data/native_dict_with_write_error_lifetime405/recorded-path-map.json). SHA-256 `937fd1832d3d8c5d85278a51a59cb91abf13ebd230727a1d38cf12cc7ed1d7c5`. Publication path map of independently checked exact required inputs and captures.

## Source inspection

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/8.6.18/original-tclDictObj.c`, function `TclDictWithFinish original dictionary selection and allocation`, lines 3655–3764. Full-source SHA-256 `532bd64ff27831be331be1028f08b541b6f206b927ff769431ea2d99326f2c77`; snippet SHA-256 `e64bf810c9a3c1c409f9228777ef30089523cb265c86b9e8357b301b4196be5d`; retained evidence `native_dict_with_write_error_lifetime405-request-8.6.18-original-tclDictObj.c`.

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

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/8.6.18/original-tclDictObj.c`, function `TclDictWithFinish original failed writeback cleanup`, lines 3765–3785. Full-source SHA-256 `532bd64ff27831be331be1028f08b541b6f206b927ff769431ea2d99326f2c77`; snippet SHA-256 `c1407cae4565d6f42cbf4336836b0d93327ec7e498e760ac2851c56e8df3d1d7`; retained evidence `native_dict_with_write_error_lifetime405-request-8.6.18-original-tclDictObj.c`.

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

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/8.6.18/observed-tclDictObj.c`, function `TclDictWithFinish live pre-set refcount/allocdict and post-failure Var-only observer`, lines 3773–3792. Full-source SHA-256 `d8178284dafdb563f9dd17f502095936fac7a8a601fc8c36e091cfd737c61df9`; snippet SHA-256 `26790e7fdd214dbdb5aa9950ebe9790e85ea60c7aba5e881cb7c68ee099d4f61`; retained evidence `native_dict_with_write_error_lifetime405-request-8.6.18-observed-tclDictObj.c`.

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

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/8.6.18/observed-tclObj.c`, function `TclFreeObj watched free-entry integer counter; debug/non-debug branch`, lines 1295–1308. Full-source SHA-256 `ef6c4f2465f4937f8babf19171d4ec2dd8445ce8ac70963e14e2238638af0c15`; snippet SHA-256 `fe8637c6f20ddd776f10d9b43d77f388edfc5ccf5aa637cb66d16ddb9a39e1c5`; retained evidence `native_dict_with_write_error_lifetime405-request-8.6.18-observed-tclObj.c`.

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

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/8.6.18/observed-tclObj.c`, function `TclFreeObj watched free-entry integer counter; debug/non-debug branch`, lines 1425–1438. Full-source SHA-256 `ef6c4f2465f4937f8babf19171d4ec2dd8445ce8ac70963e14e2238638af0c15`; snippet SHA-256 `0152d9dd2ad74e6e4a1486d362ae51373a785a3e55081582428779b3f79b32d5`; retained evidence `native_dict_with_write_error_lifetime405-request-8.6.18-observed-tclObj.c`.

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

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/observed-probe.c`, function `Unchanged source evaluation with integer-only body-return report before public report/source release/teardown`, lines 124–124. Full-source SHA-256 `41af7df7e377b7f12bc03df0bd8269c649738f85e2c97d4c958ff6abdeb0fdf1`; snippet SHA-256 `2e32a6274b20362681aef7744d1b36b8d7a31cfd63f694e495bf0a10a1c25190`; retained evidence `native_dict_with_write_error_lifetime405-request-observed-probe.c`.

```text
static void probe_bytes(const char*id,const char*src,int source_len){caseid=id;pathid="fixed-grammar";active=create();Obj*o=NEW(active,src,source_len);INC(o);int code=SCRIPT(active,o);fprintf(stderr,"WITH_BODY_RETURN|entered=%d|before_refcount=%d|allocated=%d|failed=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n",tclLspWithEntered,tclLspWithBeforeRefcount,tclLspWithAllocated,tclLspWithFailed,tclLspWithSameCell,tclLspWithDefinedScalar,tclLspWithFinalFree);report("native-result",code);DEC(active,o);DESTROY(active);}

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.0.4/original-tclDictObj.c`, function `TclDictWithFinish original dictionary selection and allocation`, lines 3842–3951. Full-source SHA-256 `7b9c1e3a88e956222dfb76d6642814ed08066556f72c02e0efa0eb541a77f33a`; snippet SHA-256 `5b204b7d9f2963b67e0128fdee31c77279796191bc16414692dbdf4857c4e043`; retained evidence `native_dict_with_write_error_lifetime405-request-9.0.4-original-tclDictObj.c`.

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

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.0.4/original-tclDictObj.c`, function `TclDictWithFinish original failed writeback cleanup`, lines 3952–3972. Full-source SHA-256 `7b9c1e3a88e956222dfb76d6642814ed08066556f72c02e0efa0eb541a77f33a`; snippet SHA-256 `c1407cae4565d6f42cbf4336836b0d93327ec7e498e760ac2851c56e8df3d1d7`; retained evidence `native_dict_with_write_error_lifetime405-request-9.0.4-original-tclDictObj.c`.

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

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.0.4/observed-tclDictObj.c`, function `TclDictWithFinish live pre-set refcount/allocdict and post-failure Var-only observer`, lines 3960–3979. Full-source SHA-256 `f6ae80c55b265a1b94c2c210810c6d9ffaea71a78781c81ca7189485126f8c2b`; snippet SHA-256 `26790e7fdd214dbdb5aa9950ebe9790e85ea60c7aba5e881cb7c68ee099d4f61`; retained evidence `native_dict_with_write_error_lifetime405-request-9.0.4-observed-tclDictObj.c`.

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

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.0.4/observed-tclObj.c`, function `TclFreeObj watched free-entry integer counter; debug/non-debug branch`, lines 1271–1284. Full-source SHA-256 `614f12bde244a2e20e5315b0da7920cf39978fe754f445f080a2146e912e9d52`; snippet SHA-256 `fe8637c6f20ddd776f10d9b43d77f388edfc5ccf5aa637cb66d16ddb9a39e1c5`; retained evidence `native_dict_with_write_error_lifetime405-request-9.0.4-observed-tclObj.c`.

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

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.0.4/observed-tclObj.c`, function `TclFreeObj watched free-entry integer counter; debug/non-debug branch`, lines 1401–1414. Full-source SHA-256 `614f12bde244a2e20e5315b0da7920cf39978fe754f445f080a2146e912e9d52`; snippet SHA-256 `0152d9dd2ad74e6e4a1486d362ae51373a785a3e55081582428779b3f79b32d5`; retained evidence `native_dict_with_write_error_lifetime405-request-9.0.4-observed-tclObj.c`.

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

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/observed-probe.c`, function `Unchanged source evaluation with integer-only body-return report before public report/source release/teardown`, lines 124–124. Full-source SHA-256 `41af7df7e377b7f12bc03df0bd8269c649738f85e2c97d4c958ff6abdeb0fdf1`; snippet SHA-256 `2e32a6274b20362681aef7744d1b36b8d7a31cfd63f694e495bf0a10a1c25190`; retained evidence `native_dict_with_write_error_lifetime405-request-observed-probe.c`.

```text
static void probe_bytes(const char*id,const char*src,int source_len){caseid=id;pathid="fixed-grammar";active=create();Obj*o=NEW(active,src,source_len);INC(o);int code=SCRIPT(active,o);fprintf(stderr,"WITH_BODY_RETURN|entered=%d|before_refcount=%d|allocated=%d|failed=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n",tclLspWithEntered,tclLspWithBeforeRefcount,tclLspWithAllocated,tclLspWithFailed,tclLspWithSameCell,tclLspWithDefinedScalar,tclLspWithFinalFree);report("native-result",code);DEC(active,o);DESTROY(active);}

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.1.0/original-tclDictObj.c`, function `TclDictWithFinish original dictionary selection and allocation`, lines 3845–3954. Full-source SHA-256 `01011866986e55b372fcca2f6d6f500917acc6a4e0ebe7b3993e858e5875df71`; snippet SHA-256 `428bcba82275099003822c38c7ded5f495363c289aeb9e0be7b6bce94ff4af78`; retained evidence `native_dict_with_write_error_lifetime405-request-9.1.0-original-tclDictObj.c`.

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

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.1.0/original-tclDictObj.c`, function `TclDictWithFinish original failed writeback cleanup`, lines 3955–3975. Full-source SHA-256 `01011866986e55b372fcca2f6d6f500917acc6a4e0ebe7b3993e858e5875df71`; snippet SHA-256 `552dab18233bae3f8c126df37e534b0cec57bb83d0e0bf4fc5daaad34a1b5610`; retained evidence `native_dict_with_write_error_lifetime405-request-9.1.0-original-tclDictObj.c`.

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

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.1.0/observed-tclDictObj.c`, function `TclDictWithFinish live pre-set refcount/allocdict and post-failure Var-only observer`, lines 3963–3982. Full-source SHA-256 `e43e41396f1a3738ce59e44fb4ca88b0b3dea97be7fa86f6b0a0a2d6ca24eeca`; snippet SHA-256 `26790e7fdd214dbdb5aa9950ebe9790e85ea60c7aba5e881cb7c68ee099d4f61`; retained evidence `native_dict_with_write_error_lifetime405-request-9.1.0-observed-tclDictObj.c`.

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

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.1.0/observed-tclObj.c`, function `TclFreeObj watched free-entry integer counter; debug/non-debug branch`, lines 1274–1287. Full-source SHA-256 `8203ae396514737f252422c86563acac22fe34e4fa5b1dc642816b1cf1796791`; snippet SHA-256 `90da3d3af92d3a97b8b98870dd0ad527b6303757ecece8f954edbfccc44865bd`; retained evidence `native_dict_with_write_error_lifetime405-request-9.1.0-observed-tclObj.c`.

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

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/9.1.0/observed-tclObj.c`, function `TclFreeObj watched free-entry integer counter; debug/non-debug branch`, lines 1404–1417. Full-source SHA-256 `8203ae396514737f252422c86563acac22fe34e4fa5b1dc642816b1cf1796791`; snippet SHA-256 `f441068705dc8607abb3d92fea304321070898dcd7e911a5dca6f2654b4b8fb0`; retained evidence `native_dict_with_write_error_lifetime405-request-9.1.0-observed-tclObj.c`.

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

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-with-lifetime405/observed-probe.c`, function `Unchanged source evaluation with integer-only body-return report before public report/source release/teardown`, lines 124–124. Full-source SHA-256 `41af7df7e377b7f12bc03df0bd8269c649738f85e2c97d4c958ff6abdeb0fdf1`; snippet SHA-256 `2e32a6274b20362681aef7744d1b36b8d7a31cfd63f694e495bf0a10a1c25190`; retained evidence `native_dict_with_write_error_lifetime405-request-observed-probe.c`.

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

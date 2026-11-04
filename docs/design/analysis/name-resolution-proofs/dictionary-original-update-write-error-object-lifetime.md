# naming.dictionary.original-update-write-error-object-lifetime

Kind: `native-observation`

## Problem statement

Captured caller bytes cannot establish the defined lifetime of a dictionary after original compiled write-error cleanup. A bounded observation needs live pre-release selection checks and reached final-free evidence, while later signal termination and post-free reporting remain distinct outcomes.

## Question

For the unchanged original compiled-update write-error control on the separately pinned current C8.6/C9.0/C9.1 builds, does live cleanup select a defined scalar at refcount1 and reach its watched free entry, and what is reported before source-object release versus later process termination?

## Conclusion

The separate current C8.6.18 observer records errors=1, refcount_before_release=1, same_cell=1 and defined_scalar=1 immediately before the allocated compiled-update dictionary is decremented on the original write-error cleanup path; the watched TclFreeObj entry records final_free=1. Its unchanged case9 baseline and observer stdout are byte-identical and the processes exit0. Those stdout fields, including outer completion, diagnostics and caller read status, are observed after this final free and do not establish defined later storage or a general completion/read guarantee. Request398 final reporting occurs after source-object release and interpreter teardown: its C9.0.4/C9.1.0 case9 and independent case19 processes terminate by signal4 before that reporter. Request399 reports the same integer counters before those later releases, using exactly the398 observer archives: all three case9 rows report 1/1/1/1/1 at that boundary; C9.0/C9.1 later terminate by signal4. Plain case0 and independent with-write-error case19 counters are 0/-1/-1/-1/0; case19 still signals on C9.0/C9.1 and supplies no watched compiled-update lifetime conclusion. The51 exact closed commands comprise21 build/compile steps and30 process windows (27 case processes plus3 independent version queries);39 exit0 and12 terminate by signal4. No successful aggregate provider result, safe freed-content reproduction, other archive private identity, generic dict-update/dict-with lifetime, Native frame/compiler/admission, or Rust assertion pass follows. Current VM software coverage reads the exact398 C8.6 baseline/observer stdout and integer stderr, then checks its own retained dictionary header/member policy under the unchanged original source. That host policy is independent of native freed contents. The broader unchanged329-source comparator retains326 complete results plus three captured post-free C8.5/C8.6 update-error and C8.6 with-error diagnostic/read-status windows; no later outer/result/read field becomes a defined native storage/completion/effect guarantee. Software execution outcomes belong only to their exact pinned Rust source/image in the independent validation ledger.

## Scope

Two independent requests preserve the exact original guest SHA092c3aa2 and C API probe SHAe9545172 from the recorded compiled-update case9, plain0 and with-write-error19 controls. Tcl_CreateInterp and counted Tcl_EvalObjEx run without Tcl_Init; this is not a full-distribution initialisation test. Request398 copies the original current archives, separately compiles only instrumented tclExecute.o/tclObj.o with the configured source build, and retains baseline, observer and version executables. Live counters read the still-live refcount, compare the selected variable cell and check scalar-defined state before the unchanged decrement. The watched free-entry hook increments an integer and clears the watch before ordinary freeing; it does not dereference a freed header or alter Tcl ownership. Request399 only adds an earlier integer report before source-object release/teardown and reuses the exact398 observer archives. Complete source copies/patches, required SDK inputs, original/observer archives and objects, probes/executables, version rows, receipts and raw warning/counter/allocation-error stdout/stderr are retained separately. Later process output remains captured output, not a defined storage, completion, read, private identity or effect contract. C8.4, C8.5, Jim and BIG-IP are not tested by these requests; the separate existing C8.5 lifetime record is an independent observation.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Exact original compiled-update cleanup lifetime purpose.

No process or lifetime counter observation for this provider is executed by request398 or399.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Exact original compiled-update cleanup lifetime purpose.

No process or lifetime counter observation for this provider is executed by request398 or399.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original current archive 980630e3a7d37f82322fda1a7e45a85b86d8d905cb395a2aa0c083b3c7eaa8cb; separate398 observer archive 0fa455d1ae1e8c98ed5110e4d1953b9ed4289a500da92c05f11d47b4929419d1, reused byte-for-byte by399. Actual configured source/header/Makefile, object, archive, probe executable and command pins retained; compiler version and inherited environment unrecorded.. Channel: Unchanged original counted C API source through Tcl_CreateInterp/Tcl_EvalObjEx without Tcl_Init; separate live integer observer counters and complete public streams.. Dialect: Actual independently queried original C release; exact compiled-update cleanup lifetime purpose.

Both398 and399 case9 processes exit0; the live pre-release/free counters are1/1/1/1/1 and plain0/with19 counters0/-1/-1/-1/0. Exact398 baseline/observer case9 whole stdout equality is observed after final free; it is not a defined later byte/storage/completion guarantee.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original current archive dace08db925224497714ba86969fabb51c05a78f2bda22db6c05f795c3a70ae4; separate398 observer archive dfc648b4c5d1788c6caea8b24686f7e7bf97f9fe8d0de3835e5e9fd04efb6dd5, reused byte-for-byte by399. Actual configured source/header/Makefile, object, archive, probe executable and command pins retained; compiler version and inherited environment unrecorded.. Channel: Unchanged original counted C API source through Tcl_CreateInterp/Tcl_EvalObjEx without Tcl_Init; separate live integer observer counters and complete public streams.. Dialect: Actual independently queried original C release; exact compiled-update cleanup lifetime purpose.

The398 case9/with19 baseline and observer processes terminate by signal4 before the final reporter; no missing counter is inferred. The399 earlier case9 report records1/1/1/1/1, then the process terminates by signal4. Plain0 exits0 with0/-1/-1/-1/0; independent with19 reports0/-1/-1/-1/0 and later signals. These limited counters and raw outputs do not certify successful completion or defined post-free storage.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original current archive 513028850edf096856a29e4e16869aac91cba2ef75c3924aaaaeeb6e45e167db; separate398 observer archive a039185258e7518a8125d18cdb302cdb9ee6d9ba3a0ed9dbd53a2fad0734d6e4, reused byte-for-byte by399. Actual configured source/header/Makefile, object, archive, probe executable and command pins retained; compiler version and inherited environment unrecorded.. Channel: Unchanged original counted C API source through Tcl_CreateInterp/Tcl_EvalObjEx without Tcl_Init; separate live integer observer counters and complete public streams.. Dialect: Actual independently queried original C release; exact compiled-update cleanup lifetime purpose.

The398 case9/with19 baseline and observer processes terminate by signal4 before the final reporter; no missing counter is inferred. The399 earlier case9 report records1/1/1/1/1, then the process terminates by signal4. Plain0 exits0 with0/-1/-1/-1/0; independent with19 reports0/-1/-1/-1/0 and later signals. These limited counters and raw outputs do not certify successful completion or defined post-free storage.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Exact original compiled-update cleanup lifetime purpose.

No process or lifetime counter observation for this provider is executed by request398 or399.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Exact original compiled-update cleanup lifetime purpose.

No process or lifetime counter observation for this provider is executed by request398 or399.

## Historical references

- `native_dict_update_write_error_lifetime398-request-8.6.18-observed-tclExecute.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/observed-tclExecute.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/observed-tclExecute.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-8.6.18-observed-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/observed-tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/observed-tclObj.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-8.6.18-original-tclExecute.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/original-tclExecute.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/original-tclExecute.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-8.6.18-original-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/original-tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/original-tclObj.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-8.6.18-original-tclVar.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/original-tclVar.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/original-tclVar.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-8.6.18-tclExecute.c.patch` (input): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/tclExecute.c.patch](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/tclExecute.c.patch). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-8.6.18-tclObj.c.patch` (input): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/tclObj.c.patch](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/8.6.18/tclObj.c.patch). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-9.0.4-observed-tclExecute.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/observed-tclExecute.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/observed-tclExecute.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-9.0.4-observed-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/observed-tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/observed-tclObj.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-9.0.4-original-tclExecute.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/original-tclExecute.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/original-tclExecute.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-9.0.4-original-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/original-tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/original-tclObj.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-9.0.4-original-tclVar.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/original-tclVar.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/original-tclVar.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-9.0.4-tclExecute.c.patch` (input): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/tclExecute.c.patch](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/tclExecute.c.patch). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-9.0.4-tclObj.c.patch` (input): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/tclObj.c.patch](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.0.4/tclObj.c.patch). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-9.1.0-observed-tclExecute.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/observed-tclExecute.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/observed-tclExecute.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-9.1.0-observed-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/observed-tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/observed-tclObj.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-9.1.0-original-tclExecute.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/original-tclExecute.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/original-tclExecute.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-9.1.0-original-tclObj.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/original-tclObj.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/original-tclObj.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-9.1.0-original-tclVar.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/original-tclVar.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/original-tclVar.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-9.1.0-tclExecute.c.patch` (input): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/tclExecute.c.patch](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/tclExecute.c.patch). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-9.1.0-tclObj.c.patch` (input): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/tclObj.c.patch](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/9.1.0/tclObj.c.patch). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-capture.py` (input): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/capture.py](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/capture.py). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-observed-probe.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/observed-probe.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/observed-probe.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-original-probe.c` (input): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/original-probe.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/original-probe.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-original-source.tcl` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/original-source.tcl](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/original-source.tcl). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-request-request.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/request.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/request.json). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime398-8.6.18-Makefile.observer` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/Makefile.observer](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/Makefile.observer). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-archive-copy.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/archive-copy.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/archive-copy.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-baseline-case0.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case0.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case0.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-baseline-case0.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case0.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case0.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-baseline-case0.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case0.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case0.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-baseline-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case19.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-baseline-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case19.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-baseline-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case19.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-baseline-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case9.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-baseline-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case9.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-baseline-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case9.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-baseline-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-compile.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-baseline-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-compile.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-baseline-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-compile.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-baseline-probe` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-probe](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-probe). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-build-archive.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/build-archive.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/build-archive.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-build-archive.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/build-archive.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/build-archive.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-build-archive.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/build-archive.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/build-archive.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-build-execute.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/build-execute.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/build-execute.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-build-execute.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/build-execute.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/build-execute.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-build-execute.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/build-execute.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/build-execute.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-build-object.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/build-object.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/build-object.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-build-object.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/build-object.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/build-object.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-build-object.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/build-object.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/build-object.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-libtcl8.6.a` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/libtcl8.6.a](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/libtcl8.6.a). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-observed-case0.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case0.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case0.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-observed-case0.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case0.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case0.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-observed-case0.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case0.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case0.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-observed-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case19.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-observed-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case19.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-observed-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case19.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-observed-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case9.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-observed-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case9.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-observed-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case9.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-observed-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-compile.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-observed-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-compile.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-observed-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-compile.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-observed-probe` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-probe](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-probe). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-summary.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/summary.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/summary.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-tclExecute.o` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/tclExecute.o](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/tclExecute.o). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-tclObj.o` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/tclObj.o](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/tclObj.o). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-version-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/version-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/version-compile.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-version-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/version-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/version-compile.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-version-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/version-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/version-compile.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-version-probe` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/version-probe](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/version-probe). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-version.c` (input): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/version.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/version.c). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-version.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/version.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/version.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-version.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/version.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/version.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-8.6.18-version.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/version.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/version.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-Makefile.observer` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/Makefile.observer](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/Makefile.observer). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-archive-copy.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/archive-copy.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/archive-copy.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-baseline-case0.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-case0.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-case0.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-baseline-case0.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-case0.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-case0.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-baseline-case0.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-case0.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-case0.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-baseline-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-case19.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-baseline-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-case19.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-baseline-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-case19.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-baseline-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-case9.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-baseline-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-case9.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-baseline-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-case9.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-baseline-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-compile.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-baseline-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-compile.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-baseline-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-compile.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-baseline-probe` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-probe](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/baseline-probe). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-build-archive.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/build-archive.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/build-archive.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-build-archive.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/build-archive.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/build-archive.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-build-archive.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/build-archive.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/build-archive.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-build-execute.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/build-execute.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/build-execute.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-build-execute.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/build-execute.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/build-execute.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-build-execute.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/build-execute.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/build-execute.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-build-object.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/build-object.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/build-object.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-build-object.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/build-object.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/build-object.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-build-object.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/build-object.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/build-object.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-libtcl9.0.a` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/libtcl9.0.a](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/libtcl9.0.a). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-observed-case0.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-case0.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-case0.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-observed-case0.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-case0.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-case0.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-observed-case0.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-case0.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-case0.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-observed-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-case19.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-observed-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-case19.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-observed-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-case19.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-observed-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-case9.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-observed-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-case9.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-observed-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-case9.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-observed-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-compile.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-observed-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-compile.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-observed-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-compile.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-observed-probe` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-probe](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/observed-probe). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-summary.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/summary.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/summary.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-tclExecute.o` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/tclExecute.o](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/tclExecute.o). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-tclObj.o` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/tclObj.o](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/tclObj.o). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-version-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/version-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/version-compile.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-version-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/version-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/version-compile.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-version-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/version-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/version-compile.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-version-probe` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/version-probe](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/version-probe). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-version.c` (input): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/version.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/version.c). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-version.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/version.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/version.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-version.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/version.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/version.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.0.4-version.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/version.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.0.4/version.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-Makefile.observer` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/Makefile.observer](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/Makefile.observer). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-archive-copy.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/archive-copy.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/archive-copy.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-baseline-case0.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-case0.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-case0.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-baseline-case0.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-case0.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-case0.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-baseline-case0.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-case0.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-case0.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-baseline-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-case19.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-baseline-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-case19.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-baseline-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-case19.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-baseline-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-case9.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-baseline-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-case9.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-baseline-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-case9.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-baseline-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-compile.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-baseline-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-compile.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-baseline-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-compile.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-baseline-probe` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-probe](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/baseline-probe). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-build-archive.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/build-archive.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/build-archive.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-build-archive.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/build-archive.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/build-archive.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-build-archive.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/build-archive.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/build-archive.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-build-execute.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/build-execute.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/build-execute.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-build-execute.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/build-execute.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/build-execute.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-build-execute.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/build-execute.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/build-execute.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-build-object.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/build-object.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/build-object.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-build-object.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/build-object.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/build-object.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-build-object.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/build-object.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/build-object.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-libtcl9.1.a` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/libtcl9.1.a](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/libtcl9.1.a). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-observed-case0.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-case0.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-case0.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-observed-case0.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-case0.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-case0.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-observed-case0.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-case0.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-case0.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-observed-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-case19.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-observed-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-case19.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-observed-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-case19.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-observed-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-case9.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-observed-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-case9.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-observed-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-case9.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-observed-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-compile.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-observed-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-compile.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-observed-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-compile.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-observed-probe` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-probe](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/observed-probe). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-summary.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/summary.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/summary.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-tclExecute.o` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/tclExecute.o](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/tclExecute.o). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-tclObj.o` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/tclObj.o](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/tclObj.o). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-version-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/version-compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/version-compile.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-version-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/version-compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/version-compile.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-version-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/version-compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/version-compile.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-version-probe` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/version-probe](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/version-probe). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-version.c` (input): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/version.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/version.c). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-version.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/version.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/version.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-version.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/version.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/version.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-9.1.0-version.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/version.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/9.1.0/version.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-summary.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/summary.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/summary.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-request-capture.py` (input): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/request/capture.py](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/request/capture.py). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime399-request-observed-probe.c` (source-anchor): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/request/observed-probe.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/request/observed-probe.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime399-request-original-probe.c` (input): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/request/original-probe.c](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/request/original-probe.c). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime399-request-original-source.tcl` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/request/original-source.tcl](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/request/original-source.tcl). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime399-request-request.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/request/request.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/request/request.json). Exact original request, probe, counted guest, source copy or independent instrumentation patch; bytes are unchanged.
- `native_dict_update_write_error_lifetime399-8.6.18-case0.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/case0.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/case0.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-8.6.18-case0.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/case0.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/case0.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-8.6.18-case0.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/case0.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/case0.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-8.6.18-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/case19.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-8.6.18-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/case19.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-8.6.18-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/case19.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-8.6.18-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/case9.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-8.6.18-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/case9.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-8.6.18-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/case9.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-8.6.18-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/compile.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-8.6.18-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/compile.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-8.6.18-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/compile.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-8.6.18-observed-probe` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/observed-probe](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/8.6.18/observed-probe). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.0.4-case0.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/case0.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/case0.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.0.4-case0.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/case0.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/case0.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.0.4-case0.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/case0.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/case0.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.0.4-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/case19.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.0.4-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/case19.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.0.4-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/case19.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.0.4-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/case9.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.0.4-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/case9.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.0.4-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/case9.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.0.4-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/compile.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.0.4-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/compile.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.0.4-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/compile.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.0.4-observed-probe` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/observed-probe](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.0.4/observed-probe). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.1.0-case0.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/case0.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/case0.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.1.0-case0.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/case0.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/case0.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.1.0-case0.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/case0.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/case0.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.1.0-case19.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/case19.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/case19.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.1.0-case19.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/case19.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/case19.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.1.0-case19.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/case19.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/case19.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.1.0-case9.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/case9.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/case9.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.1.0-case9.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/case9.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/case9.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.1.0-case9.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/case9.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/case9.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.1.0-compile.receipt.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/compile.receipt.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/compile.receipt.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.1.0-compile.stderr` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/compile.stderr](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/compile.stderr). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.1.0-compile.stdout` (observation): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/compile.stdout](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/compile.stdout). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-9.1.0-observed-probe` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/observed-probe](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/9.1.0/observed-probe). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime399-summary.json` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/summary.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime399/summary.json). Complete actual capture file: command receipt, whole stream, summary, independently built object/archive or executable; no rows, signal diagnostics or addresses are normalised.
- `native_dict_update_write_error_lifetime398-request-sdk-8.6.18-generic-tcl.h` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/generic/tcl.h). Exact independently pinned original SDK header/configured Makefile/archive used by the actual baseline, version query or observer compilation.
- `native_dict_update_write_error_lifetime398-request-sdk-8.6.18-generic-tclInt.h` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/generic/tclInt.h](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/generic/tclInt.h). Exact independently pinned original SDK header/configured Makefile/archive used by the actual baseline, version query or observer compilation.
- `native_dict_update_write_error_lifetime398-request-sdk-8.6.18-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix/Makefile). Exact independently pinned original SDK header/configured Makefile/archive used by the actual baseline, version query or observer compilation.
- `native_dict_update_write_error_lifetime398-request-sdk-8.6.18-unix-libtcl8.6.a` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix/libtcl8.6.a](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/8.6.18/unix/libtcl8.6.a). Exact independently pinned original SDK header/configured Makefile/archive used by the actual baseline, version query or observer compilation.
- `native_dict_update_write_error_lifetime398-request-sdk-9.0.4-generic-tcl.h` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/generic/tcl.h). Exact independently pinned original SDK header/configured Makefile/archive used by the actual baseline, version query or observer compilation.
- `native_dict_update_write_error_lifetime398-request-sdk-9.0.4-generic-tclInt.h` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/generic/tclInt.h](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/generic/tclInt.h). Exact independently pinned original SDK header/configured Makefile/archive used by the actual baseline, version query or observer compilation.
- `native_dict_update_write_error_lifetime398-request-sdk-9.0.4-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix/Makefile). Exact independently pinned original SDK header/configured Makefile/archive used by the actual baseline, version query or observer compilation.
- `native_dict_update_write_error_lifetime398-request-sdk-9.0.4-unix-libtcl9.0.a` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix/libtcl9.0.a](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.0.4/unix/libtcl9.0.a). Exact independently pinned original SDK header/configured Makefile/archive used by the actual baseline, version query or observer compilation.
- `native_dict_update_write_error_lifetime398-request-sdk-9.1.0-generic-tcl.h` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/generic/tcl.h](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/generic/tcl.h). Exact independently pinned original SDK header/configured Makefile/archive used by the actual baseline, version query or observer compilation.
- `native_dict_update_write_error_lifetime398-request-sdk-9.1.0-generic-tclInt.h` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/generic/tclInt.h](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/generic/tclInt.h). Exact independently pinned original SDK header/configured Makefile/archive used by the actual baseline, version query or observer compilation.
- `native_dict_update_write_error_lifetime398-request-sdk-9.1.0-unix-Makefile` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix/Makefile](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix/Makefile). Exact independently pinned original SDK header/configured Makefile/archive used by the actual baseline, version query or observer compilation.
- `native_dict_update_write_error_lifetime398-request-sdk-9.1.0-unix-libtcl9.1.a` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix/libtcl9.1.a](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/sdk/9.1.0/unix/libtcl9.1.a). Exact independently pinned original SDK header/configured Makefile/archive used by the actual baseline, version query or observer compilation.
- `native_dict_update_write_error_lifetime398-recorded-path-map` (provider): [rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/recorded-path-map.json](../../../../rust/tcl-registry/tests/data/native_dict_update_write_error_lifetime398/recorded-path-map.json). Publication path map for all exact required inputs and captures; independently checked byte preservation, not a new process result.

## Source inspection

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/8.6.18/original-tclExecute.c`, function `TclExecuteByteCode::INST_DICT_UPDATE_END original error cleanup`, lines 7782–7796. Retained reference `native_dict_update_write_error_lifetime398-request-8.6.18-original-tclExecute.c`.

```text
	    DECACHE_STACK_INFO();
	    objResultPtr = TclPtrSetVarIdx(interp, varPtr, NULL, NULL, NULL,
		    dictPtr, TCL_LEAVE_ERR_MSG, opnd);
	    CACHE_STACK_INFO();
	    if (objResultPtr == NULL) {
		if (allocdict) {
		    TclDecrRefCount(dictPtr);
		}
		TRACE_ERROR(interp);
		goto gotError;
	    }
	}
	TRACE_APPEND(("written back\n"));
	NEXT_INST_F(9, 1, 0);


```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/8.6.18/observed-tclExecute.c`, function `TclExecuteByteCode::INST_DICT_UPDATE_END live pre-release observer`, lines 7791–7810. Retained reference `native_dict_update_write_error_lifetime398-request-8.6.18-observed-tclExecute.c`.

```text
	    objResultPtr = TclPtrSetVarIdx(interp, varPtr, NULL, NULL, NULL,
		    dictPtr, TCL_LEAVE_ERR_MSG, opnd);
	    CACHE_STACK_INFO();
	    if (objResultPtr == NULL) {
		if (allocdict) {
                    tclLspDictErrorObserved++;
                    tclLspDictErrorRefcount = dictPtr->refCount;
                    tclLspDictErrorSameCell = (varPtr->value.objPtr == dictPtr);
                    tclLspDictErrorDefinedScalar =
                        TclIsVarScalar(varPtr) && !TclIsVarUndefined(varPtr);
                    tclLspDictReleaseWatch = dictPtr;
		    TclDecrRefCount(dictPtr);
		}
		TRACE_ERROR(interp);
		goto gotError;
	    }
	}
	TRACE_APPEND(("written back\n"));
	NEXT_INST_F(9, 1, 0);


```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/8.6.18/observed-tclObj.c`, function `TclFreeObj watched entry (conditional debug/non-debug branches)`, lines 1295–1309. Retained reference `native_dict_update_write_error_lifetime398-request-8.6.18-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)	/* The object to be freed. */
{
    if (objPtr == tclLspDictReleaseWatch) {
        tclLspDictFinalFreeObserved++;
        tclLspDictReleaseWatch = NULL;
    }
    const Tcl_ObjType *typePtr = objPtr->typePtr;

    /*
     * This macro declares a variable, so must come here...
     */

    ObjInitDeletionContext(context);

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/8.6.18/observed-tclObj.c`, function `TclFreeObj watched entry (conditional debug/non-debug branches)`, lines 1424–1438. Retained reference `native_dict_update_write_error_lifetime398-request-8.6.18-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)	/* The object to be freed. */
{
    if (objPtr == tclLspDictReleaseWatch) {
        tclLspDictFinalFreeObserved++;
        tclLspDictReleaseWatch = NULL;
    }
    /*
     * Invalidate the string rep first so we can use the bytes value for our
     * pointer chain, and signal an obj deletion (as opposed to shimmering)
     * with 'length == -1'.
     */

    TclInvalidateStringRep(objPtr);

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/8.6.18/original-tclVar.c`, function `TclPtrSetVarIdx original replacement and write-trace failure`, lines 1980–2007. Retained reference `native_dict_update_write_error_lifetime398-request-8.6.18-original-tclVar.c`.

```text
	 * In this case we are replacing the value, so we don't need to do
	 * more than swap the objects.
	 */

	varPtr->value.objPtr = newValuePtr;
	Tcl_IncrRefCount(newValuePtr);		/* Var is another ref. */
	if (oldValuePtr != NULL) {
	    TclDecrRefCount(oldValuePtr);	/* Discard old value. */
	}
    }

    /*
     * Invoke any write traces for the variable.
     */

    if ((varPtr->flags & VAR_TRACED_WRITE)
	    || (arrayPtr && (arrayPtr->flags & VAR_TRACED_WRITE))) {
	if (TCL_ERROR == TclObjCallVarTraces(iPtr, arrayPtr, varPtr, part1Ptr,
		part2Ptr, (flags & (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY))
		| TCL_TRACE_WRITES, (flags & TCL_LEAVE_ERR_MSG), index)) {
	    goto cleanup;
	}
    }

    /*
     * Return the variable's value unless the variable was changed in some
     * gross way by a trace (e.g. it was unset and then recreated as an
     * array).

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/observed-probe.c`, function `Independent final counter report after source-object release and interpreter teardown`, lines 185–186. Retained reference `native_dict_update_write_error_lifetime398-request-observed-probe.c`.

```text
fprintf(stderr, "DICT_RELEASE_OBSERVER|errors=%d|refcount_before_release=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n", tclLspDictErrorObserved, tclLspDictErrorRefcount, tclLspDictErrorSameCell, tclLspDictErrorDefinedScalar, tclLspDictFinalFreeObserved);
return 0;}

```

tcl8.6 8.6.18, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime399/observed-probe.c`, function `Independent body-return integer report before source-object release and interpreter teardown`, lines 127–127. Retained reference `native_dict_update_write_error_lifetime399-request-observed-probe.c`.

```text
static void probe_bytes(const char*id,const char*src,int source_len){caseid=id;pathid="fixed-grammar";active=create();Obj*o=NEW(active,src,source_len);INC(o);int code=SCRIPT(active,o);report("native-result",code);fprintf(stderr, "DICT_RELEASE_BODY_RETURN|errors=%d|refcount_before_release=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n", tclLspDictErrorObserved, tclLspDictErrorRefcount, tclLspDictErrorSameCell, tclLspDictErrorDefinedScalar, tclLspDictFinalFreeObserved);DEC(active,o);DESTROY(active);}

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.0.4/original-tclExecute.c`, function `TclExecuteByteCode::INST_DICT_UPDATE_END original error cleanup`, lines 7295–7309. Retained reference `native_dict_update_write_error_lifetime398-request-9.0.4-original-tclExecute.c`.

```text
	    DECACHE_STACK_INFO();
	    objResultPtr = TclPtrSetVarIdx(interp, varPtr, NULL, NULL, NULL,
		    dictPtr, TCL_LEAVE_ERR_MSG, opnd);
	    CACHE_STACK_INFO();
	    if (objResultPtr == NULL) {
		if (allocdict) {
		    TclDecrRefCount(dictPtr);
		}
		TRACE_ERROR(interp);
		goto gotError;
	    }
	}
	TRACE_APPEND(("written back\n"));
	NEXT_INST_F(9, 1, 0);


```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.0.4/observed-tclExecute.c`, function `TclExecuteByteCode::INST_DICT_UPDATE_END live pre-release observer`, lines 7304–7323. Retained reference `native_dict_update_write_error_lifetime398-request-9.0.4-observed-tclExecute.c`.

```text
	    objResultPtr = TclPtrSetVarIdx(interp, varPtr, NULL, NULL, NULL,
		    dictPtr, TCL_LEAVE_ERR_MSG, opnd);
	    CACHE_STACK_INFO();
	    if (objResultPtr == NULL) {
		if (allocdict) {
                    tclLspDictErrorObserved++;
                    tclLspDictErrorRefcount = dictPtr->refCount;
                    tclLspDictErrorSameCell = (varPtr->value.objPtr == dictPtr);
                    tclLspDictErrorDefinedScalar =
                        TclIsVarScalar(varPtr) && !TclIsVarUndefined(varPtr);
                    tclLspDictReleaseWatch = dictPtr;
		    TclDecrRefCount(dictPtr);
		}
		TRACE_ERROR(interp);
		goto gotError;
	    }
	}
	TRACE_APPEND(("written back\n"));
	NEXT_INST_F(9, 1, 0);


```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.0.4/observed-tclObj.c`, function `TclFreeObj watched entry (conditional debug/non-debug branches)`, lines 1271–1285. Retained reference `native_dict_update_write_error_lifetime398-request-9.0.4-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)	/* The object to be freed. */
{
    if (objPtr == tclLspDictReleaseWatch) {
        tclLspDictFinalFreeObserved++;
        tclLspDictReleaseWatch = NULL;
    }
    const Tcl_ObjType *typePtr = objPtr->typePtr;

    /*
     * This macro declares a variable, so must come here...
     */

    ObjInitDeletionContext(context);

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.0.4/observed-tclObj.c`, function `TclFreeObj watched entry (conditional debug/non-debug branches)`, lines 1400–1414. Retained reference `native_dict_update_write_error_lifetime398-request-9.0.4-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)	/* The object to be freed. */
{
    if (objPtr == tclLspDictReleaseWatch) {
        tclLspDictFinalFreeObserved++;
        tclLspDictReleaseWatch = NULL;
    }
    /*
     * Invalidate the string rep first so we can use the bytes value for our
     * pointer chain, and signal an obj deletion (as opposed to shimmering)
     * with 'length == -1'.
     */

    TclInvalidateStringRep(objPtr);

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.0.4/original-tclVar.c`, function `TclPtrSetVarIdx original replacement and write-trace failure`, lines 2025–2052. Retained reference `native_dict_update_write_error_lifetime398-request-9.0.4-original-tclVar.c`.

```text
	 * In this case we are replacing the value, so we don't need to do
	 * more than swap the objects.
	 */

	varPtr->value.objPtr = newValuePtr;
	Tcl_IncrRefCount(newValuePtr);		/* Var is another ref. */
	if (oldValuePtr != NULL) {
	    TclDecrRefCount(oldValuePtr);	/* Discard old value. */
	}
    }

    /*
     * Invoke any write traces for the variable.
     */

    if ((varPtr->flags & VAR_TRACED_WRITE)
	    || (arrayPtr && (arrayPtr->flags & VAR_TRACED_WRITE))) {
	if (TCL_ERROR == TclObjCallVarTraces(iPtr, arrayPtr, varPtr, part1Ptr,
		part2Ptr, (flags & (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY))
		| TCL_TRACE_WRITES, (flags & TCL_LEAVE_ERR_MSG), index)) {
	    goto cleanup;
	}
    }

    /*
     * Return the variable's value unless the variable was changed in some
     * gross way by a trace (e.g. it was unset and then recreated as an
     * array).

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/observed-probe.c`, function `Independent final counter report after source-object release and interpreter teardown`, lines 185–186. Retained reference `native_dict_update_write_error_lifetime398-request-observed-probe.c`.

```text
fprintf(stderr, "DICT_RELEASE_OBSERVER|errors=%d|refcount_before_release=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n", tclLspDictErrorObserved, tclLspDictErrorRefcount, tclLspDictErrorSameCell, tclLspDictErrorDefinedScalar, tclLspDictFinalFreeObserved);
return 0;}

```

tcl9.0 9.0.4, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime399/observed-probe.c`, function `Independent body-return integer report before source-object release and interpreter teardown`, lines 127–127. Retained reference `native_dict_update_write_error_lifetime399-request-observed-probe.c`.

```text
static void probe_bytes(const char*id,const char*src,int source_len){caseid=id;pathid="fixed-grammar";active=create();Obj*o=NEW(active,src,source_len);INC(o);int code=SCRIPT(active,o);report("native-result",code);fprintf(stderr, "DICT_RELEASE_BODY_RETURN|errors=%d|refcount_before_release=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n", tclLspDictErrorObserved, tclLspDictErrorRefcount, tclLspDictErrorSameCell, tclLspDictErrorDefinedScalar, tclLspDictFinalFreeObserved);DEC(active,o);DESTROY(active);}

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.1.0/original-tclExecute.c`, function `TclExecuteByteCode::INST_DICT_UPDATE_END original error cleanup`, lines 7684–7698. Retained reference `native_dict_update_write_error_lifetime398-request-9.1.0-original-tclExecute.c`.

```text
	    DECACHE_STACK_INFO();
	    objResultPtr = TclPtrSetVarIdx(interp, varPtr, NULL, NULL, NULL,
		    dictPtr, TCL_LEAVE_ERR_MSG, varIdx);
	    CACHE_STACK_INFO();
	    if (objResultPtr == NULL) {
		if (allocateDict) {
		    TclDecrRefCount(dictPtr);
		}
		TRACE_ERROR(interp);
		goto gotError;
	    }
	}
	TRACE_APPEND("written back\n");
	NEXT_INST_F0(9, 1);


```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.1.0/observed-tclExecute.c`, function `TclExecuteByteCode::INST_DICT_UPDATE_END live pre-release observer`, lines 7693–7712. Retained reference `native_dict_update_write_error_lifetime398-request-9.1.0-observed-tclExecute.c`.

```text
	    objResultPtr = TclPtrSetVarIdx(interp, varPtr, NULL, NULL, NULL,
		    dictPtr, TCL_LEAVE_ERR_MSG, varIdx);
	    CACHE_STACK_INFO();
	    if (objResultPtr == NULL) {
		if (allocateDict) {
                    tclLspDictErrorObserved++;
                    tclLspDictErrorRefcount = dictPtr->refCount;
                    tclLspDictErrorSameCell = (varPtr->value.objPtr == dictPtr);
                    tclLspDictErrorDefinedScalar =
                        TclIsVarScalar(varPtr) && !TclIsVarUndefined(varPtr);
                    tclLspDictReleaseWatch = dictPtr;
		    TclDecrRefCount(dictPtr);
		}
		TRACE_ERROR(interp);
		goto gotError;
	    }
	}
	TRACE_APPEND("written back\n");
	NEXT_INST_F0(9, 1);


```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.1.0/observed-tclObj.c`, function `TclFreeObj watched entry (conditional debug/non-debug branches)`, lines 1274–1288. Retained reference `native_dict_update_write_error_lifetime398-request-9.1.0-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)		/* The object to be freed. */
{
    if (objPtr == tclLspDictReleaseWatch) {
        tclLspDictFinalFreeObserved++;
        tclLspDictReleaseWatch = NULL;
    }
    const Tcl_ObjType *typePtr = objPtr->typePtr;

    /*
     * This macro declares a variable, so must come here...
     */

    ObjInitDeletionContext(context);

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.1.0/observed-tclObj.c`, function `TclFreeObj watched entry (conditional debug/non-debug branches)`, lines 1403–1417. Retained reference `native_dict_update_write_error_lifetime398-request-9.1.0-observed-tclObj.c`.

```text
void
TclFreeObj(
    Tcl_Obj *objPtr)		/* The object to be freed. */
{
    if (objPtr == tclLspDictReleaseWatch) {
        tclLspDictFinalFreeObserved++;
        tclLspDictReleaseWatch = NULL;
    }
    /*
     * Invalidate the string rep first so we can use the bytes value for our
     * pointer chain, and signal an obj deletion (as opposed to shimmering)
     * with 'length == -1'.
     */

    TclInvalidateStringRep(objPtr);

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/9.1.0/original-tclVar.c`, function `TclPtrSetVarIdx original replacement and write-trace failure`, lines 2079–2106. Retained reference `native_dict_update_write_error_lifetime398-request-9.1.0-original-tclVar.c`.

```text
	 * In this case we are replacing the value, so we don't need to do
	 * more than swap the objects.
	 */

	varPtr->value.objPtr = newValuePtr;
	Tcl_IncrRefCount(newValuePtr);		/* Var is another ref. */
	if (oldValuePtr != NULL) {
	    TclDecrRefCount(oldValuePtr);	/* Discard old value. */
	}
    }

    /*
     * Invoke any write traces for the variable.
     */

    if ((varPtr->flags & VAR_TRACED_WRITE)
	    || (arrayPtr && (arrayPtr->flags & VAR_TRACED_WRITE))) {
	if (TCL_ERROR == TclObjCallVarTraces(iPtr, arrayPtr, varPtr, part1Ptr,
		part2Ptr, (flags & (TCL_GLOBAL_ONLY|TCL_NAMESPACE_ONLY))
		| TCL_TRACE_WRITES, (flags & TCL_LEAVE_ERR_MSG), index)) {
	    goto cleanup;
	}
    }

    /*
     * Return the variable's value unless the variable was changed in some
     * gross way by a trace (e.g. it was unset and then recreated as an
     * array).

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime398/observed-probe.c`, function `Independent final counter report after source-object release and interpreter teardown`, lines 185–186. Retained reference `native_dict_update_write_error_lifetime398-request-observed-probe.c`.

```text
fprintf(stderr, "DICT_RELEASE_OBSERVER|errors=%d|refcount_before_release=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n", tclLspDictErrorObserved, tclLspDictErrorRefcount, tclLspDictErrorSameCell, tclLspDictErrorDefinedScalar, tclLspDictFinalFreeObserved);
return 0;}

```

tcl9.1 9.1.0, revision `Independently pinned original source and separate observer build; source-control revision unrecorded.`, `/tmp/native-dictionary-update-lifetime399/observed-probe.c`, function `Independent body-return integer report before source-object release and interpreter teardown`, lines 127–127. Retained reference `native_dict_update_write_error_lifetime399-request-observed-probe.c`.

```text
static void probe_bytes(const char*id,const char*src,int source_len){caseid=id;pathid="fixed-grammar";active=create();Obj*o=NEW(active,src,source_len);INC(o);int code=SCRIPT(active,o);report("native-result",code);fprintf(stderr, "DICT_RELEASE_BODY_RETURN|errors=%d|refcount_before_release=%d|same_cell=%d|defined_scalar=%d|final_free=%d\n", tclLspDictErrorObserved, tclLspDictErrorRefcount, tclLspDictErrorSameCell, tclLspDictErrorDefinedScalar, tclLspDictFinalFreeObserved);DEC(active,o);DESTROY(active);}

```


## Consumer bindings

- [rust/tcl-vm/src/cmd_dict.rs](../../../../rust/tcl-vm/src/cmd_dict.rs), `cmd_dict::native_rmw_fixture_tests::dictionary_body_writeback_compares_original_native_observation_windows` (linked): Retain329 unchanged captured outer process-code references and326 complete results plus three bounded post-free observations: C8.5/C8.6 update-error and C8.6 with-error. Every later outer/result/diagnostic/caller-read field remains captured output without defined storage/completion/read/effect or another archived executable private lifetime; the original seven signal controls remain separate.
- [rust/tcl-vm/src/cmd_dict.rs](../../../../rust/tcl-vm/src/cmd_dict.rs), `cmd_dict::native_rmw_fixture_tests::c86_dictionary_write_error_retains_its_own_defined_host_storage` (linked): Verify exact398 source, baseline/observer bytes and reached integer counters, then independently assert VM safe live dictionary header/member storage. Native freed bytes, old captured-header identity and compiled frame ownership are not reconstructed.

A named test is a coverage binding, not a claim that it executed.

## Replay

Both exact request/launcher programs, all required input mappings, original and independent observer sources/patches/archives/objects/probes/executables, actual command argument vectors/working directories, process exits and whole streams are retained. Absolute paths and inherited build environment/compiler version are not made portable or reconstructed. Request398 records timeout=false; request399 completed process receipts retain returned exits without a timeout field. Signal(-4) outcomes remain failures, not completions. Publication runs no native compiler, launcher, provider or Rust test. Re-execution is a new observation and may change post-free output or signal timing.

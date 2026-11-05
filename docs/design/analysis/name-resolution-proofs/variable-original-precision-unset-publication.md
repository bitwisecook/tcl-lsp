# naming.variable.original-precision-unset-publication

Kind: `native-observation`

## Problem statement

A final failing source completion cannot locate which precision read failed or report later reads. Root unset, linked unset and an unrelated same-tail variable require their own caught public observations.

## Question

What exact existence and caught read results follow root or linked unset of precision, and how does an unrelated namespace variable with the same tail behave?

## Conclusion

For C8.4/C8.5/C8.6 the root control reports existence1, caught read0 and value12. The linked control reports old local alias and original root absent, both caught reads failing, but the global reported alias-name ::p read succeeds with value12. C9.0/C9.1/Jim report ordinary absence and caught read failures for root, linked alias, original root and reported alias-name. Every provider reports unrelated namespace value LOCAL and unchanged root12 with both existence checks1. All 18 fresh original source processes and six compilations exit 0 with empty stderr. The separate unguarded whole precision source has ORIGINAL code1 in every provider; that completion alone cannot locate the failing read or answer later operands. These public existence/result fields establish no hidden trace, cell/header identity, allocation, pointer reuse, opcode or generic special-variable lifecycle. BIG-IP and Rust assertions are not tested. The linked Runtime and VM controls use the three untouched originals across all six columns and compare complete original public completion values under genuine NativeCore initialisation, refusing host decline. The Runtime implementation independently captures its actual selected physical precision cell and follows the pinned callback reported-name branch when reinstalling after unset. That software cell owner and the original public native values remain distinct evidence channels; the public rows assert no hidden cell pointer, header or callback mechanism.

## Scope

Three exact original ASCII LF sources run independently in fresh full provider initialisation through the unchanged counted C API driver. Each source explicitly sets precision12; linked unset uses an original upvar before independently caught reads and unrelated namespace storage stays separate. Whole original inputs, actual commands/receipts/streams/ELFs and required provider/header/archive/source/build pins are retained. The independent unguarded original source and all actual provider receipts remain under limited-control with their measured final guest errors; no failing operand is inferred from an uncaught final completion.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18.

Exact independent ORIGINAL payloads: root=1 0 12; linked=0 0 1 {can't read "p": no such variable} 1 {can't read "::tcl_precision": no such variable} 0 12; unrelated=1 LOCAL 1 12. All are original completion0 and actual exit0/empty stderr. The independent unguarded source reports only final ORIGINAL code1 and cannot locate a failing read; no hidden cell/header/trace cause is inferred.

### tcl9.0, tcl9.1, jim

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

Exact independent ORIGINAL payloads: root=0 1 {can't read "::tcl_precision": no such variable}; linked=0 0 1 {can't read "p": no such variable} 1 {can't read "::tcl_precision": no such variable} 1 {can't read "::p": no such variable}; unrelated=1 LOCAL 1 12. All are original completion0 and actual exit0/empty stderr. The independent unguarded source reports only final ORIGINAL code1 and cannot locate a failing read; no hidden cell/header/trace cause is inferred.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

- [runtime/rust/src/interp/native_precision.rs](../../../../runtime/rust/src/interp/native_precision.rs), `Interp::native_precision_trace`: Require the actual selected Runtime physical cell; original reported-name unset routing is an implementation owner, independent of public native pointer/cause evidence.
- [runtime/rust/src/interp/native_precision.rs](../../../../runtime/rust/src/interp/native_precision.rs), `interp::native_precision::tests::original_precision_unset_matches_all_six_native_sources` (linked): Use all three byte-identical root/linked/unrelated originals across six providers with genuine NativeCore; compare complete original completion0/public result values and refuse host decline. No hidden cell/header/private-cause observation.
- [rust/tcl-vm/src/interp/native_precision_tests.rs](../../../../rust/tcl-vm/src/interp/native_precision_tests.rs), `interp::native_precision_tests::original_precision_unset_matches_all_six_native_sources` (linked): Use all three byte-identical root/linked/unrelated originals across six providers with genuine NativeCore; compare complete original completion0/public result values and refuse host decline. No hidden cell/header/private-cause observation.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_precision_unset319](../../../../rust/tcl-registry/tests/data/native_precision_unset319)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_precision_unset319/limited-control/8.4.20/original-precision-unset](../../../../rust/tcl-registry/tests/data/native_precision_unset319/limited-control/8.4.20/original-precision-unset)
- [rust/tcl-registry/tests/data/native_precision_unset319/limited-control/8.5.19/original-precision-unset](../../../../rust/tcl-registry/tests/data/native_precision_unset319/limited-control/8.5.19/original-precision-unset)
- [rust/tcl-registry/tests/data/native_precision_unset319/limited-control/8.6.18/original-precision-unset](../../../../rust/tcl-registry/tests/data/native_precision_unset319/limited-control/8.6.18/original-precision-unset)
- [rust/tcl-registry/tests/data/native_precision_unset319/limited-control/9.0.4/original-precision-unset](../../../../rust/tcl-registry/tests/data/native_precision_unset319/limited-control/9.0.4/original-precision-unset)
- [rust/tcl-registry/tests/data/native_precision_unset319/limited-control/9.1.0/original-precision-unset](../../../../rust/tcl-registry/tests/data/native_precision_unset319/limited-control/9.1.0/original-precision-unset)
- [rust/tcl-registry/tests/data/native_precision_unset319/limited-control/jim/original-precision-unset](../../../../rust/tcl-registry/tests/data/native_precision_unset319/limited-control/jim/original-precision-unset)

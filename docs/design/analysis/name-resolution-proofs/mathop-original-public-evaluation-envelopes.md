# naming.mathop.original-public-evaluation-envelopes

Kind: `native-observation`

## Problem statement

A native arithmetic-worker replacement comparison depends on its exact original evaluation channel. Bare counted completion, a CLI catch/puts envelope and a variable-source catch envelope must retain their own public results and guest stdout before any compiler-selection cause is inferred.

## Question

What public code/result and guest stdout does the original mathop replacement control produce as bare counted source, its exact original CLI envelope and a separate variable-source catch envelope?

## Conclusion

C8.5.19/C8.6.18/C9.0.4/C9.1.0 record bare ORIGINAL code0/result5 OVERRIDE; the exact CLI envelope prints guest catch0/result hex35204f56455252494445 while driver ORIGINAL is code0/empty; the variable-source envelope returns driver code0/result0 {5 OVERRIDE}. C8.4.20/Jim record the original UNAVAILABLE branch: bare code0/UNAVAILABLE, CLI guest0/UNAVAILABLE hex with empty driver completion, and variable-source code0/result0 UNAVAILABLE. All eighteen external source processes and six driver compilations exit0 with empty stderr. Envelope observations agree on these finite caught/public payloads while their outer source completions and stdout channels remain distinct. Agreement supplies no native compiler/opcode identity, cache/epoch mechanism, cause of software disagreement, physical command object/header/frame, successful arbitrary arithmetic or Rust assertion result. BIG-IP is not tested.

## Scope

Three exact ASCII original sources retain their own bytes and hashes. The bare source is original native_mathop_identity case9; the CLI envelope retains its original catch/result binary-scan/puts source, and the variable-source control passes a retained script variable to catch. Each runs in a fresh fully initialised provider under the unchanged Native202 counted C API driver: C Tcl_EvalEx flags0, Jim counted object Jim_EvalObj. The driver adds no source envelope. Whole guest stdout and driver VERSION/ORIGINAL tagged hex are kept in original order, including Jim guest/driver buffering order.

## Provider answers

### tcl8.4, jim

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl8.4 8.4.20; jim 0.84-9-g5bac7c9.

Bare source: code0/result UNAVAILABLE. Exact CLI envelope: guest catch0/result hex554e415641494c41424c45, driver ORIGINAL code0/empty. Variable-source catch: driver code0/result 0 UNAVAILABLE. The original availability guard observes UNAVAILABLE; no replacement body is reached. Whole three stdout/stderr/receipts and actual compiler pins are preserved; no private compiler-selection inference.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Bare source: code0/result 5 OVERRIDE. Exact CLI envelope: guest catch0/result hex35204f56455252494445, driver ORIGINAL code0/empty. Variable-source catch: driver code0/result 0 {5 OVERRIDE}. This is the original public replacement control. Whole three stdout/stderr/receipts and actual compiler pins are preserved; no private compiler-selection inference.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_mathop_public_envelopes222](../../../../rust/tcl-registry/tests/data/native_mathop_public_envelopes222)
- [rust/tcl-registry/tests/data/native_mathop_public_envelopes222/8.4.20](../../../../rust/tcl-registry/tests/data/native_mathop_public_envelopes222/8.4.20)
- [rust/tcl-registry/tests/data/native_mathop_public_envelopes222/8.5.19](../../../../rust/tcl-registry/tests/data/native_mathop_public_envelopes222/8.5.19)
- [rust/tcl-registry/tests/data/native_mathop_public_envelopes222/8.6.18](../../../../rust/tcl-registry/tests/data/native_mathop_public_envelopes222/8.6.18)
- [rust/tcl-registry/tests/data/native_mathop_public_envelopes222/9.0.4](../../../../rust/tcl-registry/tests/data/native_mathop_public_envelopes222/9.0.4)
- [rust/tcl-registry/tests/data/native_mathop_public_envelopes222/9.1.0](../../../../rust/tcl-registry/tests/data/native_mathop_public_envelopes222/9.1.0)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_mathop_public_envelopes222/jim](../../../../rust/tcl-registry/tests/data/native_mathop_public_envelopes222/jim)

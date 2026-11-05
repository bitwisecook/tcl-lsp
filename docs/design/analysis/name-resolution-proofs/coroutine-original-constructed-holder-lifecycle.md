# naming.coroutine.original-constructed-holder-lifecycle

Kind: `native-observation`

## Problem statement

Coroutine reports can combine holder and simple-name colon boundaries into equal bytes. Resumption, relative deletion and relative rename need their own original public controls rather than identity inferred from displayed names.

## Question

Can original coroutine commands created as p in current a: and :p in current a resume independently, and do relative deletion or rename of the first preserve the second despite equal coroutine reports?

## Conclusion

C8.6/C9.0/C9.1 retain ORIGINAL0 `{FIRST ::a:::p} {SECOND ::a:::p} {DONE FIRST} {DONE SECOND}` for independent original relative resumptions. The deletion source returns `{FIRST ::a:::p} {SECOND ::a:::p} 0 {DONE SECOND}`: its original first command query is empty while the second resumes to its own result. Relative rename retains `{FIRST ::a:::p} {SECOND ::a:::p} {DONE FIRST} {DONE SECOND}` through original renamed/second calls. C8.4/C8.5/Jim reach ORIGINAL0/NOT_APPLICABLE through the actual coroutine capability branch. All18 host processes and six compiler commands exit0 with empty stderr. Equal info-coroutine reports do not prove a shared or different private pointer, token, worker sidecar/header, entered frame or stack; the independent original call/deletion/rename outcomes are the measured facts. No coroinject/coroprobe, private lifetime or compiler/Normal authority follows. Linked Runtime and VM controls each compare all three untouched originals across six public columns (18 comparisons per backend), including measured unavailable branches. Runtime resolves the original operand to the actual command registration generation and retains that generation separately from its mutable info-coroutine report; rename updates the report, while completion/deletion retires the selected generation. VM retains its independent actual command-sidecar owner. These implementation joins are bounded software controls: the public native result rows do not certify a native private generation, pointer, worker, sidecar, header, frame or stack, and linked tests contain no executed passing-result claim.

A marked VM definition links eighteen exact public coroutine holder lifecycle windows under the selected core/compiler fixture.

## Scope

Three untouched ASCII LF sources retain original current-holder activation, creation spelling, yielded reporting bytes and independent relative resume/delete/rename operations. Each runs separately in a fresh full-init counted C API process without an added source envelope. Exact originals/requests/driver/launcher/ELFs/whole output/receipts and independently required executable/archive/header/build/source pins are retained. Actual unsupported stock capability branches remain separate from supported coroutine observations. BIG-IP and Rust assertions are not tested.

The source definition launches no original provider process and inspects no private native holder/token pointer, refcount or coroutine frame. Complete code/result bytes preserve availability and independent relative-resume/move/delete branches; reporting-label equality supplies no identity. No executed software outcome, compiled opcode or broader runtime admission is attached.

## Provider answers

### tcl8.4, tcl8.5, jim

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; jim 0.84-9-g5bac7c9.

Every unchanged source reaches ORIGINAL0/NOT_APPLICABLE through its actual capability branch. No supported TclOO/coroutine API or substitute Jim/C implementation is inferred.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Complete ORIGINAL code/value fields: distinct-original-relative-resumptions=0/{FIRST ::a:::p} {SECOND ::a:::p} {DONE FIRST} {DONE SECOND}; original-relative-deletion-preserves-other-holder=0/{FIRST ::a:::p} {SECOND ::a:::p} 0 {DONE SECOND}; original-relative-rename-preserves-other-holder=0/{FIRST ::a:::p} {SECOND ::a:::p} {DONE FIRST} {DONE SECOND}.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

- [runtime/rust/src/interp/native_coroutine_names.rs](../../../../runtime/rust/src/interp/native_coroutine_names.rs), `Interp::register_coroutine_command_in_slot`: Retain the actual selected software command registration generation independently of rendered coroutine reports.
- [runtime/rust/src/interp/native_command_names.rs](../../../../runtime/rust/src/interp/native_command_names.rs), `Interp::resolve_original_command_generation`: Resolve genuine original command operands through the actual command owner rather than infer registration from a displayed name.
- [runtime/rust/src/cmd_coro.rs](../../../../runtime/rust/src/cmd_coro.rs), `CoroIdentity`: Keep actual software coroutine generation separate from mutable public report bytes; original source results grant no native private identity.
- [runtime/rust/src/cmd_coro/native_original_holder_tests.rs](../../../../runtime/rust/src/cmd_coro/native_original_holder_tests.rs), `cmd_coro::native_original_holder_tests::original_coroutine_holder_lifecycle_matches_all_native_public_results` (linked): Compare all three exact original-relative resume/delete/rename sources against complete six-provider original completion code/result rows (18 comparisons), including unavailable branches, and reject host refusal. No private native generation or passing Rust execution follows from the linked source test.

- [rust/tcl-vm/src/cmd_coro/native_original_holder_tests.rs](../../../../rust/tcl-vm/src/cmd_coro/native_original_holder_tests.rs), `original_result`: Decode exact retained ORIGINAL code/result-hex fields for public coroutine comparison, without creating a token or private holder observation.
- [rust/tcl-vm/src/native_fixture.rs](../../../../rust/tcl-vm/src/native_fixture.rs), `interpreter`: Construct an actual selected software native core and independently install its source compiler for the original-source conformance fixture; distribution libraries require separate explicit selection.
- [rust/tcl-vm/src/cmd_coro/native_original_holder_tests.rs](../../../../rust/tcl-vm/src/cmd_coro/native_original_holder_tests.rs), `cmd_coro::native_original_holder_tests::original_coroutine_holder_lifecycle_matches_all_native_public_results` (linked): Three unchanged original ASCII coroutine holder programs across six selected software cores compare all18 original public completion/result windows and require no Host cause. Relative resumption, deletion and rename remain separate whole inputs; equal info coroutine reporting text is not physical holder equality.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-registry/tests/data/native_coroutine_original_holder250](../../../../rust/tcl-registry/tests/data/native_coroutine_original_holder250)
- [rust/tcl-registry/tests/data/native_coroutine_original_holder250/8.4.20](../../../../rust/tcl-registry/tests/data/native_coroutine_original_holder250/8.4.20)
- [rust/tcl-registry/tests/data/native_coroutine_original_holder250/8.5.19](../../../../rust/tcl-registry/tests/data/native_coroutine_original_holder250/8.5.19)
- [rust/tcl-registry/tests/data/native_coroutine_original_holder250/8.6.18](../../../../rust/tcl-registry/tests/data/native_coroutine_original_holder250/8.6.18)
- [rust/tcl-registry/tests/data/native_coroutine_original_holder250/9.0.4](../../../../rust/tcl-registry/tests/data/native_coroutine_original_holder250/9.0.4)
- [rust/tcl-registry/tests/data/native_coroutine_original_holder250/9.1.0](../../../../rust/tcl-registry/tests/data/native_coroutine_original_holder250/9.1.0)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_coroutine_original_holder250/jim](../../../../rust/tcl-registry/tests/data/native_coroutine_original_holder250/jim)

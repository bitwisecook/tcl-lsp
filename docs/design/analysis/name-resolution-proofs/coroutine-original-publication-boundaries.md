# naming.coroutine.original-publication-boundaries

Kind: `native-observation`

## Problem statement

String-only coroutine creation can reject opaque native bytes, choose the wrong holder or create namespaces the native coroutine handler requires to exist.

## Question

Which name bytes, namespace holder, missing-namespace errors and empty simple names does coroutine creation select in the finite original-object controls?

## Conclusion

C8.6.18/9.0.4/9.1.0 accept 26 of 30 original name/context pairs and reject four missing-holder pairs without creating Missing. The original name pointer reaches the driver unchanged. Raw zero clips the CString name; C080, FF and D800 remain separate accepted bytes. Empty names and existing Q:: empty tails are accepted. v2 info-coroutine results independently report the current holder for bare/relative names and the global holder for absolute names. C8.4/8.5 and current Jim report coroutine unavailable in these controls.

## Scope

Two immutable all-six variants, 15 original names in global and N contexts. v1 start/resume and inventory cannot alone establish publication holder; v2 info-coroutine fullname controls supply the independent holder discriminator. Setup creates N, N::Q and Q only. Original Tcl string objects carry explicitly counted bytes, not ByteArray getters or Document/source UTF re-encoding. Supported C85+ options are post-result observations. No command cache/compiler/preparation, fresh holder creation, lifecycle or body Normal grant.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: Original counted native string objects and direct object vectors. Publication driver receives the unchanged name object inside a selected namespace-eval list; v2 separately executes info coroutine to report the actual published token name. C return options are measured after the result getter where available; C8.4/Jim options APIs are unqueried.. Dialect: Tcl.

All 30 attempted coroutine name/context controls reject because coroutine is unavailable; publication/resume/fullname semantics were not reached. Jim Missing namespace-exists queries are themselves unsupported, so they do not prove absence.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: Original counted native string objects and direct object vectors. Publication driver receives the unchanged name object inside a selected namespace-eval list; v2 separately executes info coroutine to report the actual published token name. C return options are measured after the result getter where available; C8.4/Jim options APIs are unqueried.. Dialect: Tcl.

All 30 attempted coroutine name/context controls reject because coroutine is unavailable; publication/resume/fullname semantics were not reached. Jim Missing namespace-exists queries are themselves unsupported, so they do not prove absence.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: Original counted native string objects and direct object vectors. Publication driver receives the unchanged name object inside a selected namespace-eval list; v2 separately executes info coroutine to report the actual published token name. C return options are measured after the result getter where available; C8.4/Jim options APIs are unqueried.. Dialect: Tcl.

C8.6.18/9.0.4/9.1.0 accept 26 of 30 original name/context pairs and reject four missing-holder pairs without creating Missing. The original name pointer reaches the driver unchanged. Raw zero clips the CString name; C080, FF and D800 remain separate accepted bytes. Empty names and existing Q:: empty tails are accepted. v2 info-coroutine results independently report the current holder for bare/relative names and the global holder for absolute names. C8.4/8.5 and current Jim report coroutine unavailable in these controls.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: Original counted native string objects and direct object vectors. Publication driver receives the unchanged name object inside a selected namespace-eval list; v2 separately executes info coroutine to report the actual published token name. C return options are measured after the result getter where available; C8.4/Jim options APIs are unqueried.. Dialect: Tcl.

C8.6.18/9.0.4/9.1.0 accept 26 of 30 original name/context pairs and reject four missing-holder pairs without creating Missing. The original name pointer reaches the driver unchanged. Raw zero clips the CString name; C080, FF and D800 remain separate accepted bytes. Empty names and existing Q:: empty tails are accepted. v2 info-coroutine results independently report the current holder for bare/relative names and the global holder for absolute names. C8.4/8.5 and current Jim report coroutine unavailable in these controls.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: Original counted native string objects and direct object vectors. Publication driver receives the unchanged name object inside a selected namespace-eval list; v2 separately executes info coroutine to report the actual published token name. C return options are measured after the result getter where available; C8.4/Jim options APIs are unqueried.. Dialect: Tcl.

C8.6.18/9.0.4/9.1.0 accept 26 of 30 original name/context pairs and reject four missing-holder pairs without creating Missing. The original name pointer reaches the driver unchanged. Raw zero clips the CString name; C080, FF and D800 remain separate accepted bytes. Empty names and existing Q:: empty tails are accepted. v2 info-coroutine results independently report the current holder for bare/relative names and the global holder for absolute names. C8.4/8.5 and current Jim report coroutine unavailable in these controls.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual reported version, configured library/header/source/Makefile/executable hashes and compile/process0 retained independently for both variants.. Channel: Original counted native string objects and direct object vectors. Publication driver receives the unchanged name object inside a selected namespace-eval list; v2 separately executes info coroutine to report the actual published token name. C return options are measured after the result getter where available; C8.4/Jim options APIs are unqueried.. Dialect: Jim.

All 30 attempted coroutine name/context controls reject because coroutine is unavailable; publication/resume/fullname semantics were not reached. Jim Missing namespace-exists queries are themselves unsupported, so they do not prove absence.

### bigip

Status: `not-tested`. Version: not measured. Build: not recorded. Channel: No input supplied. Dialect: BIG-IP.

No observation for this question.

## Exact evidence

- `probe-v1` (input): [rust/tcl-registry/tests/data/native_coroutine_publication/probe.c](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/probe.c). SHA-256 `e47227595500d3399848c7a42fe0867de33df4f25c87a8f7b765692f7ee397bb`. Exact immutable original-object v1 probe source; count constructors, pointer timing, per-stage getters and untested API branches are inspectable.
- `probe-v2` (input): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/probe.c](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/probe.c). SHA-256 `ab41e399af2ba7e074d779a1ee7e76fd9fe16e127c9d07f060010e2c1aae6a44`. Exact v2 probe, adding separate info-coroutine fullname controls without changing the v1 outcomes.
- `tcl8.4-v1-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/8.4.20/receipt.json). SHA-256 `223324cc59c6ef3461fac683933a8870677da186ec139b7d3c9a8f2741273bbc`. Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl8.4-v1-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/8.4.20/stdout.tsv). SHA-256 `c09e51943e575e81f93693134a43356065c24ae51b9bdc89bc83eb0968190ee9`. Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl8.4-v1-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact process stderr, separate from guest completion.
- `tcl8.4-v2-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.4.20/receipt.json). SHA-256 `5dd08cd6a90a0cd3c7454c2ff63091cbae44f09cdf583c2187dadc8442d896f1`. Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl8.4-v2-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.4.20/stdout.tsv). SHA-256 `6cc6b2ad5731a5d394dc07ccf0449f09f514be86920be0c02c82f3365b4ba68a`. Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl8.4-v2-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact process stderr, separate from guest completion.
- `tcl8.5-v1-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/8.5.19/receipt.json). SHA-256 `61e1c0e43949265c2f60e4e758fbc8e438ec9874be1cc6030a15c2b2196078bd`. Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl8.5-v1-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/8.5.19/stdout.tsv). SHA-256 `a11bab67ac23b8bbd55e65b1ee7c3d15a922464ec137f1241c4a69b3261c1a05`. Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl8.5-v1-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact process stderr, separate from guest completion.
- `tcl8.5-v2-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.5.19/receipt.json). SHA-256 `39d7ac301aaaaeb21372338eab79cf5a60ad6da521a573aac3f1b186fd93fc6b`. Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl8.5-v2-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.5.19/stdout.tsv). SHA-256 `b4b80f1449819cfc861e9cc682afcf574a397b5f0c42d5cab124954d90f59986`. Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl8.5-v2-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact process stderr, separate from guest completion.
- `tcl8.6-v1-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/8.6.18/receipt.json). SHA-256 `331d3e480ae49ad04c6c61265eccbac35e80b43257b42ea8f8271ecd5e2c3481`. Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl8.6-v1-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/8.6.18/stdout.tsv). SHA-256 `1dc871ca7b937c84b92b2e334dfe472714b6dbb32c9372b3e9ea63d7a99668c9`. Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl8.6-v1-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact process stderr, separate from guest completion.
- `tcl8.6-v2-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.6.18/receipt.json). SHA-256 `8043de3fc616f71722feccbb550d117d5f573f5dff9f0d162ac57fcc3ecea4e3`. Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl8.6-v2-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.6.18/stdout.tsv). SHA-256 `82d4c0f4f5c63f8e02731ae42e686d0826162f6a1775646166c2736c69475f66`. Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl8.6-v2-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact process stderr, separate from guest completion.
- `tcl9.0-v1-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.0.4/receipt.json). SHA-256 `c1bf5098a3ebbaa72add2fb01c095a26f03dea780f38d90f712dd757849afecd`. Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl9.0-v1-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.0.4/stdout.tsv). SHA-256 `72a921792befe2b7cfa21b4a62901691dbf3c72adc2c787ec43a65fa4d19ae25`. Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl9.0-v1-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact process stderr, separate from guest completion.
- `tcl9.0-v2-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.0.4/receipt.json). SHA-256 `2b48085da900371c3947fbc6befa11418fc55c8f498fc4c4e9d9742edfd97b26`. Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl9.0-v2-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.0.4/stdout.tsv). SHA-256 `b8c2e31c53ce64e96986734dfe0ef0cb2de8440c07e5e0325d43822732c80dd8`. Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl9.0-v2-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact process stderr, separate from guest completion.
- `tcl9.1-v1-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/receipt.json). SHA-256 `13209de752ec751ea45bb1c8136d13959a3d54f4001b79fefce9f2f06405734f`. Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl9.1-v1-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stdout.tsv). SHA-256 `94805bbf1a96f65176a2aff451195912444e292b6ceec360898ebb1204830d04`. Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl9.1-v1-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact process stderr, separate from guest completion.
- `tcl9.1-v2-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/receipt.json). SHA-256 `9bd5decf3fa0e8b465b817c8eef53551aa674896bfe582592011660c006300a6`. Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `tcl9.1-v2-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stdout.tsv). SHA-256 `acaa106607bb5e8cbc663efaf2d39bc08cbcf5fe754fb8bb9524e743ae3429d3`. Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `tcl9.1-v2-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact process stderr, separate from guest completion.
- `jim-v1-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/jim/receipt.json). SHA-256 `e24afd84b7d2dba62c7c2a80eb125189edfda32890364c037d1552e3f2a016ae`. Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `jim-v1-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/jim/stdout.tsv). SHA-256 `5821ad5c2742f982fb99beeb20c5a9b5b30315a6b14b9ac481642d28b134d90e`. Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `jim-v1-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/jim/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact process stderr, separate from guest completion.
- `jim-v2-receipt` (provider): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/jim/receipt.json). SHA-256 `c643cbf227ba2dab16fc16886dcfb95ef7c50747331844d283877c2f9fc2b5c9`. Actual compile/process exits, reported version and exact library/header/source/input/executable/stream associations.
- `jim-v2-rows` (observation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/jim/stdout.tsv). SHA-256 `d15b7f5bd03bebc960d3333c99ade9977e1766e8817395078ad213700589f8c3`. Exact finite protocol rows; stage labels preserve the distinction between observations, input-only rows and untested operations.
- `jim-v2-stderr` (limitation): [rust/tcl-registry/tests/data/native_coroutine_publication/v2/jim/stderr](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/v2/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact process stderr, separate from guest completion.
- `source-0` (source-anchor): [rust/tcl-registry/tests/data/native_coroutine_publication/source-windows.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/source-windows.json). SHA-256 `ef5ccbf8675a6c164c672a128a6eab1cf44a59017558788e48d8c09dbc54c306`. JSON pointer `/0/snippet`. Exact pinned TclNRCoroutineObjCmd source window, independent of result observations.
- `source-1` (source-anchor): [rust/tcl-registry/tests/data/native_coroutine_publication/source-windows.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/source-windows.json). SHA-256 `ef5ccbf8675a6c164c672a128a6eab1cf44a59017558788e48d8c09dbc54c306`. JSON pointer `/1/snippet`. Exact pinned TclNRCoroutineObjCmd source window, independent of result observations.
- `source-2` (source-anchor): [rust/tcl-registry/tests/data/native_coroutine_publication/source-windows.json](../../../../rust/tcl-registry/tests/data/native_coroutine_publication/source-windows.json). SHA-256 `ef5ccbf8675a6c164c672a128a6eab1cf44a59017558788e48d8c09dbc54c306`. JSON pointer `/2/snippet`. Exact pinned TclNRCoroutineObjCmd source window, independent of result observations.
- `coroutine-resume-registration-source-tcl8.6` (source-anchor): [rust/tcl-registry/tests/data/native_coroutine_resume_source/source-windows.json](../../../../rust/tcl-registry/tests/data/native_coroutine_resume_source/source-windows.json). SHA-256 `d4508103ded2ec9b0de1d18b32d51a66deff0c8b0bcad0fa37b2c937004d99b3`. JSON pointer `/0/snippet`. Native resume consumes the selected coroutine clientData; argv[0] is diagnostic text only. No coroutine-name lookup is performed by this handler.
- `coroutine-resume-registration-source-tcl9.0` (source-anchor): [rust/tcl-registry/tests/data/native_coroutine_resume_source/source-windows.json](../../../../rust/tcl-registry/tests/data/native_coroutine_resume_source/source-windows.json). SHA-256 `d4508103ded2ec9b0de1d18b32d51a66deff0c8b0bcad0fa37b2c937004d99b3`. JSON pointer `/1/snippet`. Native resume consumes the selected coroutine clientData; argv[0] is diagnostic text only. No coroutine-name lookup is performed by this handler.
- `coroutine-resume-registration-source-tcl9.1` (source-anchor): [rust/tcl-registry/tests/data/native_coroutine_resume_source/source-windows.json](../../../../rust/tcl-registry/tests/data/native_coroutine_resume_source/source-windows.json). SHA-256 `d4508103ded2ec9b0de1d18b32d51a66deff0c8b0bcad0fa37b2c937004d99b3`. JSON pointer `/2/snippet`. Native resume consumes the selected coroutine clientData; argv[0] is diagnostic text only. No coroutine-name lookup is performed by this handler.

## Source inspection

tcl9.1 9.1.0, revision `Exact pinned source associated by immutable native coroutine publication receipts`, `generic/tclBasic.c`, function `TclNRInterpCoroutine`, lines 9983–10034. Full-source SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`; snippet SHA-256 `7c0939272a7d1c1cdcb18b7bc4fa671751c6ce5ea31f13fdc0549ebcb2903c90`; retained evidence `coroutine-resume-registration-source-tcl9.1`.

```text
int
TclNRInterpCoroutine(
    void *clientData,
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    CoroutineData *corPtr = (CoroutineData *)clientData;

    if (!COR_IS_SUSPENDED(corPtr)) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"coroutine \"%s\" is already running",
		TclGetString(objv[0])));
	Tcl_SetErrorCode(interp, "TCL", "COROUTINE", "BUSY", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Parse all the arguments to work out what to feed as the result of the
     * [yield]. TRICKY POINT: objc==0 happens here! It occurs when a coroutine
     * is deleted!
     */

    switch (corPtr->nargs) {
    case COROUTINE_ARGUMENTS_SINGLE_OPTIONAL:
	if (objc == 2) {
	    Tcl_SetObjResult(interp, objv[1]);
	} else if (objc > 2) {
	    Tcl_WrongNumArgs(interp, 1, objv, "?arg?");
	    return TCL_ERROR;
	}
	break;
    default:
	if (corPtr->nargs + 1 != objc) {
	    Tcl_SetObjResult(interp, Tcl_NewStringObj(
		    "wrong coro nargs; how did we get here? "
		    "not implemented!", TCL_INDEX_NONE));
	    Tcl_SetErrorCode(interp, "TCL", "WRONGARGS", (char *)NULL);
	    return TCL_ERROR;
	}
	TCL_FALLTHROUGH();
    case COROUTINE_ARGUMENTS_ARBITRARY:
	if (objc > 1) {
	    Tcl_SetObjResult(interp, Tcl_NewListObj(objc - 1, objv + 1));
	}
	break;
    }

    TclNRAddCallback(interp, TclNRCoroutineActivateCallback, corPtr,
	    NULL, NULL, NULL);
    return TCL_OK;
}

```

tcl9.0 9.0.4, revision `Pinned full source, independently associated by captured receipt required_sha256`, `generic/tclBasic.c`, function `TclNRCoroutineObjCmd`, lines 9737–9786. Full-source SHA-256 `6d995c1af94fea92717183882ca41775fe8c0e088def163f6ac8fe699cbffc67`; snippet SHA-256 `76a4a21dfafdc024444202fb4186be5666b3f7de4fb229f8d6fa453fdbfeac33`; retained evidence `source-1`.

```text
int
TclNRCoroutineObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const objv[])	/* Argument objects. */
{
    Command *cmdPtr;
    CoroutineData *corPtr;
    const char *procName, *simpleName;
    Namespace *nsPtr, *altNsPtr, *cxtNsPtr,
	*inNsPtr = (Namespace *)TclGetCurrentNamespace(interp);
    Namespace *lookupNsPtr = iPtr->varFramePtr->nsPtr;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "name cmd ?arg ...?");
	return TCL_ERROR;
    }

    procName = TclGetString(objv[1]);
    TclGetNamespaceForQualName(interp, procName, inNsPtr, 0,
	    &nsPtr, &altNsPtr, &cxtNsPtr, &simpleName);

    if (nsPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't create procedure \"%s\": unknown namespace",
		procName));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "NAMESPACE", (char *)NULL);
	return TCL_ERROR;
    }
    if (simpleName == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't create procedure \"%s\": bad procedure name",
		procName));
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "COMMAND", procName, (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * We ARE creating the coroutine command: allocate the corresponding
     * struct and create the corresponding command.
     */

    corPtr = (CoroutineData *)Tcl_Alloc(sizeof(CoroutineData));

    cmdPtr = (Command *) TclNRCreateCommandInNs(interp, simpleName,
	    (Tcl_Namespace *)nsPtr, /*objProc*/ NULL, TclNRInterpCoroutine,
	    corPtr, DeleteCoroutine);

    corPtr->cmdPtr = cmdPtr;

```

tcl9.1 9.1.0, revision `Pinned full source, independently associated by captured receipt required_sha256`, `generic/tclBasic.c`, function `TclNRCoroutineObjCmd`, lines 10047–10096. Full-source SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`; snippet SHA-256 `809ad73194e8f1b7966944017b0472e6ae161c8f20455b90065bb027bc712083`; retained evidence `source-2`.

```text
int
TclNRCoroutineObjCmd(
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Current interpreter. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* Argument objects. */
{
    Command *cmdPtr;
    CoroutineData *corPtr;
    const char *procName, *simpleName;
    Namespace *nsPtr, *altNsPtr, *cxtNsPtr;
    Namespace *inNsPtr = (Namespace *)TclGetCurrentNamespace(interp);
    Namespace *lookupNsPtr = iPtr->varFramePtr->nsPtr;

    if (objc < 3) {
	Tcl_WrongNumArgs(interp, 1, objv, "name cmd ?arg ...?");
	return TCL_ERROR;
    }

    procName = TclGetString(objv[1]);
    TclGetNamespaceForQualName(interp, procName, inNsPtr, 0,
	    &nsPtr, &altNsPtr, &cxtNsPtr, &simpleName);

    if (nsPtr == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't create procedure \"%s\": unknown namespace",
		procName));
	Tcl_SetErrorCode(interp, "TCL", "LOOKUP", "NAMESPACE", (char *)NULL);
	return TCL_ERROR;
    }
    if (simpleName == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"can't create procedure \"%s\": bad procedure name",
		procName));
	Tcl_SetErrorCode(interp, "TCL", "VALUE", "COMMAND", procName, (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * We ARE creating the coroutine command: allocate the corresponding
     * struct and create the corresponding command.
     */

    corPtr = (CoroutineData *)Tcl_Alloc(sizeof(CoroutineData));

    cmdPtr = (Command *) TclNRCreateCommandInNs(interp, simpleName,
	    (Tcl_Namespace *)nsPtr, /*objProc*/ NULL, TclNRInterpCoroutine,
	    corPtr, DeleteCoroutine);

    corPtr->cmdPtr = cmdPtr;

```


## Consumer bindings

- [rust/tcl-vm/src/interp/native_coroutine_names.rs](../../../../rust/tcl-vm/src/interp/native_coroutine_names.rs), `original_coroutine_names_match_native_publication_controls`: Actual original-object comparison harness; v2 fullname supplement independent of v1 relative lookup.
- [runtime/rust/src/interp/native_coroutine_names.rs](../../../../runtime/rust/src/interp/native_coroutine_names.rs), `original_coroutine_names_match_native_publication_controls`: Actual original-object comparison harness; v2 fullname supplement independent of v1 relative lookup.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::coroutine_publication_slot`: Independently selected name extent only; actual holder and publication remain backend-owned.
- [runtime/rust/src/interp/native_compilation.rs](../../../../runtime/rust/src/interp/native_compilation.rs), `Interp::enter_native_builtin`: Retains actual selected registration generation separately from optional stock identity, with existing activation guard/context lifetime; direct compiled helpers carry no generation.
- [runtime/rust/src/interp/native_compilation.rs](../../../../runtime/rust/src/interp/native_compilation.rs), `Interp::active_builtin_command_placement`: Queries placement of actual selected generation, without reconstruction from original argv or donation from stock metadata.
- [runtime/rust/src/cmd_coro.rs](../../../../runtime/rust/src/cmd_coro.rs), `coro_resume_command`: Uses actual selected coroutine placement while retaining unchanged caller argument objects. Does not reparse bytes after the native name extent.
- [rust/tcl-vm/src/cmd_coro.rs](../../../../rust/tcl-vm/src/cmd_coro.rs), `coro_resume`: Uses the actual selected command sidecar and retained original argv[0]; no text representation or command-name relookup selects the coroutine.
- [rust/tcl-vm/src/interp/native_coroutine_names.rs](../../../../rust/tcl-vm/src/interp/native_coroutine_names.rs), `interp::native_coroutine_names::tests::original_coroutine_names_match_native_publication_controls` (linked): Original starts/resumes, exact missing-namespace errors and no Missing creation; v2 comparison supplement adds separately measured info-coroutine published fullname. No test execution claimed.
- [runtime/rust/src/interp/native_coroutine_names.rs](../../../../runtime/rust/src/interp/native_coroutine_names.rs), `interp::native_coroutine_names::tests::original_coroutine_names_match_native_publication_controls` (linked): Original starts/resumes, exact missing-namespace errors and no Missing creation; v2 comparison supplement adds separately measured info-coroutine published fullname. No test execution claimed.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `naming::native::coroutine_publication_tests::coroutine_slots_keep_current_holders_and_native_byte_extents` (linked): Pure selected CString slot geometry, including existing holders and raw-zero clipping; no handler or availability grant.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-registry/tests/data/native_coroutine_publication/verify.py"
]
```

Offline exact receipt/input/stream association verification only; zero native/compiler/Rust launches. Original capture.py and queue retain absolute provisioned input pins and immutable output paths; fresh recapture must use new output directories and recheck every required hash. Source windows reproduce separately from exact pinned full source LF ranges. No missing ELF is fabricated or claimed portable.

# naming.tcloo.native-forward-registration-prefix-boundaries

Kind: `native-observation`

## Problem statement

Forward registration stores prefix values and future evaluation is a separate operation; a definition-scope variable error cannot be relabelled as a forward registration failure.

## Question

Which exact fresh stock forward source scripts register, replace and later invoke their prefix under the six retained providers?

## Conclusion

C86/C90/C91 agree on static missing target, no target call at definition, replacements, empty target, required prefix and superclass controls. The complete dynamic PREFIX_CAPTURE source differs: C86 returns INITIAL EXTRA and C90/C91 error reading value. C84/C85/Jim report absent OO availability.

## Scope

39 exact version/answer rows from separate startup/availability and nine fresh-interpreter ASCII source scripts. No source-preflight/CPP, opaque name, cache/pointer/storage layout, callback absence, generic variable scope equivalence, implementation Normal transfer or arbitrary method chain claim. Native probe result getter records counted bytes, but inputs are ASCII NUL-terminated source, not original object vectors.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Actual executable SHA 500161ca9b98f212033b3aec3d5de61fb8d38804b19898effa08e34db8e0fb46; retained compile argv and SDK/header/library/build/source/stream joins. Compiler binary/version not retained.. Channel: separate actual ASCII info patchlevel/availability startup; each case uses a fresh interpreter and exact ASCII NUL-terminated Tcl_Eval or Jim_Eval source. Dialect: Tcl.

Availability is empty; no forward script case is run.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Actual executable SHA e40696c7691e5f56c5e8747a3aae9c2cac0109fc056e6fe16892bfdec7a45e0e; retained compile argv and SDK/header/library/build/source/stream joins. Compiler binary/version not retained.. Channel: separate actual ASCII info patchlevel/availability startup; each case uses a fresh interpreter and exact ASCII NUL-terminated Tcl_Eval or Jim_Eval source. Dialect: Tcl.

Availability is empty; no forward script case is run.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable SHA a7557820ed77f1018379d62e06f23505d9290a5f3666f5baa581c3c0392aceb5; retained compile argv and SDK/header/library/build/source/stream joins. Compiler binary/version not retained.. Channel: separate actual ASCII info patchlevel/availability startup; each case uses a fresh interpreter and exact ASCII NUL-terminated Tcl_Eval or Jim_Eval source. Dialect: Tcl.

Missing-target forward registration succeeds; invocation returns invalid-command error. Definition invokes no target; script/forward replacement and empty target registration succeed, missing prefix errors, superclass forward retains a::b. The complete PREFIX_CAPTURE script returns INITIAL EXTRA.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA 17783bdbc4ee348e3bb6f66cf39b06b4e09a0c51a735ff20e19dc62627055127; retained compile argv and SDK/header/library/build/source/stream joins. Compiler binary/version not retained.. Channel: separate actual ASCII info patchlevel/availability startup; each case uses a fresh interpreter and exact ASCII NUL-terminated Tcl_Eval or Jim_Eval source. Dialect: Tcl.

Missing-target forward registration succeeds; invocation returns invalid-command error. Definition invokes no target; script/forward replacement and empty target registration succeed, missing prefix errors, superclass forward retains a::b. The complete PREFIX_CAPTURE script errors reading value; this does not locate a registration or capture failure separately from definition-scope substitution.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA 006140dd0e751076b51dc1a91b326a6261df8b94ee94823fef3e285e23b0f35b; retained compile argv and SDK/header/library/build/source/stream joins. Compiler binary/version not retained.. Channel: separate actual ASCII info patchlevel/availability startup; each case uses a fresh interpreter and exact ASCII NUL-terminated Tcl_Eval or Jim_Eval source. Dialect: Tcl.

Missing-target forward registration succeeds; invocation returns invalid-command error. Definition invokes no target; script/forward replacement and empty target registration succeed, missing prefix errors, superclass forward retains a::b. The complete PREFIX_CAPTURE script errors reading value; this does not locate a registration or capture failure separately from definition-scope substitution.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA 10fc320da024334c6f5aa546cb2838b3c22efbd2a5550d22f36068db8e5d9858; retained compile argv and SDK/header/library/build/source/stream joins. Compiler binary/version not retained.. Channel: separate actual ASCII info patchlevel/availability startup; each case uses a fresh interpreter and exact ASCII NUL-terminated Tcl_Eval or Jim_Eval source. Dialect: Jim Tcl.

Availability is empty; no forward script case is run.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation for the exact forward scripts.

## Exact evidence

- `forward-probe.c` (input): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/probe.c](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/probe.c). SHA-256 `9426f3bb7655531c34759934ddeccf94534b3cf6d106fc229c5d35133ae7ed0b`. Retained exact input/channel/capture protocol or actual provider association; the compiler binary/version is not retained by this runner.
- `forward-capture.py` (input): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/capture.py](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/capture.py). SHA-256 `c203dace948bb8b2bd2388a1d41130164bd6fe1b84f6fa49ee39a13aac5a17d6`. Retained exact input/channel/capture protocol or actual provider association; the compiler binary/version is not retained by this runner.
- `forward-queue.json` (input): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/queue.json](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/queue.json). SHA-256 `4b75633ac8519e93e441680edc9fb50dcb392cb95be0fc6fd1eb26ee3068f4fd`. Retained exact input/channel/capture protocol or actual provider association; the compiler binary/version is not retained by this runner.
- `forward-sdk-bindings.json` (provider): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/sdk-bindings.json](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/sdk-bindings.json). SHA-256 `ce7426e316ff0d9637ee2136aacb973dab82c0125aac802760abeb6a93fd0aea`. Retained exact input/channel/capture protocol or actual provider association; the compiler binary/version is not retained by this runner.
- `forward-receipt.json` (provider): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/receipt.json). SHA-256 `f86086f4e23a8f5b5c4cd376e4f3dc69d063335ad98aefe35372a9d8941fa3f4`. Retained exact input/channel/capture protocol or actual provider association; the compiler binary/version is not retained by this runner.
- `forward-8.4.20-receipt.json` (provider): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.4.20/receipt.json). SHA-256 `fa88fee593e0f1f17f5c2cdaeb907fe7110bc53d24b1cabf59d9996f79bd6e79`. Actual compile/process/version and exact raw stream association for these finite source scripts. All reported pointer bits are zero; no pointer observation.
- `forward-8.4.20-stdout.tsv` (observation): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.4.20/stdout.tsv). SHA-256 `a1b0bac227244975692fda8bf4cfa86943bab9a838509efcf9e76bf2f9606e05`. Actual compile/process/version and exact raw stream association for these finite source scripts. All reported pointer bits are zero; no pointer observation.
- `forward-8.4.20-stderr` (observation): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual compile/process/version and exact raw stream association for these finite source scripts. All reported pointer bits are zero; no pointer observation.
- `forward-8.5.19-receipt.json` (provider): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.5.19/receipt.json). SHA-256 `df9def1890a75b6971051609546ed66c430911ba5ac3fcfbe283edc302f693fd`. Actual compile/process/version and exact raw stream association for these finite source scripts. All reported pointer bits are zero; no pointer observation.
- `forward-8.5.19-stdout.tsv` (observation): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.5.19/stdout.tsv). SHA-256 `058d3399de6dca9ff1d6a8b4e56666064bc38637ed95d0ac5acdfa126524dd88`. Actual compile/process/version and exact raw stream association for these finite source scripts. All reported pointer bits are zero; no pointer observation.
- `forward-8.5.19-stderr` (observation): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual compile/process/version and exact raw stream association for these finite source scripts. All reported pointer bits are zero; no pointer observation.
- `forward-8.6.18-receipt.json` (provider): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.6.18/receipt.json). SHA-256 `471f7ca8b6698e4683d7d394ec5d1a72047a1cb73a958c6c278e0aa3b2bbc30b`. Actual compile/process/version and exact raw stream association for these finite source scripts. All reported pointer bits are zero; no pointer observation.
- `forward-8.6.18-stdout.tsv` (observation): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.6.18/stdout.tsv). SHA-256 `5b1b0c4c210d2b7a6ac8ad81d706f522fd03183fa6d48dd81c8c14bc24572d19`. Actual compile/process/version and exact raw stream association for these finite source scripts. All reported pointer bits are zero; no pointer observation.
- `forward-8.6.18-stderr` (observation): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual compile/process/version and exact raw stream association for these finite source scripts. All reported pointer bits are zero; no pointer observation.
- `forward-9.0.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/9.0.4/receipt.json). SHA-256 `2eab30b95b98d23e7b427b1d93dbc9b5ac72f1b1b76a03cda44a6d19689f38b3`. Actual compile/process/version and exact raw stream association for these finite source scripts. All reported pointer bits are zero; no pointer observation.
- `forward-9.0.4-stdout.tsv` (observation): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/9.0.4/stdout.tsv). SHA-256 `f09798f1105c461014712126c03a6cbc8db1411ae5a09f188c4c0aade0d33dce`. Actual compile/process/version and exact raw stream association for these finite source scripts. All reported pointer bits are zero; no pointer observation.
- `forward-9.0.4-stderr` (observation): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual compile/process/version and exact raw stream association for these finite source scripts. All reported pointer bits are zero; no pointer observation.
- `forward-9.1.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/9.1.0/receipt.json). SHA-256 `24bb3c7b0ae7e7f09b28802a27352b08155a8b44de7731eec76297c45937fdd8`. Actual compile/process/version and exact raw stream association for these finite source scripts. All reported pointer bits are zero; no pointer observation.
- `forward-9.1.0-stdout.tsv` (observation): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/9.1.0/stdout.tsv). SHA-256 `d41dab420bcce062b3475d56990bed5d627184d07d0cbd5551292bb6396c8819`. Actual compile/process/version and exact raw stream association for these finite source scripts. All reported pointer bits are zero; no pointer observation.
- `forward-9.1.0-stderr` (observation): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual compile/process/version and exact raw stream association for these finite source scripts. All reported pointer bits are zero; no pointer observation.
- `forward-jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/jim/receipt.json). SHA-256 `909298914043b6fd963e5bb24a5f97f2ff667f5624f6babfe77a2fb4c59e8cfe`. Actual compile/process/version and exact raw stream association for these finite source scripts. All reported pointer bits are zero; no pointer observation.
- `forward-jim-stdout.tsv` (observation): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/jim/stdout.tsv). SHA-256 `0abca9e3d45d6d4a98e0674698d86a7c00289ff7bea962f438ac5b273f3e074b`. Actual compile/process/version and exact raw stream association for these finite source scripts. All reported pointer bits are zero; no pointer observation.
- `forward-jim-stderr` (observation): [rust/tcl-registry/tests/data/native_tcloo_forward_registration/jim/stderr](../../../../rust/tcl-registry/tests/data/native_tcloo_forward_registration/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual compile/process/version and exact raw stream association for these finite source scripts. All reported pointer bits are zero; no pointer observation.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `definer::original_forward_tests::original_forward_layout_matches_captured_registration_boundaries` (linked): Pure selected layout compares the exact availability, registration/error/deferred-invocation rows and preserves the dynamic-script scope contrast; no implementation execution result inferred.

A named test is a coverage binding, not a claim that it executed.

## Replay

Retained capture.py protocol needs independently verified SDKs and fresh output paths. Six ELF files are omitted while their original digests/compile argv remain in receipts. Compiler binary/version not retained. The native question does not certify a source transfer or Native admission.

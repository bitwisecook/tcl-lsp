# naming.regex.c-command-prefix-object-arguments

Kind: `native-observation`

## Problem statement

regsub -command carries an original List prefix member to a callback and separately creates match arguments. Reissuing the prefix from displayed text would lose object identity, while older command surfaces reject this option before callback entry.

## Question

Which original prefix member and match-object windows do the C regsub -command callbacks receive?

## Conclusion

C9.0 and C9.1 call keep twice with the same original List seed member and a string match argument whose resident bytes are initially absent; the first callback sees a stringless seed and the second sees its materialized bytes. The result is SS. C8.4–8.6 reject -command and execute no callback; their exact guest errors remain retained.

## Scope

Only the selected cases of this retained public C API probe and the recorded five library/header builds are covered. Original input object construction, reference holders, reporting conversions and callback setup are part of the probe. Private physical fields are sampled before result rendering where specified. A recorded build label is not an independent runtime version query. No new native launch, Rust test result, Jim or BIG-IP behavior is claimed; manually invalidated objects and aborted attempts provide no successful adapter or language completion.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20 (recorded build label; runtime version query not retained). Build: Library SHA256 d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; probe executable SHA256 15923b6f2dec95f3337706dd373128d37f33059aac7a13f0637e49934a55996b; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 8.4.20; compile exit 0. Selected original capture windows: [{"case":5,"exit":0,"stdout":"result\t1\tstring\t1\t1\t6261642073776974636820222d636f6d6d616e64223a206d757374206265202d616c6c2c202d6e6f636173652c202d657870616e6465642c202d6c696e652c202d6c696e6573746f702c202d6c696e65616e63686f722c202d73746172742c206f72202d2d\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.5

Status: `unsupported`. Version: 8.5.19 (recorded build label; runtime version query not retained). Build: Library SHA256 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; probe executable SHA256 808e26d6f6d9cfc40bd5fbbe16f5c2498cf1868f0aa65c39d21894e24416dcfb; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 8.5.19; compile exit 0. Selected original capture windows: [{"case":5,"exit":0,"stdout":"result\t1\tstring\t1\t1\t6261642073776974636820222d636f6d6d616e64223a206d757374206265202d616c6c2c202d6e6f636173652c202d657870616e6465642c202d6c696e652c202d6c696e6573746f702c202d6c696e65616e63686f722c202d73746172742c206f72202d2d\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.6

Status: `unsupported`. Version: 8.6.18 (recorded build label; runtime version query not retained). Build: Library SHA256 a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; probe executable SHA256 9cc3a8be42775066c0660d41a5134c681eadd60290fd54bd53149bb786a38393; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 8.6.18; compile exit 0. Selected original capture windows: [{"case":5,"exit":0,"stdout":"result\t1\tstring\t1\t1\t626164206f7074696f6e20222d636f6d6d616e64223a206d757374206265202d616c6c2c202d6e6f636173652c202d657870616e6465642c202d6c696e652c202d6c696e6573746f702c202d6c696e65616e63686f722c202d73746172742c206f72202d2d\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.0

Status: `observed`. Version: 9.0.4 (recorded build label; runtime version query not retained). Build: Library SHA256 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; probe executable SHA256 88957a4c70cba565fc8533e5ada50be50597d6a8882d0fcee8263acb16c8f8eb; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 9.0.4; compile exit 0. Selected original capture windows: [{"case":5,"exit":0,"stdout":"callback\t1\t1\tlist\t0\t1\tstring\t0\ncallback\t1\t1\tlist\t1\t1\tstring\t0\nresult\t0\tstring\t0\t1\t5353\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.1

Status: `observed`. Version: 9.1.0 (recorded build label; runtime version query not retained). Build: Library SHA256 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; probe executable SHA256 57e7d7744b930c4feec94ea34a290993d32074fb4f94f482b6ebf79ed88c4893; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 9.1.0; compile exit 0. Selected original capture windows: [{"case":5,"exit":0,"stdout":"callback\t1\t1\tlist\t0\t1\tstring\t0\ncallback\t1\t1\tlist\t1\t1\tstring\t0\nresult\t0\tstring\t0\t1\t5353\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

## Exact evidence

- `original-capture` (provider): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). SHA-256 `50248809f0a9c83a28dee996973692e561adfa8dcb4216df613c95150eb29ed3`. Immutable build/header/library/executable identities and complete case statuses/streams.
- `original-probe` (input): [runtime/rust/tests/data/native_regex_original/probe.c](../../../../runtime/rust/tests/data/native_regex_original/probe.c). SHA-256 `d53afd2143239af6cc9fc8d3d166d7d7054f748e7e614c2c60bcbe5c3ae7c336`. Exact retained original object constructors, Tcl_EvalEx/Tcl_EvalObjv or RegExp API calls, reporting order and case selector.
- `tcl8.4-case-5` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). SHA-256 `50248809f0a9c83a28dee996973692e561adfa8dcb4216df613c95150eb29ed3`. JSON pointer `/0/captures/5`. Exact case 5 process status, raw stdout and raw stderr.
- `tcl8.5-case-5` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). SHA-256 `50248809f0a9c83a28dee996973692e561adfa8dcb4216df613c95150eb29ed3`. JSON pointer `/1/captures/5`. Exact case 5 process status, raw stdout and raw stderr.
- `tcl8.6-case-5` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). SHA-256 `50248809f0a9c83a28dee996973692e561adfa8dcb4216df613c95150eb29ed3`. JSON pointer `/2/captures/5`. Exact case 5 process status, raw stdout and raw stderr.
- `tcl9.0-case-5` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). SHA-256 `50248809f0a9c83a28dee996973692e561adfa8dcb4216df613c95150eb29ed3`. JSON pointer `/3/captures/5`. Exact case 5 process status, raw stdout and raw stderr.
- `tcl9.1-case-5` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). SHA-256 `50248809f0a9c83a28dee996973692e561adfa8dcb4216df613c95150eb29ed3`. JSON pointer `/4/captures/5`. Exact case 5 process status, raw stdout and raw stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `regexp_cmd`: Independent current regex consumer; the retained native windows do not certify a Rust implementation result.
- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `cmd_regex::tests::original_regsub_prefix_and_unicode_arguments_match_native_c9_controls` (linked): Checks its scoped original input/object/result windows against retained native controls. Adapter withdrawal after an aborted native attempt is a separate Rust contract; no executed Rust result is attached.

A named test is a coverage binding, not a claim that it executed.

## Replay

No replay runner or new execution is asserted. Rebuild the retained probe only with independently verified matching source/header/library identities, run each selected case in a fresh process and preserve stdout, stderr and process status separately. Original abnormal exits are retained observations of incomplete attempts; they are not successful completions and are not requested to be rerun.

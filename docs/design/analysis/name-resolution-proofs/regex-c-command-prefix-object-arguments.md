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

Status: `unsupported`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl8.4 8.4.20 (recorded build label; runtime version query not retained).

Recorded build label 8.4.20; compile exit 0. Selected original capture windows: [{"case":5,"exit":0,"stdout":"result\t1\tstring\t1\t1\t6261642073776974636820222d636f6d6d616e64223a206d757374206265202d616c6c2c202d6e6f636173652c202d657870616e6465642c202d6c696e652c202d6c696e6573746f702c202d6c696e65616e63686f722c202d73746172742c206f72202d2d\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.5

Status: `unsupported`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl8.5 8.5.19 (recorded build label; runtime version query not retained).

Recorded build label 8.5.19; compile exit 0. Selected original capture windows: [{"case":5,"exit":0,"stdout":"result\t1\tstring\t1\t1\t6261642073776974636820222d636f6d6d616e64223a206d757374206265202d616c6c2c202d6e6f636173652c202d657870616e6465642c202d6c696e652c202d6c696e6573746f702c202d6c696e65616e63686f722c202d73746172742c206f72202d2d\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.6

Status: `unsupported`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl8.6 8.6.18 (recorded build label; runtime version query not retained).

Recorded build label 8.6.18; compile exit 0. Selected original capture windows: [{"case":5,"exit":0,"stdout":"result\t1\tstring\t1\t1\t626164206f7074696f6e20222d636f6d6d616e64223a206d757374206265202d616c6c2c202d6e6f636173652c202d657870616e6465642c202d6c696e652c202d6c696e6573746f702c202d6c696e65616e63686f722c202d73746172742c206f72202d2d\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.0

Status: `observed`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl9.0 9.0.4 (recorded build label; runtime version query not retained).

Recorded build label 9.0.4; compile exit 0. Selected original capture windows: [{"case":5,"exit":0,"stdout":"callback\t1\t1\tlist\t0\t1\tstring\t0\ncallback\t1\t1\tlist\t1\t1\tstring\t0\nresult\t0\tstring\t0\t1\t5353\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.1

Status: `observed`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl9.1 9.1.0 (recorded build label; runtime version query not retained).

Recorded build label 9.1.0; compile exit 0. Selected original capture windows: [{"case":5,"exit":0,"stdout":"callback\t1\t1\tlist\t0\t1\tstring\t0\ncallback\t1\t1\tlist\t1\t1\tstring\t0\nresult\t0\tstring\t0\t1\t5353\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### jim, bigip

Status: `not-tested`.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

## Consumer bindings

- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `regexp_cmd`: Independent current regex consumer; the retained native windows do not certify a Rust implementation result.
- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `cmd_regex::tests::original_regsub_prefix_and_unicode_arguments_match_native_c9_controls` (linked): Checks its scoped original input/object/result windows against retained native controls. Adapter withdrawal after an aborted native attempt is a separate Rust contract; no executed Rust result is attached.

## Evidence

- [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json)
- [runtime/rust/tests/data/native_regex_original/probe.c](../../../../runtime/rust/tests/data/native_regex_original/probe.c)

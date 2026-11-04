# naming.regex.c-match-capture-store-frontiers

Kind: `native-observation`

## Problem statement

regexp -all writes a capture variable for each match. A trace can fail on the second write, leaving earlier capture/submatch state; treating the command as a single transactional write loses that frontier. The oldest failing-trace attempt also aborts instead of returning a caught result.

## Question

What per-match write values and first-failure result does the original regexp -all trace control retain?

## Conclusion

Every normal case records the capture write sequence a, b, c and final capture/submatch c. The failing trace reaches a then b on C8.5–9.1 and leaves the preceding submatch a, with release-specific error text/code. The C8.4 failing-trace process aborts with free(): invalid pointer and no result output; it supplies no successful completion.

## Scope

Only the selected cases of this retained public C API probe and the recorded five library/header builds are covered. Original input object construction, reference holders, reporting conversions and callback setup are part of the probe. Private physical fields are sampled before result rendering where specified. A recorded build label is not an independent runtime version query. No new native launch, Rust test result, Jim or BIG-IP behavior is claimed; manually invalidated objects and aborted attempts provide no successful adapter or language completion.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 8.4.20; compile exit 0. Selected original capture windows: [{"case":0,"exit":0,"stdout":"result\t0\tlist\t0\t1\t7b61206220637d20632063\n","stderr":""},{"case":1,"exit":-6,"stdout":"","stderr":"free(): invalid pointer\n"}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.5

Status: `observed`. Version: 8.5.19 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 8.5.19; compile exit 0. Selected original capture windows: [{"case":0,"exit":0,"stdout":"result\t0\tlist\t0\t1\t7b61206220637d20632063\n","stderr":""},{"case":1,"exit":0,"stdout":"result\t0\tlist\t0\t1\t31207b6120627d207b636f756c646e277420736574207661726961626c6520226d227d2061204e4f4e45\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.6

Status: `observed`. Version: 8.6.18 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 8.6.18; compile exit 0. Selected original capture windows: [{"case":0,"exit":0,"stdout":"result\t0\tlist\t0\t1\t7b61206220637d20632063\n","stderr":""},{"case":1,"exit":0,"stdout":"result\t0\tlist\t0\t1\t31207b6120627d207b63616e27742073657420226d223a2053544f507d2061207b54434c205752495445205641524e414d457d\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.0

Status: `observed`. Version: 9.0.4 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 9.0.4; compile exit 0. Selected original capture windows: [{"case":0,"exit":0,"stdout":"result\t0\tlist\t0\t1\t7b61206220637d20632063\n","stderr":""},{"case":1,"exit":0,"stdout":"result\t0\tlist\t0\t1\t31207b6120627d207b63616e27742073657420226d223a2053544f507d2061207b54434c205752495445205641524e414d457d\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.1

Status: `observed`. Version: 9.1.0 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 9.1.0; compile exit 0. Selected original capture windows: [{"case":0,"exit":0,"stdout":"result\t0\tlist\t0\t1\t7b61206220637d20632063\n","stderr":""},{"case":1,"exit":0,"stdout":"result\t0\tlist\t0\t1\t31207b6120627d207b63616e27742073657420226d223a2053544f507d2061207b54434c205752495445205641524e414d457d\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

## Historical references

- `original-capture` (provider): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). Immutable build/header/library/executable identities and complete case statuses/streams.
- `original-probe` (input): [runtime/rust/tests/data/native_regex_original/probe.c](../../../../runtime/rust/tests/data/native_regex_original/probe.c). Exact retained original object constructors, Tcl_EvalEx/Tcl_EvalObjv or RegExp API calls, reporting order and case selector.
- `tcl8.4-case-0` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/0/captures/0`. Exact case 0 process status, raw stdout and raw stderr.
- `tcl8.4-case-1` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/0/captures/1`. Exact case 1 process status, raw stdout and raw stderr.
- `tcl8.5-case-0` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/1/captures/0`. Exact case 0 process status, raw stdout and raw stderr.
- `tcl8.5-case-1` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/1/captures/1`. Exact case 1 process status, raw stdout and raw stderr.
- `tcl8.6-case-0` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/2/captures/0`. Exact case 0 process status, raw stdout and raw stderr.
- `tcl8.6-case-1` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/2/captures/1`. Exact case 1 process status, raw stdout and raw stderr.
- `tcl9.0-case-0` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/3/captures/0`. Exact case 0 process status, raw stdout and raw stderr.
- `tcl9.0-case-1` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/3/captures/1`. Exact case 1 process status, raw stdout and raw stderr.
- `tcl9.1-case-0` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/4/captures/0`. Exact case 0 process status, raw stdout and raw stderr.
- `tcl9.1-case-1` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/4/captures/1`. Exact case 1 process status, raw stdout and raw stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `regexp_cmd`: Independent current regex consumer; the retained native windows do not certify a Rust implementation result.
- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `cmd_regex::tests::original_regexp_each_match_and_first_failure_match_native_controls` (linked): Checks its scoped original input/object/result windows against retained native controls. Adapter withdrawal after an aborted native attempt is a separate Rust contract; no executed Rust result is attached.

A named test is a coverage binding, not a claim that it executed.

## Replay

No replay runner or new execution is asserted. Rebuild the retained probe only with independently verified matching source/header/library identities, run each selected case in a fresh process and preserve stdout, stderr and process status separately. Original abnormal exits are retained observations of incomplete attempts; they are not successful completions and are not requested to be rerun.

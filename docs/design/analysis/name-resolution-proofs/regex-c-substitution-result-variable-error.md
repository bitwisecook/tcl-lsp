# naming.regex.c-substitution-result-variable-error

Kind: `native-observation`

## Problem statement

A regsub result destination a(k) can fail because a is scalar. A catch-wrapped call and a direct call expose different surrounding result owners, and older releases publish different diagnostic primaries and error codes.

## Question

What completion, original result primary and diagnostic bytes does regsub retain when its result destination indexes a scalar?

## Conclusion

All five recorded builds reject the scalar a(k) result destination. The caught control returns a list describing the failure; direct regsub returns error. The C8.4 direct result is untyped, whereas the later direct results have string primary, and detailed diagnostic/error-code bytes differ by release. No failed-set rollback or arbitrary variable state is inferred.

## Scope

Only the selected cases of this retained public C API probe and the recorded five library/header builds are covered. Original input object construction, reference holders, reporting conversions and callback setup are part of the probe. Private physical fields are sampled before result rendering where specified. A recorded build label is not an independent runtime version query. No new native launch, Rust test result, Jim or BIG-IP behavior is claimed; manually invalidated objects and aborted attempts provide no successful adapter or language completion.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 8.4.20; compile exit 0. Selected original capture windows: [{"case":2,"exit":0,"stdout":"result\t0\tlist\t0\t1\t31207b636f756c646e277420736574207661726961626c65202261286b29227d204e4f4e45\n","stderr":""},{"case":4,"exit":0,"stdout":"result\t1\tnone\t1\t1\t636f756c646e277420736574207661726961626c65202261286b2922\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.5

Status: `observed`. Version: 8.5.19 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 8.5.19; compile exit 0. Selected original capture windows: [{"case":2,"exit":0,"stdout":"result\t0\tlist\t0\t1\t31207b636f756c646e277420736574207661726961626c65202261286b29227d204e4f4e45\n","stderr":""},{"case":4,"exit":0,"stdout":"result\t1\tstring\t1\t1\t636f756c646e277420736574207661726961626c65202261286b2922\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.6

Status: `observed`. Version: 8.6.18 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 8.6.18; compile exit 0. Selected original capture windows: [{"case":2,"exit":0,"stdout":"result\t0\tlist\t0\t1\t31207b63616e277420736574202261286b29223a207661726961626c652069736e27742061727261797d207b54434c204c4f4f4b5550205641524e414d4520617d\n","stderr":""},{"case":4,"exit":0,"stdout":"result\t1\tstring\t1\t1\t63616e277420736574202261286b29223a207661726961626c652069736e2774206172726179\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.0

Status: `observed`. Version: 9.0.4 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 9.0.4; compile exit 0. Selected original capture windows: [{"case":2,"exit":0,"stdout":"result\t0\tlist\t0\t1\t31207b63616e277420736574202261286b29223a207661726961626c652069736e27742061727261797d207b54434c204c4f4f4b5550205641524e414d4520617d\n","stderr":""},{"case":4,"exit":0,"stdout":"result\t1\tstring\t1\t1\t63616e277420736574202261286b29223a207661726961626c652069736e2774206172726179\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.1

Status: `observed`. Version: 9.1.0 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 9.1.0; compile exit 0. Selected original capture windows: [{"case":2,"exit":0,"stdout":"result\t0\tlist\t0\t1\t31207b63616e277420736574202261286b29223a207661726961626c652069736e27742061727261797d207b54434c204c4f4f4b5550205641524e414d4520617d\n","stderr":""},{"case":4,"exit":0,"stdout":"result\t1\tstring\t1\t1\t63616e277420736574202261286b29223a207661726961626c652069736e2774206172726179\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

## Historical references

- `original-capture` (provider): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). Immutable build/header/library/executable identities and complete case statuses/streams.
- `original-probe` (input): [runtime/rust/tests/data/native_regex_original/probe.c](../../../../runtime/rust/tests/data/native_regex_original/probe.c). Exact retained original object constructors, Tcl_EvalEx/Tcl_EvalObjv or RegExp API calls, reporting order and case selector.
- `tcl8.4-case-2` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/0/captures/2`. Exact case 2 process status, raw stdout and raw stderr.
- `tcl8.4-case-4` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/0/captures/4`. Exact case 4 process status, raw stdout and raw stderr.
- `tcl8.5-case-2` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/1/captures/2`. Exact case 2 process status, raw stdout and raw stderr.
- `tcl8.5-case-4` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/1/captures/4`. Exact case 4 process status, raw stdout and raw stderr.
- `tcl8.6-case-2` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/2/captures/2`. Exact case 2 process status, raw stdout and raw stderr.
- `tcl8.6-case-4` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/2/captures/4`. Exact case 4 process status, raw stdout and raw stderr.
- `tcl9.0-case-2` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/3/captures/2`. Exact case 2 process status, raw stdout and raw stderr.
- `tcl9.0-case-4` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/3/captures/4`. Exact case 4 process status, raw stdout and raw stderr.
- `tcl9.1-case-2` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/4/captures/2`. Exact case 2 process status, raw stdout and raw stderr.
- `tcl9.1-case-4` (observation): [runtime/rust/tests/data/native_regex_original/manifest.json](../../../../runtime/rust/tests/data/native_regex_original/manifest.json). JSON pointer `/4/captures/4`. Exact case 4 process status, raw stdout and raw stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `regexp_cmd`: Independent current regex consumer; the retained native windows do not certify a Rust implementation result.
- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `cmd_regex::tests::original_regex_unmatched_target_and_quiet_write_match_native_controls` (linked): Checks its scoped original input/object/result windows against retained native controls. Adapter withdrawal after an aborted native attempt is a separate Rust contract; no executed Rust result is attached.

A named test is a coverage binding, not a claim that it executed.

## Replay

No replay runner or new execution is asserted. Rebuild the retained probe only with independently verified matching source/header/library identities, run each selected case in a fresh process and preserve stdout, stderr and process status separately. Original abnormal exits are retained observations of incomplete attempts; they are not successful completions and are not requested to be rerun.

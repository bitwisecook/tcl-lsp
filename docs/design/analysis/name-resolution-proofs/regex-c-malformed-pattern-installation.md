# naming.regex.c-malformed-pattern-installation

Kind: `native-observation`

## Problem statement

A failed compile of an original string pattern must not be treated as a valid cached RegExp merely because a getter was attempted. Result errors and installed pattern state are independent.

## Question

Does compiling the original malformed pattern ( install a C RegExp primary?

## Conclusion

Every recorded build reports compile false and leaves the original pattern untyped with resident bytes and reference count one. The probe does not print detailed compilation error bytes; none are inferred.

## Scope

Only the selected cases of this retained public C API probe and the recorded five library/header builds are covered. Original input object construction, reference holders, reporting conversions and callback setup are part of the probe. Private physical fields are sampled before result rendering where specified. A recorded build label is not an independent runtime version query. No new native launch, Rust test result, Jim or BIG-IP behavior is claimed; manually invalidated objects and aborted attempts provide no successful adapter or language completion.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 8.4.20; compile exit 0. Selected original capture windows: [{"case":2,"exit":0,"stdout":"pattern\tnone\t1\t1\ncompile\t0\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.5

Status: `observed`. Version: 8.5.19 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 8.5.19; compile exit 0. Selected original capture windows: [{"case":2,"exit":0,"stdout":"pattern\tnone\t1\t1\ncompile\t0\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.6

Status: `observed`. Version: 8.6.18 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 8.6.18; compile exit 0. Selected original capture windows: [{"case":2,"exit":0,"stdout":"pattern\tnone\t1\t1\ncompile\t0\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.0

Status: `observed`. Version: 9.0.4 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 9.0.4; compile exit 0. Selected original capture windows: [{"case":2,"exit":0,"stdout":"pattern\tnone\t1\t1\ncompile\t0\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.1

Status: `observed`. Version: 9.1.0 (recorded build label; runtime version query not retained). Build: Library; probe executable; exact header digests in original-capture.. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case.. Dialect: C Tcl.

Recorded build label 9.1.0; compile exit 0. Selected original capture windows: [{"case":2,"exit":0,"stdout":"pattern\tnone\t1\t1\ncompile\t0\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

## Historical references

- `original-capture` (provider): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). Immutable build/header/library/executable identities and complete case statuses/streams.
- `original-probe` (input): [runtime/rust/tests/data/native_regexp_cache/probe.c](../../../../runtime/rust/tests/data/native_regexp_cache/probe.c). Exact retained original object constructors, Tcl_EvalEx/Tcl_EvalObjv or RegExp API calls, reporting order and case selector.
- `tcl8.4-case-2` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/0/captures/2`. Exact case 2 process status, raw stdout and raw stderr.
- `tcl8.5-case-2` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/1/captures/2`. Exact case 2 process status, raw stdout and raw stderr.
- `tcl8.6-case-2` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/2/captures/2`. Exact case 2 process status, raw stdout and raw stderr.
- `tcl9.0-case-2` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/3/captures/2`. Exact case 2 process status, raw stdout and raw stderr.
- `tcl9.1-case-2` (observation): [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json). JSON pointer `/4/captures/2`. Exact case 2 process status, raw stdout and raw stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `regsub_cmd`: Independent current regex consumer; the retained native windows do not certify a Rust implementation result.

A named test is a coverage binding, not a claim that it executed.

## Replay

No replay runner or new execution is asserted. Rebuild the retained probe only with independently verified matching source/header/library identities, run each selected case in a fresh process and preserve stdout, stderr and process status separately. Original abnormal exits are retained observations of incomplete attempts; they are not successful completions and are not requested to be rerun.

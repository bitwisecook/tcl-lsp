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

Status: `observed`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl8.4 8.4.20 (recorded build label; runtime version query not retained).

Recorded build label 8.4.20; compile exit 0. Selected original capture windows: [{"case":2,"exit":0,"stdout":"pattern\tnone\t1\t1\ncompile\t0\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.5

Status: `observed`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl8.5 8.5.19 (recorded build label; runtime version query not retained).

Recorded build label 8.5.19; compile exit 0. Selected original capture windows: [{"case":2,"exit":0,"stdout":"pattern\tnone\t1\t1\ncompile\t0\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl8.6

Status: `observed`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl8.6 8.6.18 (recorded build label; runtime version query not retained).

Recorded build label 8.6.18; compile exit 0. Selected original capture windows: [{"case":2,"exit":0,"stdout":"pattern\tnone\t1\t1\ncompile\t0\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.0

Status: `observed`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl9.0 9.0.4 (recorded build label; runtime version query not retained).

Recorded build label 9.0.4; compile exit 0. Selected original capture windows: [{"case":2,"exit":0,"stdout":"pattern\tnone\t1\t1\ncompile\t0\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### tcl9.1

Status: `observed`. Channel: Original public C API objects or counted Tcl_EvalEx source, selected by the attached probe case. Version: tcl9.1 9.1.0 (recorded build label; runtime version query not retained).

Recorded build label 9.1.0; compile exit 0. Selected original capture windows: [{"case":2,"exit":0,"stdout":"pattern\tnone\t1\t1\ncompile\t0\n","stderr":""}]. Abnormal process statuses establish incomplete attempts, even if earlier output is present.

### jim, bigip

Status: `not-tested`.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

## Consumer bindings

- [runtime/rust/src/cmd_regex.rs](../../../../runtime/rust/src/cmd_regex.rs), `regsub_cmd`: Independent current regex consumer; the retained native windows do not certify a Rust implementation result.

## Evidence

- [runtime/rust/tests/data/native_regexp_cache/manifest.json](../../../../runtime/rust/tests/data/native_regexp_cache/manifest.json)
- [runtime/rust/tests/data/native_regexp_cache/probe.c](../../../../runtime/rust/tests/data/native_regexp_cache/probe.c)

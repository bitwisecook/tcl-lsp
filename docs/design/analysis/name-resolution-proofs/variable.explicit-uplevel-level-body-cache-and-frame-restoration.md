# naming.variable.explicit-uplevel-level-body-cache-and-frame-restoration

Kind: `native-observation`

## Problem statement

Explicit level objects can acquire a frame cache, contain raw zero suffixes, or fail to parse, while a selected body can fail/parse-error/break or materialise a list string. A final result alone cannot establish the selected frame, cache route, or restoration after failure.

## Question

For fourteen original level/body pairs, which frame/cache/body-string transitions and completion bytes are observed, and is the saved variable frame restored?

## Conclusion

All seventy retained C windows restore the saved frame and produce the listed original level/cache/body/completion results. C84 uses its selected string TclGetFrame route; C85+ use original-object TclObjGetFrame and can retain levelReference caches. This is explicit evaluation, not a native uplevel opcode proof or a Jim/BIG-IP observation.

## Scope

C84.20/C85.19/C86.18/C90.4/C91.0 private interpreter probes: fourteen original objects per release, including raw 00 suffix level objects, root/current/relative/default nonlevel/error selection and error/parse/list/break bodies. Each window uses a fresh interpreter. Returned result bytes and before/after cache/resident-string flags are distinct axes.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Version: tcl8.4 8.4.20.

Rows use original ordinal: 0 #0, 1 0, 2 1, 3 bad, 4 +0, 5 99, 6 -1, 7 1.0, 8 counted 1\x00X, 9 counted #0\x00X; 10 error BODY, 11 parse error, 12 pure list script, 13 break.

| ordinal | before type | before string | code | result | after type | after level string | after body string | restored |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 1 | none | 1 | 0 | "1 1" | none | 1 | 1 | 1 |
| 2 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 3 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 4 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 5 | none | 1 | 1 | "bad level \"99\"" | none | 1 | 1 | 1 |
| 6 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 7 | none | 1 | 1 | "expected integer but got \"1.0\"" | none | 1 | 1 | 1 |
| 8 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 9 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 10 | none | 1 | 1 | "BODY" | none | 1 | 1 | 1 |
| 11 | none | 1 | 1 | "missing close-brace" | none | 1 | 1 | 1 |
| 12 | none | 1 | 0 | "VALUE" | none | 1 | 1 | 1 |
| 13 | none | 1 | 1 | "invoked \"break\" outside of a loop" | none | 1 | 1 | 1 |

### tcl8.5

Status: `observed`. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Version: tcl8.5 8.5.19.

Rows use original ordinal: 0 #0, 1 0, 2 1, 3 bad, 4 +0, 5 99, 6 -1, 7 1.0, 8 counted 1\x00X, 9 counted #0\x00X; 10 error BODY, 11 parse error, 12 pure list script, 13 break.

| ordinal | before type | before string | code | result | after type | after level string | after body string | restored |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | none | 1 | 0 | "0 0" | levelReference | 1 | 1 | 1 |
| 1 | none | 1 | 0 | "1 1" | levelReference | 1 | 1 | 1 |
| 2 | none | 1 | 0 | "0 0" | levelReference | 1 | 1 | 1 |
| 3 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 4 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 5 | none | 1 | 1 | "bad level \"99\"" | levelReference | 1 | 1 | 1 |
| 6 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 7 | none | 1 | 1 | "expected integer but got \"1.0\"" | none | 1 | 1 | 1 |
| 8 | none | 1 | 0 | "0 0" | levelReference | 1 | 1 | 1 |
| 9 | none | 1 | 0 | "0 0" | levelReference | 1 | 1 | 1 |
| 10 | none | 1 | 1 | "BODY" | levelReference | 1 | 1 | 1 |
| 11 | none | 1 | 1 | "missing close-brace" | levelReference | 1 | 1 | 1 |
| 12 | none | 1 | 0 | "VALUE" | levelReference | 1 | 0 | 1 |
| 13 | none | 1 | 1 | "invoked \"break\" outside of a loop" | levelReference | 1 | 1 | 1 |

### tcl8.6

Status: `observed`. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Version: tcl8.6 8.6.18.

Rows use original ordinal: 0 #0, 1 0, 2 1, 3 bad, 4 +0, 5 99, 6 -1, 7 1.0, 8 counted 1\x00X, 9 counted #0\x00X; 10 error BODY, 11 parse error, 12 pure list script, 13 break.

| ordinal | before type | before string | code | result | after type | after level string | after body string | restored |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | none | 1 | 0 | "0 0" | levelReference | 1 | 1 | 1 |
| 1 | none | 1 | 0 | "1 1" | int | 1 | 1 | 1 |
| 2 | none | 1 | 0 | "0 0" | int | 1 | 1 | 1 |
| 3 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 4 | none | 1 | 0 | "1 1" | int | 1 | 1 | 1 |
| 5 | none | 1 | 1 | "bad level \"99\"" | int | 1 | 1 | 1 |
| 6 | none | 1 | 0 | "0 0" | int | 1 | 1 | 1 |
| 7 | none | 1 | 1 | "bad level \"1.0\"" | double | 1 | 1 | 1 |
| 8 | none | 1 | 0 | "0 0" | int | 1 | 1 | 1 |
| 9 | none | 1 | 0 | "0 0" | levelReference | 1 | 1 | 1 |
| 10 | none | 1 | 1 | "BODY" | levelReference | 1 | 1 | 1 |
| 11 | none | 1 | 1 | "missing close-brace" | levelReference | 1 | 1 | 1 |
| 12 | none | 1 | 0 | "VALUE" | levelReference | 1 | 0 | 1 |
| 13 | none | 1 | 1 | "invoked \"break\" outside of a loop" | levelReference | 1 | 1 | 1 |

### tcl9.0, tcl9.1

Status: `observed`. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Rows use original ordinal: 0 #0, 1 0, 2 1, 3 bad, 4 +0, 5 99, 6 -1, 7 1.0, 8 counted 1\x00X, 9 counted #0\x00X; 10 error BODY, 11 parse error, 12 pure list script, 13 break.

| ordinal | before type | before string | code | result | after type | after level string | after body string | restored |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | none | 1 | 0 | "0 0" | levelReference | 1 | 1 | 1 |
| 1 | none | 1 | 0 | "1 1" | int | 1 | 1 | 1 |
| 2 | none | 1 | 0 | "0 0" | int | 1 | 1 | 1 |
| 3 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 4 | none | 1 | 0 | "1 1" | int | 1 | 1 | 1 |
| 5 | none | 1 | 1 | "bad level \"99\"" | int | 1 | 1 | 1 |
| 6 | none | 1 | 1 | "bad level \"-1\"" | int | 1 | 1 | 1 |
| 7 | none | 1 | 0 | "0 0" | none | 1 | 1 | 1 |
| 8 | none | 1 | 0 | "0 0" | int | 1 | 1 | 1 |
| 9 | none | 1 | 0 | "0 0" | levelReference | 1 | 1 | 1 |
| 10 | none | 1 | 1 | "BODY" | levelReference | 1 | 1 | 1 |
| 11 | none | 1 | 1 | "missing close-brace" | levelReference | 1 | 1 | 1 |
| 12 | none | 1 | 0 | "VALUE" | levelReference | 1 | 0 | 1 |
| 13 | none | 1 | 1 | "invoked \"break\" outside of a loop" | levelReference | 1 | 1 | 1 |

### jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-vm/src/exec/native_uplevel_tests.rs](../../../../rust/tcl-vm/src/exec/native_uplevel_tests.rs), `explicit_uplevel_preserves_seventy_native_level_body_and_restore_windows`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-vm/src/exec/native_uplevel_tests.rs](../../../../rust/tcl-vm/src/exec/native_uplevel_tests.rs), `exec::native_uplevel_tests::explicit_uplevel_preserves_seventy_native_level_body_and_restore_windows` (linked): Compares the seventy exact original level/cache/body result windows and requires frame restoration, independently of portable uplevel opcode selection.

## Evidence

- [rust/tcl-vm/tests/data/native_explicit_uplevel/manifest.json](../../../../rust/tcl-vm/tests/data/native_explicit_uplevel/manifest.json)
- [rust/tcl-vm/tests/data/native_explicit_uplevel/probe.c](../../../../rust/tcl-vm/tests/data/native_explicit_uplevel/probe.c)
- [rust/tcl-vm/tests/data/native_explicit_uplevel/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_explicit_uplevel/8.4.20.tsv)
- [rust/tcl-vm/tests/data/native_explicit_uplevel/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_explicit_uplevel/8.5.19.tsv)
- [rust/tcl-vm/tests/data/native_explicit_uplevel/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_explicit_uplevel/8.6.18.tsv)
- [rust/tcl-vm/tests/data/native_explicit_uplevel/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_explicit_uplevel/9.0.4.tsv)
- [rust/tcl-vm/tests/data/native_explicit_uplevel/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_explicit_uplevel/9.1.0.tsv)

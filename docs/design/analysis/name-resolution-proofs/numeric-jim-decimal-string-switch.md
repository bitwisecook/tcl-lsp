# naming.numeric.jim-decimal-string-switch

Kind: `native-observation`

## Problem statement

A switch completion-name fallback parses a decimal CString independently from object numeric caches, expression radix grammar and ambient errno. Using a general numeric parser would accept the wrong radix and impose unmeasured overflow rules.

## Question

What does the original Jim_StringToWide base10 stage return for sixteen inputs with errno0 versus preseeded ERANGE?

## Conclusion

Both errno seeds produce identical rows. Decimal 010 returns10; 1 followed by raw zero returns1; 1.0, 0x10, empty and bad reject. The captured positive over-range values wrap to signed results and report success, including the tested larger-than-u64 spelling. This is the measured CString conversion stage, not complete Jim numeric-object grammar or a safe mathematical integer range.

## Scope

Original Jim_StringToWide(...,10) calls with sixteen C strings, including raw-zero suffix, repeated under errno0/ERANGE. No original object primary or interpreter result/error state is sampled. One32-row Jim capture has probe/header/library/executable hashes and run0; original stdout hash, compile status, runtime patchlevel/revision and full flags are not recorded. C Tcl and BIG-IP are not tested.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

No native observation of this exact question is attached for this provider.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

No native observation of this exact question is attached for this provider.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

No native observation of this exact question is attached for this provider.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

No native observation of this exact question is attached for this provider.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

No native observation of this exact question is attached for this provider.

### jim

Status: `observed`. Version: not queried (capture association; launched patchlevel/revision unqueried). Build: binary_sha256=8f4051cdd357af5753f610f448279f45b64b93a42d41628035d280565d5e0e3e; library_sha256=75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; header_sha256=d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Direct Jim_StringToWide CString conversion with explicit base10 and errno seed; no interpreter evaluator.. Dialect: Jim Tcl.

Exact relevant original rows:

```text
0	0	0	0
1	0	0	1
2	0	0	-1
3	0	0	1
4	0	0	1
5	0	1	1
6	0	1	0
7	0	0	10
8	0	0	9223372036854775807
9	0	0	-9223372036854775808
10	0	0	-1
11	0	0	-1
12	0	0	-9223372036854775808
13	0	0	1
14	0	1	0
15	0	1	0
0	1	0	0
1	1	0	1
2	1	0	-1
3	1	0	1
4	1	0	1
5	1	1	1
6	1	1	0
7	1	0	10
8	1	0	9223372036854775807
9	1	0	-9223372036854775808
10	1	0	-1
11	1	0	-1
12	1	0	-9223372036854775808
13	1	0	1
14	1	1	0
15	1	1	0
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No native observation of this exact question is attached for this provider.

## Exact evidence

- `probe-0` (input): [rust/tcl-syntax/tests/data/native_scalar_getters/decimal-switch/probe.c](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/decimal-switch/probe.c). SHA-256 `40d46d5a6becb34ef483c8d29a49abe0e74408880d72532b7bf5ebe6b7db2393`. Exact retained input constructors, selected APIs, callback and before/after observers.
- `receipt-0` (provider): [rust/tcl-syntax/tests/data/native_scalar_getters/decimal-switch/manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/decimal-switch/manifest.json). SHA-256 `97b25f18972514555ff9e9af486c2a7ba42691a2c5c52af8c4efdda9cde00002`. Original release/build/status association. Only fields actually recorded are claimed; reconstructed current checksums do not create an absent original full stdout digest.
- `rows-jim` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/decimal-switch/jim0.84.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/decimal-switch/jim0.84.txt). SHA-256 `13ea60dc1dc2aca746dfd5a86b22f061928fcfe2c3027deb52fb9ee7f6ff82a2`. Original retained full stream; provider answer selects only this question's relevant windows.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/scalar_getter.rs](../../../../rust/tcl-syntax/src/scalar_getter.rs), `NativeScalarGetterProtocol::jim_decimal_wide_probe`: Owns this decimal string-only stage independently from primitive cache and expression numeral purposes.
- [rust/tcl-syntax/src/scalar_getter/tests.rs](../../../../rust/tcl-syntax/src/scalar_getter/tests.rs), `scalar_getter::tests::jim_decimal_string_switch_keeps_its_own_native_stage` (linked): Compares32 exact outcomes while rejecting the stage under an unrelated C protocol.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. The retained exact probe is an input artifact, not native implementation source. Reconfirmation must independently identify actual release, headers/library/build and preserve the original constructor, selected getter/API, observer order and input channel. Capture-time absolute paths are not a portable replay runner. Unrecorded compiler/configure/version queries and original output digests remain explicit limits.

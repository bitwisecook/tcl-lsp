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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, bigip

Status: `not-tested`.

No native observation of this exact question is attached for this provider.

### jim

Status: `observed`. Channel: Direct Jim_StringToWide CString conversion with explicit base10 and errno seed; no interpreter evaluator. Version: jim launched patchlevel/revision unqueried).

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

## Consumer bindings

- [rust/tcl-syntax/src/scalar_getter.rs](../../../../rust/tcl-syntax/src/scalar_getter.rs), `NativeScalarGetterProtocol::jim_decimal_wide_probe`: Owns this decimal string-only stage independently from primitive cache and expression numeral purposes.
- [rust/tcl-syntax/src/scalar_getter/tests.rs](../../../../rust/tcl-syntax/src/scalar_getter/tests.rs), `scalar_getter::tests::jim_decimal_string_switch_keeps_its_own_native_stage` (linked): Compares32 exact outcomes while rejecting the stage under an unrelated C protocol.

## Evidence

- [rust/tcl-syntax/tests/data/native_scalar_getters/decimal-switch/probe.c](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/decimal-switch/probe.c)
- [rust/tcl-syntax/tests/data/native_scalar_getters/decimal-switch/manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/decimal-switch/manifest.json)
- [rust/tcl-syntax/tests/data/native_scalar_getters/decimal-switch/jim0.84.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/decimal-switch/jim0.84.txt)

# naming.utf.native-isolated-unit-encode

Kind: `native-observation`

## Problem statement

Forward decoding, isolated unit encoding and reverse boundary movement need not agree with strict Unicode geometry or each other for malformed and surrogate inputs. Source spans cannot borrow an unmeasured native byte boundary.

## Question

What bytes result when each actually decoded native unit is separately passed to Tcl_UniCharToUtf?

## Conclusion

Raw zero encodes C080 in every C release. C8.6 astral decoding re-encodes the two isolated surrogate units as CESU bytes; C9 re-encodes the scalar as four bytes. Malformed/overlong source follows each release's decoded units. This isolated-unit sequence does not establish arbitrary source string conversion or all original UTF encoder values.

## Scope

Fourteen exact native byte arrays passed directly to public C character APIs, including raw00, malformed continuations, overlong sequences, surrogate, astral/max/out-of-range scalar and truncated sequence. The probe bounds its loop by retained counted byte length but individual public decoders see the source's terminator. Five output/source/library/executable digests and run0 captured; headers/compiler status/flags/runtime patch query absent. Jim and BIG-IP are not tested.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Direct public Tcl_UtfToUniChar, isolated Tcl_UniCharToUtf and Tcl_UtfPrev APIs over retained native byte arrays. Version: tcl8.4 8.4.20 (capture association; launched patchlevel/revision unqueried); tcl8.5 8.5.19 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
case=0 input=41 units=41:1 encoded=41 boundaries=0
case=1 input=00 units=0:1 encoded=c080 boundaries=0
case=2 input=8078 units=80:1,78:1 encoded=c28078 boundaries=0,1
case=3 input=9f78 units=9f:1,78:1 encoded=c29f78 boundaries=0,1
case=4 input=c080 units=0:2 encoded=c080 boundaries=0,0
case=5 input=c081 units=1:2 encoded=01 boundaries=0,0
case=6 input=c1bf units=7f:2 encoded=7f boundaries=0,0
case=7 input=e08080 units=0:3 encoded=c080 boundaries=0,0,0
case=8 input=eda080 units=d800:3 encoded=eda080 boundaries=0,0,0
case=9 input=f09f9880 units=f0:1,9f:1,98:1,80:1 encoded=c3b0c29fc298c280 boundaries=0,0,0,3
case=10 input=f48fbfbf units=f4:1,8f:1,bf:1,bf:1 encoded=c3b4c28fc2bfc2bf boundaries=0,0,0,3
case=11 input=f4908080 units=f4:1,90:1,80:1,80:1 encoded=c3b4c290c280c280 boundaries=0,0,0,3
case=12 input=e282 units=e2:1,82:1 encoded=c3a2c282 boundaries=0,0
case=13 input=418080 units=41:1,80:1,80:1 encoded=41c280c280 boundaries=0,1,2
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.6

Status: `observed`. Channel: Direct public Tcl_UtfToUniChar, isolated Tcl_UniCharToUtf and Tcl_UtfPrev APIs over retained native byte arrays. Version: tcl8.6 8.6.18 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
case=0 input=41 units=41:1 encoded=41 boundaries=0
case=1 input=00 units=0:1 encoded=c080 boundaries=0
case=2 input=8078 units=80:1,78:1 encoded=c28078 boundaries=0,1
case=3 input=9f78 units=9f:1,78:1 encoded=c29f78 boundaries=0,1
case=4 input=c080 units=0:2 encoded=c080 boundaries=0,0
case=5 input=c081 units=c0:1,81:1 encoded=c380c281 boundaries=0,1
case=6 input=c1bf units=c1:1,bf:1 encoded=c381c2bf boundaries=0,1
case=7 input=e08080 units=e0:1,80:1,80:1 encoded=c3a0c280c280 boundaries=0,1,2
case=8 input=eda080 units=d800:3 encoded=eda080 boundaries=0,0,0
case=9 input=f09f9880 units=d83d:1,de00:3 encoded=eda0bdedb880 boundaries=0,1,2,3
case=10 input=f48fbfbf units=dbff:1,dfff:3 encoded=edafbfedbfbf boundaries=0,1,2,3
case=11 input=f4908080 units=f4:1,90:1,80:1,80:1 encoded=c3b4c290c280c280 boundaries=0,1,2,3
case=12 input=e282 units=e2:1,82:1 encoded=c3a2c282 boundaries=0,0
case=13 input=418080 units=41:1,80:1,80:1 encoded=41c280c280 boundaries=0,1,2
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Direct public Tcl_UtfToUniChar, isolated Tcl_UniCharToUtf and Tcl_UtfPrev APIs over retained native byte arrays. Version: tcl9.0 9.0.4 (capture association; launched patchlevel/revision unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
case=0 input=41 units=41:1 encoded=41 boundaries=0
case=1 input=00 units=0:1 encoded=c080 boundaries=0
case=2 input=8078 units=20ac:1,78:1 encoded=e282ac78 boundaries=0,1
case=3 input=9f78 units=178:1,78:1 encoded=c5b878 boundaries=0,1
case=4 input=c080 units=0:2 encoded=c080 boundaries=0,0
case=5 input=c081 units=c0:1,81:1 encoded=c380c281 boundaries=0,1
case=6 input=c1bf units=c1:1,bf:1 encoded=c381c2bf boundaries=0,1
case=7 input=e08080 units=e0:1,20ac:1,20ac:1 encoded=c3a0e282ace282ac boundaries=0,1,2
case=8 input=eda080 units=d800:3 encoded=eda080 boundaries=0,0,0
case=9 input=f09f9880 units=1f600:4 encoded=f09f9880 boundaries=0,0,0,0
case=10 input=f48fbfbf units=10ffff:4 encoded=f48fbfbf boundaries=0,0,0,0
case=11 input=f4908080 units=f4:1,90:1,20ac:1,20ac:1 encoded=c3b4c290e282ace282ac boundaries=0,1,2,3
case=12 input=e282 units=e2:1,201a:1 encoded=c3a2e2809a boundaries=0,0
case=13 input=418080 units=41:1,20ac:1,20ac:1 encoded=41e282ace282ac boundaries=0,1,2
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### jim, bigip

Status: `not-tested`.

No native observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-syntax/src/native_tcl_utf.rs](../../../../rust/tcl-syntax/src/native_tcl_utf.rs), `NativeTclUtf`: Owns canonical release-selected native unit decoding/encoding and reverse boundaries separately from document UTF.
- [rust/tcl-syntax/src/native_tcl_utf.rs](../../../../rust/tcl-syntax/src/native_tcl_utf.rs), `native_tcl_utf::tests::canonical_native_units_encoding_and_previous_boundaries_match_five_releases` (linked): Checks70 exact native decoder widths/unit/isolated encoder and every reverse boundary, retaining each independent field.

## Evidence

- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/tcl-character-units-probe.c](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/tcl-character-units-probe.c)
- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-manifest.json)
- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-8.4.20.txt)
- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-8.5.19.txt)
- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-8.6.18.txt)
- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-9.0.4.txt)
- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/character-units-9.1.0.txt)

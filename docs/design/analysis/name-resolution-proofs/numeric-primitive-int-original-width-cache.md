# naming.numeric.primitive-int-original-width-cache

Kind: `native-observation`

## Problem statement

A primitive Int getter can narrow a larger cached integer while preserving its original cache, or fail after changing the original primary. A result value alone cannot determine width, mutation or error-code publication.

## Question

How does Tcl_GetIntFromObj convert fourteen original String/Double/Wide inputs across the five C releases?

## Conclusion

The captured2147483648 and4294967295 conversions narrow to -2147483648 and -1 across C releases. Larger inputs and negative4294967295 differ by release/range; the full matrix records success, sentinel value, cache, message and seeded errorCode. C8.4 rejects raw-zero1 and08; C9 accepts them as1 and8. Original Double1.0 rejects in every release. These are primitive Int getter results, not expression or Long/Wide conversion rules.

## Scope

Fourteen original constructors enter Tcl_GetIntFromObj directly after SEEDED CODE. Result message and error code/return-options are observed afterward. Five embedded original JSON row arrays match the retained six-column TSV projections exactly; the full original JSONL stdout stream is not retained. Source/library hashes are recorded; compiler status/header/executable/runtime version query are not. Jim and BIG-IP are not tested. The linked CmdCore integer error transport control compares the unchanged primitive Int getter String-bad row10 separately for C8.6, C9.0 and C9.1, retaining each release message and error-code classification plus the selected String-result producer. The public primitive fields and software transport identity remain independent from physical Native headers, frames and executed consumer assertions.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Direct public Tcl_GetIntFromObj on original String, Double or WideInt objects with seeded interpreter error state. Version: tcl8.4 8.4.20 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
0	1	777	string	657870656374656420696e74656765722062757420676f7420223122	53454544454420434f4445
1	0	-2147483648	int	-	53454544454420434f4445
2	0	-1	int	-	53454544454420434f4445
3	1	777	int	696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e74206173206e6f6e2d6c6f6e6720696e7465676572	53454544454420434f4445
4	0	1	int	-	53454544454420434f4445
5	1	777	int	696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e74206173206e6f6e2d6c6f6e6720696e7465676572	53454544454420434f4445
6	1	777	string	657870656374656420696e74656765722062757420676f742022312e3022	53454544454420434f4445
7	1	777	string	657870656374656420696e74656765722062757420676f7420224e614e22	53454544454420434f4445
8	1	777	string	657870656374656420696e74656765722062757420676f74202230382220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229	53454544454420434f4445
9	0	1	int	-	53454544454420434f4445
10	1	777	string	657870656374656420696e74656765722062757420676f74202262616422	53454544454420434f4445
11	1	777	double	657870656374656420696e74656765722062757420676f742022312e3022	53454544454420434f4445
12	0	1	wideInt	-	53454544454420434f4445
13	1	777	double	657870656374656420696e74656765722062757420676f7420226e616e22	53454544454420434f4445
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.5

Status: `observed`. Channel: Direct public Tcl_GetIntFromObj on original String, Double or WideInt objects with seeded interpreter error state. Version: tcl8.5 8.5.19 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
0	0	1	int	-	53454544454420434f4445
1	0	-2147483648	int	-	53454544454420434f4445
2	0	-1	int	-	53454544454420434f4445
3	1	777	int	696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e74206173206e6f6e2d6c6f6e6720696e7465676572	415249544820494f564552464c4f57207b696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e74206173206e6f6e2d6c6f6e6720696e74656765727d
4	0	1	int	-	53454544454420434f4445
5	1	777	bignum	696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e74206173206e6f6e2d6c6f6e6720696e7465676572	415249544820494f564552464c4f57207b696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e74206173206e6f6e2d6c6f6e6720696e74656765727d
6	1	777	string	657870656374656420696e74656765722062757420676f742022312e3022	54434c2056414c5545204e554d424552
7	1	777	string	657870656374656420696e74656765722062757420676f7420224e614e22	54434c2056414c5545204e554d424552
8	1	777	string	657870656374656420696e74656765722062757420676f742022303822	54434c2056414c5545204e554d424552
9	0	1	int	-	53454544454420434f4445
10	1	777	string	657870656374656420696e74656765722062757420676f74202262616422	54434c2056414c5545204e554d424552
11	1	777	double	657870656374656420696e74656765722062757420676f742022312e3022	53454544454420434f4445
12	0	1	int	-	53454544454420434f4445
13	1	777	double	657870656374656420696e74656765722062757420676f7420224e614e22	53454544454420434f4445
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.6

Status: `observed`. Channel: Direct public Tcl_GetIntFromObj on original String, Double or WideInt objects with seeded interpreter error state. Version: tcl8.6 8.6.18 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
0	0	1	int	-	53454544454420434f4445
1	0	-2147483648	int	-	53454544454420434f4445
2	0	-1	int	-	53454544454420434f4445
3	1	777	int	696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e74	415249544820494f564552464c4f57207b696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e747d
4	0	1	int	-	53454544454420434f4445
5	1	777	bignum	696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e74	415249544820494f564552464c4f57207b696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e747d
6	1	777	double	657870656374656420696e74656765722062757420676f742022312e3022	54434c2056414c554520494e5445474552
7	1	777	double	696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e74	415249544820494f564552464c4f57207b696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e747d
8	1	777	string	657870656374656420696e74656765722062757420676f742022303822	54434c2056414c554520494e5445474552
9	0	1	int	-	53454544454420434f4445
10	1	777	string	657870656374656420696e74656765722062757420676f74202262616422	54434c2056414c554520494e5445474552
11	1	777	double	657870656374656420696e74656765722062757420676f742022312e3022	54434c2056414c554520494e5445474552
12	0	1	int	-	53454544454420434f4445
13	1	777	double	696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e74	415249544820494f564552464c4f57207b696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e747d
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Direct public Tcl_GetIntFromObj on original String, Double or WideInt objects with seeded interpreter error state. Version: tcl9.0 9.0.4 (capture association; launched patchlevel/revision unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
0	0	1	int	-	53454544454420434f4445
1	0	-2147483648	int	-	53454544454420434f4445
2	0	-1	int	-	53454544454420434f4445
3	1	777	int	696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e74	415249544820494f564552464c4f57207b696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e747d
4	1	777	int	696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e74	415249544820494f564552464c4f57207b696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e747d
5	1	777	bignum	696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e74	415249544820494f564552464c4f57207b696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e747d
6	1	777	string	657870656374656420696e74656765722062757420676f742022312e3022	54434c2056414c5545204e554d424552
7	1	777	string	657870656374656420696e74656765722062757420676f7420224e614e22	54434c2056414c5545204e554d424552
8	0	8	int	-	53454544454420434f4445
9	0	1	int	-	53454544454420434f4445
10	1	777	string	657870656374656420696e74656765722062757420676f74202262616422	54434c2056414c5545204e554d424552
11	1	777	double	657870656374656420696e74656765722062757420676f742022312e3022	54434c2056414c554520494e5445474552
12	0	1	int	-	53454544454420434f4445
13	1	777	double	657870656374656420696e74656765722062757420676f7420224e614e22	54434c2056414c554520494e5445474552
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### jim, bigip

Status: `not-tested`.

No native observation of this exact question is attached for this provider.

## Consumer bindings

- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `native_scalar_getter`: Applies the selected primitive Int conversion and original cache/error publication obligations.
- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `typed_value::tests::primitive_int_matches_native_width_cache_and_failure_on_original_objects` (linked): Compares70 original value/cache/message/error-code windows with independently unavailable bignum-backend withdrawal.
- [rust/tcl-cmd-core/src/native_info_level.rs](../../../../rust/tcl-cmd-core/src/native_info_level.rs), `native_info_level::tests::native_level_integer_error_retains_primitive_fields_and_string_producer` (linked): Original primitive String-bad row10 supplies exact per-release message/error-code fields (C8.6 INTEGER versus C9 NUMBER); the typed software presenter retains the selected primitive and String-result producer, without claiming original physical header or frame identity.

## Evidence

- [rust/tcl-syntax/tests/data/native_scalar_getters/int/probe.c](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/probe.c)
- [rust/tcl-syntax/tests/data/native_scalar_getters/int/manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/manifest.json)
- [rust/tcl-syntax/tests/data/native_scalar_getters/int/8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/8.4.20.txt)
- [rust/tcl-syntax/tests/data/native_scalar_getters/int/8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/8.5.19.txt)
- [rust/tcl-syntax/tests/data/native_scalar_getters/int/8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/8.6.18.txt)
- [rust/tcl-syntax/tests/data/native_scalar_getters/int/9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/9.0.4.txt)
- [rust/tcl-syntax/tests/data/native_scalar_getters/int/9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/9.1.0.txt)

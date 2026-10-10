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

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel/revision unqueried). Build: library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Direct public Tcl_GetIntFromObj on original String, Double or WideInt objects with seeded interpreter error state.. Dialect: C Tcl.

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

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel/revision unqueried). Build: library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Direct public Tcl_GetIntFromObj on original String, Double or WideInt objects with seeded interpreter error state.. Dialect: C Tcl.

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

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel/revision unqueried). Build: library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Direct public Tcl_GetIntFromObj on original String, Double or WideInt objects with seeded interpreter error state.. Dialect: C Tcl.

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

### tcl9.0

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel/revision unqueried). Build: library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Direct public Tcl_GetIntFromObj on original String, Double or WideInt objects with seeded interpreter error state.. Dialect: C Tcl.

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

### tcl9.1

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel/revision unqueried). Build: library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Direct public Tcl_GetIntFromObj on original String, Double or WideInt objects with seeded interpreter error state.. Dialect: C Tcl.

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

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No native observation of this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No native observation of this exact question is attached for this provider.

## Exact evidence

- `probe-0` (input): [rust/tcl-syntax/tests/data/native_scalar_getters/int/probe.c](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/probe.c). SHA-256 `07998ce7fd101f2b8faad99ae82dc664518c07917a1791e8c3fb57e49c167bcb`. Exact retained input constructors, selected APIs, callback and before/after observers.
- `receipt-0` (provider): [rust/tcl-syntax/tests/data/native_scalar_getters/int/manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/manifest.json). SHA-256 `7570c7e689881907b85bbdbe2f29a714981bf90fd2e909abf96eb120fc1c13ed`. Original release/build/status association. Only fields actually recorded are claimed; reconstructed current checksums do not create an absent original full stdout digest.
- `rows-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/int/8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/8.4.20.txt). SHA-256 `b2a76489fbc21d1928a2bcc6712aad4a48f7260db9731ff96ae5b276ef7423c4`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/int/8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/8.5.19.txt). SHA-256 `d7925477646a3b55cc39183531741b817a2bec418262f7d2bee95e1c43bb1f53`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/int/8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/8.6.18.txt). SHA-256 `e2897daca308717db608ec26584bd5f88cc37aa51a90e0b9b49c0d39bf97dee1`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/int/9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/9.0.4.txt). SHA-256 `b740babf3a8ba0c945ad09c29952c0ad8d2a36e54e838f3b523ec59266c967d0`. Original retained full stream; provider answer selects only this question's relevant windows.
- `rows-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/int/9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/int/9.1.0.txt). SHA-256 `b740babf3a8ba0c945ad09c29952c0ad8d2a36e54e838f3b523ec59266c967d0`. Original retained full stream; provider answer selects only this question's relevant windows.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `native_scalar_getter`: Applies the selected primitive Int conversion and original cache/error publication obligations.
- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `typed_value::tests::primitive_int_matches_native_width_cache_and_failure_on_original_objects` (linked): Compares70 original value/cache/message/error-code windows with independently unavailable bignum-backend withdrawal.
- [rust/tcl-cmd-core/src/native_info_level.rs](../../../../rust/tcl-cmd-core/src/native_info_level.rs), `native_info_level::tests::native_level_integer_error_retains_primitive_fields_and_string_producer` (linked): Original primitive String-bad row10 supplies exact per-release message/error-code fields (C8.6 INTEGER versus C9 NUMBER); the typed software presenter retains the selected primitive and String-result producer, without claiming original physical header or frame identity.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. The retained exact probe is an input artifact, not native implementation source. Reconfirmation must independently identify actual release, headers/library/build and preserve the original constructor, selected getter/API, observer order and input channel. Capture-time absolute paths are not a portable replay runner. Unrecorded compiler/configure/version queries and original output digests remain explicit limits.

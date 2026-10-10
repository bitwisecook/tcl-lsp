# naming.numeric.c84-long-wide-original-primary

Kind: `native-observation`

## Problem statement

On a build where long and Tcl_WideInt are both eight bytes, an Int, Long and Wide getter still have distinct primary and narrowing rules. Collapsing them by host width changes original storage.

## Question

Which original primary, residency, allocation and primitive value changes occur for thirteen C8.4 constructors under Int/Long/Wide getters?

## Conclusion

The probe records sizeof(long)=sizeof(Tcl_WideInt)=8. Int/Long preserve original int primaries, while Wide converts int to wideInt even for17. Long accepts4294967296 while Int rejects; fresh numeric strings acquire the getter-specific primary and retain their allocated string. Original Double17.0 rejects and materializes its message string; raw-zero1X and08 reject. Full39 rows retain large signed/unsigned boundary outcomes. No error-code observation is printed.

## Scope

Exactly C8.4 capture with public original Int/Long/Wide/String/Double constructors and three direct getter selections. META width row plus39 actual windows; probe/library/executable hashes, compile0/run0/empty stderr retained, rust_comparisons0. Other releases, Jim and BIG-IP are not tested; no inferred universal host-width rule.

## Provider answers

### tcl8.4

Status: `observed`. Version: not queried (capture association; launched patchlevel/revision unqueried). Build: binary_sha256=79fd606a11aaea80d59eb06777968d6243fc3323aacf732a8ed48e576bbfaf53; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; compile_exit=0; exit=0. Compiler version/full configure flags unqueried; missing header/compiler status/executable digests were not measured.. Channel: Direct public Tcl_GetIntFromObj/Tcl_GetLongFromObj/Tcl_GetWideIntFromObj before original byte observer.. Dialect: C Tcl.

Exact relevant original rows:

```text
META	8	8
ROW	0	0	int	int	0	0	0	17		0
ROW	0	1	int	int	0	0	0	17		0
ROW	0	2	int	wideInt	0	0	0	17		0
ROW	1	0	int	int	0	0	0	-1		0
ROW	1	1	int	int	0	0	0	4294967295		0
ROW	1	2	int	wideInt	0	0	0	4294967295		0
ROW	2	0	wideInt	wideInt	0	0	0	17		0
ROW	2	1	wideInt	wideInt	0	0	0	17		0
ROW	2	2	wideInt	wideInt	0	0	0	17		0
ROW	3	0	wideInt	wideInt	0	0	0	777	696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e74206173206e6f6e2d6c6f6e6720696e7465676572	1
ROW	3	1	wideInt	wideInt	0	0	0	4294967296		0
ROW	3	2	wideInt	wideInt	0	0	0	4294967296		0
ROW	4	0	NULL	int	1	1	1	17		0
ROW	4	1	NULL	int	1	1	1	17		0
ROW	4	2	NULL	wideInt	1	1	1	17		0
ROW	5	0	NULL	int	1	1	1	-1		0
ROW	5	1	NULL	int	1	1	1	4294967295		0
ROW	5	2	NULL	wideInt	1	1	1	4294967295		0
ROW	6	0	NULL	int	1	1	1	777	696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e74206173206e6f6e2d6c6f6e6720696e7465676572	1
ROW	6	1	NULL	int	1	1	1	4294967296		0
ROW	6	2	NULL	wideInt	1	1	1	4294967296		0
ROW	7	0	NULL	int	1	1	1	-1		0
ROW	7	1	NULL	int	1	1	1	-1		0
ROW	7	2	NULL	wideInt	1	1	1	-1		0
ROW	8	0	NULL	int	1	1	1	1		0
ROW	8	1	NULL	int	1	1	1	1		0
ROW	8	2	NULL	wideInt	1	1	1	1		0
ROW	9	0	NULL	NULL	1	1	1	777	696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e74	1
ROW	9	1	NULL	NULL	1	1	1	777	696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e74	1
ROW	9	2	NULL	NULL	1	1	1	777	696e74656765722076616c756520746f6f206c6172676520746f20726570726573656e74	1
ROW	10	0	NULL	NULL	1	1	1	777	657870656374656420696e74656765722062757420676f7420223122	1
ROW	10	1	NULL	NULL	1	1	1	777	657870656374656420696e74656765722062757420676f7420223122	1
ROW	10	2	NULL	NULL	1	1	1	777	657870656374656420696e74656765722062757420676f7420223122	1
ROW	11	0	double	double	0	1	0	777	657870656374656420696e74656765722062757420676f74202231372e3022	1
ROW	11	1	double	double	0	1	0	777	657870656374656420696e74656765722062757420676f74202231372e3022	1
ROW	11	2	double	double	0	1	0	777	657870656374656420696e74656765722062757420676f74202231372e3022	1
ROW	12	0	NULL	NULL	1	1	1	777	657870656374656420696e74656765722062757420676f74202230382220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229	1
ROW	12	1	NULL	NULL	1	1	1	777	657870656374656420696e74656765722062757420676f74202230382220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229	1
ROW	12	2	NULL	NULL	1	1	1	777	657870656374656420696e74656765722062757420676f74202230382220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229	1
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

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

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No native observation of this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No native observation of this exact question is attached for this provider.

## Exact evidence

- `probe-0` (input): [rust/tcl-syntax/tests/data/native_scalar_getters/long84/probe.c](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/long84/probe.c). SHA-256 `dbfa61509809d5d20c0b89e9334f7b95c828bb89d888440bfa110ab49992ef87`. Exact retained input constructors, selected APIs, callback and before/after observers.
- `receipt-0` (provider): [rust/tcl-syntax/tests/data/native_scalar_getters/long84/manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/long84/manifest.json). SHA-256 `c87897b71108a7116232510ba74679eb4c4c7db376ac03bd0e27d6971c13849a`. Original release/build/status association. Only fields actually recorded are claimed; reconstructed current checksums do not create an absent original full stdout digest.
- `rows-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_scalar_getters/long84/8.4.20.tsv](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/long84/8.4.20.tsv). SHA-256 `b8b4fda4caffa67734ee9d362536c94d49085c01eeba07eb04c3db416bc6d3f8`. Original retained full stream; provider answer selects only this question's relevant windows.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/scalar_getter.rs](../../../../rust/tcl-syntax/src/scalar_getter.rs), `NativeScalarGetterProtocol::cached_conversion`: Keeps the retained C8.4 long primary separate from primitive Wide conversion.
- [rust/tcl-syntax/src/scalar_getter/tests.rs](../../../../rust/tcl-syntax/src/scalar_getter/tests.rs), `scalar_getter::tests::legacy_long_and_wide_retain_distinct_native_primary_caches` (linked): Checks39 original primitive/cache/value windows independently of unobserved interpreter error-code state.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. The retained exact probe is an input artifact, not native implementation source. Reconfirmation must independently identify actual release, headers/library/build and preserve the original constructor, selected getter/API, observer order and input channel. Capture-time absolute paths are not a portable replay runner. Unrecorded compiler/configure/version queries and original output digests remain explicit limits.

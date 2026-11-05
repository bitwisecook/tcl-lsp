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

Status: `observed`. Channel: Direct public Tcl_GetIntFromObj/Tcl_GetLongFromObj/Tcl_GetWideIntFromObj before original byte observer. Version: tcl8.4 launched patchlevel/revision unqueried).

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

### tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No native observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-syntax/src/scalar_getter.rs](../../../../rust/tcl-syntax/src/scalar_getter.rs), `NativeScalarGetterProtocol::cached_conversion`: Keeps the retained C8.4 long primary separate from primitive Wide conversion.
- [rust/tcl-syntax/src/scalar_getter/tests.rs](../../../../rust/tcl-syntax/src/scalar_getter/tests.rs), `scalar_getter::tests::legacy_long_and_wide_retain_distinct_native_primary_caches` (linked): Checks39 original primitive/cache/value windows independently of unobserved interpreter error-code state.

## Evidence

- [rust/tcl-syntax/tests/data/native_scalar_getters/long84/probe.c](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/long84/probe.c)
- [rust/tcl-syntax/tests/data/native_scalar_getters/long84/manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/long84/manifest.json)
- [rust/tcl-syntax/tests/data/native_scalar_getters/long84/8.4.20.tsv](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/long84/8.4.20.tsv)

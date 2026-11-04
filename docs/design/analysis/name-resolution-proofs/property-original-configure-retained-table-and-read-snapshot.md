# naming.property.original-configure-retained-table-and-read-snapshot

Kind: `native-observation`

## Problem statement

A property setter or getter can change the property roster while configure remains active. Reusing an old selected name as though it were a current member, or rebuilding a configure-all read inventory after each getter, changes the selected property and returned snapshot. Public result equality must remain separate from the native table/header storage recipe.

## Question

For the two exact original configurable-class scripts, which setter executes after the first setter inserts b, and which properties and events does configure-all return when its first getter inserts b and an unrelated method?

## Conclusion

Tcl 9.0.4 and 9.1.0 each close both scripts exit0 with empty stderr. Multi-write reports events {a b}, while configure-all reports {-a A -z Z} and events {a z}, excluding newly added b; both final rosters include -a,-b,-z. Tcl 8.4.20, 8.5.19, 8.6.18 and Jim 0.84-9-g5bac7c9 reject oo::configurable before these workers. Two generic-class attempts reject property before the intended worker and supply no intended answer. Independent C9 source excerpts explain old cached name-table selection against current members and the retained configure-all List; C9.0 printed values do not grant the C9.1 software physical purpose. BIG-IP and the named Rust body have no execution outcome.

## Scope

Twelve exact configurable-script CLI observations, independent own version/process receipts, and two separately retained generic-class setup refusals. Script-visible code/result/event/roster bytes are measured; no custom header updater, pointer identity, cache primary, reference-count chronology or internal error-formatting stop is observed. Six invoked-file/downstream executable joins and whole lossless ELF bytes are retained. ldd reports dependencies without retained shared-library byte closure; inherited full environment and configure/build replay are absent. Eight exact source windows join whole C9.0/C9.1 tclOOProp.c bytes independently. One source-bound C9.1 software comparator remains unrun; no physical frame, Host metadata, Native admission, whole engine, BIG-IP/TMM or Rust implementation equivalence follows.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual invoked shell wrapper and its downstream Tcl ELF identified by original recorded whole hashes; exact executable gzip roundtrip and 8.4.20 printed version are retained. Reported ldd dependencies lack retained shared-library bytes and launch-time dependency hashes. Source inspection is not configure/build replay or a binary/source equivalence proof.. Channel: Two exact ordinary script files through recorded CLI argv; independent version script, whole public stdout/stderr and own closed process receipts.. Dialect: tcl8.4.

Both exact configurable scripts close exit1 with empty stdout and invalid command name "oo::configurable" before either intended configure worker is reached. This measures producer unavailability for these invocations, not the intended retained-table/read-snapshot behavior.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual invoked shell wrapper and its downstream Tcl ELF identified by original recorded whole hashes; exact executable gzip roundtrip and 8.5.19 printed version are retained. Reported ldd dependencies lack retained shared-library bytes and launch-time dependency hashes. Source inspection is not configure/build replay or a binary/source equivalence proof.. Channel: Two exact ordinary script files through recorded CLI argv; independent version script, whole public stdout/stderr and own closed process receipts.. Dialect: tcl8.5.

Both exact configurable scripts close exit1 with empty stdout and invalid command name "oo::configurable" before either intended configure worker is reached. This measures producer unavailability for these invocations, not the intended retained-table/read-snapshot behavior.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual invoked shell wrapper and its downstream Tcl ELF identified by original recorded whole hashes; exact executable gzip roundtrip and 8.6.18 printed version are retained. Reported ldd dependencies lack retained shared-library bytes and launch-time dependency hashes. Source inspection is not configure/build replay or a binary/source equivalence proof.. Channel: Two exact ordinary script files through recorded CLI argv; independent version script, whole public stdout/stderr and own closed process receipts.. Dialect: tcl8.6.

Both exact configurable scripts close exit1 with empty stdout and invalid command name "oo::configurable" before either intended configure worker is reached. This measures producer unavailability for these invocations, not the intended retained-table/read-snapshot behavior.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual invoked shell wrapper and its downstream Tcl ELF identified by original recorded whole hashes; exact executable gzip roundtrip and 9.0.4 printed version are retained. Reported ldd dependencies lack retained shared-library bytes and launch-time dependency hashes. Source inspection is not configure/build replay or a binary/source equivalence proof.. Channel: Two exact ordinary script files through recorded CLI argv; independent version script, whole public stdout/stderr and own closed process receipts.. Dialect: tcl9.0.

Both exact configurable scripts close exit0 with empty stderr. Multi-write prints code 0, empty result, events {a b} and properties {-a -b -z}; configure-all prints code 0, result {-a A -z Z}, events {a z} and properties {-a -b -z}. The latter excludes newly added b from its read result. These public values alone establish no pointer/refcount/physical cache recipe or Rust assertion outcome.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual invoked shell wrapper and its downstream Tcl ELF identified by original recorded whole hashes; exact executable gzip roundtrip and 9.1.0 printed version are retained. Reported ldd dependencies lack retained shared-library bytes and launch-time dependency hashes. Source inspection is not configure/build replay or a binary/source equivalence proof.. Channel: Two exact ordinary script files through recorded CLI argv; independent version script, whole public stdout/stderr and own closed process receipts.. Dialect: tcl9.1.

Both exact configurable scripts close exit0 with empty stderr. Multi-write prints code 0, empty result, events {a b} and properties {-a -b -z}; configure-all prints code 0, result {-a A -z Z}, events {a z} and properties {-a -b -z}. The latter excludes newly added b from its read result. These public values alone establish no pointer/refcount/physical cache recipe or Rust assertion outcome.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual invoked Jim ELF identified by original recorded whole hashes; exact executable gzip roundtrip and 0.84-9-g5bac7c9 printed version are retained. Reported ldd dependencies lack retained shared-library bytes and launch-time dependency hashes. Source inspection is not configure/build replay or a binary/source equivalence proof.. Channel: Two exact ordinary script files through recorded CLI argv; independent version script, whole public stdout/stderr and own closed process receipts.. Dialect: jim.

Both exact configurable scripts close exit1 with empty stdout and invalid command name "oo::configurable" before either intended configure worker is reached. This measures producer unavailability for these invocations, not the intended retained-table/read-snapshot behavior.

### bigip

Status: `not-tested`. Version: not tested. Build: No original BIG-IP process for this property question. Channel: No BIG-IP property script response. Dialect: bigip.

No original BIG-IP response is attached. Authored Runtime profiles and local Tcl/Jim observations establish no BIG-IP/TMM property behavior.

## Historical references

- `multi-write-configurable-input` (input): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/multi-write-configurable.tcl](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/multi-write-configurable.tcl). Exact ordinary configurable-class input supplied to all six measured provider invocations. Public printed result, event order and roster are separate from native pointer/layout observations.
- `configure-all-configurable-input` (input): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configure-all-configurable.tcl](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configure-all-configurable.tcl). Exact ordinary configurable-class input supplied to all six measured provider invocations. Public printed result, event order and roster are separate from native pointer/layout observations.
- `matrix-original` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/matrix.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/matrix.json). Original twelve provider/case summaries with actual argv, input/wrapper hashes, exits/durations, raw stream hashes and independent version channels. Each original own receipt and raw stream is retained independently.
- `configurable-original-receipt` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configurable-receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configurable-receipt.json). Two independently retained C9.1 original process records with the actual configurable producer. These are distinct from the twelve-case matrix invocations.
- `rejected-original-receipt` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/submitted-receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/submitted-receipt.json). Two exact C9.1 generic oo::class submissions close exit1 at invalid property command before the intended configure workers. Their setup refusal establishes no retained-table/read-snapshot result.
- `multi-write-submitted-tcl` (input): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/multi-write-submitted.tcl](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/multi-write-submitted.tcl). Exact original generic-class attempt bytes. The intended property configure worker is not reached; rejected setup must not supply a successful property observation.
- `multi-write-submitted-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/multi-write-submitted.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/multi-write-submitted.stdout). Exact original generic-class attempt bytes. The intended property configure worker is not reached; rejected setup must not supply a successful property observation.
- `multi-write-submitted-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/multi-write-submitted.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/multi-write-submitted.stderr). Exact original generic-class attempt bytes. The intended property configure worker is not reached; rejected setup must not supply a successful property observation.
- `configure-all-submitted-tcl` (input): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configure-all-submitted.tcl](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configure-all-submitted.tcl). Exact original generic-class attempt bytes. The intended property configure worker is not reached; rejected setup must not supply a successful property observation.
- `configure-all-submitted-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configure-all-submitted.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configure-all-submitted.stdout). Exact original generic-class attempt bytes. The intended property configure worker is not reached; rejected setup must not supply a successful property observation.
- `configure-all-submitted-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configure-all-submitted.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configure-all-submitted.stderr). Exact original generic-class attempt bytes. The intended property configure worker is not reached; rejected setup must not supply a successful property observation.
- `version-input` (input): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/version.tcl](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/version.tcl). Exact independent version/nameofexecutable script. This ordinary version channel does not measure native property memory or full build provenance.
- `provider-retention-original` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-retention.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-retention.json). Original full invoked-wrapper/downstream-ELF and source joins. Six retained gzip executables independently round-trip to recorded full SHA and size. ldd outputs report loader dependencies without retained shared-library bytes or launch-time hashes; configure/build replay is absent.
- `tcl8.4-multi-write-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/receipt.json). Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl8.4-multi-write-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/stdout). Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl8.4-multi-write-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/stderr). Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl8.4-multi-write-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/version.stdout). Exact independent printed version and executable path from this original invocation.
- `tcl8.4-multi-write-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/version.stderr). Exact independent version-channel stderr, retained empty.
- `tcl8.4-configure-all-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/receipt.json). Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl8.4-configure-all-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/stdout). Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl8.4-configure-all-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/stderr). Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl8.4-configure-all-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/version.stdout). Exact independent printed version and executable path from this original invocation.
- `tcl8.4-configure-all-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/version.stderr). Exact independent version-channel stderr, retained empty.
- `tcl8.4-executable-elf-gz` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/executable.elf.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/executable.elf.gz). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.4-ldd-stdout` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/ldd.stdout). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.4-ldd-stderr` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/ldd.stderr). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.4-wrapper-sh` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/wrapper.sh](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/wrapper.sh). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.5-multi-write-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/receipt.json). Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl8.5-multi-write-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/stdout). Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl8.5-multi-write-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/stderr). Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl8.5-multi-write-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/version.stdout). Exact independent printed version and executable path from this original invocation.
- `tcl8.5-multi-write-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/version.stderr). Exact independent version-channel stderr, retained empty.
- `tcl8.5-configure-all-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/receipt.json). Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl8.5-configure-all-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/stdout). Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl8.5-configure-all-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/stderr). Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl8.5-configure-all-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/version.stdout). Exact independent printed version and executable path from this original invocation.
- `tcl8.5-configure-all-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/version.stderr). Exact independent version-channel stderr, retained empty.
- `tcl8.5-executable-elf-gz` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/executable.elf.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/executable.elf.gz). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.5-ldd-stdout` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/ldd.stdout). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.5-ldd-stderr` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/ldd.stderr). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.5-wrapper-sh` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/wrapper.sh](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/wrapper.sh). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.6-multi-write-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/receipt.json). Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl8.6-multi-write-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/stdout). Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl8.6-multi-write-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/stderr). Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl8.6-multi-write-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/version.stdout). Exact independent printed version and executable path from this original invocation.
- `tcl8.6-multi-write-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/version.stderr). Exact independent version-channel stderr, retained empty.
- `tcl8.6-configure-all-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/receipt.json). Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl8.6-configure-all-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/stdout). Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl8.6-configure-all-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/stderr). Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl8.6-configure-all-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/version.stdout). Exact independent printed version and executable path from this original invocation.
- `tcl8.6-configure-all-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/version.stderr). Exact independent version-channel stderr, retained empty.
- `tcl8.6-executable-elf-gz` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/executable.elf.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/executable.elf.gz). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.6-ldd-stdout` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/ldd.stdout). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.6-ldd-stderr` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/ldd.stderr). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.6-wrapper-sh` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/wrapper.sh](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/wrapper.sh). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl9.0-multi-write-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/receipt.json). Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl9.0-multi-write-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/stdout). Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl9.0-multi-write-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/stderr). Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl9.0-multi-write-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/version.stdout). Exact independent printed version and executable path from this original invocation.
- `tcl9.0-multi-write-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/version.stderr). Exact independent version-channel stderr, retained empty.
- `tcl9.0-configure-all-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/receipt.json). Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl9.0-configure-all-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/stdout). Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl9.0-configure-all-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/stderr). Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl9.0-configure-all-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/version.stdout). Exact independent printed version and executable path from this original invocation.
- `tcl9.0-configure-all-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/version.stderr). Exact independent version-channel stderr, retained empty.
- `tcl9.0-executable-elf-gz` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/executable.elf.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/executable.elf.gz). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl9.0-ldd-stdout` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/ldd.stdout). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl9.0-ldd-stderr` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/ldd.stderr). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl9.0-wrapper-sh` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/wrapper.sh](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/wrapper.sh). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl9.1-multi-write-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/receipt.json). Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl9.1-multi-write-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/stdout). Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl9.1-multi-write-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/stderr). Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl9.1-multi-write-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/version.stdout). Exact independent printed version and executable path from this original invocation.
- `tcl9.1-multi-write-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/version.stderr). Exact independent version-channel stderr, retained empty.
- `tcl9.1-configure-all-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/receipt.json). Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl9.1-configure-all-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/stdout). Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl9.1-configure-all-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/stderr). Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl9.1-configure-all-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/version.stdout). Exact independent printed version and executable path from this original invocation.
- `tcl9.1-configure-all-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/version.stderr). Exact independent version-channel stderr, retained empty.
- `tcl9.1-executable-elf-gz` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/executable.elf.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/executable.elf.gz). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl9.1-ldd-stdout` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/ldd.stdout). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl9.1-ldd-stderr` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/ldd.stderr). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl9.1-wrapper-sh` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/wrapper.sh](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/wrapper.sh). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `jim-multi-write-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/receipt.json). Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `jim-multi-write-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/stdout). Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `jim-multi-write-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/stderr). Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `jim-multi-write-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/version.stdout). Exact independent printed version and executable path from this original invocation.
- `jim-multi-write-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/version.stderr). Exact independent version-channel stderr, retained empty.
- `jim-configure-all-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/receipt.json). Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `jim-configure-all-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/stdout). Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `jim-configure-all-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/stderr). Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `jim-configure-all-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/version.stdout). Exact independent printed version and executable path from this original invocation.
- `jim-configure-all-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/version.stderr). Exact independent version-channel stderr, retained empty.
- `jim-executable-elf-gz` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/executable.elf.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/executable.elf.gz). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `jim-ldd-stdout` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/ldd.stdout). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `jim-ldd-stderr` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/ldd.stderr). Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `source-window-0` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.1.0-274-343.c](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.1.0-274-343.c). Exact physical-LF source excerpt rejoined to retained whole tclOOProp.c SHA. It inspects current-member lookup, copied old table List/cache release, or configure-all retained working List. C9.0 source joins and printed values do not issue the software adapter C9.1 physical purpose.
- `source-window-1` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.1.0-347-360.c](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.1.0-347-360.c). Exact physical-LF source excerpt rejoined to retained whole tclOOProp.c SHA. It inspects current-member lookup, copied old table List/cache release, or configure-all retained working List. C9.0 source joins and printed values do not issue the software adapter C9.1 physical purpose.
- `source-window-2` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.1.0-365-409.c](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.1.0-365-409.c). Exact physical-LF source excerpt rejoined to retained whole tclOOProp.c SHA. It inspects current-member lookup, copied old table List/cache release, or configure-all retained working List. C9.0 source joins and printed values do not issue the software adapter C9.1 physical purpose.
- `source-window-3` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.1.0-416-466.c](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.1.0-416-466.c). Exact physical-LF source excerpt rejoined to retained whole tclOOProp.c SHA. It inspects current-member lookup, copied old table List/cache release, or configure-all retained working List. C9.0 source joins and printed values do not issue the software adapter C9.1 physical purpose.
- `source-window-4` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.0.4-163-232.c](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.0.4-163-232.c). Exact physical-LF source excerpt rejoined to retained whole tclOOProp.c SHA. It inspects current-member lookup, copied old table List/cache release, or configure-all retained working List. C9.0 source joins and printed values do not issue the software adapter C9.1 physical purpose.
- `source-window-5` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.0.4-236-249.c](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.0.4-236-249.c). Exact physical-LF source excerpt rejoined to retained whole tclOOProp.c SHA. It inspects current-member lookup, copied old table List/cache release, or configure-all retained working List. C9.0 source joins and printed values do not issue the software adapter C9.1 physical purpose.
- `source-window-6` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.0.4-254-298.c](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.0.4-254-298.c). Exact physical-LF source excerpt rejoined to retained whole tclOOProp.c SHA. It inspects current-member lookup, copied old table List/cache release, or configure-all retained working List. C9.0 source joins and printed values do not issue the software adapter C9.1 physical purpose.
- `source-window-7` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.0.4-305-355.c](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.0.4-305-355.c). Exact physical-LF source excerpt rejoined to retained whole tclOOProp.c SHA. It inspects current-member lookup, copied old table List/cache release, or configure-all retained working List. C9.0 source joins and printed values do not issue the software adapter C9.1 physical purpose.
- `tcl9.0-whole-property-source` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/tclOOProp.c.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/tclOOProp.c.gz). Lossless complete original tclOOProp.c bytes identified by provider-retention.json; exact physical-LF excerpts are selected independently. Inspected source does not prove that a binary was built from these bytes.
- `tcl9.1-whole-property-source` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/tclOOProp.c.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/tclOOProp.c.gz). Lossless complete original tclOOProp.c bytes identified by provider-retention.json; exact physical-LF excerpts are selected independently. Inspected source does not prove that a binary was built from these bytes.
- `current-source-0` (implementation): [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs). Current software owner or actual parent module registration. The source definition has no attached matching owned Rust assertion result; the public C9 prints do not establish its physical cache/lifetime behavior.
- `current-source-1` (implementation): [runtime/rust/src/cmd_oo/original_property_reporting_tests.rs](../../../../runtime/rust/src/cmd_oo/original_property_reporting_tests.rs). Current software owner or actual parent module registration. The source definition has no attached matching owned Rust assertion result; the public C9 prints do not establish its physical cache/lifetime behavior.
- `current-source-2` (implementation): [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs). Current software owner or actual parent module registration. The source definition has no attached matching owned Rust assertion result; the public C9 prints do not establish its physical cache/lifetime behavior.
- `receiver-current642-2637358fa5ac296a` (implementation): [rust/tcl-vm/src/cmd_oo/native_properties/stage_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_properties/stage_tests.rs). Current authored OO/property/method/lifecycle source or genuine control declaration, inspected only for the linked software scope. No matching current own-image/assertion, provider, physical callback/reference or lookup authority is attached.

## Source inspection

tcl9.1 9.1.0, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `GetPropertyName`, lines 274–343. Retained reference `source-window-0`.

```text
GetPropertyName(
    Tcl_Interp *interp,		/* Context and error reporting. */
    Object *oPtr,		/* Object to get property name from. */
    int flags,			/* Are we looking for a writable property?
				 * Can we do a fallback message?
				 * See GPNFlags for possible values */
    Tcl_Obj *namePtr,		/* The name supplied by the user. */
    GPNCache **cachePtr)	/* Where to cache the table, if the caller
				 * wants that. The contents are to be freed
				 * with Tcl_Free if the cache is used. */
{
    Tcl_Size objc, index, i;
    Tcl_Obj *listPtr = GetAllObjectProperties(
	    oPtr, (flags & GPN_WRITABLE) != 0);
    Tcl_Obj **objv;
    GPNCache *tablePtr;

    (void) Tcl_ListObjGetElements(NULL, listPtr, &objc, &objv);
    if (cachePtr && *cachePtr) {
	tablePtr = *cachePtr;
    } else {
	tablePtr = (GPNCache *) TclStackAlloc(interp,
		offsetof(GPNCache, names) + sizeof(char *) * ((size_t)objc + 1));

	for (i = 0; i < objc; i++) {
	    tablePtr->names[i] = TclGetString(objv[i]);
	}
	tablePtr->names[objc] = NULL;
	if (cachePtr) {
	    /*
	     * Have a cache, but nothing in it so far.
	     *
	     * We cache the list here so it doesn't vanish from under our
	     * feet if a property implementation does something crazy like
	     * changing the set of properties. The type of copy this does
	     * means that the copy holds the references to the names in the
	     * table.
	     */
	    tablePtr->listPtr = TclListObjCopy(NULL, listPtr);
	    Tcl_IncrRefCount(tablePtr->listPtr);
	    *cachePtr = tablePtr;
	} else {
	    tablePtr->listPtr = NULL;
	}
    }
    int result = Tcl_GetIndexFromObjStruct(interp, namePtr, tablePtr->names,
	    sizeof(char *), "property", TCL_INDEX_TEMP_TABLE, &index);
    if (result == TCL_ERROR && !(flags & GPN_FALLING_BACK)) {
	/*
	 * If property can be accessed the other way, use a special message.
	 * We use a recursive call to look this up.
	 */

	Tcl_InterpState state = Tcl_SaveInterpState(interp, result);
	Tcl_Obj *otherName = GetPropertyName(interp, oPtr,
		flags ^ (GPN_WRITABLE | GPN_FALLING_BACK), namePtr, NULL);
	result = Tcl_RestoreInterpState(interp, state);
	if (otherName != NULL) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "property \"%s\" is %s only",
		    TclGetString(otherName),
		    (flags & GPN_WRITABLE) ? "read" : "write"));
	}
    }
    if (!cachePtr) {
	TclStackFree(interp, tablePtr);
    }
    if (result != TCL_OK) {
	return NULL;
    }

```

tcl9.1 9.1.0, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `ReleasePropertyNameCache`, lines 347–360. Retained reference `source-window-1`.

```text
/* Release the cache made by GetPropertyName(). */
static inline void
ReleasePropertyNameCache(
    Tcl_Interp *interp,
    GPNCache **cachePtr)
{
    if (*cachePtr) {
	GPNCache *tablePtr = *cachePtr;
	if (tablePtr->listPtr) {
	    Tcl_DecrRefCount(tablePtr->listPtr);
	}
	TclStackFree(interp, tablePtr);
	*cachePtr = NULL;
    }

```

tcl9.1 9.1.0, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `TclOO_Configurable_Configure`, lines 365–409. Retained reference `source-window-2`.

```text
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Interpreter used for the result, error
				 * reporting, etc. */
    Tcl_ObjectContext context,	/* The object/call context. */
    Tcl_Size objc,		/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The actual arguments. */
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    Tcl_Obj *namePtr;
    Tcl_Size i, namec;
    int code = TCL_OK;

    objc -= skip;
    if ((objc & 1) && (objc != 1)) {
	/*
	 * Bad (odd > 1) number of arguments.
	 */

	Tcl_WrongNumArgs(interp, skip, objv, "?-option value ...?");
	return TCL_ERROR;
    }

    objv += skip;
    if (objc == 0) {
	/*
	 * Read all properties.
	 */

	Tcl_Obj *listPtr = GetAllObjectProperties(oPtr, false);
	Tcl_Obj *resultPtr = Tcl_NewObj(), **namev;

	Tcl_IncrRefCount(listPtr);
	ListObjGetElements(listPtr, namec, namev);

	for (i = 0; i < namec; ) {
	    code = ReadProperty(interp, oPtr, namev[i]);
	    if (code != TCL_OK) {
		Tcl_DecrRefCount(resultPtr);
		break;
	    }
	    Tcl_DictObjPut(NULL, resultPtr, namev[i],
		    Tcl_GetObjResult(interp));
	    if (++i >= namec) {
		Tcl_SetObjResult(interp, resultPtr);

```

tcl9.1 9.1.0, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `TclOO_Configurable_Configure`, lines 416–466. Retained reference `source-window-3`.

```text
    } else if (objc == 1) {
	/*
	 * Read a single named property.
	 */

	namePtr = GetPropertyName(interp, oPtr, 0, objv[0], NULL);
	if (namePtr == NULL) {
	    return TCL_ERROR;
	}
	return ReadProperty(interp, oPtr, namePtr);
    } else if (objc == 2) {
	/*
	 * Special case for writing to one property. Saves fiddling with the
	 * cache in this common case.
	 */

	namePtr = GetPropertyName(interp, oPtr, GPN_WRITABLE, objv[0], NULL);
	if (namePtr == NULL) {
	    return TCL_ERROR;
	}
	code = WriteProperty(interp, oPtr, namePtr, objv[1]);
	if (code == TCL_OK) {
	    Tcl_ResetResult(interp);
	}
	return code;
    } else {
	/*
	 * Write properties. Slightly tricky because we want to cache the
	 * table of property names.
	 */
	GPNCache *cache = NULL;

	code = TCL_OK;
	for (i = 0; i < objc; i += 2) {
	    namePtr = GetPropertyName(interp, oPtr, GPN_WRITABLE, objv[i],
		    &cache);
	    if (namePtr == NULL) {
		code = TCL_ERROR;
		break;
	    }
	    code = WriteProperty(interp, oPtr, namePtr, objv[i + 1]);
	    if (code != TCL_OK) {
		break;
	    }
	}
	if (code == TCL_OK) {
	    Tcl_ResetResult(interp);
	}
	ReleasePropertyNameCache(interp, &cache);
	return code;
    }

```

tcl9.0 9.0.4, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOProp.c`, function `GetPropertyName`, lines 163–232. Retained reference `source-window-4`.

```text
GetPropertyName(
    Tcl_Interp *interp,		/* Context and error reporting. */
    Object *oPtr,		/* Object to get property name from. */
    int flags,			/* Are we looking for a writable property?
				 * Can we do a fallback message?
				 * See GPNFlags for possible values */
    Tcl_Obj *namePtr,		/* The name supplied by the user. */
    GPNCache **cachePtr)	/* Where to cache the table, if the caller
				 * wants that. The contents are to be freed
				 * with Tcl_Free if the cache is used. */
{
    Tcl_Size objc, index, i;
    Tcl_Obj *listPtr = GetAllObjectProperties(
	    oPtr, flags & GPN_WRITABLE);
    Tcl_Obj **objv;
    GPNCache *tablePtr;

    (void) Tcl_ListObjGetElements(NULL, listPtr, &objc, &objv);
    if (cachePtr && *cachePtr) {
	tablePtr = *cachePtr;
    } else {
	tablePtr = (GPNCache *) TclStackAlloc(interp,
		offsetof(GPNCache, names) + sizeof(char *) * (objc + 1));

	for (i = 0; i < objc; i++) {
	    tablePtr->names[i] = TclGetString(objv[i]);
	}
	tablePtr->names[objc] = NULL;
	if (cachePtr) {
	    /*
	     * Have a cache, but nothing in it so far.
	     *
	     * We cache the list here so it doesn't vanish from under our
	     * feet if a property implementation does something crazy like
	     * changing the set of properties. The type of copy this does
	     * means that the copy holds the references to the names in the
	     * table.
	     */
	    tablePtr->listPtr = TclListObjCopy(NULL, listPtr);
	    Tcl_IncrRefCount(tablePtr->listPtr);
	    *cachePtr = tablePtr;
	} else {
	    tablePtr->listPtr = NULL;
	}
    }
    int result = Tcl_GetIndexFromObjStruct(interp, namePtr, tablePtr->names,
	    sizeof(char *), "property", TCL_INDEX_TEMP_TABLE, &index);
    if (result == TCL_ERROR && !(flags & GPN_FALLING_BACK)) {
	/*
	 * If property can be accessed the other way, use a special message.
	 * We use a recursive call to look this up.
	 */

	Tcl_InterpState state = Tcl_SaveInterpState(interp, result);
	Tcl_Obj *otherName = GetPropertyName(interp, oPtr,
		flags ^ (GPN_WRITABLE | GPN_FALLING_BACK), namePtr, NULL);
	result = Tcl_RestoreInterpState(interp, state);
	if (otherName != NULL) {
	    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		    "property \"%s\" is %s only",
		    TclGetString(otherName),
		    (flags & GPN_WRITABLE) ? "read" : "write"));
	}
    }
    if (!cachePtr) {
	TclStackFree(interp, tablePtr);
    }
    if (result != TCL_OK) {
	return NULL;
    }

```

tcl9.0 9.0.4, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOProp.c`, function `ReleasePropertyNameCache`, lines 236–249. Retained reference `source-window-5`.

```text
/* Release the cache made by GetPropertyName(). */
static inline void
ReleasePropertyNameCache(
    Tcl_Interp *interp,
    GPNCache **cachePtr)
{
    if (*cachePtr) {
	GPNCache *tablePtr = *cachePtr;
	if (tablePtr->listPtr) {
	    Tcl_DecrRefCount(tablePtr->listPtr);
	}
	TclStackFree(interp, tablePtr);
	*cachePtr = NULL;
    }

```

tcl9.0 9.0.4, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOProp.c`, function `TclOO_Configurable_Configure`, lines 254–298. Retained reference `source-window-6`.

```text
    TCL_UNUSED(void *),
    Tcl_Interp *interp,		/* Interpreter used for the result, error
				 * reporting, etc. */
    Tcl_ObjectContext context,	/* The object/call context. */
    int objc,			/* Number of arguments. */
    Tcl_Obj *const *objv)	/* The actual arguments. */
{
    Object *oPtr = (Object *) Tcl_ObjectContextObject(context);
    Tcl_Size skip = Tcl_ObjectContextSkippedArgs(context);
    Tcl_Obj *namePtr;
    Tcl_Size i, namec;
    int code = TCL_OK;

    objc -= skip;
    if ((objc & 1) && (objc != 1)) {
	/*
	 * Bad (odd > 1) number of arguments.
	 */

	Tcl_WrongNumArgs(interp, skip, objv, "?-option value ...?");
	return TCL_ERROR;
    }

    objv += skip;
    if (objc == 0) {
	/*
	 * Read all properties.
	 */

	Tcl_Obj *listPtr = GetAllObjectProperties(oPtr, 0);
	Tcl_Obj *resultPtr = Tcl_NewObj(), **namev;

	Tcl_IncrRefCount(listPtr);
	ListObjGetElements(listPtr, namec, namev);

	for (i = 0; i < namec; ) {
	    code = ReadProperty(interp, oPtr, TclGetString(namev[i]));
	    if (code != TCL_OK) {
		Tcl_DecrRefCount(resultPtr);
		break;
	    }
	    Tcl_DictObjPut(NULL, resultPtr, namev[i],
		    Tcl_GetObjResult(interp));
	    if (++i >= namec) {
		Tcl_SetObjResult(interp, resultPtr);

```

tcl9.0 9.0.4, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOProp.c`, function `TclOO_Configurable_Configure`, lines 305–355. Retained reference `source-window-7`.

```text
    } else if (objc == 1) {
	/*
	 * Read a single named property.
	 */

	namePtr = GetPropertyName(interp, oPtr, 0, objv[0], NULL);
	if (namePtr == NULL) {
	    return TCL_ERROR;
	}
	return ReadProperty(interp, oPtr, TclGetString(namePtr));
    } else if (objc == 2) {
	/*
	 * Special case for writing to one property. Saves fiddling with the
	 * cache in this common case.
	 */

	namePtr = GetPropertyName(interp, oPtr, GPN_WRITABLE, objv[0], NULL);
	if (namePtr == NULL) {
	    return TCL_ERROR;
	}
	code = WriteProperty(interp, oPtr, TclGetString(namePtr), objv[1]);
	if (code == TCL_OK) {
	    Tcl_ResetResult(interp);
	}
	return code;
    } else {
	/*
	 * Write properties. Slightly tricky because we want to cache the
	 * table of property names.
	 */
	GPNCache *cache = NULL;

	code = TCL_OK;
	for (i = 0; i < objc; i += 2) {
	    namePtr = GetPropertyName(interp, oPtr, GPN_WRITABLE, objv[i],
		    &cache);
	    if (namePtr == NULL) {
		code = TCL_ERROR;
		break;
	    }
	    code = WriteProperty(interp, oPtr, TclGetString(namePtr),
		    objv[i + 1]);
	    if (code != TCL_OK) {
		break;
	    }
	}
	if (code == TCL_OK) {
	    Tcl_ResetResult(interp);
	}
	ReleasePropertyNameCache(interp, &cache);
	return code;

```


## Consumer bindings

- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs), `lookup_property`: Selected software property lookup retains the old copied name table while mapping its selected index into the actual current property header. Actual currency/target/lifetime checks remain independent; public C9.0 values do not grant this C9.1 physical purpose.
- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs), `configure_all_original`: Selected software configure-all retains the actual working property header/member inventory across accessor calls. The native two-script output comparison does not certify callback Host/refusal cleanup or physical reference counts.
- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs), `configure_native_original`: Selected software multi-write carries a retained lookup table between actual sequential accessors; its issuer, actual object ownership and completion guards remain independent of printed result equality.
- [runtime/rust/src/cmd_oo/original_property_reporting_tests.rs](../../../../runtime/rust/src/cmd_oo/original_property_reporting_tests.rs), `cmd_oo::original_property_reporting_tests::materialized_property_reports_preserve_native_snapshot_and_table_index_roles` (linked): Two exact original configurable scripts and printed results/events/rosters supply a bounded software comparison under the actual C9.1 physical adapter purpose. C9.0 equal public results do not broaden that physical issuer. This body is unrun; independent sort/refusal/private-inventory controls and native pointer/refcount/lifetime equality are not inferred.
- [rust/tcl-vm/src/cmd_oo/native_properties/stage_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_properties/stage_tests.rs), `cmd_oo::native_properties::stage_tests::materialized_property_reports_preserve_native_snapshot_and_table_index_roles` (linked): Materialized property reports preserve native snapshot and table index roles. This is a current authored software control without an attached matching current owned assertion outcome. It grants no external-provider observation, Native callback/free chronology, physical reference/frame identity or execution purpose.

A named test is a coverage binding, not a claim that it executed.

## Replay

Exact original CLI argv and script/version inputs remain in the own receipts. Restore gzip ELF and whole-source copies to their recorded SHA/size before old replay. Wrappers select the original executable path/TCL_LIBRARY, but complete library/environment and configure/build replay inputs are absent. Initial generic-class scripts stop before the intended worker. No native/Rust replay is performed by publication; C9.0 public equality is independent of the C9.1 selected physical software contract.

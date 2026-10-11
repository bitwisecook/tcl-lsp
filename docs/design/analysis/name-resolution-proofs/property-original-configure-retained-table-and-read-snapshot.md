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

## Exact evidence

- `multi-write-configurable-input` (input): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/multi-write-configurable.tcl](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/multi-write-configurable.tcl). SHA-256 `ec3701e9be1a215a4deef521d3c3d1f166dd481d27889bb399476313c069be80`. Exact ordinary configurable-class input supplied to all six measured provider invocations. Public printed result, event order and roster are separate from native pointer/layout observations.
- `configure-all-configurable-input` (input): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configure-all-configurable.tcl](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configure-all-configurable.tcl). SHA-256 `f32e501e32060df1b2657732e66bf8f89b7f394b42e4e64e1b7e54c2e7b58290`. Exact ordinary configurable-class input supplied to all six measured provider invocations. Public printed result, event order and roster are separate from native pointer/layout observations.
- `matrix-original` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/matrix.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/matrix.json). SHA-256 `264239231a59437b8dd2099f2d4406f530d157dcbd3837dea011b8acba69cd3f`. Original twelve provider/case summaries with actual argv, input/wrapper hashes, exits/durations, raw stream hashes and independent version channels. Each original own receipt and raw stream is retained independently.
- `configurable-original-receipt` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configurable-receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configurable-receipt.json). SHA-256 `afe5a503a13596b541f10776b2fd700e48f52d16885ed19159c833d003ca74bf`. Two independently retained C9.1 original process records with the actual configurable producer. These are distinct from the twelve-case matrix invocations.
- `rejected-original-receipt` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/submitted-receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/submitted-receipt.json). SHA-256 `cb669feccbdc08d46bfcc356703332b7631f46f56f74d66936ab273f8d70007b`. Two exact C9.1 generic oo::class submissions close exit1 at invalid property command before the intended configure workers. Their setup refusal establishes no retained-table/read-snapshot result.
- `multi-write-submitted-tcl` (input): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/multi-write-submitted.tcl](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/multi-write-submitted.tcl). SHA-256 `8715c0fdf3ec71f863c7ebc1b7a22d7db9225e24ac45c071034769d773ce3905`. Exact original generic-class attempt bytes. The intended property configure worker is not reached; rejected setup must not supply a successful property observation.
- `multi-write-submitted-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/multi-write-submitted.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/multi-write-submitted.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original generic-class attempt bytes. The intended property configure worker is not reached; rejected setup must not supply a successful property observation.
- `multi-write-submitted-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/multi-write-submitted.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/multi-write-submitted.stderr). SHA-256 `82274ba2140b4296aeb226afc4054996baadf798e3101ed801f2a5ab2d61b1f8`. Exact original generic-class attempt bytes. The intended property configure worker is not reached; rejected setup must not supply a successful property observation.
- `configure-all-submitted-tcl` (input): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configure-all-submitted.tcl](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configure-all-submitted.tcl). SHA-256 `475701e60bc5ee3d0fd496bc4d0d4d75101b8d3fa7393e299e0e82b706a8040a`. Exact original generic-class attempt bytes. The intended property configure worker is not reached; rejected setup must not supply a successful property observation.
- `configure-all-submitted-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configure-all-submitted.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configure-all-submitted.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original generic-class attempt bytes. The intended property configure worker is not reached; rejected setup must not supply a successful property observation.
- `configure-all-submitted-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configure-all-submitted.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configure-all-submitted.stderr). SHA-256 `811a9e04c76050533be5b15716c2bb62ab6f8b76d7b7f20d56b854ed179db3e0`. Exact original generic-class attempt bytes. The intended property configure worker is not reached; rejected setup must not supply a successful property observation.
- `version-input` (input): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/version.tcl](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/version.tcl). SHA-256 `c150325350330227902317fba9520b1ceeb87194e7537137648cbd8c00d7c165`. Exact independent version/nameofexecutable script. This ordinary version channel does not measure native property memory or full build provenance.
- `provider-retention-original` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-retention.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-retention.json). SHA-256 `ea4ca508f4531f8823609f59a41a7d994854584f25cfe0330ed53c6964676ffc`. Original full invoked-wrapper/downstream-ELF and source joins. Six retained gzip executables independently round-trip to recorded full SHA and size. ldd outputs report loader dependencies without retained shared-library bytes or launch-time hashes; configure/build replay is absent.
- `tcl8.4-multi-write-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/receipt.json). SHA-256 `b5cb4f8f010b2c58b2074808b0ec9a64df3162aa00a8f5d005359133ec8c3841`. Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl8.4-multi-write-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl8.4-multi-write-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/stderr). SHA-256 `9543d12e63583969894d41c150234f8ff70e472e5593cc4f55ce4071a8cd09c4`. Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl8.4-multi-write-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/version.stdout). SHA-256 `068188ab11ced78c7f40d01dae26088440c13e92e4f7ecb8500848d9327722fd`. Exact independent printed version and executable path from this original invocation.
- `tcl8.4-multi-write-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/multi-write-configurable/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact independent version-channel stderr, retained empty.
- `tcl8.4-configure-all-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/receipt.json). SHA-256 `9b1eea9d1437a095e7437b3e1767c26ac72e61d89d1073137f9969c10300404f`. Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl8.4-configure-all-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl8.4-configure-all-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/stderr). SHA-256 `32ab00fd10205660070f4e1eb3f3d0bef4241eb47fca342883e2b848d503a541`. Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl8.4-configure-all-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/version.stdout). SHA-256 `068188ab11ced78c7f40d01dae26088440c13e92e4f7ecb8500848d9327722fd`. Exact independent printed version and executable path from this original invocation.
- `tcl8.4-configure-all-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/84/configure-all-configurable/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact independent version-channel stderr, retained empty.
- `tcl8.4-executable-elf-gz` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/executable.elf.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/executable.elf.gz). SHA-256 `0ef35bc55b478afbee73fdeaad303290df5449989182a55e9535b339d8ed1f28`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.4-ldd-stdout` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/ldd.stdout). SHA-256 `322fce485c073c190020aa0b1592e59e8da792787e87eef2ab643f7b6826f7eb`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.4-ldd-stderr` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/ldd.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.4-wrapper-sh` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/wrapper.sh](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/wrapper.sh). SHA-256 `b100108bc3747ea4fa6a4411a28756c26c8c201390a1983ae35443ce7bece817`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.5-multi-write-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/receipt.json). SHA-256 `f83dacebe1ed4aec5e3fbd0dd91ad017f640dd45fcff56ef9c4b9359b440ad2a`. Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl8.5-multi-write-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl8.5-multi-write-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/stderr). SHA-256 `9543d12e63583969894d41c150234f8ff70e472e5593cc4f55ce4071a8cd09c4`. Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl8.5-multi-write-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/version.stdout). SHA-256 `259a49bd971e95b8fc49e6ac8781be0003b69017f397c279c80f1f7dc24db0ca`. Exact independent printed version and executable path from this original invocation.
- `tcl8.5-multi-write-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/multi-write-configurable/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact independent version-channel stderr, retained empty.
- `tcl8.5-configure-all-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/receipt.json). SHA-256 `c762f4438e96dec27b9d4a3fafd96f936ec97c2adaaba2924670d4c765d4a75c`. Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl8.5-configure-all-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl8.5-configure-all-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/stderr). SHA-256 `32ab00fd10205660070f4e1eb3f3d0bef4241eb47fca342883e2b848d503a541`. Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl8.5-configure-all-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/version.stdout). SHA-256 `259a49bd971e95b8fc49e6ac8781be0003b69017f397c279c80f1f7dc24db0ca`. Exact independent printed version and executable path from this original invocation.
- `tcl8.5-configure-all-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/85/configure-all-configurable/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact independent version-channel stderr, retained empty.
- `tcl8.5-executable-elf-gz` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/executable.elf.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/executable.elf.gz). SHA-256 `47cba54ad641a54ba11e204d74a5ce4232e847fc198142f5a4846078ec5c6c58`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.5-ldd-stdout` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/ldd.stdout). SHA-256 `889f87ad90d1610bb864bbf25706a6d801265f0ecd7a9381c97ba943b58aa783`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.5-ldd-stderr` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/ldd.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.5-wrapper-sh` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/wrapper.sh](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/wrapper.sh). SHA-256 `6b2005f227b608208c8bfdf472d5d6c1ed9595ad10f4b7442d5ee1e23e9e400b`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.6-multi-write-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/receipt.json). SHA-256 `4ced5f30e40f34ca9c49c6a4415407b95c487fcf0b5e43d7a0293d5bf898f847`. Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl8.6-multi-write-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl8.6-multi-write-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/stderr). SHA-256 `9543d12e63583969894d41c150234f8ff70e472e5593cc4f55ce4071a8cd09c4`. Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl8.6-multi-write-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/version.stdout). SHA-256 `a81129b21e3b9f356fdcce11e8283263ddef932c26801d5356865d71bfa5cbbc`. Exact independent printed version and executable path from this original invocation.
- `tcl8.6-multi-write-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/multi-write-configurable/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact independent version-channel stderr, retained empty.
- `tcl8.6-configure-all-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/receipt.json). SHA-256 `c7ad100405a75b5f2f66240197e523869a6821bc6d57b993f0f7731aad4732d6`. Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl8.6-configure-all-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl8.6-configure-all-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/stderr). SHA-256 `32ab00fd10205660070f4e1eb3f3d0bef4241eb47fca342883e2b848d503a541`. Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl8.6-configure-all-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/version.stdout). SHA-256 `a81129b21e3b9f356fdcce11e8283263ddef932c26801d5356865d71bfa5cbbc`. Exact independent printed version and executable path from this original invocation.
- `tcl8.6-configure-all-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/86/configure-all-configurable/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact independent version-channel stderr, retained empty.
- `tcl8.6-executable-elf-gz` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/executable.elf.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/executable.elf.gz). SHA-256 `7f604661926c1c5237e7423365f39c9ffda3bd849441190a130d903f046a8ba8`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.6-ldd-stdout` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/ldd.stdout). SHA-256 `16279183390cbadc66886bce49b5d5f2e6e37e39c6a71ce1ea4f14adbc184cc5`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.6-ldd-stderr` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/ldd.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl8.6-wrapper-sh` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/wrapper.sh](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/wrapper.sh). SHA-256 `2e103024652a7d81f5496a979464fe0acad87a9033e798305be231ef484c5c5a`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl9.0-multi-write-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/receipt.json). SHA-256 `a55329f4d45cda0f5c6b170c6f68f2f4998994959ee07c3127efc008d9e1ed7a`. Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl9.0-multi-write-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/stdout). SHA-256 `3895d8ce2308703009139185084af03dee5dfc6e7776f4b9e1e0df64de34afcc`. Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl9.0-multi-write-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl9.0-multi-write-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/version.stdout). SHA-256 `69eee34205bb624bf438509249dfaa4b1378791c3999e0dd58c00be6ba8158e3`. Exact independent printed version and executable path from this original invocation.
- `tcl9.0-multi-write-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/multi-write-configurable/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact independent version-channel stderr, retained empty.
- `tcl9.0-configure-all-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/receipt.json). SHA-256 `2e2afea092a3e794c1150d6c81716c7d6a9f675dc2f5dfd15159a6c5578fdeea`. Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl9.0-configure-all-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/stdout). SHA-256 `d144c19f27db833224ec594d22af0aeba951367d98ac4fa4772dd16c9ea5c309`. Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl9.0-configure-all-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl9.0-configure-all-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/version.stdout). SHA-256 `69eee34205bb624bf438509249dfaa4b1378791c3999e0dd58c00be6ba8158e3`. Exact independent printed version and executable path from this original invocation.
- `tcl9.0-configure-all-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/90/configure-all-configurable/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact independent version-channel stderr, retained empty.
- `tcl9.0-executable-elf-gz` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/executable.elf.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/executable.elf.gz). SHA-256 `efd77c98ed7007ad21ec1b50feb70b2533301fa372379cc17f17ce85333cea3a`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl9.0-ldd-stdout` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/ldd.stdout). SHA-256 `0eaff2aa38de4eb9b9dfd0b8cdb9847c3abe24cfffe0825b26bc4356844d8ebf`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl9.0-ldd-stderr` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/ldd.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl9.0-wrapper-sh` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/wrapper.sh](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/wrapper.sh). SHA-256 `f55225fd9e74448b244b90f41d3dce2989fc7e205bee4f2f0d4a08057880e668`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl9.1-multi-write-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/receipt.json). SHA-256 `52fea2d702c77bccebbcaa9e5c88a64c00644d2eb2bdbf789fd896692099609b`. Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl9.1-multi-write-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/stdout). SHA-256 `3895d8ce2308703009139185084af03dee5dfc6e7776f4b9e1e0df64de34afcc`. Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl9.1-multi-write-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl9.1-multi-write-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/version.stdout). SHA-256 `8b89a9cc4b86d71b504dc58191a433a1226a5bd0f407cd3e3c0b6606f7e96f11`. Exact independent printed version and executable path from this original invocation.
- `tcl9.1-multi-write-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/multi-write-configurable/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact independent version-channel stderr, retained empty.
- `tcl9.1-configure-all-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/receipt.json). SHA-256 `0adc0d57c6fb9063a6e171dc9ff15ddbe48b809128643056eaab57673da46389`. Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `tcl9.1-configure-all-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/stdout). SHA-256 `d144c19f27db833224ec594d22af0aeba951367d98ac4fa4772dd16c9ea5c309`. Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `tcl9.1-configure-all-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `tcl9.1-configure-all-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/version.stdout). SHA-256 `8b89a9cc4b86d71b504dc58191a433a1226a5bd0f407cd3e3c0b6606f7e96f11`. Exact independent printed version and executable path from this original invocation.
- `tcl9.1-configure-all-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/91/configure-all-configurable/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact independent version-channel stderr, retained empty.
- `tcl9.1-executable-elf-gz` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/executable.elf.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/executable.elf.gz). SHA-256 `fc6c12552b3f4c0e1e5d023322046330cecaafa80f3fb45d46727cf1261262a2`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl9.1-ldd-stdout` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/ldd.stdout). SHA-256 `24871b07178963e8dc2df2ae1a86a7e5a6ae7e45944f2de2e79961852dc286ac`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl9.1-ldd-stderr` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/ldd.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `tcl9.1-wrapper-sh` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/wrapper.sh](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/wrapper.sh). SHA-256 `c5638d48d9a54d3a0d0901a4dc94698f8e018715473f8f6c2e3264408694ce47`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `jim-multi-write-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/receipt.json). SHA-256 `5a30b1b1ca50c31c85d11d6b6d14e2371e3959db052d7a3ee578840d9e63e33e`. Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `jim-multi-write-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `jim-multi-write-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/stderr). SHA-256 `91daf4b9065a360f5cb67415b12004217807028447ecb8d5f0937be3b61e5a06`. Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `jim-multi-write-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/version.stdout). SHA-256 `99b5bb7754238fcb794ac3b8e264e28f187a60f3042185ad32824c06f315853a`. Exact independent printed version and executable path from this original invocation.
- `jim-multi-write-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/multi-write-configurable/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact independent version-channel stderr, retained empty.
- `jim-configure-all-configurable-receipt-json` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/receipt.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/receipt.json). SHA-256 `96949be436959f3bf80cf4a9c1ac9ab2c98908ac72824c14cff7a0cca0ae5ada`. Own original argv/input/invoked-file identity, closed exit/duration and whole raw stream hashes; no before/after full environment or reproducible-build closure.
- `jim-configure-all-configurable-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original public script stdout. C9 successful printed events/result/roster are observable values only; older/Jim stdout is empty after producer refusal.
- `jim-configure-all-configurable-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/stderr). SHA-256 `ba8c5b6f5f11b1aad7a095c8ce1b867155047ec4370e6727a8b994555eb3889f`. Whole original stderr: empty for successful C9 scripts; older/Jim invalid oo::configurable setup refusal precedes the intended configure worker.
- `jim-configure-all-configurable-version-stdout` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/version.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/version.stdout). SHA-256 `99b5bb7754238fcb794ac3b8e264e28f187a60f3042185ad32824c06f315853a`. Exact independent printed version and executable path from this original invocation.
- `jim-configure-all-configurable-version-stderr` (observation): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/version.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/jim/configure-all-configurable/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact independent version-channel stderr, retained empty.
- `jim-executable-elf-gz` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/executable.elf.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/executable.elf.gz). SHA-256 `4bb1e19e71a50cfeaf339019c06cb10c52ce4110a385229e99c95f89e92be8ea`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `jim-ldd-stdout` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/ldd.stdout). SHA-256 `4429c9cfdc881a14d6b2a1d86cc3d7cca4367f991b6ef145bcca954ec4eb25a9`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `jim-ldd-stderr` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/ldd.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Retained original provider file. Whole ELF gzip decompresses to the original recorded SHA/size; wrappers match invocation hashes. ldd text lists reported dependencies only and supplies no loaded-address, library-byte or ABI/build correspondence.
- `source-window-0` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.1.0-274-343.c](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.1.0-274-343.c). SHA-256 `343a1ba048fc015da66373af15de4e9a6d99d4fe96fff45de55b38242fa7249c`. Exact physical-LF source excerpt rejoined to retained whole tclOOProp.c SHA. It inspects current-member lookup, copied old table List/cache release, or configure-all retained working List. C9.0 source joins and printed values do not issue the software adapter C9.1 physical purpose.
- `source-window-1` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.1.0-347-360.c](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.1.0-347-360.c). SHA-256 `6104be04b61fb5e6243ad46ce99e4c414e3d9d421e7932b83c2b18138f483c36`. Exact physical-LF source excerpt rejoined to retained whole tclOOProp.c SHA. It inspects current-member lookup, copied old table List/cache release, or configure-all retained working List. C9.0 source joins and printed values do not issue the software adapter C9.1 physical purpose.
- `source-window-2` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.1.0-365-409.c](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.1.0-365-409.c). SHA-256 `c97921ab14cabd97832031611208646c5a4ada47cc748732bc16fc8b8d6d542b`. Exact physical-LF source excerpt rejoined to retained whole tclOOProp.c SHA. It inspects current-member lookup, copied old table List/cache release, or configure-all retained working List. C9.0 source joins and printed values do not issue the software adapter C9.1 physical purpose.
- `source-window-3` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.1.0-416-466.c](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.1.0-416-466.c). SHA-256 `38c1be425b81bb92d3bb125434474674da38459769b0cf220c59aaf29ecc53ac`. Exact physical-LF source excerpt rejoined to retained whole tclOOProp.c SHA. It inspects current-member lookup, copied old table List/cache release, or configure-all retained working List. C9.0 source joins and printed values do not issue the software adapter C9.1 physical purpose.
- `source-window-4` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.0.4-163-232.c](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.0.4-163-232.c). SHA-256 `01f8eaa50b99a7223632096ce6176a5dae8786ad8ac91d58823e256699c2fdae`. Exact physical-LF source excerpt rejoined to retained whole tclOOProp.c SHA. It inspects current-member lookup, copied old table List/cache release, or configure-all retained working List. C9.0 source joins and printed values do not issue the software adapter C9.1 physical purpose.
- `source-window-5` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.0.4-236-249.c](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.0.4-236-249.c). SHA-256 `6104be04b61fb5e6243ad46ce99e4c414e3d9d421e7932b83c2b18138f483c36`. Exact physical-LF source excerpt rejoined to retained whole tclOOProp.c SHA. It inspects current-member lookup, copied old table List/cache release, or configure-all retained working List. C9.0 source joins and printed values do not issue the software adapter C9.1 physical purpose.
- `source-window-6` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.0.4-254-298.c](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.0.4-254-298.c). SHA-256 `6a042e426c201fc8e00149facae37076cbfa9f0fe91bec3150dd572364d8f665`. Exact physical-LF source excerpt rejoined to retained whole tclOOProp.c SHA. It inspects current-member lookup, copied old table List/cache release, or configure-all retained working List. C9.0 source joins and printed values do not issue the software adapter C9.1 physical purpose.
- `source-window-7` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.0.4-305-355.c](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/source-windows/tcl9.0.4-305-355.c). SHA-256 `43b5eefd3b3b6d1776c8c1d226125349ba46b8ea273e24f6ea7ce2d2040a7a6d`. Exact physical-LF source excerpt rejoined to retained whole tclOOProp.c SHA. It inspects current-member lookup, copied old table List/cache release, or configure-all retained working List. C9.0 source joins and printed values do not issue the software adapter C9.1 physical purpose.
- `tcl9.0-whole-property-source` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/tclOOProp.c.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/tclOOProp.c.gz). SHA-256 `7c91177bd05c0cffa3fee909749df371b089b10ac2a6a00267118c7b230f0cc9`. Lossless complete original tclOOProp.c bytes identified by provider-retention.json; exact physical-LF excerpts are selected independently. Inspected source does not prove that a binary was built from these bytes.
- `tcl9.1-whole-property-source` (source-anchor): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/tclOOProp.c.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/tclOOProp.c.gz). SHA-256 `794ef09bb04b32ddd5a7a12aabb5542b43414197cd86572f5de1d0877d791438`. Lossless complete original tclOOProp.c bytes identified by provider-retention.json; exact physical-LF excerpts are selected independently. Inspected source does not prove that a binary was built from these bytes.
- `current-source-0` (implementation): [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs). SHA-256 `c842e5b8a918739a5f00522b577dd58a4b82033f96addc6e97788fb7704e300e`. Current software owner or actual parent module registration. The source definition has no attached matching owned Rust assertion result; the public C9 prints do not establish its physical cache/lifetime behavior.
- `current-source-1` (implementation): [runtime/rust/src/cmd_oo/original_property_reporting_tests.rs](../../../../runtime/rust/src/cmd_oo/original_property_reporting_tests.rs). SHA-256 `f67448eddff6e99cc9bf22de5c675882100cd50e46dfb58ccc79f1b7d1cca51d`. Current software owner or actual parent module registration. The source definition has no attached matching owned Rust assertion result; the public C9 prints do not establish its physical cache/lifetime behavior.
- `current-source-2` (implementation): [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs). SHA-256 `dcd59fb5b35890c1ccc6e779dbecbf33231a991f802ed4823d21720c77db780c`. Current software owner or actual parent module registration. The source definition has no attached matching owned Rust assertion result; the public C9 prints do not establish its physical cache/lifetime behavior.

## Source inspection

tcl9.1 9.1.0, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `GetPropertyName`, lines 274–343. Full-source SHA-256 `c791423ae9b49c877bea8da8a7c2b74b40520e600863341e8d12a63b10c2aaaf`; snippet SHA-256 `343a1ba048fc015da66373af15de4e9a6d99d4fe96fff45de55b38242fa7249c`; retained evidence `source-window-0`.

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

tcl9.1 9.1.0, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `ReleasePropertyNameCache`, lines 347–360. Full-source SHA-256 `c791423ae9b49c877bea8da8a7c2b74b40520e600863341e8d12a63b10c2aaaf`; snippet SHA-256 `6104be04b61fb5e6243ad46ce99e4c414e3d9d421e7932b83c2b18138f483c36`; retained evidence `source-window-1`.

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

tcl9.1 9.1.0, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `TclOO_Configurable_Configure`, lines 365–409. Full-source SHA-256 `c791423ae9b49c877bea8da8a7c2b74b40520e600863341e8d12a63b10c2aaaf`; snippet SHA-256 `c97921ab14cabd97832031611208646c5a4ada47cc748732bc16fc8b8d6d542b`; retained evidence `source-window-2`.

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

tcl9.1 9.1.0, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOOProp.c`, function `TclOO_Configurable_Configure`, lines 416–466. Full-source SHA-256 `c791423ae9b49c877bea8da8a7c2b74b40520e600863341e8d12a63b10c2aaaf`; snippet SHA-256 `38c1be425b81bb92d3bb125434474674da38459769b0cf220c59aaf29ecc53ac`; retained evidence `source-window-3`.

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

tcl9.0 9.0.4, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOProp.c`, function `GetPropertyName`, lines 163–232. Full-source SHA-256 `43f16f2b596af56b27320ff5e735c3073e081dbf183b05b5aa785ff7a2b57789`; snippet SHA-256 `01f8eaa50b99a7223632096ce6176a5dae8786ad8ac91d58823e256699c2fdae`; retained evidence `source-window-4`.

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

tcl9.0 9.0.4, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOProp.c`, function `ReleasePropertyNameCache`, lines 236–249. Full-source SHA-256 `43f16f2b596af56b27320ff5e735c3073e081dbf183b05b5aa785ff7a2b57789`; snippet SHA-256 `6104be04b61fb5e6243ad46ce99e4c414e3d9d421e7932b83c2b18138f483c36`; retained evidence `source-window-5`.

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

tcl9.0 9.0.4, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOProp.c`, function `TclOO_Configurable_Configure`, lines 254–298. Full-source SHA-256 `43f16f2b596af56b27320ff5e735c3073e081dbf183b05b5aa785ff7a2b57789`; snippet SHA-256 `6a042e426c201fc8e00149facae37076cbfa9f0fe91bec3150dd572364d8f665`; retained evidence `source-window-6`.

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

tcl9.0 9.0.4, revision `Configured original source tree joined by complete SHA; configure/build processes have no replay receipts`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOOProp.c`, function `TclOO_Configurable_Configure`, lines 305–355. Full-source SHA-256 `43f16f2b596af56b27320ff5e735c3073e081dbf183b05b5aa785ff7a2b57789`; snippet SHA-256 `43b5eefd3b3b6d1776c8c1d226125349ba46b8ea273e24f6ea7ce2d2040a7a6d`; retained evidence `source-window-7`.

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

A named test is a coverage binding, not a claim that it executed.

## Replay

Exact original CLI argv and script/version inputs remain in the own receipts. Restore gzip ELF and whole-source copies to their recorded SHA/size before old replay. Wrappers select the original executable path/TCL_LIBRARY, but complete library/environment and configure/build replay inputs are absent. Initial generic-class scripts stop before the intended worker. No native/Rust replay is performed by publication; C9.0 public equality is independent of the C9.1 selected physical software contract.

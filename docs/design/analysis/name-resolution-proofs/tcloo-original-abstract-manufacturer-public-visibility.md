# naming.tcloo.original-abstract-manufacturer-public-visibility

Kind: `native-observation`

## Problem statement

An abstract metaclass can hide public manufacturer methods without replacing the inherited constructor machinery. Treating failed public Abstract new/create calls as a universal constructor-handler prohibition also mispredicts a concrete subclass or an explicit object-level export.

## Question

For the exact original script, which manufacturer methods are publicly visible on an oo::abstract instance, do new/create/createWithNamespace public calls fail, can a concrete subclass create an instance, and does explicit object export new restore public Abstract new?

## Conclusion

Tcl 9.0.4 and 9.1.0 each close the exact script exit0 with empty stderr: the three manufacturers are private, their public calls return unknown-method code1, a concrete subclass creates normally, and explicitly exporting new enables a public Abstract new with code0. Tcl 8.4.20, 8.5.19, 8.6.18 and Jim 0.84-9-g5bac7c9 stop at unavailable oo::abstract before the intended visibility work. The retained source unexports those names and the manual describes inherited construction; neither printed values nor inspection establish constructor-handler execution, Method/payload identity, physical frames, Native admission or Rust outcomes. BIG-IP is not tested.

## Scope

Six independent ordinary-script observations and six independent version channels; complete original argv/input hashes, exit/duration and stdout/stderr are retained. Two C9 scripts reach the declared visibility work, while four older/Jim scripts measure producer unavailability only. Actual invoked-file/downstream ELF identities reuse the independently whole-hash-joined property335 canonical payloads, without duplicating executables. Reported ldd dependencies lack library-byte and inherited full environment/build closure. Four retained whole source/manual files and exact physical-LF excerpts inspect public unexport declarations independently. No constructor callback chronology, internal error-formatting stop, native header/refcount/Method/Scope/frame identity, BIG-IP/TMM, software assertion or implementation equivalence is measured.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Original invoked wrapper or Jim ELF and downstream executable match the exact shared property335 retention hashes, independently recorded by this manufacturer receipt. Whole ELF gzip restoration is verified. Version output is retained; reported ldd dependencies do not close shared-library bytes, inherited environment or configure/build provenance.. Channel: One exact ordinary script file and independent version script through each recorded CLI argv; whole stdout/stderr and own process receipts.. Dialect: tcl8.4.

The exact script closes exit1 with empty stdout at invalid command name "oo::abstract". Its intended abstract manufacturer visibility and explicit-export work is not reached; this response measures only producer unavailability for the actual invocation.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original invoked wrapper or Jim ELF and downstream executable match the exact shared property335 retention hashes, independently recorded by this manufacturer receipt. Whole ELF gzip restoration is verified. Version output is retained; reported ldd dependencies do not close shared-library bytes, inherited environment or configure/build provenance.. Channel: One exact ordinary script file and independent version script through each recorded CLI argv; whole stdout/stderr and own process receipts.. Dialect: tcl8.5.

The exact script closes exit1 with empty stdout at invalid command name "oo::abstract". Its intended abstract manufacturer visibility and explicit-export work is not reached; this response measures only producer unavailability for the actual invocation.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original invoked wrapper or Jim ELF and downstream executable match the exact shared property335 retention hashes, independently recorded by this manufacturer receipt. Whole ELF gzip restoration is verified. Version output is retained; reported ldd dependencies do not close shared-library bytes, inherited environment or configure/build provenance.. Channel: One exact ordinary script file and independent version script through each recorded CLI argv; whole stdout/stderr and own process receipts.. Dialect: tcl8.6.

The exact script closes exit1 with empty stdout at invalid command name "oo::abstract". Its intended abstract manufacturer visibility and explicit-export work is not reached; this response measures only producer unavailability for the actual invocation.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original invoked wrapper or Jim ELF and downstream executable match the exact shared property335 retention hashes, independently recorded by this manufacturer receipt. Whole ELF gzip restoration is verified. Version output is retained; reported ldd dependencies do not close shared-library bytes, inherited environment or configure/build provenance.. Channel: One exact ordinary script file and independent version script through each recorded CLI argv; whole stdout/stderr and own process receipts.. Dialect: tcl9.0.

The exact script closes exit0 with empty stderr. Abstract initially publishes only destroy; new, create and createWithNamespace remain in its private list and each public attempt returns code1/unknown method. A concrete subclass creates its own instance with code0. Explicit oo::objdefine Abstract export new then permits public Abstract new with code0, and the public list becomes {destroy new}. These values measure public visibility and dispatch results, not constructor-handler entry or physical Method/frame/lifetime identity.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original invoked wrapper or Jim ELF and downstream executable match the exact shared property335 retention hashes, independently recorded by this manufacturer receipt. Whole ELF gzip restoration is verified. Version output is retained; reported ldd dependencies do not close shared-library bytes, inherited environment or configure/build provenance.. Channel: One exact ordinary script file and independent version script through each recorded CLI argv; whole stdout/stderr and own process receipts.. Dialect: tcl9.1.

The exact script closes exit0 with empty stderr. Abstract initially publishes only destroy; new, create and createWithNamespace remain in its private list and each public attempt returns code1/unknown method. A concrete subclass creates its own instance with code0. Explicit oo::objdefine Abstract export new then permits public Abstract new with code0, and the public list becomes {destroy new}. These values measure public visibility and dispatch results, not constructor-handler entry or physical Method/frame/lifetime identity.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Original invoked wrapper or Jim ELF and downstream executable match the exact shared property335 retention hashes, independently recorded by this manufacturer receipt. Whole ELF gzip restoration is verified. Version output is retained; reported ldd dependencies do not close shared-library bytes, inherited environment or configure/build provenance.. Channel: One exact ordinary script file and independent version script through each recorded CLI argv; whole stdout/stderr and own process receipts.. Dialect: jim.

The exact script closes exit1 with empty stdout at invalid command name "oo::abstract". Its intended abstract manufacturer visibility and explicit-export work is not reached; this response measures only producer unavailability for the actual invocation.

### bigip

Status: `not-tested`. Version: not tested. Build: No original BIG-IP abstract-manufacturer invocation. Channel: No BIG-IP script response for this question. Dialect: bigip.

No original BIG-IP response is attached. Authored profiles and these local C/Jim results establish no BIG-IP/TMM manufacturer visibility or constructor behavior.

## Historical references

- `exact-manufacturer-input` (input): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/manufacturer-visibility.tcl](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/manufacturer-visibility.tcl). Exact ordinary script measuring initial public/private manufacturer visibility, public errors, a concrete subclass and explicit object export new. It does not inspect Method/clientData, constructor-handler entry or physical frames.
- `exact-version-input` (input): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/version.tcl](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/version.tcl). Exact independent script that prints patchlevel and executable path for each original invocation.
- `six-original-matrix` (provider): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/matrix.json](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/matrix.json). Six independent original script/version process records with exact argv/input/invocation-file identity, exits/durations and raw stream hashes. Older/Jim intended oo::abstract work is not reached.
- `shared-original-provider-binding` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-retention.json](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-retention.json). Exact original property335 provider dataset independently joined by whole hash in every manufacturer336 receipt. It retains original wrappers/downstream ELFs and ldd reports; the original manufacturer receipt mapping is retained unchanged. No independent shared-library byte closure or configure/build replay is claimed.
- `tcl8.4-receipt-json` (provider): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/84/receipt.json](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/84/receipt.json). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl8.4-stdout` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/84/stdout](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/84/stdout). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl8.4-stderr` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/84/stderr](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/84/stderr). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl8.4-version-stdout` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/84/version.stdout](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/84/version.stdout). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl8.4-version-stderr` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/84/version.stderr](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/84/version.stderr). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl8.4-shared-executable-elf-gz` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/executable.elf.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/executable.elf.gz). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl8.4-shared-ldd-stdout` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/ldd.stdout). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl8.4-shared-ldd-stderr` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/ldd.stderr). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl8.4-shared-wrapper-sh` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/wrapper.sh](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/84/wrapper.sh). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl8.5-receipt-json` (provider): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/85/receipt.json](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/85/receipt.json). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl8.5-stdout` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/85/stdout](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/85/stdout). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl8.5-stderr` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/85/stderr](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/85/stderr). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl8.5-version-stdout` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/85/version.stdout](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/85/version.stdout). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl8.5-version-stderr` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/85/version.stderr](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/85/version.stderr). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl8.5-shared-executable-elf-gz` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/executable.elf.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/executable.elf.gz). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl8.5-shared-ldd-stdout` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/ldd.stdout). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl8.5-shared-ldd-stderr` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/ldd.stderr). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl8.5-shared-wrapper-sh` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/wrapper.sh](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/85/wrapper.sh). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl8.6-receipt-json` (provider): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/86/receipt.json](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/86/receipt.json). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl8.6-stdout` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/86/stdout](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/86/stdout). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl8.6-stderr` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/86/stderr](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/86/stderr). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl8.6-version-stdout` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/86/version.stdout](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/86/version.stdout). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl8.6-version-stderr` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/86/version.stderr](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/86/version.stderr). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl8.6-shared-executable-elf-gz` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/executable.elf.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/executable.elf.gz). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl8.6-shared-ldd-stdout` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/ldd.stdout). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl8.6-shared-ldd-stderr` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/ldd.stderr). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl8.6-shared-wrapper-sh` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/wrapper.sh](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/86/wrapper.sh). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl9.0-receipt-json` (provider): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/90/receipt.json](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/90/receipt.json). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl9.0-stdout` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/90/stdout](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/90/stdout). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl9.0-stderr` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/90/stderr](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/90/stderr). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl9.0-version-stdout` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/90/version.stdout](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/90/version.stdout). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl9.0-version-stderr` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/90/version.stderr](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/90/version.stderr). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl9.0-shared-executable-elf-gz` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/executable.elf.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/executable.elf.gz). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl9.0-shared-ldd-stdout` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/ldd.stdout). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl9.0-shared-ldd-stderr` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/ldd.stderr). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl9.0-shared-wrapper-sh` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/wrapper.sh](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/90/wrapper.sh). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl9.1-receipt-json` (provider): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/91/receipt.json](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/91/receipt.json). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl9.1-stdout` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/91/stdout](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/91/stdout). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl9.1-stderr` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/91/stderr](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/91/stderr). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl9.1-version-stdout` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/91/version.stdout](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/91/version.stdout). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl9.1-version-stderr` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/91/version.stderr](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/91/version.stderr). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `tcl9.1-shared-executable-elf-gz` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/executable.elf.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/executable.elf.gz). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl9.1-shared-ldd-stdout` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/ldd.stdout). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl9.1-shared-ldd-stderr` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/ldd.stderr). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `tcl9.1-shared-wrapper-sh` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/wrapper.sh](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/91/wrapper.sh). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `jim-receipt-json` (provider): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/jim/receipt.json](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/jim/receipt.json). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `jim-stdout` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/jim/stdout](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/jim/stdout). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `jim-stderr` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/jim/stderr](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/jim/stderr). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `jim-version-stdout` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/jim/version.stdout](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/jim/version.stdout). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `jim-version-stderr` (observation): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/jim/version.stderr](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/jim/version.stderr). Whole own original process receipt or raw stream. Public script values/errors and version output remain separate from internal native Method/constructor/cache/frame behavior.
- `jim-shared-executable-elf-gz` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/executable.elf.gz](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/executable.elf.gz). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `jim-shared-ldd-stdout` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/ldd.stdout](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/ldd.stdout). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `jim-shared-ldd-stderr` (provider): [runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/ldd.stderr](../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/provider-files/jim/ldd.stderr). Shared exact retained provider payload independently joined by the original manufacturer receipt. Gzip full ELF SHA/size matches provider-retention.json; ldd is a reported dependency list without retained library-byte or load-address closure.
- `whole-source-retention` (provider): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/retained-source.json](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/retained-source.json). Exact original full-source gzip mapping for C9.0/C9.1 tclOO.c and abstract.n. Whole decompressed SHA/size and physical-LF snippets are verified independently; inspected source/documentation is not binary/source build equivalence.
- `whole-source-0` (source-anchor): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/source/90/generic/tclOO.c.gz](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/source/90/generic/tclOO.c.gz). Whole original source or manual bytes retained in lossless gzip, matched to original complete SHA and byte count.
- `source-window-0` (source-anchor): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/source-windows/source-0.txt](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/source-windows/source-0.txt). Exact physical-LF excerpt from the retained full-source SHA. The code explicitly unexports three manufacturers; the manual describes visibility and inherited construction. It supplies no execution, Method payload lifetime or physical frame grant.
- `whole-source-1` (source-anchor): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/source/90/doc/abstract.n.gz](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/source/90/doc/abstract.n.gz). Whole original source or manual bytes retained in lossless gzip, matched to original complete SHA and byte count.
- `source-window-1` (source-anchor): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/source-windows/source-1.txt](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/source-windows/source-1.txt). Exact physical-LF excerpt from the retained full-source SHA. The code explicitly unexports three manufacturers; the manual describes visibility and inherited construction. It supplies no execution, Method payload lifetime or physical frame grant.
- `whole-source-2` (source-anchor): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/source/91/generic/tclOO.c.gz](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/source/91/generic/tclOO.c.gz). Whole original source or manual bytes retained in lossless gzip, matched to original complete SHA and byte count.
- `source-window-2` (source-anchor): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/source-windows/source-2.txt](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/source-windows/source-2.txt). Exact physical-LF excerpt from the retained full-source SHA. The code explicitly unexports three manufacturers; the manual describes visibility and inherited construction. It supplies no execution, Method payload lifetime or physical frame grant.
- `whole-source-3` (source-anchor): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/source/91/doc/abstract.n.gz](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/source/91/doc/abstract.n.gz). Whole original source or manual bytes retained in lossless gzip, matched to original complete SHA and byte count.
- `source-window-3` (source-anchor): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/source-windows/source-3.txt](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/source-windows/source-3.txt). Exact physical-LF excerpt from the retained full-source SHA. The code explicitly unexports three manufacturers; the manual describes visibility and inherited construction. It supplies no execution, Method payload lifetime or physical frame grant.
- `canonical-provider-joins` (provider): [runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/canonical-provider-joins.json](../../../../runtime/rust/tests/data/native_abstract_manufacturer_public_visibility_original/336/canonical-provider-joins.json). Explicit mapping from unchanged original relative receipt bindings to shared canonical property335 payloads, including exact packed/original ELF SHA and size restoration joins.

- `own-source347-software-outcomes` (observation): [docs/design/analysis/name-resolution-proofs/rust-validation/compiler-core-runtime-owned347-experiments.json](../../../../docs/design/analysis/name-resolution-proofs/rust-validation/compiler-core-runtime-owned347-experiments.json). JSON pointer `/exact_question_body_outcomes`. Exact original-source347 body/question joins, closed owning producers, strict actual inventories/admissions and per-selector results. Current implementation source and unselected/new controls receive no result from these images.

## Source inspection

tcl9.0 9.0.4, revision `Configured source tree joined by complete SHA; configure/build replay is unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.c`, function `MakeAdditionalClasses`, lines 701–713. Retained reference `source-window-0`.

```text
    /*
     * Make the oo::abstract class.
     */

    abstractCls = (Object *) Tcl_NewObjectInstance(interp,
	    (Tcl_Class) fPtr->classCls, "::oo::abstract",
	    NULL, TCL_INDEX_NONE, NULL, 0);
    /* Set the superclass to oo::class */
    MarkAsMetaclass(fPtr, abstractCls->classPtr);
    /* Unexport methods */
    TclOOUnexportMethods(abstractCls->classPtr,
	    "create", "createWithNamespace", "new", NULL);


```

tcl9.0 9.0.4, revision `Configured source tree joined by complete SHA; configure/build replay is unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.0.4/doc/abstract.n`, function `abstract(n) public manufacturer description`, lines 27–48. Retained reference `source-window-1`.

```text
Abstract classes are classes that can contain definitions, but which cannot be
directly manufactured; they are intended to only ever be inherited from and
instantiated indirectly. The characteristic methods of \fBoo::class\fR
(\fBcreate\fR and \fBnew\fR) are not exported by an instance of
\fBoo::abstract\fR.
.PP
Note that \fBoo::abstract\fR is not itself an instance of \fBoo::abstract\fR.
.SS CONSTRUCTOR
The \fBoo::abstract\fR class does not define an explicit constructor; this
means that it is effectively the same as the constructor of the
\fBoo::class\fR class.
.SS DESTRUCTOR
The \fBoo::abstract\fR class does not define an explicit destructor;
destroying an instance of it is just like destroying an ordinary class (and
will destroy all its subclasses).
.SS "EXPORTED METHODS"
The \fBoo::abstract\fR class defines no new exported methods.
.SS "NON-EXPORTED METHODS"
The \fBoo::abstract\fR class explicitly states that \fBcreate\fR,
\fBcreateWithNamespace\fR, and \fBnew\fR are unexported.
.SH EXAMPLES
.PP

```

tcl9.1 9.1.0, revision `Configured source tree joined by complete SHA; configure/build replay is unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.c`, function `MakeAdditionalClasses`, lines 683–697. Retained reference `source-window-2`.

```text
    /*
     * Make the abstract class.
     */

    // A metaclass that is used to make classes that can't be directly
    // instantiated. See abstract(n).
    Object *abstractCls = (Object *) Tcl_NewObjectInstance(interp,
	    (Tcl_Class) fPtr->classCls, "::oo::abstract",
	    NULL, TCL_INDEX_NONE, NULL, 0);
    // Set the superclass to oo::class
    MarkAsMetaclass(fPtr, abstractCls->classPtr);
    // Unexport methods
    TclOOUnexportMethods(abstractCls->classPtr,
	    "create", "createWithNamespace", "new", NULL);


```

tcl9.1 9.1.0, revision `Configured source tree joined by complete SHA; configure/build replay is unrecorded`, `/workspace/tcl-lsp/tmp/tcl9.1.0/doc/abstract.n`, function `abstract(n) public manufacturer description`, lines 27–48. Retained reference `source-window-3`.

```text
Abstract classes are classes that can contain definitions, but which cannot be
directly manufactured; they are intended to only ever be inherited from and
instantiated indirectly. The characteristic methods of \fBoo::class\fR
(\fBcreate\fR and \fBnew\fR) are not exported by an instance of
\fBoo::abstract\fR.
.PP
Note that \fBoo::abstract\fR is not itself an instance of \fBoo::abstract\fR.
.SS CONSTRUCTOR
The \fBoo::abstract\fR class does not define an explicit constructor; this
means that it is effectively the same as the constructor of the
\fBoo::class\fR class.
.SS DESTRUCTOR
The \fBoo::abstract\fR class does not define an explicit destructor;
destroying an instance of it is just like destroying an ordinary class (and
will destroy all its subclasses).
.SS "EXPORTED METHODS"
The \fBoo::abstract\fR class defines no new exported methods.
.SS "NON-EXPORTED METHODS"
The \fBoo::abstract\fR class explicitly states that \fBcreate\fR,
\fBcreateWithNamespace\fR, and \fBnew\fR are unexported.
.SH EXAMPLES
.PP

```


## Consumer bindings

- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `type_infer::tests::original_caller_safe_factory_does_not_promote_abstract_exports_or_custom_metaclasses` (linked): Finite source-defined control: original caller safe factory does not promote abstract exports or custom metaclasses. The C9 public-call proof permits explicit export of Abstract new. That does not issue the ordinary Native default recipe required by this separate precision gate, or establish a completed allocation. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-compiler/src/command_binding/source_transition_advice/source_class.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/source_class.rs), `command_binding::source_transition_advice::source_class::tests::original_abstract_constructor_defaults_and_exports_keep_selected_native_source_grammar` (linked): Finite source-defined control: original abstract constructor defaults and exports keep selected native source grammar. A Native source-name model is not a entered Native constructor receipt. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-compiler/src/command_binding/source_transition_advice/source_class.rs](../../../../rust/tcl-compiler/src/command_binding/source_transition_advice/source_class.rs), `command_binding::source_transition_advice::source_class::tests::original_abstract_constructor_logical_advice_keeps_defaults_without_native_identity` (linked): Finite source-defined control: original abstract constructor logical advice keeps defaults without native identity. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `definer::tests::original_abstract_manufacturer_visibility_keeps_metaclass_and_created_class_surfaces` (linked): Finite source-defined control: original abstract manufacturer visibility keeps metaclass and created class surfaces. Public C9 observations establish defaults, not Native constructor admission. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-spec-studio/src/render_rs.rs](../../../../rust/tcl-spec-studio/src/render_rs.rs), `render_rs::tests::original_abstract_grammar_rust_render_keeps_the_shared_named_descriptor` (linked): Finite source-defined control: original abstract grammar rust render keeps the shared named descriptor. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-spec-studio/src/render_spectcl.rs](../../../../rust/tcl-spec-studio/src/render_spectcl.rs), `render_spectcl::tests::original_abstract_grammar_roundtrip_keeps_created_class_and_factory_visibilities` (linked): Finite source-defined control: original abstract grammar roundtrip keeps created class and factory visibilities. Roundtrip the selected semantic data without exporting Native hooks. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-spectcl/src/loader.rs](../../../../rust/tcl-spectcl/src/loader.rs), `loader::tests::original_abstract_definition_grammar_loads_without_changing_factory_visibility` (linked): Finite source-defined control: original abstract definition grammar loads without changing factory visibility. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original exact script/version argv and own process receipts are retained unchanged. Explicit canonical joins identify shared property335 original ELF gzip payloads; restore exact whole SHA/size before original replay. Complete inherited environment, shared-library bytes and configure/build replay are absent. Ordinary public visibility output does not measure constructor-handler/native frame behavior; no native or Rust replay is performed by publication.


## Exact original-source Compiler software outcomes

The table reports only admitted selectors attached to this question by exact comments in the retained original test bodies. Each result is limited to its named source snapshot, owned Compiler image and selector invocation. No other source/image, unselected assertion, external-provider observation or performance conclusion follows.

| Original operation | Passed | Failed | Incomplete |
| --- | ---: | ---: | ---: |
| `integration-compiler-focused-consumer-tests344` | 1 | 2 | 0 |

[Exact original-source body hashes, receipts, streams, inventories and owned images](rust-validation/compiler-owned341345-experiments.json) preserve the per-selector events and independent producer/admission limits. The existing provider answers retain their own experimental scope.


## Exact original-source347 software outcomes

These results belong only to the retained source347 body/question pairs and their owning Compiler or Core image. They do not establish an outcome for current source, unselected controls or external providers.

| Original operation | Passed | Failed |
| --- | ---: | ---: |
| `integration-compiler-focused-consumer-tests347` | 1 | 2 |

[Exact original body hashes, producer/list/admission receipts, raw logs and owned images](rust-validation/compiler-core-runtime-owned347-experiments.json) retain the finite per-selector results and independent failed Runtime producer.

# naming.namespace.source-export-filter-units

Kind: `native-observation`

## Problem statement

Source declarations with adjacent opaque units can be collapsed by display conversion, or a consumer can incorrectly reuse Tcl export filtering for Jim. This changes imported-command candidate sets.

## Question

Do the exact adjacent source-produced surrogate units remain distinct, and does namespace export filter or leading clear select the same commands on C Tcl and Jim?

## Conclusion

All five C releases retain FIRST/SECOND as distinct callable names, import only the exported adjacent-unit name, and leading -clear replaces before with after. Jim retains distinct callable names but ignores export filtering and leading -clear: both names import and before remains available.

A marked Compiler source definition binds exact surrogate export-pattern storage for the five selected C source recipes to the shared namespace transfer owner.

## Scope

The exact ASCII source-file controls are captured on Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim 0.84-9-g5bac7c9. Backslash substitutions produce the tested adjacent surrogate units. Attempt 2 reports provider string length/index and scan %c results as decimal ASCII units; it does not assert raw bytes or a universal scalar representation. These observations grant no counted API, raw NUL, physical cache, compiler preparation, arbitrary glob equivalence or source execution authority. BIG-IP was not tested.

The fixture analyses original Document source under selected C lexer/name policy and Direct mode; it launches no native interpreter and supplies no external output, private character/header, token or loader result. Encoded opaque export-pattern state is conditional source advice. The unchanged original public export-filter observations retain their own release/input scope, and no software passing outcome is attached.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Executable SHA-256 b100108bc3747ea4fa6a4411a28756c26c8c201390a1983ae35443ce7bece817. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

FIRST SECOND; export-filter result 1 0; leading-clear result 0 AFTER.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Executable SHA-256 6b2005f227b608208c8bfdf472d5d6c1ed9595ad10f4b7442d5ee1e23e9e400b. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

FIRST SECOND; export-filter result 1 0; leading-clear result 0 AFTER.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Executable SHA-256 2e103024652a7d81f5496a979464fe0acad87a9033e798305be231ef484c5c5a. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

FIRST SECOND; export-filter result 1 0; leading-clear result 0 AFTER.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Executable SHA-256 f55225fd9e74448b244b90f41d3dce2989fc7e205bee4f2f0d4a08057880e668. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

FIRST SECOND; export-filter result 1 0; leading-clear result 0 AFTER.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Executable SHA-256 c5638d48d9a54d3a0d0901a4dc94698f8e018715473f8f6c2e3264408694ce47. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

FIRST SECOND; export-filter result 1 0; leading-clear result 0 AFTER.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Jim Tcl.

FIRST SECOND; export-filter result 1 1; leading-clear result 1 AFTER.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `source` (input): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/cases.tcl](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/cases.tcl). SHA-256 `e744d3ba4b9237b459549aecc16d80cd1cc65dfd02f2f4dcf857af37dfaf05c4`. Exact ASCII source program with eleven independent catch-result rows.
- `receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/receipt.json). SHA-256 `ef27d64b23dc3324b15a7579d93ee9d3b138b2d4b7448a380edfbc5e59cfdbba`. All six actual process, reported version, source/executable/output hashes and row counts.
- `runner` (input): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/capture.py](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/capture.py). SHA-256 `7cbc9896b77e4075d092e6d2a6a27571dc09059acc75205b1f959fdf256cf29d`. Retained capture program; its original workspace paths are recorded.
- `tcl8.4-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20/stdout). SHA-256 `ca0598f895a7ea46763967273bb1ee66d77c09a1e457aed4d878915ad69d4d7d`. Actual numeric result rows.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20/receipt.json). SHA-256 `5e1e062cc0b5c16b1da400a78b68c84b63925456009b6e66d50e39b0047073b8`. Actual provider identity and invocation/output digests.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual empty stderr stream.
- `tcl8.5-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19/stdout). SHA-256 `ca0598f895a7ea46763967273bb1ee66d77c09a1e457aed4d878915ad69d4d7d`. Actual numeric result rows.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19/receipt.json). SHA-256 `984729181762d93776bbbf65bc041869845ed848bebd21c7fbcb59cdc4bccf7a`. Actual provider identity and invocation/output digests.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual empty stderr stream.
- `tcl8.6-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18/stdout). SHA-256 `ca0598f895a7ea46763967273bb1ee66d77c09a1e457aed4d878915ad69d4d7d`. Actual numeric result rows.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18/receipt.json). SHA-256 `ecf25dfe12b36f6169995acfff63e0f6dab008b9fdafe64f34c367db6f613dff`. Actual provider identity and invocation/output digests.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual empty stderr stream.
- `tcl9.0-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4/stdout). SHA-256 `ca0598f895a7ea46763967273bb1ee66d77c09a1e457aed4d878915ad69d4d7d`. Actual numeric result rows.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4/receipt.json). SHA-256 `1a0980b115400e45b87d64ff458c9b6fc99381d313b7e0ae7a1fe797ffef2497`. Actual provider identity and invocation/output digests.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual empty stderr stream.
- `tcl9.1-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0/stdout). SHA-256 `ca0598f895a7ea46763967273bb1ee66d77c09a1e457aed4d878915ad69d4d7d`. Actual numeric result rows.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0/receipt.json). SHA-256 `1bff05a5838eaaa20af703a4e41e32e0e35c7aab478c2d99d9c3d20512450d02`. Actual provider identity and invocation/output digests.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual empty stderr stream.
- `jim-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim/stdout). SHA-256 `27162260b7b7501c40f2429bc349e88854210f8150bfa3913e9b47f9d462c51f`. Actual numeric result rows.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim/receipt.json). SHA-256 `f9f987cf85fd49d361422e7c3e66c8db740f6f47787e784bcf40ab14c54bad70`. Actual provider identity and invocation/output digests.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual empty stderr stream.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `apply_exports`: Project selected original namespace export operands through their retained name-purpose recipe into exact opaque export alternatives; quiet source-model completion and Native observation remain separate.
- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `command_binding::original_namespace_binding::tests::original_namespace_export_storage_keeps_exact_opaque_units` (linked): Five selected C source-model fixtures retain the original export pattern p plus encoded surrogateD800 as exact opaque NameBytes, distinct from D801, in the checkpoint namespace export alternatives. This inspects conditional source state only, with no new original process or object String conversion.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Replay

The retained capture.py records exact retained workspace commands and hashes. The source fixtures and raw outputs are permanent; no portable executable provisioning replay is supplied. Provider answers remain scoped to the listed cases and actual channels.

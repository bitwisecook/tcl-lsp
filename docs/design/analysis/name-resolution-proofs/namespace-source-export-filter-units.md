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

Status: `observed`. Version: 8.4.20. Build: Executable. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

FIRST SECOND; export-filter result 1 0; leading-clear result 0 AFTER.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Executable. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

FIRST SECOND; export-filter result 1 0; leading-clear result 0 AFTER.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Executable. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

FIRST SECOND; export-filter result 1 0; leading-clear result 0 AFTER.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Executable. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

FIRST SECOND; export-filter result 1 0; leading-clear result 0 AFTER.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Executable. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

FIRST SECOND; export-filter result 1 0; leading-clear result 0 AFTER.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Executable. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Jim Tcl.

FIRST SECOND; export-filter result 1 1; leading-clear result 1 AFTER.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `source` (input): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/cases.tcl](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/cases.tcl). Exact ASCII source program with eleven independent catch-result rows.
- `receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/receipt.json). All six actual process, reported version, source/executable/output hashes and row counts.
- `runner` (input): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/capture.py](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/capture.py). Retained capture program; its original workspace paths are recorded.
- `tcl8.4-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20/stdout). Actual numeric result rows.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20/receipt.json). Actual provider identity and invocation/output digests.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.4.20/stderr). Actual empty stderr stream.
- `tcl8.5-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19/stdout). Actual numeric result rows.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19/receipt.json). Actual provider identity and invocation/output digests.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.5.19/stderr). Actual empty stderr stream.
- `tcl8.6-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18/stdout). Actual numeric result rows.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18/receipt.json). Actual provider identity and invocation/output digests.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/8.6.18/stderr). Actual empty stderr stream.
- `tcl9.0-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4/stdout). Actual numeric result rows.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4/receipt.json). Actual provider identity and invocation/output digests.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.0.4/stderr). Actual empty stderr stream.
- `tcl9.1-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0/stdout). Actual numeric result rows.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0/receipt.json). Actual provider identity and invocation/output digests.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/9.1.0/stderr). Actual empty stderr stream.
- `jim-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim/stdout). Actual numeric result rows.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim/receipt.json). Actual provider identity and invocation/output digests.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/attempt2/jim/stderr). Actual empty stderr stream.

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

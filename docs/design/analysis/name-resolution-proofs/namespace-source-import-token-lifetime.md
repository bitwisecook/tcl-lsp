# naming.namespace.source-import-token-lifetime

Kind: `native-observation`

## Problem statement

An imported command may retain an actual source token or a name recipe. Reusing one interpretation across engines gives wrong call and forget behavior after command moves.

## Question

Does a source import survive moving the original command, and does qualified forget remove an imported command after the imported local name moves?

## Conclusion

C Tcl imports retain callable source identity across the tested source rename; local forget removes only the selected adjacent-unit import, and qualified origin forget removes the renamed imported local command. Jim source rename breaks the tested import, and namespace forget is unavailable.

A marked conditional source-world control preserves the distinct surviving import and its declaration-site origin after a closed prelude, import move and forget.

## Scope

The exact ASCII source-file controls are captured on Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim 0.84-9-g5bac7c9. Backslash substitutions produce the tested adjacent surrogate units. Attempt 2 reports provider string length/index and scan %c results as decimal ASCII units; it does not assert raw bytes or a universal scalar representation. These observations grant no counted API, raw NUL, physical cache, compiler preparation, arbitrary glob equivalence or source execution authority. BIG-IP was not tested.

No original token/header/refcount lifetime, external process, entered handler or current native table is measured by this source definition. The fixture explicitly removes auto_import before its closed model transfer; equal source spelling or a moved reporting label cannot replace retained declaration-site origin. Original native import/move/forget rows remain independent and unchanged; no assertion outcome is attached.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Executable. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

Source move returns HELD; local forget returns 0 SECOND; qualified forget after local move returns 0.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Executable. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

Source move returns HELD; local forget returns 0 SECOND; qualified forget after local move returns 0.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Executable. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

Source move returns HELD; local forget returns 0 SECOND; qualified forget after local move returns 0.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Executable. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

Source move returns HELD; local forget returns 0 SECOND; qualified forget after local move returns 0.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Executable. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Tcl.

Source move returns HELD; local forget returns 0 SECOND; qualified forget after local move returns 0.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Executable. Channel: ASCII source-file shell ingress; result string length/index and scan %c are reported as decimal units in an ASCII-only row. No raw counted NativeString NUL, binary byte projection or physical object cache claim.. Dialect: Jim Tcl.

Source move gives code 1 invalid command name Scope88MoveA::p; both forget controls give code 1 unavailable subcommand.

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

- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `apply_import`: Retain source import origins under actual selected namespace/name policy; source prelude and destination closure remain independent completion premises.
- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `apply_forget`: Withdraw selected conditional import publications by retained original source pattern/origin rather than printed moved labels.
- [rust/tcl-compiler/src/command_binding/original_namespace_binding.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_binding.rs), `command_binding::original_namespace_binding::tests::original_namespace_import_and_forget_keep_actual_origin_with_closed_prelude` (linked): Five selected C conditional source worlds explicitly vacate auto_import, import distinct D800/D801 names, move the D800 import and forget its original source pattern. The surviving D801 Imported publication retains the actual same source declaration-site origin; this is source-world correspondence, not an original native token/lifetime probe.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Replay

The retained capture.py records exact retained workspace commands and hashes. The source fixtures and raw outputs are permanent; no portable executable provisioning replay is supplied. Provider answers remain scoped to the listed cases and actual channels.

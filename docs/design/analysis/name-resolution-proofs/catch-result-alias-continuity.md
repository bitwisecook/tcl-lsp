# naming.variable.catch-result-alias-continuity

Kind: `native-observation`

## Problem statement

A caught command can replace a procedure formal cell with a global alias before catch stores its result. Retaining the original formal identity solely because its written name is unchanged would direct later reads or renames at the wrong cell. A known error-only body and a separate catch result destination distinguish catch storage from the callback's prior alias mutation.

## Question

After a caught callback unsets formal name and links it to ::destination, does catch store BOOM through that surviving alias, and what do the plain and separate-result controls produce on each tested engine?

## Conclusion

All six selected native shells return LINKED = BOOM BOOM, PLAIN = BOOM and SEPARATE_RESULT = OLD BOOM OLD. The linked catch result reaches the global destination after callback alias replacement; the separate-result control confirms the alias mutation independently. These finite ASCII source observations do not preserve an arbitrary incoming formal identity across an unknown caught command and do not establish physical variable, object, compiler or Normal capabilities.

## Scope

Three exact caught ASCII source controls supplied as native shell stdin on Tcl 8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and current Jim 0.84-9-g5bac7c9. Actual info patchlevel, selected executable/header/Makefile/source-owner hashes and exact streams are retained. No raw NUL, opaque byte name, arbitrary object representation, native compiler callback, resolver, trace or BIG-IP appliance is tested. Static library digests and configure/compiler command histories are not recorded in this capture.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual selected shell SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a; independent original header/Makefile/source-owner digests retained. Linked library digest and configure/compiler histories are unrecorded.. Channel: ASCII file bytes passed to shell stdin. Dialect: Tcl.

LINKED caught completion0/result hex424f4f4d20424f4f4d (BOOM BOOM); PLAIN 0/424f4f4d (BOOM); SEPARATE_RESULT 0/4f4c4420424f4f4d204f4c44 (OLD BOOM OLD). Shell process0; stderr empty; actual info patchlevel reported before controls.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual selected shell SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a; independent original header/Makefile/source-owner digests retained. Linked library digest and configure/compiler histories are unrecorded.. Channel: ASCII file bytes passed to shell stdin. Dialect: Tcl.

LINKED caught completion0/result hex424f4f4d20424f4f4d (BOOM BOOM); PLAIN 0/424f4f4d (BOOM); SEPARATE_RESULT 0/4f4c4420424f4f4d204f4c44 (OLD BOOM OLD). Shell process0; stderr empty; actual info patchlevel reported before controls.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual selected shell SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff; independent original header/Makefile/source-owner digests retained. Linked library digest and configure/compiler histories are unrecorded.. Channel: ASCII file bytes passed to shell stdin. Dialect: Tcl.

LINKED caught completion0/result hex424f4f4d20424f4f4d (BOOM BOOM); PLAIN 0/424f4f4d (BOOM); SEPARATE_RESULT 0/4f4c4420424f4f4d204f4c44 (OLD BOOM OLD). Shell process0; stderr empty; actual info patchlevel reported before controls.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual selected shell SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1; independent original header/Makefile/source-owner digests retained. Linked library digest and configure/compiler histories are unrecorded.. Channel: ASCII file bytes passed to shell stdin. Dialect: Tcl.

LINKED caught completion0/result hex424f4f4d20424f4f4d (BOOM BOOM); PLAIN 0/424f4f4d (BOOM); SEPARATE_RESULT 0/4f4c4420424f4f4d204f4c44 (OLD BOOM OLD). Shell process0; stderr empty; actual info patchlevel reported before controls.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual selected shell SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d; independent original header/Makefile/source-owner digests retained. Linked library digest and configure/compiler histories are unrecorded.. Channel: ASCII file bytes passed to shell stdin. Dialect: Tcl.

LINKED caught completion0/result hex424f4f4d20424f4f4d (BOOM BOOM); PLAIN 0/424f4f4d (BOOM); SEPARATE_RESULT 0/4f4c4420424f4f4d204f4c44 (OLD BOOM OLD). Shell process0; stderr empty; actual info patchlevel reported before controls.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual selected shell SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; independent original header/Makefile/source-owner digests retained. Linked library digest and configure/compiler histories are unrecorded.. Channel: ASCII file bytes passed to shell stdin. Dialect: Jim Tcl.

LINKED caught completion0/result hex424f4f4d20424f4f4d (BOOM BOOM); PLAIN 0/424f4f4d (BOOM); SEPARATE_RESULT 0/4f4c4420424f4f4d204f4c44 (OLD BOOM OLD). Shell process0; stderr empty; actual info patchlevel reported before controls.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `exact-source` (input): [rust/tcl-vm/tests/data/native_catch_formal_alias/cases.tcl](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/cases.tcl). Exact 733-byte ASCII source supplied to every original shell stdin; SHA is checked separately in each original provider receipt.
- `aggregate` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/receipt.json](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/receipt.json). Exact original six-provider capture array including actual patchlevel, input/build associations, process exits and reached result rows.
- `original-capture-helper` (provider): [rust/tcl-vm/tests/data/native_catch_formal_alias/capture.py](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/capture.py). Exact original capture helper; recorded absolute provider roots are retained as metadata and are not portable replay defaults.
- `maintained-replay` (provider): [rust/tcl-vm/tests/data/native_catch_formal_alias/replay.py](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/replay.py). Portable root-parameterized required-input verifier and exact process/status/stdout/stderr comparator; verification alone does not launch native code.
- `tcl8.4-receipt` (provider): [rust/tcl-vm/tests/data/native_catch_formal_alias/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.4.20/receipt.json). Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl8.4-stdout` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/8.4.20/stdout](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.4.20/stdout). Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.4.20/stderr). Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl8.5-receipt` (provider): [rust/tcl-vm/tests/data/native_catch_formal_alias/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.5.19/receipt.json). Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl8.5-stdout` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/8.5.19/stdout](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.5.19/stdout). Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.5.19/stderr). Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl8.6-receipt` (provider): [rust/tcl-vm/tests/data/native_catch_formal_alias/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.6.18/receipt.json). Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl8.6-stdout` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/8.6.18/stdout](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.6.18/stdout). Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.6.18/stderr). Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl9.0-receipt` (provider): [rust/tcl-vm/tests/data/native_catch_formal_alias/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/9.0.4/receipt.json). Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl9.0-stdout` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/9.0.4/stdout](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/9.0.4/stdout). Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/9.0.4/stderr). Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl9.1-receipt` (provider): [rust/tcl-vm/tests/data/native_catch_formal_alias/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/9.1.0/receipt.json). Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl9.1-stdout` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/9.1.0/stdout](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/9.1.0/stdout). Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/9.1.0/stderr). Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `jim-receipt` (provider): [rust/tcl-vm/tests/data/native_catch_formal_alias/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/jim/receipt.json). Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `jim-stdout` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/jim/stdout](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/jim/stdout). Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `jim-stderr` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/jim/stderr](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/jim/stderr). Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `cmd_catch`: Store reached caught results through the actual surviving receiver rather than original formal-name metadata.
- [rust/tcl-vm/src/command/native_catch_formal_alias_tests.rs](../../../../rust/tcl-vm/src/command/native_catch_formal_alias_tests.rs), `caught_callback_result_follows_replaced_formal_alias_native_controls`: Compare exact source-triggered alias continuity and result-destination controls with scoped native shell outputs.
- [rust/tcl-vm/src/command/native_catch_formal_alias_tests.rs](../../../../rust/tcl-vm/src/command/native_catch_formal_alias_tests.rs), `command::native_catch_formal_alias_tests::caught_callback_result_follows_replaced_formal_alias_native_controls` (linked): Evaluate the exact retained ASCII source bytes under six independently selected VM core fixtures and compare all three caught result rows against matching original shell stdout. VM reported-version presentation is excluded from this semantic comparison.
- [rust/tcl-compiler/src/analyser/scope.rs](../../../../rust/tcl-compiler/src/analyser/scope.rs), `analyser::scope::tests::caught_caller_rebinding_cannot_retain_the_original_formal_symbol` (linked): Withdraw source formal identity after exact caller unset/upvar rebinding and incomplete rename coverage, independently of the native caught result rows.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-vm/tests/data/native_catch_formal_alias/replay.py",
  "--c-root",
  "tmp",
  "--jim-root",
  "<exact-built-jim-root>",
  "--output",
  "<new-replay-output>"
]
```

Run from repository root with the exact retained shell/header/Makefile/source-owner hashes and C library directories. Full replay launches six native shell processes and compares process status and both complete streams, including actual patchlevel. --verify-only checks required inputs without executing native or Rust code. Rust comparison selectors are separate and have no execution claim in this record.

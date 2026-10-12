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

## Exact evidence

- `exact-source` (input): [rust/tcl-vm/tests/data/native_catch_formal_alias/cases.tcl](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/cases.tcl). SHA-256 `1dd339248836dfcdd3365901230b31f6b0a97e9a5557ec92be4886255ec4e143`. Exact 733-byte ASCII source supplied to every original shell stdin; SHA is checked separately in each original provider receipt.
- `aggregate` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/receipt.json](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/receipt.json). SHA-256 `9bb5a261889cf5805338567ab26812df8035eb0bfc7dee5e0e382d1f790f4264`. Exact original six-provider capture array including actual patchlevel, input/build associations, process exits and reached result rows.
- `original-capture-helper` (provider): [rust/tcl-vm/tests/data/native_catch_formal_alias/capture.py](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/capture.py). SHA-256 `1375de22fde9dafada7e33d29081a45a8c1e354cafc6800817cbcf0d41a8bb9b`. Exact original capture helper; recorded absolute provider roots are retained as metadata and are not portable replay defaults.
- `maintained-replay` (provider): [rust/tcl-vm/tests/data/native_catch_formal_alias/replay.py](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/replay.py). SHA-256 `020c9ec4b44a9ad4e3f8beede61169abd331e928362c91f79e3a7c51b45f5273`. Portable root-parameterized required-input verifier and exact process/status/stdout/stderr comparator; verification alone does not launch native code.
- `tcl8.4-receipt` (provider): [rust/tcl-vm/tests/data/native_catch_formal_alias/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.4.20/receipt.json). SHA-256 `178b09538bec804eb82e6a645231a76da9ccc14aea264da47c33bb9922e4887f`. Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl8.4-stdout` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/8.4.20/stdout](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.4.20/stdout). SHA-256 `3d999d48eb99ad1dce13a2b268d035658e54c66c92f4fe702ddcbd422925d31a`. Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl8.5-receipt` (provider): [rust/tcl-vm/tests/data/native_catch_formal_alias/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.5.19/receipt.json). SHA-256 `25cd630e13e150668913a8737e30238566e3705406e18cb009d6c57eb8cca93c`. Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl8.5-stdout` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/8.5.19/stdout](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.5.19/stdout). SHA-256 `cfa56b0a9c08c13e89b65a559204e94ae76932292ccf7f579ca1b4bf61e4b812`. Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl8.6-receipt` (provider): [rust/tcl-vm/tests/data/native_catch_formal_alias/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.6.18/receipt.json). SHA-256 `d49eb91fa2fadb393ff6e689ed518352c4b939fd6c8aee0a7f816ccbe7cf63f7`. Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl8.6-stdout` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/8.6.18/stdout](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.6.18/stdout). SHA-256 `e77d6bf22e08380d34b44e6ad169a6d408bc90952cf11421dad9dd71424753f9`. Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl9.0-receipt` (provider): [rust/tcl-vm/tests/data/native_catch_formal_alias/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/9.0.4/receipt.json). SHA-256 `72105a975974fb2046b97be44d014d3ef50eb5a7e52f5ec375758fa9b689a2ac`. Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl9.0-stdout` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/9.0.4/stdout](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/9.0.4/stdout). SHA-256 `57938f3e727c68183e1638293f9d59022d3c88be7f7d5719a3110a87f8c802c0`. Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl9.1-receipt` (provider): [rust/tcl-vm/tests/data/native_catch_formal_alias/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/9.1.0/receipt.json). SHA-256 `237714c530d3dd51c5d10a3a1c44422b55349f27d217692f0a6ea60e9969e7d5`. Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl9.1-stdout` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/9.1.0/stdout](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/9.1.0/stdout). SHA-256 `3b7bfa6fba018c38a7d31df8d2385060af681c9589abf2cb97453e4ebfe0dbb2`. Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `jim-receipt` (provider): [rust/tcl-vm/tests/data/native_catch_formal_alias/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/jim/receipt.json). SHA-256 `23c9cc850035c6668458733a2db7fddbcdb35bfcea7667ee191b7c863d4e643a`. Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `jim-stdout` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/jim/stdout](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/jim/stdout). SHA-256 `648b87097509d5203b934d6ac282616b94d13a733349f3db89abe74a6f183126`. Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.
- `jim-stderr` (observation): [rust/tcl-vm/tests/data/native_catch_formal_alias/jim/stderr](../../../../rust/tcl-vm/tests/data/native_catch_formal_alias/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original provider receipt or stream; caught guest result codes are distinct from shell process status.

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

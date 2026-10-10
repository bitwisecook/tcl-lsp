# naming.info.loaded-original-interpreter-path

Kind: `native-observation`

## Problem statement

Decoding an interpreter path into a display name can lose its original list element boundaries or choose a different child; a setup failure also cannot answer the later loaded query.

## Question

How do the exact source controls resolve root and nested original interpreter list paths, materialised binary child names and malformed/missing paths for info loaded?

## Conclusion

All five C providers return an empty loaded list for the root, the existing two-element nested child path, and a child created and queried with the same binary-produced runtime value. A malformed list produces unmatched open brace in list, while the missing child produces could not find interpreter "MissingInfo085". Jim explicitly rejects direct info loaded queries. Its nested and binary-child controls fail at interp create before reaching info loaded and provide no loaded-path result. The runtime binary values undergo string conversion; successful lookup supplies no raw-zero argv, physical child-key/cache law or interpreter lifetime grant.

## Scope

Five loaded-path controls in the exact shared twelve-row ASCII LF source-file program, independently captured under C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9. Empty loaded lists do not establish general library enumeration, raw counted storage, cache, native compiler or Normal capability. No Rust result is attached.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Exact shell executable SHA b100108bc3747ea4fa6a4411a28756c26c8c201390a1983ae35443ce7bece817; linked library/header/configure/build flags unrecorded.. Channel: ASCII LF source-file shell ingress. Runtime binary-produced raw/encoded bytes are separate; output decimal string-index scan-%c units, not original physical bytes. No direct API/object cache/CPP/Normal grant.. Dialect: 8.4.20.

All five C providers return an empty loaded list for the root, the existing two-element nested child path, and a child created and queried with the same binary-produced runtime value. A malformed list produces unmatched open brace in list, while the missing child produces could not find interpreter "MissingInfo085". Jim explicitly rejects direct info loaded queries. Its nested and binary-child controls fail at interp create before reaching info loaded and provide no loaded-path result. The runtime binary values undergo string conversion; successful lookup supplies no raw-zero argv, physical child-key/cache law or interpreter lifetime grant.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Exact shell executable SHA 6b2005f227b608208c8bfdf472d5d6c1ed9595ad10f4b7442d5ee1e23e9e400b; linked library/header/configure/build flags unrecorded.. Channel: ASCII LF source-file shell ingress. Runtime binary-produced raw/encoded bytes are separate; output decimal string-index scan-%c units, not original physical bytes. No direct API/object cache/CPP/Normal grant.. Dialect: 8.5.19.

All five C providers return an empty loaded list for the root, the existing two-element nested child path, and a child created and queried with the same binary-produced runtime value. A malformed list produces unmatched open brace in list, while the missing child produces could not find interpreter "MissingInfo085". Jim explicitly rejects direct info loaded queries. Its nested and binary-child controls fail at interp create before reaching info loaded and provide no loaded-path result. The runtime binary values undergo string conversion; successful lookup supplies no raw-zero argv, physical child-key/cache law or interpreter lifetime grant.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Exact shell executable SHA 2e103024652a7d81f5496a979464fe0acad87a9033e798305be231ef484c5c5a; linked library/header/configure/build flags unrecorded.. Channel: ASCII LF source-file shell ingress. Runtime binary-produced raw/encoded bytes are separate; output decimal string-index scan-%c units, not original physical bytes. No direct API/object cache/CPP/Normal grant.. Dialect: 8.6.18.

All five C providers return an empty loaded list for the root, the existing two-element nested child path, and a child created and queried with the same binary-produced runtime value. A malformed list produces unmatched open brace in list, while the missing child produces could not find interpreter "MissingInfo085". Jim explicitly rejects direct info loaded queries. Its nested and binary-child controls fail at interp create before reaching info loaded and provide no loaded-path result. The runtime binary values undergo string conversion; successful lookup supplies no raw-zero argv, physical child-key/cache law or interpreter lifetime grant.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Exact shell executable SHA f55225fd9e74448b244b90f41d3dce2989fc7e205bee4f2f0d4a08057880e668; linked library/header/configure/build flags unrecorded.. Channel: ASCII LF source-file shell ingress. Runtime binary-produced raw/encoded bytes are separate; output decimal string-index scan-%c units, not original physical bytes. No direct API/object cache/CPP/Normal grant.. Dialect: 9.0.4.

All five C providers return an empty loaded list for the root, the existing two-element nested child path, and a child created and queried with the same binary-produced runtime value. A malformed list produces unmatched open brace in list, while the missing child produces could not find interpreter "MissingInfo085". Jim explicitly rejects direct info loaded queries. Its nested and binary-child controls fail at interp create before reaching info loaded and provide no loaded-path result. The runtime binary values undergo string conversion; successful lookup supplies no raw-zero argv, physical child-key/cache law or interpreter lifetime grant.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Exact shell executable SHA c5638d48d9a54d3a0d0901a4dc94698f8e018715473f8f6c2e3264408694ce47; linked library/header/configure/build flags unrecorded.. Channel: ASCII LF source-file shell ingress. Runtime binary-produced raw/encoded bytes are separate; output decimal string-index scan-%c units, not original physical bytes. No direct API/object cache/CPP/Normal grant.. Dialect: 9.1.0.

All five C providers return an empty loaded list for the root, the existing two-element nested child path, and a child created and queried with the same binary-produced runtime value. A malformed list produces unmatched open brace in list, while the missing child produces could not find interpreter "MissingInfo085". Jim explicitly rejects direct info loaded queries. Its nested and binary-child controls fail at interp create before reaching info loaded and provide no loaded-path result. The runtime binary values undergo string conversion; successful lookup supplies no raw-zero argv, physical child-key/cache law or interpreter lifetime grant.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Exact shell executable SHA e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; linked library/header/configure/build flags unrecorded.. Channel: ASCII LF source-file shell ingress. Runtime binary-produced raw/encoded bytes are separate; output decimal string-index scan-%c units, not original physical bytes. No direct API/object cache/CPP/Normal grant.. Dialect: 0.84-9-g5bac7c9.

Direct loaded-root/malformed-list/missing-child controls reach info loaded and reject its unknown subcommand. Nested and binary-child controls fail at interp create with wrong # args before loaded; those two later queries are unattempted.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

Observed guest rejection of info functions in all seven function controls; this is not a command-cache, fixed-table or function execution answer.

## Exact evidence

- `source` (input): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/cases.tcl](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/cases.tcl). SHA-256 `c5ce25bcb86f2cf8a803faac0816c985c3fd265873f1da05a59d707a6b5aef8f`. Exact complete ASCII LF source-file program; runtime binary values and decimal result units are separate channels.
- `capture-runner` (input): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/capture.py](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/capture.py). SHA-256 `3af3a56d4d498527632ab3ae7545b00b7b2224e7638c6c95285d665991e5bcf2`. Exact executed shell runner and separately queried patchlevel protocol.
- `capture-receipts` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/receipt.json). SHA-256 `22bf3fcd10096f5278dcce80f3510e11cc0b4662f7d21356793fa6df0144b7a1`. Six actual command/source/executable/process and complete output attributions; no static library or configured build attribution.
- `case-queue` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/queue.json](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/queue.json). SHA-256 `dee7947a570eea78cc5591f1c4dfbabc8e72231633c55158b0390d5155f9622b`. Authored exact finite case inventory; not an outcome observation.
- `offline-verifier` (reconfirmation): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/replay.py](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/replay.py). SHA-256 `53ac7e860b62917c70bfd5c2d9507526e0504cdc7b6f3877d42f1e923a15719c`. Verify-only source/runner/provider/stream associations without native launches.
- `tcl8.4-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.4.20/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.4.20/receipt.json). SHA-256 `34cfc4266cc3e07c34c4dbbc3a215e27ad46e9b9da3930d0d146b49a808398b9`. Exact tcl8.4 receipt.json from the independent source-file shell capture.
- `tcl8.4-stdout` (observation): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.4.20/stdout](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.4.20/stdout). SHA-256 `44df979b0b9ecac1feed942a00c46bb0ff5d0f2891397206d84ebf66361d5190`. Exact tcl8.4 stdout from the independent source-file shell capture.
- `tcl8.4-stderr` (observation): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.4.20/stderr](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact tcl8.4 stderr from the independent source-file shell capture.
- `tcl8.4-version.stdout` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.4.20/version.stdout](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.4.20/version.stdout). SHA-256 `c4a268983bec029286482df6caa3aaae81571c33d467c0a13f258e335da5954f`. Exact tcl8.4 version.stdout from the independent source-file shell capture.
- `tcl8.4-version.stderr` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.4.20/version.stderr](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.4.20/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact tcl8.4 version.stderr from the independent source-file shell capture.
- `tcl8.5-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.5.19/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.5.19/receipt.json). SHA-256 `54a0b75520b996287ef238f31a18f1490ac3e22c8c0961f5da888d19e0489ff4`. Exact tcl8.5 receipt.json from the independent source-file shell capture.
- `tcl8.5-stdout` (observation): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.5.19/stdout](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.5.19/stdout). SHA-256 `574f010724d99ba99ed9dd027d48a7cb7b8be21524cc19ae36e30a5bdc2268c4`. Exact tcl8.5 stdout from the independent source-file shell capture.
- `tcl8.5-stderr` (observation): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.5.19/stderr](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact tcl8.5 stderr from the independent source-file shell capture.
- `tcl8.5-version.stdout` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.5.19/version.stdout](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.5.19/version.stdout). SHA-256 `6b58793c92a3db92926bdacbe0a52a520d417fe876aaa7cf8c29fed54a1fea71`. Exact tcl8.5 version.stdout from the independent source-file shell capture.
- `tcl8.5-version.stderr` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.5.19/version.stderr](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.5.19/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact tcl8.5 version.stderr from the independent source-file shell capture.
- `tcl8.6-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.6.18/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.6.18/receipt.json). SHA-256 `d352dff0cac7ffe3b1c9bcd7d1ebf1e6ebe3189608b266a8eb90c5f6926cc1f1`. Exact tcl8.6 receipt.json from the independent source-file shell capture.
- `tcl8.6-stdout` (observation): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.6.18/stdout](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.6.18/stdout). SHA-256 `574f010724d99ba99ed9dd027d48a7cb7b8be21524cc19ae36e30a5bdc2268c4`. Exact tcl8.6 stdout from the independent source-file shell capture.
- `tcl8.6-stderr` (observation): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.6.18/stderr](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact tcl8.6 stderr from the independent source-file shell capture.
- `tcl8.6-version.stdout` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.6.18/version.stdout](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.6.18/version.stdout). SHA-256 `7523e113df6c321e18099a10ec22086db746e2fcc31a879e16d961e5d3878d01`. Exact tcl8.6 version.stdout from the independent source-file shell capture.
- `tcl8.6-version.stderr` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.6.18/version.stderr](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.6.18/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact tcl8.6 version.stderr from the independent source-file shell capture.
- `tcl9.0-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.0.4/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.0.4/receipt.json). SHA-256 `8d03d0ea09bae3c5bd471464f984343d421b9e67f8e7e9eeb383ee734b2af06f`. Exact tcl9.0 receipt.json from the independent source-file shell capture.
- `tcl9.0-stdout` (observation): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.0.4/stdout](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.0.4/stdout). SHA-256 `574f010724d99ba99ed9dd027d48a7cb7b8be21524cc19ae36e30a5bdc2268c4`. Exact tcl9.0 stdout from the independent source-file shell capture.
- `tcl9.0-stderr` (observation): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.0.4/stderr](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact tcl9.0 stderr from the independent source-file shell capture.
- `tcl9.0-version.stdout` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.0.4/version.stdout](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.0.4/version.stdout). SHA-256 `b83920bac1b2d47c3840f588f95bee9dc69b85e26117f471541cdf85d7da52a2`. Exact tcl9.0 version.stdout from the independent source-file shell capture.
- `tcl9.0-version.stderr` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.0.4/version.stderr](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.0.4/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact tcl9.0 version.stderr from the independent source-file shell capture.
- `tcl9.1-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.1.0/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.1.0/receipt.json). SHA-256 `18b675f3b4a67e81b8a841f931fadc0352387d0e770298035de76e4942123d97`. Exact tcl9.1 receipt.json from the independent source-file shell capture.
- `tcl9.1-stdout` (observation): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.1.0/stdout](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.1.0/stdout). SHA-256 `574f010724d99ba99ed9dd027d48a7cb7b8be21524cc19ae36e30a5bdc2268c4`. Exact tcl9.1 stdout from the independent source-file shell capture.
- `tcl9.1-stderr` (observation): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.1.0/stderr](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact tcl9.1 stderr from the independent source-file shell capture.
- `tcl9.1-version.stdout` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.1.0/version.stdout](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.1.0/version.stdout). SHA-256 `077ee5b3a3a7fc2622ceff6466fea0c28f3fb08b24653dcc7f4baab29b9aef36`. Exact tcl9.1 version.stdout from the independent source-file shell capture.
- `tcl9.1-version.stderr` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.1.0/version.stderr](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.1.0/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact tcl9.1 version.stderr from the independent source-file shell capture.
- `jim-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/jim/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/jim/receipt.json). SHA-256 `660b700709ba70584655adfd754035c2e2795389a4287f2e7433ad07d1226aa9`. Exact jim receipt.json from the independent source-file shell capture.
- `jim-stdout` (observation): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/jim/stdout](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/jim/stdout). SHA-256 `9c14f999aa2cba142600b7b238b10a92bdcbd4967877b30810e65d3b50937748`. Exact jim stdout from the independent source-file shell capture.
- `jim-stderr` (observation): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/jim/stderr](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact jim stderr from the independent source-file shell capture.
- `jim-version.stdout` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/jim/version.stdout](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/jim/version.stdout). SHA-256 `f9752899fa012cf994a528e0250402609067a6f67a144a1172644a9329386803`. Exact jim version.stdout from the independent source-file shell capture.
- `jim-version.stderr` (provider): [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/jim/version.stderr](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/jim/version.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact jim version.stderr from the independent source-file shell capture.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/cmd_info.rs](../../../../rust/tcl-vm/src/cmd_info.rs), `cmd_info`: Retain original interpreter list path elements for loaded queries independently of reporting names and source labels.
- [runtime/rust/src/cmd_info.rs](../../../../runtime/rust/src/cmd_info.rs), `stock_loaded`: Pass the original path object to the independently selected live child-path owner before enumerating loaded entries.
- [rust/tcl-vm/src/cmd_info.rs](../../../../rust/tcl-vm/src/cmd_info.rs), `cmd_info::tests::original_info_loaded_uses_counted_interpreter_path_elements` (linked): Compare the matched finite C source/result purposes through the actual selected VM/Runtime provider; this binding claims no Rust execution or physical byte/header/cache parity.
- [runtime/rust/src/cmd_info.rs](../../../../runtime/rust/src/cmd_info.rs), `cmd_info::tests::original_info_loaded_uses_counted_interpreter_path_elements` (linked): Compare the matched finite C source/result purposes through the actual selected VM/Runtime provider; this binding claims no Rust execution or physical byte/header/cache parity.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-cmd-core/tests/data/native_info_functions_loaded/replay.py",
  "--verify-only"
]
```

The verifier checks retained exact input/provider/stream associations and launches no native provider. The copied capture runner requires its recorded external environment and original absolute source/shell paths. Native executable/library/header/configure artifacts are not included. Numeric output units and materialised binary values supply no raw resident-string, direct counted API, cache or Rust execution claim.

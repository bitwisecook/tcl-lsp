# naming.tcloo.resident-variable-key-header

Kind: `native-observation`

## Problem statement

A shortened public variable-name result does not show whether the object namespace actually stores a short key. The counted TclOO declaration, namespace hash key and report leaf can be different objects. These original-header snapshots distinguish retained counted keys from later API result conversion; they apply only to the captured current object/namespace records, not arbitrary header or compiler capabilities.

## Question

At the captured pre/post reporter windows, what are the existing declaration and object-namespace hash-key header lengths, resident bytes and equality-to-original flags?

## Conclusion

C8.6/C9.0/C9.1 snapshots retain the full counted raw-zero declaration and actual resident namespace key, including length6 and equality-to-original/stored-declaration flags, before and after the public reporter. C8.6 has a short legacy report leaf while its resident key remains counted. Qualification-after-zero controls do not gain the same namespace key. These are original header windows, not returned-list or native compiler authority.

## Scope

Six fixed declaration/primary arrangements (plain, raw zero, qualifier-after-zero, D800, FF, prefix negative), exact original v1..v5 inputs/observer chronology and API windows. C86/C90/C91 supported OO, C84/C85/Jim explicit unavailable OO controls. v3 C86 failed compile is retained separately.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Actual executable SHA256 ca8d707d7628349fa854ff57472f3c840d7a475cd00a44cc6fb978b0caab99c1; retained compile command, header/static-library/Makefile/source-owner hashes and process streams.. Channel: Configured private-header C snapshots of actual stored declaration and object namespace hash key before/after public reporter; no new getter in existing-byte/header window.. Dialect: Tcl.

Captured OO_UNAVAILABLE control; no TclOO resident-key/API comparison executed for this provider.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Actual executable SHA256 b242053a7acf493e590c73839189cda0ce9055c1ea73d2e6fe40e66868cf560b; retained compile command, header/static-library/Makefile/source-owner hashes and process streams.. Channel: Configured private-header C snapshots of actual stored declaration and object namespace hash key before/after public reporter; no new getter in existing-byte/header window.. Dialect: Tcl.

Captured OO_UNAVAILABLE control; no TclOO resident-key/API comparison executed for this provider.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable SHA256 a0de997a410ae30c96aa29b7a2df1eed015cb954a2d3cf4b41fc8436c0174b4d; retained compile command, header/static-library/Makefile/source-owner hashes and process streams.. Channel: Configured private-header C snapshots of actual stored declaration and object namespace hash key before/after public reporter; no new getter in existing-byte/header window.. Dialect: Tcl.

Declaration and resident raw-zero key header length6 and full bytes 6b007461696c are retained with same-original/stored-declaration equality flags.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA256 9ca65c401f5347a75b4a74569f54e39103ce1d8adabc794936f26e09c32c3ce7; retained compile command, header/static-library/Makefile/source-owner hashes and process streams.. Channel: Configured private-header C snapshots of actual stored declaration and object namespace hash key before/after public reporter; no new getter in existing-byte/header window.. Dialect: Tcl.

Declaration and resident raw-zero key header length6 and full bytes 6b007461696c are retained with same-original/stored-declaration equality flags.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA256 d74ae94b8c225377586bcf9c89635d7e527450265a727b06e647b13357c42088; retained compile command, header/static-library/Makefile/source-owner hashes and process streams.. Channel: Configured private-header C snapshots of actual stored declaration and object namespace hash key before/after public reporter; no new getter in existing-byte/header window.. Dialect: Tcl.

Declaration and resident raw-zero key header length6 and full bytes 6b007461696c are retained with same-original/stored-declaration equality flags.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA256 c27128e0e4a8ff9e1c64664595376c355125593aedfb36773804e50c72596392; retained compile command, header/static-library/Makefile/source-owner hashes and process streams.. Channel: Configured private-header C snapshots of actual stored declaration and object namespace hash key before/after public reporter; no new getter in existing-byte/header window.. Dialect: Jim Tcl.

Captured OO_UNAVAILABLE control; no TclOO resident-key/API comparison executed for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `v4-probe.c` (input): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/probe.c](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/probe.c). SHA-256 `c2020522010d433f8e486fb80e522e94a82b4b333ad36ef63a155e48a1df73e0`. Exact retained input/runner or aggregate native build/process/stream association. Prior variants are immutable and are not interchangeable.
- `v4-capture.py` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/capture.py](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/capture.py). SHA-256 `41a2f1771f13ff17799a75455af0b8f1a6531fe1fe26738d0caff8bccb1537dc`. Exact retained input/runner or aggregate native build/process/stream association. Prior variants are immutable and are not interchangeable.
- `v4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/receipt.json). SHA-256 `121dc2a38efa969a73217752e6db8ed0aebc6260e7da7236a2167a73a6d2c913`. Exact retained input/runner or aggregate native build/process/stream association. Prior variants are immutable and are not interchangeable.
- `v4-tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.4.20/receipt.json). SHA-256 `ab20b2ea61bd7c5aa9500a79bf6939332da555d6c905a929b242be0377ab8eb0`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v4-tcl8.4-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.4.20/stdout.tsv). SHA-256 `46f05ae01d71cf78a58b0557e6cab7ddc56fe225a9872ef14387c1c9227f2fc3`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v4-tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v4-tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.5.19/receipt.json). SHA-256 `4e3c3ce63b4d41d989a1c4542745aabc9bece93563ea19a4332ffdcf81542ba6`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v4-tcl8.5-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.5.19/stdout.tsv). SHA-256 `6763de318120d961bc97b57f9f5476dfdd1a9bc433a782898d4c618ca28564d0`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v4-tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v4-tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.6.18/receipt.json). SHA-256 `d8b0d67b98f38a0607ae9e966e7e0cf979c162e6dae5df1ca7decd194a523891`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v4-tcl8.6-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.6.18/stdout.tsv). SHA-256 `35e350181efc8f27d7a1c7d4aea4ee2b551ddd0b8905e3d9d304ff12e0eda082`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v4-tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v4-tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/9.0.4/receipt.json). SHA-256 `377ddfccfd8a03a9e2132ce5cee79f6c5641e2d40e7f5b2841cfd3af9a2c2bad`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v4-tcl9.0-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/9.0.4/stdout.tsv). SHA-256 `46bb2ca089e91236d19f7a94d12090b4894ce8a7c8afe8f2124b3058c378d0e7`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v4-tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v4-tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/9.1.0/receipt.json). SHA-256 `6fc2de3baecbf6621b819050abdca4645f12fb9161a2535eee086411d9ce366d`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v4-tcl9.1-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/9.1.0/stdout.tsv). SHA-256 `254e50b7e045929b1ee2177b255cc21ae659e7cb149e69bb0d3845b049df142d`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v4-tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v4-jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/jim/receipt.json). SHA-256 `7b40fc08f9fc3f5a61583122e819f137496ee70efbd784fc47b58d86d9bb1ea2`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v4-jim-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/jim/stdout.tsv). SHA-256 `5707d1b91c4f468cfe79a38218291016830110fb0bc37c8d942c2c7eff73518a`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.
- `v4-jim-stderr` (observation): [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/jim/stderr](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original process/build/version and exact captured native rows. Successful process exit is independent of guest error or unsupported doors.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original runners contain captured workspace paths and refuse output-directory reuse. Exact input/source/build/executable/stdout/stderr hashes remain in immutable receipts. No portable re-launch or Rust execution result is inferred; v3 failed compile produced no C86 guest rows.

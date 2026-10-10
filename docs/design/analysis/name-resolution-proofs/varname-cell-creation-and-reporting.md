# naming.tcloo.varname-cell-creation-and-reporting

Kind: `native-observation`

## Problem statement

Varname returned a constructed display path without performing the native cell creation and followed-target lookup; String reconstruction also lost raw original operands.

## Question

What followed variable name and namespace creation does stock my varname expose for these eight original counted operands?

## Conclusion

The separately pinned v2 stock controls create and report namespace::k for raw00 relative names on all supported releases, distinct from the full-counted key created by explicit LinkVar. Encoded C080/FF survive reporting. a(k) succeeds and reports an element; relative N::k fails missing parent. Absolute raw00 reports ::global. Exact created keys and error/result/options remain labelled. V1 user-method recursion does not establish a stock varname outcome.

## Scope

Only separately pinned v2 report/write controls on fresh C8.6/C9.0/C9.1 objects, with original input and before/after namespace rows; unavailable setup on C8.4/C8.5/Jim. Generated object namespace numbers are actual presentation, not cross-control allocation identity. No private mapping, native object/header/cache or successful unrelated operation claim.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Actual compile0/process0; header/Makefile/library/executable and original input/stream digests retained.. Channel: Fresh counted setup source and original counted string-object argv. Result/options public getters and before/after namespace source inspection are labelled separately.. Dialect: Tcl.

TclOO class setup rejects; the selected method is not attempted.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Actual compile0/process0; header/Makefile/library/executable and original input/stream digests retained.. Channel: Fresh counted setup source and original counted string-object argv. Result/options public getters and before/after namespace source inspection are labelled separately.. Dialect: Tcl.

TclOO class setup rejects; the selected method is not attempted.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual compile0/process0; header/Makefile/library/executable and original input/stream digests retained.. Channel: Fresh counted setup source and original counted string-object argv. Result/options public getters and before/after namespace source inspection are labelled separately.. Dialect: Tcl.

Eight counted-name controls reached the selected stock method; exact key/local/result/error and partial state rows retained.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual compile0/process0; header/Makefile/library/executable and original input/stream digests retained.. Channel: Fresh counted setup source and original counted string-object argv. Result/options public getters and before/after namespace source inspection are labelled separately.. Dialect: Tcl.

Eight counted-name controls reached the selected stock method; exact key/local/result/error and partial state rows retained.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual compile0/process0; header/Makefile/library/executable and original input/stream digests retained.. Channel: Fresh counted setup source and original counted string-object argv. Result/options public getters and before/after namespace source inspection are labelled separately.. Dialect: Tcl.

Eight counted-name controls reached the selected stock method; exact key/local/result/error and partial state rows retained.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual compile0/process0; header/Makefile/library/executable and original input/stream digests retained.. Channel: Fresh counted setup source and original counted string-object argv. Result/options public getters and before/after namespace source inspection are labelled separately.. Dialect: Jim Tcl.

TclOO class setup rejects; the selected method is not attempted.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `v2-probe.c` (input): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/probe.c](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/probe.c). SHA-256 `c95b3e837673f3f473972509734666e32097cac3946ffa3cba34239701ca7090`. Exact immutable capture program/input/build association, with distinct attempted guest controls.
- `v2-inputs.json` (input): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/inputs.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/inputs.json). SHA-256 `79d0bd97c6fd032a790b6942c67b5c0d66c045b48d0c13e25fd1a097f1999c9e`. Exact immutable capture program/input/build association, with distinct attempted guest controls.
- `v2-queue.json` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/queue.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/queue.json). SHA-256 `78a238affd09d6ff5bfe30c6ea98285051d6878d31cd1ffe4c330cea2bd8e7a4`. Exact immutable capture program/input/build association, with distinct attempted guest controls.
- `v2-capture.py` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/capture.py](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/capture.py). SHA-256 `a40f26a345a7dceef40cb40ba9970347036d272e366220eef9bab9b4870be822`. Exact immutable capture program/input/build association, with distinct attempted guest controls.
- `v2-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/receipt.json). SHA-256 `bb299cda7ef130d625bf5330b3596b7b7e5f6d43c1247cce9ab0397aae15d83d`. Exact immutable capture program/input/build association, with distinct attempted guest controls.
- `v2-tcl8.4-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.4.20/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.4.20/receipt.json). SHA-256 `0c9c03d53bdaa853b1a41ac79bb815ddc75bfdf3d1b0d94460a53799bb188073`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl8.4-stdout.tsv` (observation): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.4.20/stdout.tsv](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.4.20/stdout.tsv). SHA-256 `770bfe6e5c486707ccd3c7b01e3b7df11328a6a73c55300aa0707ad57074a93a`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl8.4-stderr` (observation): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.4.20/stderr](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl8.4-compile.stdout` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.4.20/compile.stdout](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.4.20/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl8.4-compile.stderr` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.4.20/compile.stderr](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.4.20/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl8.5-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.5.19/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.5.19/receipt.json). SHA-256 `ffd514cac6d7aa9a924de7fb56ef55418ea73006eb5dbe0ae00bfcfc979856be`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl8.5-stdout.tsv` (observation): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.5.19/stdout.tsv](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.5.19/stdout.tsv). SHA-256 `7c7c4c44dcf566103aad3898944e4b9b9bad010705acb80310948b469b119190`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl8.5-stderr` (observation): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.5.19/stderr](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl8.5-compile.stdout` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.5.19/compile.stdout](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.5.19/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl8.5-compile.stderr` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.5.19/compile.stderr](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.5.19/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl8.6-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.6.18/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.6.18/receipt.json). SHA-256 `72e062e4cf06b2669fb933ba353763bb64a7749a36089e81b1b2997bfcf72c53`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl8.6-stdout.tsv` (observation): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.6.18/stdout.tsv](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.6.18/stdout.tsv). SHA-256 `d5abb9fe4d3eac73f4fbc5b50b26c6b49e1ddde895ceacd3a839d80e03f7873b`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl8.6-stderr` (observation): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.6.18/stderr](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl8.6-compile.stdout` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.6.18/compile.stdout](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.6.18/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl8.6-compile.stderr` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.6.18/compile.stderr](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/8.6.18/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl9.0-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.0.4/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.0.4/receipt.json). SHA-256 `8681c421ccd774525df96a15beffa88901491f333bd7775626b04d3de5bd8eae`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl9.0-stdout.tsv` (observation): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.0.4/stdout.tsv](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.0.4/stdout.tsv). SHA-256 `5d0c5e77586e32d06f4648180d201ecf13c4ef652cb32689fad6a923b26b0983`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl9.0-stderr` (observation): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.0.4/stderr](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl9.0-compile.stdout` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.0.4/compile.stdout](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.0.4/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl9.0-compile.stderr` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.0.4/compile.stderr](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.0.4/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl9.1-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.1.0/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.1.0/receipt.json). SHA-256 `71153f665a672400812e4a06c11976f57de8727d8b433cdb4c5da39a0c48ad1b`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl9.1-stdout.tsv` (observation): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.1.0/stdout.tsv](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.1.0/stdout.tsv). SHA-256 `77ade6895a6420725a4736399a7c37b0e90d70abfeb040f174749e8a0589459a`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl9.1-stderr` (observation): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.1.0/stderr](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl9.1-compile.stdout` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.1.0/compile.stdout](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.1.0/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-tcl9.1-compile.stderr` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.1.0/compile.stderr](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/9.1.0/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-jim-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/jim/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/jim/receipt.json). SHA-256 `c35f47310d033b8f74b354567ffaa1f30ac88979cea7b461ddc4401b8da4cc39`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-jim-stdout.tsv` (observation): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/jim/stdout.tsv](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/jim/stdout.tsv). SHA-256 `6fd3534086abf7f3febbfbf45251bb11157ead5dbbf90cd9e49073d90bf2027d`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-jim-stderr` (observation): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/jim/stderr](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-jim-compile.stdout` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/jim/compile.stdout](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/jim/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.
- `v2-jim-compile.stderr` (provider): [rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/jim/compile.stderr](../../../../rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/v2/jim/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual original setup/operation/input/result/options/state stream or matching build receipt. No omitted executable or Rust execution claim.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/interp/native_variable_names/native_tcloo.rs](../../../../rust/tcl-vm/src/interp/native_variable_names/native_tcloo.rs), `original_c_oo_varname`: Selected actual original-object namespace lookup and exact live cell/alias operation; availability, frame and method selection remain independent.
- [runtime/rust/src/interp/native_variable_names/native_tcloo.rs](../../../../runtime/rust/src/interp/native_variable_names/native_tcloo.rs), `original_c_oo_varname`: Selected actual original-object namespace lookup and exact live cell/alias operation; availability, frame and method selection remain independent.
- [rust/tcl-vm/src/cmd_oo/native_explicit_variable_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_explicit_variable_tests.rs), `cmd_oo::native_explicit_variable_tests::explicit_variable_links_and_varname_match_original_counted_controls` (linked): 72 operation and72 after-state C8.6/C9.0/C9.1 comparisons per port, with generated namespace presentation normalized; original counted argument bytes preserved. No options/header/Rust execution claim.
- [runtime/rust/src/cmd_oo/native_explicit_variable_tests.rs](../../../../runtime/rust/src/cmd_oo/native_explicit_variable_tests.rs), `cmd_oo::native_explicit_variable_tests::explicit_variable_links_and_varname_match_original_counted_controls` (linked): 72 operation and72 after-state C8.6/C9.0/C9.1 comparisons per port, with generated namespace presentation normalized; original counted argument bytes preserved. No options/header/Rust execution claim.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3 rust/tcl-cmd-core/tests/data/native_tcloo_explicit_variable/replay.py --verify-only"
]
```

Offline verify-only checked each immutable capture variant and all1740 protocol rows; zero native/compiler/Rust launches. Actual object namespace IDs belong to their individual fresh captures. V1 user varname recursion is a measured harness-input limitation, not guest unavailable. C8.4/Jim return-options C API not captured. Source inspection can be reproduced only by matching pinned full-file SHA and LF snippet windows; native runner cannot reinspect source.

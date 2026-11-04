# naming.substitution.original-options-and-flag-families

Kind: `native-observation`

## Problem statement

An original option object may contain binary zero, encoded zero or opaque bytes. A display conversion or shared full-count matcher can change its selected option and diagnostic. Tcl 9.1 also adds positive flag options whose defaults differ from negative options. Correct template flags require the actual option declaration and original getter/cache purpose; the following controls measure result code and bytes only.

## Question

How do direct counted original Subst option objects distinguish binary zero from C080/ff, and which engines accept positive flag options or mixed families?

## Conclusion

All six linked engines accept -novariables followed by binary zero and leave $x literal. Encoded C080 and raw ff suffixes fail with their original diagnostic bytes. Only C9.1 accepts the three positive options: -backslashes and -commands leave $x literal, while -variables returns VALUE. C9.1 rejects -variables combined with -nocommands. C8.4/C8.5 use the diagnostic noun switch; C8.6/C9/Jim use option. Native result bytes do not prove option-cache/header effects or Rust implementation parity.

## Scope

Original counted option objects through Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector; template $x and ASCII setup x=VALUE. Subst v1 has three option controls; v2 additionally has positive and mixed controls. No source numeric escapes, document ingress, internal-representation observation, errorCode or arbitrary template behavior is claimed.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual linked executable. Public header/static library/Makefile/source-owner/probe/stream hashes and compile command retained; executed info patchlevel reports this version.. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source.. Dialect: Tcl.

Binary-zero suffix matches; C080 and ff suffixes fail with exact diagnostic bytes. Positive options rejected; three-entry diagnostic with noun switch.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual linked executable. Public header/static library/Makefile/source-owner/probe/stream hashes and compile command retained; executed info patchlevel reports this version.. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source.. Dialect: Tcl.

Binary-zero suffix matches; C080 and ff suffixes fail with exact diagnostic bytes. Positive options rejected; three-entry diagnostic with noun switch.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual linked executable. Public header/static library/Makefile/source-owner/probe/stream hashes and compile command retained; executed info patchlevel reports this version.. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source.. Dialect: Tcl.

Binary-zero suffix matches; C080 and ff suffixes fail with exact diagnostic bytes. Positive options rejected; three-entry diagnostic with noun option.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual linked executable. Public header/static library/Makefile/source-owner/probe/stream hashes and compile command retained; executed info patchlevel reports this version.. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source.. Dialect: Tcl.

Binary-zero suffix matches; C080 and ff suffixes fail with exact diagnostic bytes. Positive options rejected; three-entry diagnostic with noun option.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual linked executable. Public header/static library/Makefile/source-owner/probe/stream hashes and compile command retained; executed info patchlevel reports this version.. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source.. Dialect: Tcl.

Binary-zero suffix matches; C080 and ff suffixes fail with exact diagnostic bytes. All three positive options accepted; mixed families rejected; six-entry option diagnostic.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual linked executable. Public header/static library/Makefile/source-owner/probe/stream hashes and compile command retained; executed info patchlevel reports this version.. Channel: Original counted string objects passed directly to public Jim_EvalObjVector; ASCII Jim_Eval setup/version source.. Dialect: Jim Tcl.

Binary-zero suffix matches; C080 and ff suffixes fail with exact diagnostic bytes. Positive options rejected; three-entry diagnostic with noun option.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation for these exact inputs.

## Historical references

- `v1-input` (input): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/probe.c](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/probe.c). Complete exact C/Jim probe source. Counted object construction and each immediate result getter are explicit. Separate variants are not declared identical.
- `v1-aggregate` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/receipt.json). Exact complete original six-provider receipts for this probe variant.
- `v2-input` (input): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/probe.c](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/probe.c). Complete exact C/Jim probe source. Counted object construction and each immediate result getter are explicit. Separate variants are not declared identical.
- `v2-aggregate` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/receipt.json). Exact complete original six-provider receipts for this probe variant.
- `v1-tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.4.20/receipt.json). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl8.4-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.4.20/stdout.tsv). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.4.20/stderr). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.4.20/receipt.json). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl8.4-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.4.20/stdout.tsv). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.4.20/stderr). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.5.19/receipt.json). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl8.5-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.5.19/stdout.tsv). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.5.19/stderr). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.5.19/receipt.json). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl8.5-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.5.19/stdout.tsv). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.5.19/stderr). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.6.18/receipt.json). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl8.6-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.6.18/stdout.tsv). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/8.6.18/stderr). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.6.18/receipt.json). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl8.6-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.6.18/stdout.tsv). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/8.6.18/stderr). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.0.4/receipt.json). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl9.0-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.0.4/stdout.tsv). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.0.4/stderr). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.0.4/receipt.json). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl9.0-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.0.4/stdout.tsv). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.0.4/stderr). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.1.0/receipt.json). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl9.1-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.1.0/stdout.tsv). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/9.1.0/stderr). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.1.0/receipt.json). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl9.1-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.1.0/stdout.tsv). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/9.1.0/stderr). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/jim/receipt.json). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-jim-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/jim/stdout.tsv). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v1-jim-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v1/jim/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1/jim/stderr). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/jim/receipt.json). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-jim-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/jim/stdout.tsv). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.
- `v2-jim-stderr` (observation): [rust/tcl-vm/tests/data/native_subst_catch_operands/v2/jim/stderr](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2/jim/stderr). Exact original provider/build/process receipt or complete native process stream. A code 1 guest result is expected data, independently of process exit 0.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/substitution.rs](../../../../rust/tcl-registry/src/substitution.rs), `NativeSubstitutionOptions`: Select the actual option table and fold canonical flag families; no object-cache or handler registration grant.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `cmd_subst`: Consume original option objects then retain selected template flags.
- [runtime/rust/src/builtins.rs](../../../../runtime/rust/src/builtins.rs), `subst_cmd`: Consume original option objects through the independent selected static index/enum owner.
- [rust/tcl-vm/src/command/native_subst_operand_tests.rs](../../../../rust/tcl-vm/src/command/native_subst_operand_tests.rs), `command::native_subst_operand_tests::original_subst_options_match_counted_native_families` (linked): Compare exact captured result code and native result bytes for original byte inputs; pure Registry test independently checks available authored table parity/flag folding. No native observation alone asserts Rust execution.
- [runtime/rust/src/builtins/native_subst_operand_tests.rs](../../../../runtime/rust/src/builtins/native_subst_operand_tests.rs), `builtins::native_subst_operand_tests::original_subst_options_match_counted_native_families` (linked): Compare exact captured result code and native result bytes for original byte inputs; pure Registry test independently checks available authored table parity/flag folding. No native observation alone asserts Rust execution.
- [rust/tcl-registry/src/substitution.rs](../../../../rust/tcl-registry/src/substitution.rs), `substitution::tests::native_runtime_option_declaration_matches_registry_surface_and_family_fold` (linked): Compare exact captured result code and native result bytes for original byte inputs; pure Registry test independently checks available authored table parity/flag folding. No native observation alone asserts Rust execution.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-vm/tests/data/native_subst_catch_operands/replay.py",
  "--c-root",
  "tmp",
  "--jim-root",
  "/workspace/.proofs/native-providers/jimtcl",
  "--output",
  "/tmp/native-subst-catch-reconfirmation"
]
```

Requires exact pinned probe/public header/static library/Makefile/source-owner and retained stream hashes. Recompiles both independent variants and compares complete exit/stdout/stderr; guest result errors are expected. --verify-only checks correspondence without any native or Rust execution. This question has no cache/header/errorCode windows.

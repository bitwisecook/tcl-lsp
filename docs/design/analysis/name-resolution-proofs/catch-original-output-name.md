# naming.variable.catch-original-output-name

Kind: `native-observation`

## Problem statement

A catch output name can be opaque native bytes even when its display is invalid or contains zero. Reconstructing it from a display can write the replacement-character name or clip a counted operand. The output result, plain-name sentinel and replacement-character sentinel must be checked separately, and the old C8.4 setter extent cannot be generalized to later engines.

## Question

Where does public catch store BOOM when its original output name contains ff, binary zero or encoded C080, and do plain or replacement-character sibling cells change?

## Conclusion

All six providers return catch status 1 and reading the exact original output name returns BOOM. The replacement-character sibling remains REPLACEMENT. Raw ff and C080 preserve the plain r sentinel on all six. With binary zero, only C8.4 changes plain r to BOOM; C8.5/C8.6/C9.0/C9.1/Jim keep plain r as PLAIN. These are direct output-setter/result observations, not alias, trace, normal-completion, header/cache or complete namespace laws.

## Scope

Each fresh interpreter starts with r=PLAIN and r plus U+FFFD UTF8 bytes=REPLACEMENT. Public catch receives original counted r+ff, r+00+tail or r+C080+tail and ASCII error BOOM body. Each result is inspected before the next command replaces it. No optionVarName, callbacks, aliases, array names or missing namespace is tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual linked executable. Public header/static library/Makefile/source-owner/probe/stream hashes and compile command retained; executed info patchlevel reports this version.. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source.. Dialect: Tcl.

All exact output reads BOOM; replacement sibling unchanged. Binary-zero output also changes plain r to BOOM. Raw ff and C080 keep the plain sentinel.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual linked executable. Public header/static library/Makefile/source-owner/probe/stream hashes and compile command retained; executed info patchlevel reports this version.. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source.. Dialect: Tcl.

All exact output reads BOOM; replacement sibling unchanged. Binary-zero output leaves plain r as PLAIN. Raw ff and C080 keep the plain sentinel.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual linked executable. Public header/static library/Makefile/source-owner/probe/stream hashes and compile command retained; executed info patchlevel reports this version.. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source.. Dialect: Tcl.

All exact output reads BOOM; replacement sibling unchanged. Binary-zero output leaves plain r as PLAIN. Raw ff and C080 keep the plain sentinel.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual linked executable. Public header/static library/Makefile/source-owner/probe/stream hashes and compile command retained; executed info patchlevel reports this version.. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source.. Dialect: Tcl.

All exact output reads BOOM; replacement sibling unchanged. Binary-zero output leaves plain r as PLAIN. Raw ff and C080 keep the plain sentinel.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual linked executable. Public header/static library/Makefile/source-owner/probe/stream hashes and compile command retained; executed info patchlevel reports this version.. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source.. Dialect: Tcl.

All exact output reads BOOM; replacement sibling unchanged. Binary-zero output leaves plain r as PLAIN. Raw ff and C080 keep the plain sentinel.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual linked executable. Public header/static library/Makefile/source-owner/probe/stream hashes and compile command retained; executed info patchlevel reports this version.. Channel: Original counted string objects passed directly to public Jim_EvalObjVector; ASCII Jim_Eval setup/version source.. Dialect: Jim Tcl.

All exact output reads BOOM; replacement sibling unchanged. Binary-zero output leaves plain r as PLAIN. Raw ff and C080 keep the plain sentinel.

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

- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `finish_catch`: Store through the original result-variable object using the selected native setter purpose.
- [rust/tcl-vm/src/command/native_subst_operand_tests.rs](../../../../rust/tcl-vm/src/command/native_subst_operand_tests.rs), `command::native_subst_operand_tests::catch_original_output_names_match_counted_native_controls` (linked): Compare exact catch result codes and counted output-name reads, plain/replacement sibling values and native result bytes for the three independent original byte inputs. No option table, alias, observer, normal completion or Rust execution is inferred.
- [runtime/rust/src/builtins/native_subst_operand_tests.rs](../../../../runtime/rust/src/builtins/native_subst_operand_tests.rs), `builtins::native_subst_operand_tests::catch_original_output_names_match_counted_native_controls` (linked): Compare exact catch result codes and counted output-name reads, plain/replacement sibling values and native result bytes for the three independent original byte inputs. No option table, alias, observer, normal completion or Rust execution is inferred.

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

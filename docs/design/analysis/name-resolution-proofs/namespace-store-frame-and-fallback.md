# naming.variable.namespace-store-frame-and-fallback

Kind: `native-observation`

## Problem statement

A bare Set inside namespace eval can write an existing root variable, a namespace variable or an activation-local variable. Inferring a namespace target from the written scope alone can publish the wrong source symbol. The fixed root-present, root-absent and explicit-variable controls distinguish these alternatives without relying on an earlier substitution setup.

## Question

Where does a bare namespace-eval Set write x with an existing root x or no root x, and how does explicit variable x VALUE change that target on the six tested providers?

## Conclusion

C8.4-C8.6 reuses an existing root x and creates namespace x only when the root is absent; C9.0/C9.1 creates namespace x even when root x exists. Jim bare Set creates an activation-local x and leaves both namespace global x and existing root x unchanged. Explicit variable creates namespace x on all six. These are the fixed ASCII namespace-entry/store/read controls, not arbitrary resolver or trace closure.

## Scope

Three caught ASCII source programs supplied as native shell stdin; actual info patchlevel, exact input and streams, executable/header/library/Makefile/source-owner hashes retained. Initial namespace names are fresh. No raw zero, physical variable header, compiler admission, alias or observer claim.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual shell executable SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a; independent original header/library/Makefile/source-owner hashes retained. Actual info patchlevel result is retained.. Channel: ASCII file bytes supplied to native shell stdin. Dialect: Tcl.

EXISTING_ROOT: bare Set updates root to VALUE and ::N::x is absent. FRESH_NO_ROOT: ::Fresh::x=VALUE and root is absent. EXPLICIT_VARIABLE: namespace x=VALUE and root stays ROOT.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual shell executable SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a; independent original header/library/Makefile/source-owner hashes retained. Actual info patchlevel result is retained.. Channel: ASCII file bytes supplied to native shell stdin. Dialect: Tcl.

EXISTING_ROOT: bare Set updates root to VALUE and ::N::x is absent. FRESH_NO_ROOT: ::Fresh::x=VALUE and root is absent. EXPLICIT_VARIABLE: namespace x=VALUE and root stays ROOT.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual shell executable SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff; independent original header/library/Makefile/source-owner hashes retained. Actual info patchlevel result is retained.. Channel: ASCII file bytes supplied to native shell stdin. Dialect: Tcl.

EXISTING_ROOT: bare Set updates root to VALUE and ::N::x is absent. FRESH_NO_ROOT: ::Fresh::x=VALUE and root is absent. EXPLICIT_VARIABLE: namespace x=VALUE and root stays ROOT.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual shell executable SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1; independent original header/library/Makefile/source-owner hashes retained. Actual info patchlevel result is retained.. Channel: ASCII file bytes supplied to native shell stdin. Dialect: Tcl.

EXISTING_ROOT: ::N::x=VALUE and root stays ROOT. FRESH_NO_ROOT: namespace x=VALUE and root is absent. EXPLICIT_VARIABLE: namespace x=VALUE and root stays ROOT.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual shell executable SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d; independent original header/library/Makefile/source-owner hashes retained. Actual info patchlevel result is retained.. Channel: ASCII file bytes supplied to native shell stdin. Dialect: Tcl.

EXISTING_ROOT: ::N::x=VALUE and root stays ROOT. FRESH_NO_ROOT: namespace x=VALUE and root is absent. EXPLICIT_VARIABLE: namespace x=VALUE and root stays ROOT.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual shell executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; independent original header/library/Makefile/source-owner hashes retained. Actual info patchlevel result is retained.. Channel: ASCII file bytes supplied to native shell stdin. Dialect: Jim Tcl.

Bare Set creates activation-local x: namespace global x is absent in both cases and an existing root stays ROOT. Explicit variable creates ::Explicit::x=VALUE and leaves root ROOT.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation for this exact question.

## Exact evidence

- `input` (input): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/namespace-cases.tcl](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/namespace-cases.tcl). SHA-256 `f920581da05d2bd8e52dfd325c86ec5b0aa3c54e219f3d491ab440af00b56469`. Exact immutable native source input.
- `tcl8.4-receipt.json` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20/receipt.json). SHA-256 `3894a288ecf53e55462db6326668d1e2fbff60b0ff0dbab0395b5f2a93feb624`. JSON pointer `/namespace`. Actual original provider/source/header/library/Makefile/executable/process correspondence.
- `tcl8.4-namespace.stdout` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20/namespace.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20/namespace.stdout). SHA-256 `3c1d6b59c4ebd3dee321fb9c0cacd88b77a6dca26cb2fcf8f88ca3aa15ea2de1`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl8.4-namespace.stderr` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20/namespace.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20/namespace.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl8.5-receipt.json` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19/receipt.json). SHA-256 `748bcd7b740b18871429681b33ff65cd542093889f86488140fee3db969317ea`. JSON pointer `/namespace`. Actual original provider/source/header/library/Makefile/executable/process correspondence.
- `tcl8.5-namespace.stdout` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19/namespace.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19/namespace.stdout). SHA-256 `1f5efecf7157da289c48936c339da3ea9e94c076c802f9e23de7d4f571b7ef42`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl8.5-namespace.stderr` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19/namespace.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19/namespace.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl8.6-receipt.json` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18/receipt.json). SHA-256 `7f8d85a9efaa740768d94faf3d442da13951ddc251b4824a41a55c6f28a5a04c`. JSON pointer `/namespace`. Actual original provider/source/header/library/Makefile/executable/process correspondence.
- `tcl8.6-namespace.stdout` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18/namespace.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18/namespace.stdout). SHA-256 `bf367e4fed2af3f916cd19ae6fe7cb10cae1ad0728200e1a8eaa1ffcce284341`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl8.6-namespace.stderr` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18/namespace.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18/namespace.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl9.0-receipt.json` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4/receipt.json). SHA-256 `221390c1d3e2f71689b649043e485da290603d5ae7266f9465109002722f7514`. JSON pointer `/namespace`. Actual original provider/source/header/library/Makefile/executable/process correspondence.
- `tcl9.0-namespace.stdout` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4/namespace.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4/namespace.stdout). SHA-256 `046e993f7d4e1608758514115c7c05843915e6c4a944266e78076704a3a6e0b1`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl9.0-namespace.stderr` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4/namespace.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4/namespace.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl9.1-receipt.json` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0/receipt.json). SHA-256 `d80dd5e75bedca0c65f99420f2451069ab0d301522002fc0204bbf3c341e815e`. JSON pointer `/namespace`. Actual original provider/source/header/library/Makefile/executable/process correspondence.
- `tcl9.1-namespace.stdout` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0/namespace.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0/namespace.stdout). SHA-256 `213f8d4f073eaa5a74d898bbd068076aab0418f6a496bd80210699958c9de7bb`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl9.1-namespace.stderr` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0/namespace.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0/namespace.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `jim-receipt.json` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim/receipt.json). SHA-256 `9d3d471970950421fc1b5155d444a40b2338bb82397156b03a27d6485ce26718`. JSON pointer `/namespace`. Actual original provider/source/header/library/Makefile/executable/process correspondence.
- `jim-namespace.stdout` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim/namespace.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim/namespace.stdout). SHA-256 `2e327cfdf9f5cd6209eb251d7d984a9d66327288efcb7beed980e401e93922d9`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `jim-namespace.stderr` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim/namespace.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim/namespace.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/cmd_namespace.rs](../../../../rust/tcl-vm/src/cmd_namespace.rs), `eval_in_ns`: Enter the actual selected namespace frame before variable operations; the invocation does not itself prove a variable address.
- [rust/tcl-vm/src/cmd_namespace/native_store_tests.rs](../../../../rust/tcl-vm/src/cmd_namespace/native_store_tests.rs), `cmd_namespace::native_store_tests::namespace_stores_match_native_existing_root_fresh_and_declared_controls` (linked): Compare all18 native namespace-store result rows using independent actual engine fixtures, root-present/absent initialization and original native namespace script objects.
- [rust/tcl-compiler/src/var_resolve/original_bytes.rs](../../../../rust/tcl-compiler/src/var_resolve/original_bytes.rs), `var_resolve::original_bytes::tests::namespace_runtime_root_requires_current_or_global_table_membership` (linked): Independent current/global inventory for ASCII x distinguishes C8 existing-root fallback from C9 namespace selection and declines missing membership; opaque-byte equality is separate Rust correspondence beyond the measured ASCII native controls.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/replay.py",
  "--c-root",
  "tmp",
  "--jim-root",
  "/workspace/.proofs/native-providers/jimtcl",
  "--output",
  "/tmp/native-namespace-store-trace-subject-reconfirmation"
]
```

Requires exact provider shell/header/static-library/source-owner/Makefile hashes and immutable input/probe bytes. Compiles the public probe, compares complete stdout/stderr and exit for both inputs. Guest errors are data; nonzero harness exit or stream mismatch fails replay. --verify-only checks source/provider/stream associations without compiling or launching native code. No Rust execution claim.

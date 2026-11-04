# naming.source.argument-replacement-body-data

Kind: `native-observation`

## Problem statement

If the selected runtime handler treats an advisory body operand as data, changing descendant separators can change a result even when the enclosing word remains valid source syntax.

## Question

Does the proposed body-separator-only source change preserve the custom handler result after the tested argument substitution replaces if?

## Conclusion

The original and proposed body-separator candidate both complete successfully on all six tested providers, but their counted result bytes differ. Each result equals the exact body bytes independently retained by the replacement custom handler. The difference is observed in both explicit C evaluation modes and in Jim source evaluation. The candidate is proposed source, not captured output of the minifier, and this observation does not prove general rewrite equivalence or failure.

## Scope

The two ASCII scripts differ only in separators inside the outer braced body operand. Literal data VALUE retains its internal spaces. Result and custom-body bytes are sampled independently after fresh source evaluation. C Tcl_EvalEx flags0 and TCL_EVAL_DIRECT are separate controls; Jim Direct remains unavailable.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual executable; exact compile argv, public header/library/build/source hashes and process streams remain in the original provider receipt.. Channel: Original ASCII source passed to Tcl_EvalEx with explicit flags0 and TCL_EVAL_DIRECT; independent ASCII post-evaluation queries.. Dialect: Tcl.

Both source cases complete0 in both actual C modes, with different counted result bytes. Each result equals that case's independently captured custom body. The candidate is proposed source, not actual minifier output.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable; exact compile argv, public header/library/build/source hashes and process streams remain in the original provider receipt.. Channel: Original ASCII source passed to Tcl_EvalEx with explicit flags0 and TCL_EVAL_DIRECT; independent ASCII post-evaluation queries.. Dialect: Tcl.

Both source cases complete0 in both actual C modes, with different counted result bytes. Each result equals that case's independently captured custom body. The candidate is proposed source, not actual minifier output.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable; exact compile argv, public header/library/build/source hashes and process streams remain in the original provider receipt.. Channel: Original ASCII source passed to Tcl_EvalEx with explicit flags0 and TCL_EVAL_DIRECT; independent ASCII post-evaluation queries.. Dialect: Tcl.

Both source cases complete0 in both actual C modes, with different counted result bytes. Each result equals that case's independently captured custom body. The candidate is proposed source, not actual minifier output.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable; exact compile argv, public header/library/build/source hashes and process streams remain in the original provider receipt.. Channel: Original ASCII source passed to Tcl_EvalEx with explicit flags0 and TCL_EVAL_DIRECT; independent ASCII post-evaluation queries.. Dialect: Tcl.

Both source cases complete0 in both actual C modes, with different counted result bytes. Each result equals that case's independently captured custom body. The candidate is proposed source, not actual minifier output.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable; exact compile argv, public header/library/build/source hashes and process streams remain in the original provider receipt.. Channel: Original ASCII source passed to Tcl_EvalEx with explicit flags0 and TCL_EVAL_DIRECT; independent ASCII post-evaluation queries.. Dialect: Tcl.

Both source cases complete0 in both actual C modes, with different counted result bytes. Each result equals that case's independently captured custom body. The candidate is proposed source, not actual minifier output.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual executable; exact compile argv, public header/library/build/source hashes and process streams remain in the original provider receipt.. Channel: Original ASCII source passed to Jim_Eval; independent ASCII post-evaluation queries. No Jim Direct recipe.. Dialect: Jim Tcl.

Both source cases complete0 under Jim_Eval, with different counted result bytes. Each result equals that case's independently captured custom body. The candidate is proposed source, not actual minifier output; Jim Direct is unavailable.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance capture for this exact source-handler replacement and proposed-body question.

## Historical references

- `probe` (input): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/probe.c](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/probe.c). Exact original and proposed body-separator candidate source strings, fresh interpreter per case and explicit Tcl_EvalEx mode or Jim_Eval driver.
- `inputs` (input): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/inputs.json](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/inputs.json). Exact ASCII source cases; the candidate is proposed source, not observed output of a minifier.
- `aggregate` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/receipt.json](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/receipt.json). Actual all-six compile/process/header/library/source/build/startup/executable/stream correspondence.
- `queue` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/queue.json](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/queue.json). Immutable original provider pins, source inputs, compilation flags and captured output destination.
- `capture-runner` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/capture.py](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/capture.py). Exact original Root-run capture harness; its absolute paths are archival.
- `replay` (implementation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/replay.py](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/replay.py). Standalone retained-byte verifier and optional fresh capture using the exact pinned provider paths. Offline verification makes no fresh native execution claim.
- `tcl8.4-receipt.json` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/receipt.json](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/receipt.json). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.4-stdout.tsv` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/stdout.tsv](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/stdout.tsv). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.4-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/stderr). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.4-compile.stdout` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/compile.stdout](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/compile.stdout). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.4-compile.stderr` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/compile.stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/compile.stderr). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.5-receipt.json` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/receipt.json](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/receipt.json). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.5-stdout.tsv` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/stdout.tsv](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/stdout.tsv). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.5-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/stderr). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.5-compile.stdout` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/compile.stdout](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/compile.stdout). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.5-compile.stderr` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/compile.stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/compile.stderr). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.6-receipt.json` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/receipt.json](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/receipt.json). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.6-stdout.tsv` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/stdout.tsv](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/stdout.tsv). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.6-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/stderr). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.6-compile.stdout` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/compile.stdout](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/compile.stdout). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.6-compile.stderr` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/compile.stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/compile.stderr). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.0-receipt.json` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/receipt.json](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/receipt.json). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.0-stdout.tsv` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/stdout.tsv](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/stdout.tsv). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.0-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/stderr). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.0-compile.stdout` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/compile.stdout](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/compile.stdout). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.0-compile.stderr` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/compile.stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/compile.stderr). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.1-receipt.json` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/receipt.json](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/receipt.json). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.1-stdout.tsv` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/stdout.tsv](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/stdout.tsv). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.1-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/stderr). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.1-compile.stdout` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/compile.stdout](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/compile.stdout). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.1-compile.stderr` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/compile.stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/compile.stderr). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `jim-receipt.json` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/receipt.json](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/receipt.json). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `jim-stdout.tsv` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/stdout.tsv](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/stdout.tsv). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `jim-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/stderr). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `jim-compile.stdout` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/compile.stdout](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/compile.stdout). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `jim-compile.stderr` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/compile.stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/compile.stderr). Original immutable provider receipt or process stream, preserved without rewriting capture bytes.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/source_structure.rs](../../../../rust/tcl-lsp-core/src/source_structure.rs), `SourceStructure`: Keep measured body-as-data discriminator separate from source geometry and closed descendant traversal.
- [rust/tcl-lsp-core/src/source_structure.rs](../../../../rust/tcl-lsp-core/src/source_structure.rs), `source_structure::syntax_compaction_tests::original_syntax_regions_reject_body_roles_when_arguments_replace_the_head` (linked): The matched original source remains readonly advisory body geometry, while the separate closed syntax traversal excludes that outer body and retains the authentic lexical substitution. This binding does not claim native Rust execution.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-lsp-core/tests/data/native_minifier_script_role/replay.py",
  "--verify-only"
]
```

Offline verification checks exact retained input/stream hashes and finite control rows without launching a native process. Fresh capture requires the exact provider files at the archived pin paths and --output to a new absent directory; it records new attempts separately. Jim has no Direct control. No minifier or Rust test execution is inferred.

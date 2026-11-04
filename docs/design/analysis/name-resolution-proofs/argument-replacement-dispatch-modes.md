# naming.source.argument-replacement-dispatch-modes

Kind: `native-observation`

## Problem statement

A source head selected before argument evaluation can be replaced by an argument substitution. Compiled source and direct evaluation must be measured separately before treating a Registry body role as stable.

## Question

Which handler receives the following body operand after the tested substitution replaces if, in explicit C ScriptCode flags0 and Direct evaluation and in the available Jim source driver?

## Conclusion

All five tested C releases invoke the replacement custom if handler in both Tcl_EvalEx flags0 and TCL_EVAL_DIRECT for both retained inputs. The independent markers report custom handler1 and builtin body0, and the command-table query reports if and saved. The current Jim source driver has the same observed selection; a Jim TCL_EVAL_DIRECT recipe is unavailable. These finite results do not describe all compiler specialisation or dispatch behaviour.

## Scope

Exact ASCII source strings evaluated in a fresh interpreter per mode and case, followed by separate handler-marker, captured-body and command-table queries. C mode flags are actual public API arguments. Jim uses Jim_Eval only and records Direct unavailable. No source execution mode is inferred from an Unknown compilation label.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual executable SHA256 da5ff6a82c77e5e2459a2d3b0bf006667667f5b658f6716471715f6e4acbf308; exact compile argv, public header/library/build/source hashes and process streams remain in the original provider receipt.. Channel: Original ASCII source passed to Tcl_EvalEx with explicit flags0 and TCL_EVAL_DIRECT; independent ASCII post-evaluation queries.. Dialect: Tcl.

Both actual C modes select the replacement custom handler for both source cases: custom marker1, builtin marker0, command-table result if saved.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable SHA256 4a29cb2905698f1f4d18784510ec20bdecbffc696f68d404b349cd695b934930; exact compile argv, public header/library/build/source hashes and process streams remain in the original provider receipt.. Channel: Original ASCII source passed to Tcl_EvalEx with explicit flags0 and TCL_EVAL_DIRECT; independent ASCII post-evaluation queries.. Dialect: Tcl.

Both actual C modes select the replacement custom handler for both source cases: custom marker1, builtin marker0, command-table result if saved.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable SHA256 cf07c855ee57ddbbe9547e294c67b11ffb537dd8e166f53c877aa365ec4276d7; exact compile argv, public header/library/build/source hashes and process streams remain in the original provider receipt.. Channel: Original ASCII source passed to Tcl_EvalEx with explicit flags0 and TCL_EVAL_DIRECT; independent ASCII post-evaluation queries.. Dialect: Tcl.

Both actual C modes select the replacement custom handler for both source cases: custom marker1, builtin marker0, command-table result if saved.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA256 bce0211f3fe6e82fa9c7635f1563f9f72b5aadd2e9fdcc8362a018553df04688; exact compile argv, public header/library/build/source hashes and process streams remain in the original provider receipt.. Channel: Original ASCII source passed to Tcl_EvalEx with explicit flags0 and TCL_EVAL_DIRECT; independent ASCII post-evaluation queries.. Dialect: Tcl.

Both actual C modes select the replacement custom handler for both source cases: custom marker1, builtin marker0, command-table result if saved.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA256 e079cd275751e7e5a100ec2157e3dcc7bf5c1479aa7ff2a69ab42bd0c6047fdf; exact compile argv, public header/library/build/source hashes and process streams remain in the original provider receipt.. Channel: Original ASCII source passed to Tcl_EvalEx with explicit flags0 and TCL_EVAL_DIRECT; independent ASCII post-evaluation queries.. Dialect: Tcl.

Both actual C modes select the replacement custom handler for both source cases: custom marker1, builtin marker0, command-table result if saved.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA256 4ab94466217e20118deebf0d5fcdd14cc98488bb1accbfe8bf27ed4a62e0fa5f; exact compile argv, public header/library/build/source hashes and process streams remain in the original provider receipt.. Channel: Original ASCII source passed to Jim_Eval; independent ASCII post-evaluation queries. No Jim Direct recipe.. Dialect: Jim Tcl.

Jim_Eval selects the replacement custom handler for both source cases: custom marker1, builtin marker0, command-table result if saved. A Jim TCL_EVAL_DIRECT recipe is unavailable and was not tested.

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

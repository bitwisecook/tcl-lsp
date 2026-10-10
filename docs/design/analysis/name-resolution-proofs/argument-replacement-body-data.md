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

Status: `observed`. Version: 8.4.20. Build: Actual executable SHA256 da5ff6a82c77e5e2459a2d3b0bf006667667f5b658f6716471715f6e4acbf308; exact compile argv, public header/library/build/source hashes and process streams remain in the original provider receipt.. Channel: Original ASCII source passed to Tcl_EvalEx with explicit flags0 and TCL_EVAL_DIRECT; independent ASCII post-evaluation queries.. Dialect: Tcl.

Both source cases complete0 in both actual C modes, with different counted result bytes. Each result equals that case's independently captured custom body. The candidate is proposed source, not actual minifier output.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual executable SHA256 4a29cb2905698f1f4d18784510ec20bdecbffc696f68d404b349cd695b934930; exact compile argv, public header/library/build/source hashes and process streams remain in the original provider receipt.. Channel: Original ASCII source passed to Tcl_EvalEx with explicit flags0 and TCL_EVAL_DIRECT; independent ASCII post-evaluation queries.. Dialect: Tcl.

Both source cases complete0 in both actual C modes, with different counted result bytes. Each result equals that case's independently captured custom body. The candidate is proposed source, not actual minifier output.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual executable SHA256 cf07c855ee57ddbbe9547e294c67b11ffb537dd8e166f53c877aa365ec4276d7; exact compile argv, public header/library/build/source hashes and process streams remain in the original provider receipt.. Channel: Original ASCII source passed to Tcl_EvalEx with explicit flags0 and TCL_EVAL_DIRECT; independent ASCII post-evaluation queries.. Dialect: Tcl.

Both source cases complete0 in both actual C modes, with different counted result bytes. Each result equals that case's independently captured custom body. The candidate is proposed source, not actual minifier output.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual executable SHA256 bce0211f3fe6e82fa9c7635f1563f9f72b5aadd2e9fdcc8362a018553df04688; exact compile argv, public header/library/build/source hashes and process streams remain in the original provider receipt.. Channel: Original ASCII source passed to Tcl_EvalEx with explicit flags0 and TCL_EVAL_DIRECT; independent ASCII post-evaluation queries.. Dialect: Tcl.

Both source cases complete0 in both actual C modes, with different counted result bytes. Each result equals that case's independently captured custom body. The candidate is proposed source, not actual minifier output.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual executable SHA256 e079cd275751e7e5a100ec2157e3dcc7bf5c1479aa7ff2a69ab42bd0c6047fdf; exact compile argv, public header/library/build/source hashes and process streams remain in the original provider receipt.. Channel: Original ASCII source passed to Tcl_EvalEx with explicit flags0 and TCL_EVAL_DIRECT; independent ASCII post-evaluation queries.. Dialect: Tcl.

Both source cases complete0 in both actual C modes, with different counted result bytes. Each result equals that case's independently captured custom body. The candidate is proposed source, not actual minifier output.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA256 4ab94466217e20118deebf0d5fcdd14cc98488bb1accbfe8bf27ed4a62e0fa5f; exact compile argv, public header/library/build/source hashes and process streams remain in the original provider receipt.. Channel: Original ASCII source passed to Jim_Eval; independent ASCII post-evaluation queries. No Jim Direct recipe.. Dialect: Jim Tcl.

Both source cases complete0 under Jim_Eval, with different counted result bytes. Each result equals that case's independently captured custom body. The candidate is proposed source, not actual minifier output; Jim Direct is unavailable.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance capture for this exact source-handler replacement and proposed-body question.

## Exact evidence

- `probe` (input): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/probe.c](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/probe.c). SHA-256 `23b4ad86768d46d40519dd955aaab82eb0d35a86cdee47f8dffed25b083620a7`. Exact original and proposed body-separator candidate source strings, fresh interpreter per case and explicit Tcl_EvalEx mode or Jim_Eval driver.
- `inputs` (input): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/inputs.json](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/inputs.json). SHA-256 `955d76bd180104e99ecdfd71b99b77579a48ad0339b14067e0a8c622bd0bf1f5`. Exact ASCII source cases; the candidate is proposed source, not observed output of a minifier.
- `aggregate` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/receipt.json](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/receipt.json). SHA-256 `a6eb5c0e1fcf50cca8c261fa901e62a50d80c62b605f092478ab3f1acf9957f4`. Actual all-six compile/process/header/library/source/build/startup/executable/stream correspondence.
- `queue` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/queue.json](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/queue.json). SHA-256 `ccfd30652ed1276158a591e92756d1b29baa40ddb47f0851e033f37c090d8c97`. Immutable original provider pins, source inputs, compilation flags and captured output destination.
- `capture-runner` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/capture.py](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/capture.py). SHA-256 `ce9f84ab41ba5d4f2708aa7a327adf0c2b3f00aed536346c646a130944b3e114`. Exact original Root-run capture harness; its absolute paths are archival.
- `replay` (implementation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/replay.py](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/replay.py). SHA-256 `29b7f7275a06362b4d609d8040db101d9ab35e6c18a13e165cf47bf8a41a0426`. Standalone retained-byte verifier and optional fresh capture using the exact pinned provider paths. Offline verification makes no fresh native execution claim.
- `tcl8.4-receipt.json` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/receipt.json](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/receipt.json). SHA-256 `496afdcf56ace2e1786626e2d90b2c35f47ded8ea6cb2ea2aa4b64171736889e`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.4-stdout.tsv` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/stdout.tsv](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/stdout.tsv). SHA-256 `2cb89d163c5c37172ba7e4e80e5f28469135475f5344123ed4a2354ef0f2137a`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.4-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.4-compile.stdout` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/compile.stdout](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.4-compile.stderr` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/compile.stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.4.20/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.5-receipt.json` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/receipt.json](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/receipt.json). SHA-256 `24596d53c47b3469afb815583481d6d2ee115091a4398c452d0e739ed60a596a`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.5-stdout.tsv` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/stdout.tsv](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/stdout.tsv). SHA-256 `edacfee490b152bd9d728f692223f928f7de727eb9f2325695f96ee594986e49`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.5-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.5-compile.stdout` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/compile.stdout](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.5-compile.stderr` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/compile.stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.5.19/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.6-receipt.json` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/receipt.json](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/receipt.json). SHA-256 `b4ce89cc4c6c004a3d48b5bcefd21d8d293624d694ed81d7eb0b22960c884b50`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.6-stdout.tsv` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/stdout.tsv](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/stdout.tsv). SHA-256 `c846ef5b1ff06951503181178acba25ac408aaeb4a398f54e26c20af5e54a2cd`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.6-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.6-compile.stdout` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/compile.stdout](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl8.6-compile.stderr` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/compile.stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/8.6.18/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.0-receipt.json` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/receipt.json](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/receipt.json). SHA-256 `3103d82681ac16885cf65440f4987cd9c682cfde5a4f8b0c306f7bf4cbbb3108`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.0-stdout.tsv` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/stdout.tsv](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/stdout.tsv). SHA-256 `c3144d587b50ef173703ccf1a5c477e2d66abd3aa064e8236b71f977682fd8bc`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.0-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.0-compile.stdout` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/compile.stdout](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.0-compile.stderr` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/compile.stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.0.4/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.1-receipt.json` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/receipt.json](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/receipt.json). SHA-256 `b6bf628105594c9774aa2fb7b819b0fdf2533d9f515b765866bbafa69ec0c320`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.1-stdout.tsv` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/stdout.tsv](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/stdout.tsv). SHA-256 `c8df00d86db8bfb7f5567b4bfd652675e2e8c024b9757cdc6ae8b77649a9f748`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.1-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.1-compile.stdout` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/compile.stdout](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `tcl9.1-compile.stderr` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/compile.stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/9.1.0/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `jim-receipt.json` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/receipt.json](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/receipt.json). SHA-256 `dfff4d65a842c12aacc9bd441bd8a8a19690cfbe79493fcdd1dbbd06d09f285e`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `jim-stdout.tsv` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/stdout.tsv](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/stdout.tsv). SHA-256 `7b734d38df121aebb78fcafce11385250f46fd35aa29b35101bde4ae5da07ced`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `jim-stderr` (observation): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `jim-compile.stdout` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/compile.stdout](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.
- `jim-compile.stderr` (provider): [rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/compile.stderr](../../../../rust/tcl-lsp-core/tests/data/native_minifier_script_role/jim/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original immutable provider receipt or process stream, preserved without rewriting capture bytes.

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

# Original word and subst object concatenation

Proof ID: `naming.word-and-subst.original-concatenation`

## Problem statement

A word template and subst can produce identical bytes while selecting different native concatenation operations. Eagerly converting each input object to bytes can destroy a pure ByteArray, materialize a Unicode object, lose an original result alias, or hide template cache effects. The question concerns retained original objects and two invocations, not equivalent Unicode displays or a compiler grant inferred from a list of values.

## Question

How do original compiled word evaluation and subst differ in output bytes, original object primary/residency, result aliasing and original template cache across the exact eight physical inputs and two calls?

## Scope

Public original object entry on C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and current Jim 0.84-9-g5bac7c9; original String set-result source via EvalObjEx/EvalObj versus subst argv through EvalObjv/EvalObjVector. No Document character channel, command mutation, callback-written variable, arbitrary flag combination, literal world, foreign interpreter, compilation/header or BIG-IP admission is established.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Compile0/process0; 33 rows. All eight input kinds reached both modes twice; template caches and original pointer/header outcomes retained. |
| tcl8.5 8.5.19 | observed | Compile0/process0; 33 rows. All eight input kinds reached both modes twice; template caches and original pointer/header outcomes retained. |
| tcl8.6 8.6.18 | observed | Compile0/process0; 33 rows. All eight input kinds reached both modes twice; template caches and original pointer/header outcomes retained. |
| tcl9.0 9.0.4 | observed | Compile0/process0; 33 rows. All eight input kinds reached both modes twice; template caches and original pointer/header outcomes retained. |
| tcl9.1 9.1.0 | observed | Compile0/process0; 33 rows. All eight input kinds reached both modes twice; template caches and original pointer/header outcomes retained. |
| jim 0.84-9-g5bac7c9 | observed | Compile0/process0; 29 rows. Unicode/ByteArray constructor controls explicitly unavailable; other six input kinds reached both modes twice. |
| bigip not recorded | not-tested | No appliance original-object or cache observation for this question. |

The eight inputs are ordinary String `A`, Unicode units `0000 D800`, resident String `41 00 42`, proper ByteArray `00 FF`, resident String `FF ED A0 80`, empty List, pure Double `1.5`, and empty String. Unicode and ByteArray constructor controls are unavailable on the Jim probe API. A native string getter runs only after the result/header snapshot; if the result aliases an input, this getter intentionally affects the second call.

## Conclusion

The operations require independently selected purposes. C8.4 compiled concatenation can return an untouched Unicode first operand while subst materializes it. C8.5 subst preserves pure Unicode or binary input through its token append path, independently of compiled concatenation. C8.6 word and subst preserve a binary result, but subst materializes the Unicode-empty concatenation that compiled word evaluation returns unchanged. C9 both operations retain the observed binary and empty-operand reuse behavior. Original subst templates acquire SubstCode only on C8.6/C9; Jim word results and both template caches use its independent source/script types. The complete retained rows define the narrow result/cache/alias behavior, including changes caused by the first result getter before the second call.

## Evidence

- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/probe.c`, SHA-256 `85591ce9af05c47fac5debaaec0187ca6b608bd151ba26bc26e440874fbea8a5`. Exact public C/Jim original object constructors, pre-getter snapshots and repeated entry.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/input.json`, SHA-256 `63c64b4dac844c4997b72ad6910c7c307f6cd445eaf81b29f6c1ab1f51b5491a`. Eight input kinds, exact source templates, operation modes and observation boundary.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/receipt.json`, SHA-256 `512c96256533cf4696408520361fde482762ed8a37826547834a6633685aab30`. Six actual compile/run receipts; exact input/library/header/executable and stream hashes.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/observations.tsv`, SHA-256 `9fa23d5c0f1ca09e0fee7a74be8165a34c513a255bd43c6e8b9c08dbf5c0b3e3`. Derived selected header/alias/result rows consumed by Rust tests; not a substitute for original stdout.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/replay.py`, SHA-256 `c433fc4b8ab7d72db74a43340a772185658805e91c323911147bd951b6fe634b`. Strict sequential native reconfirmation; startup, unavailable constructor controls and all rows compared.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/8.4.20/stdout.jsonl`, SHA-256 `fe74bf29bc7997432973b96ae8a3c0dce481467f9298967a3d1f082a11e516ce`. Actual native rows, including launched patchlevel, original primary/resident and alias state before result getter.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/8.4.20/receipt.json`, SHA-256 `31fbe2d46d6bdc036efc429477a3c37cc26ba5fc6ecfb01845dd2ec650f099cd`. Actual process and compile status and original static-library/header/executable identities.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/8.4.20/stderr`, SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Empty stderr for the completed native program.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/8.5.19/stdout.jsonl`, SHA-256 `b72c71fe6dc1ca04b6563907af36b3d42a4d45f8602b478ee2988b789e507f6a`. Actual native rows, including launched patchlevel, original primary/resident and alias state before result getter.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/8.5.19/receipt.json`, SHA-256 `35e2511ba418a8d790d03f9409e25dc467cbd17147e1d26bb3f635154757b185`. Actual process and compile status and original static-library/header/executable identities.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/8.5.19/stderr`, SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Empty stderr for the completed native program.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/8.6.18/stdout.jsonl`, SHA-256 `18c0c706c97e6547df4dd27edfdac97b8424bb9e8c0239e135b9066d4cade8b8`. Actual native rows, including launched patchlevel, original primary/resident and alias state before result getter.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/8.6.18/receipt.json`, SHA-256 `7c0de92ecea3899f1eccba716f09a067236966cbd9adcc39b43b11c2995a03f6`. Actual process and compile status and original static-library/header/executable identities.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/8.6.18/stderr`, SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Empty stderr for the completed native program.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/9.0.4/stdout.jsonl`, SHA-256 `549822c6c086bb51386c4347c204558dce6678f96546484eb7ba4087eb4a1c97`. Actual native rows, including launched patchlevel, original primary/resident and alias state before result getter.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/9.0.4/receipt.json`, SHA-256 `211a98ebf652f81e854e79daac5726c6c6ed1d50b36baec19f6556765b90f8c2`. Actual process and compile status and original static-library/header/executable identities.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/9.0.4/stderr`, SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Empty stderr for the completed native program.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/9.1.0/stdout.jsonl`, SHA-256 `a86009fdc25a37ed30590cf1ec0a85f2da85c5e6f0def897efe07034a174572d`. Actual native rows, including launched patchlevel, original primary/resident and alias state before result getter.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/9.1.0/receipt.json`, SHA-256 `59e55d4cb29cb8715f259f67b944d579909555e9fdd4fc8cd371d11421142ff4`. Actual process and compile status and original static-library/header/executable identities.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/9.1.0/stderr`, SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Empty stderr for the completed native program.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/jim/stdout.jsonl`, SHA-256 `6b9d1893527a1fb1a238dc0bfef2d2da362849e49443b3ccd81d6ce52a533410`. Actual native rows, including launched patchlevel, original primary/resident and alias state before result getter.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/jim/receipt.json`, SHA-256 `a891ff0dcbd2aba0eb70c55485ea051b6a822d5fdafb0c9be34adb3f1d85bc92`. Actual process and compile status and original static-library/header/executable identities.
- `rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/jim/stderr`, SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Empty stderr for the completed native program.

## Source anchors

The retained public probe is the execution evidence. Interpreter source supplies separate operation explanations: C8.4 `generic/tclCmdMZ.c::Tcl_SubstObj` starts a fresh string result; C8.5 `generic/tclParse.c::TclSubstTokens` adopts then appends token objects; C8.6/C9 `generic/tclCompile.c::CompileSubstObj` has its own interpreter/epoch/namespace/flags-dependent SubstCode owner. No broader cache admission follows from these observed primary names.

## Shared owners and tests

- `rust/tcl-cmd-core/src/native_cat.rs`: `concatenate_compiled`. Actual C compiled-word concatenation on original objects.
- `rust/tcl-vm/src/subst.rs`: `concatenate_word_values`. Completed compiled word values use independently retained native protocol.
- `runtime/rust/src/interp/native_body_artifact.rs`: `Interp::concatenate_body_values`. Genuinely admitted native body artifact result concatenation.
- `rust/tcl-vm/src/subst.rs`: `subst::tests::compiled_word_parts_match_original_native_concatenation_windows`. Compare all eighty available compiled C word result/operand windows, repeated-call getter changes and result aliasing; does not assert template Subst cache.
- `runtime/rust/src/interp/native_body_artifact.rs`: `interp::native_body_artifact::tests::original_body_word_parts_match_native_concatenation_windows`. Compare eighty actual C compiled word concatenation windows on retained Runtime objects; actual artifact admission and original template caches remain independent.

Native observations and Rust correspondence are independent. Only actual execution of each named Rust test establishes its implementation result.

## Reconfirmation

```text
python3 rust/tcl-cmd-core/tests/data/native_word_subst_concatenation/replay.py --tcl-source-root tmp --jim-source-root /workspace/.proofs/native-providers/jimtcl --output /tmp/word-subst-reconfirmation
```

Requires the exact built static libraries and headers for five named C releases plus Jim source revision5bac7c99ad65864c87da513e22e2f01703fa4e03 configured UTF8=1 and reporting0.84-9-g5bac7c9. Sequential calls have a60-second process budget. All startup/result rows are compared exactly; constructor unavailable rows are expected probe admission data. Compile, launch, nonzero process, stderr and malformed JSON are harness failures. Output rows alone do not prove Rust correspondence or cache lifetime authority.

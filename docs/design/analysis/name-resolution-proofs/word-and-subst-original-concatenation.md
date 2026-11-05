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

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

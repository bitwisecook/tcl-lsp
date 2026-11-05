# naming.substitution.counted-template-completions

Kind: `native-observation`

## Problem statement

Substitution templates contain raw zero, encoded zero, array-index commands, malformed suffixes and non-OK command completions. Treating the template as ordinary script source or flattening every completion into an error changes the reached prefix and native result representation. The ten explicit-count inputs and eight flag masks bound the question; the short v1 return inputs are distinct from later complete inputs.

## Question

What code, original template primary, result primary, resident string state and result bytes are reached for the ten counted templates under each subst flag mask and repeated call?

## Conclusion

The retained C5 rows distinguish ordinary values, counted zero, syntax errors and completion handling under all eight masks. In the full v6 inputs, flags7 returns 700070c08058587461696c for the raw-zero template, VALUEpX for the return template and OTHERpX for code7; continue skips its command and break stops at the reached prefix. Error and result header rows are observed before their result getter. This is the exact input/flag question, not a universal arbitrary-object or compiler admission claim.

An additional marked lexical source definition retains variable, escape and command parts inside an array index when outer command/backslash substitution is disabled. Disabling variables preserves the entire original source literal. The four source grammar selections remain explicit.

## Scope

Ten original native-value templates, masks7 through0, ROOT and ROOT_REPEAT. v1 shortens the final x of return sources8/9; v2-v6 use lengths17/25. Other setup labels are retained independently. Jim and BIG-IP were not run for this C private-header probe.

Word-part recognition is a software syntax obligation; it does not execute an index child or re-observe the original counted Native subst completion rows. Jim grammar selection stays separate from C evaluator purposes. No Native object/getter/frame/handler, completion, source admission or assertion result is added.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Ten explicit-count Tcl_NewStringObj templates supplied to stock subst through Tcl_EvalObjv with flags0, all subst masks, ROOT and ROOT_REPEAT; ASCII setup uses NUL-terminated Tcl_Eval. Version: tcl8.4 8.4.20 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel); tcl8.5 8.5.19 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel); tcl8.6 8.6.18 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel); tcl9.0 9.0.4 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel); tcl9.1 9.1.0 (source/header association; VERSION prints compile-time TCL_PATCH_LEVEL, not a queried linked runtime patchlevel).

Full v6 ten-template ROOT/ROOT_REPEAT flag matrix retained; flags7 raw-zero700070c08058587461696c, return56414c55457058, code7 4f544845527058; guest errors remain recorded outcomes.

### jim, bigip

Status: `not-tested`.

This exact C substitution/private-header question was not tested on this provider.

## Consumer bindings

- [rust/tcl-vm/src/command/native_substitution_tests.rs](../../../../rust/tcl-vm/src/command/native_substitution_tests.rs), `command::native_substitution_tests::original_compiled_substitution_matches_counted_native_flags_and_completions` (linked): 480 C86/C90/C91 R-row comparisons: completion, original/result primary and resident state before reached result getter, counted byte result, template-only cache.
- [runtime/rust/src/interp/native_body_artifact/native_substitution.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_substitution.rs), `interp::native_body_artifact::native_substitution::tests::original_compiled_substitution_matches_native_template_windows` (linked): 480 C86/C90/C91 original native object R-row comparisons with independent substcode artifact; template is not ordinary body bytecode.
- [rust/tcl-lexer/src/executable_parts.rs](../../../../rust/tcl-lexer/src/executable_parts.rs), `executable_parts::tests::template_masks_leave_array_index_substitutions_enabled` (linked): Outer disabled command/backslash substitutions retain independent ALL variable-index child execution. This is a lexer coverage assertion; the C v6 provider matrix independently records native results.
- [rust/tcl-lexer/src/word_parts.rs](../../../../rust/tcl-lexer/src/word_parts.rs), `word_parts::template_policy_tests::c_template_retains_disabled_sigil_and_escape_token_boundaries` (linked): C86/C90/C91 selected template token segmentation retains disabled sigils, escapes and raw zero independently of WrittenWord merging. Jim source inspection is a separate question and supplies no C matrix outcome.

- [rust/tcl-lexer/src/word_parts.rs](../../../../rust/tcl-lexer/src/word_parts.rs), `decompose`: Apply the actual selected source grammar and substitution mask at each original word/index boundary; an outer command mask cannot erase the independently selected array-index children.
- [rust/tcl-lexer/src/word_parts.rs](../../../../rust/tcl-lexer/src/word_parts.rs), `word_parts::tests::outer_substitution_mask_does_not_disable_array_index_commands` (linked): The original word-part grammar retains array-index variable, escaped-text and command children despite the outer commands/backslashes mask; disabling variables instead preserves the whole source as literal under the four selected source grammars.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_substitution_owner](../../../../rust/tcl-registry/tests/data/native_substitution_owner)
- [rust/tcl-registry/tests/data/native_substitution_owner/v1](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v1)
- [rust/tcl-registry/tests/data/native_substitution_owner/v2](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v2)
- [rust/tcl-registry/tests/data/native_substitution_owner/v3](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v3)
- [rust/tcl-registry/tests/data/native_substitution_owner/v5](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v5)
- [rust/tcl-registry/tests/data/native_substitution_owner/v6](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v6)
- [rust/tcl-registry/tests/data/native_substitution_owner/v4](../../../../rust/tcl-registry/tests/data/native_substitution_owner/v4)

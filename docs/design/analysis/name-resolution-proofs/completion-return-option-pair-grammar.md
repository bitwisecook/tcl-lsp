# naming.completion.return-option-pair-grammar

Kind: `native-observation`

## Problem statement

Original return source can select a source evaluator path whose duplicate-option and nested-dictionary results differ from an alias call targeting the same command. Numeric source grammar, validation order and optional catch fields must be retained with their actual release and channel.

## Question

Which option pairs, duplicate validation order, code/level conversions, nested options and binary-produced NUL operands are publicly accepted by the fixed original return sources in C Tcl 8.4–9.1 and current Jim?

## Conclusion

Twelve fresh CLI processes execute the fixed 61-source roster and 15-source extension, giving 76 complete public rows per provider with exit 0 and empty stderr. C8.4 has no catch options field and rejects -level/-options; its code validation occurs before a later duplicate can replace a bad code. C8.5–9.1 accept a later -code ok after BAD in the measured pair roster; C8.5/8.6 report eventual code 010 as 8, and C9 reports it as 10. C8.4 exposes no eventual-code options field in this observation. C8.5/8.6 accept the extension -level -4294967295 as level 1 after their finite conversion; C9 and Jim reject that negative input. Jim code 010 and level 010 are decimal 10, accepts the finite positive level 4294967295 with reported level -1, and accepts binary-produced numeric NUL prefixes. These Jim controls do not establish a general u32 range. C8.5 original-source nested-option rows must retain their own results rather than borrowing evaluated-worker383 results. All exact result/error/options bytes remain in the full fields. The independent failed378 C8.4 wrapper compilation is preserved and contributes no case observation.

## Scope

Exactly 61 main and 15 extension case scripts, each observed through the recorded original-source CLI wrapper on six pinned providers. ROW fields are catch code, counted result hex, optional -code/-level, errorcode hex and whole options hex. C8.4 ABSENT options fields remain absent. NUL operands are constructed by binary format inside the original case source; source characters remain ASCII. Failed378 retains all provider streams, including the C8.4 setup rejection. No evaluated-worker generalisation, private header/object/table, frame/instruction/compiler-body admission, arbitrary return input, editor equivalence, successful Rust test or BIG-IP observation follows.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Fresh CLI execution of each retained ASCII probe. Original return source inside the exact catch wrapper. Version: tcl8.4 8.4.20.

The main 61 and extension 15 sources produce 76 rows; catch -code/-level/options fields are ABSENT. -level/-options fail as unsupported options. BAD then ok still reports a bad completion code. Original378 fails wrapper compilation before a case row; its whole stderr is retained. Primary commands exit 0 with empty stderr; no Rust execution conclusion follows.

### tcl8.5

Status: `observed`. Channel: Fresh CLI execution of each retained ASCII probe. Original return source inside the exact catch wrapper. Version: tcl8.5 8.5.19.

The 76 source rows retain octal code 010/level 010 as 8 and code 4294967295 as -1. BAD then ok returns catch 2/-code 0. The finite -level -4294967295 extension returns level 1. Nested-before-outer-code reports -code 0; repeated invalid nested options then good reports catch 1/-code 1 with RESULT, differing from 383. Primary commands exit 0 with empty stderr; no Rust execution conclusion follows.

### tcl8.6

Status: `observed`. Channel: Fresh CLI execution of each retained ASCII probe. Original return source inside the exact catch wrapper. Version: tcl8.6 8.6.18.

The 76 source rows retain octal code 010/level 010 as 8 and code 4294967295 as -1. BAD then ok returns catch 2/-code 0. The finite -level -4294967295 extension returns level 1. Invalid nested dictionaries retain the observed TCL RESULT ILLEGAL_OPTIONS error. Primary commands exit 0 with empty stderr; no Rust execution conclusion follows.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Fresh CLI execution of each retained ASCII probe. Original return source inside the exact catch wrapper. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

The 76 source rows retain decimal code 010/level 010 as 10 and code 4294967295 as -1. BAD then ok returns catch 2/-code 0. Negative -level -4294967295 is rejected with the captured ILLEGAL_LEVEL fields. Invalid nested dictionaries retain ILLEGAL_OPTIONS. Primary commands exit 0 with empty stderr; no Rust execution conclusion follows.

### jim

Status: `observed`. Channel: Fresh CLI execution of each retained ASCII probe. Original return source inside the exact catch wrapper. Version: jim 0.84-9-g5bac7c9.

The 76 source rows retain decimal code 010/level 010 as 10. Both BAD/ok duplicate orders reject BAD; the finite positive level 4294967295 reports level -1. Binary-produced numeric NUL prefixes are accepted in the fixed controls. -options nested controls return the actual usage error. These finite widths are not a general u32 grant. Primary commands exit 0 with empty stderr; no Rust execution conclusion follows.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

`value_transfer::completion::decode_return_words_in(grammar, numbers, args, facet)` requires an explicit `ReturnInvocationFacet`. Typed source metadata uses `OriginalSource`; `EvaluatedArguments` requires an independently retained evaluated-worker entry. The argument dialect supplies the release, while `Numbers` supplies lexical numeral interpretation. Neither purpose tag nor type metadata creates worker, compiler, frame, object or header authority. Original result/direct errorcode operands retain argv ordinals; an options-dictionary errorcode is `merged_error_code`, not a fabricated operand position.

The public-field control reads the complete76-row source roster per provider. It compares immediate completion, accepted result and available code/level/errorcode fields; a grammar rejection compares code1 only. It does not compare whole options, errorInfo or rejection messages. The two C8.5 source/worker disagreements and three Jim wider-level rows remain explicit Unknown. The other six controls are software carrier and policy assertions, separate from the immutable Native observations.

- `value_transfer::completion::return_decoder_tests::source_return_decoder_matches_measured_public_fields_with_explicit_refusals`: Reads the exact 76 original-source rows per provider from379/380; compares immediate code, accepted result and available eventual-code/level/errorcode fields, with rejection code only. C85 two disputed source rows and Jim three wider-level rows explicitly remain Unknown. Whole options, errorInfo and rejection messages are not compared.
- `value_transfer::completion::return_decoder_tests::return_decoder_preserves_unknown_values_count_and_original_ordinals`: Software carrier control: unknown/expanded/opaque words and unknown nested options retain explicit uncertainty; direct result/errorcode operands retain original argv ordinals, while nested merged_error_code is a value with no fabricated ordinal.
- `value_transfer::completion::return_decoder_tests::return_value_transfer_keeps_actual_numeric_context_and_merged_error_code`: Software value-transfer control: a retained C90 context with an independent Tcl85 numeral overlay keeps octal interpretation; exact nested errorcode remains the merged value. This is not a measured mixed Native engine or physical header.
- `value_transfer::completion::return_decoder_tests::return_decoder_release_and_invocation_facet_are_independent_premises`: Software purpose control: missing release cannot borrow numeral grammar; OriginalSource and EvaluatedArguments remain distinct, including C85 disputed source rows and an independently selected worker interpretation. No evaluated worker entry is manufactured.
- `value_transfer::completion::return_decoder_tests::return_conversion_width_keeps_release_independent_of_numeral_overlay`: Software mixed-carrier control: actual C90 signed-conversion width remains independent of Tcl85 numeral overlay; C86 finite low-width conversion differs, missing release retains Unknown, and octal010 follows the supplied numeral policy.
- `value_transfer::completion::return_decoder_tests::return_decoder_keeps_unknown_grammar_and_numbers_as_separate_premises`: Software partial-policy control: narrow agreed common results/code remain available; missing grammar with a normalisation disagreement and actual C86 with unknown numeral010 interpretation retain Unknown rather than borrowing catalogue defaults.
- `value_transfer::completion::return_decoder_tests::registry_return_routes_keep_release_grammar_and_numerals_independent`: Software public Registry route control: actual C90 release with a Tcl84 numeral overlay retains options grammar while actual C84 with a Tcl90 overlay rejects; signed conversion width follows actual C release rather than numeral overlay. This projection grants no entered handler, compiler, object or frame facts.

These tests are linked to their current source bodies. No executed Rust assertion or backend pass is claimed by this binding. Their finite fields do not expand the Native question or provider scope.

## Evidence

- [rust/tcl-registry/tests/data/native_return_option_pair_grammar378](../../../../rust/tcl-registry/tests/data/native_return_option_pair_grammar378)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_return_option_pair_grammar379](../../../../rust/tcl-registry/tests/data/native_return_option_pair_grammar379)
- [rust/tcl-registry/tests/data/native_return_option_pair_grammar380](../../../../rust/tcl-registry/tests/data/native_return_option_pair_grammar380)
